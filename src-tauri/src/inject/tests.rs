use super::*;
use crate::files::MAX_DOCUMENT_SIZE;
use image::BootImage;
use std::io::{Seek, SeekFrom, Write};

const ANSWER: &[u8] = br#"<?xml version="1.0"?><unattend xmlns="urn:schemas-microsoft-com:unattend"><settings pass="oobeSystem"/></unattend>"#;
static IMAGE_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn invalid_generated_answers_preserve_the_previous_selection() {
    let mut selection = Selection::default();
    selection.set_answer(ANSWER.to_vec()).unwrap();
    assert!(selection.set_answer(b"invalid".to_vec()).is_err());
    assert_eq!(selection.xml.as_deref(), Some(ANSWER));
}

#[test]
fn document_decoding_rejects_malformed_utf16_and_preserves_utf8() {
    assert_eq!(answer::decode(ANSWER).unwrap().as_bytes(), ANSWER);
    assert!(answer::decode(&[0xff, 0xfe, 0]).is_err());
    assert!(answer::decode(&[0xff]).is_err());
}

#[test]
fn answer_files_accept_namespaces_scripts_and_supported_encodings() {
    validate_answer(ANSWER).unwrap();
    validate_answer(&[&[0xef, 0xbb, 0xbf], ANSWER].concat()).unwrap();
    let text = std::str::from_utf8(ANSWER).unwrap();
    for little in [false, true] {
        let mut bytes = if little {
            vec![0xff, 0xfe]
        } else {
            vec![0xfe, 0xff]
        };
        for character in text.encode_utf16() {
            bytes.extend(if little {
                character.to_le_bytes()
            } else {
                character.to_be_bytes()
            });
        }
        validate_answer(&bytes).unwrap();
    }
    validate_answer(br#"<u:unattend xmlns:u="urn:schemas-microsoft-com:unattend"><u:settings>&amp; &#65; <![CDATA[Write-Host 'a<b']]> </u:settings></u:unattend>"#).unwrap();
}

#[test]
fn malformed_oversized_and_external_entity_answers_are_rejected() {
    for bytes in [
        b"".as_slice(),
        b"not XML",
        b"<unattend/>",
        br#"<unattend xmlns="wrong"/>"#,
        br#"<unattend xmlns="urn:schemas-microsoft-com:unattend">"#,
        br#"<unattend xmlns="urn:schemas-microsoft-com:unattend"><settings></wrong></unattend>"#,
        br#"<unattend xmlns="urn:schemas-microsoft-com:unattend"><unknown:settings/></unattend>"#,
        br#"<unattend xmlns="urn:schemas-microsoft-com:unattend">&unknown;</unattend>"#,
        br#"<!DOCTYPE unattend [<!ENTITY x SYSTEM "file:///secret">]><unattend xmlns="urn:schemas-microsoft-com:unattend">&x;</unattend>"#,
        br#"<unattend xmlns="urn:schemas-microsoft-com:unattend" x="1" x="2"/>"#,
        br#"<unattend xmlns="urn:schemas-microsoft-com:unattend"/><second/>"#,
    ] {
        assert!(
            validate_answer(bytes).is_err(),
            "{}",
            String::from_utf8_lossy(bytes)
        );
    }
    assert!(validate_answer(&vec![b' '; MAX_DOCUMENT_SIZE as usize + 1]).is_err());
    assert!(validate_answer(&[0xff, 0xfe, 1]).is_err());
}

#[test]
fn document_reads_preserve_bytes_and_reject_oversized_files() {
    let workspace = Workspace::new(&std::env::temp_dir()).unwrap();
    let path = workspace.0.join("answer.xml");
    fs::write(&path, ANSWER).unwrap();
    assert_eq!(crate::files::read_document(&path).unwrap(), ANSWER);
    File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(MAX_DOCUMENT_SIZE + 1)
        .unwrap();
    assert!(crate::files::read_document(&path).is_err());
}

#[test]
fn verification_detects_changed_setup_files_after_large_reads() {
    let workspace = Workspace::new(&std::env::temp_dir()).unwrap();
    let source = workspace.0.join("source");
    let output = workspace.0.join("output");
    for root in [&source, &output] {
        fs::create_dir_all(root.join("nested")).unwrap();
        fs::write(root.join("large.bin"), vec![1u8; 1024 * 1024 + 7]).unwrap();
        fs::write(root.join("nested/small.bin"), [2u8; 31]).unwrap();
    }
    let mut buffer = vec![0u8; 2 * 1024 * 1024];
    let cancelled = AtomicBool::new(false);
    verify_tree(&source, &output, &cancelled, true, &mut buffer).unwrap();
    fs::write(output.join("nested/small.bin"), [3u8; 31]).unwrap();
    assert!(verify_tree(&source, &output, &cancelled, true, &mut buffer).is_err());
}

fn boots() -> Vec<BootImage> {
    let mut efi = vec![0u8; 2880 * 512];
    efi[11..13].copy_from_slice(&512u16.to_le_bytes());
    efi[19..21].copy_from_slice(&2880u16.to_le_bytes());
    efi[510..512].copy_from_slice(&[0x55, 0xaa]);
    vec![
        BootImage {
            platform: 0,
            bytes: vec![0x42; 4096],
        },
        BootImage {
            platform: 0xef,
            bytes: efi,
        },
    ]
}

fn install_image(images: &[(u32, &str, &str)]) -> Vec<u8> {
    let mut xml = String::from("<WIM>");
    for (index, (build, name, product_type)) in images.iter().enumerate() {
        xml.push_str(&format!(
            "<IMAGE INDEX=\"{}\"><NAME>{name}</NAME><WINDOWS><PRODUCTTYPE>{product_type}</PRODUCTTYPE><VERSION><MAJOR>10</MAJOR><MINOR>0</MINOR><BUILD>{build}</BUILD></VERSION></WINDOWS></IMAGE>",
            index + 1
        ));
    }
    xml.push_str("</WIM>");
    let mut metadata = vec![0xff, 0xfe];
    for character in xml.encode_utf16() {
        metadata.extend_from_slice(&character.to_le_bytes());
    }
    let mut bytes = vec![0x45; 16_384];
    bytes[..208].fill(0);
    bytes[..8].copy_from_slice(b"MSWIM\0\0\0");
    bytes[8..12].copy_from_slice(&208u32.to_le_bytes());
    bytes[44..48].copy_from_slice(&(images.len() as u32).to_le_bytes());
    bytes[72..80].copy_from_slice(&(metadata.len() as u64).to_le_bytes());
    bytes[80..88].copy_from_slice(&208u64.to_le_bytes());
    bytes[88..96].copy_from_slice(&(metadata.len() as u64).to_le_bytes());
    bytes[208..208 + metadata.len()].copy_from_slice(&metadata);
    bytes
}

fn windows11_image() -> Vec<u8> {
    install_image(&[(26100, "Windows 11 Enterprise LTSC", "WinNT")])
}

#[test]
fn media_versions_reject_windows10_including_ltsc_and_mixed_images() {
    let workspace = Workspace::new(&std::env::temp_dir()).unwrap();
    let path = workspace.0.join("windows11.iso");
    for (build, name) in [
        (19045, "Windows 10 Pro"),
        (19044, "Windows 10 Enterprise LTSC"),
        (17763, "Windows 10 IoT Enterprise LTSC"),
        (19044, "Windows 11 Pro"),
        (21999, "Renamed image"),
    ] {
        fs::write(&path, install_image(&[(build, name, "WinNT")])).unwrap();
        assert!(
            media::validate_install_image(&path)
                .unwrap_err()
                .to_string()
                .contains("Windows 10")
        );
    }
    fs::write(
        &path,
        install_image(&[
            (26100, "Windows 11 Pro", "WinNT"),
            (19044, "Windows 10 Enterprise LTSC", "WinNT"),
        ]),
    )
    .unwrap();
    assert!(media::validate_install_image(&path).is_err());
    fs::write(
        &path,
        install_image(&[(26100, "Windows Server", "ServerNT")]),
    )
    .unwrap();
    assert!(media::validate_install_image(&path).is_err());
}

#[test]
fn media_versions_accept_windows11_and_ltsc_by_build_in_all_image_formats() {
    let workspace = Workspace::new(&std::env::temp_dir()).unwrap();
    fs::create_dir(workspace.0.join("sources")).unwrap();
    fs::write(workspace.0.join("setup.exe"), b"setup").unwrap();
    fs::write(workspace.0.join("sources/boot.wim"), b"boot").unwrap();
    for name in ["install.wim", "install.esd", "install.swm"] {
        let path = workspace.0.join("sources").join(name);
        for (build, edition) in [
            (22000, "Windows 10 Pro"),
            (26200, "Windows 11 Pro"),
            (26100, "Windows 11 Enterprise LTSC"),
            (26100, "Windows 11 Enterprise N LTSC"),
            (26100, "Windows 11 IoT Enterprise LTSC"),
        ] {
            fs::write(&path, install_image(&[(build, edition, "WinNT")])).unwrap();
            media::validate_source(&workspace.0).unwrap();
        }
        fs::remove_file(path).unwrap();
    }
    assert!(media::validate_source(&workspace.0).is_err());
}

#[test]
fn source_reports_actual_image_names() {
    let workspace = Workspace::new(&std::env::temp_dir()).unwrap();
    fs::create_dir(workspace.0.join("sources")).unwrap();
    fs::write(workspace.0.join("setup.exe"), b"setup").unwrap();
    fs::write(workspace.0.join("sources/boot.wim"), b"boot").unwrap();
    fs::write(workspace.0.join("sources/install.esd"), install_image(&[
        (26100, "Windows 11 Pro", "WinNT"),
        (26100, "Windows 11 Enterprise LTSC Evaluation", "WinNT"),
    ])).unwrap();
    assert_eq!(media::validate_source(&workspace.0).unwrap(), vec!["Windows 11 Enterprise LTSC Evaluation", "Windows 11 Pro"]);
}

#[test]
fn media_versions_reject_missing_damaged_and_oversized_metadata() {
    let workspace = Workspace::new(&std::env::temp_dir()).unwrap();
    let path = workspace.0.join("install.wim");
    for (range, replacement) in [
        (0..8, b"NOTAWIM!".to_vec()),
        (72..80, 0u64.to_le_bytes().to_vec()),
        (72..80, (32u64 * 1024 * 1024).to_le_bytes().to_vec()),
        (80..88, u64::MAX.to_le_bytes().to_vec()),
        (88..96, 0u64.to_le_bytes().to_vec()),
        (44..48, 2u32.to_le_bytes().to_vec()),
        (208..210, vec![0, 0]),
        (210..212, vec![0, 0xd8]),
    ] {
        let mut bytes = windows11_image();
        bytes[range].copy_from_slice(&replacement);
        fs::write(&path, bytes).unwrap();
        assert!(media::validate_install_image(&path).is_err());
    }
    for bytes in [Vec::new(), b"invalid".to_vec(), install_image(&[])] {
        fs::write(&path, bytes).unwrap();
        assert!(media::validate_install_image(&path).is_err());
    }
}

fn fixture(workspace: &Workspace, boot_images: &[BootImage]) -> PathBuf {
    let _com = Com::new().unwrap();
    let root = workspace.0.join("tree");
    fs::create_dir_all(root.join("sources")).unwrap();
    fs::create_dir_all(root.join("nested folder")).unwrap();
    fs::write(root.join("setup.exe"), b"setup payload").unwrap();
    fs::write(root.join("sources/boot.wim"), vec![0x31; 8192]).unwrap();
    fs::write(root.join("sources/install.wim"), windows11_image()).unwrap();
    fs::write(
        root.join("nested folder/payload.txt"),
        "unchanged payload \u{1f680}",
    )
    .unwrap();
    fs::write(root.join("AUTOUNATTEND.XML"), ANSWER).unwrap();
    let source = workspace.0.join("source.iso");
    image::build(
        &root,
        boot_images,
        ANSWER,
        &source,
        &AtomicBool::new(false),
        &mut |_, _| {},
    )
    .unwrap();
    source
}

#[test]
fn injection_preserves_payloads_and_boot_images_in_both_save_modes() {
    let _guard = IMAGE_TEST_LOCK.lock().unwrap();
    let workspace = Workspace::new(&std::env::temp_dir()).unwrap();
    let source = fixture(&workspace, &boots());
    let original = fs::read(&source).unwrap();
    let changed = br#"<unattend xmlns="urn:schemas-microsoft-com:unattend"><settings pass="specialize"/></unattend>"#;
    let copy = workspace.0.join("copy.iso");
    let cancelled = AtomicBool::new(false);
    inject(&source, changed, &copy, false, &cancelled, &mut |_, _| {}).unwrap();
    assert_eq!(fs::read(&source).unwrap(), original);
    assert_eq!(
        image::boot_images(&mut File::open(&copy).unwrap()).unwrap(),
        boots()
    );
    let _com = Com::new().unwrap();
    {
        let mounted = MountedIso::open(&copy).unwrap();
        assert_eq!(
            fs::read(mounted.root.join("autounattend.xml")).unwrap(),
            changed
        );
        assert_eq!(
            fs::read(mounted.root.join("nested folder/payload.txt")).unwrap(),
            "unchanged payload \u{1f680}".as_bytes()
        );
        assert_eq!(
            fs::read(mounted.root.join("sources/install.wim")).unwrap(),
            windows11_image()
        );
    }
    inject(&source, changed, &source, true, &cancelled, &mut |_, _| {}).unwrap();
    let mounted = MountedIso::open(&source).unwrap();
    assert_eq!(
        fs::read(mounted.root.join("autounattend.xml")).unwrap(),
        changed
    );
    assert_eq!(
        image::boot_images(&mut File::open(&source).unwrap()).unwrap(),
        boots()
    );
}

#[test]
fn cancellation_and_validation_errors_preserve_source_and_existing_output() {
    let _guard = IMAGE_TEST_LOCK.lock().unwrap();
    let workspace = Workspace::new(&std::env::temp_dir()).unwrap();
    let source = fixture(&workspace, &boots());
    let original = fs::read(&source).unwrap();
    let output = workspace.0.join("existing.iso");
    fs::write(&output, b"existing output").unwrap();
    let cancelled = AtomicBool::new(false);
    assert!(
        inject(
            &source,
            b"invalid",
            &source,
            true,
            &cancelled,
            &mut |_, _| {}
        )
        .is_err()
    );
    let result = inject(
        &source,
        ANSWER,
        &output,
        false,
        &cancelled,
        &mut |message, _| {
            if message == "Writing the ISO…" {
                cancelled.store(true, Ordering::Relaxed);
            }
        },
    );
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::Interrupted);
    assert_eq!(fs::read(&source).unwrap(), original);
    assert_eq!(fs::read(&output).unwrap(), b"existing output");
    assert!(!fs::read_dir(&workspace.0).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".orbital-inject-")
    }));
    cancelled.store(false, Ordering::Relaxed);
    let alias = workspace.0.join("alias.iso");
    fs::hard_link(&source, &alias).unwrap();
    assert!(inject(&source, ANSWER, &alias, false, &cancelled, &mut |_, _| {}).is_err());
    assert_eq!(fs::read(&source).unwrap(), original);
    let permissions = source.metadata().unwrap().permissions();
    let mut readonly = permissions.clone();
    readonly.set_readonly(true);
    fs::set_permissions(&source, readonly).unwrap();
    let result = inject(&source, ANSWER, &source, true, &cancelled, &mut |_, _| {});
    fs::set_permissions(&source, permissions).unwrap();
    assert!(result.unwrap_err().to_string().contains("read-only"));
    assert_eq!(fs::read(&source).unwrap(), original);
}

