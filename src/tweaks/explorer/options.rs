use super::*;

const LIBRARIES_KEY: &str = r"Software\Classes\CLSID\{031E4825-7B94-4dc3-B131-E946B44C8DD5}";
const CABINET_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\CabinetState";
pub(super) const POLICIES_KEY: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Policies\Explorer";

pub(super) const HIDE_FILE_EXTENSIONS: DwordPreference = DwordPreference {
    name: "Hide known file extensions",
    path: ADVANCED_KEY,
    value: "HideFileExt",
    default_value: true,
    delete_when_default: false,
};
pub(super) const SHOW_ITEM_CHECKBOXES: DwordPreference = DwordPreference {
    name: "Show selection checkboxes",
    path: ADVANCED_KEY,
    value: "AutoCheckSelect",
    default_value: false,
    delete_when_default: false,
};
pub(super) const SHOW_LIBRARIES: DwordPreference = DwordPreference {
    name: "Show Libraries",
    path: LIBRARIES_KEY,
    value: "System.IsPinnedToNameSpaceTree",
    default_value: false,
    delete_when_default: false,
};
pub(super) const SHOW_FULL_PATH: DwordPreference = DwordPreference {
    name: "Show full path in title bar",
    path: CABINET_KEY,
    value: "FullPath",
    default_value: false,
    delete_when_default: false,
};
pub(super) const HIDE_EMPTY_DRIVES: DwordPreference = DwordPreference {
    name: "Hide empty drives",
    path: ADVANCED_KEY,
    value: "HideDrivesWithNoMedia",
    default_value: true,
    delete_when_default: false,
};
pub(super) const CONFIRM_FILE_DELETE: DwordPreference = DwordPreference {
    name: "Confirm before recycling files",
    path: POLICIES_KEY,
    value: "ConfirmFileDelete",
    default_value: false,
    delete_when_default: false,
};
pub(super) const ICONS_ONLY: DwordPreference = DwordPreference {
    name: "Show icons instead of thumbnails",
    path: ADVANCED_KEY,
    value: "IconsOnly",
    default_value: false,
    delete_when_default: false,
};
pub(super) const SHOW_INFO_TIPS: DwordPreference = DwordPreference {
    name: "Show hover descriptions",
    path: ADVANCED_KEY,
    value: "ShowInfoTip",
    default_value: true,
    delete_when_default: false,
};
pub(super) const SHOW_SYSTEM_FILES: DwordPreference = DwordPreference {
    name: "Show protected operating system files",
    path: ADVANCED_KEY,
    value: "ShowSuperHidden",
    default_value: false,
    delete_when_default: false,
};

pub(super) const RESTORE_FOLDER_WINDOWS: DwordPreference = DwordPreference {
    name: "Restore folder windows at sign-in",
    path: ADVANCED_KEY,
    value: "PersistBrowsers",
    default_value: false,
    delete_when_default: false,
};

pub(super) const OPTIONS: [&DwordPreference; 10] = [
    &HIDE_FILE_EXTENSIONS,
    &SHOW_ITEM_CHECKBOXES,
    &SHOW_LIBRARIES,
    &SHOW_FULL_PATH,
    &HIDE_EMPTY_DRIVES,
    &CONFIRM_FILE_DELETE,
    &ICONS_ONLY,
    &SHOW_INFO_TIPS,
    &SHOW_SYSTEM_FILES,
    &RESTORE_FOLDER_WINDOWS,
];

pub(super) fn hidden_files_in(user: &RegKey) -> io::Result<bool> {
    match read_original(user, ADVANCED_KEY, "Hidden")? {
        None => Ok(false),
        Some(value) => match <u32 as winreg::types::FromRegValue>::from_reg_value(&value)? {
            1 => Ok(true),
            2 => Ok(false),
            _ => Err(io::Error::other("Hidden must be 1 or 2")),
        },
    }
}

pub(super) fn shell_state_in(user: &RegKey) -> io::Result<RegValue<'static>> {
    if let Some(value) = read_original(user, EXPLORER_KEY, "ShellState")? {
        if value.vtype != winreg::enums::REG_BINARY || value.bytes.len() < 5 {
            return Err(io::Error::other(
                "ShellState must contain valid binary settings",
            ));
        }
        return Ok(value);
    }
    use windows::Win32::UI::Shell::{SHELLSTATEA, SHGetSetSettings, SSF_MASK};
    let mut state = SHELLSTATEA::default();
    unsafe { SHGetSetSettings(Some(&mut state), SSF_MASK(0x07ff_ffff), false) };
    let words = [
        36,
        state._bitfield1 as u32,
        state.dwWin95Unused,
        state.uWin95Unused,
        state.lParamSort as u32,
        state.iSortDirection as u32,
        state.version,
        state.uNotUsed,
        state._bitfield2 as u32,
    ];
    Ok(RegValue {
        bytes: words
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect::<Vec<_>>()
            .into(),
        vtype: winreg::enums::REG_BINARY,
    })
}

