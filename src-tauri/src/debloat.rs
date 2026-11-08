use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    io::{BufRead, BufReader, Read, Write},
    os::windows::process::CommandExt,
    process::{Child, ChildStdout, Command, Stdio},
    sync::{Arc, Mutex},
    thread::JoinHandle,
};
use tauri::{State, ipc::Channel};

const SCRIPT: &str = include_str!("debloat.ps1");

#[derive(Clone, Deserialize, Serialize)]
pub struct AppDetail {
    label: String,
    value: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct InstalledApp {
    #[serde(default)]
    key: String,
    name: String,
    id: String,
    kind: String,
    version: String,
    #[serde(default)]
    catalog_id: String,
    #[serde(default)]
    cells: Vec<String>,
    details: Vec<AppDetail>,
    selectable: bool,
    #[serde(default)]
    blocked_reason: String,
    #[serde(default)]
    requires_permanent: bool,
}

#[derive(Deserialize, Serialize)]
pub struct AppList {
    #[serde(default)]
    headers: Vec<String>,
    apps: Vec<InstalledApp>,
    #[serde(default)]
    warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ListError {
    message: String,
    output: String,
}

impl From<String> for ListError {
    fn from(message: String) -> Self {
        Self {
            message,
            output: String::new(),
        }
    }
}

#[derive(Default)]
struct Inventory {
    generation: u64,
    apps: Vec<InstalledApp>,
    valid: bool,
    permanent: bool,
}

#[derive(Clone, Default)]
pub struct RemovalState(Arc<Mutex<Inventory>>);

impl RemovalState {
    pub fn while_idle<T>(&self, operation: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
        let _inventory = self.0.try_lock().map_err(|_| "Wait for app removal or inventory refresh to finish before relaunching.")?;
        operation()
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct UninstallResult {
    #[serde(default)]
    key: String,
    #[serde(default)]
    name: String,
    success: bool,
    output: String,
    restart_required: bool,
}

#[derive(Clone, Serialize)]
pub struct UninstallEvent {
    phase: &'static str,
    name: String,
    command: String,
    success: bool,
    output: String,
}

struct NativeService {
    child: Child,
    stdout: BufReader<ChildStdout>,
    stderr: Option<JoinHandle<std::io::Result<String>>>,
}

impl NativeService {
    fn start() -> Result<Self, String> {
        Self::start_script(SCRIPT)
    }

    fn start_script(script: &str) -> Result<Self, String> {
        let executable = std::env::var_os("SystemRoot")
            .map(std::path::PathBuf::from)
            .ok_or("Windows system directory is unavailable.")?
            .join(r"System32\WindowsPowerShell\v1.0\powershell.exe");
        let mut child = Command::new(executable)
            .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", script])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags(0x0800_0000)
            .spawn()
            .map_err(|error| format!("Could not start Windows app servicing: {error}"))?;
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let mut stderr = child.stderr.take().unwrap();
        let stderr = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            stderr.read_to_end(&mut bytes)?;
            Ok(String::from_utf8_lossy(&bytes).trim().to_owned())
        });
        Ok(Self {
            child,
            stdout,
            stderr: Some(stderr),
        })
    }

    fn errors(&mut self) -> String {
        self.stderr
            .take()
            .and_then(|reader| reader.join().ok())
            .and_then(Result::ok)
            .unwrap_or_default()
    }

    fn request(&mut self, request: serde_json::Value) -> Result<String, String> {
        let mut payload = serde_json::to_vec(&request).map_err(|error| error.to_string())?;
        payload.push(b'\n');
        let input = self.child.stdin.as_mut()
            .ok_or("App servicing input is unavailable.")?;
        if let Err(error) = input.write_all(&payload).and_then(|_| input.flush()) {
            return Err(format!("Could not send the removal request: {error}"));
        }
        let mut output = String::new();
        if self.stdout.read_line(&mut output).map_err(|error| error.to_string())? == 0 {
            let _ = self.child.wait();
            return Err(format!("Windows app servicing stopped before returning a result.\n{}", self.errors()));
        }
        Ok(output.trim_start_matches('\u{feff}').trim().to_owned())
    }

