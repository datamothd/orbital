mod answer;
mod image;
#[cfg(test)]
mod tests;

use answer::validate_answer;
use image::{Com, MountedIso, active, error};
use serde_json::{Value, json};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read},
    os::windows::{fs::OpenOptionsExt, io::AsRawHandle},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{State, Window, ipc::Channel};
use windows::{
    Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::{
            BY_HANDLE_FILE_INFORMATION, FILE_SHARE_READ, GetFileInformationByHandle,
            MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
        },
    },
    core::HSTRING,
};
use windows_sys::core::w;

#[derive(Default)]
struct Selection {
    iso: Option<PathBuf>,
    xml: Option<Vec<u8>>,
}

impl Selection {
    fn set_answer(&mut self, bytes: Vec<u8>) -> io::Result<()> {
        validate_answer(&bytes)?;
        self.xml = Some(bytes);
        Ok(())
    }
}

#[derive(Default)]
pub struct InjectionState {
    selection: Arc<Mutex<Selection>>,
    cancelled: Arc<AtomicBool>,
}

struct Workspace(PathBuf);
impl Workspace {
    fn new(parent: &Path) -> io::Result<Self> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        let path = parent.join(format!(".orbital-inject-{}-{nonce}", std::process::id()));
        fs::create_dir(&path)?;
        Ok(Self(path))
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn extension(path: &Path, kind: &str) -> io::Result<()> {
    if !path
        .extension()
        .is_some_and(|value| value.eq_ignore_ascii_case(kind))
    {
        return Err(error(format!("Choose a .{kind} file.")));
    }
    Ok(())
}

#[tauri::command]
pub async fn select_injection_file(
    window: Window,
    state: State<'_, InjectionState>,
    kind: String,
) -> Result<Option<Value>, String> {
    let selection = state.selection.clone();
    crate::files::with_dialog(window, move |owner| {
        let mut selection = selection
            .try_lock()
            .map_err(|_| "An injection operation is already in progress.".to_string())?;
        let (filter, title, ext) = match kind.as_str() {
            "iso" => (
                w!("Windows ISO (*.iso)\0*.iso\0"),
                w!("Choose a Windows installation ISO"),
                w!("iso"),
            ),
            "xml" => (
                w!("Answer file (*.xml)\0*.xml\0"),
                w!("Choose an answer file"),
                w!("xml"),
            ),
            _ => return Err("Unknown injection file type.".into()),
        };
        let Some(path) = crate::files::choose(owner, filter, title, ext, None)? else {
            return Ok(None);
        };
        let result: io::Result<_> = (|| {
            extension(&path, &kind)?;
            if kind == "iso" {
                image::boot_images(&mut File::open(&path)?)?;
                selection.iso = Some(path.clone());
            } else {
                let bytes = crate::files::read_document(&path)?;
                selection.set_answer(bytes)?;
            }
            Ok(Some(json!({
                "name": path.file_name().unwrap_or_default().to_string_lossy()
            })))
        })();
        result.map_err(|reason| reason.to_string())
    })
    .await
}

#[tauri::command]
pub fn set_answer_file(state: State<'_, InjectionState>, xml: String) -> Result<Value, String> {
    state
        .selection
        .try_lock()
        .map_err(|_| "An injection operation is already in progress.")?
        .set_answer(xml.into_bytes())
        .map_err(|error| error.to_string())?;
    Ok(json!({ "name": "Orbital answer file.xml" }))
}

#[tauri::command]
pub async fn answer_document(
    window: Window,
    content: Option<String>,
    kind: String,
) -> Result<Option<String>, String> {
    crate::files::with_dialog(window, move |owner| {
        let (filter, ext, name) = match kind.as_str() {
            "json" => (
                w!("Orbital settings (*.json)\0*.json\0"),
                w!("json"),
                "orbital-answer.json",
            ),
            "xml" => (
                w!("Answer file (*.xml)\0*.xml\0"),
                w!("xml"),
                "autounattend.xml",
            ),
            "open" if content.is_none() => (
                w!("Answer settings (*.xml;*.json)\0*.xml;*.json\0"),
                w!("xml"),
                "autounattend.xml",
            ),
            _ => return Err("Choose XML or JSON.".into()),
        };
        if let Some(text) = &content {
            if text.len() as u64 > crate::files::MAX_DOCUMENT_SIZE {
                return Err("Choose a file under 1 MiB.".into());
            }
            if kind == "xml" {
                validate_answer(text.as_bytes()).map_err(|error| error.to_string())?;
            } else {
                let value: Value = serde_json::from_str(text).map_err(|error| error.to_string())?;
                if value["format"] != "orbital-answer" || value["version"] != 1 {
                    return Err("Choose Orbital answer settings.".into());
                }
            }
        }
        let Some(path) = crate::files::choose(
            owner,
            filter,
            w!("Answer file"),
            ext,
            content.as_ref().map(|_| Path::new(name)),
        )?
        else {
            return Ok(None);
        };
        if let Some(text) = content {
            extension(&path, &kind).map_err(|error| error.to_string())?;
            let workspace = Workspace::new(path.parent().ok_or("Choose an output folder.")?)
                .map_err(|error| error.to_string())?;
            let temporary = workspace.0.join("answer.tmp");
            fs::write(&temporary, text).map_err(|error| error.to_string())?;
            unsafe {
                MoveFileExW(
                    &HSTRING::from(temporary.as_path()),
                    &HSTRING::from(path.as_path()),
                    MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
                )
            }
            .map_err(|error| error.to_string())?;
            Ok(Some("File saved.".into()))
        } else {
            let bytes = crate::files::read_document(&path).map_err(|error| error.to_string())?;
            answer::decode(&bytes)
                .map(Some)
                .map_err(|error| error.to_string())
        }
    })
    .await
}

#[tauri::command]
pub fn clear_injection_file(state: State<'_, InjectionState>, kind: String) -> Result<(), String> {
    let mut selection = state
        .selection
        .try_lock()
        .map_err(|_| "An injection operation is already in progress.".to_string())?;
    match kind.as_str() {
        "iso" => selection.iso = None,
        "xml" => selection.xml = None,
        _ => return Err("Unknown injection file type.".into()),
    }
    Ok(())
}

#[tauri::command]
pub fn cancel_injection(state: State<'_, InjectionState>) {
    state.cancelled.store(true, Ordering::Relaxed);
}

fn identity(file: &File) -> io::Result<(u32, u32, u32)> {
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    unsafe {
        GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &mut info)?;
    }
    Ok((
        info.dwVolumeSerialNumber,
        info.nFileIndexHigh,
        info.nFileIndexLow,
    ))
}

