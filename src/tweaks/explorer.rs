use std::os::windows::process::CommandExt;
use std::{
    io::{self, Write},
    process::Command as ProcessCommand,
};
use winreg::{
    HKCU, HKLM,
    enums::{KEY_QUERY_VALUE, KEY_SET_VALUE},
};

const CLASSIC_CONTEXT_MENU_KEY: &str =
    r"Software\Classes\CLSID\{86ca1aa0-34aa-4e8b-a509-50c905bae2a2}\InprocServer32";

fn supports_classic_context_menu() -> io::Result<bool> {
    let key = HKLM.open_subkey_with_flags(
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
        KEY_QUERY_VALUE,
    )?;
    let build: String = key.get_value("CurrentBuildNumber")?;
    let build: u32 = build
        .parse()
        .map_err(|_| io::Error::other("Invalid Windows build number"))?;
    Ok(build >= 22000)
}

fn classic_context_menu_enabled() -> io::Result<bool> {
    let key = match HKCU.open_subkey_with_flags(CLASSIC_CONTEXT_MENU_KEY, KEY_QUERY_VALUE) {
        Ok(key) => key,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    match key.get_value::<String, _>("") {
        Ok(value) => Ok(value.is_empty()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

#[derive(serde::Serialize)]
pub struct ExplorerSettings {
    pub taskbar_centered: bool,
    pub compact_mode: bool,
    pub classic_context_menu: bool,
    pub classic_context_menu_supported: bool,
}

pub fn settings() -> io::Result<ExplorerSettings> {
    let key = HKCU.open_subkey_with_flags(
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced",
        KEY_QUERY_VALUE,
    )?;
    let read = |name: &str, default: u32| -> io::Result<bool> {
        let value: u32 = match key.get_value(name) {
            Ok(value) => value,
            Err(error) if error.kind() == io::ErrorKind::NotFound => default,
            Err(error) => return Err(error),
        };
        match value {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(io::Error::other(format!("{name} must be 0 or 1"))),
        }
    };
    let classic_context_menu_supported = supports_classic_context_menu()?;
    Ok(ExplorerSettings {
        taskbar_centered: read("TaskbarAl", 1)?,
        compact_mode: read("UseCompactMode", 0)?,
        classic_context_menu: classic_context_menu_supported && classic_context_menu_enabled()?,
        classic_context_menu_supported,
    })
}

fn confirm(prompt: &str) -> io::Result<bool> {
    loop {
        print!("{prompt} [Y/n] ");
        io::stdout().flush()?;
        let mut answer = String::new();
        if io::stdin().read_line(&mut answer)? == 0 {
            return Ok(false);
        }
        match answer.trim().to_ascii_lowercase().as_str() {
            "" | "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => println!("Please enter y or n."),
        }
    }
}

pub fn toggle_taskbar_alignment(yes: bool) -> io::Result<()> {
    let key = HKCU.open_subkey_with_flags(
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced",
        KEY_QUERY_VALUE | KEY_SET_VALUE,
    )?;
    let current: u32 = match key.get_value("TaskbarAl") {
        Ok(value) => value,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if !yes {
                println!("TaskbarAl is missing; using the default.");
            }
            1
        }
        Err(error) => return Err(error),
    };
    let next = match current {
        0 => 1u32,
        1 => 0u32,
        _ => return Err(io::Error::other("TaskbarAl must be 0 or 1")),
    };
    if !yes {
        println!(
            "Current TaskbarAl: {current} ({})",
            if current == 1 { "center" } else { "left" }
        );
        if !confirm("Toggle alignment and restart Explorer?")? {
            println!("Cancelled.");
            return Ok(());
        }
    }
    key.set_value("TaskbarAl", &next)?;
    println!(
        "Taskbar alignment saved: {current} ({}) >> {next} ({})",
        if current == 1 { "center" } else { "left" },
        if next == 1 { "center" } else { "left" }
    );

    restart_explorer("Alignment")
}

pub fn toggle_explorer_compact_mode(yes: bool) -> io::Result<()> {
    let key = HKCU.open_subkey_with_flags(
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced",
        KEY_QUERY_VALUE | KEY_SET_VALUE,
    )?;
    let current: u32 = match key.get_value("UseCompactMode") {
        Ok(value) => value,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if !yes {
                println!("UseCompactMode is missing; using the default.");
            }
            0
        }
        Err(error) => return Err(error),
    };
    let next = match current {
        0 => 1u32,
        1 => 0u32,
        _ => return Err(io::Error::other("UseCompactMode must be 0 or 1")),
    };
    if !yes {
        println!(
            "Current UseCompactMode: {current} ({})",
            if current == 1 { "enabled" } else { "disabled" }
        );
        if !confirm("Toggle compact mode and restart Explorer?")? {
            println!("Cancelled.");
            return Ok(());
        }
    }
    key.set_value("UseCompactMode", &next)?;
    println!(
        "Explorer compact mode saved: {current} ({}) >> {next} ({})",
        if current == 1 { "enabled" } else { "disabled" },
        if next == 1 { "enabled" } else { "disabled" }
    );
    restart_explorer("Compact mode")
}

pub fn toggle_classic_context_menu(yes: bool) -> io::Result<()> {
    if !supports_classic_context_menu()? {
        return Err(io::Error::other("Classic context menu requires Windows 11"));
    }
    let current = classic_context_menu_enabled()?;
    if !yes {
        println!(
            "Classic context menu: {}",
            if current { "enabled" } else { "disabled" }
        );
        if !confirm("Toggle classic context menu and restart Explorer?")? {
            println!("Cancelled.");
            return Ok(());
        }
    }
    if current {
        let key = HKCU.open_subkey_with_flags(CLASSIC_CONTEXT_MENU_KEY, KEY_SET_VALUE)?;
        match key.delete_value("") {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    } else {
        let (key, _) = HKCU.create_subkey(CLASSIC_CONTEXT_MENU_KEY)?;
        key.set_value("", &"")?;
    }
    println!(
        "Classic context menu saved: {}",
        if current { "disabled" } else { "enabled" }
    );
    restart_explorer("Classic context menu")
}

fn restart_explorer(setting: &str) -> io::Result<()> {
    let stopped = ProcessCommand::new("taskkill")
        .creation_flags(0x08000000)
        .args(["/F", "/IM", "explorer.exe"])
        .output()?;
    if !stopped.status.success() {
        return Err(io::Error::other(format!(
            "{setting} saved, but Explorer could not be stopped: {}",
            String::from_utf8_lossy(&stopped.stderr).trim()
        )));
    }
    ProcessCommand::new("explorer.exe")
        .creation_flags(0x08000000)
        .spawn()?;
    Ok(())
}
