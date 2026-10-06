use clap::{Parser, Subcommand};
use std::{
    io::{self, Write},
    process::Command as ProcessCommand,
};
use sysinfo::System;
use winreg::{
    HKCU, HKLM,
    enums::{KEY_QUERY_VALUE, KEY_SET_VALUE},
};

#[derive(Parser)]
#[command(name = "orbital", about = "Basic framework of a Windows optimization tool. Only displays system information and Windows preferences for the time being.")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Sysinfo,
    TaskbarAlignment {
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

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

fn toggle_taskbar_alignment(yes: bool) -> io::Result<()> {
    let key = HKCU.open_subkey_with_flags(
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced",
        KEY_QUERY_VALUE | KEY_SET_VALUE,
    )?;
    let current: u32 = match key.get_value("TaskbarAl") {
        Ok(value) => value,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            println!("TaskbarAl is missing; using the centered default.");
            1
        }
        Err(error) => return Err(error),
    };
    let next = match current {
        0 => 1u32,
        1 => 0u32,
        _ => return Err(io::Error::other("TaskbarAl must be 0 or 1")),
    };
    println!(
        "Current TaskbarAl: {current} ({})",
        if current == 1 { "center" } else { "left" }
    );
    if !yes && !confirm()? {
        println!("Cancelled.");
        return Ok(());
    }
    key.set_value("TaskbarAl", &next)?;
    println!(
        "Taskbar alignment saved: {}",
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

fn main() -> io::Result<()> {
    match Cli::parse().command {
        Command::TaskbarAlignment { yes } => toggle_taskbar_alignment(yes)?,
        Command::Sysinfo => {
            let mut system = System::new();
            let cur_ver = HKLM
                .open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion")
                .expect("Failed to open Windows version registry key");
            let current_version: String = cur_ver
                .get_value("DisplayVersion")
                .expect("Failed to read DisplayVersion");
            system.refresh_cpu_all();
            system.refresh_memory();

            println!(
                "Host: {}",
                System::host_name().unwrap_or_else(|| "Unknown".into())
            );
            println!(
                "OS: {}",
                System::long_os_version().unwrap_or_else(|| "Unknown".into())
            );
            println!("Windows Version: {}", current_version);
            println!(
                "CPU: {}",
                system.cpus().first().map_or("Unknown", |cpu| cpu.brand())
            );
            println!("Threads: {}", system.cpus().len());
            println!(
                "Memory: {:.1} / {:.1} GiB",
                system.used_memory() as f64 / 1_073_741_824.0,
                system.total_memory() as f64 / 1_073_741_824.0,
            );
        }
    }
    Ok(())
}
