use std::os::windows::process::CommandExt;
use std::{
    fs::OpenOptions,
    io::{self, Write},
    path::PathBuf,
    process::Command as ProcessCommand,
};
use winreg::{
    HKCU, HKLM, RegKey, RegValue,
    enums::{KEY_QUERY_VALUE, KEY_SET_VALUE},
};

const CLASSIC_CONTEXT_MENU_KEY: &str =
    r"Software\Classes\CLSID\{86ca1aa0-34aa-4e8b-a509-50c905bae2a2}\InprocServer32";
const EXPLORER_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer";
const ADVANCED_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced";
const SHELL_ICONS_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Shell Icons";
const HOME_CLSID: &str = "{f874310e-b6b7-47dc-bc84-b9e6b38f5903}";
const GALLERY_CLSID: &str = "{e88865ea-0e1c-4e20-9aa6-edcd0212c87c}";

fn supports_windows11() -> io::Result<bool> {
    let key = HKLM.open_subkey_with_flags(
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
        KEY_QUERY_VALUE,
    )?;
    let build: String = key.get_value("CurrentBuildNumber")?;
    let build: u32 = build
        .parse()
        .map_err(|_| io::Error::other("Invalid Windows build number"))?;
    Ok(build >= 22000)
}

fn classic_context_menu_enabled() -> io::Result<bool> {
    let key = match HKCU.open_subkey_with_flags(CLASSIC_CONTEXT_MENU_KEY, KEY_QUERY_VALUE) {
        Ok(key) => key,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    match key.get_value::<String, _>("") {
        Ok(value) => Ok(value.is_empty()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

#[derive(serde::Serialize)]
pub struct ExplorerSettings {
    pub disable_taskbar_centering: bool,
    pub disable_explorer_spacing: bool,
    pub disable_modern_context_menu: bool,
    pub windows11_supported: bool,
    pub hide_recent_files: bool,
    pub hide_frequent_folders: bool,
    pub hide_office_files: bool,
    pub hide_home_folder: bool,
    pub hide_gallery: bool,
    // None means a custom overlay or a missing blank icon needs to be replaced.
    pub hide_shortcut_arrow: Option<bool>,
}

pub fn settings() -> io::Result<ExplorerSettings> {
    let windows11_supported = supports_windows11()?;
    Ok(ExplorerSettings {
        disable_taskbar_centering: DISABLE_TASKBAR_CENTERING.enabled_in(HKCU)?,
        disable_explorer_spacing: DISABLE_EXPLORER_SPACING.enabled_in(HKCU)?,
        disable_modern_context_menu: windows11_supported && classic_context_menu_enabled()?,
        windows11_supported,
        hide_recent_files: HIDE_RECENT_FILES.enabled_in(HKCU)?,
        hide_frequent_folders: HIDE_FREQUENT_FOLDERS.enabled_in(HKCU)?,
        hide_office_files: windows11_supported && HIDE_OFFICE_FILES.enabled_in(HKCU)?,
        hide_home_folder: windows11_supported && !namespace_visible(HOME_CLSID)?,
        hide_gallery: windows11_supported && !namespace_visible(GALLERY_CLSID)?,
        hide_shortcut_arrow: shortcut_arrows_hidden()?,
    })
}

fn confirm(prompt: &str) -> io::Result<bool> {
    loop {
        print!("{prompt} [Y/n] ");
        io::stdout().flush()?;
        let mut answer = String::new();
        if io::stdin().read_line(&mut answer)? == 0 {
            return Ok(false);
        }
        match answer.trim().to_ascii_lowercase().as_str() {
            "" | "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => println!("Please enter y or n."),
        }
    }
}

pub fn toggle_disable_taskbar_centering(yes: bool) -> io::Result<()> {
    DISABLE_TASKBAR_CENTERING.toggle(yes)
}

pub fn toggle_disable_explorer_spacing(yes: bool) -> io::Result<()> {
    DISABLE_EXPLORER_SPACING.toggle(yes)
}

pub fn toggle_disable_modern_context_menu(yes: bool) -> io::Result<()> {
    let name = "Disable Windows 11 context menu";
    require_windows11(name)?;
    let current = classic_context_menu_enabled()?;
    if !confirm_toggle(name, current, yes)? {
        return Ok(());
    }
    let changes = [RegistryChange::new(
        HKCU,
        CLASSIC_CONTEXT_MENU_KEY,
        "",
        if current {
            SettingValue::Delete
        } else {
            SettingValue::String(String::new())
        },
    )];
    let prepared = prepare_changes(&changes)?;
    apply_changes(&changes, &prepared)?;
    println!(
        "{name} saved: {}",
        if current { "disabled" } else { "enabled" }
    );
    restart_explorer(name)
}

fn read_toggle(root: &RegKey, path: &str, name: &str, default: bool) -> io::Result<bool> {
    let key = match root.open_subkey_with_flags(path, KEY_QUERY_VALUE) {
        Ok(key) => key,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(default),
        Err(error) => return Err(error),
    };
    let value: u32 = match key.get_value(name) {
        Ok(value) => value,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(default),
        Err(error) => return Err(error),
    };
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(io::Error::other(format!("{name} must be 0 or 1"))),
    }
}

fn namespace_paths(clsid: &str) -> (String, String, String) {
    (
        format!(r"Software\Classes\CLSID\{clsid}"),
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\NonEnum".into(),
        format!(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Desktop\NameSpace\{clsid}"),
    )
}

fn namespace_visible(clsid: &str) -> io::Result<bool> {
    namespace_visible_in(HKCU, HKLM, clsid)
}

fn namespace_visible_in(user: &RegKey, machine: &RegKey, clsid: &str) -> io::Result<bool> {
    let (pinned, non_enum, hidden) = namespace_paths(clsid);
    // Missing overrides leave the built-in Windows namespace visible. An explicit
    // hide in any of the three locations takes precedence.
    let pinned = read_toggle(user, &pinned, "System.IsPinnedToNameSpaceTree", true)?;
    let excluded = read_toggle(machine, &non_enum, clsid, false)?;
    let hidden = read_toggle(machine, &hidden, "HiddenByDefault", false)?;
    Ok(pinned && !excluded && !hidden)
}

fn blank_icon_path() -> io::Result<PathBuf> {
    let system_root = std::env::var_os("SystemRoot")
        .ok_or_else(|| io::Error::other("SystemRoot is unavailable"))?;
    let root = PathBuf::from(system_root);
    if !root.is_absolute() {
        return Err(io::Error::other("SystemRoot must be an absolute path"));
    }
    Ok(root.join("blank.ico"))
}

fn shortcut_arrows_hidden() -> io::Result<Option<bool>> {
    shortcut_arrows_hidden_in(HKLM, &blank_icon_path()?)
}

fn shortcut_arrows_hidden_in(
    root: &RegKey,
    blank_path: &std::path::Path,
) -> io::Result<Option<bool>> {
    let key = match root.open_subkey_with_flags(SHELL_ICONS_KEY, KEY_QUERY_VALUE) {
        Ok(key) => key,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Some(false)),
        Err(error) => return Err(error),
    };
    let overlay: String = match key.get_value("29") {
        Ok(value) => value,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Some(false)),
        Err(error) => return Err(error),
    };
    if overlay.eq_ignore_ascii_case(&blank_path.to_string_lossy()) && blank_path.is_file() {
        return Ok(Some(true));
    }
    Ok(None)
}

enum SettingValue {
    Dword(u32),
    String(String),
    Delete,
}

struct RegistryChange<'a> {
    root: &'a RegKey,
    path: String,
    name: String,
    value: SettingValue,
}

impl<'a> RegistryChange<'a> {
    fn new(root: &'a RegKey, path: &str, name: &str, value: SettingValue) -> Self {
        Self {
            root,
            path: path.into(),
            name: name.into(),
            value,
        }
    }
}

fn delete_value_if_present(key: &RegKey, name: &str) -> io::Result<()> {
    match key.delete_value(name) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn setting_error(error: io::Error) -> io::Error {
    if error.kind() == io::ErrorKind::PermissionDenied {
        io::Error::new(
            error.kind(),
            "This preference requires administrator access. Close Orbital and run it as administrator.",
        )
    } else {
        error
    }
}

type RegistrySnapshot = (RegKey, Option<RegValue<'static>>);

fn prepare_changes(changes: &[RegistryChange<'_>]) -> io::Result<Vec<RegistrySnapshot>> {
    changes
        .iter()
        .map(|change| {
            // Check access to every key before changing any values, so an HKLM
            // permission failure cannot leave only the HKCU part of a tweak applied.
            let (key, _) = change
                .root
                .create_subkey_with_flags(&change.path, KEY_QUERY_VALUE | KEY_SET_VALUE)?;
            let previous = match key.get_raw_value(&change.name) {
                Ok(value) => Some(value),
                Err(error) if error.kind() == io::ErrorKind::NotFound => None,
                Err(error) => return Err(error),
            };
            Ok((key, previous))
        })
        .collect::<io::Result<_>>()
        .map_err(setting_error)
}

fn apply_changes(changes: &[RegistryChange<'_>], prepared: &[RegistrySnapshot]) -> io::Result<()> {
    for (index, (change, (key, _))) in changes.iter().zip(prepared).enumerate() {
        let result = match &change.value {
            SettingValue::Dword(value) => key.set_value(&change.name, value),
            SettingValue::String(value) => key.set_value(&change.name, value),
            SettingValue::Delete => delete_value_if_present(key, &change.name),
        };
        if let Err(error) = result {
            let mut failures = Vec::new();
            for (change, (key, previous)) in changes[..index].iter().zip(&prepared[..index]).rev() {
                let restored = match previous {
                    Some(value) => key.set_raw_value(&change.name, value),
                    None => delete_value_if_present(key, &change.name),
                };
                if let Err(error) = restored {
                    failures.push(format!("{}: {error}", change.name));
                }
            }
            let error = setting_error(error);
            return if failures.is_empty() {
                Err(error)
            } else {
                Err(io::Error::other(format!(
                    "{error}. Could not restore previous values: {}",
                    failures.join("; ")
                )))
            };
        }
    }
    Ok(())
}

fn confirm_toggle(name: &str, current: bool, yes: bool) -> io::Result<bool> {
    if yes {
        return Ok(true);
    }
    println!("{name}: {}", if current { "enabled" } else { "disabled" });
    if !confirm(&format!("Toggle {name} and restart Explorer?"))? {
        println!("Cancelled.");
        return Ok(false);
    }
    Ok(true)
}

struct DwordPreference {
    name: &'static str,
    path: &'static str,
    values: &'static [&'static str],
    default: bool,
    enabled_value: bool,
    delete_when_disabled: bool,
}

const DISABLE_TASKBAR_CENTERING: DwordPreference = DwordPreference {
    name: "Disable centered taskbar icons",
    path: ADVANCED_KEY,
    values: &["TaskbarAl"],
    default: true,
    enabled_value: false,
    delete_when_disabled: false,
};
const DISABLE_EXPLORER_SPACING: DwordPreference = DwordPreference {
    name: "Disable extra spacing",
    path: ADVANCED_KEY,
    values: &["UseCompactMode"],
    default: false,
    enabled_value: true,
    delete_when_disabled: false,
};
const HIDE_RECENT_FILES: DwordPreference = DwordPreference {
    name: "Hide recently used files",
    path: EXPLORER_KEY,
    values: &["ShowRecent", "ShowRecommendations"],
    default: true,
    enabled_value: false,
    delete_when_disabled: false,
};
const HIDE_FREQUENT_FOLDERS: DwordPreference = DwordPreference {
    name: "Hide frequently used folders",
    path: EXPLORER_KEY,
    values: &["ShowFrequent"],
    default: true,
    enabled_value: false,
    delete_when_disabled: false,
};
const HIDE_OFFICE_FILES: DwordPreference = DwordPreference {
    name: "Hide files from Office.com",
    path: EXPLORER_KEY,
    values: &["ShowCloudFilesInQuickAccess"],
    default: true,
    enabled_value: false,
    delete_when_disabled: true,
};

impl DwordPreference {
    fn enabled_in(&self, root: &RegKey) -> io::Result<bool> {
        let states = self
            .values
            .iter()
            .map(|value| read_toggle(root, self.path, value, self.default))
            .collect::<io::Result<Vec<_>>>()?;
        // Every related value must match the applied preference, including recommendations.
        Ok(states.iter().all(|value| *value == self.enabled_value))
    }

    fn changes_in<'a>(&self, root: &'a RegKey, enabled: bool) -> Vec<RegistryChange<'a>> {
        self.values
            .iter()
            .map(|value| {
                let next = if !enabled && self.delete_when_disabled {
                    SettingValue::Delete
                } else {
                    SettingValue::Dword(u32::from(if enabled {
                        self.enabled_value
                    } else {
                        !self.enabled_value
                    }))
                };
                RegistryChange::new(root, self.path, value, next)
            })
            .collect()
    }

    fn toggle(&self, yes: bool) -> io::Result<()> {
        let current = self.enabled_in(HKCU)?;
        if !confirm_toggle(self.name, current, yes)? {
            return Ok(());
        }
        let changes = self.changes_in(HKCU, !current);
        let prepared = prepare_changes(&changes)?;
        apply_changes(&changes, &prepared)?;
        println!(
            "{} saved: {}",
            self.name,
            if current { "disabled" } else { "enabled" }
        );
        restart_explorer(self.name)
    }
}

pub fn toggle_hide_recent_files(yes: bool) -> io::Result<()> {
    HIDE_RECENT_FILES.toggle(yes)
}

pub fn toggle_hide_frequent_folders(yes: bool) -> io::Result<()> {
    HIDE_FREQUENT_FOLDERS.toggle(yes)
}

fn require_windows11(name: &str) -> io::Result<()> {
    if supports_windows11()? {
        Ok(())
    } else {
        Err(io::Error::other(format!("{name} requires Windows 11")))
    }
}

pub fn toggle_hide_office_files(yes: bool) -> io::Result<()> {
    require_windows11(HIDE_OFFICE_FILES.name)?;
    HIDE_OFFICE_FILES.toggle(yes)
}

fn toggle_namespace(name: &str, clsid: &str, yes: bool) -> io::Result<()> {
    require_windows11(name)?;
    let current = !namespace_visible(clsid)?;
    if !confirm_toggle(name, current, yes)? {
        return Ok(());
    }
    // Turning hiding off restores visibility in all three registry locations.
    let changes = namespace_changes(HKCU, HKLM, clsid, current);
    let prepared = prepare_changes(&changes)?;
    apply_changes(&changes, &prepared)?;
    println!(
        "{name} saved: {}",
        if current { "disabled" } else { "enabled" }
    );
    restart_explorer(name)
}

fn namespace_changes<'a>(
    user: &'a RegKey,
    machine: &'a RegKey,
    clsid: &str,
    visible: bool,
) -> [RegistryChange<'a>; 3] {
    let (pinned, non_enum, hidden) = namespace_paths(clsid);
    [
        RegistryChange::new(
            user,
            &pinned,
            "System.IsPinnedToNameSpaceTree",
            SettingValue::Dword(u32::from(visible)),
        ),
        RegistryChange::new(
            machine,
            &non_enum,
            clsid,
            SettingValue::Dword(u32::from(!visible)),
        ),
        RegistryChange::new(
            machine,
            &hidden,
            "HiddenByDefault",
            SettingValue::Dword(u32::from(!visible)),
        ),
    ]
}

pub fn toggle_hide_home_folder(yes: bool) -> io::Result<()> {
    toggle_namespace("Hide Home folder", HOME_CLSID, yes)
}

pub fn toggle_hide_gallery(yes: bool) -> io::Result<()> {
    toggle_namespace("Hide Gallery", GALLERY_CLSID, yes)
}

fn transparent_icon() -> Vec<u8> {
    // A 32x32, 32-bit ICO with transparent pixels and an all-transparent AND mask.
    // Build it locally instead of invoking PowerShell or downloading an asset.
    let image_size = 40 + 32 * 32 * 4 + 32 * 4;
    let mut bytes = vec![0, 0, 1, 0, 1, 0, 32, 32, 0, 0, 1, 0, 32, 0];
    bytes.extend_from_slice(&(image_size as u32).to_le_bytes());
    bytes.extend_from_slice(&22u32.to_le_bytes());
    for value in [40u32, 32, 64] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&[1, 0, 32, 0]);
    bytes.resize(22 + 40 + 32 * 32 * 4, 0);
    bytes.resize(22 + image_size, 0xff);
    bytes
}

