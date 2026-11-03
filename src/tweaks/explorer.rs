#[path = "explorer/desktop_layout.rs"]
mod desktop_layout;
#[path = "explorer/folder_views.rs"]
mod folder_views;
#[path = "explorer/options.rs"]
mod options;
#[path = "explorer/profiles.rs"]
mod profiles;

pub use options::ExplorerLaunch;
use options::*;

pub use folder_views::{FolderGrouping, FolderSort, FolderView, SortColumn};
pub use profiles::{
    SettingsCommand, TweakProfile, apply_batch, apply_imported_tweaks, export_tweaks,
    set_explorer_launch, toggle_preference,
};

use std::os::windows::{
    io::{AsRawHandle, FromRawHandle, OwnedHandle},
    process::CommandExt,
};
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
    pub folder_view: Option<FolderView>,
    pub folder_grouping: Option<FolderGrouping>,
    pub center_taskbar_icons: bool,
    pub show_task_view: bool,
    pub show_taskbar_flashing: bool,
    pub allow_widgets: bool,
    pub enable_taskbar_end_task: bool,
    pub taskbar_end_task_supported: bool,
    pub use_compact_mode: bool,
    pub use_classic_context_menu: bool,
    pub windows11_supported: bool,
    pub show_recent_files: bool,
    pub show_recommendations: bool,
    pub show_frequent_folders: bool,
    pub show_cloud_files: bool,
    pub show_home_folder: bool,
    pub home_supported: bool,
    pub show_gallery: bool,
    pub gallery_supported: bool,
    pub hide_shortcut_arrow: Option<bool>,
    pub hide_file_extensions: bool,
    pub show_hidden_files: bool,
    pub show_item_checkboxes: bool,
    pub show_libraries: bool,
    pub show_full_path: bool,
    pub double_click_open: bool,
    pub hide_empty_drives: bool,
    pub confirm_file_delete: bool,
    pub confirm_file_delete_supported: bool,
    pub icons_only: bool,
    pub show_info_tips: bool,
    pub show_system_files: bool,
    pub restore_folder_windows: bool,
    pub expand_transfer_details: bool,
    pub show_network: bool,
    pub open_explorer_to: Option<ExplorerLaunch>,
    pub explorer_launch_options: Vec<ExplorerLaunch>,
    pub recommended_preferences: Vec<(Preference, bool)>,
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
        folder_sort: folder_views::current_sort_in(HKCU, HKLM)?,
        folder_view: folder_views::current_mode_in(HKCU, HKLM)?,
        folder_grouping: folder_views::current_grouping_in(HKCU, HKLM)?,
        center_taskbar_icons: CENTER_TASKBAR_ICONS.enabled_in(HKCU)?,
        show_task_view: SHOW_TASK_VIEW.enabled_in(HKCU)?,
        show_taskbar_flashing: SHOW_TASKBAR_FLASHING.enabled_in(HKCU)?,
        allow_widgets: windows11_supported && ALLOW_WIDGETS.enabled_in(HKLM)?,
        enable_taskbar_end_task: taskbar_end_task_supported
            && ENABLE_TASKBAR_END_TASK.enabled_in(HKCU)?,
        taskbar_end_task_supported,
        use_compact_mode: windows11_supported && USE_COMPACT_MODE.enabled_in(HKCU)?,
        use_classic_context_menu: windows11_supported && classic_context_menu_enabled()?,
        windows11_supported,
        show_recent_files: SHOW_RECENT_FILES.enabled_in(HKCU)?,
        show_recommendations: windows11_supported && SHOW_RECOMMENDATIONS.enabled_in(HKCU)?,
        show_frequent_folders: SHOW_FREQUENT_FOLDERS.enabled_in(HKCU)?,
        show_cloud_files: windows11_supported && SHOW_CLOUD_FILES.enabled_in(HKCU)?,
        show_home_folder: home_supported && namespace_visible_in(HKCU, HKLM, HOME_CLSID)?,
        home_supported,
        show_gallery: gallery_supported && namespace_visible_in(HKCU, HKLM, GALLERY_CLSID)?,
        gallery_supported,
        hide_shortcut_arrow: shortcut_arrows_hidden_in(HKLM, &blank_icon_path()?)?,
        hide_file_extensions: HIDE_FILE_EXTENSIONS.enabled_in(HKCU)?,
        show_hidden_files: hidden_files_in(HKCU)?,
        show_item_checkboxes: SHOW_ITEM_CHECKBOXES.enabled_in(HKCU)?,
        show_libraries: SHOW_LIBRARIES.enabled_in(HKCU)?,
        show_full_path: SHOW_FULL_PATH.enabled_in(HKCU)?,
        double_click_open: shell_state_in(HKCU)?.bytes[4] & 0x20 != 0,
        hide_empty_drives: HIDE_EMPTY_DRIVES.enabled_in(HKCU)?,
        confirm_file_delete: confirm_delete_in(HKCU, HKLM)?,
        confirm_file_delete_supported: read_original(HKLM, POLICIES_KEY, "ConfirmFileDelete")?
            .is_none(),
        icons_only: ICONS_ONLY.enabled_in(HKCU)?,
        show_info_tips: SHOW_INFO_TIPS.enabled_in(HKCU)?,
        show_system_files: SHOW_SYSTEM_FILES.enabled_in(HKCU)?,
        restore_folder_windows: RESTORE_FOLDER_WINDOWS.enabled_in(HKCU)?,
        expand_transfer_details: EXPAND_TRANSFER_DETAILS.enabled_in(HKCU)?,
        show_network: read_toggle(
            HKCU,
            NETWORK_KEY,
            "System.IsPinnedToNameSpaceTree",
            read_toggle(HKLM, NETWORK_KEY, "System.IsPinnedToNameSpaceTree", true)?,
        )?,
        open_explorer_to: explorer_launch_in(HKCU)?,
        explorer_launch_options: launch_options_in(HKCU, build)?,
        recommended_preferences: Preference::ALL
            .into_iter()
            .filter_map(|preference| {
                preference
                    .recommended_value()
                    .map(|enabled| (preference, enabled))
            })
            .collect(),
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
    #[serde(alias = "disable_taskbar_centering")]
    #[command(alias = "disable-taskbar-centering")]
    CenterTaskbarIcons,
    #[serde(alias = "hide_task_view")]
    #[command(alias = "hide-task-view")]
    ShowTaskView,
    ShowTaskbarFlashing,
    #[serde(alias = "disable_widgets")]
    #[command(alias = "disable-widgets")]
    AllowWidgets,
    EnableTaskbarEndTask,
    #[serde(alias = "disable_explorer_spacing")]
    #[command(alias = "disable-explorer-spacing")]
    UseCompactMode,
    #[serde(alias = "disable_modern_context_menu")]
    #[command(alias = "disable-modern-context-menu")]
    UseClassicContextMenu,
    #[serde(alias = "hide_recent_files")]
    #[command(alias = "hide-recent-files")]
    ShowRecentFiles,
    #[serde(alias = "hide_recommendations")]
    #[command(alias = "hide-recommendations")]
    ShowRecommendations,
    #[serde(alias = "hide_frequent_folders")]
    #[command(alias = "hide-frequent-folders")]
    ShowFrequentFolders,
    #[serde(alias = "hide_office_files")]
    #[command(alias = "hide-office-files")]
    ShowCloudFiles,
    #[serde(alias = "hide_home_folder")]
    #[command(alias = "hide-home-folder")]
    ShowHomeFolder,
    #[serde(alias = "hide_gallery")]
    #[command(alias = "hide-gallery")]
    ShowGallery,
    HideShortcutArrow,
    #[serde(alias = "show_file_extensions")]
    #[command(alias = "show-file-extensions")]
    HideFileExtensions,
    ShowHiddenFiles,
    ShowItemCheckboxes,
    ShowLibraries,
    ShowFullPath,
    #[serde(alias = "single_click_open")]
    #[command(alias = "single-click-open")]
    DoubleClickOpen,
    #[serde(alias = "show_empty_drives")]
    #[command(alias = "show-empty-drives")]
    HideEmptyDrives,
    ConfirmFileDelete,
    IconsOnly,
    #[serde(alias = "hide_info_tips")]
    #[command(alias = "hide-info-tips")]
    ShowInfoTips,
    ShowSystemFiles,
    RestoreFolderWindows,
    ExpandTransferDetails,
    ShowNetwork,
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

