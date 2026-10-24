use super::*;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TweakProfile {
    pub format: String,
    pub version: u32,
    pub tweaks: Vec<Preference>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disabled_tweaks: Vec<Preference>,
    pub folder_sort: Option<FolderSort>,
    pub folder_grouping: Option<FolderGrouping>,
}

pub enum SettingsCommand {
    Toggle(Preference),
    Recommended,
}

impl TweakProfile {
    pub fn validate(&self) -> io::Result<()> {
        if self.format != "orbital-tweaks" || !matches!(self.version, 1..=3) {
            return Err(io::Error::other(
                "Choose an Orbital tweaks JSON file (version 1, 2 or 3).",
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
            if !self.tweaks.contains(&Preference::HideRecommendations)
                && !self
                    .disabled_tweaks
                    .contains(&Preference::HideRecommendations)
            {
                for tweaks in [&mut self.tweaks, &mut self.disabled_tweaks] {
                    if tweaks.contains(&Preference::HideRecentFiles) {
                        tweaks.push(Preference::HideRecommendations);
                    }
                }
            }
            self.version = 3;
        }
        Ok(())
    }

    fn retain_changes(&mut self, current: &ExplorerSettings) {
        for (tweaks, enabled) in [(&mut self.tweaks, true), (&mut self.disabled_tweaks, false)] {
            tweaks.retain(|tweak| tweak.state(current) != Some(enabled));
        }
        self.folder_sort = self
            .folder_sort
            .filter(|sort| current.folder_sort != Some(*sort));
        self.folder_grouping = self
            .folder_grouping
            .filter(|grouping| current.folder_grouping != Some(*grouping));
    }
}

impl Preference {
    fn name(self) -> &'static str {
        match self {
            Self::DisableTaskbarCentering => DISABLE_TASKBAR_CENTERING.name,
            Self::HideTaskView => HIDE_TASK_VIEW.name,
            Self::DisableWidgets => DISABLE_WIDGETS.name,
            Self::EnableTaskbarEndTask => ENABLE_TASKBAR_END_TASK.name,
            Self::DisableExplorerSpacing => DISABLE_EXPLORER_SPACING.name,
            Self::DisableModernContextMenu => "Disable Windows 11 context menu",
            Self::HideRecentFiles => HIDE_RECENT_FILES.name,
            Self::HideRecommendations => HIDE_RECOMMENDATIONS.name,
            Self::HideFrequentFolders => HIDE_FREQUENT_FOLDERS.name,
            Self::HideOfficeFiles => HIDE_OFFICE_FILES.name,
            Self::HideHomeFolder => "Hide Home folder",
            Self::HideGallery => "Hide Gallery",
            Self::HideShortcutArrow => "Hide shortcut arrow icon",
        }
    }

    fn state(self, current: &ExplorerSettings) -> Option<bool> {
        Some(match self {
            Self::DisableTaskbarCentering => current.disable_taskbar_centering,
            Self::HideTaskView => current.hide_task_view,
            Self::DisableWidgets => current.disable_widgets,
            Self::EnableTaskbarEndTask => current.enable_taskbar_end_task,
            Self::DisableExplorerSpacing => current.disable_explorer_spacing,
            Self::DisableModernContextMenu => current.disable_modern_context_menu,
            Self::HideRecentFiles => current.hide_recent_files,
            Self::HideRecommendations => current.hide_recommendations,
            Self::HideFrequentFolders => current.hide_frequent_folders,
            Self::HideOfficeFiles => current.hide_office_files,
            Self::HideHomeFolder => current.hide_home_folder,
            Self::HideGallery => current.hide_gallery,
            Self::HideShortcutArrow => return current.hide_shortcut_arrow,
        })
    }

    fn supported(self, current: &ExplorerSettings) -> bool {
        match self {
            Self::EnableTaskbarEndTask => current.taskbar_end_task_supported,
            Self::HideHomeFolder => current.home_supported,
            Self::HideGallery => current.gallery_supported,
            Self::DisableWidgets
            | Self::DisableTaskbarCentering
            | Self::DisableExplorerSpacing
            | Self::HideRecommendations
            | Self::DisableModernContextMenu
            | Self::HideOfficeFiles => current.windows11_supported,
            _ => true,
        }
    }

    fn requires_administrator(self) -> bool {
        matches!(
            self,
            Self::DisableWidgets
                | Self::HideHomeFolder
                | Self::HideGallery
                | Self::HideShortcutArrow
        )
    }
}

pub fn export_tweaks() -> io::Result<TweakProfile> {
    Ok(profile_from_settings(&settings()?))
}

fn profile_from_settings(current: &ExplorerSettings) -> TweakProfile {
    let (tweaks, disabled_tweaks) = [
        Preference::DisableTaskbarCentering,
        Preference::HideTaskView,
        Preference::DisableWidgets,
        Preference::EnableTaskbarEndTask,
        Preference::DisableExplorerSpacing,
        Preference::DisableModernContextMenu,
        Preference::HideRecentFiles,
        Preference::HideRecommendations,
        Preference::HideFrequentFolders,
        Preference::HideOfficeFiles,
        Preference::HideHomeFolder,
        Preference::HideGallery,
        Preference::HideShortcutArrow,
    ]
    .into_iter()
    .filter(|preference| preference.supported(current) && preference.state(current).is_some())
    .partition(|preference| preference.state(current) == Some(true));
    TweakProfile {
        format: "orbital-tweaks".into(),
        version: 3,
        tweaks,
        disabled_tweaks,
        folder_sort: current.folder_sort,
        folder_grouping: current.folder_grouping,
    }
}

fn batch_profile(
    commands: &[SettingsCommand],
    current: &ExplorerSettings,
) -> io::Result<TweakProfile> {
    let mut profile = profile_from_settings(current);
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
                for preference in std::mem::take(&mut profile.disabled_tweaks) {
                    if preference == Preference::DisableTaskbarCentering {
                        profile.disabled_tweaks.push(preference);
                    } else {
                        profile.tweaks.push(preference);
                    }
                }
                if !profile.tweaks.contains(&Preference::HideShortcutArrow) {
                    profile.tweaks.push(Preference::HideShortcutArrow);
                }
                profile.folder_sort = Some(FolderSort {
                    column: SortColumn::Type,
                    descending: false,
                });
                profile.folder_grouping = Some(FolderGrouping::None);
            }
        }
    }
    profile.retain_changes(current);
    Ok(profile)
}

