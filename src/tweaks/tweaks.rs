use std::{
    io::{self, Write},
    process::Command as ProcessCommand,
};
use winreg::{
    HKCU,
    enums::{KEY_QUERY_VALUE, KEY_SET_VALUE},
};

fn confirm() -> io::Result<bool> {
    loop {
        print!("Toggle alignment and restart Explorer? [Y/n] ");
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
                println!("TaskbarAl is missing; using the centered default.");
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
        if !confirm()? {
            println!("Cancelled.");
            return Ok(());
        }
    }
    key.set_value("TaskbarAl", &next)?;
    println!(
        "Taskbar alignment saved: {next} ({})",
        if next == 1 { "center" } else { "left" }
    );

    let stopped = ProcessCommand::new("taskkill")
        .args(["/F", "/IM", "explorer.exe"])
        .output()?;
    if !stopped.status.success() {
        return Err(io::Error::other(format!(
            "Alignment saved, but Explorer could not be stopped: {}",
            String::from_utf8_lossy(&stopped.stderr).trim()
        )));
    }
    ProcessCommand::new("explorer.exe").spawn()?;
    Ok(())
}
