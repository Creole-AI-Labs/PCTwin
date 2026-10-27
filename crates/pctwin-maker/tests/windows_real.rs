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

#[test]
fn a_changed_old_copy_names_no_maker() {
    let original = signed_programs().remove(0);
    let dir = tempfile::tempdir().unwrap();
    let copy = copy_into(&dir, &original);
    let mut bytes = std::fs::read(&copy).unwrap();
    let mid = bytes.len() / 2;
    bytes[mid] ^= 0x01;
    std::fs::write(&copy, &bytes).unwrap();
    let e = windows_maker_of(&copy).unwrap_err();
    assert!(e.contains("changed after it was signed"), "{e}");
}

/// The signing certificate and the signature bytes of a real signed program.
fn real_signature() -> (Vec<u8>, x509_cert::Certificate) {
    use object::read::pe::PeFile64;
    let p = signed_programs()
        .into_iter()
        .find(|p| {
            let d = std::fs::read(p).unwrap();
            PeFile64::parse(&d[..]).is_ok()
        })
        .expect("a 64-bit signed program");
    let data = std::fs::read(&p).unwrap();
    let pe = PeFile64::parse(&data[..]).unwrap();
    let entry = authenticode::AttributeCertificateIterator::new(&pe)
        .unwrap()
        .unwrap()
        .next()
        .unwrap()
        .unwrap();
    let bytes = entry.data.to_vec();
    let sig = authenticode::AuthenticodeSignature::from_bytes(&bytes).unwrap();
    let leaf = signer_cert(&bytes);
    let _ = sig;
    (bytes, leaf)
}

fn signed_data(bytes: &[u8]) -> cms::signed_data::SignedData {
    use der::Decode;
    let mut r = der::SliceReader::new(bytes).unwrap();
    let info = cms::content_info::ContentInfo::decode(&mut r).unwrap();
    info.content.decode_as().unwrap()
}

fn signer_cert(bytes: &[u8]) -> x509_cert::Certificate {
    let sd = signed_data(bytes);
    let cms::signed_data::SignerIdentifier::IssuerAndSerialNumber(id) =
        &sd.signer_infos.0.as_slice()[0].sid
    else {
        panic!("signer by key")
    };
    sd.certificates
        .unwrap()
        .0
        .iter()
        .find_map(|c| match c {
            cms::cert::CertificateChoices::Certificate(c)
                if c.tbs_certificate.issuer == id.issuer
                    && c.tbs_certificate.serial_number == id.serial_number =>
            {
                Some(c.clone())
            }
            _ => None,
        })
        .unwrap()
}

/// The signature with its certificate list changed.
fn with_certificates(
    bytes: &[u8],
    change: impl FnOnce(&mut Vec<cms::cert::CertificateChoices>),
) -> Vec<u8> {
    use der::Encode;
    let mut sd = signed_data(bytes);
    let mut certs: Vec<_> = sd.certificates.take().unwrap().0.into_vec();
    change(&mut certs);
    sd.certificates = Some(cms::signed_data::CertificateSet(
        der::asn1::SetOfVec::try_from(certs).unwrap(),
    ));
    let info = cms::content_info::ContentInfo {
        content_type: const_oid::db::rfc5911::ID_SIGNED_DATA,
        content: der::Any::encode_from(&sd).unwrap(),
    };
    info.to_der().unwrap()
}

#[test]
fn the_signer_is_the_one_certificate_with_its_issuer_and_serial() {
    use pctwin_maker::signer::read_signature;
    let (bytes, leaf) = real_signature();
    let ours = read_signature(&bytes).unwrap().signer.unwrap();
    let theirs = pctwin_maker::signer::signer_from_certificate(&leaf).unwrap();
    assert_eq!(ours, theirs);

    // Another certificate from the same issuer, with another serial, is not the signer.
    let other_serial = with_certificates(&bytes, |certs| {
        let mut twin = leaf.clone();
        twin.tbs_certificate.serial_number =
            x509_cert::serial_number::SerialNumber::new(&[0x42]).unwrap();
        certs.push(cms::cert::CertificateChoices::Certificate(twin));
    });
    assert_eq!(read_signature(&other_serial).unwrap().signer.unwrap(), ours);

    // Two certificates both naming the signer: refused.
    let twice = with_certificates(&bytes, |certs| {
        let mut twin = leaf.clone();
        twin.signature = der::asn1::BitString::from_bytes(&[1, 2, 3]).unwrap();
        certs.push(cms::cert::CertificateChoices::Certificate(twin));
    });
    let e = read_signature(&twice).unwrap().signer.unwrap_err();
    assert!(e.contains("two certificates"), "{e}");

    // The signer's certificate missing.
    let missing = with_certificates(&bytes, |certs| {
        certs.retain(|c| !matches!(c, cms::cert::CertificateChoices::Certificate(c) if c == &leaf));
    });
    let e = read_signature(&missing).unwrap().signer.unwrap_err();
    assert!(e.contains("missing"), "{e}");

    // A signed digest of the wrong length is not a signature.
    let mut wrong = bytes.clone();
    let digest = authenticode::AuthenticodeSignature::from_bytes(&bytes)
        .unwrap()
        .digest()
        .to_vec();
    let at = wrong
        .windows(digest.len())
        .position(|w| w == &digest[..])
        .unwrap();
    // Shorten the OCTET STRING's length by one and drop a byte: the length is then wrong.
    if wrong[at - 1] as usize == digest.len() {
        wrong[at - 1] -= 1;
        wrong.remove(at);
        assert!(read_signature(&wrong).is_err());
    }
}

/// The certificate with its subject's names replaced.
fn with_subject(leaf: &x509_cert::Certificate, rdns: Vec<(&str, &str)>) -> x509_cert::Certificate {
    use std::str::FromStr;
    let text: Vec<String> = rdns.iter().map(|(k, v)| format!("{k}={v}")).collect();
    let mut c = leaf.clone();
    c.tbs_certificate.subject = x509_cert::name::Name::from_str(&text.join(",")).unwrap();
    c
}

#[test]
fn subject_names_are_read_whatever_their_order() {
    use pctwin_maker::signer::signer_from_certificate;
    let (_, leaf) = real_signature();
    // A part PCTwin does not read (a department) comes first.
    let c = with_subject(
        &leaf,
        vec![
            ("OU", "Tools"),
            ("CN", "Example Ltd"),
            ("O", "Example Ltd"),
            ("C", "NG"),
        ],
    );
    let s = signer_from_certificate(&c).unwrap().subject;
    assert_eq!(s.organization.as_deref(), Some("Example Ltd"));
    assert_eq!(s.common_name.as_deref(), Some("Example Ltd"));
    assert_eq!(s.country.as_deref(), Some("NG"));
    // Two names in one part (a department and the common name together).
    let c = with_subject(
        &leaf,
        vec![("OU", "Tools+CN=Example Ltd"), ("O", "Example Ltd")],
    );
    let s = signer_from_certificate(&c).unwrap().subject;
    assert_eq!(s.common_name.as_deref(), Some("Example Ltd"));
    // The same name twice is fine; two different ones are refused.
    let c = with_subject(
        &leaf,
        vec![
            ("CN", "Example Ltd"),
            ("CN", "Example Ltd"),
            ("O", "Example Ltd"),
        ],
    );
    assert!(signer_from_certificate(&c).is_ok());
    let c = with_subject(
        &leaf,
        vec![
            ("CN", "Example Ltd"),
            ("CN", "Other Ltd"),
            ("O", "Example Ltd"),
        ],
    );
    let e = signer_from_certificate(&c).unwrap_err();
    assert!(e.contains("twice"), "{e}");
}
