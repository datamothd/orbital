use super::*;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TweakProfile {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub apps_to_remove: Vec<AppRemoval>,
    pub format: String,
    pub version: u32,
    pub tweaks: Vec<Preference>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disabled_tweaks: Vec<Preference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder_view: Option<FolderView>,
    pub folder_sort: Option<FolderSort>,
    pub folder_grouping: Option<FolderGrouping>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_explorer_to: Option<ExplorerLaunch>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppRemoval {
    pub name: String,
    pub id: String,
    pub kind: AppIdentityKind,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppIdentityKind {
    Appx,
    #[serde(alias = "winget")]
    Desktop,
    Feature,
    Capability,
}

pub enum SettingsCommand {
    Toggle(Preference),
    Recommended,
}

impl TweakProfile {
    pub fn validate(&self) -> io::Result<()> {
        if self.format != "orbital-tweaks" || !matches!(self.version, 1..=5) {
            return Err(io::Error::other(
                "Choose an Orbital tweaks JSON file (version 1, 2, 3, 4 or 5).",
            ));
        }
        if (!self.apps_to_remove.is_empty() && self.version < 5)
            || self.apps_to_remove.len() > 2048
            || self.apps_to_remove.iter().enumerate().any(|(index, app)| {
                app.name.trim().is_empty()
                    || app.name.len() > 512
                    || app.id.trim().is_empty()
                    || app.id.len() > 1024
                    || app.id.chars().any(char::is_control)
                    || self.apps_to_remove[..index].iter().any(|other| {
                        other.kind == app.kind && other.id.eq_ignore_ascii_case(&app.id)
                    })
            })
        {
            return Err(io::Error::other(
                "Invalid or duplicate app removal selections.",
            ));
        }
        let all: Vec<_> = self.tweaks.iter().chain(&self.disabled_tweaks).collect();
        if all
            .iter()
            .enumerate()
            .any(|(index, item)| all[..index].contains(item))
        {
            return Err(io::Error::other(
                "The file contains duplicate or conflicting tweaks.",
            ));
        }
        Ok(())
    }

    pub fn normalize(&mut self) -> io::Result<()> {
        self.validate()?;
        if self.version < 3 {
            if !self.tweaks.contains(&Preference::ShowRecommendations)
                && !self
                    .disabled_tweaks
                    .contains(&Preference::ShowRecommendations)
            {
                for tweaks in [&mut self.tweaks, &mut self.disabled_tweaks] {
                    if tweaks.contains(&Preference::ShowRecentFiles) {
                        tweaks.push(Preference::ShowRecommendations);
                    }
                }
            }
            self.version = 3;
        }
        if self.version < 4 {
            let enabled = std::mem::take(&mut self.tweaks);
            let disabled = std::mem::take(&mut self.disabled_tweaks);
            for (preferences, state) in [(enabled, true), (disabled, false)] {
                for preference in preferences {
                    if state != preference.legacy_inverted() {
                        self.tweaks.push(preference);
                    } else {
                        self.disabled_tweaks.push(preference);
                    }
                }
            }
            self.version = 4;
        }
        Ok(())
    }

    fn change_count(&self) -> usize {
        self.tweaks.len()
            + self.disabled_tweaks.len()
            + usize::from(self.folder_view.is_some())
            + usize::from(self.folder_sort.is_some())
            + usize::from(self.folder_grouping.is_some())
            + usize::from(self.open_explorer_to.is_some())
    }

    fn retain_changes(&mut self, current: &ExplorerSettings) {
        self.open_explorer_to = self
            .open_explorer_to
            .filter(|target| current.open_explorer_to != Some(*target));
        for (tweaks, enabled) in [(&mut self.tweaks, true), (&mut self.disabled_tweaks, false)] {
            tweaks.retain(|tweak| tweak.state(current) != Some(enabled));
        }
        self.folder_view = self
            .folder_view
            .filter(|view| current.folder_view != Some(*view));
        self.folder_sort = self
            .folder_sort
            .filter(|sort| current.folder_sort != Some(*sort));
        self.folder_grouping = self
            .folder_grouping
            .filter(|grouping| current.folder_grouping != Some(*grouping));
    }
}

impl Preference {
    pub(super) const ALL: [Self; 28] = [
        Self::CenterTaskbarIcons,
        Self::ShowTaskView,
        Self::ShowTaskbarFlashing,
        Self::AllowWidgets,
        Self::EnableTaskbarEndTask,
        Self::UseCompactMode,
        Self::UseClassicContextMenu,
        Self::ShowRecentFiles,
        Self::ShowRecommendations,
        Self::ShowFrequentFolders,
        Self::ShowCloudFiles,
        Self::ShowHomeFolder,
        Self::ShowGallery,
        Self::HideShortcutArrow,
        Self::HideFileExtensions,
        Self::ShowHiddenFiles,
        Self::ShowItemCheckboxes,
        Self::ShowLibraries,
        Self::ShowFullPath,
        Self::DoubleClickOpen,
        Self::HideEmptyDrives,
        Self::ConfirmFileDelete,
        Self::IconsOnly,
        Self::ShowInfoTips,
        Self::ShowSystemFiles,
        Self::RestoreFolderWindows,
        Self::ExpandTransferDetails,
        Self::ShowNetwork,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::CenterTaskbarIcons => CENTER_TASKBAR_ICONS.name,
            Self::ShowTaskView => SHOW_TASK_VIEW.name,
            Self::ShowTaskbarFlashing => SHOW_TASKBAR_FLASHING.name,
            Self::AllowWidgets => ALLOW_WIDGETS.name,
            Self::EnableTaskbarEndTask => ENABLE_TASKBAR_END_TASK.name,
            Self::UseCompactMode => USE_COMPACT_MODE.name,
            Self::UseClassicContextMenu => "Use classic context menu",
            Self::ShowRecentFiles => SHOW_RECENT_FILES.name,
            Self::ShowRecommendations => SHOW_RECOMMENDATIONS.name,
            Self::ShowFrequentFolders => SHOW_FREQUENT_FOLDERS.name,
            Self::ShowCloudFiles => SHOW_CLOUD_FILES.name,
            Self::ShowHomeFolder => "Show Home folder",
            Self::ShowGallery => "Show Gallery",
            Self::HideShortcutArrow => "Hide shortcut arrow icon",
            Self::HideFileExtensions => HIDE_FILE_EXTENSIONS.name,
            Self::ShowHiddenFiles => "Show hidden files",
            Self::ShowItemCheckboxes => SHOW_ITEM_CHECKBOXES.name,
            Self::ShowLibraries => SHOW_LIBRARIES.name,
            Self::ShowFullPath => SHOW_FULL_PATH.name,
            Self::DoubleClickOpen => "Double-click to open items",
            Self::HideEmptyDrives => HIDE_EMPTY_DRIVES.name,
            Self::ConfirmFileDelete => CONFIRM_FILE_DELETE.name,
            Self::IconsOnly => ICONS_ONLY.name,
            Self::ShowInfoTips => SHOW_INFO_TIPS.name,
            Self::ShowSystemFiles => SHOW_SYSTEM_FILES.name,
            Self::RestoreFolderWindows => RESTORE_FOLDER_WINDOWS.name,
            Self::ExpandTransferDetails => EXPAND_TRANSFER_DETAILS.name,
            Self::ShowNetwork => SHOW_NETWORK.name,
        }
    }

    fn state(self, current: &ExplorerSettings) -> Option<bool> {
        Some(match self {
            Self::CenterTaskbarIcons => current.center_taskbar_icons,
            Self::ShowTaskView => current.show_task_view,
            Self::ShowTaskbarFlashing => current.show_taskbar_flashing,
            Self::AllowWidgets => current.allow_widgets,
            Self::EnableTaskbarEndTask => current.enable_taskbar_end_task,
            Self::UseCompactMode => current.use_compact_mode,
            Self::UseClassicContextMenu => current.use_classic_context_menu,
            Self::ShowRecentFiles => current.show_recent_files,
            Self::ShowRecommendations => current.show_recommendations,
            Self::ShowFrequentFolders => current.show_frequent_folders,
            Self::ShowCloudFiles => current.show_cloud_files,
            Self::ShowHomeFolder => current.show_home_folder,
            Self::ShowGallery => current.show_gallery,
            Self::HideShortcutArrow => return current.hide_shortcut_arrow,
            Self::HideFileExtensions => current.hide_file_extensions,
            Self::ShowHiddenFiles => current.show_hidden_files,
            Self::ShowItemCheckboxes => current.show_item_checkboxes,
            Self::ShowLibraries => current.show_libraries,
            Self::ShowFullPath => current.show_full_path,
            Self::DoubleClickOpen => current.double_click_open,
            Self::HideEmptyDrives => current.hide_empty_drives,
            Self::ConfirmFileDelete => current.confirm_file_delete,
            Self::IconsOnly => current.icons_only,
            Self::ShowInfoTips => current.show_info_tips,
            Self::ShowSystemFiles => current.show_system_files,
            Self::RestoreFolderWindows => current.restore_folder_windows,
            Self::ExpandTransferDetails => current.expand_transfer_details,
            Self::ShowNetwork => current.show_network,
        })
    }

    fn supported(self, current: &ExplorerSettings) -> bool {
        match self {
            Self::EnableTaskbarEndTask => current.taskbar_end_task_supported,
            Self::ConfirmFileDelete => current.confirm_file_delete_supported,
            Self::ShowHomeFolder => current.home_supported,
            Self::ShowGallery => current.gallery_supported,
            Self::ShowTaskbarFlashing
            | Self::AllowWidgets
            | Self::CenterTaskbarIcons
            | Self::UseCompactMode
            | Self::ShowRecommendations
            | Self::UseClassicContextMenu
            | Self::ShowCloudFiles => current.windows11_supported,
            _ => true,
        }
    }

    fn requires_administrator(self) -> bool {
        matches!(
            self,
            Self::AllowWidgets | Self::ShowHomeFolder | Self::ShowGallery | Self::HideShortcutArrow
        )
    }

    fn legacy_inverted(self) -> bool {
        matches!(
            self,
            Self::CenterTaskbarIcons
                | Self::ShowTaskView
                | Self::AllowWidgets
                | Self::ShowRecentFiles
                | Self::ShowRecommendations
                | Self::ShowFrequentFolders
                | Self::ShowCloudFiles
                | Self::ShowHomeFolder
                | Self::ShowGallery
                | Self::HideFileExtensions
                | Self::DoubleClickOpen
                | Self::HideEmptyDrives
                | Self::ShowInfoTips
        )
    }

    pub(super) fn recommended_value(self) -> Option<bool> {
        match self {
            Self::ShowTaskView
            | Self::ShowTaskbarFlashing
            | Self::AllowWidgets
            | Self::ShowRecentFiles
            | Self::ShowRecommendations
            | Self::ShowFrequentFolders
            | Self::ShowCloudFiles
            | Self::ShowHomeFolder
            | Self::ShowGallery
            | Self::ShowNetwork
            | Self::HideFileExtensions => Some(false),
            Self::EnableTaskbarEndTask
            | Self::UseCompactMode
            | Self::UseClassicContextMenu
            | Self::HideShortcutArrow
            | Self::ExpandTransferDetails
            | Self::ShowHiddenFiles
            | Self::ShowFullPath => Some(true),
            _ => None,
        }
    }
}

pub fn export_tweaks() -> io::Result<TweakProfile> {
    Ok(profile_from_settings(&settings()?))
}

fn profile_from_settings(current: &ExplorerSettings) -> TweakProfile {
    let (tweaks, disabled_tweaks) = Preference::ALL
        .into_iter()
        .filter(|preference| preference.supported(current) && preference.state(current).is_some())
        .partition(|preference| preference.state(current) == Some(true));
    TweakProfile {
        apps_to_remove: vec![],
        format: "orbital-tweaks".into(),
        version: 4,
        tweaks,
        disabled_tweaks,
        folder_view: current.folder_view,
        folder_sort: current.folder_sort,
        folder_grouping: current.folder_grouping,
        open_explorer_to: current
            .open_explorer_to
            .filter(|target| current.explorer_launch_options.contains(target)),
    }
}

fn batch_profile(
    commands: &[SettingsCommand],
    current: &ExplorerSettings,
) -> io::Result<TweakProfile> {
    let mut profile = profile_from_settings(current);
    profile.open_explorer_to = None;
    for command in commands {
        match command {
            SettingsCommand::Toggle(preference) => {
                if !preference.supported(current) {
                    return Err(io::Error::other(format!(
                        "{} is unavailable on this system",
                        preference.name()
                    )));
                }
                if let Some(index) = profile.tweaks.iter().position(|item| item == preference) {
                    profile.tweaks.remove(index);
                    profile.disabled_tweaks.push(*preference);
                } else {
                    profile.disabled_tweaks.retain(|item| item != preference);
                    profile.tweaks.push(*preference);
                }
            }
            SettingsCommand::Recommended => {
                for preference in Preference::ALL {
                    if let Some(enabled) = preference
                        .recommended_value()
                        .filter(|_| preference.supported(current))
                    {
                        profile.tweaks.retain(|item| *item != preference);
                        profile.disabled_tweaks.retain(|item| *item != preference);
                        if enabled {
                            profile.tweaks.push(preference);
                        } else {
                            profile.disabled_tweaks.push(preference);
                        }
                    }
                }
                profile.folder_view = Some(FolderView::Details);
                profile.folder_sort = Some(FolderSort {
                    column: SortColumn::Type,
                    descending: false,
                });
                profile.folder_grouping = Some(FolderGrouping::None);
                if current
                    .explorer_launch_options
                    .contains(&ExplorerLaunch::Downloads)
                {
                    profile.open_explorer_to = Some(ExplorerLaunch::Downloads);
                }
            }
        }
    }
    profile.retain_changes(current);
    Ok(profile)
}

pub fn apply_batch(commands: &[SettingsCommand], yes: bool) -> io::Result<()> {
    let profile = batch_profile(commands, &settings()?)?;
    if profile.change_count() == 0 {
        println!("No settings need to change.");
        return Ok(());
    }
    println!("Planned changes:");
    for (preferences, state) in [
        (&profile.tweaks, "enabled"),
        (&profile.disabled_tweaks, "disabled"),
    ] {
        for preference in preferences {
            println!("  - {}: {state}", preference.name());
        }
    }
    if profile.folder_view.is_some() {
        println!("  - Set default folder view to Details");
    }
    if profile.folder_sort.is_some() {
        println!("  - Sort folders by Type - A to Z");
    }
    if profile.folder_grouping.is_some() {
        println!("  - Set folder grouping to None");
    }
    if profile.open_explorer_to == Some(ExplorerLaunch::Downloads) {
        println!("  - Open Explorer to Downloads");
    }
    if !yes && !confirm("Apply these changes and restart Explorer?")? {
        println!("Cancelled.");
        return Ok(());
    }
    let changed = apply_imported_tweaks(profile, true)?;
    println!("{changed} settings applied.");
    Ok(())
}

pub fn set_explorer_launch(target: ExplorerLaunch) -> io::Result<()> {
    apply_imported_tweaks(
        TweakProfile {
            apps_to_remove: vec![],
            format: "orbital-tweaks".into(),
            version: 4,
            tweaks: vec![],
            disabled_tweaks: vec![],
            folder_view: None,
            folder_sort: None,
            folder_grouping: None,
            open_explorer_to: Some(target),
        },
        false,
    )
    .map(|_| ())
}

pub fn toggle_preference(preference: Preference, replace_blank_icon: bool) -> io::Result<()> {
    let current = settings()?;
    let profile = batch_profile(&[SettingsCommand::Toggle(preference)], &current)?;
    apply_profile(profile, &current, replace_blank_icon).map(|_| ())
}

fn imported_changes<'a>(
    user: &'a RegKey,
    machine: &'a RegKey,
    tweaks: &[Preference],
    disabled_tweaks: &[Preference],
    icon: &Path,
) -> io::Result<Vec<RegistryChange<'a>>> {
    let mut changes = Vec::new();
    let icon_value = icon.as_os_str();
    for (tweak, enabled) in tweaks
        .iter()
        .map(|tweak| (tweak, true))
        .chain(disabled_tweaks.iter().map(|tweak| (tweak, false)))
    {
        let change = match tweak {
            Preference::CenterTaskbarIcons => CENTER_TASKBAR_ICONS.change_in(user, enabled),
            Preference::ShowTaskView => SHOW_TASK_VIEW.change_in(user, enabled),
            Preference::ShowTaskbarFlashing => SHOW_TASKBAR_FLASHING.change_in(user, enabled),
            Preference::AllowWidgets => ALLOW_WIDGETS.change_in(machine, enabled),
            Preference::EnableTaskbarEndTask => ENABLE_TASKBAR_END_TASK.change_in(user, enabled),
            Preference::UseCompactMode => USE_COMPACT_MODE.change_in(user, enabled),
            Preference::ShowRecentFiles => SHOW_RECENT_FILES.change_in(user, enabled),
            Preference::ShowRecommendations => SHOW_RECOMMENDATIONS.change_in(user, enabled),
            Preference::ShowFrequentFolders => SHOW_FREQUENT_FOLDERS.change_in(user, enabled),
            Preference::ShowCloudFiles => SHOW_CLOUD_FILES.change_in(user, enabled),
            Preference::HideFileExtensions => HIDE_FILE_EXTENSIONS.change_in(user, enabled),
            Preference::ShowHiddenFiles => RegistryChange::new(
                user,
                ADVANCED_KEY,
                "Hidden",
                Some((if enabled { 1u32 } else { 2u32 }).to_reg_value()),
            ),
            Preference::ShowItemCheckboxes => SHOW_ITEM_CHECKBOXES.change_in(user, enabled),
            Preference::ShowLibraries => SHOW_LIBRARIES.change_in(user, enabled),
            Preference::ShowFullPath => SHOW_FULL_PATH.change_in(user, enabled),
            Preference::DoubleClickOpen => double_click_change(user, enabled)?,
            Preference::HideEmptyDrives => HIDE_EMPTY_DRIVES.change_in(user, enabled),
            Preference::ConfirmFileDelete => CONFIRM_FILE_DELETE.change_in(user, enabled),
            Preference::IconsOnly => ICONS_ONLY.change_in(user, enabled),
            Preference::ShowInfoTips => SHOW_INFO_TIPS.change_in(user, enabled),
            Preference::ShowSystemFiles => SHOW_SYSTEM_FILES.change_in(user, enabled),
            Preference::RestoreFolderWindows => RESTORE_FOLDER_WINDOWS.change_in(user, enabled),
            Preference::ExpandTransferDetails => EXPAND_TRANSFER_DETAILS.change_in(user, enabled),
            Preference::ShowNetwork => SHOW_NETWORK.change_in(user, enabled),
            Preference::UseClassicContextMenu => RegistryChange::new(
                user,
                CLASSIC_CONTEXT_MENU_KEY,
                "",
                enabled.then(|| "".to_reg_value()),
            ),
            Preference::ShowHomeFolder => {
                changes.extend(namespace_changes(user, machine, HOME_CLSID, enabled));
                continue;
            }
            Preference::ShowGallery => {
                changes.extend(namespace_changes(user, machine, GALLERY_CLSID, enabled));
                continue;
            }
            Preference::HideShortcutArrow => RegistryChange::new(
                machine,
                SHELL_ICONS_KEY,
                "29",
                enabled.then(|| icon_value.to_reg_value()),
            ),
        };
        changes.push(change);
    }
    Ok(changes)
}

