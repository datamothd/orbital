#[path = "explorer/desktop_layout.rs"]
mod desktop_layout;
#[path = "explorer/folder_views.rs"]
mod folder_views;
#[path = "explorer/profiles.rs"]
mod profiles;

pub use folder_views::{FolderGrouping, FolderSort, SortColumn};
pub use profiles::{
    SettingsCommand, TweakProfile, apply_batch, apply_imported_tweaks, export_tweaks,
};

use std::os::windows::process::CommandExt;
use std::{
    fs::OpenOptions,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
};
use winreg::{
    HKCU, HKLM, RegKey, RegValue,
    enums::{KEY_QUERY_VALUE, KEY_SET_VALUE},
    types::ToRegValue,
};

const CLASSIC_CONTEXT_MENU_KEY: &str =
    r"Software\Classes\CLSID\{86ca1aa0-34aa-4e8b-a509-50c905bae2a2}\InprocServer32";
const EXPLORER_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer";
const APP_SETTINGS_KEY: &str = r"Software\Orbital\Settings";
const ADVANCED_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced";
const TASKBAR_DEVELOPER_KEY: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced\TaskbarDeveloperSettings";
const WIDGETS_KEY: &str = r"SOFTWARE\Policies\Microsoft\Dsh";
const SHELL_ICONS_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Shell Icons";
const HOME_CLSID: &str = "{f874310e-b6b7-47dc-bc84-b9e6b38f5903}";
const GALLERY_CLSID: &str = "{e88865ea-0e1c-4e20-9aa6-edcd0212c87c}";

fn desktop_layout_safe_mode_in(user: &RegKey) -> io::Result<bool> {
    read_toggle(user, APP_SETTINGS_KEY, "DesktopLayoutSafeMode", false)
}

fn protect_desktop_layout_in(user: &RegKey, folder_views_changed: bool) -> io::Result<bool> {
    Ok(folder_views_changed || desktop_layout_safe_mode_in(user)?)
}

pub fn set_desktop_layout_safe_mode(enabled: bool) -> io::Result<()> {
    let (key, _) = HKCU.create_subkey(APP_SETTINGS_KEY)?;
    key.set_value("DesktopLayoutSafeMode", &u32::from(enabled))
}

