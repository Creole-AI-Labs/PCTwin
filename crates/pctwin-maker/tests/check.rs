//! The whole check on real files, with the system's own checks stood in for, so it runs the same
//! on every system.

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};

use pctwin_maker::facts::*;
use pctwin_maker::{CheckError, Expected, Limits, SystemChecks, SystemSignature, check};
use sha2::{Digest, Sha256 as H};

/// A minimal 64-bit Windows program with no signature.
fn tiny_pe() -> Vec<u8> {
    let mut b = vec![0u8; 0x400];
    b[0] = b'M';
    b[1] = b'Z';
    b[0x3C..0x40].copy_from_slice(&0x80u32.to_le_bytes());
    b[0x80..0x84].copy_from_slice(b"PE\0\0");
    let coff = 0x84;
    b[coff..coff + 2].copy_from_slice(&0x8664u16.to_le_bytes());
    b[coff + 2..coff + 4].copy_from_slice(&1u16.to_le_bytes());
    b[coff + 16..coff + 18].copy_from_slice(&240u16.to_le_bytes());
    let opt = coff + 20;
    b[opt..opt + 2].copy_from_slice(&0x20Bu16.to_le_bytes());
    b[opt + 60..opt + 64].copy_from_slice(&0x200u32.to_le_bytes());
    b[opt + 108..opt + 112].copy_from_slice(&16u32.to_le_bytes());
    let sec = opt + 240;
    b[sec + 16..sec + 20].copy_from_slice(&0x200u32.to_le_bytes());
    b[sec + 20..sec + 24].copy_from_slice(&0x200u32.to_le_bytes());
    b
}

fn maker() -> Maker {
    Maker::Windows(WindowsSigner {
        subject: Subject {
            common_name: Some("Example Ltd".into()),
            organization: Some("Example Ltd".into()),
            country: Some("NG".into()),
            ..Subject::default()
        },
        certificate: Sha256([1; 32]),
        issuer: "CN=CA".into(),
        issuer_organization: Some("Some CA Ltd".into()),
        public_key: Some(Sha256([2; 32])),
    })
}

fn expected() -> Expected {
    Expected {
        system: System::Windows,
        maker: maker(),
        formats: vec![Format::WindowsProgram, Format::WindowsInstaller],
        arch: Arch::X64,
        bytes: None,
    }
}

type During = Box<dyn Fn(&Path)>;

/// Stands in for Windows: answers as told, counts calls, and can act on the file mid-check.
struct Fake {
    signature: SystemSignature,
    malware: Malware,
    calls: Cell<u32>,
    during: RefCell<Option<During>>,
}

impl Fake {
    fn new(signature: Signature, embedded: bool) -> Self {
        Self {
            signature: SystemSignature {
                signature,
                embedded,
                maker: None,
                arch: None,
            },
            malware: Malware::NoDetection {
                coverage: Coverage {
                    scanner: "stand-in".into(),
                    later_downloads_not_checked: true,
                },
            },
            calls: Cell::new(0),
            during: RefCell::new(None),
        }
    }
}

impl SystemChecks for Fake {
    fn signature(&self, _: &Path, _: Format, _: &Limits) -> SystemSignature {
        self.calls.set(self.calls.get() + 1);
        self.signature.clone()
    }
    fn malware(&self, path: &Path, _: Format, _: &Signature, _: &Limits) -> Malware {
        self.calls.set(self.calls.get() + 1);
        if let Some(f) = self.during.borrow().as_ref() {
            f(path);
        }
        self.malware.clone()
    }
}

fn write(dir: &tempfile::TempDir, name: &str, bytes: &[u8]) -> PathBuf {
    let p = dir.path().join(name);
    std::fs::write(&p, bytes).unwrap();
    p
}