fn ensure_blank_icon() -> io::Result<PathBuf> {
    let path = blank_icon_path()?;
    match OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(mut file) => {
            if let Err(error) = file.write_all(&transparent_icon()) {
                drop(file);
                // A failed creation must not leave a truncated icon for the next try.
                let _ = std::fs::remove_file(&path);
                return Err(setting_error(error));
            }
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists && path.is_file() => {}
        Err(error) => return Err(setting_error(error)),
    }
    Ok(path)
}

pub fn toggle_hide_shortcut_arrow(yes: bool) -> io::Result<()> {
    let name = "Hide shortcut arrow icon";
    let current = shortcut_arrows_hidden()?.unwrap_or(false);
    if !confirm_toggle(name, current, yes)? {
        return Ok(());
    }
    let mut changes = [RegistryChange::new(
        HKLM,
        SHELL_ICONS_KEY,
        "29",
        SettingValue::Delete,
    )];
    let prepared = prepare_changes(&changes)?;
    if !current {
        let path = ensure_blank_icon()?;
        changes[0].value = SettingValue::String(path.to_string_lossy().into_owned());
    }
    // Keep blank.ico when restoring arrows: deleting it invalidates the icon cache.
    apply_changes(&changes, &prepared)?;
    println!(
        "{name} saved: {}",
        if current { "disabled" } else { "enabled" }
    );
    restart_explorer(name)
}