fn is_administrator() -> io::Result<bool> {
    use std::ffi::c_void;

    #[repr(C)]
    struct AdministratorSid {
        revision: u8,
        subauthority_count: u8,
        authority: [u8; 6],
        subauthorities: [u32; 2],
    }

    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn CheckTokenMembership(token: *mut c_void, sid: *const c_void, member: *mut i32) -> i32;
    }

    let sid = AdministratorSid {
        revision: 1,
        subauthority_count: 2,
        authority: [0, 0, 0, 0, 0, 5],
        subauthorities: [32, 544],
    };
    let mut member = 0;
    if unsafe {
        CheckTokenMembership(
            std::ptr::null_mut(),
            std::ptr::from_ref(&sid).cast(),
            &mut member,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(member != 0)
}

fn windows_build() -> io::Result<u32> {
    let key = HKLM.open_subkey_with_flags(
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
        KEY_QUERY_VALUE,
    )?;
    let build: String = key.get_value("CurrentBuildNumber")?;
    build
        .parse()
        .map_err(|_| io::Error::other("Invalid Windows build number"))
}

fn supports_windows11() -> io::Result<bool> {
    Ok(windows_build()? >= 22000)
}

fn supports_taskbar_end_task(build: u32) -> io::Result<bool> {
    if build >= 22631 {
        return Ok(true);
    }
    if build != 22621 {
        return Ok(false);
    }
    let key = HKLM.open_subkey_with_flags(
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
        KEY_QUERY_VALUE,
    )?;
    Ok(optional(key.get_value::<u32, _>("UBR"))?.is_some_and(|revision| revision >= 3958))
}

fn classic_context_menu_enabled() -> io::Result<bool> {
    let Some(key) =
        optional(HKCU.open_subkey_with_flags(CLASSIC_CONTEXT_MENU_KEY, KEY_QUERY_VALUE))?
    else {
        return Ok(false);
    };
    Ok(optional(key.get_value::<String, _>(""))?.is_some_and(|value| value.is_empty()))
}

#[derive(Default, serde::Serialize)]
pub struct ExplorerSettings {
    pub is_administrator: bool,
    pub desktop_layout_safe_mode: bool,
    pub folder_sort: Option<FolderSort>,
    pub folder_grouping: Option<FolderGrouping>,
    pub disable_taskbar_centering: bool,
    pub hide_task_view: bool,
    pub disable_widgets: bool,
    pub enable_taskbar_end_task: bool,
    pub taskbar_end_task_supported: bool,
    pub disable_explorer_spacing: bool,
    pub disable_modern_context_menu: bool,
    pub windows11_supported: bool,
    pub hide_recent_files: bool,
    pub hide_recommendations: bool,
    pub hide_frequent_folders: bool,
    pub hide_office_files: bool,
    pub hide_home_folder: bool,
    pub home_supported: bool,
    pub hide_gallery: bool,
    pub gallery_supported: bool,
    pub hide_shortcut_arrow: Option<bool>,
}

pub fn settings() -> io::Result<ExplorerSettings> {
    let build = windows_build()?;
    let windows11_supported = build >= 22000;
    let home_supported = windows11_supported && namespace_supported_in(HKLM, HOME_CLSID)?;
    let gallery_supported = windows11_supported && namespace_supported_in(HKLM, GALLERY_CLSID)?;
    let taskbar_end_task_supported = supports_taskbar_end_task(build)?;
    Ok(ExplorerSettings {
        is_administrator: is_administrator()?,
        desktop_layout_safe_mode: desktop_layout_safe_mode_in(HKCU)?,
        folder_sort: folder_views::current()?,
        folder_grouping: folder_views::current_grouping()?,
        disable_taskbar_centering: DISABLE_TASKBAR_CENTERING.enabled_in(HKCU)?,
        hide_task_view: HIDE_TASK_VIEW.enabled_in(HKCU)?,
        disable_widgets: windows11_supported && DISABLE_WIDGETS.enabled_in(HKLM)?,
        enable_taskbar_end_task: taskbar_end_task_supported
            && ENABLE_TASKBAR_END_TASK.enabled_in(HKCU)?,
        taskbar_end_task_supported,
        disable_explorer_spacing: windows11_supported
            && DISABLE_EXPLORER_SPACING.enabled_in(HKCU)?,
        disable_modern_context_menu: windows11_supported && classic_context_menu_enabled()?,
        windows11_supported,
        hide_recent_files: HIDE_RECENT_FILES.enabled_in(HKCU)?,
        hide_recommendations: windows11_supported && HIDE_RECOMMENDATIONS.enabled_in(HKCU)?,
        hide_frequent_folders: HIDE_FREQUENT_FOLDERS.enabled_in(HKCU)?,
        hide_office_files: windows11_supported && HIDE_OFFICE_FILES.enabled_in(HKCU)?,
        hide_home_folder: home_supported && !namespace_visible_in(HKCU, HKLM, HOME_CLSID)?,
        home_supported,
        hide_gallery: gallery_supported && !namespace_visible_in(HKCU, HKLM, GALLERY_CLSID)?,
        gallery_supported,
        hide_shortcut_arrow: shortcut_arrows_hidden_in(HKLM, &blank_icon_path()?)?,
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

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, clap::Subcommand, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Preference {
    DisableTaskbarCentering,
    HideTaskView,
    DisableWidgets,
    EnableTaskbarEndTask,
    DisableExplorerSpacing,
    DisableModernContextMenu,
    HideRecentFiles,
    HideRecommendations,
    HideFrequentFolders,
    HideOfficeFiles,
    HideHomeFolder,
    HideGallery,
    HideShortcutArrow,
}

pub fn toggle_preference(
    preference: Preference,
    yes: bool,
    replace_blank_icon: bool,
) -> io::Result<()> {
    let (setting, root) = match preference {
        Preference::DisableTaskbarCentering => (&DISABLE_TASKBAR_CENTERING, HKCU),
        Preference::HideTaskView => (&HIDE_TASK_VIEW, HKCU),
        Preference::DisableWidgets => (&DISABLE_WIDGETS, HKLM),
        Preference::EnableTaskbarEndTask => (&ENABLE_TASKBAR_END_TASK, HKCU),
        Preference::DisableExplorerSpacing => (&DISABLE_EXPLORER_SPACING, HKCU),
        Preference::HideRecentFiles => (&HIDE_RECENT_FILES, HKCU),
        Preference::HideRecommendations => (&HIDE_RECOMMENDATIONS, HKCU),
        Preference::HideFrequentFolders => (&HIDE_FREQUENT_FOLDERS, HKCU),
        Preference::HideOfficeFiles => (&HIDE_OFFICE_FILES, HKCU),
        Preference::DisableModernContextMenu => return toggle_modern_context_menu(yes),
        Preference::HideHomeFolder => return toggle_namespace("Hide Home folder", HOME_CLSID, yes),
        Preference::HideGallery => return toggle_namespace("Hide Gallery", GALLERY_CLSID, yes),
        Preference::HideShortcutArrow => return toggle_shortcut_arrow(yes, replace_blank_icon),
    };
    match preference {
        Preference::HideTaskView
        | Preference::HideRecentFiles
        | Preference::HideFrequentFolders => {}
        Preference::EnableTaskbarEndTask => {
            if !supports_taskbar_end_task(windows_build()?)? {
                return Err(io::Error::other(
                    "Taskbar End task is unavailable on this system",
                ));
            }
        }
        _ => require_windows11(setting.name)?,
    }
    setting.toggle(root, yes)
}

fn toggle_modern_context_menu(yes: bool) -> io::Result<()> {
    let name = "Disable Windows 11 context menu";
    require_windows11(name)?;
    let current = classic_context_menu_enabled()?;
    let changes = [RegistryChange::new(
        HKCU,
        CLASSIC_CONTEXT_MENU_KEY,
        "",
        (!current).then(|| "".to_reg_value()),
    )];
    apply_toggle(name, current, yes, &changes)
}

fn read_toggle(root: &RegKey, path: &str, name: &str, default: bool) -> io::Result<bool> {
    let Some(key) = optional(root.open_subkey_with_flags(path, KEY_QUERY_VALUE))? else {
        return Ok(default);
    };
    match optional(key.get_value::<u32, _>(name))?.unwrap_or(u32::from(default)) {
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

fn namespace_supported_in(machine: &RegKey, clsid: &str) -> io::Result<bool> {
    Ok(optional(
        machine.open_subkey_with_flags(format!(r"Software\Classes\CLSID\{clsid}"), KEY_QUERY_VALUE),
    )?
    .is_some())
}

fn namespace_visible_in(user: &RegKey, machine: &RegKey, clsid: &str) -> io::Result<bool> {
    let (pinned, non_enum, hidden) = namespace_paths(clsid);
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
    Ok(root.join("orbital-shortcut.ico"))
}

fn shortcut_arrows_hidden_in(root: &RegKey, blank_path: &Path) -> io::Result<Option<bool>> {
    let Some(key) = optional(root.open_subkey_with_flags(SHELL_ICONS_KEY, KEY_QUERY_VALUE))? else {
        return Ok(Some(false));
    };
    let Some(overlay) = optional(key.get_value::<String, _>("29"))? else {
        return Ok(Some(false));
    };
    if overlay.eq_ignore_ascii_case(&blank_path.to_string_lossy()) && blank_icon_valid(blank_path)?
    {
        return Ok(Some(true));
    }
    Ok(None)
}

struct RegistryChange<'a> {
    root: &'a RegKey,
    path: String,
    name: String,
    value: Option<RegValue<'static>>,
}

impl<'a> RegistryChange<'a> {
    fn new(root: &'a RegKey, path: &str, name: &str, value: Option<RegValue<'_>>) -> Self {
        Self {
            root,
            path: path.into(),
            name: name.into(),
            value: value.map(|value| RegValue {
                bytes: value.bytes.into_owned().into(),
                vtype: value.vtype,
            }),
        }
    }
}

fn delete_value_if_present(key: &RegKey, name: &str) -> io::Result<()> {
    optional(key.delete_value(name)).map(|_| ())
}

fn optional<T>(result: io::Result<T>) -> io::Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn write_value(key: &RegKey, name: &str, value: Option<&RegValue<'_>>) -> io::Result<()> {
    match value {
        Some(value) => key.set_raw_value(name, value),
        None => delete_value_if_present(key, name),
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
            let (key, _) = change
                .root
                .create_subkey_with_flags(&change.path, KEY_QUERY_VALUE | KEY_SET_VALUE)?;
            let previous = optional(key.get_raw_value(&change.name))?;
            Ok((key, previous))
        })
        .collect::<io::Result<_>>()
        .map_err(setting_error)
}

fn apply_changes(changes: &[RegistryChange<'_>], prepared: &[RegistrySnapshot]) -> io::Result<()> {
    for (index, (change, (key, _))) in changes.iter().zip(prepared).enumerate() {
        if let Err(error) = write_value(key, &change.name, change.value.as_ref()) {
            let mut failures = Vec::new();
            for (change, (key, previous)) in changes[..index].iter().zip(&prepared[..index]).rev() {
                if let Err(error) = write_value(key, &change.name, previous.as_ref()) {
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

fn apply_toggle(
    name: &str,
    current: bool,
    yes: bool,
    changes: &[RegistryChange<'_>],
) -> io::Result<()> {
    if !confirm_toggle(name, current, yes)? {
        return Ok(());
    }
    save_original_settings()?;
    let prepared = prepare_changes(changes)?;
    apply_changes(changes, &prepared)?;
    println!(
        "{name} saved: {}",
        if current { "disabled" } else { "enabled" }
    );
    restart_explorer(name, false, false)
}

struct DwordPreference {
    name: &'static str,
    path: &'static str,
    value: &'static str,
    enabled_value: bool,
    delete_when_disabled: bool,
}

const DISABLE_TASKBAR_CENTERING: DwordPreference = DwordPreference {
    name: "Disable centered taskbar icons",
    path: ADVANCED_KEY,
    value: "TaskbarAl",
    enabled_value: false,
    delete_when_disabled: false,
};
const HIDE_TASK_VIEW: DwordPreference = DwordPreference {
    name: "Hide Task View button",
    path: ADVANCED_KEY,
    value: "ShowTaskViewButton",
    enabled_value: false,
    delete_when_disabled: false,
};
const DISABLE_WIDGETS: DwordPreference = DwordPreference {
    name: "Disable Widgets",
    path: WIDGETS_KEY,
    value: "AllowNewsAndInterests",
    enabled_value: false,
    delete_when_disabled: true,
};
const ENABLE_TASKBAR_END_TASK: DwordPreference = DwordPreference {
    name: "Enable taskbar End task",
    path: TASKBAR_DEVELOPER_KEY,
    value: "TaskbarEndTask",
    enabled_value: true,
    delete_when_disabled: false,
};
const DISABLE_EXPLORER_SPACING: DwordPreference = DwordPreference {
    name: "Disable extra spacing",
    path: ADVANCED_KEY,
    value: "UseCompactMode",
    enabled_value: true,
    delete_when_disabled: false,
};
const HIDE_RECENT_FILES: DwordPreference = DwordPreference {
    name: "Hide recently used files",
    path: EXPLORER_KEY,
    value: "ShowRecent",
    enabled_value: false,
    delete_when_disabled: false,
};
const HIDE_RECOMMENDATIONS: DwordPreference = DwordPreference {
    name: "Hide recommendations",
    path: EXPLORER_KEY,
    value: "ShowRecommendations",
    enabled_value: false,
    delete_when_disabled: false,
};
const HIDE_FREQUENT_FOLDERS: DwordPreference = DwordPreference {
    name: "Hide frequently used folders",
    path: EXPLORER_KEY,
    value: "ShowFrequent",
    enabled_value: false,
    delete_when_disabled: false,
};
const HIDE_OFFICE_FILES: DwordPreference = DwordPreference {
    name: "Hide files from Office.com",
    path: EXPLORER_KEY,
    value: "ShowCloudFilesInQuickAccess",
    enabled_value: false,
    delete_when_disabled: true,
};

impl DwordPreference {
    fn enabled_in(&self, root: &RegKey) -> io::Result<bool> {
        Ok(read_toggle(root, self.path, self.value, !self.enabled_value)? == self.enabled_value)
    }

    fn change_in<'a>(&self, root: &'a RegKey, enabled: bool) -> RegistryChange<'a> {
        let next_value = u32::from(enabled == self.enabled_value);
        let value = if !enabled && self.delete_when_disabled {
            None
        } else {
            Some(next_value.to_reg_value())
        };
        RegistryChange::new(root, self.path, self.value, value)
    }

    fn toggle(&self, root: &RegKey, yes: bool) -> io::Result<()> {
        let current = self.enabled_in(root)?;
        apply_toggle(self.name, current, yes, &[self.change_in(root, !current)])
    }
}

fn require_windows11(name: &str) -> io::Result<()> {
    if supports_windows11()? {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "{name} is unavailable on this system"
        )))
    }
}

fn toggle_namespace(name: &str, clsid: &str, yes: bool) -> io::Result<()> {
    require_windows11(name)?;
    if !namespace_supported_in(HKLM, clsid)? {
        return Err(io::Error::other(format!(
            "{name} is unavailable on this system"
        )));
    }
    let current = !namespace_visible_in(HKCU, HKLM, clsid)?;
    let changes = namespace_changes(HKCU, HKLM, clsid, current);
    apply_toggle(name, current, yes, &changes)
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
            Some(u32::from(visible).to_reg_value()),
        ),
        RegistryChange::new(
            machine,
            &non_enum,
            clsid,
            Some(u32::from(!visible).to_reg_value()),
        ),
        RegistryChange::new(
            machine,
            &hidden,
            "HiddenByDefault",
            Some(u32::from(!visible).to_reg_value()),
        ),
    ]
}

fn settings_changes<'a>(
    user: &'a RegKey,
    machine: &'a RegKey,
    windows11: bool,
) -> Vec<RegistryChange<'a>> {
    let mut changes = Vec::new();
    for preference in [
        &DISABLE_TASKBAR_CENTERING,
        &DISABLE_EXPLORER_SPACING,
        &HIDE_RECENT_FILES,
        &HIDE_RECOMMENDATIONS,
        &HIDE_FREQUENT_FOLDERS,
    ] {
        changes.push(preference.change_in(user, true));
    }
    if windows11 {
        changes.push(HIDE_OFFICE_FILES.change_in(user, true));
        changes.push(RegistryChange::new(
            user,
            CLASSIC_CONTEXT_MENU_KEY,
            "",
            Some("".to_reg_value()),
        ));
        for clsid in [HOME_CLSID, GALLERY_CLSID] {
            changes.extend(namespace_changes(user, machine, clsid, false));
        }
    }
    changes.push(RegistryChange::new(machine, SHELL_ICONS_KEY, "29", None));
    changes.push(HIDE_TASK_VIEW.change_in(user, true));
    if windows11 {
        changes.push(DISABLE_WIDGETS.change_in(machine, true));
        changes.push(ENABLE_TASKBAR_END_TASK.change_in(user, true));
    }
    changes
}

fn recommended_changes<'a>(
    user: &'a RegKey,
    machine: &'a RegKey,
    windows11: bool,
    icon: &Path,
) -> io::Result<Vec<RegistryChange<'a>>> {
    let mut changes = settings_changes(user, machine, windows11);
    changes.retain(|change| {
        !(change.path == ADVANCED_KEY && change.name == "TaskbarAl"
            || change.path == SHELL_ICONS_KEY && change.name == "29")
    });
    if windows11 {
        for clsid in [HOME_CLSID, GALLERY_CLSID] {
            if !namespace_supported_in(machine, clsid)?
                || !namespace_visible_in(user, machine, clsid)?
            {
                changes.retain(|change| change.name != clsid && !change.path.ends_with(clsid));
            }
        }
    }
    if !windows11 {
        changes.retain(|change| {
            !matches!(
                change.name.as_str(),
                "UseCompactMode" | "ShowRecommendations"
            )
        });
    }
    if shortcut_arrows_hidden_in(machine, icon)? != Some(true) {
        changes.push(RegistryChange::new(
            machine,
            SHELL_ICONS_KEY,
            "29",
            Some(icon.as_os_str().to_reg_value()),
        ));
    }
    Ok(changes)
}