pub(super) fn double_click_change(user: &RegKey, enabled: bool) -> io::Result<RegistryChange<'_>> {
    let mut value = shell_state_in(user)?;
    value.bytes.to_mut()[4] = (value.bytes[4] & !0x20) | if enabled { 0x20 } else { 0 };
    Ok(RegistryChange::new(
        user,
        EXPLORER_KEY,
        "ShellState",
        Some(value),
    ))
}

pub(super) fn confirm_delete_in(user: &RegKey, machine: &RegKey) -> io::Result<bool> {
    for root in [machine, user] {
        if read_original(root, POLICIES_KEY, "ConfirmFileDelete")?.is_some() {
            return CONFIRM_FILE_DELETE.enabled_in(root);
        }
    }
    Ok(shell_state_in(user)?.bytes[4] & 0x04 == 0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExplorerLaunch {
    ThisPc,
    Home,
    Downloads,
    OneDrive,
}

impl ExplorerLaunch {
    pub(super) fn value(self) -> u32 {
        match self {
            Self::ThisPc => 1,
            Self::Home => 2,
            Self::Downloads => 3,
            Self::OneDrive => 4,
        }
    }
}

pub(super) fn launch_options_in(user: &RegKey, build: u32) -> io::Result<Vec<ExplorerLaunch>> {
    let mut choices = vec![ExplorerLaunch::Home, ExplorerLaunch::ThisPc];
    if build >= 22621 {
        choices.push(ExplorerLaunch::Downloads);
        if let Some(accounts) = optional(user.open_subkey(r"Software\Microsoft\OneDrive\Accounts"))?
        {
            for account in accounts.enum_keys() {
                let key = accounts.open_subkey(account?)?;
                if optional(key.get_value::<String, _>("UserFolder"))?
                    .is_some_and(|path| Path::new(&path).is_dir())
                {
                    choices.push(ExplorerLaunch::OneDrive);
                    break;
                }
            }
        }
    }
    Ok(choices)
}

pub(super) fn explorer_launch_in(user: &RegKey) -> io::Result<Option<ExplorerLaunch>> {
    let value = read_original(user, ADVANCED_KEY, "LaunchTo")?
        .map(|value| <u32 as winreg::types::FromRegValue>::from_reg_value(&value))
        .transpose()?
        .unwrap_or(2);
    Ok(match value {
        1 => Some(ExplorerLaunch::ThisPc),
        2 => Some(ExplorerLaunch::Home),
        3 => Some(ExplorerLaunch::Downloads),
        4 => Some(ExplorerLaunch::OneDrive),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::super::tests::TestRegistry;
    use super::*;

    #[test]
    fn double_click_preserves_other_shell_flags_and_rejects_invalid_binary() {
        let registry = TestRegistry::new();
        let key = registry.key.create_subkey(EXPLORER_KEY).unwrap().0;
        let original = RegValue {
            bytes: vec![36, 0, 0, 0, 0xff, 0xa8, 0, 0].into(),
            vtype: winreg::enums::REG_BINARY,
        };
        key.set_raw_value("ShellState", &original).unwrap();
        for enabled in [true, false] {
            let changes = [double_click_change(&registry.key, enabled).unwrap()];
            let prepared = prepare_changes(&changes).unwrap();
            apply_changes(&changes, &prepared).unwrap();
            let current = shell_state_in(&registry.key).unwrap();
            assert_eq!(current.bytes[4] & 0x20 != 0, enabled);
            assert_eq!(current.bytes[4] & !0x20, original.bytes[4] & !0x20);
            assert_eq!(current.bytes[5..], original.bytes[5..]);
        }
        key.set_value("ShellState", &"invalid").unwrap();
        assert!(double_click_change(&registry.key, true).is_err());
    }

    #[test]
    fn launch_choices_follow_build_and_configured_onedrive_accounts() {
        let registry = TestRegistry::new();
        assert_eq!(
            launch_options_in(&registry.key, 22000).unwrap(),
            [ExplorerLaunch::Home, ExplorerLaunch::ThisPc]
        );
        assert_eq!(
            launch_options_in(&registry.key, 22621).unwrap(),
            [
                ExplorerLaunch::Home,
                ExplorerLaunch::ThisPc,
                ExplorerLaunch::Downloads
            ]
        );
        let account = registry
            .key
            .create_subkey(r"Software\Microsoft\OneDrive\Accounts\Personal")
            .unwrap()
            .0;
        account
            .set_value(
                "UserFolder",
                &std::env::temp_dir().to_string_lossy().as_ref(),
            )
            .unwrap();
        assert!(
            launch_options_in(&registry.key, 22621)
                .unwrap()
                .contains(&ExplorerLaunch::OneDrive)
        );
        let advanced = registry.key.create_subkey(ADVANCED_KEY).unwrap().0;
        for target in [
            ExplorerLaunch::Home,
            ExplorerLaunch::ThisPc,
            ExplorerLaunch::Downloads,
            ExplorerLaunch::OneDrive,
        ] {
            advanced.set_value("LaunchTo", &target.value()).unwrap();
            assert_eq!(explorer_launch_in(&registry.key).unwrap(), Some(target));
        }
        advanced.set_value("LaunchTo", &99u32).unwrap();
        assert_eq!(explorer_launch_in(&registry.key).unwrap(), None);
        advanced.set_value("Hidden", &2u32).unwrap();
        assert!(!hidden_files_in(&registry.key).unwrap());
        advanced.set_value("Hidden", &1u32).unwrap();
        assert!(hidden_files_in(&registry.key).unwrap());
    }
}

pub(super) const NETWORK_KEY: &str =
    r"Software\Classes\CLSID\{F02C1A0D-BE21-4350-88B0-7367FC96EF3C}";
const REMOVABLE_CLSID: &str = "{F5FB2C77-0E2F-4A16-A381-3E560C68BC83}";
const NON_ENUM_KEYS: [&str; 2] = [
    r"Software\Microsoft\Windows\CurrentVersion\Policies\NonEnum",
    r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Policies\NonEnum",
];
const THIS_PC_FOLDERS: [&str; 7] = [
    "{31C0DD25-9439-4F12-BF41-7FF4EDA38722}",
    "{a0c69a99-21c8-4671-8703-7934162fcf1d}",
    "{7d83ee9b-2244-4e70-b1f5-5393042af1e4}",
    "{0ddd015d-b06c-45d5-8c4c-f59713854639}",
    "{35286a68-3c57-41a1-bbb1-0eae73d76c95}",
    "{f42ee2d3-909f-4907-8871-4c22fc0bf756}",
    "{B4BFCC3A-DB2C-424C-B029-7FE99A87C641}",
];

pub(super) const EXPAND_TRANSFER_DETAILS: DwordPreference = DwordPreference {
    name: "Expand file-transfer details",
    path: r"Software\Microsoft\Windows\CurrentVersion\Explorer\OperationStatusManager",
    value: "EnthusiastMode",
    default_value: false,
    delete_when_default: false,
};
pub(super) const SHOW_NETWORK: DwordPreference = DwordPreference {
    name: "Show Network in navigation pane",
    path: NETWORK_KEY,
    value: "System.IsPinnedToNameSpaceTree",
    default_value: true,
    delete_when_default: false,
};

pub(super) fn retired_snapshot_slots<'a>(
    user: &'a RegKey,
    machine: &'a RegKey,
) -> Vec<RegistryChange<'a>> {
    let mut slots = Vec::new();
    for base in [
        EXPLORER_KEY,
        r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Explorer",
    ] {
        for id in THIS_PC_FOLDERS {
            slots.push(RegistryChange::new(
                machine,
                &format!(r"{base}\FolderDescriptions\{id}\PropertyBag"),
                "ThisPCPolicy",
                None,
            ));
        }
    }
    for path in NON_ENUM_KEYS {
        for root in [user, machine] {
            slots.push(RegistryChange::new(root, path, REMOVABLE_CLSID, None));
        }
    }
    slots
}

#[cfg(test)]
mod navigation_tests {
    use super::super::tests::TestRegistry;
    use super::*;

    #[test]
    fn navigation_and_transfer_options_round_trip() {
        let user = TestRegistry::new();
        assert!(!EXPAND_TRANSFER_DETAILS.enabled_in(&user.key).unwrap());
        assert!(SHOW_NETWORK.enabled_in(&user.key).unwrap());
        for enabled in [true, false] {
            let changes = vec![
                EXPAND_TRANSFER_DETAILS.change_in(&user.key, enabled),
                SHOW_NETWORK.change_in(&user.key, enabled),
            ];
            assert!(
                changes
                    .iter()
                    .all(|change| recommendation(change).is_some())
            );
            let prepared = prepare_changes(&changes).unwrap();
            apply_changes(&changes, &prepared).unwrap();
            assert_eq!(
                EXPAND_TRANSFER_DETAILS.enabled_in(&user.key).unwrap(),
                enabled
            );
            assert_eq!(SHOW_NETWORK.enabled_in(&user.key).unwrap(), enabled);
        }
        assert_eq!(
            Preference::ExpandTransferDetails.recommended_value(),
            Some(true)
        );
        assert_eq!(Preference::ShowNetwork.recommended_value(), Some(false));
    }

    #[test]
    fn new_navigation_values_restore_their_exact_original_values() {
        let user = TestRegistry::new();
        let machine = TestRegistry::new();
        let backup = original_settings_in(&user.key, &machine.key, true).unwrap();
        let changes = vec![
            SHOW_NETWORK.change_in(&user.key, false),
            EXPAND_TRANSFER_DETAILS.change_in(&user.key, true),
        ];
        apply_changes(&changes, &prepare_changes(&changes).unwrap()).unwrap();
        let restored = original_changes(&user.key, &machine.key, &backup).unwrap();
        apply_changes(&restored, &prepare_changes(&restored).unwrap()).unwrap();
        assert!(!EXPAND_TRANSFER_DETAILS.enabled_in(&user.key).unwrap());
        assert!(SHOW_NETWORK.enabled_in(&user.key).unwrap());
        for path in NON_ENUM_KEYS {
            for root in [&user.key, &machine.key] {
                assert!(
                    read_original(root, path, REMOVABLE_CLSID)
                        .unwrap()
                        .is_none()
                );
            }
        }
    }
}
