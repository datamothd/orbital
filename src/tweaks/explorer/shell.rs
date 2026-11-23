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
const PRIVACY_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Privacy";
const ADVERTISING_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\AdvertisingInfo";
const ADVERTISING_POLICY: &str = r"Software\Policies\Microsoft\Windows\AdvertisingInfo";
const DATA_COLLECTION_POLICY: &str = r"Software\Policies\Microsoft\Windows\DataCollection";
const SYSTEM_POLICY: &str = r"Software\Policies\Microsoft\Windows\System";
const APP_PRIVACY_POLICY: &str = r"Software\Policies\Microsoft\Windows\AppPrivacy";
const WINDOWS_AI_POLICY: &str = r"Software\Policies\Microsoft\Windows\WindowsAI";
const FEEDBACK_RULES: &str = r"Software\Microsoft\Siuf\Rules";

#[derive(Clone, Copy)]
enum Backing {
    Dword(&'static str, &'static str, u32, &'static [u32]),
    MachinePolicy(&'static str, &'static str, u32, &'static [u32]),
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

use Backing::{Dword, Flag, MachinePolicy, Retired};

const fn machine_toggle(path: &'static str, name: &'static str, default: u32) -> Backing {
    MachinePolicy(path, name, default, &[0, 1])
}

const fn app_permission(name: &'static str) -> Backing {
    MachinePolicy(APP_PRIVACY_POLICY, name, 0, &[0, 1, 2])
}

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
    TaskbarPosition => None, Dword(ADVANCED_KEY, "TaskbarLocation", 3, &[0, 1, 2, 3]);
    TaskbarSize => None, Dword(ADVANCED_KEY, "TaskbarSize", 0, &[0, 1]);
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
    BlockAdvertisingId => Some(1), machine_toggle(ADVERTISING_POLICY, "DisabledByGroupPolicy", 0);
    BlockWebsiteLanguageList => Some(1), toggle(r"Control Panel\International\User Profile", "HttpAcceptLanguageOptOut", 0);
    PersonalizedOffers => None, toggle(PRIVACY_KEY, "TailoredExperiencesWithDiagnosticDataEnabled", 0);
    DiagnosticData => None, MachinePolicy(DATA_COLLECTION_POLICY, "AllowTelemetry", 4, &[0, 1, 3, 4]);
    BlockFeedbackRequests => Some(1), machine_toggle(DATA_COLLECTION_POLICY, "DoNotShowFeedbackNotifications", 0);
    LimitDiagnosticLogs => None, machine_toggle(DATA_COLLECTION_POLICY, "LimitDiagnosticLogCollection", 0);
    LimitCrashDumps => None, machine_toggle(DATA_COLLECTION_POLICY, "LimitDumpCollection", 0);
    CloudContentSearch => None, MachinePolicy(r"Software\Policies\Microsoft\Windows\Windows Search", "AllowCloudSearch", 2, &[0, 1, 2]);
    PublishUserActivities => None, machine_toggle(SYSTEM_POLICY, "PublishUserActivities", 1);
    CrossDeviceClipboard => None, machine_toggle(SYSTEM_POLICY, "AllowCrossDeviceClipboard", 1);
    OnlineSpeechRecognition => None, toggle(r"Software\Microsoft\Speech_OneCore\Settings\OnlineSpeechPrivacy", "HasAccepted", 0);
    BlockRecallSnapshots => None, machine_toggle(WINDOWS_AI_POLICY, "DisableAIDataAnalysis", 0);
    BlockLocationService => None, machine_toggle(r"Software\Policies\Microsoft\Windows\LocationAndSensors", "DisableLocation", 0);
    AppCameraAccess => None, app_permission("LetAppsAccessCamera");
    AppMicrophoneAccess => None, app_permission("LetAppsAccessMicrophone");
    AppAccountInfoAccess => None, app_permission("LetAppsAccessAccountInfo");
    AppDiagnosticAccess => None, app_permission("LetAppsGetDiagnosticInfo");
    WindowsColorMode => None, Retired(&[0, 1]);
    AppColorMode => None, Retired(&[0, 1]);
    TransparencyEffects => None, Retired(&[0, 1]);
    StartTaskbarAccent => None, Retired(&[0, 1]);
    WindowBorderAccent => None, Retired(&[0, 1]);
}

impl ShellPreference {
    pub(super) fn accepts(self, value: u32) -> bool {
        match self.definition().1 {
            Dword(_, _, _, values) | MachinePolicy(_, _, _, values) | Retired(values) => values.contains(&value),
            Backing::Text(..) | Flag(_) => value <= 1,
        }
    }

    pub(super) fn requires_administrator(self) -> bool {
        matches!(self.definition().1, MachinePolicy(..))
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
                | Self::SearchStoreSuggestions
                | Self::TaskbarPosition
                | Self::TaskbarSize => refreshed_layouts,
                _ if self.retired() => false,
                _ => true,
            }
    }
}

fn supported_build(build: u32, revision: u32) -> bool {
    matches!((build, revision), (26200, 6584..) | (26300, 9457..))
}

fn windows_edition(machine: &RegKey) -> io::Result<String> {
    let edition = read_original(machine, WINDOWS_VERSION_KEY, "EditionID")?;
    Ok(edition.as_ref().map(String::from_reg_value).transpose()?.unwrap_or_default())
}

fn enterprise_or_education(edition: &str) -> bool {
    ["Enterprise", "Education", "IoTEnterprise"].iter().any(|prefix| edition.starts_with(prefix))
}

fn supports_device_privacy_policies(edition: &str) -> bool {
    edition.starts_with("Professional") || enterprise_or_education(edition)
}

fn privacy_recommendable(preference: ShellPreference, user: &RegKey) -> io::Result<bool> {
    Ok(match preference {
        ShellPreference::BlockAdvertisingId => {
            let consent = read_original(user, r"Software\Microsoft\Windows\CurrentVersion\CPSS\Store\AdvertisingInfo", "Value")?;
            let consent = match consent {
                Some(_) => consent,
                None => read_original(user, ADVERTISING_KEY, "Enabled")?,
            };
            consent.as_ref().is_none_or(|raw| u32::from_reg_value(raw).ok() == Some(1))
        }
        ShellPreference::BlockFeedbackRequests => {
            read_original(user, FEEDBACK_RULES, "NumberOfSIUFInPeriod")?.is_none()
                && read_original(user, FEEDBACK_RULES, "PeriodInNanoSeconds")?.is_none()
        }
        _ => true,
    })
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub choices: Option<&'static [u32]>,
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
    let edition = windows_edition(machine)?;
    for &preference in ShellPreference::ALL {
        if !preference.supported(build, revision)
            || preference.requires_administrator() && !supports_device_privacy_policies(&edition)
        {
            continue;
        }
        let policy = match preference {
            ShellPreference::PersonalizedOffers => Some((user, r"Software\Policies\Microsoft\Windows\CloudContent", "DisableTailoredExperiencesWithDiagnosticData", 1)),
            ShellPreference::OnlineSpeechRecognition => Some((machine, r"Software\Policies\Microsoft\InputPersonalization", "AllowInputPersonalization", 0)),
            _ => None,
        };
        if let Some((root, path, name, blocked)) = policy
            && available(read_original(root, path, name).and_then(|value| {
                value.as_ref().map(u32::from_reg_value).transpose()
            }))? == Some(blocked)
        {
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
        let root = if preference.requires_administrator() { machine } else { user };
        let value = match backing {
            Dword(path, name, default, _) | MachinePolicy(path, name, default, _) => {
                available(read_original(root, path, name).and_then(|value| {
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
        if let Some(mut value) = value.filter(|&value| preference.accepts(value)) {
            let choices: Option<&'static [u32]> = if preference == ShellPreference::DiagnosticData
                && !enterprise_or_education(&edition)
            {
                if value == 0 { value = 1; }
                Some(&[1, 3, 4])
            } else {
                None
            };
            let recommended = recommended.filter(|&target| {
                value == target || matches!(backing, Dword(_, _, default, _) | MachinePolicy(_, _, default, _) if value == default)
            });
            let recommended = if recommended.is_some_and(|target| value != target)
                && available(privacy_recommendable(preference, user).map(Some))? != Some(true)
            {
                None
            } else {
                recommended
            };
            settings.insert(preference, ShellSetting { value, recommended, choices });
        }
    }
    Ok(settings)
}

pub(super) fn changes_in<'a>(
    user: &'a RegKey,
    machine: &'a RegKey,
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
            MachinePolicy(path, name, default, _) => changes.push(RegistryChange::new(
                machine,
                path,
                name,
                (value != default).then(|| value.to_reg_value()),
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
    device_privacy: bool,
    build: u32,
    revision: u32,
) -> impl Iterator<Item = (&'static str, &'static str, Option<RegistryFlag>, bool)> {
    ShellPreference::ALL
        .iter()
        .filter(move |preference| {
            preference.supported(build, revision)
                && **preference != ShellPreference::RemovableDrivesOnlyInThisPc
                && (device_privacy || !preference.requires_administrator())
        })
        .filter_map(|preference| match preference.definition().1 {
            Dword(path, name, _, _) | Backing::Text(path, name, _) => Some((path, name, None, false)),
            MachinePolicy(path, name, _, _) => Some((path, name, None, true)),
            Retired(_) => None,
            Flag(flag) => Some((flag.path, "Settings", Some(flag), false)),
        })
}

pub(super) fn save_originals(user: &RegKey, machine: &RegKey, build: u32, revision: u32) -> io::Result<()> {
    let device_privacy = supports_device_privacy_policies(&windows_edition(machine)?);
    for (path, name, flag, machine_policy) in backup_slots(device_privacy, build, revision) {
        let backup = format!(r"{BACKUP_KEY}\{path}\{name}");
        if read_original(user, &backup, "Saved")?.is_some() {
            continue;
        }
        let root = if machine_policy { machine } else { user };
        let Some(original) = available(read_original(root, path, name).map(Some))? else {
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

pub(super) fn original_changes<'a>(
    user: &'a RegKey,
    machine: &'a RegKey,
    build: u32,
    revision: u32,
) -> io::Result<Vec<RegistryChange<'a>>> {
    let mut changes = Vec::new();
    let device_privacy = supports_device_privacy_policies(&windows_edition(machine)?);
    for (path, name, flag, machine_policy) in backup_slots(device_privacy, build, revision) {
        let backup = format!(r"{BACKUP_KEY}\{path}\{name}");
        if read_original(user, &backup, "Saved")?.is_none() {
            continue;
        }
        let root = if machine_policy { machine } else { user };
        let Some(current) = available(read_original(root, path, name).map(Some))? else {
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
        changes.push(RegistryChange::new(root, path, name, original));
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
    fn privacy_preferences_respect_default_consent_and_restore_device_policies() {
        let user = super::super::tests::TestRegistry::new();
        let machine = super::super::tests::TestRegistry::new();
        let (user, machine) = (&user.key, &machine.key);
        let edition = machine.create_subkey(WINDOWS_VERSION_KEY).unwrap().0;
        let diagnostics = machine.create_subkey(DATA_COLLECTION_POLICY).unwrap().0;
        let settings = |build, revision| settings_in(user, machine, build, revision).unwrap();

        edition.set_value("EditionID", &"Core").unwrap();
        assert!(!settings(26200, 6584).contains_key(&DiagnosticData));
        assert_eq!(settings(26200, 6584)[&BlockWebsiteLanguageList].recommended, Some(1));
        edition.set_value("EditionID", &"Professional").unwrap();
        let pro = settings(26200, 6584);
        assert_eq!(pro[&DiagnosticData].choices, Some(&[1, 3, 4][..]));
        assert_eq!(pro[&BlockAdvertisingId].recommended, Some(1));
        assert_eq!(pro[&BlockFeedbackRequests].recommended, Some(1));
        assert_eq!(pro[&DiagnosticData].recommended, None);
        assert_eq!(pro[&BlockRecallSnapshots].recommended, None);
        assert_eq!(pro[&AppCameraAccess].recommended, None);

        user.create_subkey(ADVERTISING_KEY).unwrap().0.set_value("Enabled", &0u32).unwrap();
        user.create_subkey(FEEDBACK_RULES).unwrap().0.set_value("NumberOfSIUFInPeriod", &0u32).unwrap();
        assert_eq!(settings(26300, 9457)[&BlockAdvertisingId].recommended, None);
        assert_eq!(settings(26300, 9457)[&BlockFeedbackRequests].recommended, None);

        diagnostics.set_value("AllowTelemetry", &0u32).unwrap();
        assert_eq!(settings(26300, 9457)[&DiagnosticData].value, 1);
        edition.set_value("EditionID", &"Enterprise").unwrap();
        assert_eq!(settings(26300, 9457)[&DiagnosticData].value, 0);
        save_originals(user, machine, 26300, 9457).unwrap();
        let selected = BTreeMap::from([(BlockWebsiteLanguageList, 1), (DiagnosticData, 4), (AppCameraAccess, 2)]);
        let changes = changes_in(user, machine, &selected).unwrap();
        apply_changes(&changes, &prepare_changes(&changes).unwrap()).unwrap();
        let current = settings(26300, 9457);
        assert!(selected.iter().all(|(preference, value)| current[preference].value == *value));
        assert!(read_original(user, APP_PRIVACY_POLICY, "LetAppsAccessCamera").unwrap().is_none());
        assert!(read_original(machine, DATA_COLLECTION_POLICY, "AllowTelemetry").unwrap().is_none());
        let restore = pending_changes(original_changes(user, machine, 26300, 9457).unwrap()).unwrap();
        assert_eq!(restore.len(), 3);
        apply_changes(&restore, &prepare_changes(&restore).unwrap()).unwrap();
        let restored = settings(26300, 9457);
        assert_eq!(restored[&DiagnosticData].value, 0);
        assert_eq!(restored[&AppCameraAccess].value, 0);
        assert_eq!(restored[&BlockWebsiteLanguageList].value, 0);
    }

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
        for preference in [TaskbarPosition, TaskbarSize] {
            assert!(preference.supported(26300, 9711));
            assert!(preference.supported(26200, 9278));
            assert!(!preference.supported(26200, 9277));
        }
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
        save_originals(user, machine, build, revision).unwrap();
        let selected = BTreeMap::from([(OmitShortcutSuffix, 1), (RemovableDrivesOnlyInThisPc, 1)]);
        let changes = changes_in(user, machine, &selected).unwrap();
        apply_changes(&changes, &prepare_changes(&changes).unwrap()).unwrap();
        let state = settings();
        assert!(selected.iter().all(|(preference, value)| {
            state[preference].value == *value && state[preference].recommended.is_none()
        }));
        let state = settings_in(user, machine, 26200, revision).unwrap();
        assert!(selected.iter().all(|(preference, value)| state[preference].value == *value));
        let off = BTreeMap::from([(OmitShortcutSuffix, 0), (RemovableDrivesOnlyInThisPc, 0)]);
        assert!(changes_in(user, machine, &off).unwrap().iter().all(|change| change.value.is_none()));
        let policy = machine.create_subkey(NON_ENUM_KEYS[0]).unwrap().0;
        policy.set_value(REMOVABLE_CLSID, &1u32).unwrap();
        assert!(!settings().contains_key(&RemovableDrivesOnlyInThisPc));
        policy.delete_value(REMOVABLE_CLSID).unwrap();
        let mut restore = super::super::original_changes(user, machine, &original).unwrap();
        restore.extend(original_changes(user, machine, build, revision).unwrap());
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
        let initial = settings_in(user, user, 26300, 9711).unwrap();
        assert_eq!(initial[&TaskbarPosition].value, 3);
        assert_eq!(initial[&TaskbarSize].value, 0);
        let selected = BTreeMap::from([
            (TaskbarCombine, 2),
            (TaskbarAutoHide, 1),
            (TaskbarLastActiveClick, 1),
            (TaskbarPosition, 2),
            (TaskbarSize, 1),
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
        save_originals(user, user, 26300, 9711).unwrap();
        let changes = changes_in(user, user, &selected).unwrap();
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

        save_originals(user, user, 26300, 9711).unwrap();
        let restore = pending_changes(original_changes(user, user, 26300, 9711).unwrap()).unwrap();
        assert_eq!(restore.len(), selected.len());
        apply_changes(&restore, &prepare_changes(&restore).unwrap()).unwrap();
        assert_eq!(key.get_value::<u32, _>("TaskbarLocation").unwrap(), 1);
        assert!(read_original(user, ADVANCED_KEY, "TaskbarSize").unwrap().is_none());
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
