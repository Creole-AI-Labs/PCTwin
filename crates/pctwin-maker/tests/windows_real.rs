//! The maker check on this Windows laptop, for real: Windows' own signed programs and installers,
//! Windows' signature check (PowerShell's Get-AuthenticodeSignature) and Microsoft Defender.
//! Where a file or Defender is missing (a stripped-down machine), the test says so and checks
//! what it can.

#![cfg(windows)]

use std::path::{Path, PathBuf};

use pctwin_maker::facts::*;
use pctwin_maker::pe::{PeSignature, read_pe};
use pctwin_maker::run::SystemRunner;
use pctwin_maker::windows::{DefenderState, parse_status, status_call};
use pctwin_maker::{Expected, Limits, NativeChecks, SystemChecks, check, windows_maker_of};

fn system32(name: &str) -> PathBuf {
    let root = std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into());
    Path::new(&root).join("System32").join(name)
}

/// Programs with the maker's signature inside the file, as a download has.
fn signed_programs() -> Vec<PathBuf> {
    let pf = std::env::var_os("ProgramFiles").unwrap_or_else(|| r"C:\Program Files".into());
    let candidates = [
        system32("MpSigStub.exe"),
        Path::new(&pf).join(r"Windows Defender\MpCmdRun.exe"),
        Path::new(&pf).join(r"Git\git-bash.exe"),
        system32("xgamehelper.exe"),
    ];
    let found: Vec<PathBuf> = candidates
        .into_iter()
        .filter(|p| windows_maker_of(p).is_ok())
        .collect();
    assert!(
        !found.is_empty(),
        "no program with an embedded signature found"
    );
    found
}

fn laptop_arch() -> Arch {
    match std::env::consts::ARCH {
        "aarch64" => Arch::Arm64,
        "x86" => Arch::X86,
        _ => Arch::X64,
    }
}

fn defender_ready() -> bool {
    use pctwin_maker::run::Runner;
    let state = parse_status(&SystemRunner.run(&status_call(std::time::Duration::from_secs(60))));
    eprintln!("Defender: {state:?}");
    matches!(state, DefenderState::Ready { .. })
}

#[test]
fn reads_the_maker_of_real_signed_programs_as_windows_does() {
    let native = NativeChecks::default();
    let mut compared = 0;
    for p in signed_programs() {
        let ours = windows_maker_of(&p).unwrap();
        eprintln!("{}: {:?}", p.display(), ours.subject);
        assert!(ours.subject.organization.is_some(), "{}", p.display());
        let sys = native.signature(&p, Format::WindowsProgram, &Limits::default());
        assert_eq!(sys.signature, Signature::Trusted, "{}", p.display());
        if !sys.embedded {
            // Also in a Windows catalog: Windows reports the catalog's signer, not the file's.
            eprintln!("{}: Windows checked it through a catalog", p.display());
            let v = check(&p, &expected_for(&p), &native, &Limits::default()).unwrap();
            assert!(
                matches!(&v.signature, Signature::NotTrusted { why } if why.contains("catalog"))
            );
            continue;
        }
        compared += 1;
        let Some(Ok(Maker::Windows(theirs))) = sys.maker else {
            panic!("Windows named no signer for {}", p.display())
        };
        // Both readings name the very same certificate.
        assert_eq!(ours, theirs, "{}", p.display());
    }
    assert!(
        compared > 0,
        "no program Windows checks by its own signature"
    );
    let stub = system32("MpSigStub.exe");
    if let Ok(s) = windows_maker_of(&stub) {
        assert_eq!(
            s.subject.organization.as_deref(),
            Some("Microsoft Corporation")
        );
        assert_eq!(s.subject.country.as_deref(), Some("US"));
    }
}

#[test]
fn the_streamed_digest_is_the_authenticode_crates_digest() {
    use object::read::pe::{PeFile32, PeFile64};
    use sha2::Digest;
    for p in signed_programs() {
        let data = std::fs::read(&p).unwrap();
        let mut theirs = sha2::Sha256::new();
        let parsed64 = PeFile64::parse(&data[..]);
        let parsed32 = PeFile32::parse(&data[..]);
        match (&parsed64, &parsed32) {
            (Ok(pe), _) => authenticode::authenticode_digest(pe, &mut theirs).unwrap(),
            (_, Ok(pe)) => authenticode::authenticode_digest(pe, &mut theirs).unwrap(),
            _ => panic!("not a PE: {}", p.display()),
        }
        let theirs = theirs.finalize().to_vec();
        // The crate's digest is the digest the maker signed (these are SHA-256 signatures)...
        let table = match (&parsed64, &parsed32) {
            (Ok(pe), _) => authenticode::AttributeCertificateIterator::new(pe),
            (_, Ok(pe)) => authenticode::AttributeCertificateIterator::new(pe),
            _ => unreachable!(),
        }
        .unwrap()
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .get_authenticode_signature()
        .unwrap();
        assert_eq!(table.digest(), &theirs[..], "{}", p.display());
        // ...and PCTwin's streamed reading agrees.
        let mut f = std::fs::File::open(&p).unwrap();
        let ours = read_pe(&mut f, data.len() as u64).unwrap();
        assert!(
            matches!(
                ours,
                PeSignature::Signed {
                    digest_matches: true,
                    ..
                }
            ),
            "{}: {ours:?}",
            p.display()
        );
    }
}

