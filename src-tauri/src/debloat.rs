use serde::Serialize;
use std::{
    collections::HashSet,
    os::windows::process::CommandExt,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
};
use tauri::{State, ipc::Channel};

#[derive(Clone, Serialize)]
pub struct AppDetail {
    label: String,
    value: String,
}

#[derive(Clone, Serialize)]
pub struct InstalledApp {
    key: String,
    name: String,
    id: String,
    version: String,
    cells: Vec<String>,
    details: Vec<AppDetail>,
    selectable: bool,
    #[serde(skip_serializing)]
    target_id: String,
}

#[derive(Serialize)]
pub struct AppList {
    headers: Vec<String>,
    apps: Vec<InstalledApp>,
}

#[derive(Debug, Serialize)]
pub struct ListError {
    message: String,
    output: String,
}

impl From<String> for ListError {
    fn from(message: String) -> Self {
        Self { message, output: String::new() }
    }
}

impl From<&str> for ListError {
    fn from(message: &str) -> Self {
        message.to_owned().into()
    }
}

const LIST_ARGUMENTS: &[&str] = &[
    "list",
    "--details",
    "--accept-source-agreements",
    "--disable-interactivity",
];

fn list_error(output: String) -> ListError {
    let message = if output.contains("--details") && output.contains("0x8A150002") {
        "This WinGet installation does not support 'list --details'. Update App Installer, then refresh."
    } else {
        "WinGet could not read installed apps. See the diagnostic output below."
    };
    ListError { message: message.into(), output }
}

#[derive(Default)]
struct Inventory {
    generation: u64,
    apps: Vec<InstalledApp>,
    valid: bool,
}

#[derive(Default)]
pub struct WingetState(Arc<Mutex<Inventory>>);

#[derive(Clone, Serialize)]
pub struct UninstallResult {
    key: String,
    name: String,
    success: bool,
    output: String,
}

#[derive(Clone, Serialize)]
pub struct UninstallEvent {
    phase: &'static str,
    name: String,
    command: String,
    success: bool,
    output: String,
}

