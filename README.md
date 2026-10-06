# Orbital

(Will be) A Windows optimization and `.iso` configuration tool that lets users change certain aspects of Windows, specifically security, performance, and speed.

The idea comes from frustration with Windows configuration tools that require a proprietary parent app, don't let you change settings before the OS is installed, or leave out the options you actually want. All while making the process less clear than some power users would like.

Orbital should eventually be an alternative, allowing users to configure Windows before and after its initial installation. This "software" is intended for people who frequently reset their computers, people who want to get the most out of their system while remaining stable, and people who want to enable their OCD (which was actually the main motivation for this project).

Right now, we can tell basic system information and have a few tweaks for your system (currently only for devices with Windows already installed). Orbital is NOT that sophisticated yet.

## Current features

- Display basic system information: host, OS, Windows version, CPU, thread count, and memory usage.
- Toggle Windows 11 taskbar icons between left and centered.
- Toggle File Explorer compact mode on or off.
- Show the current taskbar setting and ask before changing it.
- Skip the explanation and confirmation with `-y`.

ISO configuration and the broader security and performance tweaks are planned. They aren't implemented yet.

## Basic setup

Use Windows. The current code reads the Windows registry directly, and the taskbar tweak is intended for Windows 11.

1. [Install Rust using rustup](https://rust-lang.org/tools/install/). Follow the installer prompts, including the Visual Studio C++ Build Tools setup if needed.
2. Download or clone this repository.
3. Open PowerShell in the `orbital` folder, where `Cargo.toml` lives. If you just installed Rust, open a fresh terminal so it can find Cargo.
4. Build the program:

```powershell
cargo build
```

## Running it

Show the available commands:

```powershell
cargo run -- --help
```

Display system information:

```powershell
cargo run -- sysinfo
```

Toggle taskbar alignment:

```powershell
cargo run -- taskbar-alignment
```

This displays the current `TaskbarAl` value: `0` means left, `1` means centered. Press Enter or type `y` to toggle it, or type `n` to cancel. If the value is missing, the centered default is assumed.

To skip confirmation prompts:

```powershell
cargo run -- taskbar-alignment -y
```

(`--yes` works too).

Toggle File Explorer compact mode:

```powershell
cargo run -- explorer-compact-mode
```

This displays the current `UseCompactMode` value: `0` means disabled, `1` means enabled. Press Enter or type `y` to toggle it, or type `n` to cancel. If the value is missing, the disabled default is assumed. Explorer restarts after the change. Use `-y` or `--yes` to skip confirmation.

To build an executable you can run directly:

```powershell
cargo build --release
.\target\release\orbital.exe --help
```