#[test]
fn an_unsigned_program_ends_needs_action_bound_to_its_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = tiny_pe();
    let p = write(&dir, "setup.exe", &bytes);
    let fake = Fake::new(Signature::Missing, false);
    let v = check(&p, &expected(), &fake, &Limits::default()).unwrap();
    assert_eq!(v.bytes.size, bytes.len() as u64);
    assert_eq!(v.bytes.sha256, Sha256(H::digest(&bytes).into()));
    assert_eq!(v.steady, Steady::Yes);
    assert_eq!(v.signature, Signature::Missing);
    assert_eq!(v.arch, ArchFact::Matches);
    assert_eq!(
        v.format,
        FormatFact::Matches {
            found: Format::WindowsProgram
        }
    );
    assert!(matches!(v.same_maker, SameMaker::Unknown { .. }));
    assert_eq!(
        v.decide(),
        Decision::NeedsAction {
            why: vec![Why::Unsigned, Why::MakerUnknown]
        }
    );
}

#[test]
fn a_sign_in_page_saved_as_setup_exe_is_not_checked_further() {
    // T21-023.
    let dir = tempfile::tempdir().unwrap();
    let p = write(&dir, "setup.exe", b"<!DOCTYPE html><title>Sign in</title>");
    let fake = Fake::new(Signature::Trusted, true);
    let v = check(&p, &expected(), &fake, &Limits::default()).unwrap();
    assert_eq!(
        v.format,
        FormatFact::Wrong {
            found: Format::WebPage
        }
    );
    assert_eq!(fake.calls.get(), 0, "nothing ran on a web page");
    let Decision::NeedsAction { why } = v.decide() else {
        panic!("installed a web page")
    };
    assert_eq!(why[0], Why::WrongKindOfFile);
}

#[test]
fn the_wrong_architecture_is_refused() {
    // T21-003: an x64 program for an ARM laptop.
    let dir = tempfile::tempdir().unwrap();
    let p = write(&dir, "setup.exe", &tiny_pe());
    let mut e = expected();
    e.arch = Arch::Arm64;
    let v = check(
        &p,
        &e,
        &Fake::new(Signature::Missing, false),
        &Limits::default(),
    )
    .unwrap();
    assert_eq!(v.arch, ArchFact::Wrong { found: Arch::X64 });
    assert!(
        matches!(v.decide(), Decision::NeedsAction { why } if why.contains(&Why::WrongArchitecture))
    );
}

#[test]
fn a_recipe_digest_that_differs_is_refused() {
    // T21-029.
    let dir = tempfile::tempdir().unwrap();
    let p = write(&dir, "setup.exe", &tiny_pe());
    let mut e = expected();
    e.bytes = Some(Sha256([0; 32]));
    let v = check(
        &p,
        &e,
        &Fake::new(Signature::Missing, false),
        &Limits::default(),
    )
    .unwrap();
    assert_eq!(v.expected_bytes, Some(Sha256([0; 32])));
    assert!(
        matches!(v.decide(), Decision::NeedsAction { why } if why.contains(&Why::NotExpectedBytes))
    );
}

#[test]
fn windows_finding_a_signature_pctwin_did_not_is_broken() {
    let dir = tempfile::tempdir().unwrap();
    let p = write(&dir, "setup.exe", &tiny_pe());
    for s in [
        Signature::Trusted,
        Signature::NotTrusted { why: "x".into() },
        Signature::Broken { why: "x".into() },
    ] {
        let v = check(&p, &expected(), &Fake::new(s, true), &Limits::default()).unwrap();
        assert!(
            matches!(v.signature, Signature::Broken { .. }),
            "{:?}",
            v.signature
        );
    }
    // A catalog signature is not the maker's own.
    let v = check(
        &p,
        &expected(),
        &Fake::new(Signature::Trusted, false),
        &Limits::default(),
    )
    .unwrap();
    assert!(matches!(v.signature, Signature::NotTrusted { why } if why.contains("catalog")));
    let v = check(
        &p,
        &expected(),
        &Fake::new(
            Signature::NotChecked {
                why: "no tool".into(),
            },
            false,
        ),
        &Limits::default(),
    )
    .unwrap();
    assert!(matches!(v.signature, Signature::NotChecked { .. }));
}