struct DwordPreference {
    name: &'static str,
    path: &'static str,
    value: &'static str,
    default_value: bool,
    delete_when_default: bool,
}

const CENTER_TASKBAR_ICONS: DwordPreference = DwordPreference {
    name: "Center taskbar icons",
    path: ADVANCED_KEY,
    value: "TaskbarAl",
    default_value: true,
    delete_when_default: false,
};
const SHOW_TASK_VIEW: DwordPreference = DwordPreference {
    name: "Show Task View button",
    path: ADVANCED_KEY,
    value: "ShowTaskViewButton",
    default_value: true,
    delete_when_default: false,
};
const SHOW_TASKBAR_FLASHING: DwordPreference = DwordPreference {
    name: "Show flashing on taskbar apps",
    path: ADVANCED_KEY,
    value: "TaskbarFlashing",
    default_value: true,
    delete_when_default: false,
};
const ALLOW_WIDGETS: DwordPreference = DwordPreference {
    name: "Allow Widgets",
    path: WIDGETS_KEY,
    value: "AllowNewsAndInterests",
    default_value: true,
    delete_when_default: true,
};
const ENABLE_TASKBAR_END_TASK: DwordPreference = DwordPreference {
    name: "Enable taskbar End task",
    path: TASKBAR_DEVELOPER_KEY,
    value: "TaskbarEndTask",
    default_value: false,
    delete_when_default: false,
};
const USE_COMPACT_MODE: DwordPreference = DwordPreference {
    name: "Use compact spacing",
    path: ADVANCED_KEY,
    value: "UseCompactMode",
    default_value: false,
    delete_when_default: false,
};
const SHOW_RECENT_FILES: DwordPreference = DwordPreference {
    name: "Show recently used files",
    path: EXPLORER_KEY,
    value: "ShowRecent",
    default_value: true,
    delete_when_default: false,
};
const SHOW_RECOMMENDATIONS: DwordPreference = DwordPreference {
    name: "Show recommendations",
    path: EXPLORER_KEY,
    value: "ShowRecommendations",
    default_value: true,
    delete_when_default: false,
};
const SHOW_FREQUENT_FOLDERS: DwordPreference = DwordPreference {
    name: "Show frequently used folders",
    path: EXPLORER_KEY,
    value: "ShowFrequent",
    default_value: true,
    delete_when_default: false,
};
const SHOW_CLOUD_FILES: DwordPreference = DwordPreference {
    name: "Show cloud files in Quick access",
    path: EXPLORER_KEY,
    value: "ShowCloudFilesInQuickAccess",
    default_value: true,
    delete_when_default: true,
};

