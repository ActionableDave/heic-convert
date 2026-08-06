use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
#[path = "convert_windows.rs"]
mod platform;

#[cfg(target_os = "macos")]
#[path = "convert_macos.rs"]
mod platform;

/// Convert a single HEIC file. Returns the output path on success.
pub fn convert_one(
    input: &str,
    format: &str,
    quality: u8,
    out_dir: Option<&str>,
) -> Result<String, String> {
    let input_path = Path::new(input);
    if !input_path.is_file() {
        return Err("File not found".into());
    }
    let ext = match format {
        "jpeg" => "jpg",
        "png" => "png",
        other => return Err(format!("Unsupported format: {other}")),
    };
    let output = unique_output_path(input_path, ext, out_dir)?;

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    platform::convert(input_path, &output, format, quality)?;

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    return Err("This platform is not supported yet".into());

    // Keep the original photo's modified time so date-sorted folders stay in order.
    if let Ok(meta) = std::fs::metadata(input_path) {
        let mtime = filetime::FileTime::from_last_modification_time(&meta);
        let _ = filetime::set_file_mtime(&output, mtime);
    }
    Ok(output.to_string_lossy().into_owned())
}

/// Same base name with the new extension; appends " (1)", " (2)", ... on collision.
fn unique_output_path(input: &Path, ext: &str, out_dir: Option<&str>) -> Result<PathBuf, String> {
    let dir: PathBuf = match out_dir {
        Some(d) => PathBuf::from(d),
        None => input
            .parent()
            .map(PathBuf::from)
            .ok_or("Cannot determine output folder")?,
    };
    if !dir.is_dir() {
        return Err(format!("Output folder does not exist: {}", dir.display()));
    }
    let stem = input
        .file_stem()
        .ok_or("Cannot determine file name")?
        .to_string_lossy();

    let candidate = dir.join(format!("{stem}.{ext}"));
    if !candidate.exists() {
        return Ok(candidate);
    }
    for n in 1..1000 {
        let candidate = dir.join(format!("{stem} ({n}).{ext}"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err("Could not find a free output file name".into())
}
