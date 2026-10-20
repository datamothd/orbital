#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use orbital::{explorer, sysinfo};
mod profiles;

#[tauri::command]
async fn system_info() -> Result<sysinfo::SystemInfo, String> {
    let info = sysinfo::collect().map_err(|error| error.to_string())?;
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
    explorer::save_original_settings().map_err(|error| error.to_string())?;
    explorer::settings().map_err(|error| error.to_string())
}

#[tauri::command]
async fn set_folder_sort(sort: explorer::FolderSort) -> Result<(), String> {
    explorer::set_folder_sort(sort).map_err(|error| error.to_string())
}

#[tauri::command]
async fn set_folder_grouping(grouping: explorer::FolderGrouping) -> Result<(), String> {
    explorer::set_folder_grouping(grouping).map_err(|error| error.to_string())
}

#[tauri::command]
async fn apply_recommended_settings(
    preview: Option<bool>,
    selected: Option<Vec<explorer::Recommendation>>,
) -> Result<Vec<explorer::Recommendation>, String> {
    explorer::apply_recommended_settings(preview.unwrap_or(false), selected.as_deref())
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn restore_original_settings() -> Result<bool, String> {
    explorer::restore_original_settings().map_err(|error| error.to_string())
}

#[tauri::command]
async fn toggle_preference(preference: explorer::Preference) -> Result<(), String> {
    explorer::toggle_preference(preference, true).map_err(|error| error.to_string())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            system_info,
            explorer_settings,
            set_folder_sort,
            set_folder_grouping,
            apply_recommended_settings,
            restore_original_settings,
            toggle_preference,
            profiles::export_tweaks,
            profiles::import_tweaks,
            profiles::apply_imported_tweaks,
        ])
        .run(tauri::generate_context!())
        .expect("Failed to run Orbital");
}
