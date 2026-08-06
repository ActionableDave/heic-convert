//! Explorer right-click menu integration (Windows only).
//! Writes per-user registry entries under HKCU\Software\Classes\SystemFileAssociations
//! for .heic/.heif files — no admin rights needed. On Windows 11 the entries appear
//! in the classic menu ("Show more options").

use winreg::enums::*;
use winreg::RegKey;

const EXTENSIONS: [&str; 2] = [".heic", ".heif"];
const VERBS: [(&str, &str, &str); 2] = [
    ("HEICConvert.jpeg", "Convert to JPEG", "jpeg"),
    ("HEICConvert.png", "Convert to PNG", "png"),
];

fn shell_key(ext: &str) -> String {
    format!("Software\\Classes\\SystemFileAssociations\\{ext}\\shell")
}

pub fn enable() -> Result<(), String> {
    let exe = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .into_owned();
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    for ext in EXTENSIONS {
        for (verb, label, fmt) in VERBS {
            let (key, _) = hkcu
                .create_subkey(format!("{}\\{verb}", shell_key(ext)))
                .map_err(|e| e.to_string())?;
            key.set_value("", &label).map_err(|e| e.to_string())?;
            key.set_value("Icon", &exe).map_err(|e| e.to_string())?;
            // Allow the verb on large multi-selections (Explorer default caps at 15 files).
            key.set_value("MultiSelectModel", &"Player")
                .map_err(|e| e.to_string())?;
            let (cmd, _) = key.create_subkey("command").map_err(|e| e.to_string())?;
            cmd.set_value("", &format!("\"{exe}\" --quick {fmt} \"%1\""))
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub fn disable() -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    for ext in EXTENSIONS {
        for (verb, _, _) in VERBS {
            let path = format!("{}\\{verb}", shell_key(ext));
            match hkcu.delete_subkey_all(&path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.to_string()),
            }
        }
    }
    Ok(())
}

pub fn is_enabled() -> bool {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(format!("{}\\{}", shell_key(".heic"), VERBS[0].0))
        .is_ok()
}
