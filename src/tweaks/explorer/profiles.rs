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

impl TweakProfile {
    pub fn validate(&self) -> io::Result<()> {
        if self.format != "orbital-tweaks" || !matches!(self.version, 1 | 2) {
            return Err(io::Error::other(
                "Choose an Orbital tweaks JSON file (version 1 or 2).",
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
}

impl Preference {
    fn state(self, current: &ExplorerSettings) -> Option<bool> {
        Some(match self {
            Self::DisableTaskbarCentering => current.disable_taskbar_centering,
            Self::DisableExplorerSpacing => current.disable_explorer_spacing,
            Self::DisableModernContextMenu => current.disable_modern_context_menu,
            Self::HideRecentFiles => current.hide_recent_files,
            Self::HideFrequentFolders => current.hide_frequent_folders,
            Self::HideOfficeFiles => current.hide_office_files,
            Self::HideHomeFolder => current.hide_home_folder,
            Self::HideGallery => current.hide_gallery,
            Self::HideShortcutArrow => return current.hide_shortcut_arrow,
        })
    }

    fn requires_windows11(self) -> bool {
        matches!(
            self,
            Self::DisableModernContextMenu
                | Self::HideOfficeFiles
                | Self::HideHomeFolder
                | Self::HideGallery
        )
    }

    fn requires_administrator(self) -> bool {
        matches!(
            self,
            Self::HideHomeFolder | Self::HideGallery | Self::HideShortcutArrow
        )
    }
}

pub fn export_tweaks() -> io::Result<TweakProfile> {
    let current = settings()?;
    Ok(profile_from_settings(&current))
}

fn profile_from_settings(current: &ExplorerSettings) -> TweakProfile {
    let (tweaks, disabled_tweaks) = [
        Preference::DisableTaskbarCentering,
        Preference::DisableExplorerSpacing,
        Preference::DisableModernContextMenu,
        Preference::HideRecentFiles,
        Preference::HideFrequentFolders,
        Preference::HideOfficeFiles,
        Preference::HideHomeFolder,
        Preference::HideGallery,
        Preference::HideShortcutArrow,
    ]
    .into_iter()
    .filter(|preference| {
        (!preference.requires_windows11() || current.windows11_supported)
            && preference.state(current).is_some()
    })
    .partition(|preference| preference.state(current) == Some(true));
    TweakProfile {
        format: "orbital-tweaks".into(),
        version: 2,
        tweaks,
        disabled_tweaks,
        folder_sort: current.folder_sort,
        folder_grouping: current.folder_grouping,
    }
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
        changes.extend(match tweak {
            Preference::DisableTaskbarCentering => {
                DISABLE_TASKBAR_CENTERING.changes_in(user, enabled)
            }
            Preference::DisableExplorerSpacing => {
                DISABLE_EXPLORER_SPACING.changes_in(user, enabled)
            }
            Preference::HideRecentFiles => HIDE_RECENT_FILES.changes_in(user, enabled),
            Preference::HideFrequentFolders => HIDE_FREQUENT_FOLDERS.changes_in(user, enabled),
            Preference::HideOfficeFiles => HIDE_OFFICE_FILES.changes_in(user, enabled),
            Preference::DisableModernContextMenu => vec![RegistryChange::new(
                user,
                CLASSIC_CONTEXT_MENU_KEY,
                "",
                enabled.then(|| "".to_reg_value()),
            )],
            Preference::HideHomeFolder => {
                namespace_changes(user, machine, HOME_CLSID, !enabled).into()
            }
            Preference::HideGallery => {
                namespace_changes(user, machine, GALLERY_CLSID, !enabled).into()
            }
            Preference::HideShortcutArrow => vec![RegistryChange::new(
                machine,
                SHELL_ICONS_KEY,
                "29",
                enabled.then(|| icon_value.to_reg_value()),
            )],
        });
    }
    changes
}

pub fn apply_imported_tweaks(mut profile: TweakProfile) -> io::Result<usize> {
    profile.validate()?;
    let current = settings()?;
    profile
        .tweaks
        .retain(|tweak| tweak.state(&current) != Some(true));
    profile
        .disabled_tweaks
        .retain(|tweak| tweak.state(&current) != Some(false));
    let selected = || profile.tweaks.iter().chain(&profile.disabled_tweaks);
    if selected().any(|tweak| tweak.requires_windows11() && !current.windows11_supported) {
        return Err(io::Error::other("Some imported tweaks require Windows 11."));
    }
    if selected().any(|tweak| tweak.requires_administrator() && !current.is_administrator) {
        return Err(io::Error::other(
            "This preference requires administrator access. Close Orbital and run it as administrator.",
        ));
    }
    let mut views = Vec::new();
    if let Some(sort) = profile
        .folder_sort
        .filter(|sort| current.folder_sort != Some(*sort))
    {
        views.push(folder_views::ViewChange::Sort(sort));
    }
    if let Some(grouping) = profile
        .folder_grouping
        .filter(|grouping| current.folder_grouping != Some(*grouping))
    {
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
        ensure_blank_icon()?;
    }
    folder_views::apply_recommended(&views, || apply_changes(&changes, &prepared))?;
    restart_explorer("Imported tweaks")?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

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
            disable_explorer_spacing: false,
            disable_modern_context_menu: true,
            hide_recent_files: true,
            hide_frequent_folders: false,
            hide_office_files: false,
            hide_home_folder: false,
            hide_gallery: false,
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
        assert_eq!(profile.version, 2);
        assert!(
            profile
                .disabled_tweaks
                .contains(&Preference::DisableExplorerSpacing)
        );
        assert!(profile.disabled_tweaks.contains(&Preference::HideGallery));
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
            ["TaskbarAl", "ShowRecent", "ShowRecommendations"]
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
            Preference::DisableExplorerSpacing,
            Preference::DisableModernContextMenu,
            Preference::HideRecentFiles,
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
                &DISABLE_EXPLORER_SPACING,
                &HIDE_RECENT_FILES,
                &HIDE_FREQUENT_FOLDERS,
                &HIDE_OFFICE_FILES,
            ] {
                assert_eq!(preference.enabled_in(&user).unwrap(), enabled);
            }
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
        profile.version = 3;
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