#[test]
fn a_change_during_the_check_is_caught() {
    // T21-035: the file is changed while the system's scan looks at it.
    let dir = tempfile::tempdir().unwrap();
    let p = write(&dir, "setup.exe", &tiny_pe());
    let fake = Fake::new(Signature::Missing, false);
    let wrote = std::rc::Rc::new(Cell::new(None));
    let w = wrote.clone();
    *fake.during.borrow_mut() = Some(Box::new(move |path: &Path| {
        let mut b = tiny_pe();
        b[0x300] ^= 1;
        w.set(Some(std::fs::write(path, &b).is_ok()));
    }));
    let v = check(&p, &expected(), &fake, &Limits::default()).unwrap();
    if cfg!(windows) {
        // Windows: the held file cannot be written while it is checked.
        assert_eq!(wrote.get(), Some(false));
        assert_eq!(v.steady, Steady::Yes);
    } else {
        assert_eq!(wrote.get(), Some(true));
        assert_eq!(v.steady, Steady::Changed);
        assert!(
            matches!(v.decide(), Decision::NeedsAction { why } if why[0] == Why::ChangedDuringCheck)
        );
    }
}

#[test]
fn a_swap_of_the_name_during_the_check_is_caught() {
    let dir = tempfile::tempdir().unwrap();
    let p = write(&dir, "setup.exe", &tiny_pe());
    let fake = Fake::new(Signature::Missing, false);
    let swapped = std::rc::Rc::new(Cell::new(None));
    let s = swapped.clone();
    *fake.during.borrow_mut() = Some(Box::new(move |path: &Path| {
        let aside = path.with_extension("old");
        let moved = std::fs::rename(path, &aside).is_ok();
        if moved {
            std::fs::write(path, tiny_pe()).unwrap();
        }
        s.set(Some(moved));
    }));
    let v = check(&p, &expected(), &fake, &Limits::default()).unwrap();
    if cfg!(windows) {
        // Windows: the held file cannot be renamed while it is checked.
        assert_eq!(swapped.get(), Some(false));
        assert_eq!(v.steady, Steady::Yes);
    } else {
        // Same bytes, different file: still caught.
        assert_eq!(swapped.get(), Some(true));
        assert_eq!(v.steady, Steady::Changed);
    }
}

#[test]
fn files_that_cannot_be_checked_at_all() {
    let dir = tempfile::tempdir().unwrap();
    let fake = Fake::new(Signature::Missing, false);
    let p = write(&dir, "setup.exe", &tiny_pe());
    assert!(matches!(
        check(
            Path::new("setup.exe"),
            &expected(),
            &fake,
            &Limits::default()
        ),
        Err(CheckError::NotAbsolute)
    ));
    let small = Limits {
        max_size: 100,
        ..Limits::default()
    };
    assert!(matches!(
        check(&p, &expected(), &fake, &small),
        Err(CheckError::TooLarge(0x400))
    ));
    let exact = Limits {
        max_size: 0x400,
        ..Limits::default()
    };
    assert!(check(&p, &expected(), &fake, &exact).is_ok());
    assert!(check(dir.path(), &expected(), &fake, &Limits::default()).is_err());
    assert!(matches!(
        check(
            &dir.path().join("missing.exe"),
            &expected(),
            &fake,
            &Limits::default()
        ),
        Err(CheckError::Unreadable(_))
    ));
    #[cfg(unix)]
    {
        let link = dir.path().join("link.exe");
        std::os::unix::fs::symlink(&p, &link).unwrap();
        assert!(matches!(
            check(&link, &expected(), &fake, &Limits::default()),
            Err(CheckError::Link)
        ));
    }
    assert_eq!(fake.calls.get(), 2, "only the exact-size file was checked");
}

#[test]
fn an_empty_file_is_an_unknown_kind() {
    let dir = tempfile::tempdir().unwrap();
    let p = write(&dir, "setup.exe", b"");
    let v = check(
        &p,
        &expected(),
        &Fake::new(Signature::Missing, false),
        &Limits::default(),
    )
    .unwrap();
    assert_eq!(
        v.format,
        FormatFact::Wrong {
            found: Format::Unknown
        }
    );
    assert_eq!(v.bytes.size, 0);
}

