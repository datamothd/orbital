use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
    os::windows::io::{FromRawHandle, OwnedHandle},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use windows::{
    Win32::{
        Foundation::{HANDLE, VARIANT_FALSE},
        Storage::{
            FileSystem::{GetDiskFreeSpaceExW, GetVolumeInformationW, QueryDosDeviceW},
            Imapi::{
                BootOptions, EmulationNone, FsiFileSystemUDF, IBootOptions, IFileSystemImage2,
                MsftFileSystemImage, PlatformId,
            },
            Vhd::*,
        },
        System::{
            Com::StructuredStorage::CreateStreamOnHGlobal,
            Com::{
                CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
                CoUninitialize, IDispatch, IStream, STREAM_SEEK_SET,
            },
            Ole::{SafeArrayCreateVector, SafeArrayDestroy, SafeArrayPutElement},
            Variant::{VARIANT, VT_VARIANT},
        },
    },
    core::{BSTR, HSTRING, Interface, PWSTR},
};

pub fn error(message: impl Into<String>) -> io::Error {
    io::Error::other(message.into())
}

pub fn active(cancelled: &AtomicBool) -> io::Result<()> {
    if cancelled.load(Ordering::Relaxed) {
        Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "Injection cancelled.",
        ))
    } else {
        Ok(())
    }
}

fn wide_slice(buffer: &[u16]) -> &[u16] {
    &buffer[..buffer
        .iter()
        .position(|ch| *ch == 0)
        .unwrap_or(buffer.len())]
}

pub struct Com;
impl Com {
    pub fn new() -> io::Result<Self> {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
        }
        Ok(Self)
    }
}
impl Drop for Com {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}

pub struct MountedIso {
    _handle: OwnedHandle,
    pub root: PathBuf,
    pub existing: bool,
}

