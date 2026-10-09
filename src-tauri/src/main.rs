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
async fn toggle_disable_taskbar_centering() -> Result<(), String> {
    explorer::toggle_disable_taskbar_centering(true).map_err(|error| error.to_string())
}

#[tauri::command]
async fn toggle_disable_explorer_spacing() -> Result<(), String> {
    explorer::toggle_disable_explorer_spacing(true).map_err(|error| error.to_string())
}

#[tauri::command]
async fn toggle_disable_modern_context_menu() -> Result<(), String> {
    explorer::toggle_disable_modern_context_menu(true).map_err(|error| error.to_string())
}

#[tauri::command]
async fn toggle_hide_recent_files() -> Result<(), String> {
    explorer::toggle_hide_recent_files(true).map_err(|error| error.to_string())
}

#[tauri::command]
async fn toggle_hide_frequent_folders() -> Result<(), String> {
    explorer::toggle_hide_frequent_folders(true).map_err(|error| error.to_string())
}

#[tauri::command]
async fn toggle_hide_office_files() -> Result<(), String> {
    explorer::toggle_hide_office_files(true).map_err(|error| error.to_string())
}

#[tauri::command]
async fn toggle_hide_home_folder() -> Result<(), String> {
    explorer::toggle_hide_home_folder(true).map_err(|error| error.to_string())
}

#[tauri::command]
async fn toggle_hide_gallery() -> Result<(), String> {
    explorer::toggle_hide_gallery(true).map_err(|error| error.to_string())
}

#[tauri::command]
async fn toggle_hide_shortcut_arrow() -> Result<(), String> {
    explorer::toggle_hide_shortcut_arrow(true).map_err(|error| error.to_string())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            system_info,
            explorer_settings,
            toggle_disable_taskbar_centering,
            toggle_disable_explorer_spacing,
            toggle_disable_modern_context_menu,
            toggle_hide_recent_files,
            toggle_hide_frequent_folders,
            toggle_hide_office_files,
            toggle_hide_home_folder,
            toggle_hide_gallery,
            toggle_hide_shortcut_arrow,
        ])
        .run(tauri::generate_context!())
        .expect("Failed to run Orbital");
}
