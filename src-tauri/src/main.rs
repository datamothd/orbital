#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use orbital::{explorer, sysinfo};
use windows_sys::{Win32::UI::Shell::ShellExecuteW, core::w};
mod profiles;

fn preference_error(error: std::io::Error) -> serde_json::Value {
    if let Some(conflict) = error
        .get_ref()
        .and_then(|error| error.downcast_ref::<explorer::BlankIconConflict>())
    {
        serde_json::json!({ "blank_icon_conflict": conflict.0.to_string_lossy() })
    } else {
        serde_json::Value::String(error.to_string())
    }
}

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
    replace_blank_icon: Option<bool>,
) -> Result<Vec<explorer::Recommendation>, serde_json::Value> {
    explorer::apply_recommended_settings(
        preview.unwrap_or(false),
        selected.as_deref(),
        replace_blank_icon.unwrap_or(false),
    )
    .map_err(preference_error)
}

#[tauri::command]
async fn restore_original_settings() -> Result<bool, String> {
    explorer::restore_original_settings().map_err(|error| error.to_string())
}

#[tauri::command]
async fn toggle_preference(
    preference: explorer::Preference,
    replace_blank_icon: Option<bool>,
) -> Result<(), serde_json::Value> {
    explorer::toggle_preference(preference, true, replace_blank_icon.unwrap_or(false))
        .map_err(preference_error)
}

#[tauri::command]
async fn open_windows_download() -> Result<(), String> {
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            w!("open"),
            w!("https://www.microsoft.com/en-us/software-download/windows11"),
            std::ptr::null(),
            std::ptr::null(),
            1,
        )
    } as isize;
    if result <= 32 {
        return Err(format!("Could not launch the browser ({result})."));
    }
    Ok(())
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
            open_windows_download,
            profiles::export_tweaks,
            profiles::import_tweaks,
            profiles::apply_imported_tweaks,
        ])
        .run(tauri::generate_context!())
        .expect("Failed to run Orbital");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_conflicts_include_the_path_and_other_errors_remain_strings() {
        let path = std::path::PathBuf::from(r"C:\Windows\blank.ico");
        assert_eq!(
            preference_error(std::io::Error::other(explorer::BlankIconConflict(path))),
            serde_json::json!({ "blank_icon_conflict": r"C:\Windows\blank.ico" })
        );
        assert_eq!(
            preference_error(std::io::Error::other("other error")),
            serde_json::json!("other error")
        );
    }
}
