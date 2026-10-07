#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use orbital::{explorer, sysinfo};

#[tauri::command]
async fn system_info() -> Result<sysinfo::SystemInfo, String> {
    let info = sysinfo::collect().map_err(|error| error.to_string())?;
    // Preview system labels in development without changing the actual tweak checks.
    #[cfg(debug_assertions)]
    let info = {
        let mut info = info;
        if let Ok(os) = std::env::var("ORBITAL_DEV_OS") {
            info.os = os;
        }
        if let Ok(version) = std::env::var("ORBITAL_DEV_WINDOWS_VERSION") {
            info.windows_version = version;
        }
        info
    };
    Ok(info)
}

#[tauri::command]
async fn explorer_settings() -> Result<explorer::ExplorerSettings, String> {
    explorer::settings().map_err(|error| error.to_string())
}

#[tauri::command]
async fn toggle_taskbar_alignment() -> Result<(), String> {
    explorer::toggle_taskbar_alignment(true).map_err(|error| error.to_string())
}

#[tauri::command]
async fn toggle_explorer_compact_mode() -> Result<(), String> {
    explorer::toggle_explorer_compact_mode(true).map_err(|error| error.to_string())
}

#[tauri::command]
async fn toggle_classic_context_menu() -> Result<(), String> {
    explorer::toggle_classic_context_menu(true).map_err(|error| error.to_string())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            system_info,
            explorer_settings,
            toggle_taskbar_alignment,
            toggle_explorer_compact_mode,
            toggle_classic_context_menu,
        ])
        .run(tauri::generate_context!())
        .expect("Failed to run Orbital");
}