pub fn apply_imported_tweaks(
    mut profile: TweakProfile,
    replace_blank_icon: bool,
) -> io::Result<usize> {
    profile.normalize()?;
    apply_profile(profile, &settings()?, replace_blank_icon)
}

fn apply_profile(
    mut profile: TweakProfile,
    current: &ExplorerSettings,
    replace_blank_icon: bool,
) -> io::Result<usize> {
    profile.retain_changes(current);
    if profile
        .open_explorer_to
        .is_some_and(|target| !current.explorer_launch_options.contains(&target))
    {
        return Err(io::Error::other(
            "This Explorer starting location is unavailable on this system.",
        ));
    }
    let selected = || profile.tweaks.iter().chain(&profile.disabled_tweaks);
    if selected().any(|tweak| !tweak.supported(current)) {
        return Err(io::Error::other(
            "Some imported tweaks are unavailable on this system.",
        ));
    }
    if selected().any(|tweak| tweak.requires_administrator() && !current.is_administrator) {
        return Err(io::Error::other(
            "This preference requires administrator access. Close Orbital and run it as administrator.",
        ));
    }
    let count = profile.change_count();
    if count == 0 {
        return Ok(0);
    }
    let icon = blank_icon_path()?;
    let mut changes =
        imported_changes(HKCU, HKLM, &profile.tweaks, &profile.disabled_tweaks, &icon)?;
    if let Some(target) = profile.open_explorer_to {
        changes.push(RegistryChange::new(
            HKCU,
            ADVANCED_KEY,
            "LaunchTo",
            Some(target.value().to_reg_value()),
        ));
    }
    let changes = pending_changes(changes)?;
    save_original_settings()?;
    let prepared = prepare_changes(&changes)?;
    if profile.tweaks.contains(&Preference::HideShortcutArrow) {
        ensure_blank_icon_at(&icon, replace_blank_icon)?;
    }
    let views: Vec<_> = [
        profile.folder_view.map(folder_views::ViewChange::Mode),
        profile.folder_sort.map(folder_views::ViewChange::Sort),
        profile
            .folder_grouping
            .map(folder_views::ViewChange::Grouping),
    ]
    .into_iter()
    .flatten()
    .collect();
    folder_views::apply_in(HKCU, HKLM, &views, || apply_changes(&changes, &prepared))?;
    restart_explorer(
        "Settings",
        selected().any(|tweak| *tweak == Preference::HideShortcutArrow),
        !views.is_empty(),
    )?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_toggles_leave_other_preferences_and_folder_views_out_of_the_profile() {
        let mut current = ExplorerSettings {
            windows11_supported: true,
            home_supported: true,
            gallery_supported: true,
            taskbar_end_task_supported: true,
            confirm_file_delete_supported: true,
            hide_shortcut_arrow: Some(false),
            folder_view: None,
            folder_sort: Some(FolderSort {
                column: SortColumn::Size,
                descending: true,
            }),
            folder_grouping: Some(FolderGrouping::DateModified),
            ..Default::default()
        };
        for preference in Preference::ALL {
            let profile = batch_profile(&[SettingsCommand::Toggle(preference)], &current).unwrap();
            assert_eq!(profile.tweaks, [preference]);
            assert!(profile.disabled_tweaks.is_empty());
            assert_eq!(profile.change_count(), 1);
            assert_eq!(profile.folder_sort, None);
            assert_eq!(profile.folder_grouping, None);
        }
        current.hide_shortcut_arrow = None;
        let profile = batch_profile(
            &[SettingsCommand::Toggle(Preference::HideShortcutArrow)],
            &current,
        )
        .unwrap();
        assert_eq!(profile.tweaks, [Preference::HideShortcutArrow]);
        assert_eq!(profile.change_count(), 1);
    }

    #[test]
    fn batch_keeps_only_final_selected_changes() {
        let current = ExplorerSettings {
            show_recent_files: true,
            ..Default::default()
        };
        let profile = batch_profile(
            &[
                SettingsCommand::Toggle(Preference::ShowRecentFiles),
                SettingsCommand::Toggle(Preference::ShowTaskView),
                SettingsCommand::Toggle(Preference::ShowFrequentFolders),
                SettingsCommand::Toggle(Preference::ShowTaskView),
            ],
            &current,
        )
        .unwrap();
        assert_eq!(profile.tweaks, [Preference::ShowFrequentFolders]);
        assert_eq!(profile.disabled_tweaks, [Preference::ShowRecentFiles]);
        assert_eq!(profile.folder_sort, None);
        assert_eq!(profile.folder_grouping, None);
        profile.validate().unwrap();
    }

    #[test]
    fn batch_recommendations_respect_command_order_and_preserve_alignment() {
        let current = ExplorerSettings {
            windows11_supported: true,
            ..Default::default()
        };
        for (commands, recent) in [
            (
                [
                    SettingsCommand::Recommended,
                    SettingsCommand::Toggle(Preference::ShowRecentFiles),
                ],
                true,
            ),
            (
                [
                    SettingsCommand::Toggle(Preference::ShowRecentFiles),
                    SettingsCommand::Recommended,
                ],
                false,
            ),
            (
                [SettingsCommand::Recommended, SettingsCommand::Recommended],
                false,
            ),
        ] {
            let profile = batch_profile(&commands, &current).unwrap();
            assert_eq!(
                profile.tweaks.contains(&Preference::ShowRecentFiles),
                recent
            );
            assert!(profile.tweaks.contains(&Preference::HideShortcutArrow));
            assert!(!profile.tweaks.contains(&Preference::CenterTaskbarIcons));
            assert!(!profile.tweaks.contains(&Preference::EnableTaskbarEndTask));
            assert!(!profile.tweaks.contains(&Preference::ShowHomeFolder));
            assert!(!profile.tweaks.contains(&Preference::ShowGallery));
            assert!(profile.disabled_tweaks.is_empty());
            assert_eq!(
                profile.folder_sort,
                Some(FolderSort {
                    column: SortColumn::Type,
                    descending: false
                })
            );
            assert_eq!(profile.folder_grouping, Some(FolderGrouping::None));
            profile.validate().unwrap();
        }
    }

    #[test]
    fn recommendations_include_personal_defaults_and_preserve_manual_choices() {
        for enabled in [false, true] {
            let current = ExplorerSettings {
                windows11_supported: true,
                use_compact_mode: enabled,
                use_classic_context_menu: enabled,
                show_home_folder: enabled,
                home_supported: true,
                show_gallery: enabled,
                gallery_supported: true,
                hide_file_extensions: true,
                show_item_checkboxes: enabled,
                show_libraries: enabled,
                show_full_path: enabled,
                show_hidden_files: enabled,
                show_network: enabled,
                restore_folder_windows: enabled,
                confirm_file_delete: enabled,
                confirm_file_delete_supported: true,
                folder_view: Some(FolderView::LargeIcons),
                folder_sort: Some(FolderSort {
                    column: SortColumn::Size,
                    descending: true,
                }),
                folder_grouping: Some(FolderGrouping::DateCreated),
                open_explorer_to: Some(ExplorerLaunch::ThisPc),
                explorer_launch_options: vec![
                    ExplorerLaunch::Home,
                    ExplorerLaunch::ThisPc,
                    ExplorerLaunch::Downloads,
                ],
                ..Default::default()
            };
            let profile = batch_profile(&[SettingsCommand::Recommended], &current).unwrap();
            assert!(
                profile
                    .disabled_tweaks
                    .contains(&Preference::HideFileExtensions)
            );
            assert_eq!(profile.folder_view, Some(FolderView::Details));
            assert_eq!(profile.open_explorer_to, Some(ExplorerLaunch::Downloads));
            assert_eq!(profile.tweaks.contains(&Preference::ShowFullPath), !enabled);
            assert_eq!(
                profile.tweaks.contains(&Preference::ShowHiddenFiles),
                !enabled
            );
            assert!(!profile.tweaks.contains(&Preference::ShowNetwork));
            assert_eq!(
                profile.disabled_tweaks.contains(&Preference::ShowNetwork),
                enabled
            );
            assert_eq!(
                profile.folder_sort,
                Some(FolderSort {
                    column: SortColumn::Type,
                    descending: false
                })
            );
            assert_eq!(profile.folder_grouping, Some(FolderGrouping::None));
            assert_eq!(
                profile.tweaks.contains(&Preference::UseCompactMode),
                !enabled
            );
            assert_eq!(
                profile.tweaks.contains(&Preference::UseClassicContextMenu),
                !enabled
            );
            assert_eq!(
                profile
                    .disabled_tweaks
                    .contains(&Preference::ShowHomeFolder),
                enabled
            );
            assert_eq!(
                profile.disabled_tweaks.contains(&Preference::ShowGallery),
                enabled
            );
            for preference in [
                Preference::RestoreFolderWindows,
                Preference::ShowItemCheckboxes,
                Preference::ShowLibraries,
                Preference::ConfirmFileDelete,
                Preference::ShowSystemFiles,
                Preference::DoubleClickOpen,
                Preference::IconsOnly,
            ] {
                assert!(!profile.tweaks.contains(&preference));
                assert!(!profile.disabled_tweaks.contains(&preference));
                assert_eq!(preference.recommended_value(), None);
            }
        }
    }

    #[test]
    fn batch_rejects_unavailable_tweaks_before_applying() {
        let result = batch_profile(
            &[
                SettingsCommand::Toggle(Preference::ShowRecentFiles),
                SettingsCommand::Toggle(Preference::EnableTaskbarEndTask),
            ],
            &ExplorerSettings::default(),
        );
        assert!(result.unwrap_err().to_string().contains("unavailable"));
    }

    #[test]
    fn mixed_recent_and_recommendation_states_survive_export() {
        for recent in [false, true] {
            for recommendations in [false, true] {
                let current = ExplorerSettings {
                    windows11_supported: true,
                    show_recent_files: recent,
                    show_recommendations: recommendations,
                    ..Default::default()
                };
                let profile = profile_from_settings(&current);
                for (preference, enabled) in [
                    (Preference::ShowRecentFiles, recent),
                    (Preference::ShowRecommendations, recommendations),
                ] {
                    assert_eq!(profile.tweaks.contains(&preference), enabled);
                    assert_eq!(profile.disabled_tweaks.contains(&preference), !enabled);
                }
            }
        }
    }

    #[test]
    fn legacy_profiles_expand_combined_preferences_once() {
        for version in [1, 2, 3] {
            for enabled in [false, true] {
                let mut profile = TweakProfile {
                    apps_to_remove: vec![],
                    format: "orbital-tweaks".into(),
                    version,
                    tweaks: if enabled {
                        vec![Preference::ShowRecentFiles]
                    } else {
                        vec![]
                    },
                    disabled_tweaks: if enabled {
                        vec![]
                    } else {
                        vec![Preference::ShowRecentFiles]
                    },
                    folder_view: None,
                    folder_sort: None,
                    folder_grouping: None,
                    open_explorer_to: None,
                };
                profile.normalize().unwrap();
                profile.normalize().unwrap();
                profile.validate().unwrap();
                let selected = if enabled {
                    &profile.disabled_tweaks
                } else {
                    &profile.tweaks
                };
                assert_eq!(
                    selected.contains(&Preference::ShowRecommendations),
                    version < 3
                );
                assert_eq!(profile.version, 4);
            }
        }
    }

    #[test]
    fn unavailable_features_are_omitted_from_exports() {
        let current = ExplorerSettings::default();
        let profile = profile_from_settings(&current);
        for preference in [
            Preference::ShowHomeFolder,
            Preference::ShowGallery,
            Preference::UseCompactMode,
            Preference::CenterTaskbarIcons,
            Preference::ShowRecommendations,
        ] {
            assert!(!profile.disabled_tweaks.contains(&preference));
            assert!(!profile.tweaks.contains(&preference));
        }
    }

    #[test]
    fn export_preserves_enabled_and_disabled_supported_tweaks_and_known_folder_values() {
        let mut current = ExplorerSettings {
            is_administrator: false,
            desktop_layout_safe_mode: false,
            windows11_supported: true,
            folder_view: None,
            folder_sort: Some(FolderSort {
                column: SortColumn::Size,
                descending: true,
            }),
            folder_grouping: Some(FolderGrouping::None),
            center_taskbar_icons: true,
            show_task_view: false,
            allow_widgets: false,
            enable_taskbar_end_task: false,
            taskbar_end_task_supported: true,
            use_compact_mode: false,
            use_classic_context_menu: true,
            show_recent_files: true,
            show_recommendations: false,
            show_frequent_folders: false,
            show_cloud_files: false,
            show_home_folder: false,
            home_supported: true,
            show_gallery: false,
            gallery_supported: true,
            hide_shortcut_arrow: None,
            ..Default::default()
        };
        let profile = profile_from_settings(&current);
        assert_eq!(
            profile.tweaks,
            [
                Preference::CenterTaskbarIcons,
                Preference::UseClassicContextMenu,
                Preference::ShowRecentFiles
            ]
        );
        assert_eq!(profile.folder_sort, current.folder_sort);
        assert_eq!(profile.version, 4);
        assert!(
            profile
                .disabled_tweaks
                .contains(&Preference::UseCompactMode)
        );
        assert!(profile.disabled_tweaks.contains(&Preference::ShowGallery));
        for preference in [
            Preference::ShowTaskView,
            Preference::AllowWidgets,
            Preference::EnableTaskbarEndTask,
        ] {
            assert!(profile.disabled_tweaks.contains(&preference));
        }
        current.taskbar_end_task_supported = false;
        assert!(
            !profile_from_settings(&current)
                .disabled_tweaks
                .contains(&Preference::EnableTaskbarEndTask)
        );
        assert!(
            !profile
                .disabled_tweaks
                .contains(&Preference::HideShortcutArrow)
        );
        current.hide_shortcut_arrow = Some(false);
        assert_eq!(Preference::HideShortcutArrow.state(&current), Some(false));
        assert!(
            profile_from_settings(&current)
                .disabled_tweaks
                .contains(&Preference::HideShortcutArrow)
        );
        current.hide_shortcut_arrow = None;
        assert_eq!(Preference::HideShortcutArrow.state(&current), None);
        assert_eq!(profile.folder_grouping, Some(FolderGrouping::None));
        current.windows11_supported = false;
        assert!(
            !profile_from_settings(&current)
                .disabled_tweaks
                .contains(&Preference::AllowWidgets)
        );
        assert!(
            !profile_from_settings(&current)
                .tweaks
                .contains(&Preference::UseClassicContextMenu)
        );
    }

    #[test]
    fn imports_only_generate_writes_for_selected_tweaks() {
        let changes = imported_changes(
            HKCU,
            HKLM,
            &[Preference::CenterTaskbarIcons, Preference::ShowRecentFiles],
            &[],
            Path::new("unused.ico"),
        )
        .unwrap();
        assert_eq!(
            changes
                .iter()
                .map(|change| change.name.as_str())
                .collect::<Vec<_>>(),
            ["TaskbarAl", "ShowRecent"]
        );
        assert!(
            imported_changes(HKCU, HKLM, &[], &[], Path::new("unused.ico"))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn importing_disabled_values_restores_enabled_registry_preferences() {
        let registry = super::super::tests::TestRegistry::new();
        let user = registry.key.create_subkey("User").unwrap().0;
        let machine = registry.key.create_subkey("Machine").unwrap().0;
        let tweaks = Preference::ALL;
        let icon = Path::new("test-import-icon.ico");
        for enabled in [true, false] {
            let changes = if enabled {
                imported_changes(&user, &machine, &tweaks, &[], icon)
            } else {
                imported_changes(&user, &machine, &[], &tweaks, icon)
            }
            .unwrap();
            let prepared = prepare_changes(&changes).unwrap();
            apply_changes(&changes, &prepared).unwrap();
            for preference in [
                &CENTER_TASKBAR_ICONS,
                &SHOW_TASK_VIEW,
                &ENABLE_TASKBAR_END_TASK,
                &USE_COMPACT_MODE,
                &SHOW_RECENT_FILES,
                &SHOW_RECOMMENDATIONS,
                &SHOW_FREQUENT_FOLDERS,
                &SHOW_CLOUD_FILES,
            ] {
                assert_eq!(preference.enabled_in(&user).unwrap(), enabled);
            }
            assert_eq!(ALLOW_WIDGETS.enabled_in(&machine).unwrap(), enabled);
            for clsid in [HOME_CLSID, GALLERY_CLSID] {
                assert_eq!(
                    namespace_visible_in(&user, &machine, clsid).unwrap(),
                    enabled
                );
            }
            assert_eq!(
                read_original(&user, CLASSIC_CONTEXT_MENU_KEY, "")
                    .unwrap()
                    .is_some(),
                enabled
            );
            assert_eq!(
                read_original(&machine, SHELL_ICONS_KEY, "29")
                    .unwrap()
                    .is_some(),
                enabled
            );
        }
    }

    #[test]
    fn profile_rejects_wrong_format_version_and_duplicate_tweaks() {
        let mut profile = TweakProfile {
            apps_to_remove: vec![],
            format: "orbital-tweaks".into(),
            version: 1,
            tweaks: vec![],
            disabled_tweaks: vec![],
            folder_view: None,
            folder_sort: None,
            folder_grouping: None,
            open_explorer_to: None,
        };
        assert!(profile.validate().is_ok());
        profile.version = 6;
        assert!(profile.validate().is_err());
        profile.version = 1;
        profile.format = "other-app".into();
        assert!(profile.validate().is_err());
        profile.format = "orbital-tweaks".into();
        profile.tweaks = vec![Preference::ShowRecentFiles; 2];
        assert!(profile.validate().is_err());
        profile.tweaks.pop();
        profile.disabled_tweaks = vec![Preference::ShowRecentFiles];
        assert!(profile.validate().is_err());
    }
}
