use orbital::explorer::{self, TweakProfile};
use std::{io::Read, os::windows::ffi::OsStringExt, path::PathBuf};
use windows_sys::Win32::UI::Controls::Dialogs::{
    CommDlgExtendedError, GetOpenFileNameW, GetSaveFileNameW, OFN_EXPLORER, OFN_FILEMUSTEXIST,
    OFN_NOCHANGEDIR, OFN_OVERWRITEPROMPT, OFN_PATHMUSTEXIST, OPENFILENAMEW,
};
use windows_sys::core::w;

fn choose_file(owner: usize, save: bool) -> Result<Option<PathBuf>, String> {
    let mut filename = vec![0u16; 32768];
    if save {
        for (slot, ch) in filename
            .iter_mut()
            .zip("orbital-tweaks.json".encode_utf16())
        {
            *slot = ch;
        }
    }
    let mut dialog = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: owner as _,
        lpstrFilter: w!("Orbital tweaks (*.json)\0*.json\0"),
        nFilterIndex: 1,
        lpstrFile: filename.as_mut_ptr(),
        nMaxFile: filename.len() as u32,
        lpstrTitle: if save {
            w!("Export tweak settings")
        } else {
            w!("Import tweaks")
        },
        lpstrDefExt: w!("json"),
        Flags: OFN_EXPLORER
            | OFN_NOCHANGEDIR
            | OFN_PATHMUSTEXIST
            | if save {
                OFN_OVERWRITEPROMPT
            } else {
                OFN_FILEMUSTEXIST
            },
        ..Default::default()
    };
    let chosen = unsafe {
        if save {
            GetSaveFileNameW(&mut dialog)
        } else {
            GetOpenFileNameW(&mut dialog)
        }
    };
    if chosen == 0 {
        let error = unsafe { CommDlgExtendedError() };
        return if error == 0 {
            Ok(None)
        } else {
            Err(format!("Could not open the file dialog ({error:#x})."))
        };
    }
    let length = filename
        .iter()
        .position(|ch| *ch == 0)
        .unwrap_or(filename.len());
    Ok(Some(
        std::ffi::OsString::from_wide(&filename[..length]).into(),
    ))
}

#[tauri::command]
pub async fn export_tweaks(window: tauri::Window) -> Result<bool, String> {
    let owner = window.hwnd().map_err(|error| error.to_string())?.0 as usize;
    tauri::async_runtime::spawn_blocking(move || {
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
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn import_tweaks(window: tauri::Window) -> Result<Option<TweakProfile>, String> {
    let owner = window.hwnd().map_err(|error| error.to_string())?.0 as usize;
    tauri::async_runtime::spawn_blocking(move || {
        let Some(path) = choose_file(owner, false)? else {
            return Ok(None);
        };
        let file = std::fs::File::open(path)
            .map_err(|error| format!("Could not read the file: {error}"))?;
        let mut json = Vec::new();
        file.take(1_048_577)
            .read_to_end(&mut json)
            .map_err(|error| format!("Could not read the file: {error}"))?;
        if json.len() > 1_048_576 {
            return Err(
                "The file is too large. Choose an Orbital tweaks JSON file under 1 MiB.".into(),
            );
        }
        let mut profile: TweakProfile =
            serde_json::from_slice(json.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&json))
                .map_err(|error| format!("Invalid Orbital tweaks file: {error}"))?;
        profile.normalize().map_err(|error| error.to_string())?;
        Ok(Some(profile))
    })
    .await
    .map_err(|error| error.to_string())?
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
