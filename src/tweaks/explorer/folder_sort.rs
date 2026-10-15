use std::io;
use winreg::{HKCU, HKLM, RegKey, enums::KEY_READ};

const FOLDER_TYPES: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\FolderTypes";
const BACKUP: &str = r"Software\Orbital\OriginalFolderSorting";
const PENDING: &str = r"Software\Orbital\PendingFolderSorting";
const VIEW_KEYS: [&str; 6] = [
    FOLDER_TYPES,
    r"Software\Classes\Local Settings\Software\Microsoft\Windows\Shell\Bags",
    r"Software\Classes\Local Settings\Software\Microsoft\Windows\Shell\BagMRU",
    r"Software\Microsoft\Windows\Shell\Bags",
    r"Software\Microsoft\Windows\Shell\BagMRU",
    r"Software\Microsoft\Windows\CurrentVersion\Explorer\Streams\Defaults",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortColumn {
    Name,
    DateModified,
    DateCreated,
    Type,
    Size,
}

impl SortColumn {
    fn property(self) -> &'static str {
        match self {
            Self::Name => "System.ItemNameDisplay",
            Self::DateModified => "System.DateModified",
            Self::DateCreated => "System.DateCreated",
            Self::Type => "System.ItemTypeText",
            Self::Size => "System.Size",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FolderSort {
    pub column: SortColumn,
    pub descending: bool,
}

impl FolderSort {
    fn registry_value(self) -> String {
        format!(
            "prop:{}{}",
            if self.descending { '-' } else { '+' },
            self.column.property()
        )
    }

    fn parse(value: &str) -> Option<Self> {
        for column in [
            SortColumn::Name,
            SortColumn::DateModified,
            SortColumn::DateCreated,
            SortColumn::Type,
            SortColumn::Size,
        ] {
            for descending in [false, true] {
                let sort = Self { column, descending };
                if value == sort.registry_value() {
                    return Some(sort);
                }
            }
        }
        None
    }
}

fn open_optional(root: &RegKey, path: &str) -> io::Result<Option<RegKey>> {
    match root.open_subkey_with_flags(path, KEY_READ) {
        Ok(key) => Ok(Some(key)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn delete_tree(root: &RegKey, path: &str) -> io::Result<()> {
    match root.delete_subkey_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn top_views(root: &RegKey) -> io::Result<Vec<String>> {
    let mut views = Vec::new();
    for folder_type in root.enum_keys() {
        let path = format!(r"{}\TopViews", folder_type?);
        if let Some(key) = open_optional(root, &path)? {
            for view in key.enum_keys() {
                views.push(format!(r"{path}\{}", view?));
            }
        }
    }
    Ok(views)
}

pub(super) fn current() -> io::Result<Option<FolderSort>> {
    current_in(HKCU, HKLM)
}

fn current_in(user: &RegKey, machine: &RegKey) -> io::Result<Option<FolderSort>> {
    let Some(defaults) = open_optional(machine, FOLDER_TYPES)? else {
        return Ok(None);
    };
    let Some(overrides) = open_optional(user, FOLDER_TYPES)? else {
        return Ok(None);
    };
    let mut paths = top_views(&defaults)?;
    paths.extend(top_views(&overrides)?);
    paths.sort();
    paths.dedup();
    let mut current = None;
    for path in paths {
        let Some(key) = open_optional(&overrides, &path)? else {
            return Ok(None);
        };
        let value: String = match key.get_value("SortByList") {
            Ok(value) => value,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        let Some(sort) = FolderSort::parse(&value) else {
            return Ok(None);
        };
        if current.is_some_and(|previous| previous != sort) {
            return Ok(None);
        }
        current = Some(sort);
    }
    Ok(current)
}

fn snapshot(user: &RegKey, path: &str) -> io::Result<RegKey> {
    delete_tree(user, path)?;
    let (backup, _) = user.create_subkey(path)?;
    for (index, path) in VIEW_KEYS.iter().enumerate() {
        if let Some(source) = open_optional(user, path)? {
            let (destination, _) = backup.create_subkey(index.to_string())?;
            source.copy_tree("", &destination)?;
        }
    }
    backup.set_value("Complete", &1u32)?;
    Ok(backup)
}

fn restore_snapshot(user: &RegKey, backup: &RegKey) -> io::Result<()> {
    if backup.get_value::<u32, _>("Complete")? != 1 {
        return Err(io::Error::other("The folder sorting backup is incomplete."));
    }
    for (index, path) in VIEW_KEYS.iter().enumerate() {
        delete_tree(user, path)?;
        if let Some(source) = open_optional(backup, &index.to_string())? {
            let (destination, _) = user.create_subkey(path)?;
            source.copy_tree("", &destination)?;
        }
    }
    Ok(())
}

fn with_rollback(user: &RegKey, operation: impl FnOnce() -> io::Result<()>) -> io::Result<()> {
    let backup = snapshot(user, PENDING)?;
    if let Err(error) = operation() {
        if let Err(rollback) = restore_snapshot(user, &backup) {
            return Err(io::Error::other(format!(
                "{error}. Could not restore folder views: {rollback}. The backup is retained."
            )));
        }
        delete_tree(user, PENDING)?;
        return Err(error);
    }
    delete_tree(user, PENDING)
}

fn merge_defaults(source: &RegKey, destination: &RegKey) -> io::Result<()> {
    for value in source.enum_values() {
        let (name, value) = value?;
        match destination.get_raw_value(&name) {
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                destination.set_raw_value(&name, &value)?;
            }
            Err(error) => return Err(error),
        }
    }
    for name in source.enum_keys() {
        let name = name?;
        let (child, _) = destination.create_subkey(&name)?;
        merge_defaults(&source.open_subkey(&name)?, &child)?;
    }
    Ok(())
}

fn remove_saved_sort(key: &RegKey) -> io::Result<()> {
    for name in ["Sort", "SortByList"] {
        match key.delete_value(name) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    for child in key.enum_keys() {
        remove_saved_sort(&key.open_subkey_with_flags(child?, winreg::enums::KEY_ALL_ACCESS)?)?;
    }
    Ok(())
}

fn apply_in(user: &RegKey, machine: &RegKey, sort: FolderSort) -> io::Result<()> {
    let defaults = machine.open_subkey(FOLDER_TYPES)?;
    if top_views(&defaults)?.is_empty() {
        return Err(io::Error::other(
            "Explorer folder templates are unavailable.",
        ));
    }
    let backup = open_optional(user, BACKUP)?;
    if backup
        .as_ref()
        .is_none_or(|key| key.get_value::<u32, _>("Complete").ok() != Some(1))
    {
        drop(backup);
        snapshot(user, BACKUP)?;
    }
    with_rollback(user, || {
        let (overrides, _) = user.create_subkey(FOLDER_TYPES)?;
        merge_defaults(&defaults, &overrides)?;
        for path in top_views(&overrides)? {
            overrides
                .create_subkey(path)?
                .0
                .set_value("SortByList", &sort.registry_value())?;
        }
        let pending = user.open_subkey(PENDING)?;
        for path in &VIEW_KEYS[1..] {
            delete_tree(user, path)?;
        }
        for (index, child) in [(1, "AllFolders"), (3, "AllFolders"), (3, r"1\Desktop")] {
            if let Some(source) = open_optional(&pending, &format!(r"{index}\{child}"))? {
                let (destination, _) =
                    user.create_subkey(format!(r"{}\{child}", VIEW_KEYS[index]))?;
                source.copy_tree("", &destination)?;
                if child == "AllFolders" {
                    remove_saved_sort(&destination)?;
                }
            }
        }
        user.create_subkey(BACKUP)?.0.set_value("Applied", &1u32)
    })
}

pub(super) fn apply(sort: FolderSort) -> io::Result<()> {
    apply_in(HKCU, HKLM, sort)
}

pub(super) fn restore() -> io::Result<bool> {
    restore_in(HKCU)
}

fn restore_in(user: &RegKey) -> io::Result<bool> {
    let Some(backup) = open_optional(user, BACKUP)? else {
        return Ok(false);
    };
    let applied: u32 = match backup.get_value("Applied") {
        Ok(value) => value,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    if applied == 0 {
        return Ok(false);
    }
    with_rollback(user, || {
        restore_snapshot(user, &backup)?;
        user.create_subkey(BACKUP)?.0.set_value("Applied", &0u32)
    })?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct Registry {
        user: RegKey,
        machine: RegKey,
        path: String,
    }

    impl Registry {
        fn new() -> Self {
            static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = format!(
                r"Software\OrbitalTests\FolderSort-{}-{timestamp}-{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed)
            );
            let (root, _) = HKCU.create_subkey(&path).unwrap();
            let (user, _) = root.create_subkey("User").unwrap();
            let (machine, _) = root.create_subkey("Machine").unwrap();
            for template in [
                "Generic",
                "Downloads",
                "Pictures",
                "Music",
                "Documents",
                "Videos",
            ] {
                for view in ["Default", "Alternate"] {
                    let (key, _) = machine
                        .create_subkey(format!(r"{FOLDER_TYPES}\{template}\TopViews\{view}"))
                        .unwrap();
                    key.set_value("SortByList", &"prop:+System.ItemNameDisplay")
                        .unwrap();
                    key.set_value("LogicalViewMode", &1u32).unwrap();
                }
            }
            Self {
                user,
                machine,
                path,
            }
        }
    }

    impl Drop for Registry {
        fn drop(&mut self) {
            HKCU.delete_subkey_all(&self.path).unwrap();
        }
    }

    fn set_binary(key: &RegKey, name: &str, bytes: &[u8]) {
        key.set_raw_value(
            name,
            &winreg::RegValue {
                bytes: bytes.into(),
                vtype: winreg::enums::REG_BINARY,
            },
        )
        .unwrap();
    }

    #[test]
    fn applies_every_template_and_restores_original_views_exactly() {
        let registry = Registry::new();
        let path = format!(r"{FOLDER_TYPES}\Generic\TopViews\Default");
        let (original, _) = registry.user.create_subkey(&path).unwrap();
        original
            .set_value("SortByList", &"prop:-System.Size")
            .unwrap();
        original.set_value("LogicalViewMode", &4u32).unwrap();
        let (custom, _) = registry
            .user
            .create_subkey(format!(r"{FOLDER_TYPES}\Custom\TopViews\Default"))
            .unwrap();
        custom
            .set_value("SortByList", &"prop:+System.Size")
            .unwrap();
        for (index, path) in VIEW_KEYS.iter().enumerate().skip(1) {
            let (key, _) = registry
                .user
                .create_subkey(format!(r"{path}\SavedView"))
                .unwrap();
            key.set_value("Original", &(index as u32)).unwrap();
        }
        let (desktop, _) = registry
            .user
            .create_subkey(format!(r"{}\1\Desktop", VIEW_KEYS[3]))
            .unwrap();
        set_binary(&desktop, "IconLayout", &[1, 2, 3]);
        let (all_folders, _) = registry
            .user
            .create_subkey(format!(r"{}\AllFolders\Shell", VIEW_KEYS[1]))
            .unwrap();
        all_folders.set_value("Mode", &4u32).unwrap();
        set_binary(&all_folders, "Sort", &[4, 5, 6]);
        assert_eq!(current_in(&registry.user, &registry.machine).unwrap(), None);
        let sort = FolderSort {
            column: SortColumn::DateModified,
            descending: true,
        };
        apply_in(&registry.user, &registry.machine, sort).unwrap();
        assert_eq!(
            current_in(&registry.user, &registry.machine).unwrap(),
            Some(sort)
        );
        assert_eq!(
            registry
                .user
                .open_subkey(&path)
                .unwrap()
                .get_value::<u32, _>("LogicalViewMode")
                .unwrap(),
            4
        );
        for path in &VIEW_KEYS[1..] {
            assert!(
                open_optional(&registry.user, &format!(r"{path}\SavedView"))
                    .unwrap()
                    .is_none()
            );
        }
        assert_eq!(
            registry
                .user
                .open_subkey(format!(r"{}\1\Desktop", VIEW_KEYS[3]))
                .unwrap()
                .get_raw_value("IconLayout")
                .unwrap()
                .bytes
                .into_owned(),
            vec![1, 2, 3]
        );
        let all_folders = registry
            .user
            .open_subkey(format!(r"{}\AllFolders\Shell", VIEW_KEYS[1]))
            .unwrap();
        assert_eq!(all_folders.get_value::<u32, _>("Mode").unwrap(), 4);
        assert_eq!(
            all_folders.get_raw_value("Sort").unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
        apply_in(
            &registry.user,
            &registry.machine,
            FolderSort {
                column: SortColumn::Name,
                descending: false,
            },
        )
        .unwrap();
        assert!(restore_in(&registry.user).unwrap());
        assert_eq!(
            registry
                .user
                .open_subkey(&path)
                .unwrap()
                .get_value::<String, _>("SortByList")
                .unwrap(),
            "prop:-System.Size"
        );
        assert!(
            open_optional(&registry.user, &format!(r"{FOLDER_TYPES}\Downloads"))
                .unwrap()
                .is_none()
        );
        for (index, path) in VIEW_KEYS.iter().enumerate().skip(1) {
            assert_eq!(
                registry
                    .user
                    .open_subkey(format!(r"{path}\SavedView"))
                    .unwrap()
                    .get_value::<u32, _>("Original")
                    .unwrap(),
                index as u32
            );
        }
        assert_eq!(
            registry
                .user
                .open_subkey(format!(r"{}\AllFolders\Shell", VIEW_KEYS[1]))
                .unwrap()
                .get_raw_value("Sort")
                .unwrap()
                .bytes
                .into_owned(),
            vec![4, 5, 6]
        );
        assert!(!restore_in(&registry.user).unwrap());
        assert!(open_optional(&registry.user, PENDING).unwrap().is_none());
    }

    #[test]
    fn failed_change_rolls_back_missing_empty_and_custom_keys() {
        let registry = Registry::new();
        let (key, _) = registry.user.create_subkey(VIEW_KEYS[1]).unwrap();
        set_binary(&key, "Custom", &[9, 8, 7]);
        registry.user.create_subkey(VIEW_KEYS[2]).unwrap();
        let error = with_rollback(&registry.user, || {
            delete_tree(&registry.user, VIEW_KEYS[1])?;
            delete_tree(&registry.user, VIEW_KEYS[2])?;
            registry.user.create_subkey(FOLDER_TYPES)?;
            Err(io::Error::other("Simulated failure"))
        })
        .unwrap_err();
        assert!(error.to_string().contains("Simulated failure"));
        assert!(
            open_optional(&registry.user, FOLDER_TYPES)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            registry
                .user
                .open_subkey(VIEW_KEYS[1])
                .unwrap()
                .get_raw_value("Custom")
                .unwrap()
                .bytes
                .into_owned(),
            vec![9, 8, 7]
        );
        assert!(
            registry
                .user
                .open_subkey(VIEW_KEYS[2])
                .unwrap()
                .enum_keys()
                .next()
                .is_none()
        );
    }

    #[test]
    fn incomplete_backup_is_replaced_before_applying() {
        let registry = Registry::new();
        registry.user.create_subkey(BACKUP).unwrap();
        apply_in(
            &registry.user,
            &registry.machine,
            FolderSort {
                column: SortColumn::Name,
                descending: false,
            },
        )
        .unwrap();
        assert!(restore_in(&registry.user).unwrap());
        assert!(
            open_optional(&registry.user, FOLDER_TYPES)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn detects_mixed_sorting_and_rejects_unknown_columns() {
        let registry = Registry::new();
        let sort = FolderSort {
            column: SortColumn::Size,
            descending: true,
        };
        apply_in(&registry.user, &registry.machine, sort).unwrap();
        let key = registry
            .user
            .open_subkey_with_flags(
                format!(r"{FOLDER_TYPES}\Pictures\TopViews\Default"),
                winreg::enums::KEY_ALL_ACCESS,
            )
            .unwrap();
        key.set_value("SortByList", &"prop:+System.Size").unwrap();
        assert_eq!(current_in(&registry.user, &registry.machine).unwrap(), None);
        assert_eq!(FolderSort::parse("prop:+System.Unknown"), None);
        assert_eq!(
            FolderSort::parse("prop:+System.Size;-System.ItemNameDisplay"),
            None
        );
        for column in [
            SortColumn::Name,
            SortColumn::DateModified,
            SortColumn::DateCreated,
            SortColumn::Type,
            SortColumn::Size,
        ] {
            for descending in [false, true] {
                let sort = FolderSort { column, descending };
                assert_eq!(FolderSort::parse(&sort.registry_value()), Some(sort));
            }
        }
    }
}