fn read_original(root: &RegKey, path: &str, name: &str) -> io::Result<Option<RegValue<'static>>> {
    let Some(key) = optional(root.open_subkey_with_flags(path, KEY_QUERY_VALUE))? else {
        return Ok(None);
    };
    optional(key.get_raw_value(name))
}

fn original_settings_in(user: &RegKey, machine: &RegKey, windows11: bool) -> io::Result<RegKey> {
    const BACKUP_KEY: &str = r"Software\Orbital\OriginalExplorerSettings";
    let (windows11, saved_count) = if read_original(user, BACKUP_KEY, "Windows11")?.is_some() {
        let key = user.open_subkey(BACKUP_KEY)?;
        let windows11 = read_toggle(&key, "", "Windows11", false)?;
        let count = optional(key.get_value::<u32, _>("ValueCount"))?.unwrap_or(if windows11 {
            14
        } else {
            6
        });
        (windows11, count as usize)
    } else {
        (windows11, 0)
    };
    let changes = settings_changes(user, machine, windows11);
    if saved_count > changes.len() {
        return Err(io::Error::other(
            "Original settings backup has an unsupported version",
        ));
    }
    if saved_count == changes.len() {
        return user.open_subkey(BACKUP_KEY);
    }
    let originals = changes
        .iter()
        .skip(saved_count)
        .map(|change| read_original(change.root, &change.path, &change.name))
        .collect::<io::Result<Vec<_>>>()?;
    let (key, _) = user.create_subkey(BACKUP_KEY)?;
    for (index, value) in originals.iter().enumerate() {
        let name = (saved_count + index).to_string();
        write_value(&key, &name, value.as_ref())?;
    }
    key.set_value("Windows11", &u32::from(windows11))?;
    key.set_value("ValueCount", &(changes.len() as u32))?;
    Ok(key)
}

