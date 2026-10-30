use super::{delete_value_if_present, optional};
use std::io;
use winreg::{RegKey, enums::KEY_READ};

const FOLDER_TYPES: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\FolderTypes";
const BACKUP: &str = r"Software\Orbital\OriginalFolderSorting";
const PENDING: &str = r"Software\Orbital\PendingFolderSorting";
const VIEW_CONTEXTS: [&str; 3] = ["Shell", "ComDlg", "ComDlgLegacy"];
const NO_GROUPING_FLAG: u32 = 0x0010_0000;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FolderGrouping {
    None,
    Name,
    DateModified,
    DateCreated,
    Type,
    Size,
}

#[repr(C)]
#[derive(Default)]
struct PropertyKey {
    format: [u8; 16],
    id: u32,
}

struct BagGrouping {
    grouping: FolderGrouping,
    format: String,
    property_id: u32,
}

impl BagGrouping {
    fn resolve(grouping: FolderGrouping) -> io::Result<Self> {
        #[link(name = "propsys")]
        unsafe extern "system" {
            fn PSGetPropertyKeyFromName(name: *const u16, key: *mut PropertyKey) -> i32;
        }
        #[link(name = "ole32")]
        unsafe extern "system" {
            fn StringFromGUID2(guid: *const u8, text: *mut u16, length: i32) -> i32;
            fn CoInitializeEx(reserved: *mut std::ffi::c_void, flags: u32) -> i32;
            fn CoUninitialize();
        }

        struct Apartment(bool);
        impl Drop for Apartment {
            fn drop(&mut self) {
                if self.0 {
                    unsafe { CoUninitialize() };
                }
            }
        }
        let initialized = unsafe { CoInitializeEx(std::ptr::null_mut(), 0) };
        if initialized < 0 && initialized != 0x8001_0106u32 as i32 {
            return Err(io::Error::other(format!(
                "Could not initialize the property system: {initialized:#010x}"
            )));
        }
        let _apartment = Apartment(initialized >= 0);
        let mut key = PropertyKey::default();
        if grouping != FolderGrouping::None {
            let name: Vec<u16> = grouping.property().encode_utf16().chain(Some(0)).collect();
            let result = unsafe { PSGetPropertyKeyFromName(name.as_ptr(), &mut key) };
            if result < 0 {
                return Err(io::Error::other(format!(
                    "Could not resolve {}: {result:#010x}",
                    grouping.property()
                )));
            }
        }
        let mut text = [0u16; 39];
        let length =
            unsafe { StringFromGUID2(key.format.as_ptr(), text.as_mut_ptr(), text.len() as i32) };
        if length == 0 {
            return Err(io::Error::other(
                "Could not format the grouping property key.",
            ));
        }
        Ok(Self {
            grouping,
            format: String::from_utf16_lossy(&text[..length as usize - 1]),
            property_id: key.id,
        })
    }

    fn values(&self) -> [(&'static str, u32); 3] {
        let view = if self.grouping == FolderGrouping::None {
            0
        } else {
            u32::MAX
        };
        let direction = if self.grouping.descending() {
            u32::MAX
        } else {
            1
        };
        [
            ("GroupView", view),
            ("GroupByKey:PID", self.property_id),
            ("GroupByDirection", direction),
        ]
    }

    fn write(&self, key: &RegKey) -> io::Result<()> {
        for (name, value) in self.values() {
            key.set_value(name, &value)?;
        }
        key.set_value("GroupByKey:FMTID", &self.format)?;
        if self.grouping != FolderGrouping::None
            && let Some(flags) = optional(key.get_value::<u32, _>("FFlags"))?
        {
            key.set_value("FFlags", &(flags & !NO_GROUPING_FLAG))?;
        }
        Ok(())
    }

    fn matches(&self, key: &RegKey) -> io::Result<bool> {
        for (name, expected) in self.values() {
            if optional(key.get_value::<u32, _>(name))? != Some(expected) {
                return Ok(false);
            }
        }
        let Some(format) = optional(key.get_value::<String, _>("GroupByKey:FMTID"))? else {
            return Ok(false);
        };
        if !format.eq_ignore_ascii_case(&self.format) {
            return Ok(false);
        }
        Ok(self.grouping == FolderGrouping::None
            || optional(key.get_value::<u32, _>("FFlags"))?
                .is_none_or(|flags| flags & NO_GROUPING_FLAG == 0))
    }
}

fn grouping_bag_paths(folder_types: &RegKey) -> io::Result<Vec<String>> {
    let types = folder_types.enum_keys().collect::<io::Result<Vec<_>>>()?;
    let mut paths = Vec::new();
    for bags in [VIEW_KEYS[1], VIEW_KEYS[3]] {
        for context in VIEW_CONTEXTS {
            let base = format!(r"{bags}\AllFolders\{context}");
            paths.push(base.clone());
            for folder_type in &types {
                paths.push(format!(r"{base}\{folder_type}"));
            }
        }
    }
    Ok(paths)
}

impl FolderGrouping {
    fn descending(self) -> bool {
        matches!(self, Self::DateModified | Self::DateCreated)
    }

