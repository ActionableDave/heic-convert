#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod convert;
#[cfg(target_os = "windows")]
mod context_menu;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Serialize, Clone)]
struct FileResult {
    input: String,
    output: Option<String>,
    ok: bool,
    error: Option<String>,
}

#[derive(Serialize, Clone)]
struct Progress {
    index: usize,
    total: usize,
    result: FileResult,
}

#[derive(Serialize)]
struct ContextMenuStatus {
    supported: bool,
    enabled: bool,
}

#[tauri::command]
async fn convert_files(
    app: AppHandle,
    files: Vec<String>,
    format: String,
    quality: u8,
    out_dir: Option<String>,
) -> Result<Vec<FileResult>, String> {
    let total = files.len();
    let handle = tauri::async_runtime::spawn_blocking(move || {
        let mut results = Vec::with_capacity(total);
        for (i, f) in files.iter().enumerate() {
            let res = match convert::convert_one(f, &format, quality, out_dir.as_deref()) {
                Ok(out) => FileResult {
                    input: f.clone(),
                    output: Some(out),
                    ok: true,
                    error: None,
                },
                Err(e) => FileResult {
                    input: f.clone(),
                    output: None,
                    ok: false,
                    error: Some(e),
                },
            };
            let _ = app.emit(
                "conversion-progress",
                Progress {
                    index: i,
                    total,
                    result: res.clone(),
                },
            );
            results.push(res);
        }
        results
    });
    handle.await.map_err(|e| e.to_string())
}

#[tauri::command]
fn context_menu_status() -> ContextMenuStatus {
    #[cfg(target_os = "windows")]
    {
        ContextMenuStatus {
            supported: true,
            enabled: context_menu::is_enabled(),
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        ContextMenuStatus {
            supported: false,
            enabled: false,
        }
    }
}

#[tauri::command]
fn set_context_menu(enabled: bool) -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    {
        if enabled {
            context_menu::enable()?;
        } else {
            context_menu::disable()?;
        }
        Ok(enabled)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = enabled;
        Err("Not supported on this platform".into())
    }
}

/// Show a native error dialog (quick mode has no window to report into).
#[cfg(target_os = "windows")]
fn alert_error(msg: &str) {
    use windows::core::PCWSTR;
    use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
    let text: Vec<u16> = msg.encode_utf16().chain(Some(0)).collect();
    let title: Vec<u16> = "HEIC Convert".encode_utf16().chain(Some(0)).collect();
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(text.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OK | MB_ICONERROR,
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn alert_error(msg: &str) {
    eprintln!("{msg}");
}

/// Handle command-line invocations (Explorer context menu, scripting).
/// Returns true when the invocation was handled and the GUI should not start.
fn handle_cli(args: &[String]) -> bool {
    match args.first().map(String::as_str) {
        // heic-convert.exe --quick <jpeg|png> <file> [file...]
        Some("--quick") if args.len() >= 3 => {
            let format = args[1].clone();
            let mut errors = Vec::new();
            for file in &args[2..] {
                if let Err(e) = convert::convert_one(file, &format, 85, None) {
                    errors.push(format!("{file}\n  {e}"));
                }
            }
            if !errors.is_empty() {
                alert_error(&format!(
                    "Failed to convert {} file(s):\n\n{}",
                    errors.len(),
                    errors.join("\n\n")
                ));
            }
            true
        }
        #[cfg(target_os = "windows")]
        Some("--register-context-menu") => {
            if let Err(e) = context_menu::enable() {
                alert_error(&format!("Could not add the right-click menu: {e}"));
            }
            true
        }
        #[cfg(target_os = "windows")]
        Some("--unregister-context-menu") => {
            if let Err(e) = context_menu::disable() {
                alert_error(&format!("Could not remove the right-click menu: {e}"));
            }
            true
        }
        _ => false,
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() && handle_cli(&args) {
        return;
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            convert_files,
            context_menu_status,
            set_context_menu
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
