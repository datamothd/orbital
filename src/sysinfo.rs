use ::sysinfo::System;
use std::io;
use winreg::HKLM;

#[derive(serde::Serialize)]
pub struct SystemInfo {
    pub host: String,
    pub os: String,
    pub windows_version: String,
    pub cpu: String,
    pub threads: usize,
    pub used_memory_gib: f64,
    pub total_memory_gib: f64,
}

pub fn collect() -> io::Result<SystemInfo> {
    let mut system = System::new();
    let cur_ver = HKLM.open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion")?;
    let current_version: String = cur_ver.get_value("DisplayVersion")?;
    system.refresh_cpu_all();
    system.refresh_memory();

    Ok(SystemInfo {
        host: System::host_name().unwrap_or_else(|| "Unknown".into()),
        os: System::long_os_version().unwrap_or_else(|| "Unknown".into()),
        windows_version: current_version,
        cpu: system
            .cpus()
            .first()
            .map_or("Unknown", |cpu| cpu.brand())
            .into(),
        threads: system.cpus().len(),
        used_memory_gib: system.used_memory() as f64 / 1_073_741_824.0,
        total_memory_gib: system.total_memory() as f64 / 1_073_741_824.0,
    })
}

pub fn show() -> io::Result<()> {
    let info = collect()?;
    println!("Host: {}", info.host);
    println!("OS: {}", info.os);
    println!("Windows Version: {}", info.windows_version);
    println!("CPU: {}", info.cpu);
    println!("Threads: {}", info.threads);
    println!(
        "Memory: {:.1} / {:.1} GiB",
        info.used_memory_gib, info.total_memory_gib,
    );
    Ok(())
}