    fn property(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Name => SortColumn::Name.property(),
            Self::DateModified => SortColumn::DateModified.property(),
            Self::DateCreated => SortColumn::DateCreated.property(),
            Self::Type => SortColumn::Type.property(),
            Self::Size => SortColumn::Size.property(),
        }
    }

    fn parse(value: &str) -> Option<Self> {
        [
            Self::None,
            Self::Name,
            Self::DateModified,
            Self::DateCreated,
            Self::Type,
            Self::Size,
        ]
        .into_iter()
        .find(|grouping| grouping.property() == value)
    }
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

#[derive(Clone, Copy)]
pub(super) enum ViewChange {
    Sort(FolderSort),
    Grouping(FolderGrouping),
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
        let property = value.strip_prefix("prop:")?;
        let descending = property.starts_with('-');
        let property = property.strip_prefix(if descending { '-' } else { '+' })?;
        let column = [
            SortColumn::Name,
            SortColumn::DateModified,
            SortColumn::DateCreated,
            SortColumn::Type,
            SortColumn::Size,
        ]
        .into_iter()
        .find(|column| column.property() == property)?;
        Some(Self { column, descending })
    }
}

fn open_optional(root: &RegKey, path: &str) -> io::Result<Option<RegKey>> {
    optional(root.open_subkey_with_flags(path, KEY_READ))
}

fn delete_tree(root: &RegKey, path: &str) -> io::Result<()> {
    optional(root.delete_subkey_all(path)).map(|_| ())
}

fn is_bags_path(path: &str) -> bool {
    path == VIEW_KEYS[1] || path == VIEW_KEYS[3]
}

fn clear_view_key(user: &RegKey, path: &str) -> io::Result<()> {
    if !is_bags_path(path) {
        return delete_tree(user, path);
    }
    let Some(bags) = open_optional(user, path)? else {
        return Ok(());
    };
    let mut retained_desktop = false;
    for name in bags.enum_keys().collect::<io::Result<Vec<_>>>()? {
        let bag = bags.open_subkey(&name)?;
        if open_optional(&bag, "Desktop")?.is_some() {
            retained_desktop = true;
            for child in bag.enum_keys().collect::<io::Result<Vec<_>>>()? {
                if !child.eq_ignore_ascii_case("Desktop") {
                    delete_tree(user, &format!(r"{path}\{name}\{child}"))?;
                }
            }
        } else {
            delete_tree(user, &format!(r"{path}\{name}"))?;
        }
    }
    if retained_desktop {
        let bags = user.open_subkey_with_flags(path, winreg::enums::KEY_ALL_ACCESS)?;
        for (name, _) in bags.enum_values().collect::<io::Result<Vec<_>>>()? {
            delete_value_if_present(&bags, &name)?;
        }
        Ok(())
    } else {
        delete_tree(user, path)
    }
}

