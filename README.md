<img width="256" height="256" src="exports/bohr-8-of-8.gif" alt="Animated Bohr diagram with all eight changes enabled" />

# Orbital

A Windows preference and configurator tool that lets users change certain aspects of Windows, specifically features related to security, performance, and speed.

The idea comes from frustration with existing Windows configuration tools that require a proprietary parent app, don't let you change settings before the OS is installed, or leave out the options you actually want. All while making the process less clear than some power users would like.

The Inject page adds an autounattend.xml answer file to a Windows installation ISO. Windows PE customization and bundling the Orbital CLI for post-install configuration are planned for a future update.

Supports Windows 11 Pro, versions 25H2 and 26H2.

## Current features

- Display basic system information: host, OS, Windows version, CPU, thread count, and memory usage.
- Toggles for Explorer, taskbar and desktop preferences, including Widgets, Task View and taskbar End task.
- Tauri app interface for value editing.
- Setting preference state in mass and change reverting.
- Import and export enabled and disabled tweak values as JSON, with a selectable comparison preview before importing.
- Inject an answer file into a Windows ISO, either replacing the selected image or saving an injected copy. Includes progress, cancellation, and verification of setup files and BIOS/UEFI boot images.
- An in-app autounattend.xml editor

## Planned features
- Orbital CLI integration in installation images
- More preferences, especially tailoring to security and performance

## Basic setup

Use Windows 11 Pro, version 25H2 or 26H2. The current code reads the Windows registry directly.

1. [Install Rust using rustup](https://rust-lang.org/tools/install/). Follow the installer prompts, including the Visual Studio C++ Build Tools setup if needed.
2. Download or clone this repository.
3. Open PowerShell in the `orbital` folder, where `Cargo.toml` lives. If you just installed Rust, open a fresh terminal so it can find Cargo.
4. Build the program:

```powershell
cargo build
```

## Running it

### Desktop app

Simple Tauri 2.0 app that calls to the Rust backend for the Windows customization and preferences.

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

### Injecting an ISO

Open Inject, choose an official Windows installation ISO, then choose an answer file. Upload existing XML or configure installation settings in Orbital. The configurator automates choices shown by Windows 11 installation and first-run screens: edition, product key, license terms, disk selection, language/region, computer name, local account name/password, welcome pages and setup privacy. Advanced XML is a file-editing tool, not an additional Windows setting.

ISO injection supports Windows 11, including Enterprise LTSC, Enterprise N LTSC and IoT Enterprise LTSC. Windows 10 ISOs are rejected, including LTSC, based on the installation image's version metadata rather than the ISO filename. Select the matching edition under Installation and use Windows 11 LTSC installation media for LTSC editions. The edition list shows the exact image names from your selected ISO, including evaluation or renamed images.

Leave the installation key blank to skip key entry. Disk erasure requires typing the disk number again, including after importing saved settings. All blocking issues appear together and link to the affected controls. Downloaded XML generated from installation settings can be reopened as editable settings. JSON settings containing removed advanced options are rejected; import the original XML through Complete XML to preserve them. Imported complete XML is preserved. Exports may contain readable account passwords. Validate generated files with the intended Windows image in a disposable VM.

The Easy installation preset accepts and skips license terms, skips manufacturer registration, Wi-Fi setup and key entry, and declines optional setup data sharing. Choose your edition, language and accounts separately. Windows defaults restores untouched settings. Debug mode reveals example presets that retain disk-erasure confirmation.

Click Use answer file, then Inject ISO. Choose Duplicate & inject to save a new image, or Inject selected ISO to replace the selected file. Any existing autounattend.xml at the image root is replaced. Erasing a disk creates the standard UEFI layout with a recovery partition. No Wi-Fi profile, installation-driver paths or embedded scripts are generated.

Orbital uses Windows' built-in image APIs, preserves the original BIOS/UEFI boot images, and compares every setup file with the rebuilt image before saving it. Failed or cancelled operations leave the source unchanged. Allow free space for the rebuilt image as well as temporary staging files. Eject an ISO mounted in File Explorer before choosing to replace it.

### Command line

Show the available commands:

```powershell
cargo run -- --help
```

Preview pending recommended changes and confirm before applying (add `--yes` to skip confirmation):

```powershell
cargo run -- apply-recommended-settings
```

To build an executable:

```powershell
cargo build --release
.\target\release\orbital.exe --help
```

Once the CLI executable is built, it can be called directly from the command prompt while in the same directory.

```powershell
orbital hide-task-view hide-recent-files hide-frequent-folders
```
