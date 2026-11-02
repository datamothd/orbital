use super::image::error;
use serde::Deserialize;
use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    path::Path,
};

const UNSUPPORTED: &str =
    "Choose a Windows 11 installation ISO. Windows 10, including LTSC, is not supported.";

#[derive(Deserialize)]
struct Images {
    #[serde(rename = "IMAGE", default)]
    images: Vec<Image>,
}

#[derive(Deserialize)]
#[serde(rename_all = "UPPERCASE")]
struct Image {
    #[serde(default)]
    name: String,
    windows: Windows,
}

#[derive(Deserialize)]
#[serde(rename_all = "UPPERCASE")]
struct Windows {
    version: Version,
    producttype: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "UPPERCASE")]
struct Version {
    major: u32,
    minor: u32,
    build: u32,
}

pub(super) fn validate_install_image(path: &Path) -> io::Result<Vec<String>> {
    let mut file = File::open(path)?;
    let mut header = [0u8; 208];
    file.read_exact(&mut header)?;
    if &header[..8] != b"MSWIM\0\0\0" {
        return Err(error("The installation image has an invalid WIM header."));
    }
    let size = u64::from_le_bytes(header[72..80].try_into().unwrap()) & 0x00ff_ffff_ffff_ffff;
    let offset = u64::from_le_bytes(header[80..88].try_into().unwrap());
    let original_size = u64::from_le_bytes(header[88..96].try_into().unwrap());
    let file_size = file.metadata()?.len();
    if !(2..=16 * 1024 * 1024).contains(&size)
        || !size.is_multiple_of(2)
        || size != original_size
        || header[79] & !2 != 0
        || offset < header.len() as u64
        || offset.checked_add(size).is_none_or(|end| end > file_size)
    {
        return Err(error(
            "Could not read the installation image's Windows version.",
        ));
    }
    file.seek(SeekFrom::Start(offset))?;
    let mut bytes = vec![0u8; size as usize];
    file.read_exact(&mut bytes)?;
    let words: Vec<_> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .collect();
    if words.first() != Some(&0xfeff) {
        return Err(error(
            "The installation image has invalid version metadata.",
        ));
    }
    let xml = String::from_utf16(&words[1..]).map_err(io::Error::other)?;
    let images: Images = quick_xml::de::from_str(&xml).map_err(io::Error::other)?;
    let count = u32::from_le_bytes(header[44..48].try_into().unwrap());
    if images.images.is_empty()
        || images.images.len() != count as usize
        || images.images.iter().any(|image| {
            let version = &image.windows.version;
            version.major != 10
                || version.minor != 0
                || version.build < 22000
                || image.windows.producttype != "WinNT"
        })
    {
        return Err(error(UNSUPPORTED));
    }
    Ok(images.images.into_iter().map(|image| image.name).filter(|name| !name.trim().is_empty()).collect())
}

pub(super) fn validate_source(root: &Path) -> io::Result<Vec<String>> {
    if !root.join("setup.exe").is_file() || !root.join("sources/boot.wim").is_file() {
        return Err(error(
            "Choose a Windows 11 installation ISO containing setup.exe, boot.wim, and an install image.",
        ));
    }
    let mut found = false;
    let mut names = Vec::new();
    for name in [
        "sources/install.wim",
        "sources/install.esd",
        "sources/install.swm",
    ] {
        let path = root.join(name);
        if path.is_file() {
            names.extend(validate_install_image(&path)?);
            found = true;
        }
    }
    if !found {
        return Err(error(
            "Choose a Windows 11 installation ISO containing an install image.",
        ));
    }
    names.sort();
    names.dedup();
    Ok(names)
}
