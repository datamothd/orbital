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
        let profile: TweakProfile = serde_json::from_str(json).unwrap();
        profile.validate().unwrap();
        let serialized = serde_json::to_value(&profile).unwrap();
        assert_eq!(
            serialized,
            serde_json::from_str::<serde_json::Value>(json).unwrap()
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
    fn version_two_round_trip_preserves_false_values() {
        let json = r#"{"format":"orbital-tweaks","version":2,"tweaks":["hide_recent_files"],"disabled_tweaks":["disable_explorer_spacing","hide_gallery"],"folder_sort":null,"folder_grouping":null}"#;
        let profile: TweakProfile = serde_json::from_str(json).unwrap();
        profile.validate().unwrap();
        assert_eq!(profile.disabled_tweaks.len(), 2);
        assert_eq!(
            serde_json::to_value(profile).unwrap(),
            serde_json::from_str::<serde_json::Value>(json).unwrap()
        );
    }
}
