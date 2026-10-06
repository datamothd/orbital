use std::{
    io::{self, Write},
    process::Command as ProcessCommand,
};
use winreg::{
    HKCU,
    enums::{KEY_QUERY_VALUE, KEY_SET_VALUE},
};

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

pub(crate) fn toggle_taskbar_alignment(yes: bool) -> io::Result<()> {
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
        "Taskbar alignment saved: {next} ({})",
        if next == 1 { "center" } else { "left" }
    );

    restart_explorer("Alignment")
}

pub(crate) fn toggle_explorer_compact_mode(yes: bool) -> io::Result<()> {
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
        "Explorer compact mode saved: {next} ({})",
        if next == 1 { "enabled" } else { "disabled" }
    );
    restart_explorer("Compact mode")
}

fn restart_explorer(setting: &str) -> io::Result<()> {
    let stopped = ProcessCommand::new("taskkill")
        .args(["/F", "/IM", "explorer.exe"])
        .output()?;
    if !stopped.status.success() {
        return Err(io::Error::other(format!(
            "{setting} saved, but Explorer could not be stopped: {}",
            String::from_utf8_lossy(&stopped.stderr).trim()
        )));
    }
    ProcessCommand::new("explorer.exe").spawn()?;
    Ok(())
}
