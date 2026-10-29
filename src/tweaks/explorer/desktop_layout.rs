use std::{
    io,
    time::{Duration, Instant},
};
use windows::{
    Win32::{
        Foundation::{E_FAIL, POINT, PROPERTYKEY, RPC_E_CHANGED_MODE},
        System::{
            Com::{
                CLSCTX_LOCAL_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
                CoTaskMemFree, CoUninitialize, IServiceProvider,
            },
            Variant::VARIANT,
        },
        UI::Shell::{
            CSIDL_DESKTOP, Common::ITEMIDLIST, FWF_AUTOARRANGE, FWF_SNAPTOGRID, IFolderView2,
            IShellBrowser, IShellView, IShellWindows, SID_STopLevelBrowser, SORTCOLUMN,
            SVGIO_ALLVIEW, SVSI_POSITIONITEM, SWC_DESKTOP, SWFO_NEEDDISPATCH, ShellWindows,
        },
    },
    core::{BOOL, Interface},
};

const ARRANGEMENT_FLAGS: u32 = (FWF_AUTOARRANGE.0 | FWF_SNAPTOGRID.0) as u32;

struct Apartment(bool);

impl Apartment {
    fn initialize() -> windows::core::Result<Self> {
        let result = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        if result == RPC_E_CHANGED_MODE {
            Ok(Self(false))
        } else {
            result.ok()?;
            Ok(Self(true))
        }
    }
}

impl Drop for Apartment {
    fn drop(&mut self) {
        if self.0 {
            unsafe { CoUninitialize() };
        }
    }
}

struct ItemId(*mut ITEMIDLIST);

impl Drop for ItemId {
    fn drop(&mut self) {
        unsafe { CoTaskMemFree(Some(self.0.cast())) };
    }
}

fn desktop_view() -> windows::core::Result<IShellView> {
    unsafe {
        let windows: IShellWindows = CoCreateInstance(&ShellWindows, None, CLSCTX_LOCAL_SERVER)?;
        let location = VARIANT::from(CSIDL_DESKTOP as i32);
        let mut handle = 0;
        let dispatch = windows.FindWindowSW(
            &location,
            &VARIANT::default(),
            SWC_DESKTOP,
            &mut handle,
            SWFO_NEEDDISPATCH,
        )?;
        let provider: IServiceProvider = dispatch.cast()?;
        let browser: IShellBrowser = provider.QueryService(&SID_STopLevelBrowser)?;
        browser.QueryActiveShellView()
    }
}

pub(super) struct DesktopLayout {
    items: Vec<(ItemId, POINT)>,
    flags: u32,
    sort: Vec<SORTCOLUMN>,
    group: PROPERTYKEY,
    group_ascending: BOOL,
    _apartment: Apartment,
}

impl DesktopLayout {
    pub(super) fn capture() -> io::Result<Self> {
        Self::capture_inner().map_err(|error| {
            io::Error::other(format!(
                "Could not capture the desktop layout; Explorer was not restarted: {error}"
            ))
        })
    }

    fn capture_inner() -> windows::core::Result<Self> {
        let apartment = Apartment::initialize()?;
        let shell = desktop_view()?;
        let view: IFolderView2 = shell.cast()?;
        unsafe {
            let flags = view.GetCurrentFolderFlags()?;
            let mut sort = vec![SORTCOLUMN::default(); view.GetSortColumnCount()? as usize];
            let mut group = PROPERTYKEY::default();
            let mut group_ascending = BOOL::default();
            let mut items = Vec::new();
            view.GetSortColumns(&mut sort)?;
            view.GetGroupBy(&mut group, Some(&mut group_ascending))?;
            for index in 0..view.ItemCount(SVGIO_ALLVIEW)? {
                let item = ItemId(view.Item(index)?);
                let position = view.GetItemPosition(item.0)?;
                items.push((item, position));
            }
            shell.SaveViewState()?;
            Ok(Self {
                items,
                flags,
                sort,
                group,
                group_ascending,
                _apartment: apartment,
            })
        }
    }

    pub(super) fn restore(&self) -> io::Result<()> {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            match self.restore_inner() {
                Ok(()) => return Ok(()),
                Err(error) if Instant::now() >= deadline => {
                    return Err(io::Error::other(format!(
                        "Explorer restarted, but the desktop layout could not be restored: {error}"
                    )));
                }
                Err(_) => std::thread::sleep(Duration::from_millis(100)),
            }
        }
    }

    fn restore_inner(&self) -> windows::core::Result<()> {
        let shell = desktop_view()?;
        let view: IFolderView2 = shell.cast()?;
        unsafe {
            for (item, _) in &self.items {
                view.GetItemPosition(item.0)?;
            }
            view.SetGroupBy(&self.group, self.group_ascending.as_bool())?;
            view.SetSortColumns(&self.sort)?;
            view.SetCurrentFolderFlags(ARRANGEMENT_FLAGS, 0)?;
            let positioned = self.items.iter().try_for_each(|(item, position)| {
                view.SelectAndPositionItems(
                    1,
                    &item.0.cast_const(),
                    Some(position),
                    SVSI_POSITIONITEM.0 as u32,
                )
            });
            let flags = view.SetCurrentFolderFlags(ARRANGEMENT_FLAGS, self.flags);
            positioned?;
            flags?;
            self.verify(&view)?;
            shell.SaveViewState()
        }
    }

    fn verify(&self, view: &IFolderView2) -> windows::core::Result<()> {
        unsafe {
            for (item, expected) in &self.items {
                let actual = view.GetItemPosition(item.0)?;
                if (actual.x, actual.y) != (expected.x, expected.y) {
                    return Err(E_FAIL.into());
                }
            }
            if view.GetCurrentFolderFlags()? & ARRANGEMENT_FLAGS != self.flags & ARRANGEMENT_FLAGS {
                return Err(E_FAIL.into());
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "Requires an interactive Windows desktop; reads the live shell and saves its current view"]
    fn captures_live_desktop_positions() {
        let layout = DesktopLayout::capture().unwrap();
        let shell = desktop_view().unwrap();
        layout.verify(&shell.cast().unwrap()).unwrap();
    }

    #[test]
    #[ignore = "Restarts Explorer and verifies the live desktop; run explicitly in an interactive session"]
    fn explorer_restart_preserves_live_desktop_positions() {
        let layout = DesktopLayout::capture().unwrap();
        assert!(
            !layout.items.is_empty(),
            "Place an icon on the desktop before running this test"
        );
        super::super::restart_explorer("Desktop layout verification", false, true).unwrap();
        std::thread::sleep(Duration::from_secs(1));
        let shell = desktop_view().unwrap();
        layout.verify(&shell.cast().unwrap()).unwrap();
    }
}