#[test]
fn damaged_boot_catalogs_are_rejected_without_writes() {
    let _guard = IMAGE_TEST_LOCK.lock().unwrap();
    let workspace = Workspace::new(&std::env::temp_dir()).unwrap();
    let source = fixture(&workspace, &boots());
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&source)
        .unwrap();
    let mut block = [0u8; 2048];
    for index in 16..64 {
        file.seek(SeekFrom::Start(index * 2048)).unwrap();
        file.read_exact(&mut block).unwrap();
        if block[0] == 0 && &block[1..6] == b"CD001" {
            let catalog = u32::from_le_bytes(block[71..75].try_into().unwrap()) as u64 * 2048;
            file.seek(SeekFrom::Start(catalog + 30)).unwrap();
            file.write_all(&[0, 0]).unwrap();
            break;
        }
    }
    assert!(image::boot_images(&mut file).is_err());
}

#[test]
fn efi_only_images_and_existing_mounts_are_preserved() {
    let _guard = IMAGE_TEST_LOCK.lock().unwrap();
    let workspace = Workspace::new(&std::env::temp_dir()).unwrap();
    let boot_images = boots().split_off(1);
    let source = fixture(&workspace, &boot_images);
    let original = fs::read(&source).unwrap();
    let mounted = MountedIso::open(&source).unwrap();
    let output = workspace.0.join("copy.iso");
    let cancelled = AtomicBool::new(false);
    assert!(inject(&source, ANSWER, &source, true, &cancelled, &mut |_, _| {}).is_err());
    inject(&source, ANSWER, &output, false, &cancelled, &mut |_, _| {}).unwrap();
    assert!(mounted.root.is_dir());
    assert_eq!(fs::read(&source).unwrap(), original);
    assert_eq!(
        image::boot_images(&mut File::open(output).unwrap()).unwrap(),
        boot_images
    );
}

#[test]
#[ignore = "Requires ORBITAL_TEST_ISO pointing to a Windows installation ISO and space for a copy"]
fn official_windows_iso_copy_is_verified() {
    let _guard = IMAGE_TEST_LOCK.lock().unwrap();
    let source = PathBuf::from(std::env::var_os("ORBITAL_TEST_ISO").expect("Set ORBITAL_TEST_ISO"));
    let workspace = Workspace::new(&std::env::current_dir().unwrap()).unwrap();
    let output = workspace.0.join("orbital-test.iso");
    inject(
        &source,
        ANSWER,
        &output,
        false,
        &AtomicBool::new(false),
        &mut |message, percent| eprintln!("{message} {percent:?}"),
    )
    .unwrap();
    assert!(output.metadata().unwrap().len() > 4 * 1024 * 1024 * 1024);
}