impl DwordPreference {
    fn enabled_in(&self, root: &RegKey) -> io::Result<bool> {
        read_toggle(root, self.path, self.value, self.default_value)
    }

    fn change_in<'a>(&self, root: &'a RegKey, enabled: bool) -> RegistryChange<'a> {
        let next_value = u32::from(enabled);
        let value = if enabled == self.default_value && self.delete_when_default {
            None
        } else {
            Some(next_value.to_reg_value())
        };
        RegistryChange::new(root, self.path, self.value, value)
    }
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
    let mut changes: Vec<_> = [
        &CENTER_TASKBAR_ICONS,
        &USE_COMPACT_MODE,
        &SHOW_RECENT_FILES,
        &SHOW_RECOMMENDATIONS,
        &SHOW_FREQUENT_FOLDERS,
    ]
    .into_iter()
    .map(|preference| preference.change_in(user, true))
    .collect();
    if windows11 {
        changes.push(SHOW_CLOUD_FILES.change_in(user, true));
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
    changes.push(SHOW_TASK_VIEW.change_in(user, true));
    if windows11 {
        changes.push(ALLOW_WIDGETS.change_in(machine, true));
        changes.push(ENABLE_TASKBAR_END_TASK.change_in(user, true));
    }
    changes.extend(
        OPTIONS
            .iter()
            .filter(|preference| preference.value != "PersistBrowsers")
            .map(|preference| preference.change_in(user, true)),
    );
    changes.push(RegistryChange::new(user, ADVANCED_KEY, "Hidden", None));
    changes.push(RegistryChange::new(user, EXPLORER_KEY, "ShellState", None));
    changes.push(RegistryChange::new(
        user,
        ADVANCED_KEY,
        "LaunchTo",
        Some(ExplorerLaunch::Downloads.value().to_reg_value()),
    ));
    changes.push(RESTORE_FOLDER_WINDOWS.change_in(user, true));
    changes.push(SHOW_TASKBAR_FLASHING.change_in(user, true));
    changes.push(EXPAND_TRANSFER_DETAILS.change_in(user, true));
    changes.push(SHOW_NETWORK.change_in(user, true));
    changes.extend(retired_snapshot_slots(user, machine));
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
        recommendation(change).is_some() && !(change.path == SHELL_ICONS_KEY && change.name == "29")
    });
    for change in &mut changes {
        if change.path == NETWORK_KEY && change.name == SHOW_NETWORK.value {
            change.value = Some(0u32.to_reg_value());
        }
        if change.path == ADVANCED_KEY && change.name == "Hidden" {
            change.value = Some(1u32.to_reg_value());
        }
        if matches!(
            change.name.as_str(),
            "ShowTaskViewButton"
                | "TaskbarFlashing"
                | "AllowNewsAndInterests"
                | "ShowRecent"
                | "ShowRecommendations"
                | "ShowFrequent"
                | "ShowCloudFilesInQuickAccess"
                | "HideFileExt"
        ) {
            change.value = Some(0u32.to_reg_value());
        }
    }
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
                "UseCompactMode" | "ShowRecommendations" | "LaunchTo" | "TaskbarFlashing"
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
    ExpandTransferDetails,
    ShowHiddenFiles,
    ShowNetwork,
    ShowTaskView,
    ShowTaskbarFlashing,
    AllowWidgets,
    EnableTaskbarEndTask,
    UseCompactMode,
    UseClassicContextMenu,
    ShowRecentFiles,
    ShowRecommendations,
    ShowFrequentFolders,
    ShowCloudFiles,
    ShowHomeFolder,
    ShowGallery,
    HideShortcutArrow,
    HideFileExtensions,
    OpenExplorerTo,
    ShowItemCheckboxes,
    ShowFullPath,
    FolderSort,
    FolderGrouping,
    FolderView,
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
    if build < 22621 {
        changes.retain(|change| change.name != "LaunchTo");
    }
    changes.retain(|change| recommendation(change).is_some_and(selected));
    let mut view_changes = folder_views::recommended_changes_in(HKCU, HKLM)?;
    let view_id = |change: &folder_views::ViewChange| match change {
        folder_views::ViewChange::Mode(_) => Recommendation::FolderView,
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
    folder_views::apply_in(HKCU, HKLM, &view_changes, || {
        apply_changes(&changes, &prepared)
    })?;
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
        "EnthusiastMode" if change.path == EXPAND_TRANSFER_DETAILS.path => ExpandTransferDetails,
        "Hidden" if change.path == ADVANCED_KEY => ShowHiddenFiles,
        "System.IsPinnedToNameSpaceTree" if change.path == NETWORK_KEY => ShowNetwork,
        "ShowTaskViewButton" => ShowTaskView,
        "TaskbarFlashing" => ShowTaskbarFlashing,
        "AllowNewsAndInterests" => AllowWidgets,
        "TaskbarEndTask" => EnableTaskbarEndTask,
        "UseCompactMode" => UseCompactMode,
        "LaunchTo" => OpenExplorerTo,
        "" if change.path == CLASSIC_CONTEXT_MENU_KEY => UseClassicContextMenu,
        _ if change.name == HOME_CLSID || change.path.ends_with(HOME_CLSID) => ShowHomeFolder,
        _ if change.name == GALLERY_CLSID || change.path.ends_with(GALLERY_CLSID) => ShowGallery,
        "ShowRecent" => ShowRecentFiles,
        "ShowRecommendations" => ShowRecommendations,
        "ShowFrequent" => ShowFrequentFolders,
        "ShowCloudFilesInQuickAccess" => ShowCloudFiles,
        "HideFileExt" => HideFileExtensions,
        "FullPath" => ShowFullPath,
        "29" if change.path == SHELL_ICONS_KEY => HideShortcutArrow,
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
    let views_restored = folder_views::restore_in(HKCU)?;
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

pub fn set_folder_view(view: FolderView) -> io::Result<()> {
    set_folder_default("Folder view", folder_views::ViewChange::Mode(view))
}

pub fn set_folder_sort(sort: FolderSort) -> io::Result<()> {
    set_folder_default("Folder sorting", folder_views::ViewChange::Sort(sort))
}

pub fn set_folder_grouping(grouping: FolderGrouping) -> io::Result<()> {
    set_folder_default(
        "Folder grouping",
        folder_views::ViewChange::Grouping(grouping),
    )
}

fn set_folder_default(label: &str, change: folder_views::ViewChange) -> io::Result<()> {
    folder_views::apply_in(HKCU, HKLM, &[change], || Ok(()))?;
    restart_explorer(label, false, true)
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
            if blank_icon_valid(path)? {
                return Ok(());
            }
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
            return ensure_blank_icon_at(path, false);
        }
        Err(error) => return Err(setting_error(error)),
    }
    Ok(())
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