fn copy_into(dir: &tempfile::TempDir, from: &Path) -> PathBuf {
    let to = dir.path().join("Down loads").join("setup copy.exe");
    std::fs::create_dir_all(to.parent().unwrap()).unwrap();
    std::fs::copy(from, &to).unwrap();
    to
}

fn expected_for(original: &Path) -> Expected {
    Expected {
        system: System::Windows,
        maker: Maker::Windows(windows_maker_of(original).unwrap()),
        formats: vec![Format::WindowsProgram, Format::WindowsInstaller],
        arch: laptop_arch(),
        bytes: None,
    }
}

#[test]
fn a_real_signed_program_from_the_same_maker_is_approved() {
    let original = signed_programs().remove(0);
    let dir = tempfile::tempdir().unwrap();
    let copy = copy_into(&dir, &original);
    let e = expected_for(&original);
    let v = check(&copy, &e, &NativeChecks::default(), &Limits::default()).unwrap();
    eprintln!("{v:#?}");
    assert_eq!(v.signature, Signature::Trusted);
    assert_eq!(
        v.same_maker,
        SameMaker::Yes {
            by: MatchedBy::SameCertificate
        }
    );
    assert_eq!(v.steady, Steady::Yes);
    if defender_ready() {
        assert!(
            matches!(v.malware, Malware::NoDetection { .. }),
            "{:?}",
            v.malware
        );
        if v.arch == ArchFact::Matches {
            assert_eq!(v.decide(), Decision::Install { bytes: v.bytes });
        }
    } else {
        assert!(
            matches!(v.decide(), Decision::NeedsAction { why } if why.contains(&Why::NotFullyChecked))
        );
    }

    // The same file against another maker (T21-030).
    let mut other = e.clone();
    let Maker::Windows(mut s) = other.maker.clone() else {
        unreachable!()
    };
    s.certificate = Sha256([0; 32]);
    s.subject.organization = Some("Another Company".into());
    other.maker = Maker::Windows(s);
    let v = check(&copy, &other, &NativeChecks::default(), &Limits::default()).unwrap();
    assert!(matches!(v.same_maker, SameMaker::No { .. }));
    assert!(matches!(v.decide(), Decision::NeedsAction { why } if why.contains(&Why::OtherMaker)));
}

#[test]
fn one_changed_byte_breaks_the_signature_and_the_checksum_is_left_out() {
    let original = signed_programs().remove(0);
    let dir = tempfile::tempdir().unwrap();
    let copy = copy_into(&dir, &original);
    let mut bytes = std::fs::read(&copy).unwrap();

    // The header checksum is not part of what is signed: changing it changes nothing.
    let pe = u32::from_le_bytes(bytes[0x3C..0x40].try_into().unwrap()) as usize;
    let check_sum = pe + 24 + 64;
    bytes[check_sum] ^= 0xFF;
    std::fs::write(&copy, &bytes).unwrap();
    let mut f = std::fs::File::open(&copy).unwrap();
    assert!(matches!(
        read_pe(&mut f, bytes.len() as u64).unwrap(),
        PeSignature::Signed {
            digest_matches: true,
            ..
        }
    ));
    drop(f);
    let sys = NativeChecks::default().signature(&copy, Format::WindowsProgram, &Limits::default());
    assert_eq!(
        sys.signature,
        Signature::Trusted,
        "Windows agrees the checksum is not signed"
    );

    // One byte in the middle of the program is.
    let mid = bytes.len() / 2;
    bytes[mid] ^= 0x01;
    std::fs::write(&copy, &bytes).unwrap();
    let mut f = std::fs::File::open(&copy).unwrap();
    assert!(matches!(
        read_pe(&mut f, bytes.len() as u64).unwrap(),
        PeSignature::Signed {
            digest_matches: false,
            ..
        }
    ));
    drop(f);
    let sys = NativeChecks::default().signature(&copy, Format::WindowsProgram, &Limits::default());
    assert!(
        matches!(sys.signature, Signature::Broken { .. }),
        "{:?}",
        sys.signature
    );
    let v = check(
        &copy,
        &expected_for(&original),
        &NativeChecks::default(),
        &Limits::default(),
    )
    .unwrap();
    eprintln!("{:?}", v.signature);
    assert!(matches!(v.signature, Signature::Broken { .. }));
    assert!(
        matches!(v.decide(), Decision::NeedsAction { why } if why.contains(&Why::SignatureBroken))
    );
}

