//! Windows' own signature check on a held file, and where Defender's scanner is (second review,
//! findings B2 and B3, 27 October 2026). Real files on this laptop; nothing is installed or run.
#![cfg(windows)]

use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use pctwin_helper_os::{SystemFolder, defender_scanner, system_folder, verify_file_trust};

const TRUST_E_NOSIGNATURE: i32 = 0x800B_0100_u32 as i32;
const TRUST_E_BAD_DIGEST: i32 = 0x8009_6010_u32 as i32;

fn system32(name: &str) -> PathBuf {
    system_folder(SystemFolder::Windows)
        .unwrap()
        .join("System32")
        .join(name)
}

/// Opened as the maker check holds a file: read only, others may only read, a link not followed.
fn held(path: &Path) -> std::fs::File {
    std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0x1)
        .custom_flags(0x0020_0000)
        .open(path)
        .unwrap()
}

/// A program Windows signs inside the file (not only through a catalog).
fn signed() -> PathBuf {
    system32("MpSigStub.exe")
}

#[test]
fn a_signed_program_is_trusted_and_its_signer_read() {
    let p = signed();
    let t = verify_file_trust(&held(&p), &p);
    assert_eq!(t.code, 0, "{:#x}", t.code);
    let der = t.signer.expect("the signer's certificate");
    // A certificate (a DER sequence), naming Microsoft.
    assert_eq!(der[0], 0x30);
    assert!(der.windows(9).any(|w| w == b"Microsoft"));
}

#[test]
fn a_program_signed_only_through_a_catalog_has_no_signature_inside() {
    let p = system32("notepad.exe");
    let t = verify_file_trust(&held(&p), &p);
    assert_eq!(t.code, TRUST_E_NOSIGNATURE, "{:#x}", t.code);
    assert_eq!(t.signer, None);
}

#[test]
fn a_changed_byte_is_a_bad_digest() {
    let dir = tempfile::tempdir().unwrap();
    let mut bytes = std::fs::read(signed()).unwrap();
    // A byte inside the first section, well past the headers.
    let at = bytes.len() / 3;
    bytes[at] ^= 0xFF;
    let p = dir.path().join("changed.exe");
    std::fs::write(&p, &bytes).unwrap();
    let t = verify_file_trust(&held(&p), &p);
    assert_eq!(t.code, TRUST_E_BAD_DIGEST, "{:#x}", t.code);
    assert_eq!(t.signer, None);
}

fn junction(link: &Path, target: &Path) {
    let _ = std::process::Command::new("cmd")
        .args(["/d", "/c", "rmdir"])
        .arg(link)
        .output();
    let o = std::process::Command::new("cmd")
        .args(["/d", "/c", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()
        .unwrap();
    assert!(o.status.success(), "{o:?}");
}

/// Second review, finding B2: the reviewer swapped a folder link between the file PCTwin held
/// and the one Windows read by path. Windows now reads the held file itself: with the link
/// pointing at the good copy while the changed copy is held, the answer is about the changed
/// copy, and the other way round.
#[test]
fn windows_checks_the_held_file_whatever_the_path_leads_to_now() {
    let dir = tempfile::tempdir().unwrap();
    let (good, evil, link) = (
        dir.path().join("good"),
        dir.path().join("evil"),
        dir.path().join("j"),
    );
    std::fs::create_dir_all(&good).unwrap();
    std::fs::create_dir_all(&evil).unwrap();
    let bytes = std::fs::read(signed()).unwrap();
    let mut changed = bytes.clone();
    let at = changed.len() / 3;
    changed[at] ^= 0xFF;
    std::fs::write(good.join("setup.exe"), &bytes).unwrap();
    std::fs::write(evil.join("setup.exe"), &changed).unwrap();
    let path = link.join("setup.exe");

    junction(&link, &evil);
    let changed_held = held(&path);
    junction(&link, &good);
    let t = verify_file_trust(&changed_held, &path);
    assert_eq!(t.code, TRUST_E_BAD_DIGEST, "{:#x}", t.code);
    drop(changed_held);

    let good_held = held(&path);
    junction(&link, &evil);
    let t = verify_file_trust(&good_held, &path);
    assert_eq!(t.code, 0, "{:#x}", t.code);
    drop(good_held);
    let _ = std::process::Command::new("cmd")
        .args(["/d", "/c", "rmdir"])
        .arg(&link)
        .output();
}

/// Second review, finding B3: Defender's scanner is found where Windows records it, never from
/// an environment value.
#[test]
fn defenders_scanner_is_where_windows_records_it() {
    let p = defender_scanner().expect("Microsoft Defender's scanner on this laptop");
    assert!(p.is_absolute() && p.is_file(), "{}", p.display());
    assert_eq!(p.file_name().unwrap(), "MpCmdRun.exe");
    let windows_dir = system_folder(SystemFolder::Windows).unwrap();
    let drive = windows_dir.components().next().unwrap();
    assert_eq!(p.components().next().unwrap(), drive);
}

/// Second review, finding B3: with `ProgramFiles` and `SystemRoot` pointed at a folder of the
/// person's own, Defender's scanner is still the system's. This test runs itself again with
/// those values changed and reads the answer the second run prints.
#[test]
fn defenders_scanner_ignores_the_environment() {
    if std::env::var_os("PCTWIN_TRUST_CHILD").is_some() {
        println!("SCANNER={}", defender_scanner().unwrap().display());
        return;
    }
    let decoy = tempfile::tempdir().unwrap();
    let fake = decoy.path().join("Windows Defender");
    std::fs::create_dir_all(&fake).unwrap();
    std::fs::write(fake.join("MpCmdRun.exe"), b"not Defender").unwrap();
    let out = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "defenders_scanner_ignores_the_environment",
            "--nocapture",
        ])
        .env("PCTWIN_TRUST_CHILD", "1")
        .env("ProgramFiles", decoy.path())
        .env("ProgramW6432", decoy.path())
        .env("SystemRoot", decoy.path())
        .env("windir", decoy.path())
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text
        .lines()
        .find_map(|l| l.strip_prefix("SCANNER="))
        .unwrap_or_else(|| panic!("{text}"));
    assert_eq!(Path::new(line), defender_scanner().unwrap());
    assert!(
        !line.starts_with(&*decoy.path().to_string_lossy()),
        "{line}"
    );
}