impl MountedIso {
    pub fn open(path: &Path) -> io::Result<Self> {
        let mut handle = HANDLE::default();
        unsafe {
            OpenVirtualDisk(
                &VIRTUAL_STORAGE_TYPE {
                    DeviceId: VIRTUAL_STORAGE_TYPE_DEVICE_ISO,
                    VendorId: VIRTUAL_STORAGE_TYPE_VENDOR_MICROSOFT,
                },
                &HSTRING::from(path),
                VIRTUAL_DISK_ACCESS_READ,
                OPEN_VIRTUAL_DISK_FLAG_NONE,
                None,
                &mut handle,
            )
            .ok()?;
        }
        let owned = unsafe { OwnedHandle::from_raw_handle(handle.0) };
        let mut physical = [0u16; 256];
        let mut length = std::mem::size_of_val(&physical) as u32;
        let existing = unsafe {
            GetVirtualDiskPhysicalPath(handle, &mut length, PWSTR(physical.as_mut_ptr())).is_ok()
        };
        if !existing {
            unsafe {
                AttachVirtualDisk(
                    handle,
                    None,
                    ATTACH_VIRTUAL_DISK_FLAG_READ_ONLY,
                    0,
                    None,
                    None,
                )
                .ok()?;
            }
            length = std::mem::size_of_val(&physical) as u32;
            unsafe {
                GetVirtualDiskPhysicalPath(handle, &mut length, PWSTR(physical.as_mut_ptr()))
                    .ok()?;
            }
        }
        let physical = String::from_utf16_lossy(wide_slice(&physical));
        let device = physical
            .rsplit('\\')
            .next()
            .ok_or_else(|| error("Could not identify the mounted ISO."))?;
        for _ in 0..50 {
            for letter in b'A'..=b'Z' {
                let drive = format!("{}:", letter as char);
                let mut target = [0u16; 256];
                let found = unsafe { QueryDosDeviceW(&HSTRING::from(&drive), Some(&mut target)) };
                let target = String::from_utf16_lossy(wide_slice(&target));
                if found != 0
                    && target
                        .rsplit('\\')
                        .next()
                        .is_some_and(|name| name.eq_ignore_ascii_case(device))
                {
                    let root = PathBuf::from(format!("{drive}\\"));
                    if root.is_dir() {
                        return Ok(Self {
                            _handle: owned,
                            root,
                            existing,
                        });
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        Err(error(
            "Windows did not assign a drive letter to the ISO. Eject it and try again.",
        ))
    }
}

#[derive(Debug, PartialEq)]
pub struct BootImage {
    pub platform: u8,
    pub bytes: Vec<u8>,
}

fn read_at(file: &mut File, offset: u64, bytes: &mut [u8]) -> io::Result<()> {
    file.seek(SeekFrom::Start(offset))?;
    file.read_exact(bytes)
}

pub fn boot_images(file: &mut File) -> io::Result<Vec<BootImage>> {
    let size = file.metadata()?.len();
    let mut block = [0u8; 2048];
    let mut catalog = None;
    for sector in 16..64 {
        read_at(file, sector * 2048, &mut block)?;
        if &block[1..6] == b"CD001"
            && block[0] == 0
            && block[7..30].starts_with(b"EL TORITO SPECIFICATION")
        {
            catalog = Some(u32::from_le_bytes(block[71..75].try_into().unwrap()) as u64 * 2048);
            break;
        }
    }
    read_at(
        file,
        catalog.ok_or_else(|| error("Choose a bootable Windows installation ISO."))?,
        &mut block,
    )?;
    let checksum = block[..32]
        .as_chunks::<2>()
        .0
        .iter()
        .fold(0u16, |sum, pair| {
            sum.wrapping_add(u16::from_le_bytes(*pair))
        });
    if block[0] != 1 || block[30..32] != [0x55, 0xaa] || checksum != 0 {
        return Err(error("The ISO has an invalid boot catalog."));
    }
    let mut entries = vec![(block[1], &block[32..64])];
    let mut index = 64;
    while index + 32 <= block.len() && matches!(block[index], 0x90 | 0x91) {
        let platform = block[index + 1];
        let count = u16::from_le_bytes([block[index + 2], block[index + 3]]) as usize;
        let last = block[index] == 0x91;
        index += 32;
        if count == 0 || index + count * 32 > block.len() {
            return Err(error("The ISO boot catalog is unsupported."));
        }
        for entry in block[index..index + count * 32].as_chunks::<32>().0 {
            entries.push((platform, entry.as_slice()));
        }
        index += count * 32;
        if last {
            break;
        }
    }
    let mut images = Vec::new();
    for (platform, entry) in entries {
        if entry[0] != 0x88 {
            continue;
        }
        if !matches!(platform, 0 | 0xef)
            || entry[1] != 0
            || !matches!(u16::from_le_bytes([entry[2], entry[3]]), 0 | 0x7c0)
            || images
                .iter()
                .any(|image: &BootImage| image.platform == platform)
        {
            return Err(error(
                "The ISO uses unsupported boot options. Choose an original Windows ISO.",
            ));
        }
        let offset = u32::from_le_bytes(entry[8..12].try_into().unwrap()) as u64 * 2048;
        let mut header = [0u8; 512];
        read_at(file, offset, &mut header)?;
        let length = if platform == 0xef {
            let sector_size = u16::from_le_bytes([header[11], header[12]]) as u64;
            let short = u16::from_le_bytes([header[19], header[20]]) as u64;
            let sectors = if short != 0 {
                short
            } else {
                u32::from_le_bytes(header[32..36].try_into().unwrap()) as u64
            };
            if !matches!(sector_size, 512 | 1024 | 2048 | 4096) || header[510..512] != [0x55, 0xaa]
            {
                return Err(error("The ISO has an unsupported UEFI boot image."));
            }
            sector_size * sectors
        } else {
            u16::from_le_bytes([entry[6], entry[7]]) as u64 * 512
        };
        if !(512..=32 * 1024 * 1024).contains(&length)
            || offset.checked_add(length).is_none_or(|end| end > size)
        {
            return Err(error("The ISO contains a damaged boot image."));
        }
        let mut bytes = vec![0u8; length as usize];
        read_at(file, offset, &mut bytes)?;
        images.push(BootImage { platform, bytes });
    }
    if !images.iter().any(|image| image.platform == 0xef) {
        return Err(error("The Windows ISO must support UEFI boot."));
    }
    Ok(images)
}

fn memory_stream(bytes: &[u8]) -> io::Result<IStream> {
    unsafe {
        let stream = CreateStreamOnHGlobal(Default::default(), true)?;
        let mut written = 0;
        stream
            .Write(
                bytes.as_ptr().cast(),
                bytes.len() as u32,
                Some(&mut written),
            )
            .ok()?;
        if written as usize != bytes.len() {
            return Err(error("Could not prepare the injection data."));
        }
        stream.Seek(0, STREAM_SEEK_SET, None)?;
        Ok(stream)
    }
}

pub fn build(
    root: &Path,
    boots: &[BootImage],
    answer: &[u8],
    output: &Path,
    cancelled: &AtomicBool,
    progress: &mut impl FnMut(&str, Option<u8>),
) -> io::Result<()> {
    active(cancelled)?;
    let parent = output
        .parent()
        .ok_or_else(|| error("Choose an output folder."))?;
    unsafe {
        let image: IFileSystemImage2 =
            CoCreateInstance(&MsftFileSystemImage, None, CLSCTX_INPROC_SERVER)?;
        image.SetFileSystemsToCreate(FsiFileSystemUDF)?;
        image.SetUDFRevision(0x102)?;
        image.SetFreeMediaBlocks(0)?;
        image.SetStageFiles(VARIANT_FALSE)?;
        image.SetWorkingDirectory(&BSTR::from_wide(&HSTRING::from(parent)))?;
        let mut label = [0u16; 128];
        let volume = if GetVolumeInformationW(
            &HSTRING::from(root),
            Some(&mut label),
            None,
            None,
            None,
            None,
        )
        .is_ok()
        {
            BSTR::from_wide(wide_slice(&label))
        } else {
            BSTR::from("ORBITAL")
        };
        image.SetVolumeName(&volume)?;
        let options = SafeArrayCreateVector(VT_VARIANT, 0, boots.len() as u32);
        if options.is_null() {
            return Err(error("Could not allocate the boot image options."));
        }
        let result: io::Result<()> = (|| {
            for (index, boot) in boots.iter().enumerate() {
                let option: IBootOptions =
                    CoCreateInstance(&BootOptions, None, CLSCTX_INPROC_SERVER)?;
                option.AssignBootImage(&memory_stream(&boot.bytes)?)?;
                option.SetPlatformId(PlatformId(boot.platform as i32))?;
                option.SetEmulation(EmulationNone)?;
                let value = VARIANT::from(option.cast::<IDispatch>()?);
                SafeArrayPutElement(options, &(index as i32), (&value as *const VARIANT).cast())?;
            }
            image.SetBootImageOptionsArray(options)?;
            Ok(())
        })();
        let _ = SafeArrayDestroy(options);
        result?;
        progress("Reading Windows setup files…", None);
        let directory = image.Root()?;
        directory.AddTree(&BSTR::from_wide(&HSTRING::from(root)), VARIANT_FALSE)?;
        if root.join("autounattend.xml").exists() {
            directory.Remove(&BSTR::from("autounattend.xml"))?;
        }
        directory.AddFile(&BSTR::from("autounattend.xml"), &memory_stream(answer)?)?;
        active(cancelled)?;
        progress("Building the bootable image…", None);
        let result = image.CreateResultImage()?;
        let total = result.TotalBlocks()? as u64 * result.BlockSize()? as u64;
        let mut available = 0;
        GetDiskFreeSpaceExW(&HSTRING::from(parent), Some(&mut available), None, None)?;
        if available < total {
            return Err(error(format!(
                "Not enough free space. The new ISO needs {:.1} GiB.",
                total as f64 / 1_073_741_824.0
            )));
        }
        let stream = result.ImageStream()?;
        let mut file = File::create_new(output)?;
        let mut buffer = vec![0u8; 4 * 1024 * 1024];
        let mut written = 0u64;
        let mut previous = None;
        while written < total {
            active(cancelled)?;
            let mut read = 0;
            stream
                .Read(
                    buffer.as_mut_ptr().cast(),
                    buffer.len().min((total - written) as usize) as u32,
                    Some(&mut read),
                )
                .ok()?;
            if read == 0 {
                return Err(error("The image stream ended before it was complete."));
            }
            file.write_all(&buffer[..read as usize])?;
            written += read as u64;
            let percent = Some((written * 100 / total) as u8);
            if percent != previous {
                progress("Writing the ISO…", percent);
                previous = percent;
            }
        }
        file.sync_all()?;
    }
    Ok(())
}