pub fn apply_batch(commands: &[SettingsCommand], yes: bool) -> io::Result<()> {
    let profile = batch_profile(commands, &settings()?)?;
    if profile.tweaks.is_empty()
        && profile.disabled_tweaks.is_empty()
        && profile.folder_sort.is_none()
        && profile.folder_grouping.is_none()
    {
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
    if profile.folder_sort.is_some() {
        println!("  - Sort folders by Type - A to Z");
    }
    if profile.folder_grouping.is_some() {
        println!("  - Set folder grouping to None");
    }
    if !yes && !confirm("Apply these changes and restart Explorer?")? {
        println!("Cancelled.");
        return Ok(());
    }
    let changed = apply_imported_tweaks(profile, true)?;
    println!("{changed} settings applied.");
    Ok(())
}

fn imported_changes<'a>(
    user: &'a RegKey,
    machine: &'a RegKey,
    tweaks: &[Preference],
    disabled_tweaks: &[Preference],
    icon: &Path,
) -> Vec<RegistryChange<'a>> {
    let mut changes = Vec::new();
    let icon_value = icon.as_os_str();
    for (tweak, enabled) in tweaks
        .iter()
        .map(|tweak| (tweak, true))
        .chain(disabled_tweaks.iter().map(|tweak| (tweak, false)))
    {
        let change = match tweak {
            Preference::DisableTaskbarCentering => {
                DISABLE_TASKBAR_CENTERING.change_in(user, enabled)
            }
            Preference::HideTaskView => HIDE_TASK_VIEW.change_in(user, enabled),
            Preference::DisableWidgets => DISABLE_WIDGETS.change_in(machine, enabled),
            Preference::EnableTaskbarEndTask => ENABLE_TASKBAR_END_TASK.change_in(user, enabled),
            Preference::DisableExplorerSpacing => DISABLE_EXPLORER_SPACING.change_in(user, enabled),
            Preference::HideRecentFiles => HIDE_RECENT_FILES.change_in(user, enabled),
            Preference::HideRecommendations => HIDE_RECOMMENDATIONS.change_in(user, enabled),
            Preference::HideFrequentFolders => HIDE_FREQUENT_FOLDERS.change_in(user, enabled),
            Preference::HideOfficeFiles => HIDE_OFFICE_FILES.change_in(user, enabled),
            Preference::DisableModernContextMenu => RegistryChange::new(
                user,
                CLASSIC_CONTEXT_MENU_KEY,
                "",
                enabled.then(|| "".to_reg_value()),
            ),
            Preference::HideHomeFolder => {
                changes.extend(namespace_changes(user, machine, HOME_CLSID, !enabled));
                continue;
            }
            Preference::HideGallery => {
                changes.extend(namespace_changes(user, machine, GALLERY_CLSID, !enabled));
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
    changes
}

pub fn apply_imported_tweaks(
    mut profile: TweakProfile,
    replace_blank_icon: bool,
) -> io::Result<usize> {
    profile.normalize()?;
    let current = settings()?;
    profile.retain_changes(&current);
    let selected = || profile.tweaks.iter().chain(&profile.disabled_tweaks);
    if selected().any(|tweak| !tweak.supported(&current)) {
        return Err(io::Error::other(
            "Some imported tweaks are unavailable on this system.",
        ));
    }
    if selected().any(|tweak| tweak.requires_administrator() && !current.is_administrator) {
        return Err(io::Error::other(
            "This preference requires administrator access. Close Orbital and run it as administrator.",
        ));
    }
    let mut views = Vec::new();
    if let Some(sort) = profile.folder_sort {
        views.push(folder_views::ViewChange::Sort(sort));
    }
    if let Some(grouping) = profile.folder_grouping {
        views.push(folder_views::ViewChange::Grouping(grouping));
    }
    let count = profile.tweaks.len() + profile.disabled_tweaks.len() + views.len();
    if count == 0 {
        return Ok(0);
    }
    let icon = blank_icon_path()?;
    let changes = pending_changes(imported_changes(
        HKCU,
        HKLM,
        &profile.tweaks,
        &profile.disabled_tweaks,
        &icon,
    ))?;
    save_original_settings()?;
    let prepared = prepare_changes(&changes)?;
    if profile.tweaks.contains(&Preference::HideShortcutArrow) {
        ensure_blank_icon_at(&icon, replace_blank_icon)?;
    }
    folder_views::apply_recommended(&views, || apply_changes(&changes, &prepared))?;
    restart_explorer(
        "Settings",
        selected().any(|tweak| *tweak == Preference::HideShortcutArrow),
    )?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_keeps_only_final_selected_changes() {
        let current = ExplorerSettings {
            hide_recent_files: true,
            ..Default::default()
        };
        let profile = batch_profile(
            &[
                SettingsCommand::Toggle(Preference::HideRecentFiles),
                SettingsCommand::Toggle(Preference::HideTaskView),
                SettingsCommand::Toggle(Preference::HideFrequentFolders),
                SettingsCommand::Toggle(Preference::HideTaskView),
            ],
            &current,
        )
        .unwrap();
        assert_eq!(profile.tweaks, [Preference::HideFrequentFolders]);
        assert_eq!(profile.disabled_tweaks, [Preference::HideRecentFiles]);
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
                    SettingsCommand::Toggle(Preference::HideRecentFiles),
                ],
                false,
            ),
            (
                [
                    SettingsCommand::Toggle(Preference::HideRecentFiles),
                    SettingsCommand::Recommended,
                ],
                true,
            ),
            (
                [SettingsCommand::Recommended, SettingsCommand::Recommended],
                true,
            ),
        ] {
            let profile = batch_profile(&commands, &current).unwrap();
            assert_eq!(
                profile.tweaks.contains(&Preference::HideRecentFiles),
                recent
            );
            assert!(profile.tweaks.contains(&Preference::HideShortcutArrow));
            assert!(
                !profile
                    .tweaks
                    .contains(&Preference::DisableTaskbarCentering)
            );
            assert!(!profile.tweaks.contains(&Preference::EnableTaskbarEndTask));
            assert!(!profile.tweaks.contains(&Preference::HideHomeFolder));
            assert!(!profile.tweaks.contains(&Preference::HideGallery));
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
    fn batch_rejects_unavailable_tweaks_before_applying() {
        let result = batch_profile(
            &[
                SettingsCommand::Toggle(Preference::HideRecentFiles),
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
                    hide_recent_files: recent,
                    hide_recommendations: recommendations,
                    ..Default::default()
                };
                let profile = profile_from_settings(&current);
                for (preference, enabled) in [
                    (Preference::HideRecentFiles, recent),
                    (Preference::HideRecommendations, recommendations),
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
                    format: "orbital-tweaks".into(),
                    version,
                    tweaks: if enabled {
                        vec![Preference::HideRecentFiles]
                    } else {
                        vec![]
                    },
                    disabled_tweaks: if enabled {
                        vec![]
                    } else {
                        vec![Preference::HideRecentFiles]
                    },
                    folder_sort: None,
                    folder_grouping: None,
                };
                profile.normalize().unwrap();
                profile.normalize().unwrap();
                profile.validate().unwrap();
                let selected = if enabled {
                    &profile.tweaks
                } else {
                    &profile.disabled_tweaks
                };
                assert_eq!(
                    selected.contains(&Preference::HideRecommendations),
                    version < 3
                );
                assert_eq!(profile.version, 3);
            }
        }
    }

    #[test]
    fn unavailable_features_are_omitted_from_exports() {
        let current = ExplorerSettings::default();
        let profile = profile_from_settings(&current);
        for preference in [
            Preference::HideHomeFolder,
            Preference::HideGallery,
            Preference::DisableExplorerSpacing,
            Preference::DisableTaskbarCentering,
            Preference::HideRecommendations,
        ] {
            assert!(!profile.disabled_tweaks.contains(&preference));
            assert!(!profile.tweaks.contains(&preference));
        }
    }

    #[test]
    fn export_preserves_enabled_and_disabled_supported_tweaks_and_known_folder_values() {
        let mut current = ExplorerSettings {
            is_administrator: false,
            windows11_supported: true,
            folder_sort: Some(FolderSort {
                column: SortColumn::Size,
                descending: true,
            }),
            folder_grouping: Some(FolderGrouping::None),
            disable_taskbar_centering: true,
            hide_task_view: false,
            disable_widgets: false,
            enable_taskbar_end_task: false,
            taskbar_end_task_supported: true,
            disable_explorer_spacing: false,
            disable_modern_context_menu: true,
            hide_recent_files: true,
            hide_recommendations: false,
            hide_frequent_folders: false,
            hide_office_files: false,
            hide_home_folder: false,
            home_supported: true,
            hide_gallery: false,
            gallery_supported: true,
            hide_shortcut_arrow: None,
        };
        let profile = profile_from_settings(&current);
        assert_eq!(
            profile.tweaks,
            [
                Preference::DisableTaskbarCentering,
                Preference::DisableModernContextMenu,
                Preference::HideRecentFiles
            ]
        );
        assert_eq!(profile.folder_sort, current.folder_sort);
        assert_eq!(profile.version, 3);
        assert!(
            profile
                .disabled_tweaks
                .contains(&Preference::DisableExplorerSpacing)
        );
        assert!(profile.disabled_tweaks.contains(&Preference::HideGallery));
        for preference in [
            Preference::HideTaskView,
            Preference::DisableWidgets,
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
                .contains(&Preference::DisableWidgets)
        );
        assert!(
            !profile_from_settings(&current)
                .tweaks
                .contains(&Preference::DisableModernContextMenu)
        );
    }

    #[test]
    fn imports_only_generate_writes_for_selected_tweaks() {
        let changes = imported_changes(
            HKCU,
            HKLM,
            &[
                Preference::DisableTaskbarCentering,
                Preference::HideRecentFiles,
            ],
            &[],
            Path::new("unused.ico"),
        );
        assert_eq!(
            changes
                .iter()
                .map(|change| change.name.as_str())
                .collect::<Vec<_>>(),
            ["TaskbarAl", "ShowRecent"]
        );
        assert!(imported_changes(HKCU, HKLM, &[], &[], Path::new("unused.ico")).is_empty());
    }

    #[test]
    fn importing_disabled_values_restores_enabled_registry_preferences() {
        let registry = super::super::tests::TestRegistry::new();
        let user = registry.key.create_subkey("User").unwrap().0;
        let machine = registry.key.create_subkey("Machine").unwrap().0;
        let tweaks = [
            Preference::DisableTaskbarCentering,
            Preference::HideTaskView,
            Preference::DisableWidgets,
            Preference::EnableTaskbarEndTask,
            Preference::DisableExplorerSpacing,
            Preference::DisableModernContextMenu,
            Preference::HideRecentFiles,
            Preference::HideRecommendations,
            Preference::HideFrequentFolders,
            Preference::HideOfficeFiles,
            Preference::HideHomeFolder,
            Preference::HideGallery,
            Preference::HideShortcutArrow,
        ];
        let icon = Path::new("test-import-icon.ico");
        for enabled in [true, false] {
            let changes = if enabled {
                imported_changes(&user, &machine, &tweaks, &[], icon)
            } else {
                imported_changes(&user, &machine, &[], &tweaks, icon)
            };
            let prepared = prepare_changes(&changes).unwrap();
            apply_changes(&changes, &prepared).unwrap();
            for preference in [
                &DISABLE_TASKBAR_CENTERING,
                &HIDE_TASK_VIEW,
                &ENABLE_TASKBAR_END_TASK,
                &DISABLE_EXPLORER_SPACING,
                &HIDE_RECENT_FILES,
                &HIDE_RECOMMENDATIONS,
                &HIDE_FREQUENT_FOLDERS,
                &HIDE_OFFICE_FILES,
            ] {
                assert_eq!(preference.enabled_in(&user).unwrap(), enabled);
            }
            assert_eq!(DISABLE_WIDGETS.enabled_in(&machine).unwrap(), enabled);
            for clsid in [HOME_CLSID, GALLERY_CLSID] {
                assert_eq!(
                    namespace_visible_in(&user, &machine, clsid).unwrap(),
                    !enabled
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
            format: "orbital-tweaks".into(),
            version: 1,
            tweaks: vec![],
            disabled_tweaks: vec![],
            folder_sort: None,
            folder_grouping: None,
        };
        assert!(profile.validate().is_ok());
        profile.version = 4;
        assert!(profile.validate().is_err());
        profile.version = 1;
        profile.format = "other-app".into();
        assert!(profile.validate().is_err());
        profile.format = "orbital-tweaks".into();
        profile.tweaks = vec![Preference::HideRecentFiles; 2];
        assert!(profile.validate().is_err());
        profile.tweaks.pop();
        profile.disabled_tweaks = vec![Preference::HideRecentFiles];
        assert!(profile.validate().is_err());
    }
}
