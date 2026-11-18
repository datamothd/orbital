use super::*;
use std::collections::BTreeMap;
use winreg::{enums::RegType::REG_BINARY, types::FromRegValue};

const START_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Start";
const SEARCH_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Search";
const SEARCH_SETTINGS_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\SearchSettings";
const TASKBAR_LAYOUT_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StuckRects3";
const BACKUP_KEY: &str = r"Software\Orbital\OriginalExplorerSettings\Shell";
const DELEGATE_FOLDERS_KEY: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Explorer\Desktop\NameSpace\DelegateFolders";

#[derive(Clone, Copy)]
enum Backing {
    Dword(&'static str, &'static str, u32, &'static [u32]),
    Text(&'static str, &'static str, &'static str),
    Retired(&'static [u32]),
    Flag(RegistryFlag),
}

#[derive(Clone, Copy)]
struct RegistryFlag {
    path: &'static str,
    length: usize,
    offset: usize,
    mask: u8,
}

impl RegistryFlag {
    fn state(self, raw: &RegValue<'_>) -> Option<u32> {
        (raw.vtype == REG_BINARY
            && raw.bytes.len() == self.length
            && raw.bytes[0] as usize == self.length)
            .then(|| u32::from(raw.bytes[self.offset] & self.mask != 0))
    }

    fn with_state(self, mut raw: RegValue<'static>, value: u32) -> Option<RegValue<'static>> {
        self.state(&raw)?;
        let flags = &mut raw.bytes.to_mut()[self.offset];
        *flags &= !self.mask;
        if value == 1 {
            *flags |= self.mask;
        }
        Some(raw)
    }
}

macro_rules! shell_preferences {
    ($($id:ident => $recommended:expr, $backing:expr;)*) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum ShellPreference { $($id,)* }

        impl ShellPreference {
            const ALL: &[Self] = &[$(Self::$id,)*];

            fn definition(self) -> (Option<u32>, Backing) {
                match self { $(Self::$id => ($recommended, $backing),)* }
            }
        }
    };
}

const fn toggle(path: &'static str, name: &'static str, default: u32) -> Backing {
    Backing::Dword(path, name, default, &[0, 1])
}

use Backing::{Dword, Flag, Retired};

shell_preferences! {
    BrowseFoldersInNewWindow => None, Flag(RegistryFlag { path: CABINET_KEY, length: 12, offset: 4, mask: 0x20 });
    SeparateFolderProcess => None, toggle(ADVANCED_KEY, "SeparateProcess", 0);
    SharingWizard => None, toggle(ADVANCED_KEY, "SharingWizardOn", 1);
    HideFolderMergeConflicts => None, toggle(ADVANCED_KEY, "HideMergeConflicts", 1);
    FolderSizeTips => None, toggle(ADVANCED_KEY, "FolderContentsInfoTip", 1);
    HighlightCompressedFiles => None, toggle(ADVANCED_KEY, "ShowEncryptCompressedColor", 0);
    FileTypeOverlay => None, toggle(ADVANCED_KEY, "ShowTypeOverlay", 1);
    PreviewHandlers => None, toggle(ADVANCED_KEY, "ShowPreviewHandlers", 1);
    ExplorerStatusBar => None, toggle(ADVANCED_KEY, "ShowStatusBar", 1);
    SyncProviderNotifications => Some(0), toggle(ADVANCED_KEY, "ShowSyncProviderNotifications", 1);
    ExpandNavigationToFolder => None, toggle(ADVANCED_KEY, "NavPaneExpandToCurrentFolder", 0);
    AllNavigationFolders => None, toggle(ADVANCED_KEY, "NavPaneShowAllFolders", 0);
    NavigationCloudStatus => None, toggle(ADVANCED_KEY, "NavPaneShowAllCloudStates", 0);
    ExplorerTyping => None, Dword(ADVANCED_KEY, "TypeAhead", 0, &[0, 1]);
    OmitShortcutSuffix => None, Backing::Text(r"Software\Microsoft\Windows\CurrentVersion\Explorer\NamingTemplates", "ShortcutNameTemplate", r#""%s.lnk""#);
    RemovableDrivesOnlyInThisPc => None, toggle(NON_ENUM_KEYS[0], REMOVABLE_CLSID, 0);
    DriveLetters => None, Dword(EXPLORER_KEY, "ShowDriveLettersFirst", 0, &[0, 1, 2, 4]);
    StartMenuSize => None, Retired(&[0, 1, 2]);
    StartAllAppsView => None, Retired(&[0, 1, 2]);
    StartShowAllPins => None, Retired(&[0, 1]);
    StartPinnedSection => None, toggle(START_KEY, "ShowPinnedSection", 1);
    StartRecentSection => None, toggle(START_KEY, "ShowRecentSection", 1);
    StartAllAppsSection => None, toggle(START_KEY, "ShowAllAppsSection", 1);
    StartHideProfile => None, toggle(START_KEY, "HideUserInformation", 0);
    StartRecentlyAddedApps => None, toggle(START_KEY, "ShowRecentList", 1);
    StartMostUsedApps => None, toggle(START_KEY, "ShowFrequentList", 0);
    StartRecentItems => None, toggle(ADVANCED_KEY, "Start_TrackDocs", 1);
    StartTipsAndSuggestions => Some(0), toggle(ADVANCED_KEY, "Start_IrisRecommendations", 1);
    StartAccountNotifications => Some(0), toggle(ADVANCED_KEY, "Start_AccountNotifications", 1);
    TrackAppLaunches => None, toggle(ADVANCED_KEY, "Start_TrackProgs", 1);
    SearchHighlights => Some(0), toggle(SEARCH_SETTINGS_KEY, "IsDynamicSearchBoxEnabled", 1);
    SearchWebSuggestions => Some(0), toggle(SEARCH_SETTINGS_KEY, "IsWebSuggestionsEnabled", 1);
    SearchStoreSuggestions => Some(0), toggle(SEARCH_SETTINGS_KEY, "IsStoreSuggestionsEnabled", 1);
    StartSettingsShortcut => None, Retired(&[0, 1]);
    StartExplorerShortcut => None, Retired(&[0, 1]);
    StartDocumentsShortcut => None, Retired(&[0, 1]);
    StartDownloadsShortcut => None, Retired(&[0, 1]);
    StartMusicShortcut => None, Retired(&[0, 1]);
    StartPicturesShortcut => None, Retired(&[0, 1]);
    StartVideosShortcut => None, Retired(&[0, 1]);
    StartNetworkShortcut => None, Retired(&[0, 1]);
    StartPersonalFolderShortcut => None, Retired(&[0, 1]);
    TaskbarSearch => None, Dword(SEARCH_KEY, "SearchboxTaskbarMode", 2, &[0, 1, 2, 3]);
    TaskbarWidgetsButton => Some(0), toggle(ADVANCED_KEY, "TaskbarDa", 1);
    TaskbarAutoHide => None, Flag(RegistryFlag { path: TASKBAR_LAYOUT_KEY, length: 48, offset: 8, mask: 1 });
    TaskbarPosition => None, Retired(&[0, 1, 2, 3]);
    TaskbarSize => None, Retired(&[0, 1]);
    TaskbarSmallerButtons => None, Retired(&[0, 1, 2]);
    TaskbarBadges => None, toggle(ADVANCED_KEY, "TaskbarBadges", 1);
    TaskbarShareWindow => None, toggle(ADVANCED_KEY, "TaskbarSn", 1);
    TaskbarShowDesktop => None, toggle(ADVANCED_KEY, "TaskbarSd", 0);
    TaskbarCombine => None, Dword(ADVANCED_KEY, "TaskbarGlomLevel", 0, &[0, 1, 2]);
    TaskbarLastActiveClick => None, toggle(ADVANCED_KEY, "LastActiveClick", 0);
    TaskbarMultipleDisplays => None, Retired(&[0, 1]);
    TaskbarSecondaryApps => None, Retired(&[0, 1, 2]);
    TaskbarSecondaryCombine => None, Retired(&[0, 1, 2]);
    TaskbarClockSeconds => None, toggle(ADVANCED_KEY, "ShowSecondsInSystemClock", 0);
    TaskbarShowClock => None, toggle(ADVANCED_KEY, "ShowSystrayDateTimeValueName", 1);
    TaskbarAbbreviatedClock => None, toggle(ADVANCED_KEY, "ShowShortenedDateTime", 1);
    TaskbarNotificationBell => None, toggle(ADVANCED_KEY, "ShowNotificationIcon", 1);
    NotificationCenterClock => None, toggle(ADVANCED_KEY, "ShowClockInNotificationCenter", 0);
    DesktopIcons => None, toggle(ADVANCED_KEY, "HideIcons", 0);
    WindowsColorMode => None, Retired(&[0, 1]);
    AppColorMode => None, Retired(&[0, 1]);
    TransparencyEffects => None, Retired(&[0, 1]);
    StartTaskbarAccent => None, Retired(&[0, 1]);
    WindowBorderAccent => None, Retired(&[0, 1]);
}

impl ShellPreference {
    pub(super) fn accepts(self, value: u32) -> bool {
        match self.definition().1 {
            Dword(_, _, _, values) | Retired(values) => values.contains(&value),
            Backing::Text(..) | Flag(_) => value <= 1,
        }
    }

    pub(super) fn retired(self) -> bool {
        matches!(self.definition().1, Retired(_))
    }

    fn supported(self, build: u32, revision: u32) -> bool {
        let refreshed_layouts = build == 26300 || revision >= 9278;
        supported_build(build, revision)
            && match self {
                Self::StartPinnedSection
                | Self::StartRecentSection
                | Self::StartAllAppsSection
                | Self::StartHideProfile
                | Self::SearchWebSuggestions
                | Self::SearchStoreSuggestions => refreshed_layouts,
                _ if self.retired() => false,
                _ => true,
            }
    }
}

fn supported_build(build: u32, revision: u32) -> bool {
    matches!((build, revision), (26200, 6584..) | (26300, 9457..))
}

fn available<T>(result: io::Result<Option<T>>) -> io::Result<Option<T>> {
    match result {
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::PermissionDenied | io::ErrorKind::InvalidData
            ) =>
        {
            Ok(None)
        }
        result => result,
    }
}

#[derive(serde::Serialize)]
pub struct ShellSetting {
    pub value: u32,
    pub recommended: Option<u32>,
}

pub(super) fn settings_in(
    user: &RegKey,
    machine: &RegKey,
    build: u32,
    revision: u32,
) -> io::Result<BTreeMap<ShellPreference, ShellSetting>> {
    let mut settings = BTreeMap::new();
    if !supported_build(build, revision) {
        return Ok(settings);
    }
    for &preference in ShellPreference::ALL {
        if !preference.supported(build, revision) {
            continue;
        }
        if preference == ShellPreference::RemovableDrivesOnlyInThisPc
            && (optional(machine.open_subkey_with_flags(
                format!(r"{DELEGATE_FOLDERS_KEY}\{REMOVABLE_CLSID}"),
                KEY_QUERY_VALUE,
            ))?
                .is_none()
                || read_original(machine, NON_ENUM_KEYS[0], REMOVABLE_CLSID)?
                    .is_some_and(|raw| u32::from_reg_value(&raw).ok() != Some(0)))
        {
            continue;
        }
        let (recommended, backing) = preference.definition();
        let value = match backing {
            Dword(path, name, default, _) => {
                available(read_original(user, path, name).and_then(|value| {
                    Ok(Some(
                        value
                            .as_ref()
                            .map(u32::from_reg_value)
                            .transpose()?
                            .unwrap_or(default),
                    ))
                }))?
            }
            Backing::Text(path, name, enabled) => available(
                read_original(user, path, name).and_then(|value| match value {
                    None => Ok(Some(0)),
                    Some(raw) => Ok((String::from_reg_value(&raw)? == enabled).then_some(1)),
                }),
            )?,
            Retired(_) => continue,
            Flag(flag) => available(read_original(user, flag.path, "Settings"))?
                .as_ref()
                .and_then(|raw| flag.state(raw)),
        };
        if let Some(value) = value.filter(|&value| preference.accepts(value)) {
            let recommended = recommended.filter(|&target| {
                value == target || matches!(backing, Dword(_, _, default, _) if value == default)
            });
            settings.insert(preference, ShellSetting { value, recommended });
        }
    }
    Ok(settings)
}

pub(super) fn changes_in<'a>(
    user: &'a RegKey,
    preferences: &BTreeMap<ShellPreference, u32>,
) -> io::Result<Vec<RegistryChange<'a>>> {
    let mut changes = Vec::new();
    for (&preference, &value) in preferences {
        match preference.definition().1 {
            Dword(path, name, _, _) => changes.push(RegistryChange::new(
                user,
                path,
                name,
                (value != 0 || preference != ShellPreference::RemovableDrivesOnlyInThisPc)
                    .then(|| value.to_reg_value()),
            )),
            Backing::Text(path, name, enabled) => changes.push(RegistryChange::new(
                user,
                path,
                name,
                (value == 1).then(|| enabled.to_reg_value()),
            )),
            Retired(_) => continue,
            Flag(flag) => {
                let raw = read_original(user, flag.path, "Settings")?
                    .and_then(|original| flag.with_state(original, value))
                    .ok_or_else(|| {
                        io::Error::other("Windows shell layout settings are unavailable.")
                    })?;
                changes.push(RegistryChange::new(user, flag.path, "Settings", Some(raw)));
            }
        }
    }
    Ok(changes)
}

fn backup_slots(
    build: u32,
    revision: u32,
) -> impl Iterator<Item = (&'static str, &'static str, Option<RegistryFlag>)> {
    ShellPreference::ALL
        .iter()
        .filter(move |preference| {
            preference.supported(build, revision)
                && **preference != ShellPreference::RemovableDrivesOnlyInThisPc
        })
        .filter_map(|preference| match preference.definition().1 {
            Dword(path, name, _, _) | Backing::Text(path, name, _) => Some((path, name, None)),
            Retired(_) => None,
            Flag(flag) => Some((flag.path, "Settings", Some(flag))),
        })
}

pub(super) fn save_originals(user: &RegKey, build: u32, revision: u32) -> io::Result<()> {
    for (path, name, flag) in backup_slots(build, revision) {
        let backup = format!(r"{BACKUP_KEY}\{path}\{name}");
        if read_original(user, &backup, "Saved")?.is_some() {
            continue;
        }
        let Some(original) = available(read_original(user, path, name).map(Some))? else {
            continue;
        };
        if flag.is_some_and(|flag| original.as_ref().and_then(|raw| flag.state(raw)).is_none()) {
            continue;
        }
        let (key, _) = user.create_subkey(&backup)?;
        write_value(&key, "Value", original.as_ref())?;
        key.set_value("Saved", &1u32)?;
    }
    Ok(())
}

pub(super) fn original_changes(
    user: &RegKey,
    build: u32,
    revision: u32,
) -> io::Result<Vec<RegistryChange<'_>>> {
    let mut changes = Vec::new();
    for (path, name, flag) in backup_slots(build, revision) {
        let backup = format!(r"{BACKUP_KEY}\{path}\{name}");
        if read_original(user, &backup, "Saved")?.is_none() {
            continue;
        }
        let Some(current) = available(read_original(user, path, name).map(Some))? else {
            continue;
        };
        let mut original = read_original(user, &backup, "Value")?;
        if let Some(flag) = flag {
            let restored = original
                .as_ref()
                .and_then(|raw| flag.state(raw))
                .and_then(|value| current.and_then(|raw| flag.with_state(raw, value)));
            let Some(restored) = restored else {
                continue;
            };
            original = Some(restored);
        }
        changes.push(RegistryChange::new(user, path, name, original));
    }
    Ok(changes)
}

