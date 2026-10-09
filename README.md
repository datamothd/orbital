<img width="256" height="256" src="https://github.com/datamothd/orbital/blob/main/src-tauri/icons/icon.png" /> 

# Orbital

A Windows preference and configurator tool that lets users change certain aspects of Windows, specifically security, performance, and speed.

The idea comes from frustration with Windows configuration tools that require a proprietary parent app, don't let you change settings before the OS is installed, or leave out the options you actually want. All while making the process less clear than some power users would like.

In the future, I would like to implement an actual ISO injector for autounattend.xml files and Windows PE customization to streamline the entire Windows setup process, but these features are not currently available in Orbital.

Intended for use on Windows 11 Pro on 25H2, possible 26H2 support in the future.

## Current features

- Display basic system information: host, OS, Windows version, CPU, thread count, and memory usage.
- Toggles for 9 different Windows preferences (only for Explorer related settings for now).
- Tauri app for value editing.

## Planned features
- ISO configuration
- Autounattend.xml configuration
- More preferences, especially tailoring to security and performance
- 26H2 support (as soon as I even get the update offered, thanks Windows).

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

### Desktop app

Simple Tauri 2 app the calls to the Rust backend for the Windows customization and the entire program basically.

With Rust, the Visual Studio C++ Build Tools, Node.js, and the Microsoft Edge WebView2 runtime installed, run:

```powershell
npm install
npm run dev
```

Build the small standalone desktop executable (no installer):

```powershell
npm run build
.\src-tauri\target\release\orbital-desktop.exe
```

### Command line

Show the available commands:

```powershell
cargo run -- --help
```

To build an executable:

```powershell
cargo build --release
.\target\release\orbital.exe --help
```
