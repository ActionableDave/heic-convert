//! HEIC decoding/encoding via Windows Imaging Component (WIC).
//! Relies on the OS "HEIF Image Extensions" + "HEVC Video Extensions" codecs,
//! which ship preinstalled on Windows 11 (including ARM64).

use std::path::Path;

use windows::core::{w, Interface, PCWSTR, PWSTR};
use windows::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE, WINCODEC_ERR_COMPONENTNOTFOUND};
use windows::Win32::Graphics::Imaging::*;
use windows::Win32::System::Com::StructuredStorage::{IPropertyBag2, PROPBAG2, PROPVARIANT};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
};
use windows::Win32::System::Variant::{VT_R4, VT_UI2};

thread_local! {
    static COM_INIT: () = unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    };
}

fn wide(p: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    p.as_os_str().encode_wide().chain(Some(0)).collect()
}

pub fn convert(input: &Path, output: &Path, format: &str, quality: u8) -> Result<(), String> {
    unsafe { convert_inner(input, output, format, quality) }.map_err(|e| {
        let msg = e.message();
        // The most common failure: HEVC codec not installed.
        if e.code() == WINCODEC_ERR_COMPONENTNOTFOUND {
            "No HEIC codec found. Install 'HEIF Image Extensions' and 'HEVC Video Extensions' \
             from the Microsoft Store."
                .into()
        } else {
            format!("{msg} (0x{:08X})", e.code().0)
        }
    })
}

unsafe fn convert_inner(
    input: &Path,
    output: &Path,
    format: &str,
    quality: u8,
) -> windows::core::Result<()> {
    COM_INIT.with(|_| {});

    let factory: IWICImagingFactory =
        CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;

    let in_w = wide(input);
    let decoder = factory.CreateDecoderFromFilename(
        PCWSTR(in_w.as_ptr()),
        None,
        GENERIC_READ,
        WICDecodeMetadataCacheOnDemand,
    )?;
    let frame = decoder.GetFrame(0)?;

    // Apply EXIF orientation (if present) so photos don't come out sideways.
    let mut source: IWICBitmapSource = frame.cast()?;
    if let Some(transform) = orientation_transform(&frame) {
        let rotator = factory.CreateBitmapFlipRotator()?;
        rotator.Initialize(&source, transform)?;
        source = rotator.cast()?;
    }

    let (container, pixel_format) = match format {
        "png" => (GUID_ContainerFormatPng, GUID_WICPixelFormat32bppBGRA),
        _ => (GUID_ContainerFormatJpeg, GUID_WICPixelFormat24bppBGR),
    };

    let converter = factory.CreateFormatConverter()?;
    converter.Initialize(
        &source,
        &pixel_format,
        WICBitmapDitherTypeNone,
        None,
        0.0,
        WICBitmapPaletteTypeCustom,
    )?;

    let stream = factory.CreateStream()?;
    let out_w = wide(output);
    stream.InitializeFromFilename(PCWSTR(out_w.as_ptr()), GENERIC_WRITE.0)?;

    let encoder = factory.CreateEncoder(&container, std::ptr::null())?;
    encoder.Initialize(&stream, WICBitmapEncoderNoCache)?;

    let mut frame_enc: Option<IWICBitmapFrameEncode> = None;
    let mut props: Option<IPropertyBag2> = None;
    encoder.CreateNewFrame(&mut frame_enc, &mut props)?;
    let frame_enc = frame_enc.unwrap();

    if container == GUID_ContainerFormatJpeg {
        if let Some(bag) = &props {
            set_jpeg_quality(bag, quality)?;
        }
    }
    frame_enc.Initialize(props.as_ref())?;

    let converter_src: IWICBitmapSource = converter.cast()?;
    frame_enc.WriteSource(&converter_src, std::ptr::null())?;
    frame_enc.Commit()?;
    encoder.Commit()?;
    Ok(())
}

unsafe fn set_jpeg_quality(bag: &IPropertyBag2, quality: u8) -> windows::core::Result<()> {
    use windows::Win32::System::Variant::VARIANT;
    let mut name: Vec<u16> = "ImageQuality".encode_utf16().chain(Some(0)).collect();
    let propbag = PROPBAG2 {
        pstrName: PWSTR(name.as_mut_ptr()),
        ..Default::default()
    };
    let mut var = VARIANT::default();
    (*var.Anonymous.Anonymous).vt = VT_R4;
    (*var.Anonymous.Anonymous).Anonymous.fltVal = (quality as f32 / 100.0).clamp(0.05, 1.0);
    bag.Write(1, &propbag, &var)
}

/// Read the EXIF orientation tag (274) and map it to a WIC flip/rotate transform.
/// Returns None for "normal" orientation or when no tag is present.
unsafe fn orientation_transform(frame: &IWICBitmapFrameDecode) -> Option<WICBitmapTransformOptions> {
    let reader = frame.GetMetadataQueryReader().ok()?;
    let mut value = PROPVARIANT::default();
    let paths = [w!("/ifd/{ushort=274}"), w!("/app1/ifd/{ushort=274}")];
    let mut orientation: Option<u16> = None;
    for path in paths {
        if reader.GetMetadataByName(path, &mut value).is_ok() {
            if value.Anonymous.Anonymous.vt == VT_UI2 {
                orientation = Some(value.Anonymous.Anonymous.Anonymous.uiVal);
            }
            break;
        }
    }
    match orientation? {
        2 => Some(WICBitmapTransformFlipHorizontal),
        3 => Some(WICBitmapTransformRotate180),
        4 => Some(WICBitmapTransformFlipVertical),
        // 5 and 7 (transpositions) are practically nonexistent in real photos,
        // but map them to the closest rotate+flip combination anyway.
        5 => Some(WICBitmapTransformOptions(
            WICBitmapTransformRotate90.0 | WICBitmapTransformFlipHorizontal.0,
        )),
        6 => Some(WICBitmapTransformRotate90),
        7 => Some(WICBitmapTransformOptions(
            WICBitmapTransformRotate270.0 | WICBitmapTransformFlipHorizontal.0,
        )),
        8 => Some(WICBitmapTransformRotate270),
        _ => None,
    }
}