    fn finish(&mut self) -> Result<(), String> {
        drop(self.child.stdin.take());
        let status = self.child.wait().map_err(|error| error.to_string())?;
        let stderr = self.errors();
        if !status.success() {
            return Err(format!("Windows app servicing failed.\n{stderr}"));
        }
        Ok(())
    }
}

impl Drop for NativeService {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = self.errors();
    }
}

fn run_native(request: serde_json::Value) -> Result<String, String> {
    let mut service = NativeService::start()?;
    let output = service.request(request)?;
    service.finish()?;
    Ok(output)
}

fn parse_list(output: &str, generation: u64) -> Result<AppList, String> {
    let mut list: AppList = serde_json::from_str(output)
        .map_err(|error| format!("Windows returned an unreadable app inventory: {error}"))?;
    let mut identities = HashSet::new();
    for (index, app) in list.apps.iter_mut().enumerate() {
        if !matches!(
            app.kind.as_str(),
            "appx" | "desktop" | "feature" | "capability"
        ) || app.id.is_empty()
            || app.id.chars().any(char::is_control)
            || !identities.insert((app.kind.clone(), app.id.to_lowercase()))
        {
            return Err("Windows returned invalid or duplicate app identities.".into());
        }
        app.key = format!("{generation}:{index}");
        app.cells = vec![app.name.clone(), app.id.clone(), app.version.clone()];
    }
    list.headers = ["Name", "Package", "Version / state"]
        .map(str::to_owned)
        .into();
    Ok(list)
}

fn selected_apps(inventory: &Inventory, keys: &[String]) -> Result<Vec<InstalledApp>, String> {
    if !inventory.valid {
        return Err("Refresh the installed apps list before uninstalling.".into());
    }
    if keys.is_empty() || keys.len() > inventory.apps.len() {
        return Err("Select at least one app from the current list.".into());
    }
    let mut seen = HashSet::new();
    keys.iter().filter(|key| seen.insert(key.as_str())).map(|key| {
        inventory.apps.iter().find(|app| app.key == *key && app.selectable)
            .cloned().ok_or_else(|| "The selection is unavailable or out of date. Refresh and select your apps again.".into())
    }).collect()
}

fn set_permanent(
    inventory: &mut Inventory,
    enabled: bool,
    acknowledged: bool,
) -> Result<(), String> {
    if enabled && !acknowledged {
        return Err("Acknowledge permanent component removal before enabling it.".into());
    }
    inventory.permanent = enabled;
    Ok(())
}

#[tauri::command]
pub fn set_permanent_removal(
    state: State<'_, RemovalState>,
    enabled: bool,
    acknowledged: bool,
) -> Result<(), String> {
    let mut inventory = state
        .0
        .try_lock()
        .map_err(|_| "Wait for the current app operation to finish.")?;
    set_permanent(&mut inventory, enabled, acknowledged)
}

#[tauri::command]
pub async fn list_installed_apps(
    state: State<'_, RemovalState>,
    on_command: Channel<String>,
) -> Result<AppList, ListError> {
    let state = Arc::clone(&state.0);
    tauri::async_runtime::spawn_blocking(move || {
        let mut inventory = state.try_lock().map_err(|_| ListError::from("Another app operation is running.".to_owned()))?;
        inventory.valid = false;
        let _ = on_command.send("Read Appx packages, Windows provisioning, desktop uninstallers and optional components".into());
        let output = run_native(serde_json::json!({"action": "list"})).map_err(|output| ListError {
            message: "Could not read installed apps. See the diagnostic output below.".into(), output,
        })?;
        inventory.generation += 1;
        let result = parse_list(&output, inventory.generation).map_err(|message| ListError { message, output })?;
        inventory.apps = result.apps.clone();
        inventory.valid = true;
        Ok(result)
    }).await.map_err(|error| ListError::from(error.to_string()))?
}

#[tauri::command]
pub async fn uninstall_apps(
    state: State<'_, RemovalState>,
    keys: Vec<String>,
    on_event: Channel<UninstallEvent>,
) -> Result<Vec<UninstallResult>, String> {
    let state = Arc::clone(&state.0);
    tauri::async_runtime::spawn_blocking(move || {
        let mut inventory = state.try_lock().map_err(|_| "Another app operation is running.")?;
        let apps = selected_apps(&inventory, &keys)?;
        inventory.valid = false;
        let mut results = Vec::new();
        let mut service = None;
        for app in apps {
            let command = format!("Remove {} [{}]{}", app.name, app.kind,
                if inventory.permanent { " · permanent component removal enabled" } else { "" });
            let _ = on_event.send(UninstallEvent { phase: "started", name: app.name.clone(), command, success: false, output: String::new() });
            let request = serde_json::json!({"action": "remove", "kind": app.kind, "id": app.id, "version": app.version, "permanent": inventory.permanent});
            let result = (|| {
                if service.is_none() { service = Some(NativeService::start()?); }
                let output = service.as_mut().unwrap().request(request)?;
                serde_json::from_str::<UninstallResult>(&output).map_err(|error| format!("Could not read the removal result: {error}\n{output}"))
            })();
            if result.is_err() { service = None; }
            let result = match result {
                Ok(mut result) => { result.key = app.key; result.name = app.name.clone(); result },
                Err(output) => UninstallResult { key: app.key, name: app.name.clone(), success: false, output, restart_required: false },
            };
            let _ = on_event.send(UninstallEvent { phase: "finished", name: app.name, command: String::new(), success: result.success, output: result.output.clone() });
            results.push(result);
        }
        if let Some(mut service) = service { let _ = service.finish(); }
        Ok(results)
    }).await.map_err(|error| error.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_worker_reuses_one_process_and_preserves_request_boundaries() {
        let script = r#"
[Console]::InputEncoding = [Text.UTF8Encoding]::new($false)
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
$sequence = 0
while ($null -ne ($line = [Console]::ReadLine())) {
    $request = $line | ConvertFrom-Json
    $sequence++
    [Console]::WriteLine((@{ sequence = $sequence; pid = $PID; text = $request.text } | ConvertTo-Json -Compress))
    [Console]::Out.Flush()
}
"#;
        let mut service = NativeService::start_script(script).unwrap();
        let mut pid = None;
        for (index, text) in ["应用 👩‍💻", "two\nlines"].into_iter().enumerate() {
            let output = service.request(serde_json::json!({"text": text})).unwrap();
            let result: serde_json::Value = serde_json::from_str(&output).unwrap();
            assert_eq!(result["text"], text);
            assert_eq!(result["sequence"], index + 1);
            assert_eq!(pid.get_or_insert_with(|| result["pid"].clone()), &result["pid"]);
        }
        service.finish().unwrap();
    }

    #[test]
    fn native_worker_reports_crashes_without_blocking_on_stderr() {
        let mut service = NativeService::start_script(
            "[void][Console]::ReadLine(); [Console]::Error.WriteLine(('x' * 100000)); exit 7"
        ).unwrap();
        let error = service.request(serde_json::json!({})).unwrap_err();
        assert!(error.contains("stopped before returning a result"));
        assert!(error.contains(&"x".repeat(100000)));
        assert!(service.finish().is_err());
    }

    #[test]
    fn native_removal_errors_do_not_end_the_session() {
        let mut service = NativeService::start().unwrap();
        for id in ["", "Example.NotInstalled"] {
            let output = service.request(serde_json::json!({"action": "remove", "kind": "invalid", "id": id})).unwrap();
            let result: UninstallResult = serde_json::from_str(&output).unwrap();
            assert!(!result.success);
            assert!(!result.restart_required);
            assert!(result.output.contains(if id.is_empty() { "Invalid app identity" } else { "Unsupported app identity type" }));
        }
        service.finish().unwrap();
    }

    #[test]
    fn relaunch_cannot_start_while_app_servicing_holds_the_inventory() {
        let state = RemovalState::default();
        let guard = state.0.lock().unwrap();
        assert!(state.while_idle::<()>(|| panic!("Relaunch must not run during servicing")).is_err());
        drop(guard);
        assert!(state.while_idle(|| Ok(())).is_ok());
    }

    fn inventory() -> Inventory {
        let list = parse_list(r#"{"apps":[{"name":"应用 👩‍💻","id":"Example.App","kind":"appx","version":"2026.10","details":[],"selectable":true},{"name":"Framework","id":"Example.Framework","kind":"appx","version":"1","details":[],"selectable":false,"blocked_reason":"Dependency"}],"warnings":[]}"#, 4).unwrap();
        Inventory {
            apps: list.apps,
            valid: true,
            ..Default::default()
        }
    }

    #[test]
    fn selection_rejects_stale_missing_and_protected_targets() {
        let mut state = inventory();
        assert_eq!(
            selected_apps(&state, &["4:0".into()]).unwrap()[0].name,
            "应用 👩‍💻"
        );
        for keys in [
            vec![],
            vec!["3:0".into()],
            vec!["4:1".into()],
            vec!["unknown".into()],
        ] {
            assert!(selected_apps(&state, &keys).is_err());
        }
        state.valid = false;
        assert!(selected_apps(&state, &["4:0".into()]).is_err());
    }

    #[test]
    fn permanent_mode_requires_acknowledgment_and_defaults_off() {
        let mut state = inventory();
        assert!(!state.permanent);
        assert!(set_permanent(&mut state, true, false).is_err());
        assert!(!state.permanent);
        set_permanent(&mut state, true, true).unwrap();
        assert!(state.permanent);
        set_permanent(&mut state, false, false).unwrap();
        assert!(!state.permanent);
    }

    #[test]
    fn invalid_and_duplicate_native_identities_are_rejected() {
        let state = inventory();
        let app = serde_json::to_value(&state.apps[0]).unwrap();
        assert!(
            parse_list(
                &serde_json::json!({"apps":[app.clone(), app]}).to_string(),
                1
            )
            .is_err()
        );
        let mut app = serde_json::to_value(&state.apps[0]).unwrap();
        app["kind"] = "command".into();
        assert!(parse_list(&serde_json::json!({"apps":[app]}).to_string(), 1).is_err());
        assert!(
            parse_list(r#"{"apps":[],"warnings":[]}"#, 1)
                .unwrap()
                .apps
                .is_empty()
        );
    }
}