#[test]
fn a_compound_file_that_is_not_an_installer_is_the_wrong_kind() {
    let dir = tempfile::tempdir().unwrap();
    let mut c = cfb::CompoundFile::create(std::io::Cursor::new(Vec::new())).unwrap();
    c.set_storage_clsid(
        "/",
        uuid::Uuid::from_u128(0x00020906_0000_0000_C000_000000000046),
    )
    .unwrap();
    c.flush().unwrap();
    let p = write(&dir, "setup.msi", &c.into_inner().into_inner());
    let v = check(
        &p,
        &expected(),
        &Fake::new(Signature::Trusted, true),
        &Limits::default(),
    )
    .unwrap();
    assert_eq!(
        v.format,
        FormatFact::Wrong {
            found: Format::Unknown
        }
    );
}

#[test]
fn an_unsigned_msi_is_read_as_an_installer() {
    let dir = tempfile::tempdir().unwrap();
    let mut c = cfb::CompoundFile::create(std::io::Cursor::new(Vec::new())).unwrap();
    c.set_storage_clsid(
        "/",
        uuid::Uuid::from_u128(0x000C1084_0000_0000_C000_000000000046),
    )
    .unwrap();
    c.flush().unwrap();
    let p = write(&dir, "setup.msi", &c.into_inner().into_inner());
    let v = check(
        &p,
        &expected(),
        &Fake::new(Signature::Missing, false),
        &Limits::default(),
    )
    .unwrap();
    assert_eq!(
        v.format,
        FormatFact::Matches {
            found: Format::WindowsInstaller
        }
    );
    assert_eq!(v.signature, Signature::Missing);
    // No summary: what it is built for cannot be told.
    assert_eq!(v.arch, ArchFact::Unknown);
}

const REF: &str = "[Flatpak Ref]
Name=org.example.App
Url=https://dl.flathub.org/repo/
GPGKey=AAECAwQ=
";

fn flatpak_old() -> Maker {
    Maker::Linux(LinuxSource::Flatpak {
        remote_url: "https://dl.flathub.org/repo/".into(),
        app_id: "org.example.App".into(),
        key: Sha256(H::digest([0u8, 1, 2, 3, 4]).into()),
    })
}

fn flatpak_expected() -> Expected {
    Expected {
        system: System::Linux,
        maker: flatpak_old(),
        formats: vec![Format::FlatpakRef],
        arch: Arch::X64,
        bytes: None,
    }
}

/// Second review, finding B1: a downloaded reference is never trusted and never installed, even
/// one naming exactly the old laptop's remote and key; reading it alone is never "trusted".
#[test]
fn a_flatpak_reference_is_never_installed() {
    let dir = tempfile::tempdir().unwrap();
    let e = flatpak_expected();
    let fake = Fake::new(Signature::Missing, false);
    let p = write(&dir, "app.flatpakref", REF.as_bytes());
    let v = check(&p, &e, &fake, &Limits::default()).unwrap();
    assert_eq!(fake.calls.get(), 0);
    assert_eq!(v.found_maker, Some(flatpak_old()));
    assert!(matches!(v.signature, Signature::NotChecked { .. }), "{v:?}");
    assert!(matches!(v.decide(), Decision::NeedsAction { .. }), "{v:?}");

    // The reviewer's proof: a second group of the same name pointing at another server.
    let evil = format!(
        "{REF}[Flatpak Ref]\nName=org.example.App\nUrl=https://evil.example/repo/\n\
         GPGKey=ZXZpbGtleQ==\nRuntimeRepo=https://evil.example/r.flatpakrepo\n"
    );
    let p = write(&dir, "evil.flatpakref", evil.as_bytes());
    let v = check(&p, &e, &fake, &Limits::default()).unwrap();
    assert!(matches!(v.signature, Signature::Broken { .. }), "{v:?}");
    assert!(matches!(v.decide(), Decision::NeedsAction { .. }), "{v:?}");

    // A different key (a look-alike remote serving the same app ID).
    let other = write(
        &dir,
        "other.flatpakref",
        REF.replace("AAECAwQ=", "AAECAwU=").as_bytes(),
    );
    let v = check(&other, &e, &fake, &Limits::default()).unwrap();
    assert!(matches!(v.decide(), Decision::NeedsAction { why } if why.contains(&Why::OtherMaker)));

    // No key.
    let nokey = write(
        &dir,
        "nokey.flatpakref",
        REF.replace("GPGKey=AAECAwQ=\n", "").as_bytes(),
    );
    let v = check(&nokey, &e, &fake, &Limits::default()).unwrap();
    assert!(matches!(v.signature, Signature::Broken { .. }));
    assert!(matches!(v.same_maker, SameMaker::Unknown { .. }));

    // Too large to be a real reference.
    let mut big = REF.to_owned();
    big.push_str(&"#".repeat(70 * 1024));
    let big = write(&dir, "big.flatpakref", big.as_bytes());
    let v = check(&big, &e, &fake, &Limits::default()).unwrap();
    assert!(matches!(v.signature, Signature::Broken { why } if why.contains("too large")));
}

