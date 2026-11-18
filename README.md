# Orbital

<img width="160" height="160" src="exports/bohr-8-of-8.gif" alt="Animated Bohr diagram" />

Orbital is a Windows 11 tool for changing system preferences, removing unwanted apps, and preparing a Windows installation ISO for unattended setup.

## What you can do

- **Tweaks** — Change File Explorer, taskbar, and desktop preferences. Review pending changes before applying them, try the recommended settings, or revert saved changes. Export tweaks to JSON and choose which settings to apply when importing a profile.
- **Debloat** — Browse installed apps and default Windows apps, then select what to uninstall. System-wide changes and some removal options require administrator access.
- **Inject** — Create or import an `autounattend.xml` answer file and add it to a Windows 11 ISO.
- **System information** — See your Windows version, CPU, thread count, and memory usage.

## Run from source

Orbital targets Windows 11 Pro 25H2 and 26H2. To build the desktop app, install Rust, Visual Studio C++ Build Tools, Node.js, and Microsoft Edge WebView2.

From the repository folder, run:

```powershell
npm ci
npm run dev
```

To build and run the standalone desktop executable:

```powershell
npm run build
.\src-tauri\target\release\orbital-desktop.exe
```

## Prepare a Windows ISO

1. Open **Inject** and choose an official Windows 11 ISO. Windows 11 LTSC media is supported.
2. Create an answer file in Orbital or import your own XML. The editor can prefill choices such as edition, language, local accounts, and setup privacy. Select an edition available on your ISO.
3. Select **Inject ISO**, then choose **Duplicate & inject** to save a copy or **Inject selected ISO** to replace the source. Orbital verifies the rebuilt image before saving it.

Injection replaces any existing `autounattend.xml` at the root of the output ISO. Answer files can contain readable passwords. Test the image in a virtual machine before using it, especially if your answer file erases a disk.

## Command line

Use Cargo to list commands or preview and apply recommended preferences:

```powershell
cargo run -- --help
cargo run -- apply-recommended-settings
```

Run `cargo build --release` to build `target\release\orbital.exe`.