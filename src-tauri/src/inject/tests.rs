use super::*;
use crate::files::MAX_DOCUMENT_SIZE;
use image::BootImage;
use std::io::{Seek, SeekFrom, Write};

const ANSWER: &[u8] = br#"<?xml version="1.0"?><unattend xmlns="urn:schemas-microsoft-com:unattend"><settings pass="oobeSystem"/></unattend>"#;
static IMAGE_TEST_LOCK: Mutex<()> = Mutex::new(());

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

fn fixture(workspace: &Workspace, boot_images: &[BootImage]) -> PathBuf {
    let _com = Com::new().unwrap();
    let root = workspace.0.join("tree");
    fs::create_dir_all(root.join("sources")).unwrap();
    fs::create_dir_all(root.join("nested folder")).unwrap();
    fs::write(root.join("setup.exe"), b"setup payload").unwrap();
    fs::write(root.join("sources/boot.wim"), vec![0x31; 8192]).unwrap();
    fs::write(root.join("sources/install.wim"), vec![0x45; 16_384]).unwrap();
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
            vec![0x45; 16_384]
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
