use std::os::windows::{
    ffi::OsStrExt,
    io::{FromRawHandle, OwnedHandle},
};
use tauri::State;
use windows::Win32::System::Com::{
    COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoUninitialize,
};
use windows_sys::{
    Win32::UI::Shell::{
        SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
        ShellExecuteExW,
    },
    core::w,
};

#[tauri::command]
pub fn administrator_status() -> Result<bool, String> {
    orbital::explorer::is_administrator().map_err(|error| error.to_string())
}

fn launch_elevated(owner: usize) -> Result<bool, String> {
    if administrator_status()? {
        return Err("Administrator access is already active.".into());
    }
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let filename: Vec<u16> = executable
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let directory: Vec<u16> = executable
        .parent()
        .ok_or("The app directory is unavailable.")?
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let mut request = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOASYNC | SEE_MASK_NOCLOSEPROCESS | SEE_MASK_FLAG_NO_UI,
        hwnd: owner as _,
        lpVerb: w!("runas"),
        lpFile: filename.as_ptr(),
        lpDirectory: directory.as_ptr(),
        nShow: 1,
        ..Default::default()
    };
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE)
            .ok()
            .map_err(|error| error.to_string())?;
    }
    let launched = unsafe { ShellExecuteExW(&mut request) };
    let error = std::io::Error::last_os_error();
    unsafe {
        CoUninitialize();
    }
    if launched == 0 {
        if error.raw_os_error() == Some(1223) {
            return Ok(false);
        }
        return Err(format!(
            "Could not relaunch Orbital as administrator: {error}"
        ));
    }
    if request.hProcess.is_null() {
        return Err("Windows did not confirm that the elevated app started. The current window will stay open.".into());
    }
    let _process = unsafe { OwnedHandle::from_raw_handle(request.hProcess) };
    Ok(true)
}

#[tauri::command]
pub async fn relaunch_as_administrator(
    app: tauri::AppHandle,
    window: tauri::Window,
    removal: State<'_, crate::debloat::RemovalState>,
    injection: State<'_, crate::inject::InjectionState>,
) -> Result<bool, String> {
    let owner = window.hwnd().map_err(|error| error.to_string())?.0 as usize;
    let removal = removal.inner().clone();
    let injection = injection.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        removal.while_idle(|| {
            injection.while_idle(|| {
                let launched = launch_elevated(owner)?;
                if launched {
                    app.exit(0);
                }
                Ok(launched)
            })
        })
    })
    .await
    .map_err(|error| error.to_string())?
}