fn verify_tree(
    source: &Path,
    output: &Path,
    cancelled: &AtomicBool,
    root: bool,
    buffer: &mut [u8],
) -> io::Result<()> {
    for entry in fs::read_dir(source)? {
        active(cancelled)?;
        let entry = entry?;
        if root && entry.file_name().eq_ignore_ascii_case("autounattend.xml") {
            continue;
        }
        let target = output.join(entry.file_name());
        let metadata = entry.metadata()?;
        let actual = fs::metadata(&target)?;
        if metadata.is_dir() {
            if !actual.is_dir() {
                return Err(error("A setup directory is missing from the new ISO."));
            }
            verify_tree(&entry.path(), &target, cancelled, false, buffer)?;
        } else if actual.len() != metadata.len() || !actual.is_file() {
            return Err(error(format!(
                "The new ISO has an incomplete setup file: {}",
                entry.file_name().to_string_lossy()
            )));
        } else {
            let mut expected = File::open(entry.path())?;
            let mut actual = File::open(&target)?;
            let (left, right) = buffer.split_at_mut(buffer.len() / 2);
            loop {
                active(cancelled)?;
                let read = expected.read(left)?;
                if read == 0 {
                    break;
                }
                actual.read_exact(&mut right[..read])?;
                if left[..read] != right[..read] {
                    return Err(error(format!(
                        "Setup file verification failed: {}",
                        entry.file_name().to_string_lossy()
                    )));
                }
            }
        }
    }
    Ok(())
}