pub fn save_original_settings() -> io::Result<()> {
    original_settings_in(HKCU, HKLM, supports_windows11()?).map(|_| ())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Recommendation {
    HideTaskView,
    DisableWidgets,
    EnableTaskbarEndTask,
    DisableExplorerSpacing,
    DisableModernContextMenu,
    HideRecentFiles,
    HideRecommendations,
    HideFrequentFolders,
    HideOfficeFiles,
    HideHomeFolder,
    HideGallery,
    HideShortcutArrow,
    FolderSort,
    FolderGrouping,
}

pub fn apply_recommended(yes: bool) -> io::Result<()> {
    apply_batch(&[SettingsCommand::Recommended], yes)
}

pub fn apply_recommended_settings(
    preview: bool,
    selected: Option<&[Recommendation]>,
    replace_blank_icon: bool,
) -> io::Result<Vec<Recommendation>> {
    let build = windows_build()?;
    let windows11 = build >= 22000;
    let icon = blank_icon_path()?;
    let selected = |preference| selected.is_none_or(|items| items.contains(&preference));
    let mut changes = pending_changes(recommended_changes(HKCU, HKLM, windows11, &icon)?)?;
    if !supports_taskbar_end_task(build)? {
        changes.retain(|change| change.name != "TaskbarEndTask");
    }
    changes.retain(|change| recommendation(change).is_some_and(selected));
    let mut view_changes = folder_views::recommended_changes()?;
    let view_id = |change: &folder_views::ViewChange| match change {
        folder_views::ViewChange::Sort(_) => Recommendation::FolderSort,
        folder_views::ViewChange::Grouping(_) => Recommendation::FolderGrouping,
    };
    view_changes.retain(|change| selected(view_id(change)));
    let missing_icon = selected(Recommendation::HideShortcutArrow) && !blank_icon_valid(&icon)?;
    let mut preferences = changed_preferences(&changes, missing_icon);
    preferences.extend(view_changes.iter().map(view_id));
    if preview || preferences.is_empty() {
        return Ok(preferences);
    }
    original_settings_in(HKCU, HKLM, windows11)?;
    let prepared = prepare_changes(&changes)?;
    if missing_icon {
        ensure_blank_icon_at(&icon, replace_blank_icon)?;
    }
    folder_views::apply_recommended(&view_changes, || apply_changes(&changes, &prepared))?;
    restart_explorer(
        "Recommended settings",
        preferences.contains(&Recommendation::HideShortcutArrow),
        !view_changes.is_empty(),
    )?;
    Ok(preferences)
}

fn recommendation(change: &RegistryChange<'_>) -> Option<Recommendation> {
    use Recommendation::*;
    Some(match change.name.as_str() {
        "ShowTaskViewButton" => HideTaskView,
        "AllowNewsAndInterests" => DisableWidgets,
        "TaskbarEndTask" => EnableTaskbarEndTask,
        "UseCompactMode" => DisableExplorerSpacing,
        "ShowRecent" => HideRecentFiles,
        "ShowRecommendations" => HideRecommendations,
        "ShowFrequent" => HideFrequentFolders,
        "ShowCloudFilesInQuickAccess" => HideOfficeFiles,
        "" if change.path == CLASSIC_CONTEXT_MENU_KEY => DisableModernContextMenu,
        "29" if change.path == SHELL_ICONS_KEY => HideShortcutArrow,
        _ if change.name == HOME_CLSID || change.path.ends_with(HOME_CLSID) => HideHomeFolder,
        _ if change.name == GALLERY_CLSID || change.path.ends_with(GALLERY_CLSID) => HideGallery,
        _ => return None,
    })
}

fn changed_preferences(changes: &[RegistryChange<'_>], missing_icon: bool) -> Vec<Recommendation> {
    let mut preferences = Vec::new();
    for preference in changes
        .iter()
        .filter_map(recommendation)
        .chain(missing_icon.then_some(Recommendation::HideShortcutArrow))
    {
        if !preferences.contains(&preference) {
            preferences.push(preference);
        }
    }
    preferences
}

fn pending_changes(changes: Vec<RegistryChange<'_>>) -> io::Result<Vec<RegistryChange<'_>>> {
    let mut pending = Vec::new();
    for change in changes {
        let current = read_original(change.root, &change.path, &change.name)?;
        if current != change.value {
            pending.push(change);
        }
    }
    Ok(pending)
}

fn original_changes<'a>(
    user: &'a RegKey,
    machine: &'a RegKey,
    backup: &RegKey,
) -> io::Result<Vec<RegistryChange<'a>>> {
    let windows11 = read_toggle(backup, "", "Windows11", false)?;
    let mut changes = settings_changes(user, machine, windows11);
    for (index, change) in changes.iter_mut().enumerate() {
        change.value = read_original(backup, "", &index.to_string())?;
    }
    Ok(changes)
}

pub fn restore_original_settings() -> io::Result<bool> {
    let backup = original_settings_in(HKCU, HKLM, supports_windows11()?)?;
    let changes = pending_changes(original_changes(HKCU, HKLM, &backup)?)?;
    let prepared = prepare_changes(&changes)?;
    let views_restored = folder_views::restore()?;
    if changes.is_empty() && !views_restored {
        return Ok(false);
    }
    apply_changes(&changes, &prepared)?;
    restart_explorer(
        "Original settings",
        changes
            .iter()
            .any(|change| recommendation(change) == Some(Recommendation::HideShortcutArrow)),
        views_restored,
    )?;
    Ok(true)
}

pub fn set_folder_sort(sort: FolderSort) -> io::Result<()> {
    folder_views::apply(sort)?;
    restart_explorer("Folder sorting", false, true)
}

pub fn set_folder_grouping(grouping: FolderGrouping) -> io::Result<()> {
    folder_views::apply_grouping(grouping)?;
    restart_explorer("Folder grouping", false, true)
}

fn transparent_icon() -> Vec<u8> {
    let image_size = 40 + 32 * 32 * 4 + 32 * 4;
    let mut bytes = vec![0, 0, 1, 0, 1, 0, 32, 32, 0, 0, 1, 0, 32, 0];
    bytes.extend_from_slice(&(image_size as u32).to_le_bytes());
    bytes.extend_from_slice(&22u32.to_le_bytes());
    for value in [40u32, 32, 64] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&[1, 0, 32, 0]);
    bytes.resize(22 + 40 + 32 * 32 * 4, 0);
    bytes[42..46].copy_from_slice(&4096u32.to_le_bytes());
    for row in 0..4 {
        for column in 0..4 {
            bytes[65 + (row * 32 + column) * 4] = 1;
        }
    }
    bytes.resize(22 + image_size, 0xff);
    bytes
}