#[test]
fn a_program_signed_only_through_a_windows_catalog_is_not_approved() {
    let notepad = system32("notepad.exe");
    if !notepad.exists() {
        eprintln!("no notepad.exe here");
        return;
    }
    let mut f = std::fs::File::open(&notepad).unwrap();
    let size = f.metadata().unwrap().len();
    assert_eq!(read_pe(&mut f, size).unwrap(), PeSignature::None);
    let e = expected_for(&signed_programs()[0]);
    let v = check(&notepad, &e, &NativeChecks::default(), &Limits::default()).unwrap();
    eprintln!("{:?}", v.signature);
    assert!(matches!(&v.signature, Signature::NotTrusted { why } if why.contains("catalog")));
    assert!(matches!(v.decide(), Decision::NeedsAction { .. }));
}

#[test]
fn real_installer_packages_name_the_signer_windows_names() {
    let Ok(entries) = std::fs::read_dir(r"C:\Windows\Installer") else {
        eprintln!("no installer cache readable here");
        return;
    };
    let native = NativeChecks::default();
    let mut seen = 0;
    for entry in entries.flatten().take(400) {
        let p = entry.path();
        if p.extension().is_none_or(|x| !x.eq_ignore_ascii_case("msi")) {
            continue;
        }
        let Ok(f) = std::fs::File::open(&p) else {
            continue;
        };
        let Ok(read) = pctwin_maker::msi::read_msi(f) else {
            continue;
        };
        assert!(read.is_package, "{}", p.display());
        let pctwin_maker::msi::MsiSignature::Signed(Ok(ours)) = read.signature else {
            continue;
        };
        let sys = native.signature(&p, Format::WindowsInstaller, &Limits::default());
        let Some(Ok(Maker::Windows(theirs))) = sys.maker else {
            panic!("Windows named no signer for {}", p.display())
        };
        assert_eq!(ours, theirs, "{}", p.display());
        eprintln!(
            "{}: {:?} {:?} {:?}",
            p.display(),
            ours.subject.organization,
            read.arch,
            sys.signature
        );
        assert!(read.arch.is_some(), "{}", p.display());
        seen += 1;
        if seen == 3 {
            break;
        }
    }
    eprintln!("signed installer packages checked: {seen}");
}

/// The EICAR test file, kept reversed so this source is never itself flagged.
fn eicar() -> String {
    "*H+H$!ELIF-TSET-SURIVITNA-DRADNATS-RACIE$}7)CC7)^P(45XZP\\4[PA@%P!O5X"
        .chars()
        .rev()
        .collect()
}

#[test]
fn defender_meanings_pinned_with_the_eicar_test_file() {
    if !defender_ready() {
        eprintln!("Microsoft Defender is not available here: not pinned on this machine");
        return;
    }
    let native = NativeChecks::default();
    let dir = tempfile::tempdir().unwrap();
    let limits = Limits::default();
    let clean = dir.path().join("a clean file.txt");
    std::fs::write(&clean, "hello").unwrap();
    let m = native.malware(&clean, Format::WindowsProgram, &Signature::Missing, &limits);
    eprintln!("clean: {m:?}");
    assert!(matches!(m, Malware::NoDetection { coverage } if coverage.later_downloads_not_checked));

    let missing = dir.path().join("missing.exe");
    let m = native.malware(
        &missing,
        Format::WindowsProgram,
        &Signature::Missing,
        &limits,
    );
    eprintln!("missing: {m:?}");
    assert!(matches!(m, Malware::NotFullyChecked { .. }));

    // T21-031: a known detection. Real-time protection may remove the file at once; then the scan
    // has nothing to find and the answer must still not be clean.
    let test = dir.path().join("eicar test.com");
    std::fs::write(&test, eicar()).unwrap();
    let m = native.malware(&test, Format::WindowsProgram, &Signature::Missing, &limits);
    eprintln!("EICAR: {m:?} (file still there: {})", test.exists());
    if test.exists() {
        assert!(
            matches!(&m, Malware::Detected { name } if name.contains("EICAR")),
            "{m:?}"
        );
    } else {
        assert!(!matches!(m, Malware::NoDetection { .. }));
    }
    let _ = std::fs::remove_file(&test);
}