fn inject(
    source: &Path,
    answer: &[u8],
    destination: &Path,
    replace: bool,
    cancelled: &AtomicBool,
    progress: &mut impl FnMut(&str, Option<u8>),
) -> io::Result<()> {
    validate_answer(answer)?;
    extension(source, "iso")?;
    extension(destination, "iso")?;
    let _com = Com::new()?;
    let mut source_file = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ.0)
        .open(source)?;
    if replace && source_file.metadata()?.permissions().readonly() {
        return Err(error(
            "The selected ISO is read-only. Save an injected copy or remove its read-only attribute.",
        ));
    }
    if !replace
        && destination.exists()
        && identity(&source_file)? == identity(&File::open(destination)?)?
    {
        return Err(error(
            "Choose a different output path to keep the original ISO.",
        ));
    }
    if replace && source.canonicalize()? != destination.canonicalize()? {
        return Err(error("The replacement path must be the selected ISO."));
    }
    let boots = image::boot_images(&mut source_file)?;
    active(cancelled)?;
    progress("Mounting the source ISO read-only…", None);
    let mounted = MountedIso::open(source)?;
    if replace && mounted.existing {
        return Err(error(
            "Eject the selected ISO in File Explorer before replacing it.",
        ));
    }
    if !mounted.root.join("setup.exe").is_file()
        || !mounted.root.join("sources/boot.wim").is_file()
        || ![
            "sources/install.wim",
            "sources/install.esd",
            "sources/install.swm",
        ]
        .iter()
        .any(|name| mounted.root.join(name).is_file())
    {
        return Err(error(
            "Choose a Windows installation ISO containing setup.exe, boot.wim, and an install image.",
        ));
    }
    let workspace = Workspace::new(
        destination
            .parent()
            .ok_or_else(|| error("Choose an output folder."))?,
    )?;
    let temporary = workspace.0.join("injected.iso");
    image::build(
        &mounted.root,
        &boots,
        answer,
        &temporary,
        cancelled,
        progress,
    )?;
    active(cancelled)?;
    progress(
        "Verifying the answer file, setup files, and boot images…",
        None,
    );
    if image::boot_images(&mut File::open(&temporary)?)? != boots {
        return Err(error("Boot image verification failed."));
    }
    {
        let output = MountedIso::open(&temporary)?;
        if fs::read(output.root.join("autounattend.xml"))? != answer {
            return Err(error("Answer file verification failed."));
        }
        verify_tree(
            &mounted.root,
            &output.root,
            cancelled,
            true,
            &mut vec![0u8; 2 * 1024 * 1024],
        )?;
    }
    active(cancelled)?;
    drop(mounted);
    drop(source_file);
    progress("Saving the verified ISO…", None);
    unsafe {
        MoveFileExW(
            &HSTRING::from(temporary.as_path()),
            &HSTRING::from(destination),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )?;
    }
    Ok(())
}

#[tauri::command]
pub async fn inject_iso(
    window: Window,
    state: State<'_, InjectionState>,
    duplicate: bool,
    progress: Channel<Value>,
) -> Result<Option<String>, String> {
    let selection = state.selection.clone();
    let cancelled = state.cancelled.clone();
    crate::files::with_dialog(window, move |owner| {
        let selection = selection
            .try_lock()
            .map_err(|_| "An injection operation is already in progress.".to_string())?;
        let source = selection.iso.as_ref().ok_or("Choose a source ISO first.")?;
        let answer = selection
            .xml
            .as_ref()
            .ok_or("Choose an answer file first.")?;
        cancelled.store(false, Ordering::Relaxed);
        let destination = if duplicate {
            let suggested = source.with_file_name(format!(
                "{}-orbital.iso",
                source.file_stem().unwrap_or_default().to_string_lossy()
            ));
            let Some(path) = crate::files::choose(
                owner,
                w!("Windows ISO (*.iso)\0*.iso\0"),
                w!("Save injected ISO copy"),
                w!("iso"),
                Some(&suggested),
            )?
            else {
                return Ok(None);
            };
            path
        } else {
            source.clone()
        };
        let mut report = |message: &str, percent: Option<u8>| {
            let _ = progress.send(json!({ "message": message, "percent": percent }));
        };
        match inject(
            source,
            answer,
            &destination,
            !duplicate,
            &cancelled,
            &mut report,
        ) {
            Ok(()) => Ok(Some(destination.to_string_lossy().into_owned())),
            Err(reason) if reason.kind() == io::ErrorKind::Interrupted => Ok(None),
            Err(reason) => Err(format!("{reason} The source ISO was not changed.")),
        }
    })
    .await
}