fn blank_icon_valid(path: &Path) -> io::Result<bool> {
    let Some(metadata) = optional(std::fs::metadata(path))? else {
        return Ok(false);
    };
    let expected = transparent_icon();
    if !metadata.is_file() || metadata.len() != expected.len() as u64 {
        return Ok(false);
    }
    Ok(optional(std::fs::read(path))?.is_some_and(|bytes| bytes == expected))
}

#[derive(Debug)]
pub struct BlankIconConflict(pub PathBuf);

impl std::fmt::Display for BlankIconConflict {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} already exists and is not Orbital's transparent icon. Replace, move or rename it before hiding shortcut arrows.",
            self.0.display()
        )
    }
}

impl std::error::Error for BlankIconConflict {}

fn ensure_blank_icon_at(path: &Path, replace: bool) -> io::Result<()> {
    match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(mut file) => {
            if let Err(error) = file.write_all(&transparent_icon()) {
                drop(file);
                let _ = std::fs::remove_file(path);
                return Err(setting_error(error));
            }
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            if !blank_icon_valid(path)? {
                use std::os::windows::fs::MetadataExt;
                let metadata = std::fs::symlink_metadata(path)?;
                if !metadata.is_file() || metadata.file_attributes() & 0x400 != 0 {
                    return Err(io::Error::other(format!(
                        "{} is not a regular file and cannot be replaced by Orbital.",
                        path.display()
                    )));
                }
                if !replace {
                    return Err(io::Error::other(BlankIconConflict(path.to_path_buf())));
                }
                std::fs::remove_file(path).map_err(setting_error)?;
                ensure_blank_icon_at(path, false)?;
            }
        }
        Err(error) => return Err(setting_error(error)),
    }
    Ok(())
}

fn toggle_shortcut_arrow(yes: bool, replace_blank_icon: bool) -> io::Result<()> {
    let name = "Hide shortcut arrow icon";
    let path = blank_icon_path()?;
    let current = shortcut_arrows_hidden_in(HKLM, &path)?.unwrap_or(false);
    if !confirm_toggle(name, current, yes)? {
        return Ok(());
    }
    save_original_settings()?;
    let mut changes = [RegistryChange::new(HKLM, SHELL_ICONS_KEY, "29", None)];
    let prepared = prepare_changes(&changes)?;
    if !current {
        ensure_blank_icon_at(&path, replace_blank_icon)?;
        changes[0] = RegistryChange::new(
            HKLM,
            SHELL_ICONS_KEY,
            "29",
            Some(path.as_os_str().to_reg_value()),
        );
    }
    apply_changes(&changes, &prepared)?;
    println!(
        "{name} saved: {}",
        if current { "disabled" } else { "enabled" }
    );
    restart_explorer(name, true, false)
}

fn clear_icon_cache(local: &Path) -> io::Result<()> {
    for entry in optional(std::fs::read_dir(local.join(r"Microsoft\Windows\Explorer")))?
        .into_iter()
        .flatten()
    {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("iconcache") && name.ends_with(".db") && entry.file_type()?.is_file() {
            optional(std::fs::remove_file(entry.path()))?;
        }
    }
    optional(std::fs::remove_file(local.join("IconCache.db"))).map(|_| ())
}

