//! The maker check on this Windows laptop, for real: Windows' own signed programs and installers,
//! Windows' own signature check (WinVerifyTrust on the held file) and Microsoft Defender.
//! Where a file or Defender is missing (a stripped-down machine), the test says so and checks
//! what it can.

#![cfg(windows)]

mod common;

use common::os_checks;

use std::path::{Path, PathBuf};

use pctwin_maker::facts::*;
use pctwin_maker::own::raw::PeSignature;
use pctwin_maker::windows::{DefenderState, defender_status, judge_status};
use pctwin_maker::{Expected, Limits, NativeChecks, SystemChecks, check};

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
        .filter(|p| pctwin_maker::windows_maker_of(p, &own()).is_ok())
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
    let state = judge_status(defender_status());
    eprintln!("Defender: {state:?}");
    matches!(state, DefenderState::Ready { .. })
}

#[test]
fn reads_the_maker_of_real_signed_programs_as_windows_does() {
    let native = os_checks();
    let mut compared = 0;
    for p in signed_programs() {
        let ours = pctwin_maker::windows_maker_of(&p, &own()).unwrap();
        eprintln!("{}: {:?}", p.display(), ours.subject);
        assert!(ours.subject.organization.is_some(), "{}", p.display());
        let sys = native.signature(
            &std::fs::File::open(&p).unwrap(),
            &p,
            Format::WindowsProgram,
            &Limits::default(),
        );
        assert_eq!(sys.signature, Signature::Trusted, "{}", p.display());
        // Windows is asked about the signature inside the file only, never a catalog.
        assert!(sys.embedded, "{}", p.display());
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
    if let Ok(s) = pctwin_maker::windows_maker_of(&stub, &own()) {
        assert_eq!(
            s.subject.organization.as_deref(),
            Some("Microsoft Corporation")
        );
        assert_eq!(s.subject.country.as_deref(), Some("US"));
        // Second review, B4: the authority family and the key are read, to tie renewals.
        assert_eq!(
            s.issuer_organization.as_deref(),
            Some("Microsoft Corporation")
        );
        assert!(s.public_key.is_some());
        assert_ne!(s.public_key, Some(s.certificate));
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
        let ours = pctwin_maker::own::raw::pe_signature(
            pctwin_parse_seal::in_child(),
            &mut f,
            data.len() as u64,
        )
        .unwrap();
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
        maker: Maker::Windows(pctwin_maker::windows_maker_of(original, &own()).unwrap()),
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
    let v = check(&copy, &e, &own(), &os_checks(), &Limits::default()).unwrap();
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
    let v = check(&copy, &other, &own(), &os_checks(), &Limits::default()).unwrap();
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
        pctwin_maker::own::raw::pe_signature(
            pctwin_parse_seal::in_child(),
            &mut f,
            bytes.len() as u64
        )
        .unwrap(),
        PeSignature::Signed {
            digest_matches: true,
            ..
        }
    ));
    drop(f);
    let sys = os_checks().signature(
        &std::fs::File::open(&copy).unwrap(),
        &copy,
        Format::WindowsProgram,
        &Limits::default(),
    );
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
        pctwin_maker::own::raw::pe_signature(
            pctwin_parse_seal::in_child(),
            &mut f,
            bytes.len() as u64
        )
        .unwrap(),
        PeSignature::Signed {
            digest_matches: false,
            ..
        }
    ));
    drop(f);
    let sys = os_checks().signature(
        &std::fs::File::open(&copy).unwrap(),
        &copy,
        Format::WindowsProgram,
        &Limits::default(),
    );
    assert!(
        matches!(sys.signature, Signature::Broken { .. }),
        "{:?}",
        sys.signature
    );
    let v = check(
        &copy,
        &expected_for(&original),
        &own(),
        &os_checks(),
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
    assert_eq!(
        pctwin_maker::own::raw::pe_signature(pctwin_parse_seal::in_child(), &mut f, size).unwrap(),
        PeSignature::None
    );
    let e = expected_for(&signed_programs()[0]);
    let v = check(&notepad, &e, &own(), &os_checks(), &Limits::default()).unwrap();
    eprintln!("{:?}", v.signature);
    // Windows is asked about the file alone: a catalog signature is no signature inside it.
    assert_eq!(v.signature, Signature::Missing);
    assert!(matches!(v.decide(), Decision::NeedsAction { .. }));
}