/// Second review, finding B1: a Flatpak app is installed by its ID from the old laptop's own
/// remote and key; a reference can only say which app is meant, never where from.
#[test]
fn a_flatpak_app_is_installed_from_the_old_laptops_own_source() {
    use pctwin_maker::linux::{flatpak_route, install_call, parse_flatpakref, remote_add_call};
    let t = std::time::Duration::from_secs(60);
    let route = flatpak_route(&flatpak_old(), None).unwrap();
    // A reference naming another remote, address and key changes nothing.
    let evil = parse_flatpakref(
        b"[Flatpak Ref]\nName=org.example.App\nUrl=https://evil.example/repo/\nGPGKey=ZXZpbA==\n",
    )
    .unwrap();
    assert_eq!(flatpak_route(&flatpak_old(), Some(&evil)).unwrap(), route);
    assert_eq!(route.remote_url, "https://dl.flathub.org/repo/");
    assert_eq!(route.app_id, "org.example.App");
    // ... but one pointing at another app is refused.
    let other_app = parse_flatpakref(
        b"[Flatpak Ref]\nName=org.evil.App\nUrl=https://dl.flathub.org/repo/\nGPGKey=AAECAwQ=\n",
    )
    .unwrap();
    assert!(flatpak_route(&flatpak_old(), Some(&other_app)).is_err());
    // The remote is added only with the old laptop's key, under PCTwin's own name for it.
    let key_file = std::path::Path::new("/var/tmp/pctwin/key.gpg");
    assert!(remote_add_call(&route, key_file, b"another key", t).is_err());
    let add = remote_add_call(&route, key_file, &[0, 1, 2, 3, 4], t).unwrap();
    let args: Vec<String> = add
        .args
        .iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        args,
        [
            "remote-add",
            "--system",
            "--if-not-exists",
            "--gpg-import=/var/tmp/pctwin/key.gpg",
            route.remote_name.as_str(),
            "https://dl.flathub.org/repo/",
        ]
    );
    let install = install_call(&route, t);
    let args: Vec<String> = install
        .args
        .iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        args,
        [
            "install",
            "--system",
            "--noninteractive",
            route.remote_name.as_str(),
            "org.example.App"
        ]
    );
    assert!(!args.iter().any(|a| a.contains("flatpakref")));
    // The remote's name is made from its address and key: another key or address, another name.
    let other_key = Maker::Linux(LinuxSource::Flatpak {
        remote_url: "https://dl.flathub.org/repo/".into(),
        app_id: "org.example.App".into(),
        key: Sha256([9; 32]),
    });
    let other_url = Maker::Linux(LinuxSource::Flatpak {
        remote_url: "https://dl.flathub.org/repx/".into(),
        app_id: "org.example.App".into(),
        key: Sha256(H::digest([0u8, 1, 2, 3, 4]).into()),
    });
    let n1 = flatpak_route(&other_key, None).unwrap().remote_name;
    let n2 = flatpak_route(&other_url, None).unwrap().remote_name;
    assert!(route.remote_name.starts_with("pctwin-") && route.remote_name.len() == 23);
    assert_ne!(n1, route.remote_name);
    assert_ne!(n2, route.remote_name);
    // Only a Flatpak app on the old laptop, from a secure address, with a real app ID.
    let mac = Maker::Mac(MacTeam {
        team_id: "ABCDE12345".into(),
    });
    assert!(flatpak_route(&mac, None).is_err());
    for (url, id) in [
        ("http://dl.flathub.org/repo/", "org.example.App"),
        ("https://dl.flathub.org/re po/", "org.example.App"),
        ("https://dl.flathub.org/repo/", "--user"),
        ("https://dl.flathub.org/repo/", "org.example"),
        ("https://dl.flathub.org/repo/", "org.-example.App"),
        ("https://dl.flathub.org/repo/", "org.ex ample.App"),
        ("https://dl.flathub.org/repo/", "org..App"),
    ] {
        let m = Maker::Linux(LinuxSource::Flatpak {
            remote_url: url.into(),
            app_id: id.into(),
            key: Sha256([1; 32]),
        });
        assert!(flatpak_route(&m, None).is_err(), "{url} {id}");
    }
    let long = format!("org.example.{}", "a".repeat(250));
    let m = Maker::Linux(LinuxSource::Flatpak {
        remote_url: "https://dl.flathub.org/repo/".into(),
        app_id: long,
        key: Sha256([1; 32]),
    });
    assert!(flatpak_route(&m, None).is_err());
    let ok = Maker::Linux(LinuxSource::Flatpak {
        remote_url: "https://dl.flathub.org/repo/".into(),
        app_id: "org.gnome.Text_Editor-2".into(),
        key: Sha256([1; 32]),
    });
    assert!(flatpak_route(&ok, None).is_ok());
}