fn copy_view_key(source: &RegKey, destination: &RegKey, path: &str) -> io::Result<()> {
    if !is_bags_path(path) {
        return source.copy_tree("", destination);
    }
    for value in source.enum_values() {
        let (name, value) = value?;
        destination.set_raw_value(name, &value)?;
    }
    for name in source.enum_keys() {
        let name = name?;
        let bag = source.open_subkey(&name)?;
        let mut children = bag.enum_keys().collect::<io::Result<Vec<_>>>()?;
        let desktop = children
            .iter()
            .any(|child| child.eq_ignore_ascii_case("Desktop"));
        children.retain(|child| !child.eq_ignore_ascii_case("Desktop"));
        let values = bag.enum_values().collect::<io::Result<Vec<_>>>()?;
        if desktop && values.is_empty() && children.is_empty() {
            continue;
        }
        let (target, _) = destination.create_subkey(&name)?;
        if open_optional(&target, "Desktop")?.is_none() {
            for (name, value) in values {
                target.set_raw_value(name, &value)?;
            }
        }
        for child in children {
            let (target, _) = target.create_subkey(&child)?;
            bag.open_subkey(&child)?.copy_tree("", &target)?;
        }
    }
    Ok(())
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

pub(super) fn current_grouping_in(
    user: &RegKey,
    machine: &RegKey,
) -> io::Result<Option<FolderGrouping>> {
    let Some(grouping) = current_view_in(user, machine, "GroupBy", FolderGrouping::parse)? else {
        return Ok(None);
    };
    let overrides = user.open_subkey(FOLDER_TYPES)?;
    let expected = BagGrouping::resolve(grouping)?;
    for path in grouping_bag_paths(&overrides)? {
        let Some(key) = open_optional(user, &path)? else {
            return Ok(None);
        };
        if !expected.matches(&key)? {
            return Ok(None);
        }
    }
    Ok(Some(grouping))
}

pub(super) fn current_in(user: &RegKey, machine: &RegKey) -> io::Result<Option<FolderSort>> {
    current_view_in(user, machine, "SortByList", FolderSort::parse)
}

fn current_view_in<T: Copy + Eq>(
    user: &RegKey,
    machine: &RegKey,
    name: &str,
    parse: impl Fn(&str) -> Option<T>,
) -> io::Result<Option<T>> {
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
        let Some(value) = optional(key.get_value::<String, _>(name))? else {
            return Ok(None);
        };
        let Some(value) = parse(&value) else {
            return Ok(None);
        };
        if current.is_some_and(|previous| previous != value) {
            return Ok(None);
        }
        current = Some(value);
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
        return Err(io::Error::other("The folder view backup is incomplete."));
    }
    for (index, path) in VIEW_KEYS.iter().enumerate() {
        clear_view_key(user, path)?;
        if let Some(source) = open_optional(backup, &index.to_string())? {
            let (destination, _) = user.create_subkey(path)?;
            copy_view_key(&source, &destination, path)?;
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
        if optional(destination.get_raw_value(&name))?.is_none() {
            destination.set_raw_value(&name, &value)?;
        }
    }
    for name in source.enum_keys() {
        let name = name?;
        let (child, _) = destination.create_subkey(&name)?;
        merge_defaults(&source.open_subkey(&name)?, &child)?;
    }
    Ok(())
}

fn reset_saved_view(key: &RegKey, changes: &[ViewChange]) -> io::Result<()> {
    for change in changes {
        let values: &[&str] = match change {
            ViewChange::Sort(_) => &["Sort", "SortByList"],
            ViewChange::Grouping(_) => &[
                "GroupBy",
                "GroupByKey:FMTID",
                "GroupByKey:PID",
                "GroupAscending",
                "GroupView",
                "GroupByDirection",
            ],
        };
        for name in values {
            delete_value_if_present(key, name)?;
        }
    }
    for child in key.enum_keys() {
        reset_saved_view(
            &key.open_subkey_with_flags(child?, winreg::enums::KEY_ALL_ACCESS)?,
            changes,
        )?;
    }
    Ok(())
}

pub(super) fn apply_in(
    user: &RegKey,
    machine: &RegKey,
    changes: &[ViewChange],
    apply_other: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    if changes.is_empty() {
        return apply_other();
    }
    let defaults = machine.open_subkey(FOLDER_TYPES)?;
    let grouping = changes
        .iter()
        .find_map(|change| match change {
            ViewChange::Grouping(grouping) => Some(*grouping),
            ViewChange::Sort(_) => None,
        })
        .map(BagGrouping::resolve)
        .transpose()?;
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
            let (view, _) = overrides.create_subkey(path)?;
            for change in changes {
                match change {
                    ViewChange::Sort(sort) => {
                        view.set_value("SortByList", &sort.registry_value())?
                    }
                    ViewChange::Grouping(grouping) => {
                        view.set_value("GroupBy", &grouping.property())?;
                        view.set_value("GroupAscending", &u32::from(!grouping.descending()))?;
                    }
                }
            }
        }
        let pending = user.open_subkey(PENDING)?;
        for path in &VIEW_KEYS[1..] {
            clear_view_key(user, path)?;
        }
        for (index, child) in [(1, "AllFolders"), (3, "AllFolders")] {
            if let Some(source) = open_optional(&pending, &format!(r"{index}\{child}"))? {
                let (destination, _) =
                    user.create_subkey(format!(r"{}\{child}", VIEW_KEYS[index]))?;
                source.copy_tree("", &destination)?;
                reset_saved_view(&destination, changes)?;
            }
        }
        if let Some(grouping) = &grouping {
            for path in grouping_bag_paths(&overrides)? {
                grouping.write(&user.create_subkey(path)?.0)?;
            }
        }
        user.create_subkey(BACKUP)?.0.set_value("Applied", &1u32)?;
        apply_other()
    })
}