#[test]
fn real_installer_packages_name_the_signer_windows_names() {
    let Ok(entries) = std::fs::read_dir(r"C:\Windows\Installer") else {
        eprintln!("no installer cache readable here");
        return;
    };
    let native = os_checks();
    let mut seen = 0;
    for entry in entries.flatten().take(400) {
        let p = entry.path();
        if p.extension().is_none_or(|x| !x.eq_ignore_ascii_case("msi")) {
            continue;
        }
        let Ok(f) = std::fs::File::open(&p) else {
            continue;
        };
        let Ok(read) = pctwin_maker::own::raw::msi(pctwin_parse_seal::in_child(), f) else {
            continue;
        };
        assert!(read.is_package, "{}", p.display());
        let pctwin_maker::own::raw::MsiSignature::Signed(Ok(ours)) = read.signature else {
            continue;
        };
        let sys = native.signature(
            &std::fs::File::open(&p).unwrap(),
            &p,
            Format::WindowsInstaller,
            &Limits::default(),
        );
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
    let native = os_checks();
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
    let e = pctwin_maker::windows_maker_of(&copy, &own()).unwrap_err();
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
    let (bytes, leaf) = real_signature();
    let ours = pctwin_maker::own::raw::signature(pctwin_parse_seal::in_child(), &bytes)
        .unwrap()
        .signer
        .unwrap();
    let theirs =
        pctwin_maker::own::raw::signer_of_certificate(pctwin_parse_seal::in_child(), &leaf)
            .unwrap();
    assert_eq!(ours, theirs);

    // Another certificate from the same issuer, with another serial, is not the signer.
    let other_serial = with_certificates(&bytes, |certs| {
        let mut twin = leaf.clone();
        twin.tbs_certificate.serial_number =
            x509_cert::serial_number::SerialNumber::new(&[0x42]).unwrap();
        certs.push(cms::cert::CertificateChoices::Certificate(twin));
    });
    assert_eq!(
        pctwin_maker::own::raw::signature(pctwin_parse_seal::in_child(), &other_serial)
            .unwrap()
            .signer
            .unwrap(),
        ours
    );

    // Two certificates both naming the signer: refused.
    let twice = with_certificates(&bytes, |certs| {
        let mut twin = leaf.clone();
        twin.signature = der::asn1::BitString::from_bytes(&[1, 2, 3]).unwrap();
        certs.push(cms::cert::CertificateChoices::Certificate(twin));
    });
    let e = pctwin_maker::own::raw::signature(pctwin_parse_seal::in_child(), &twice)
        .unwrap()
        .signer
        .unwrap_err();
    assert!(e.contains("two certificates"), "{e}");

    // The signer's certificate missing.
    let missing = with_certificates(&bytes, |certs| {
        certs.retain(|c| !matches!(c, cms::cert::CertificateChoices::Certificate(c) if c == &leaf));
    });
    let e = pctwin_maker::own::raw::signature(pctwin_parse_seal::in_child(), &missing)
        .unwrap()
        .signer
        .unwrap_err();
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
        assert!(pctwin_maker::own::raw::signature(pctwin_parse_seal::in_child(), &wrong).is_err());
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
    let s = pctwin_maker::own::raw::signer_of_certificate(pctwin_parse_seal::in_child(), &c)
        .unwrap()
        .subject;
    assert_eq!(s.organization.as_deref(), Some("Example Ltd"));
    assert_eq!(s.common_name.as_deref(), Some("Example Ltd"));
    assert_eq!(s.country.as_deref(), Some("NG"));
    // Two names in one part (a department and the common name together).
    let c = with_subject(
        &leaf,
        vec![("OU", "Tools+CN=Example Ltd"), ("O", "Example Ltd")],
    );
    let s = pctwin_maker::own::raw::signer_of_certificate(pctwin_parse_seal::in_child(), &c)
        .unwrap()
        .subject;
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
    assert!(
        pctwin_maker::own::raw::signer_of_certificate(pctwin_parse_seal::in_child(), &c).is_ok()
    );
    let c = with_subject(
        &leaf,
        vec![
            ("CN", "Example Ltd"),
            ("CN", "Other Ltd"),
            ("O", "Example Ltd"),
        ],
    );
    let e = pctwin_maker::own::raw::signer_of_certificate(pctwin_parse_seal::in_child(), &c)
        .unwrap_err();
    assert!(e.contains("twice"), "{e}");
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

/// Swaps a folder link to the good copy around each of Windows' own checks, as the reviewer did.
struct Swapping {
    inner: NativeChecks,
    link: PathBuf,
    good: PathBuf,
    evil: PathBuf,
}

impl SystemChecks for Swapping {
    fn signature(
        &self,
        file: &std::fs::File,
        p: &Path,
        f: Format,
        l: &Limits,
    ) -> pctwin_maker::SystemSignature {
        junction(&self.link, &self.good);
        let r = self.inner.signature(file, p, f, l);
        junction(&self.link, &self.evil);
        r
    }
    fn malware(&self, p: &Path, f: Format, s: &Signature, l: &Limits) -> Malware {
        junction(&self.link, &self.good);
        let r = self.inner.malware(p, f, s, l);
        junction(&self.link, &self.evil);
        r
    }
}

/// Second review, finding B2 (the reviewer's proof): a program with one code byte changed and its
/// signed digest rewritten to match, behind a folder link that is pointed at the real program
/// whenever Windows is asked. Windows is now asked about the held file itself, so the changed
/// program is never approved and its bytes never pass as approved.
#[test]
fn a_folder_link_swapped_during_the_check_never_approves_other_bytes() {
    use object::read::pe::PeFile64;
    use sha2::{Digest, Sha256 as H};
    let legit = signed_programs().remove(0);
    let bytes = std::fs::read(&legit).unwrap();
    let Ok(pe) = PeFile64::parse(&bytes[..]) else {
        eprintln!("not a 64-bit program here");
        return;
    };
    let mut h = H::new();
    authenticode::authenticode_digest(&pe, &mut h).unwrap();
    let old_digest = h.finalize().to_vec();
    let mut evil = bytes.clone();
    let first = pe.section_table().iter().next().unwrap();
    let at = first.pointer_to_raw_data.get(object::LittleEndian) as usize + 0x40;
    evil[at] ^= 0xFF;
    let pe2 = PeFile64::parse(&evil[..]).unwrap();
    let mut h2 = H::new();
    authenticode::authenticode_digest(&pe2, &mut h2).unwrap();
    let new_digest = h2.finalize().to_vec();
    let Some(pos) = evil.windows(32).position(|w| w == old_digest.as_slice()) else {
        eprintln!("the signed digest is not stored plainly here");
        return;
    };
    evil[pos..pos + 32].copy_from_slice(&new_digest);

    let dir = tempfile::tempdir().unwrap();
    let (good, bad, link) = (
        dir.path().join("good"),
        dir.path().join("evil"),
        dir.path().join("j"),
    );
    std::fs::create_dir_all(&good).unwrap();
    std::fs::create_dir_all(&bad).unwrap();
    std::fs::write(good.join("setup.exe"), &bytes).unwrap();
    std::fs::write(bad.join("setup.exe"), &evil).unwrap();
    junction(&link, &bad);
    let path = link.join("setup.exe");
    let exp = expected_for(&legit);
    let sw = Swapping {
        inner: os_checks(),
        link: link.clone(),
        good,
        evil: bad,
    };
    let v = check(&path, &exp, &own(), &sw, &Limits::default()).unwrap();
    eprintln!("{:?} {:?}", v.signature, v.decide());
    assert_ne!(v.signature, Signature::Trusted);
    assert!(matches!(v.decide(), Decision::NeedsAction { .. }));
    let found = Bytes {
        size: evil.len() as u64,
        sha256: Sha256(H::digest(&evil).into()),
    };
    assert!(!approved_bytes_match(&v.decide(), &found));
    let _ = std::process::Command::new("cmd")
        .args(["/d", "/c", "rmdir"])
        .arg(&link)
        .output();
}

/// PCTwin's own readers, run in the test's own process (only a parser crate's tests may).
fn own() -> pctwin_maker::InThisProcess {
    pctwin_maker::InThisProcess::new(pctwin_parse_seal::in_child())
}

/// An installer found among the person's files that is stored online only (a OneDrive folder) is
/// never opened for reading: reading it would download it. Marked here with Windows' own
/// `attrib +O` (offline), the mark cloud apps set (29 October 2026).
#[test]
fn an_installer_stored_online_only_is_not_read() {
    let dir = tempfile::tempdir().unwrap();
    let copy = copy_into(&dir, &system32("notepad.exe"));
    let attrib = |flag: &str| {
        let ok = pctwin_types::process::quiet("attrib")
            .arg(flag)
            .arg(&copy)
            .status()
            .unwrap()
            .success();
        assert!(ok, "attrib {flag}");
    };
    attrib("+O");
    let read = pctwin_maker::windows_old_copy(&copy, &own());
    attrib("-O");
    match read.maker {
        pctwin_maker::OldMaker::NotRead(why) => assert!(why.contains("online only"), "{why}"),
        other => panic!("an online-only file was read: {other:?}"),
    }
    // The same file, back on the laptop, is read as before.
    assert!(!matches!(
        pctwin_maker::windows_old_copy(&copy, &own()).maker,
        pctwin_maker::OldMaker::NotRead(_)
    ));
}
