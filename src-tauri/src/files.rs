use std::{
    ffi::OsString,
    fs::File,
    io::{self, Read},
    os::windows::ffi::{OsStrExt, OsStringExt},
    path::{Path, PathBuf},
};
use windows_sys::{
    Win32::UI::Controls::Dialogs::{
        CommDlgExtendedError, GetOpenFileNameW, GetSaveFileNameW, OFN_EXPLORER, OFN_FILEMUSTEXIST,
        OFN_NOCHANGEDIR, OFN_OVERWRITEPROMPT, OFN_PATHMUSTEXIST, OPENFILENAMEW,
    },
    core::PCWSTR,
};

pub const MAX_DOCUMENT_SIZE: u64 = 1_048_576;

pub fn read_document(path: &Path) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(MAX_DOCUMENT_SIZE + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_DOCUMENT_SIZE {
        return Err(io::Error::other("Choose a file under 1 MiB."));
    }
    Ok(bytes)
}

pub async fn with_dialog<T: Send + 'static>(
    window: tauri::Window,
    operation: impl FnOnce(usize) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let owner = window.hwnd().map_err(|error| error.to_string())?.0 as usize;
    tauri::async_runtime::spawn_blocking(move || operation(owner))
        .await
        .map_err(|error| error.to_string())?
}

pub fn choose(
    owner: usize,
    filter: PCWSTR,
    title: PCWSTR,
    extension: PCWSTR,
    output: Option<&Path>,
) -> Result<Option<PathBuf>, String> {
    let mut filename = vec![0u16; 32768];
    if let Some(path) = output {
        let suggested: Vec<_> = path.as_os_str().encode_wide().collect();
        if suggested.len() >= filename.len() {
            return Err("The file path is too long.".into());
        }
        filename[..suggested.len()].copy_from_slice(&suggested);
    }
    let mut dialog = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: owner as _,
        lpstrFilter: filter,
        nFilterIndex: 1,
        lpstrFile: filename.as_mut_ptr(),
        nMaxFile: filename.len() as u32,
        lpstrTitle: title,
        lpstrDefExt: extension,
        Flags: OFN_EXPLORER
            | OFN_NOCHANGEDIR
            | OFN_PATHMUSTEXIST
            | if output.is_some() {
                OFN_OVERWRITEPROMPT
            } else {
                OFN_FILEMUSTEXIST
            },
        ..Default::default()
    };
    let chosen = unsafe {
        if output.is_some() {
            GetSaveFileNameW(&mut dialog)
        } else {
            GetOpenFileNameW(&mut dialog)
        }
    };
    if chosen == 0 {
        let error = unsafe { CommDlgExtendedError() };
        return if error == 0 {
            Ok(None)
        } else {
            Err(format!("Could not open the file dialog ({error:#x})."))
        };
    }
    let length = filename
        .iter()
        .position(|ch| *ch == 0)
        .unwrap_or(filename.len());
    Ok(Some(OsString::from_wide(&filename[..length]).into()))
}
