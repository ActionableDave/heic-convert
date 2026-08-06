//! HEIC conversion via `sips`, which ships with every macOS install and
//! decodes HEIC natively (including orientation and metadata handling).

use std::path::Path;
use std::process::Command;

pub fn convert(input: &Path, output: &Path, format: &str, quality: u8) -> Result<(), String> {
    let fmt = if format == "png" { "png" } else { "jpeg" };
    let mut cmd = Command::new("sips");
    cmd.arg("-s").arg("format").arg(fmt);
    if fmt == "jpeg" {
        cmd.arg("-s").arg("formatOptions").arg(quality.to_string());
    }
    let out = cmd
        .arg(input)
        .arg("--out")
        .arg(output)
        .output()
        .map_err(|e| format!("Failed to run sips: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        Err(if err.trim().is_empty() {
            "sips failed to convert this file".into()
        } else {
            err.trim().to_string()
        })
    }
}