#[test]
fn a_flatpak_reference_of_exactly_the_limit_is_read() {
    let dir = tempfile::tempdir().unwrap();
    let mut full = REF.to_owned();
    full.push_str(&"#".repeat(pctwin_maker::linux::MAX_FLATPAKREF - REF.len()));
    let p = write(&dir, "full.flatpakref", full.as_bytes());
    let e = Expected {
        system: System::Linux,
        maker: Maker::Mac(MacTeam {
            team_id: "ABCDE12345".into(),
        }),
        formats: vec![Format::FlatpakRef],
        arch: Arch::X64,
        bytes: None,
    };
    let v = check(
        &p,
        &e,
        &Fake::new(Signature::Missing, false),
        &Limits::default(),
    )
    .unwrap();
    // Read whole, and still never trusted on its own word.
    assert!(matches!(v.signature, Signature::NotChecked { .. }), "{v:?}");
}

#[test]
fn the_old_copys_maker_needs_a_signed_windows_file() {
    let dir = tempfile::tempdir().unwrap();
    let page = write(&dir, "app.exe", b"<html>");
    let e = pctwin_maker::windows_maker_of(&page).unwrap_err();
    assert!(e.contains("not a Windows program"), "{e}");
    let unsigned = write(&dir, "app2.exe", &tiny_pe());
    let e = pctwin_maker::windows_maker_of(&unsigned).unwrap_err();
    assert!(e.contains("not signed"), "{e}");
    let mut c = cfb::CompoundFile::create(std::io::Cursor::new(Vec::new())).unwrap();
    c.set_storage_clsid(
        "/",
        uuid::Uuid::from_u128(0x000C1084_0000_0000_C000_000000000046),
    )
    .unwrap();
    c.flush().unwrap();
    let msi = write(&dir, "app.msi", &c.into_inner().into_inner());
    let e = pctwin_maker::windows_maker_of(&msi).unwrap_err();
    assert!(e.contains("not signed"), "{e}");
}
