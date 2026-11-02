use orbital::explorer::{self, TweakProfile};
use std::path::{Path, PathBuf};
use windows_sys::core::w;

fn choose_file(owner: usize, save: bool) -> Result<Option<PathBuf>, String> {
    crate::files::choose(
        owner,
        w!("Orbital tweaks (*.json)\0*.json\0"),
        if save {
            w!("Export tweak settings")
        } else {
            w!("Import tweaks")
        },
        w!("json"),
        save.then_some(Path::new("orbital-tweaks.json")),
    )
}

#[tauri::command]
pub async fn export_tweaks(window: tauri::Window) -> Result<bool, String> {
    crate::files::with_dialog(window, move |owner| {
        let Some(path) = choose_file(owner, true)? else {
            return Ok(false);
        };
        let profile = explorer::export_tweaks().map_err(|error| error.to_string())?;
        let json = serde_json::to_string_pretty(&profile).map_err(|error| error.to_string())?;
        std::fs::write(path, format!("{json}\n"))
            .map_err(|error| format!("Could not save the export: {error}"))?;
        Ok(true)
    })
    .await
}

#[tauri::command]
pub async fn import_tweaks(window: tauri::Window) -> Result<Option<TweakProfile>, String> {
    crate::files::with_dialog(window, move |owner| {
        let Some(path) = choose_file(owner, false)? else {
            return Ok(None);
        };
        let json = crate::files::read_document(&path)
            .map_err(|error| format!("Could not read the file: {error}"))?;
        let mut profile: TweakProfile =
            serde_json::from_slice(json.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&json))
                .map_err(|error| format!("Invalid Orbital tweaks file: {error}"))?;
        profile.normalize().map_err(|error| error.to_string())?;
        Ok(Some(profile))
    })
    .await
}

#[tauri::command]
pub async fn apply_imported_tweaks(
    profile: TweakProfile,
    replace_blank_icon: Option<bool>,
) -> Result<usize, serde_json::Value> {
    explorer::apply_imported_tweaks(profile, replace_blank_icon.unwrap_or(false))
        .map_err(crate::preference_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_round_trip_preserves_non_recommended_folder_choices() {
        let json = r#"{"format":"orbital-tweaks","version":1,"tweaks":["disable_taskbar_centering","hide_recent_files"],"folder_sort":{"column":"size","descending":true},"folder_grouping":"date_created"}"#;
        let mut profile: TweakProfile = serde_json::from_str(json).unwrap();
        profile.normalize().unwrap();
        let serialized = serde_json::to_value(&profile).unwrap();
        assert_eq!(serialized["version"], 4);
        assert_eq!(
            serialized["folder_sort"],
            serde_json::json!({"column":"size","descending":true})
        );
        assert_eq!(serialized["folder_grouping"], "date_created");
        assert_eq!(
            serialized["disabled_tweaks"],
            serde_json::json!([
                "center_taskbar_icons",
                "show_recent_files",
                "show_recommendations"
            ])
        );
    }

    #[test]
    fn malformed_or_unknown_tweaks_are_rejected() {
        for json in [
            "not json",
            r#"{"format":"orbital-tweaks","version":1,"tweaks":["unknown_tweak"]}"#,
            r#"{"format":"orbital-tweaks","version":2,"tweaks":[],"disabled_tweaks":["unknown_tweak"]}"#,
            r#"{"format":"orbital-tweaks","version":1,"tweaks":[false]}"#,
            r#"{"format":"orbital-tweaks","version":1,"tweaks":[],"folder_grouping":"unknown"}"#,
            r#"{"format":"orbital-tweaks","version":1,"tweaks":[],"registry_path":"arbitrary"}"#,
        ] {
            assert!(serde_json::from_str::<TweakProfile>(json).is_err());
        }
    }

    #[test]
    fn version_two_import_preserves_enabled_and_disabled_intent() {
        let json = r#"{"format":"orbital-tweaks","version":2,"tweaks":["hide_recent_files"],"disabled_tweaks":["disable_explorer_spacing","hide_gallery"],"folder_sort":null,"folder_grouping":null}"#;
        let mut profile: TweakProfile = serde_json::from_str(json).unwrap();
        profile.normalize().unwrap();
        let serialized = serde_json::to_value(profile).unwrap();
        assert_eq!(serialized["version"], 4);
        assert_eq!(serialized["tweaks"], serde_json::json!(["show_gallery"]));
        assert_eq!(
            serialized["disabled_tweaks"],
            serde_json::json!([
                "show_recent_files",
                "show_recommendations",
                "use_compact_mode"
            ])
        );
    }

    #[test]
    fn legacy_inverse_names_keep_their_effect_and_normalize_once() {
        for (old, new) in [
            ("disable_taskbar_centering", "center_taskbar_icons"),
            ("hide_task_view", "show_task_view"),
            ("disable_widgets", "allow_widgets"),
            ("hide_recent_files", "show_recent_files"),
            ("hide_recommendations", "show_recommendations"),
            ("hide_frequent_folders", "show_frequent_folders"),
            ("hide_office_files", "show_cloud_files"),
            ("hide_home_folder", "show_home_folder"),
            ("hide_gallery", "show_gallery"),
            ("show_file_extensions", "hide_file_extensions"),
            ("single_click_open", "double_click_open"),
            ("show_empty_drives", "hide_empty_drives"),
            ("hide_info_tips", "show_info_tips"),
        ] {
            for enabled in [true, false] {
                let mut json = serde_json::json!({"format":"orbital-tweaks", "version":3, "tweaks":[], "disabled_tweaks":[]});
                json[if enabled { "tweaks" } else { "disabled_tweaks" }] = serde_json::json!([old]);
                let mut profile: TweakProfile = serde_json::from_value(json).unwrap();
                profile.normalize().unwrap();
                let normalized = serde_json::to_value(&profile).unwrap();
                assert_eq!(
                    normalized[if enabled { "disabled_tweaks" } else { "tweaks" }],
                    serde_json::json!([new])
                );
                profile.normalize().unwrap();
                assert_eq!(normalized, serde_json::to_value(profile).unwrap());
            }
        }
    }

    #[test]
    fn explorer_options_and_starting_location_round_trip() {
        let json = r#"{"format":"orbital-tweaks","version":4,"tweaks":["hide_file_extensions","double_click_open"],"disabled_tweaks":["show_system_files"],"folder_sort":null,"folder_grouping":null,"open_explorer_to":"downloads"}"#;
        let profile: TweakProfile = serde_json::from_str(json).unwrap();
        profile.validate().unwrap();
        assert_eq!(
            serde_json::to_value(profile).unwrap(),
            serde_json::from_str::<serde_json::Value>(json).unwrap()
        );
        assert!(
            serde_json::from_str::<TweakProfile>(&json.replace("downloads", "arbitrary_path"))
                .is_err()
        );
    }
}