fn clean_output(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.trim_start_matches('\u{feff}').chars();
    while let Some(character) = chars.next() {
        match character {
            '\u{1b}' => match chars.next() {
                Some('[') => {
                    for next in chars.by_ref() {
                        if ('@'..='~').contains(&next) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    let mut escape = false;
                    for next in chars.by_ref() {
                        if next == '\u{7}' || (escape && next == '\\') {
                            break;
                        }
                        escape = next == '\u{1b}';
                    }
                }
                _ => {}
            },
            '\u{8}' => {
                result.pop();
            }
            '\n' | '\r' | '\t' => result.push(character),
            character if !character.is_control() => result.push(character),
            _ => {}
        }
    }
    result.replace("\r\n", "\n").replace('\r', "\n")
}

#[derive(Default)]
struct DetailRecord {
    name: String,
    id: String,
    sequence: Option<(usize, usize)>,
    fields: Vec<AppDetail>,
}

fn record_header(line: &str) -> Option<DetailRecord> {
    if line.starts_with(char::is_whitespace) {
        return None;
    }
    let (title, id) = line.strip_suffix(']')?.rsplit_once(" [")?;
    let (name, sequence) = if let Some(numbered) = title.strip_prefix('(') {
        if let Some((numbers, name)) = numbered.split_once(") ") {
            if let Some((index, total)) = numbers.split_once('/') {
                (name, Some((index.parse().ok()?, total.parse().ok()?)))
            } else {
                (title, None)
            }
        } else {
            (title, None)
        }
    } else {
        (title, None)
    };
    if name.trim().is_empty() || id.is_empty() || id.starts_with('-') {
        return None;
    }
    Some(DetailRecord {
        name: name.trim().into(),
        id: id.into(),
        sequence,
        ..Default::default()
    })
}

fn native_identifier(value: &str) -> bool {
    value.starts_with(r"ARP\") || value.starts_with(r"MSIX\")
}

fn parse_list(output: &str, generation: u64) -> Result<AppList, String> {
    let mut records: Vec<DetailRecord> = Vec::new();
    for line in output.lines().filter(|line| !line.trim().is_empty()) {
        if let Some(record) = record_header(line)
            && (record.sequence.is_some() || records.is_empty())
        {
            records.push(record);
            continue;
        }
        let Some(record) = records.last_mut() else {
            continue;
        };
        if line.starts_with(char::is_whitespace) {
            if let Some(field) = record.fields.last_mut() {
                if !field.value.is_empty() {
                    field.value.push('\n');
                }
                field.value.push_str(line.trim());
            }
        } else if let Some((label, value)) = line.split_once([':', '：']) {
            record.fields.push(AppDetail {
                label: label.trim().into(),
                value: value.trim().into(),
            });
        }
    }
    if records.is_empty() {
        return Err("WinGet did not return full app details. See the diagnostic output below.".into());
    }
    let count = records.len();
    let mut apps = Vec::with_capacity(count);
    for (index, record) in records.into_iter().enumerate() {
        if record
            .sequence
            .is_some_and(|sequence| sequence != (index + 1, count))
            || (count > 1 && record.sequence.is_none())
        {
            return Err(
                "WinGet returned an incomplete app inventory. Refresh to try again.".into(),
            );
        }
        let version = record
            .fields
            .first()
            .filter(|field| !field.value.is_empty())
            .ok_or("WinGet returned app details without an installed version.")?
            .value
            .clone();
        let target_id = record
            .fields
            .iter()
            .skip(1)
            .find(|field| {
                field.label.eq_ignore_ascii_case("Local Identifier")
                    && native_identifier(&field.value)
            })
            .map(|field| field.value.clone())
            .unwrap_or_else(|| record.id.clone());
        apps.push(InstalledApp {
            key: format!("{generation}:{index}"),
            cells: vec![record.name.clone(), record.id.clone(), version.clone()],
            name: record.name,
            id: record.id,
            version,
            details: record.fields.into_iter().skip(1).collect(),
            selectable: true,
            target_id,
        });
    }
    Ok(AppList {
        headers: ["Name", "Id", "Version"].map(str::to_owned).into(),
        apps,
    })
}

fn uninstall_arguments(app: &InstalledApp) -> Vec<&str> {
    let mut arguments = vec!["uninstall", "--id", &app.target_id, "--exact"];
    if !native_identifier(&app.target_id) {
        arguments.extend(["--version", &app.version]);
    }
    arguments.extend(["--accept-source-agreements", "--disable-interactivity"]);
    arguments
}

fn run_winget(arguments: &[&str]) -> Result<(bool, String), String> {
    let executable = std::env::var_os("LOCALAPPDATA")
        .map(|directory| {
            std::path::PathBuf::from(directory).join(r"Microsoft\WindowsApps\winget.exe")
        })
        .filter(|path| path.is_file())
        .unwrap_or_else(|| "winget.exe".into());
    let output = Command::new(executable)
        .args(arguments)
        .stdin(Stdio::null())
        .creation_flags(0x0800_0000)
        .output()
        .map_err(|error| {
            format!(
                "Could not start WinGet. Install or update App Installer, then try again.\n{error}"
            )
        })?;
    let stdout = clean_output(&String::from_utf8_lossy(&output.stdout));
    let stderr = clean_output(&String::from_utf8_lossy(&output.stderr));
    let mut text = [stdout.trim(), stderr.trim()]
        .into_iter()
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    if !output.status.success() {
        if let Some(code) = output.status.code() {
            text.push_str(&format!("\nWinGet exited with code 0x{:08X}.", code as u32));
        } else {
            text.push_str("\nWinGet stopped before completing the command.");
        }
    }
    Ok((output.status.success(), text))
}

fn selected_apps(inventory: &Inventory, keys: &[String]) -> Result<Vec<InstalledApp>, String> {
    if !inventory.valid {
        return Err("Refresh the installed apps list before uninstalling.".into());
    }
    if keys.is_empty() || keys.len() > inventory.apps.len() {
        return Err("Select at least one app from the current list.".into());
    }
    let mut seen = HashSet::new();
    keys.iter()
        .filter(|key| seen.insert(key.as_str()))
        .map(|key| {
            inventory
                .apps
                .iter()
                .find(|app| app.key == *key && app.selectable)
                .cloned()
                .ok_or_else(|| {
                    "The selection is out of date. Refresh and select your apps again.".into()
                })
        })
        .collect()
}

#[tauri::command]
pub async fn list_installed_apps(state: State<'_, WingetState>, on_command: Channel<String>) -> Result<AppList, ListError> {
    let state = Arc::clone(&state.0);
    tauri::async_runtime::spawn_blocking(move || {
        let mut inventory = state
            .try_lock()
            .map_err(|_| "Another WinGet operation is running. Try again when it finishes.")?;
        inventory.valid = false;
        let _ = on_command.send(format!("winget {}", LIST_ARGUMENTS.join(" ")));
        let (success, output) = run_winget(LIST_ARGUMENTS)?;
        if !success {
            return Err(list_error(output));
        }
        inventory.generation += 1;
        let result = parse_list(&output, inventory.generation).map_err(|message| ListError { message, output })?;
        inventory.apps = result.apps.clone();
        inventory.valid = true;
        Ok(result)
    })
    .await
    .map_err(|error| ListError::from(error.to_string()))?
}

#[tauri::command]
pub async fn uninstall_apps(
    state: State<'_, WingetState>,
    keys: Vec<String>,
    on_event: Channel<UninstallEvent>,
) -> Result<Vec<UninstallResult>, String> {
    let state = Arc::clone(&state.0);
    tauri::async_runtime::spawn_blocking(move || {
        let mut inventory = state
            .try_lock()
            .map_err(|_| "Another WinGet operation is running. Try again when it finishes.")?;
        let apps = selected_apps(&inventory, &keys)?;
        inventory.valid = false;
        let mut results = Vec::new();
        for app in apps {
            let arguments = uninstall_arguments(&app);
            let command = format!(
                "winget {}",
                arguments
                    .iter()
                    .map(|argument| {
                        if argument.starts_with("--") || *argument == "uninstall" {
                            (*argument).to_owned()
                        } else {
                            format!("{argument:?}")
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            let _ = on_event.send(UninstallEvent {
                phase: "started",
                name: app.name.clone(),
                command,
                success: false,
                output: String::new(),
            });
            let (success, output) = run_winget(&arguments).unwrap_or_else(|error| (false, error));
            let _ = on_event.send(UninstallEvent {
                phase: "finished",
                name: app.name.clone(),
                command: String::new(),
                success,
                output: output.clone(),
            });
            results.push(UninstallResult {
                key: app.key,
                name: app.name,
                success,
                output,
            });
        }
        Ok(results)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_details_has_a_short_actionable_error_and_keeps_diagnostics() {
        let output = format!("Argument not recognized: '--details'\n{}\nWinGet exited with code 0x8A150002.", "WinGet help text\n".repeat(200));
        let error = list_error(output.clone());
        assert!(error.message.contains("does not support 'list --details'"));
        assert!(error.message.contains("Update App Installer"));
        assert!(!error.message.contains('\n'));
        assert_eq!(error.output, output);
    }

    #[test]
    fn other_command_failures_do_not_get_misreported_as_unsupported_details() {
        let output = "Failed to open source. WinGet exited with code 0x8A15000F.";
        let error = list_error(output.into());
        assert!(!error.message.contains("does not support"));
        assert_eq!(error.output, output);
    }

    #[test]
    fn full_details_preserve_long_ids_versions_unicode_and_metadata() {
        let id = format!("MSIX\\Example.{}__publisher", "LongIdentifier".repeat(30));
        let version = "2026.123456789012345678901234567890";
        let text = format!(
            "(1/2) 应用 👩‍💻 [Beta] [{id}]\n版本: {version}\n发布者: Example: Inc.\n安装路径: C:\\Program Files\\Example\n(2/2) Editor [Example.Editor]\nVersion: 1.0\nAvailable Upgrades:\n  winget [2.0]\n  private [3.0]\n"
        );
        let list = parse_list(&text, 8).unwrap();
        assert_eq!(list.apps[0].id, id);
        assert_eq!(list.apps[0].version, version);
        assert_eq!(list.apps[0].name, "应用 👩‍💻 [Beta]");
        assert_eq!(list.apps[0].details[0].value, "Example: Inc.");
        assert_eq!(list.apps[0].details[1].value, r"C:\Program Files\Example");
        assert_eq!(list.apps[1].details[0].value, "winget [2.0]\nprivate [3.0]");
        assert!(list.apps.iter().all(|app| app.selectable));
    }

    #[test]
    fn native_identity_distinguishes_same_display_versions_and_architectures() {
        let text = "(1/2) Runtime [Microsoft.Runtime]\nVersion: > 1.8.10\nLocal Identifier: MSIX\\Runtime_8000_x64__publisher\nInstalled Architecture: X64\n(2/2) Runtime [Microsoft.Runtime]\nVersion: > 1.8.10\nLocal Identifier: MSIX\\Runtime_8000_x86__publisher\nInstalled Architecture: X86\n";
        let list = parse_list(text, 1).unwrap();
        assert_ne!(list.apps[0].target_id, list.apps[1].target_id);
        for app in &list.apps {
            let args = uninstall_arguments(app);
            assert_eq!(args[2], app.target_id);
            assert!(args.contains(&"--exact"));
            assert!(!args.contains(&"--version"));
        }
    }

    #[test]
    fn metadata_values_cannot_replace_the_uninstall_identity() {
        let list = parse_list(
            "Editor [Example.Editor]\nVersion: 1.0\nPublisher: MSIX\\Other.App\n",
            1,
        )
        .unwrap();
        assert_eq!(list.apps[0].target_id, "Example.Editor");
        let localized = parse_list(
            "应用 [Example.App]\n版本： 2.0\n发布者： Example: Inc.\n",
            1,
        )
        .unwrap();
        assert_eq!(localized.apps[0].version, "2.0");
        assert_eq!(localized.apps[0].details[0].value, "Example: Inc.");
    }

    #[test]
    fn single_results_and_arp_identifiers_do_not_depend_on_column_widths() {
        let list = parse_list(
            "Rustup: toolchain [ARP\\User\\X64\\Rust App 42]\nVersion: Unknown\nPublisher: Rust\n",
            1,
        )
        .unwrap();
        assert_eq!(list.apps[0].name, "Rustup: toolchain");
        assert_eq!(list.apps[0].target_id, r"ARP\User\X64\Rust App 42");
        assert!(!uninstall_arguments(&list.apps[0]).contains(&"--version"));
        let named = parse_list("Editor [Example.Editor]\nVersion: 1.2\n", 1).unwrap();
        let args = uninstall_arguments(&named.apps[0]);
        assert_eq!(
            args[args.iter().position(|arg| *arg == "--version").unwrap() + 1],
            "1.2"
        );
    }

    #[test]
    fn terminal_controls_and_source_warnings_do_not_corrupt_records() {
        let text = clean_output(
            "\u{feff}\u{1b}]0;winget\u{7}Failed source: msstore\r\n\u{1b}[32mEditor [Example.Editor]\u{1b}[0m\r\nVersion: 1.0\r\nPublisher: Example [Company]\r\n",
        );
        let list = parse_list(&text, 1).unwrap();
        assert_eq!(list.apps.len(), 1);
        assert_eq!(list.apps[0].details[0].value, "Example [Company]");
    }

    #[test]
    fn rejects_incomplete_invalid_and_legacy_table_output() {
        for text in [
            "Name Id Version\n----------------\nApp Example.App 1.0\n",
            "(1/2) App [Example.App]\nVersion: 1.0\n",
            "App [Example.App]\n",
            "(2/2) App [Example.App]\nVersion: 1.0\n",
            "(1/2) App [Example.App]\nVersion: 1.0\n(2/2) App [Example.Other]\n",
            "App [--all]\nVersion: 1.0\n",
        ] {
            assert!(parse_list(text, 1).is_err(), "{text}");
        }
    }

    #[test]
    fn validates_every_key_before_any_uninstall_and_rejects_old_inventories() {
        let inventory = Inventory { generation: 2, apps: parse_list("(1/2) App [Example.App]\nVersion: 1.0\n(2/2) Other [Example.Other]\nVersion: 1.0\n", 2).unwrap().apps, valid: true };
        assert!(selected_apps(&inventory, &[]).is_err());
        assert!(selected_apps(&inventory, &["2:0".into(), "arbitrary-command".into()]).is_err());
        assert!(selected_apps(&inventory, &["1:0".into()]).is_err());
        assert_eq!(
            selected_apps(&inventory, &["2:0".into(), "2:0".into()])
                .unwrap()
                .len(),
            1
        );
        assert!(
            selected_apps(
                &Inventory {
                    valid: false,
                    ..inventory
                },
                &["2:0".into()]
            )
            .is_err()
        );
    }

    #[test]
    #[ignore = "Reads the live WinGet inventory; run explicitly with App Installer installed"]
    fn reads_live_winget_inventory() {
        let (success, output) = run_winget(&[
            "list",
            "--details",
            "--accept-source-agreements",
            "--disable-interactivity",
        ])
        .unwrap();
        assert!(success, "{output}");
        let list = parse_list(&output, 1).unwrap();
        assert!(!list.apps.is_empty(), "{output}");
        assert!(
            list.apps
                .iter()
                .all(|app| app.selectable && !app.id.is_empty() && !app.version.is_empty())
        );
        println!("Read {} full installed app records.", list.apps.len());
    }
}
