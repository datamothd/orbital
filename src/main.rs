use clap::{Parser, Subcommand};
use sysinfo::System;
use winreg::{HKLM};

#[derive(Parser)]
#[command(name = "orbital", about = "Simple system information")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Sysinfo,
}

fn main() {
    match Cli::parse().command {
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
            println!(
                "Windows Version: {}", current_version
            );
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
}