fn restart_explorer(setting: &str) -> io::Result<()> {
    let stopped = ProcessCommand::new("taskkill")
        .creation_flags(0x08000000)
        .args(["/F", "/IM", "explorer.exe"])
        .output()?;
    if !stopped.status.success() {
        return Err(io::Error::other(format!(
            "{setting} saved, but Explorer could not be stopped: {}",
            String::from_utf8_lossy(&stopped.stderr).trim()
        )));
    }
    ProcessCommand::new("explorer.exe")
        .creation_flags(0x08000000)
        .spawn()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestRegistry {
        key: RegKey,
        path: String,
    }

    impl TestRegistry {
        fn new() -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = format!(r"Software\OrbitalTests\{}-{unique}", std::process::id());
            let (key, _) = HKCU.create_subkey(&path).unwrap();
            Self { key, path }
        }
    }

    impl Drop for TestRegistry {
        fn drop(&mut self) {
            HKCU.delete_subkey_all(&self.path).unwrap();
        }
    }

    #[test]
    fn missing_toggles_use_defaults_and_invalid_values_fail() {
        let registry = TestRegistry::new();
        assert!(read_toggle(&registry.key, "Missing", "ShowRecent", true).unwrap());
        assert!(!read_toggle(&registry.key, "Missing", "UseCompactMode", false).unwrap());
        let (key, _) = registry.key.create_subkey("Values").unwrap();
        assert!(read_toggle(&registry.key, "Values", "ShowRecent", true).unwrap());
        key.set_value("ShowRecent", &0u32).unwrap();
        assert!(!read_toggle(&registry.key, "Values", "ShowRecent", true).unwrap());
        key.set_value("ShowRecent", &1u32).unwrap();
        assert!(read_toggle(&registry.key, "Values", "ShowRecent", true).unwrap());
        key.set_value("ShowRecent", &2u32).unwrap();
        assert!(read_toggle(&registry.key, "Values", "ShowRecent", true).is_err());
        key.set_value("ShowRecent", &"wrong type").unwrap();
        assert!(read_toggle(&registry.key, "Values", "ShowRecent", true).is_err());
    }

    #[test]
    fn home_and_gallery_apply_all_overrides_and_respect_hide_flags() {
        let registry = TestRegistry::new();
        for clsid in [HOME_CLSID, GALLERY_CLSID] {
            assert!(namespace_visible_in(&registry.key, &registry.key, clsid).unwrap());
            for visible in [false, true] {
                let changes = namespace_changes(&registry.key, &registry.key, clsid, visible);
                let prepared = prepare_changes(&changes).unwrap();
                apply_changes(&changes, &prepared).unwrap();
                assert_eq!(
                    namespace_visible_in(&registry.key, &registry.key, clsid).unwrap(),
                    visible
                );
                let (pinned, non_enum, hidden) = namespace_paths(clsid);
                assert_eq!(
                    read_toggle(
                        &registry.key,
                        &pinned,
                        "System.IsPinnedToNameSpaceTree",
                        false
                    )
                    .unwrap(),
                    visible
                );
                assert_eq!(
                    read_toggle(&registry.key, &non_enum, clsid, false).unwrap(),
                    !visible
                );
                assert_eq!(
                    read_toggle(&registry.key, &hidden, "HiddenByDefault", false).unwrap(),
                    !visible
                );
            }
            let (_, non_enum, hidden) = namespace_paths(clsid);
            let key = registry
                .key
                .open_subkey_with_flags(&non_enum, KEY_QUERY_VALUE | KEY_SET_VALUE)
                .unwrap();
            key.set_value(clsid, &1u32).unwrap();
            assert!(!namespace_visible_in(&registry.key, &registry.key, clsid).unwrap());
            key.delete_value(clsid).unwrap();
            let key = registry
                .key
                .open_subkey_with_flags(&hidden, KEY_QUERY_VALUE | KEY_SET_VALUE)
                .unwrap();
            key.set_value("HiddenByDefault", &1u32).unwrap();
            assert!(!namespace_visible_in(&registry.key, &registry.key, clsid).unwrap());
        }
    }

    #[test]
    fn failed_grouped_write_restores_raw_values_and_missing_values() {
        let registry = TestRegistry::new();
        let (key, _) = registry.key.create_subkey("Values").unwrap();
        key.set_value("Existing", &"custom original value").unwrap();
        let original = key.get_raw_value("Existing").unwrap();
        let changes = [
            RegistryChange::new(&registry.key, "Values", "Existing", SettingValue::Dword(0)),
            RegistryChange::new(&registry.key, "Values", "Missing", SettingValue::Dword(1)),
            RegistryChange::new(&registry.key, "Values", "Failure", SettingValue::Dword(1)),
        ];
        let mut prepared = prepare_changes(&changes).unwrap();
        // Simulate a write failure after two successful writes using a read-only handle.
        prepared[2].0 = registry
            .key
            .open_subkey_with_flags("Values", KEY_QUERY_VALUE)
            .unwrap();
        assert!(apply_changes(&changes, &prepared).is_err());
        let restored = key.get_raw_value("Existing").unwrap();
        assert_eq!(restored.bytes, original.bytes);
        assert_eq!(restored.vtype, original.vtype);
        assert_eq!(
            key.get_raw_value("Missing").unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
    }

    #[test]
    fn dword_preferences_use_applied_polarity_and_restore_their_defaults() {
        let registry = TestRegistry::new();
        for preference in [
            &DISABLE_TASKBAR_CENTERING,
            &DISABLE_EXPLORER_SPACING,
            &HIDE_RECENT_FILES,
            &HIDE_FREQUENT_FOLDERS,
            &HIDE_OFFICE_FILES,
        ] {
            assert!(
                !preference.enabled_in(&registry.key).unwrap(),
                "{} default",
                preference.name
            );
            for enabled in [true, false] {
                let changes = preference.changes_in(&registry.key, enabled);
                let prepared = prepare_changes(&changes).unwrap();
                apply_changes(&changes, &prepared).unwrap();
                assert_eq!(
                    preference.enabled_in(&registry.key).unwrap(),
                    enabled,
                    "{}",
                    preference.name
                );
                let key = registry.key.open_subkey(preference.path).unwrap();
                for value in preference.values {
                    if !enabled && preference.delete_when_disabled {
                        assert_eq!(
                            key.get_raw_value(value).unwrap_err().kind(),
                            io::ErrorKind::NotFound
                        );
                    } else {
                        let actual: u32 = key.get_value(value).unwrap();
                        assert_eq!(
                            actual,
                            u32::from(if enabled {
                                preference.enabled_value
                            } else {
                                !preference.enabled_value
                            })
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn partial_recent_file_hiding_is_not_reported_as_enabled() {
        let registry = TestRegistry::new();
        let (key, _) = registry.key.create_subkey(EXPLORER_KEY).unwrap();
        key.set_value("ShowRecent", &0u32).unwrap();
        key.set_value("ShowRecommendations", &1u32).unwrap();
        assert!(!HIDE_RECENT_FILES.enabled_in(&registry.key).unwrap());
        let changes = HIDE_RECENT_FILES.changes_in(&registry.key, true);
        let prepared = prepare_changes(&changes).unwrap();
        apply_changes(&changes, &prepared).unwrap();
        assert!(HIDE_RECENT_FILES.enabled_in(&registry.key).unwrap());
    }

    #[test]
    fn shortcut_hiding_distinguishes_default_blank_and_custom_overlays() {
        struct TestIcon(PathBuf);
        impl Drop for TestIcon {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
            }
        }
        let registry = TestRegistry::new();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let icon = TestIcon(
            std::env::temp_dir().join(format!("orbital-test-{}-{unique}.ico", std::process::id())),
        );
        std::fs::write(&icon.0, transparent_icon()).unwrap();
        assert_eq!(
            shortcut_arrows_hidden_in(&registry.key, &icon.0).unwrap(),
            Some(false)
        );
        let (key, _) = registry.key.create_subkey(SHELL_ICONS_KEY).unwrap();
        key.set_value("29", &r"C:\custom-overlay.ico").unwrap();
        assert_eq!(
            shortcut_arrows_hidden_in(&registry.key, &icon.0).unwrap(),
            None
        );
        key.set_value("29", &icon.0.to_string_lossy().as_ref())
            .unwrap();
        assert_eq!(
            shortcut_arrows_hidden_in(&registry.key, &icon.0).unwrap(),
            Some(true)
        );
        delete_value_if_present(&key, "29").unwrap();
        assert!(icon.0.is_file());
        assert_eq!(
            shortcut_arrows_hidden_in(&registry.key, &icon.0).unwrap(),
            Some(false)
        );
        key.set_value("29", &icon.0.to_string_lossy().as_ref())
            .unwrap();
        std::fs::remove_file(&icon.0).unwrap();
        assert_eq!(
            shortcut_arrows_hidden_in(&registry.key, &icon.0).unwrap(),
            None
        );
    }
    #[test]
    fn transparent_ico_has_valid_headers_pixels_and_mask() {
        let icon = transparent_icon();
        let dword = |offset| u32::from_le_bytes(icon[offset..offset + 4].try_into().unwrap());
        assert_eq!(&icon[..6], &[0, 0, 1, 0, 1, 0]);
        assert_eq!(dword(18), 22);
        assert_eq!(dword(14) as usize + 22, icon.len());
        assert_eq!(dword(22), 40);
        assert_eq!(dword(26), 32);
        assert_eq!(dword(30), 64);
        assert!(icon[62..62 + 4096].iter().all(|byte| *byte == 0));
        assert!(icon[62 + 4096..].iter().all(|byte| *byte == 0xff));
    }
}