pub(super) fn refresh_start(changes: &[RegistryChange<'_>]) -> io::Result<()> {
    if !changes
        .iter()
        .any(|change| change.path == START_KEY || change.name.starts_with("Start_"))
    {
        return Ok(());
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn ProcessIdToSessionId(process: u32, session: *mut u32) -> i32;
    }

    let mut session = 0;
    if unsafe { ProcessIdToSessionId(std::process::id(), &mut session) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let processes =
        sysinfo::System::new_with_specifics(sysinfo::RefreshKind::nothing().with_processes(
            sysinfo::ProcessRefreshKind::nothing().with_exe(sysinfo::UpdateKind::OnlyIfNotSet),
        ));
    let root = std::env::var_os("SystemRoot").unwrap_or_default();
    let executable = Path::new(&root).join(r"SystemApps\Microsoft.Windows.StartMenuExperienceHost_cw5n1h2txyewy\StartMenuExperienceHost.exe");
    for (pid, process) in processes.processes() {
        let mut process_session = 0;
        if process.exe().is_some_and(|path| {
            path.as_os_str()
                .eq_ignore_ascii_case(executable.as_os_str())
        }) && unsafe { ProcessIdToSessionId(pid.as_u32(), &mut process_session) } != 0
            && process_session == session
        {
            if let Err(error) = stop_explorer(pid.as_u32())
                && unsafe { ProcessIdToSessionId(pid.as_u32(), &mut process_session) } != 0
            {
                return Err(io::Error::other(format!(
                    "Settings saved, but Start could not be restarted: {error}"
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ShellPreference::*;

    #[test]
    fn shell_preferences_follow_the_target_releases_and_feature_updates() {
        for build in [22631, 26100, 26220, 28000] {
            assert!(
                ShellPreference::ALL
                    .iter()
                    .all(|preference| !preference.supported(build, 9999))
            );
        }
        assert!(TaskbarCombine.supported(26200, 6584));
        assert!(!TaskbarCombine.supported(26200, 6583));
        assert!(!TaskbarPosition.supported(26300, 9711));
        assert!(!StartDocumentsShortcut.supported(26200, 9974));
        assert!(!StartPinnedSection.supported(26200, 9277));
        assert!(StartPinnedSection.supported(26200, 9278));
        assert!(ShellPreference::ALL.iter().all(|preference| {
            preference
                .definition()
                .0
                .is_none_or(|value| preference.accepts(value))
        }));
    }

    #[test]
    fn explorer_shortcut_and_removable_drive_preferences_round_trip() {
        let user = super::super::tests::TestRegistry::new();
        let machine = super::super::tests::TestRegistry::new();
        let (user, machine) = (&user.key, &machine.key);
        let build = 26300;
        let revision = 9974;
        let settings = || settings_in(user, machine, build, revision).unwrap();
        assert!(!settings().contains_key(&RemovableDrivesOnlyInThisPc));
        machine
            .create_subkey(format!(r"{DELEGATE_FOLDERS_KEY}\{REMOVABLE_CLSID}"))
            .unwrap();
        let original = original_settings_in(user, machine, true).unwrap();
        save_originals(user, build, revision).unwrap();
        let selected = BTreeMap::from([(OmitShortcutSuffix, 1), (RemovableDrivesOnlyInThisPc, 1)]);
        let changes = changes_in(user, &selected).unwrap();
        apply_changes(&changes, &prepare_changes(&changes).unwrap()).unwrap();
        let state = settings();
        assert!(selected.iter().all(|(preference, value)| {
            state[preference].value == *value && state[preference].recommended.is_none()
        }));
        let state = settings_in(user, machine, 26200, revision).unwrap();
        assert!(selected.iter().all(|(preference, value)| state[preference].value == *value));
        let off = BTreeMap::from([(OmitShortcutSuffix, 0), (RemovableDrivesOnlyInThisPc, 0)]);
        assert!(changes_in(user, &off).unwrap().iter().all(|change| change.value.is_none()));
        let policy = machine.create_subkey(NON_ENUM_KEYS[0]).unwrap().0;
        policy.set_value(REMOVABLE_CLSID, &1u32).unwrap();
        assert!(!settings().contains_key(&RemovableDrivesOnlyInThisPc));
        policy.delete_value(REMOVABLE_CLSID).unwrap();
        let mut restore = super::super::original_changes(user, machine, &original).unwrap();
        restore.extend(original_changes(user, build, revision).unwrap());
        let restore = pending_changes(restore).unwrap();
        assert_eq!(restore.len(), selected.len());
        apply_changes(&restore, &prepare_changes(&restore).unwrap()).unwrap();
        let state = settings();
        assert!(selected.keys().all(|preference| state[preference].value == 0));
        let Backing::Text(path, name, _) = OmitShortcutSuffix.definition().1 else {
            unreachable!();
        };
        user.create_subkey(path).unwrap().0.set_value(name, &"Custom").unwrap();
        assert!(!settings().contains_key(&OmitShortcutSuffix));
    }

    #[test]
    fn shell_changes_preserve_retired_preferences_and_restore_layout_flags() {
        let registry = super::super::tests::TestRegistry::new();
        let user = &registry.key;
        let selected = BTreeMap::from([
            (TaskbarCombine, 2),
            (TaskbarAutoHide, 1),
            (TaskbarLastActiveClick, 1),
        ]);
        let layout = user.create_subkey(TASKBAR_LAYOUT_KEY).unwrap().0;
        let mut layout_bytes = vec![0; 48];
        layout_bytes[0] = 48;
        layout_bytes[8] = 2;
        layout
            .set_raw_value(
                "Settings",
                &RegValue {
                    bytes: layout_bytes.into(),
                    vtype: REG_BINARY,
                },
            )
            .unwrap();
        let (key, _) = user.create_subkey(ADVANCED_KEY).unwrap();
        key.set_value("TaskbarLocation", &1u32).unwrap();
        save_originals(user, 26300, 9711).unwrap();
        let changes = changes_in(user, &selected).unwrap();
        assert_eq!(changes.len(), selected.len());
        apply_changes(&changes, &prepare_changes(&changes).unwrap()).unwrap();
        let state = settings_in(user, user, 26300, 9711).unwrap();
        assert_eq!(state[&TaskbarAutoHide].recommended, None);
        assert_eq!(state[&TaskbarCombine].recommended, None);
        assert!(!state.contains_key(&StartDocumentsShortcut));
        assert_eq!(state[&SyncProviderNotifications].recommended, Some(0));
        assert!(
            selected
                .iter()
                .all(|(preference, value)| state[preference].value == *value)
        );
        let mut updated = layout.get_raw_value("Settings").unwrap();
        updated.bytes.to_mut()[16] = 8;
        layout.set_raw_value("Settings", &updated).unwrap();
        key.set_value("TaskbarLocation", &2u32).unwrap();

        save_originals(user, 26300, 9711).unwrap();
        let restore = pending_changes(original_changes(user, 26300, 9711).unwrap()).unwrap();
        assert_eq!(restore.len(), selected.len());
        apply_changes(&restore, &prepare_changes(&restore).unwrap()).unwrap();
        assert_eq!(key.get_value::<u32, _>("TaskbarLocation").unwrap(), 2);
        assert!(read_original(user, ADVANCED_KEY, "LastActiveClick").unwrap().is_none());
        let restored = layout.get_raw_value("Settings").unwrap();
        assert_eq!(restored.bytes[8], 2);
        assert_eq!(restored.bytes[16], 8);
        assert!(
            read_original(user, ADVANCED_KEY, "TaskbarGlomLevel")
                .unwrap()
                .is_none()
        );
    }
}