fn restart_explorer(
    setting: &str,
    rebuild_icons: bool,
    folder_views_changed: bool,
) -> io::Result<()> {
    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetShellWindow() -> *mut std::ffi::c_void;
        fn GetWindowThreadProcessId(window: *mut std::ffi::c_void, process: *mut u32) -> u32;
    }
    let desktop = if protect_desktop_layout_in(HKCU, folder_views_changed)? {
        Some(desktop_layout::DesktopLayout::capture()?)
    } else {
        None
    };
    let mut process = 0;
    unsafe { GetWindowThreadProcessId(GetShellWindow(), &mut process) };
    if process == 0 {
        return Err(io::Error::other(format!(
            "{setting} saved, but the desktop process could not be found."
        )));
    }
    let stopped = ProcessCommand::new("taskkill")
        .creation_flags(0x08000000)
        .args(["/F", "/PID", &process.to_string()])
        .output()?;
    if !stopped.status.success() {
        return Err(io::Error::other(format!(
            "{setting} saved, but Explorer could not be stopped: {}",
            String::from_utf8_lossy(&stopped.stderr).trim()
        )));
    }
    let cache_result = if rebuild_icons {
        std::env::var_os("LOCALAPPDATA")
            .ok_or_else(|| io::Error::other("LOCALAPPDATA is unavailable"))
            .and_then(|local| clear_icon_cache(Path::new(&local)))
    } else {
        Ok(())
    };
    ProcessCommand::new("explorer.exe")
        .creation_flags(0x08000000)
        .spawn()?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while unsafe { GetShellWindow() }.is_null() {
        if std::time::Instant::now() >= deadline {
            return Err(io::Error::other(format!(
                "{setting} saved, but Explorer did not finish restarting."
            )));
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    if let Some(desktop) = desktop {
        desktop.restore()?;
    }
    cache_result.map_err(|error| {
        io::Error::other(format!(
            "{setting} saved, but the icon cache could not be rebuilt: {error}"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn administrator_check_matches_windows() {
        #[link(name = "shell32")]
        unsafe extern "system" {
            fn IsUserAnAdmin() -> i32;
        }
        assert_eq!(is_administrator().unwrap(), unsafe { IsUserAnAdmin() } != 0);
    }

    pub(super) struct TestRegistry {
        pub(super) key: RegKey,
        path: String,
    }

    impl TestRegistry {
        pub(super) fn new() -> Self {
            static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = format!(
                r"Software\OrbitalTests\{}-{unique}-{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed)
            );
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
    fn desktop_layout_protection_is_automatic_for_folder_views_and_opt_in_for_other_changes() {
        let user = TestRegistry::new();
        assert!(!desktop_layout_safe_mode_in(&user.key).unwrap());
        assert!(!protect_desktop_layout_in(&user.key, false).unwrap());
        assert!(protect_desktop_layout_in(&user.key, true).unwrap());

        let (key, _) = user.key.create_subkey(APP_SETTINGS_KEY).unwrap();
        for enabled in [true, false] {
            key.set_value("DesktopLayoutSafeMode", &u32::from(enabled))
                .unwrap();
            assert_eq!(desktop_layout_safe_mode_in(&user.key).unwrap(), enabled);
            assert_eq!(
                protect_desktop_layout_in(&user.key, false).unwrap(),
                enabled
            );
            assert!(protect_desktop_layout_in(&user.key, true).unwrap());
        }
    }

    #[test]
    fn existing_backups_add_taskbar_values_without_replacing_saved_originals() {
        for windows11 in [false, true] {
            let user = TestRegistry::new();
            let machine = TestRegistry::new();
            let (advanced, _) = user.key.create_subkey(ADVANCED_KEY).unwrap();
            advanced.set_value("TaskbarAl", &1u32).unwrap();
            advanced.set_value("ShowTaskViewButton", &1u32).unwrap();
            user.key
                .create_subkey(TASKBAR_DEVELOPER_KEY)
                .unwrap()
                .0
                .set_value("TaskbarEndTask", &0u32)
                .unwrap();
            machine
                .key
                .create_subkey(WIDGETS_KEY)
                .unwrap()
                .0
                .set_value("AllowNewsAndInterests", &1u32)
                .unwrap();
            let changes = settings_changes(&user.key, &machine.key, windows11);
            let originals: Vec<_> = changes
                .iter()
                .map(|change| read_original(change.root, &change.path, &change.name).unwrap())
                .collect();
            let backup = user
                .key
                .create_subkey(r"Software\Orbital\OriginalExplorerSettings")
                .unwrap()
                .0;
            let legacy_count = if windows11 { 14 } else { 6 };
            for (index, original) in originals[..legacy_count].iter().enumerate() {
                write_value(&backup, &index.to_string(), original.as_ref()).unwrap();
            }
            backup
                .set_value("Windows11", &u32::from(windows11))
                .unwrap();
            advanced.set_value("TaskbarAl", &0u32).unwrap();
            let backup = original_settings_in(&user.key, &machine.key, windows11).unwrap();
            assert_eq!(
                backup.get_value::<u32, _>("ValueCount").unwrap() as usize,
                changes.len()
            );
            let prepared = prepare_changes(&changes).unwrap();
            apply_changes(&changes, &prepared).unwrap();
            original_settings_in(&user.key, &machine.key, windows11).unwrap();
            let restore = original_changes(&user.key, &machine.key, &backup).unwrap();
            let prepared = prepare_changes(&restore).unwrap();
            apply_changes(&restore, &prepared).unwrap();
            for (change, original) in changes.iter().zip(originals) {
                assert_eq!(
                    read_original(change.root, &change.path, &change.name).unwrap(),
                    original
                );
            }
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
    fn missing_namespaces_are_not_recommended_or_created() {
        let user = TestRegistry::new();
        let machine = TestRegistry::new();
        let changes =
            recommended_changes(&user.key, &machine.key, true, Path::new("unused.ico")).unwrap();
        for clsid in [HOME_CLSID, GALLERY_CLSID] {
            assert!(!namespace_supported_in(&machine.key, clsid).unwrap());
            assert!(
                changes
                    .iter()
                    .all(|change| change.name != clsid && !change.path.ends_with(clsid))
            );
            machine
                .key
                .create_subkey(format!(r"Software\Classes\CLSID\{clsid}"))
                .unwrap();
            assert!(namespace_supported_in(&machine.key, clsid).unwrap());
        }
    }

    #[test]
    fn failed_grouped_write_restores_raw_values_and_missing_values() {
        let registry = TestRegistry::new();
        let (key, _) = registry.key.create_subkey("Values").unwrap();
        key.set_value("Existing", &"custom original value").unwrap();
        let original = key.get_raw_value("Existing").unwrap();
        let changes = [
            RegistryChange::new(
                &registry.key,
                "Values",
                "Existing",
                Some(0u32.to_reg_value()),
            ),
            RegistryChange::new(
                &registry.key,
                "Values",
                "Missing",
                Some(1u32.to_reg_value()),
            ),
            RegistryChange::new(
                &registry.key,
                "Values",
                "Failure",
                Some(1u32.to_reg_value()),
            ),
        ];
        let mut prepared = prepare_changes(&changes).unwrap();
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
            &HIDE_TASK_VIEW,
            &DISABLE_WIDGETS,
            &ENABLE_TASKBAR_END_TASK,
            &DISABLE_EXPLORER_SPACING,
            &HIDE_RECENT_FILES,
            &HIDE_RECOMMENDATIONS,
            &HIDE_FREQUENT_FOLDERS,
            &HIDE_OFFICE_FILES,
        ] {
            assert!(
                !preference.enabled_in(&registry.key).unwrap(),
                "{} default",
                preference.name
            );
            for enabled in [true, false] {
                let changes = [preference.change_in(&registry.key, enabled)];
                let prepared = prepare_changes(&changes).unwrap();
                apply_changes(&changes, &prepared).unwrap();
                assert_eq!(
                    preference.enabled_in(&registry.key).unwrap(),
                    enabled,
                    "{}",
                    preference.name
                );
                let key = registry.key.open_subkey(preference.path).unwrap();
                if !enabled && preference.delete_when_disabled {
                    assert_eq!(
                        key.get_raw_value(preference.value).unwrap_err().kind(),
                        io::ErrorKind::NotFound
                    );
                } else {
                    let actual: u32 = key.get_value(preference.value).unwrap();
                    assert_eq!(actual, u32::from(enabled == preference.enabled_value));
                }
            }
        }
    }

    #[test]
    fn recent_files_and_recommendations_change_independently() {
        let registry = TestRegistry::new();
        let (key, _) = registry.key.create_subkey(EXPLORER_KEY).unwrap();
        key.set_value("ShowRecent", &0u32).unwrap();
        key.set_value("ShowRecommendations", &1u32).unwrap();
        assert!(HIDE_RECENT_FILES.enabled_in(&registry.key).unwrap());
        assert!(!HIDE_RECOMMENDATIONS.enabled_in(&registry.key).unwrap());
        let changes = [HIDE_RECOMMENDATIONS.change_in(&registry.key, true)];
        let prepared = prepare_changes(&changes).unwrap();
        apply_changes(&changes, &prepared).unwrap();
        assert!(HIDE_RECENT_FILES.enabled_in(&registry.key).unwrap());
        assert!(HIDE_RECOMMENDATIONS.enabled_in(&registry.key).unwrap());
    }

    #[test]
    fn recommendations_preserve_taskbar_alignment_and_are_repeatable() {
        let user = TestRegistry::new();
        let machine = TestRegistry::new();
        for clsid in [HOME_CLSID, GALLERY_CLSID] {
            machine
                .key
                .create_subkey(format!(r"Software\Classes\CLSID\{clsid}"))
                .unwrap();
        }
        for windows11 in [false, true] {
            let changes = recommended_changes(
                &user.key,
                &machine.key,
                windows11,
                Path::new("test-blank.ico"),
            )
            .unwrap();
            assert_eq!(changes.len(), if windows11 { 16 } else { 4 });
            let (key, _) = user.key.create_subkey(ADVANCED_KEY).unwrap();
            for alignment in [None, Some(0u32), Some(1u32)] {
                match alignment {
                    Some(value) => key.set_value("TaskbarAl", &value).unwrap(),
                    None => delete_value_if_present(&key, "TaskbarAl").unwrap(),
                }
                let prepared = prepare_changes(&changes).unwrap();
                apply_changes(&changes, &prepared).unwrap();
                assert_eq!(key.get_value::<u32, _>("TaskbarAl").ok(), alignment);
                let key = machine.key.open_subkey(SHELL_ICONS_KEY).unwrap();
                assert_eq!(key.get_value::<String, _>("29").unwrap(), "test-blank.ico");
                for preference in [&HIDE_RECENT_FILES, &HIDE_FREQUENT_FOLDERS] {
                    assert!(preference.enabled_in(&user.key).unwrap());
                }
                if windows11 {
                    assert!(DISABLE_EXPLORER_SPACING.enabled_in(&user.key).unwrap());
                    assert!(HIDE_RECOMMENDATIONS.enabled_in(&user.key).unwrap());
                    assert!(HIDE_OFFICE_FILES.enabled_in(&user.key).unwrap());
                    let key = user.key.open_subkey(CLASSIC_CONTEXT_MENU_KEY).unwrap();
                    assert_eq!(key.get_value::<String, _>("").unwrap(), "");
                    for clsid in [HOME_CLSID, GALLERY_CLSID] {
                        assert!(!namespace_visible_in(&user.key, &machine.key, clsid).unwrap());
                    }
                }
            }
        }
    }

    #[test]
    fn completion_counts_preferences_instead_of_registry_values() {
        let mut changes = settings_changes(HKCU, HKLM, true);
        changes.retain(|change| change.name != "TaskbarAl");
        assert_eq!(changed_preferences(&changes, false).len(), 12);
        assert_eq!(changed_preferences(&changes, true).len(), 12);
        let recent = [HIDE_RECENT_FILES.change_in(HKCU, true)];
        assert_eq!(
            changed_preferences(&recent, false),
            [Recommendation::HideRecentFiles]
        );
        let home = namespace_changes(HKCU, HKLM, HOME_CLSID, false);
        assert_eq!(
            changed_preferences(&home, false),
            [Recommendation::HideHomeFolder]
        );
        assert!(changed_preferences(&[], false).is_empty());
        assert_eq!(
            changed_preferences(&[], true),
            [Recommendation::HideShortcutArrow]
        );
    }

    #[test]
    fn splitting_recommendations_preserves_original_backup_indices() {
        let names: Vec<_> = settings_changes(HKCU, HKLM, true)
            .into_iter()
            .map(|change| change.name)
            .collect();
        assert_eq!(
            names,
            [
                "TaskbarAl",
                "UseCompactMode",
                "ShowRecent",
                "ShowRecommendations",
                "ShowFrequent",
                "ShowCloudFilesInQuickAccess",
                "",
                "System.IsPinnedToNameSpaceTree",
                HOME_CLSID,
                "HiddenByDefault",
                "System.IsPinnedToNameSpaceTree",
                GALLERY_CLSID,
                "HiddenByDefault",
                "29",
                "ShowTaskViewButton",
                "AllowNewsAndInterests",
                "TaskbarEndTask"
            ]
        );
    }

    #[test]
    fn selected_recommendations_preserve_unchecked_settings() {
        let user = TestRegistry::new();
        let machine = TestRegistry::new();
        machine
            .key
            .create_subkey(format!(r"Software\Classes\CLSID\{HOME_CLSID}"))
            .unwrap();
        let changes =
            recommended_changes(&user.key, &machine.key, true, Path::new("test-blank.ico"))
                .unwrap();
        let originals: Vec<_> = changes
            .iter()
            .map(|change| read_original(change.root, &change.path, &change.name).unwrap())
            .collect();
        let selected = [
            Recommendation::HideRecentFiles,
            Recommendation::HideHomeFolder,
        ];
        let mut selected_changes =
            recommended_changes(&user.key, &machine.key, true, Path::new("test-blank.ico"))
                .unwrap();
        selected_changes
            .retain(|change| recommendation(change).is_some_and(|id| selected.contains(&id)));
        let prepared = prepare_changes(&selected_changes).unwrap();
        apply_changes(&selected_changes, &prepared).unwrap();
        assert_eq!(changed_preferences(&selected_changes, false), selected);
        assert!(HIDE_RECENT_FILES.enabled_in(&user.key).unwrap());
        assert!(!namespace_visible_in(&user.key, &machine.key, HOME_CLSID).unwrap());
        for (change, original) in changes.iter().zip(originals) {
            if !recommendation(change).is_some_and(|id| selected.contains(&id)) {
                assert_eq!(
                    read_original(change.root, &change.path, &change.name).unwrap(),
                    original
                );
            }
        }
    }

    #[test]
    fn matching_settings_do_not_require_machine_write_access() {
        let user = TestRegistry::new();
        let machine = TestRegistry::new();
        for clsid in [HOME_CLSID, GALLERY_CLSID] {
            machine
                .key
                .create_subkey(format!(r"Software\Classes\CLSID\{clsid}"))
                .unwrap();
        }
        let changes =
            recommended_changes(&user.key, &machine.key, true, Path::new("test-blank.ico"))
                .unwrap();
        let prepared = prepare_changes(&changes).unwrap();
        apply_changes(&changes, &prepared).unwrap();
        for clsid in [HOME_CLSID, GALLERY_CLSID] {
            let (_, non_enum, hidden) = namespace_paths(clsid);
            machine
                .key
                .open_subkey_with_flags(non_enum, KEY_SET_VALUE)
                .unwrap()
                .delete_value(clsid)
                .unwrap();
            machine
                .key
                .open_subkey_with_flags(hidden, KEY_SET_VALUE)
                .unwrap()
                .delete_value("HiddenByDefault")
                .unwrap();
        }
        let machine_readonly = HKCU
            .open_subkey_with_flags(&machine.path, KEY_QUERY_VALUE)
            .unwrap();
        let build_changes = || {
            pending_changes(
                recommended_changes(
                    &user.key,
                    &machine_readonly,
                    true,
                    Path::new("test-blank.ico"),
                )
                .unwrap(),
            )
            .unwrap()
        };
        assert!(build_changes().is_empty());
        let key = user
            .key
            .open_subkey_with_flags(EXPLORER_KEY, KEY_SET_VALUE)
            .unwrap();
        key.set_value("ShowRecent", &1u32).unwrap();
        let pending = build_changes();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].name, "ShowRecent");
        let prepared = prepare_changes(&pending).unwrap();
        apply_changes(&pending, &prepared).unwrap();
        assert!(build_changes().is_empty());
        let backup = original_settings_in(&user.key, &machine.key, true).unwrap();
        assert!(
            pending_changes(original_changes(&user.key, &machine_readonly, &backup).unwrap())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn first_load_snapshot_survives_reloads_and_restores_exact_original_values() {
        let user = TestRegistry::new();
        let machine = TestRegistry::new();
        let (key, _) = user.key.create_subkey(EXPLORER_KEY).unwrap();
        key.set_value("ShowRecent", &0u32).unwrap();
        key.set_value("ShowRecommendations", &1u32).unwrap();
        let (key, _) = machine.key.create_subkey(SHELL_ICONS_KEY).unwrap();
        key.set_value("29", &r"C:\custom-overlay.ico").unwrap();
        let changes = settings_changes(&user.key, &machine.key, true);
        let originals: Vec<_> = changes
            .iter()
            .map(|change| read_original(change.root, &change.path, &change.name).unwrap())
            .collect();
        original_settings_in(&user.key, &machine.key, true).unwrap();
        assert!(user.key.open_subkey(ADVANCED_KEY).is_err());
        assert!(
            machine
                .key
                .open_subkey(namespace_paths(HOME_CLSID).2)
                .is_err()
        );
        let prepared = prepare_changes(&changes).unwrap();
        apply_changes(&changes, &prepared).unwrap();
        let backup = original_settings_in(&user.key, &machine.key, false).unwrap();
        let restore = original_changes(&user.key, &machine.key, &backup).unwrap();
        for _ in 0..2 {
            let prepared = prepare_changes(&restore).unwrap();
            apply_changes(&restore, &prepared).unwrap();
            for (change, original) in changes.iter().zip(&originals) {
                let restored = read_original(change.root, &change.path, &change.name).unwrap();
                assert_eq!(
                    restored.as_ref().map(|v| (&v.bytes, &v.vtype)),
                    original.as_ref().map(|v| (&v.bytes, &v.vtype))
                );
            }
        }
    }

    #[test]
    fn shortcut_hiding_distinguishes_default_blank_and_custom_overlays() {
        let registry = TestRegistry::new();
        let directory = TestIconDirectory::new();
        let icon = directory.0.join("blank.ico");
        std::fs::write(&icon, transparent_icon()).unwrap();
        assert_eq!(
            shortcut_arrows_hidden_in(&registry.key, &icon).unwrap(),
            Some(false)
        );
        let (key, _) = registry.key.create_subkey(SHELL_ICONS_KEY).unwrap();
        key.set_value("29", &r"C:\custom-overlay.ico").unwrap();
        assert_eq!(
            shortcut_arrows_hidden_in(&registry.key, &icon).unwrap(),
            None
        );
        key.set_value("29", &icon.to_string_lossy().as_ref())
            .unwrap();
        assert_eq!(
            shortcut_arrows_hidden_in(&registry.key, &icon).unwrap(),
            Some(true)
        );
        std::fs::write(&icon, b"not an icon").unwrap();
        assert_eq!(
            shortcut_arrows_hidden_in(&registry.key, &icon).unwrap(),
            None
        );
        std::fs::write(&icon, transparent_icon()).unwrap();
        assert!(
            recommended_changes(&registry.key, &registry.key, true, &icon)
                .unwrap()
                .iter()
                .all(|change| change.path != SHELL_ICONS_KEY)
        );
        delete_value_if_present(&key, "29").unwrap();
        assert!(icon.is_file());
        assert_eq!(
            shortcut_arrows_hidden_in(&registry.key, &icon).unwrap(),
            Some(false)
        );
        key.set_value("29", &icon.to_string_lossy().as_ref())
            .unwrap();
        std::fs::remove_file(&icon).unwrap();
        assert_eq!(
            shortcut_arrows_hidden_in(&registry.key, &icon).unwrap(),
            None
        );
        assert!(
            recommended_changes(&registry.key, &registry.key, true, &icon)
                .unwrap()
                .iter()
                .any(|change| change.path == SHELL_ICONS_KEY && change.name == "29")
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
        assert_eq!(dword(42), 4096);
        for (index, pixel) in icon[62..4158].as_chunks::<4>().0.iter().enumerate() {
            assert_eq!(&pixel[..3], &[0, 0, 0]);
            assert_eq!(pixel[3], u8::from(index / 32 < 4 && index % 32 < 4));
        }
        assert!(icon[4158..].iter().all(|byte| *byte == 0xff));
    }

    #[test]
    fn icon_cache_rebuild_preserves_other_files_and_handles_missing_caches() {
        let directory = TestIconDirectory::new();
        clear_icon_cache(&directory.0).unwrap();
        let explorer = directory.0.join(r"Microsoft\Windows\Explorer");
        std::fs::create_dir_all(&explorer).unwrap();
        for name in [
            "iconcache_32.db",
            "iconcache_idx.db",
            "thumbcache_32.db",
            "other.db",
        ] {
            std::fs::write(explorer.join(name), b"cache").unwrap();
        }
        std::fs::write(directory.0.join("IconCache.db"), b"cache").unwrap();
        clear_icon_cache(&directory.0).unwrap();
        assert!(!directory.0.join("IconCache.db").exists());
        assert_eq!(std::fs::read_dir(&explorer).unwrap().count(), 2);
        assert!(
            ["thumbcache_32.db", "other.db"]
                .iter()
                .all(|name| explorer.join(name).is_file())
        );
        clear_icon_cache(&directory.0).unwrap();
    }

    #[test]
    fn blank_icon_validation_rejects_existing_invalid_files() {
        let directory = TestIconDirectory::new();
        let path = directory.0.join("blank.ico");
        assert!(!blank_icon_valid(&path).unwrap());
        std::fs::write(&path, b"not an icon").unwrap();
        assert!(!blank_icon_valid(&path).unwrap());
        let mut bytes = transparent_icon();
        bytes[100] = 1;
        std::fs::write(&path, &bytes).unwrap();
        assert!(!blank_icon_valid(&path).unwrap());
        std::fs::write(&path, transparent_icon()).unwrap();
        assert!(blank_icon_valid(&path).unwrap());
    }

    struct TestIconDirectory(PathBuf);

    impl TestIconDirectory {
        fn new() -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "orbital-icon-replacement-{}-{unique}",
                std::process::id()
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestIconDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn blank_icon_replacement_requires_opt_in_and_creates_no_backups() {
        let directory = TestIconDirectory::new();
        let path = directory.0.join("blank.ico");
        ensure_blank_icon_at(&path, false).unwrap();
        assert!(blank_icon_valid(&path).unwrap());
        ensure_blank_icon_at(&path, true).unwrap();
        assert_eq!(std::fs::read_dir(&directory.0).unwrap().count(), 1);

        let original = b"custom icon from another tool";
        std::fs::write(&path, original).unwrap();
        let error = ensure_blank_icon_at(&path, false).unwrap_err();
        let conflict = error
            .get_ref()
            .unwrap()
            .downcast_ref::<BlankIconConflict>()
            .unwrap();
        assert_eq!(conflict.0, path);
        assert_eq!(std::fs::read(&path).unwrap(), original);
        ensure_blank_icon_at(&path, true).unwrap();
        assert!(blank_icon_valid(&path).unwrap());
        ensure_blank_icon_at(&path, false).unwrap();
        ensure_blank_icon_at(&path, true).unwrap();
        assert_eq!(std::fs::read_dir(&directory.0).unwrap().count(), 1);
    }

    #[test]
    fn blank_icon_replacement_does_not_write_if_deletion_fails() {
        use std::os::windows::fs::OpenOptionsExt;

        let directory = TestIconDirectory::new();
        let path = directory.0.join("blank.ico");
        std::fs::write(&path, b"original icon").unwrap();
        let locked = OpenOptions::new()
            .read(true)
            .share_mode(3)
            .open(&path)
            .unwrap();
        let result = ensure_blank_icon_at(&path, true);
        drop(locked);
        assert!(result.is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"original icon");
        assert_eq!(std::fs::read_dir(&directory.0).unwrap().count(), 1);
    }

    #[test]
    fn blank_icon_replacement_rejects_directories() {
        let directory = TestIconDirectory::new();
        let path = directory.0.join("blank.ico");
        std::fs::create_dir(&path).unwrap();
        assert!(ensure_blank_icon_at(&path, true).is_err());
        assert!(path.is_dir());
        assert_eq!(std::fs::read_dir(&directory.0).unwrap().count(), 1);
    }
}