pub(super) fn recommended_changes_in(
    user: &RegKey,
    machine: &RegKey,
) -> io::Result<Vec<ViewChange>> {
    let sort = FolderSort {
        column: SortColumn::Type,
        descending: false,
    };
    let mut changes = Vec::new();
    if current_in(user, machine)? != Some(sort) {
        changes.push(ViewChange::Sort(sort));
    }
    if current_grouping_in(user, machine)? != Some(FolderGrouping::None) {
        changes.push(ViewChange::Grouping(FolderGrouping::None));
    }
    Ok(changes)
}

pub(super) fn restore_in(user: &RegKey) -> io::Result<bool> {
    let Some(backup) = open_optional(user, BACKUP)? else {
        return Ok(false);
    };
    if optional(backup.get_value::<u32, _>("Applied"))?.unwrap_or(0) == 0 {
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
    use super::super::tests::TestRegistry;
    use super::*;

    struct Registry {
        user: RegKey,
        machine: RegKey,
        _registry: TestRegistry,
    }

    impl Registry {
        fn new() -> Self {
            let registry = TestRegistry::new();
            let (user, _) = registry.key.create_subkey("User").unwrap();
            let (machine, _) = registry.key.create_subkey("Machine").unwrap();
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
                    key.set_value(
                        "GroupBy",
                        &if template == "Downloads" {
                            "System.DateModified"
                        } else {
                            ""
                        },
                    )
                    .unwrap();
                    key.set_value("GroupAscending", &1u32).unwrap();
                }
            }
            Self {
                user,
                machine,
                _registry: registry,
            }
        }
    }

    fn apply_sort(user: &RegKey, machine: &RegKey, sort: FolderSort) -> io::Result<()> {
        change_views_in(user, machine, ViewChange::Sort(sort))
    }

    fn change_views_in(user: &RegKey, machine: &RegKey, change: ViewChange) -> io::Result<()> {
        apply_in(user, machine, &[change], || Ok(()))
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
    fn folder_changes_and_restore_leave_current_desktop_bags_in_place() {
        let sort = FolderSort {
            column: SortColumn::Size,
            descending: true,
        };
        for changes in [
            vec![ViewChange::Sort(sort)],
            vec![ViewChange::Grouping(FolderGrouping::Type)],
            vec![
                ViewChange::Sort(sort),
                ViewChange::Grouping(FolderGrouping::None),
            ],
        ] {
            let registry = Registry::new();
            let mut desktops = Vec::new();
            for (index, slot) in [(1, "7"), (3, "1")] {
                let bag_path = format!(r"{}\{slot}", VIEW_KEYS[index]);
                let (bag, _) = registry.user.create_subkey(&bag_path).unwrap();
                bag.set_value("DesktopMetadata", &42u32).unwrap();
                let (desktop, _) = bag.create_subkey("Desktop").unwrap();
                set_binary(&desktop, "IconLayouts", &[1, 2, 3]);
                set_binary(&desktop, "Sort", &[4, 5, 6]);
                desktop.set_value("GroupView", &1u32).unwrap();
                desktop.set_value("FFlags", &0x4020_0220u32).unwrap();
                desktop
                    .create_subkey("Resolution")
                    .unwrap()
                    .0
                    .set_value("IconSize", &48u32)
                    .unwrap();
                bag.create_subkey("Shell")
                    .unwrap()
                    .0
                    .set_value("FolderView", &1u32)
                    .unwrap();
                desktops.push((bag_path, bag, desktop));
            }
            apply_in(&registry.user, &registry.machine, &changes, || Ok(())).unwrap();
            for (bag_path, bag, desktop) in &desktops {
                assert_eq!(
                    desktop.get_raw_value("IconLayouts").unwrap().bytes.as_ref(),
                    &[1, 2, 3]
                );
                assert_eq!(
                    desktop.get_raw_value("Sort").unwrap().bytes.as_ref(),
                    &[4, 5, 6]
                );
                assert_eq!(desktop.get_value::<u32, _>("GroupView").unwrap(), 1);
                assert_eq!(desktop.get_value::<u32, _>("FFlags").unwrap(), 0x4020_0220);
                assert_eq!(
                    desktop
                        .open_subkey("Resolution")
                        .unwrap()
                        .get_value::<u32, _>("IconSize")
                        .unwrap(),
                    48
                );
                assert_eq!(bag.get_value::<u32, _>("DesktopMetadata").unwrap(), 42);
                assert!(
                    open_optional(&registry.user, &format!(r"{bag_path}\Shell"))
                        .unwrap()
                        .is_none()
                );
                set_binary(desktop, "IconLayouts", &[9, 8, 7]);
                bag.set_value("DesktopMetadata", &99u32).unwrap();
            }
            assert!(restore_in(&registry.user).unwrap());
            for (bag_path, bag, desktop) in &desktops {
                assert_eq!(
                    desktop.get_raw_value("IconLayouts").unwrap().bytes.as_ref(),
                    &[9, 8, 7]
                );
                assert_eq!(bag.get_value::<u32, _>("DesktopMetadata").unwrap(), 99);
                assert_eq!(
                    registry
                        .user
                        .open_subkey(format!(r"{bag_path}\Shell"))
                        .unwrap()
                        .get_value::<u32, _>("FolderView")
                        .unwrap(),
                    1
                );
            }
        }
    }

    #[test]
    fn restore_preserves_new_desktops_and_does_not_resurrect_old_layouts() {
        let registry = Registry::new();
        let old_path = format!(r"{}\1\Desktop", VIEW_KEYS[3]);
        let (old, _) = registry.user.create_subkey(&old_path).unwrap();
        set_binary(&old, "IconLayouts", &[1, 2, 3]);
        apply_sort(
            &registry.user,
            &registry.machine,
            FolderSort {
                column: SortColumn::Name,
                descending: false,
            },
        )
        .unwrap();
        delete_tree(&registry.user, &old_path).unwrap();
        let new_path = format!(r"{}\2\Desktop", VIEW_KEYS[1]);
        let (new, _) = registry.user.create_subkey(&new_path).unwrap();
        set_binary(&new, "IconLayouts", &[9, 8, 7]);
        assert!(restore_in(&registry.user).unwrap());
        assert!(open_optional(&registry.user, &old_path).unwrap().is_none());
        assert_eq!(
            new.get_raw_value("IconLayouts").unwrap().bytes.as_ref(),
            &[9, 8, 7]
        );
    }

    #[test]
    fn failed_folder_change_preserves_desktop_handles_and_restores_other_views() {
        let registry = Registry::new();
        let path = format!(r"{}\1", VIEW_KEYS[3]);
        let (bag, _) = registry.user.create_subkey(&path).unwrap();
        let (desktop, _) = bag.create_subkey("Desktop").unwrap();
        set_binary(&desktop, "IconLayouts", &[1, 2, 3]);
        bag.create_subkey("Shell")
            .unwrap()
            .0
            .set_value("Original", &1u32)
            .unwrap();
        let result = apply_in(
            &registry.user,
            &registry.machine,
            &[ViewChange::Grouping(FolderGrouping::Size)],
            || Err(io::Error::other("Simulated failure")),
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Simulated failure")
        );
        assert_eq!(
            desktop.get_raw_value("IconLayouts").unwrap().bytes.as_ref(),
            &[1, 2, 3]
        );
        assert_eq!(
            bag.open_subkey("Shell")
                .unwrap()
                .get_value::<u32, _>("Original")
                .unwrap(),
            1
        );
        assert!(
            open_optional(&registry.user, FOLDER_TYPES)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn grouping_property_keys_match_the_windows_property_schema() {
        assert_eq!(std::mem::size_of::<PropertyKey>(), 20);
        for (grouping, id, direction) in [
            (FolderGrouping::None, 0, 1),
            (FolderGrouping::Name, 10, 1),
            (FolderGrouping::DateModified, 14, u32::MAX),
            (FolderGrouping::DateCreated, 15, u32::MAX),
            (FolderGrouping::Type, 4, 1),
            (FolderGrouping::Size, 12, 1),
        ] {
            let values = BagGrouping::resolve(grouping).unwrap();
            assert_eq!(values.property_id, id);
            assert!(values.values().contains(&("GroupByDirection", direction)));
            assert_eq!(
                values.format,
                if grouping == FolderGrouping::None {
                    "{00000000-0000-0000-0000-000000000000}"
                } else {
                    "{B725F130-47EF-101A-A5F1-02608C9EEBAC}"
                }
            );
        }
    }

    #[test]
    fn template_grouping_is_not_reported_as_applied_without_shell_bag_defaults() {
        let registry = Registry::new();
        let (overrides, _) = registry.user.create_subkey(FOLDER_TYPES).unwrap();
        merge_defaults(
            &registry.machine.open_subkey(FOLDER_TYPES).unwrap(),
            &overrides,
        )
        .unwrap();
        for path in top_views(&overrides).unwrap() {
            overrides
                .create_subkey(path)
                .unwrap()
                .0
                .set_value("GroupBy", &"System.ItemTypeText")
                .unwrap();
        }
        assert_eq!(
            current_grouping_in(&registry.user, &registry.machine).unwrap(),
            None
        );
        for bags in [VIEW_KEYS[1], VIEW_KEYS[3]] {
            assert!(open_optional(&registry.user, bags).unwrap().is_none());
        }
        change_views_in(
            &registry.user,
            &registry.machine,
            ViewChange::Grouping(FolderGrouping::Type),
        )
        .unwrap();
        for path in grouping_bag_paths(&overrides).unwrap() {
            let key = registry.user.open_subkey(path).unwrap();
            assert_eq!(key.get_value::<u32, _>("GroupView").unwrap(), u32::MAX);
            assert_eq!(
                key.get_value::<String, _>("GroupByKey:FMTID").unwrap(),
                "{B725F130-47EF-101A-A5F1-02608C9EEBAC}"
            );
            assert_eq!(key.get_value::<u32, _>("GroupByKey:PID").unwrap(), 4);
            assert_eq!(key.get_value::<u32, _>("GroupByDirection").unwrap(), 1);
        }
        assert_eq!(
            current_grouping_in(&registry.user, &registry.machine).unwrap(),
            Some(FolderGrouping::Type)
        );
        let path = format!(r"{}\AllFolders\Shell\Downloads", VIEW_KEYS[1]);
        let (key, _) = registry.user.create_subkey(&path).unwrap();
        key.set_value("FFlags", &(0x4120_0001u32 | NO_GROUPING_FLAG))
            .unwrap();
        assert_eq!(
            current_grouping_in(&registry.user, &registry.machine).unwrap(),
            None
        );
        change_views_in(
            &registry.user,
            &registry.machine,
            ViewChange::Grouping(FolderGrouping::Name),
        )
        .unwrap();
        assert_eq!(
            registry
                .user
                .open_subkey(&path)
                .unwrap()
                .get_value::<u32, _>("FFlags")
                .unwrap(),
            0x4120_0001
        );
        assert!(restore_in(&registry.user).unwrap());
        for bags in [VIEW_KEYS[1], VIEW_KEYS[3]] {
            assert!(open_optional(&registry.user, bags).unwrap().is_none());
        }
    }

    #[test]
    fn grouping_choices_replace_none_without_changing_sorting() {
        let registry = Registry::new();
        let sort = FolderSort {
            column: SortColumn::Size,
            descending: true,
        };
        apply_sort(&registry.user, &registry.machine, sort).unwrap();
        let saved_path = format!(r"{}\AllFolders\Shell", VIEW_KEYS[1]);
        registry.user.create_subkey(&saved_path).unwrap();
        for grouping in [
            FolderGrouping::None,
            FolderGrouping::Name,
            FolderGrouping::DateModified,
            FolderGrouping::DateCreated,
            FolderGrouping::Type,
            FolderGrouping::Size,
        ] {
            change_views_in(
                &registry.user,
                &registry.machine,
                ViewChange::Grouping(grouping),
            )
            .unwrap();
            assert_eq!(
                current_grouping_in(&registry.user, &registry.machine).unwrap(),
                Some(grouping)
            );
            assert_eq!(
                current_in(&registry.user, &registry.machine).unwrap(),
                Some(sort)
            );
            assert_eq!(FolderGrouping::parse(grouping.property()), Some(grouping));
            let saved = registry.user.open_subkey(&saved_path).unwrap();
            if grouping == FolderGrouping::None {
                assert_eq!(saved.get_value::<u32, _>("GroupView").unwrap(), 0);
            } else {
                assert_eq!(saved.get_value::<u32, _>("GroupView").unwrap(), u32::MAX);
            }
        }
        assert_eq!(FolderGrouping::parse("System.Unknown"), None);
    }

    #[test]
    fn recommendations_apply_type_and_none_once_and_rollback_on_failure() {
        let registry = Registry::new();
        let sort = FolderSort {
            column: SortColumn::Type,
            descending: false,
        };
        let changes = recommended_changes_in(&registry.user, &registry.machine).unwrap();
        assert_eq!(changes.len(), 2);
        let mut other_applies = 0;
        apply_in(&registry.user, &registry.machine, &changes, || {
            other_applies += 1;
            Ok(())
        })
        .unwrap();
        assert_eq!(other_applies, 1);
        assert_eq!(
            current_in(&registry.user, &registry.machine).unwrap(),
            Some(sort)
        );
        assert_eq!(
            current_grouping_in(&registry.user, &registry.machine).unwrap(),
            Some(FolderGrouping::None)
        );
        let changes = recommended_changes_in(&registry.user, &registry.machine).unwrap();
        assert!(changes.is_empty());
        let cached_path = format!(r"{}\ExistingFolder", VIEW_KEYS[1]);
        registry.user.create_subkey(&cached_path).unwrap();
        apply_in(&registry.user, &registry.machine, &changes, || Ok(())).unwrap();
        assert!(registry.user.open_subkey(&cached_path).is_ok());
        change_views_in(
            &registry.user,
            &registry.machine,
            ViewChange::Grouping(FolderGrouping::Name),
        )
        .unwrap();
        let changes = recommended_changes_in(&registry.user, &registry.machine).unwrap();
        assert_eq!(changes.len(), 1);
        assert!(
            apply_in(&registry.user, &registry.machine, &changes, || Err(
                io::Error::other("Other preference failed")
            ))
            .is_err()
        );
        assert_eq!(
            current_grouping_in(&registry.user, &registry.machine).unwrap(),
            Some(FolderGrouping::Name)
        );
        assert_eq!(
            current_in(&registry.user, &registry.machine).unwrap(),
            Some(sort)
        );
        apply_in(&registry.user, &registry.machine, &changes, || Ok(())).unwrap();
        let custom_sort = FolderSort {
            column: SortColumn::Name,
            descending: false,
        };
        apply_sort(&registry.user, &registry.machine, custom_sort).unwrap();
        let changes = recommended_changes_in(&registry.user, &registry.machine).unwrap();
        assert_eq!(changes.len(), 1);
        assert!(restore_in(&registry.user).unwrap());
        assert!(
            open_optional(&registry.user, FOLDER_TYPES)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn grouping_and_sorting_preserve_each_other_and_restore_together() {
        for grouping_first in [false, true] {
            let registry = Registry::new();
            let path = format!(r"{FOLDER_TYPES}\Downloads\TopViews\Default");
            let (view, _) = registry.user.create_subkey(&path).unwrap();
            view.set_value("SortByList", &"prop:-System.Size").unwrap();
            view.set_value("GroupBy", &"System.DateModified").unwrap();
            view.set_value("LogicalViewMode", &4u32).unwrap();
            let all_folders_path = format!(r"{}\AllFolders\Shell", VIEW_KEYS[1]);
            let (all_folders, _) = registry.user.create_subkey(&all_folders_path).unwrap();
            all_folders.set_value("GroupView", &1u32).unwrap();
            all_folders.set_value("GroupByKey:PID", &14u32).unwrap();
            all_folders.set_value("Mode", &4u32).unwrap();
            set_binary(&all_folders, "Sort", &[3, 2, 1]);
            assert_ne!(
                current_grouping_in(&registry.user, &registry.machine).unwrap(),
                Some(FolderGrouping::None)
            );
            let sort = FolderSort {
                column: SortColumn::Name,
                descending: true,
            };
            if grouping_first {
                change_views_in(
                    &registry.user,
                    &registry.machine,
                    ViewChange::Grouping(FolderGrouping::None),
                )
                .unwrap();
                assert_eq!(
                    registry
                        .user
                        .open_subkey(&path)
                        .unwrap()
                        .get_value::<String, _>("SortByList")
                        .unwrap(),
                    "prop:-System.Size"
                );
                assert_eq!(
                    registry
                        .user
                        .open_subkey(&all_folders_path)
                        .unwrap()
                        .get_raw_value("Sort")
                        .unwrap()
                        .bytes
                        .as_ref(),
                    &[3, 2, 1]
                );
                apply_sort(&registry.user, &registry.machine, sort).unwrap();
            } else {
                apply_sort(&registry.user, &registry.machine, sort).unwrap();
                assert_ne!(
                    current_grouping_in(&registry.user, &registry.machine).unwrap(),
                    Some(FolderGrouping::None)
                );
                change_views_in(
                    &registry.user,
                    &registry.machine,
                    ViewChange::Grouping(FolderGrouping::None),
                )
                .unwrap();
            }
            assert_eq!(
                current_grouping_in(&registry.user, &registry.machine).unwrap(),
                Some(FolderGrouping::None)
            );
            assert_eq!(
                current_in(&registry.user, &registry.machine).unwrap(),
                Some(sort)
            );
            let view = registry.user.open_subkey(&path).unwrap();
            assert_eq!(view.get_value::<u32, _>("LogicalViewMode").unwrap(), 4);
            let all_folders = registry.user.open_subkey(&all_folders_path).unwrap();
            assert_eq!(all_folders.get_value::<u32, _>("GroupView").unwrap(), 0);
            assert_eq!(all_folders.get_value::<u32, _>("Mode").unwrap(), 4);
            assert_eq!(
                all_folders.get_value::<u32, _>("GroupByKey:PID").unwrap(),
                0
            );
            assert!(restore_in(&registry.user).unwrap());
            let view = registry.user.open_subkey(&path).unwrap();
            assert_eq!(
                view.get_value::<String, _>("GroupBy").unwrap(),
                "System.DateModified"
            );
            assert_eq!(
                view.get_value::<String, _>("SortByList").unwrap(),
                "prop:-System.Size"
            );
            let all_folders = registry.user.open_subkey(&all_folders_path).unwrap();
            assert_eq!(all_folders.get_value::<u32, _>("GroupView").unwrap(), 1);
            assert_eq!(
                all_folders.get_raw_value("Sort").unwrap().bytes.as_ref(),
                &[3, 2, 1]
            );
            assert!(
                open_optional(&registry.user, &format!(r"{FOLDER_TYPES}\Generic"))
                    .unwrap()
                    .is_none()
            );
        }
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
        apply_sort(&registry.user, &registry.machine, sort).unwrap();
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
        apply_sort(
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
        apply_sort(
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
        apply_sort(&registry.user, &registry.machine, sort).unwrap();
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