fn stop_explorer(process: u32) -> io::Result<()> {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn OpenProcess(access: u32, inherit: i32, process: u32) -> *mut std::ffi::c_void;
        fn TerminateProcess(process: *mut std::ffi::c_void, exit_code: u32) -> i32;
        fn WaitForSingleObject(handle: *mut std::ffi::c_void, milliseconds: u32) -> u32;
    }
    let handle = unsafe { OpenProcess(0x0010_0001, 0, process) };
    if handle.is_null() {
        return Err(io::Error::last_os_error());
    }
    let handle = unsafe { OwnedHandle::from_raw_handle(handle) };
    if unsafe { TerminateProcess(handle.as_raw_handle(), 0) } == 0 {
        return Err(io::Error::last_os_error());
    }
    match unsafe { WaitForSingleObject(handle.as_raw_handle(), 5000) } {
        0 => Ok(()),
        258 => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "Explorer did not stop in time",
        )),
        _ => Err(io::Error::last_os_error()),
    }
}

fn restart_explorer(
    setting: &str,
    rebuild_icons: bool,
    folder_views_changed: bool,
) -> io::Result<()> {
    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetShellWindow() -> *mut std::ffi::c_void;
        fn GetForegroundWindow() -> *mut std::ffi::c_void;
        fn SetForegroundWindow(window: *mut std::ffi::c_void) -> i32;
        fn IsWindow(window: *mut std::ffi::c_void) -> i32;
        fn GetWindowThreadProcessId(window: *mut std::ffi::c_void, process: *mut u32) -> u32;
    }
    struct RestoreFocus(*mut std::ffi::c_void);
    impl Drop for RestoreFocus {
        fn drop(&mut self) {
            unsafe {
                if self.0.is_null() || IsWindow(self.0) == 0 {
                    return;
                }
                let foreground = GetForegroundWindow();
                let mut foreground_process = 0;
                let mut shell_process = 0;
                GetWindowThreadProcessId(foreground, &mut foreground_process);
                GetWindowThreadProcessId(GetShellWindow(), &mut shell_process);
                if foreground.is_null()
                    || (shell_process != 0 && foreground_process == shell_process)
                {
                    SetForegroundWindow(self.0);
                }
            }
        }
    }
    let foreground = unsafe { GetForegroundWindow() };
    let mut foreground_process = 0;
    unsafe { GetWindowThreadProcessId(foreground, &mut foreground_process) };
    let _focus = RestoreFocus(if foreground_process == std::process::id() {
        foreground
    } else {
        std::ptr::null_mut()
    });
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
    stop_explorer(process).map_err(|error| {
        io::Error::other(format!(
            "{setting} saved, but Explorer could not be stopped: {error}"
        ))
    })?;
    let cache_result = if rebuild_icons {
        std::env::var_os("LOCALAPPDATA")
            .ok_or_else(|| io::Error::other("LOCALAPPDATA is unavailable"))
            .and_then(|local| clear_icon_cache(Path::new(&local)))
    } else {
        Ok(())
    };
    if unsafe { GetShellWindow() }.is_null() {
        ProcessCommand::new("explorer.exe")
            .creation_flags(0x08000000)
            .spawn()?;
    }
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
    fn native_stop_waits_for_child_process_exit() {
        let mut child = ProcessCommand::new("powershell.exe")
            .creation_flags(0x08000000)
            .args(["-NoProfile", "-Command", "Start-Sleep -Seconds 30"])
            .spawn()
            .unwrap();
        let result = stop_explorer(child.id());
        if result.is_err() {
            let _ = child.kill();
        }
        let status = child.wait().unwrap();
        result.unwrap();
        assert_eq!(status.code(), Some(0));
    }

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
            &CENTER_TASKBAR_ICONS,
            &SHOW_TASK_VIEW,
            &SHOW_TASKBAR_FLASHING,
            &ALLOW_WIDGETS,
            &ENABLE_TASKBAR_END_TASK,
            &USE_COMPACT_MODE,
            &SHOW_RECENT_FILES,
            &SHOW_RECOMMENDATIONS,
            &SHOW_FREQUENT_FOLDERS,
            &SHOW_CLOUD_FILES,
        ]
        .into_iter()
        .chain(OPTIONS)
        {
            assert_eq!(
                preference.enabled_in(&registry.key).unwrap(),
                preference.default_value,
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
                if enabled == (preference.default_value) && preference.delete_when_default {
                    assert_eq!(
                        key.get_raw_value(preference.value).unwrap_err().kind(),
                        io::ErrorKind::NotFound
                    );
                } else {
                    let actual: u32 = key.get_value(preference.value).unwrap();
                    assert_eq!(actual, u32::from(enabled));
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
        assert!(!SHOW_RECENT_FILES.enabled_in(&registry.key).unwrap());
        assert!(SHOW_RECOMMENDATIONS.enabled_in(&registry.key).unwrap());
        let changes = [SHOW_RECOMMENDATIONS.change_in(&registry.key, false)];
        let prepared = prepare_changes(&changes).unwrap();
        apply_changes(&changes, &prepared).unwrap();
        assert!(!SHOW_RECENT_FILES.enabled_in(&registry.key).unwrap());
        assert!(!SHOW_RECOMMENDATIONS.enabled_in(&registry.key).unwrap());
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
            assert_eq!(changes.len(), if windows11 { 23 } else { 9 });
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
                for preference in [&SHOW_RECENT_FILES, &SHOW_FREQUENT_FOLDERS] {
                    assert!(!preference.enabled_in(&user.key).unwrap());
                }
                if windows11 {
                    assert!(USE_COMPACT_MODE.enabled_in(&user.key).unwrap());
                    assert!(!SHOW_RECOMMENDATIONS.enabled_in(&user.key).unwrap());
                    assert!(!SHOW_CLOUD_FILES.enabled_in(&user.key).unwrap());
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
    fn cleanup_recommendations_preserve_manual_preferences() {
        let user = TestRegistry::new();
        let machine = TestRegistry::new();
        let advanced = user.key.create_subkey(ADVANCED_KEY).unwrap().0;
        advanced.set_value("HideFileExt", &1u32).unwrap();
        advanced.set_value("LaunchTo", &1u32).unwrap();
        for value in [0u32, 1] {
            for name in ["AutoCheckSelect", "IconsOnly", "Hidden"] {
                advanced.set_value(name, &value).unwrap();
            }
            user.key
                .create_subkey(POLICIES_KEY)
                .unwrap()
                .0
                .set_value("ConfirmFileDelete", &value)
                .unwrap();
            let selected = [Recommendation::HideFileExtensions];
            let mut changes =
                recommended_changes(&user.key, &machine.key, true, Path::new("unused.ico"))
                    .unwrap();
            assert!(!changes.iter().any(|change| matches!(
                change.name.as_str(),
                "AutoCheckSelect" | "ConfirmFileDelete" | "IconsOnly" | "ShellState"
            )));
            changes
                .retain(|change| recommendation(change).is_some_and(|id| selected.contains(&id)));
            let changes = pending_changes(changes).unwrap();
            let prepared = prepare_changes(&changes).unwrap();
            apply_changes(&changes, &prepared).unwrap();
            assert_eq!(advanced.get_value::<u32, _>("HideFileExt").unwrap(), 0);
            assert_eq!(
                explorer_launch_in(&user.key).unwrap(),
                Some(ExplorerLaunch::ThisPc)
            );
            for name in ["AutoCheckSelect", "IconsOnly", "Hidden"] {
                assert_eq!(advanced.get_value::<u32, _>(name).unwrap(), value);
            }
            assert!(pending_changes(changes).unwrap().is_empty());
        }
    }

    #[test]
    fn taskbar_flashing_recommendation_disables_and_preserves_original_state() {
        for enabled in [false, true] {
            let user = TestRegistry::new();
            let machine = TestRegistry::new();
            let original = SHOW_TASKBAR_FLASHING.change_in(&user.key, enabled);
            let prepared = prepare_changes(std::slice::from_ref(&original)).unwrap();
            apply_changes(&[original], &prepared).unwrap();
            let backup = original_settings_in(&user.key, &machine.key, true).unwrap();
            let changes =
                recommended_changes(&user.key, &machine.key, true, Path::new("blank.ico")).unwrap();
            let changes: Vec<_> = pending_changes(changes)
                .unwrap()
                .into_iter()
                .filter(|change| {
                    recommendation(change) == Some(Recommendation::ShowTaskbarFlashing)
                })
                .collect();
            assert_eq!(changes.len(), usize::from(enabled));
            let prepared = prepare_changes(&changes).unwrap();
            apply_changes(&changes, &prepared).unwrap();
            assert!(!SHOW_TASKBAR_FLASHING.enabled_in(&user.key).unwrap());
            let original: Vec<_> = original_changes(&user.key, &machine.key, &backup)
                .unwrap()
                .into_iter()
                .filter(|change| change.name == "TaskbarFlashing")
                .collect();
            let prepared = prepare_changes(&original).unwrap();
            apply_changes(&original, &prepared).unwrap();
            assert_eq!(
                SHOW_TASKBAR_FLASHING.enabled_in(&user.key).unwrap(),
                enabled
            );
            assert_eq!(
                Preference::ShowTaskbarFlashing.recommended_value(),
                Some(false)
            );
        }
    }

    #[test]
    fn recommended_dword_values_differ_from_windows_defaults() {
        for (preference, setting) in [
            (Preference::ShowTaskView, &SHOW_TASK_VIEW),
            (Preference::ShowTaskbarFlashing, &SHOW_TASKBAR_FLASHING),
            (Preference::AllowWidgets, &ALLOW_WIDGETS),
            (Preference::EnableTaskbarEndTask, &ENABLE_TASKBAR_END_TASK),
            (Preference::ShowRecentFiles, &SHOW_RECENT_FILES),
            (Preference::ShowRecommendations, &SHOW_RECOMMENDATIONS),
            (Preference::ShowFrequentFolders, &SHOW_FREQUENT_FOLDERS),
            (Preference::ShowCloudFiles, &SHOW_CLOUD_FILES),
            (Preference::HideFileExtensions, &HIDE_FILE_EXTENSIONS),
            (Preference::ShowFullPath, &SHOW_FULL_PATH),
        ] {
            assert_ne!(
                preference.recommended_value().unwrap(),
                setting.default_value,
                "{}",
                setting.name
            );
        }
        for preference in [
            Preference::ShowLibraries,
            Preference::ShowItemCheckboxes,
            Preference::ConfirmFileDelete,
            Preference::IconsOnly,
            Preference::DoubleClickOpen,
            Preference::HideEmptyDrives,
            Preference::ShowSystemFiles,
            Preference::ShowInfoTips,
            Preference::CenterTaskbarIcons,
        ] {
            assert_eq!(preference.recommended_value(), None);
        }
    }

    #[test]
    fn counted_recommendations_are_pending_selectable_and_repeatable() {
        for selected in [
            Recommendation::ExpandTransferDetails,
            Recommendation::ShowHiddenFiles,
            Recommendation::ShowNetwork,
        ] {
            let user = TestRegistry::new();
            let machine = TestRegistry::new();
            let advanced = user.key.create_subkey(ADVANCED_KEY).unwrap().0;
            advanced.set_value("Hidden", &2u32).unwrap();
            let initial = [SHOW_NETWORK.change_in(&user.key, true)];
            apply_changes(&initial, &prepare_changes(&initial).unwrap()).unwrap();
            let backup = original_settings_in(&user.key, &machine.key, true).unwrap();
            let pending = || {
                let mut changes =
                    recommended_changes(&user.key, &machine.key, true, Path::new("unused.ico"))
                        .unwrap();
                changes.retain(|change| recommendation(change) == Some(selected));
                pending_changes(changes).unwrap()
            };
            let changes = pending();
            assert_eq!(changed_preferences(&changes, false), [selected]);
            apply_changes(&changes, &prepare_changes(&changes).unwrap()).unwrap();
            assert_eq!(
                EXPAND_TRANSFER_DETAILS.enabled_in(&user.key).unwrap(),
                selected == Recommendation::ExpandTransferDetails
            );
            assert_eq!(
                hidden_files_in(&user.key).unwrap(),
                selected == Recommendation::ShowHiddenFiles
            );
            assert_eq!(
                SHOW_NETWORK.enabled_in(&user.key).unwrap(),
                selected != Recommendation::ShowNetwork
            );
            assert!(pending().is_empty());
            let restored = original_changes(&user.key, &machine.key, &backup).unwrap();
            apply_changes(&restored, &prepare_changes(&restored).unwrap()).unwrap();
            assert!(!EXPAND_TRANSFER_DETAILS.enabled_in(&user.key).unwrap());
            assert!(!hidden_files_in(&user.key).unwrap());
            assert!(SHOW_NETWORK.enabled_in(&user.key).unwrap());
        }
    }

    #[test]
    fn completion_counts_preferences_instead_of_registry_values() {
        let mut changes = settings_changes(HKCU, HKLM, true);
        changes.retain(|change| change.name != "TaskbarAl");
        assert_eq!(changed_preferences(&changes, false).len(), 19);
        assert_eq!(changed_preferences(&changes, true).len(), 19);
        let recent = [SHOW_RECENT_FILES.change_in(HKCU, true)];
        assert_eq!(
            changed_preferences(&recent, false),
            [Recommendation::ShowRecentFiles]
        );
        let home = namespace_changes(HKCU, HKLM, HOME_CLSID, false);
        assert_eq!(
            changed_preferences(&home, false),
            [Recommendation::ShowHomeFolder]
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
            .take(17)
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
            Recommendation::ShowRecentFiles,
            Recommendation::ShowFrequentFolders,
        ];
        let mut selected_changes =
            recommended_changes(&user.key, &machine.key, true, Path::new("test-blank.ico"))
                .unwrap();
        selected_changes
            .retain(|change| recommendation(change).is_some_and(|id| selected.contains(&id)));
        let prepared = prepare_changes(&selected_changes).unwrap();
        apply_changes(&selected_changes, &prepared).unwrap();
        assert_eq!(changed_preferences(&selected_changes, false), selected);
        assert!(!SHOW_RECENT_FILES.enabled_in(&user.key).unwrap());
        assert!(!SHOW_FREQUENT_FOLDERS.enabled_in(&user.key).unwrap());
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
