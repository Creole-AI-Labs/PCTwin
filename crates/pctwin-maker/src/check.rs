//! The maker check of one installer file, start to end. It never runs the file.
//!
//! 1. Open the file (on Windows, so that no one can write, rename or delete it while it is open)
//!    and note which file the path leads to.
//! 2. Read every byte once: the SHA-256 the verdict is bound to, and the first and last bytes
//!    that say what kind of file it is.
//! 3. Read the signature in pure Rust where PCTwin can (Windows programs and installers), and ask
//!    the system's own checks (signature trust, malware scan).
//! 4. Check the path still leads to the same file, and read every byte again: a file changed or
//!    swapped during the check is reported as changed, never approved (T21-035).

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use std::time::Duration;

use sha2::{Digest, Sha256 as Sha256Hasher};

use crate::facts::{
    Arch, ArchFact, Bytes, Format, FormatFact, Maker, Malware, SameMaker, Sha256, Signature,
    Steady, System, Verdict, compare,
};
use crate::msi::{MsiSignature, read_msi};
use crate::pe::{PeSignature, read_pe};
use crate::sniff::{HEAD, TAIL, sniff};

/// What the old laptop's copy and the recipe say the download must be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expected {
    pub system: System,
    /// The maker of the old laptop's copy.
    pub maker: Maker,
    /// The kinds of file accepted (an EXE or an MSI, say).
    pub formats: Vec<Format>,
    /// The laptop's architecture.
    pub arch: Arch,
    /// The exact bytes, when a trusted recipe names them.
    pub bytes: Option<Sha256>,
}

/// Bounds on one check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The largest file checked.
    pub max_size: u64,
    /// How long the system's signature check may take.
    pub signature_timeout: Duration,
    /// How long the malware scan may take.
    pub scan_timeout: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_size: 16 * 1024 * 1024 * 1024,
            signature_timeout: Duration::from_secs(120),
            scan_timeout: Duration::from_secs(600),
        }
    }
}

/// What the system's own signature check says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemSignature {
    pub signature: Signature,
    /// Only for a signature inside the file; false for a Windows catalog signature.
    pub embedded: bool,
    /// The maker the system read, if any (Windows: the signer; Mac: the Team ID; Linux: the key).
    pub maker: Option<Result<Maker, String>>,
    /// What the system says the file is built for, when the bytes alone do not say.
    pub arch: Option<Arch>,
}

/// The system's own checks. The real ones call the system's tools; tests stand in for them.
pub trait SystemChecks {
    /// The signature of `file`, held open by the check, at `path`. A check that can be asked
    /// about the open file must use `file`, never reopen `path`: the bytes trusted are then the
    /// bytes the check hashed (second review, finding B2). Windows does so for a program, but its
    /// installer-package checker reads `path` again (third review, finding 3): see
    /// [`check_staged`].
    fn signature(
        &self,
        file: &File,
        path: &Path,
        format: Format,
        limits: &Limits,
    ) -> SystemSignature;
    fn malware(
        &self,
        path: &Path,
        format: Format,
        signature: &Signature,
        limits: &Limits,
    ) -> Malware;
}

/// Why no check could be made at all.
#[derive(Debug, thiserror::Error)]
pub enum CheckError {
    #[error("the installer must be named by its full path")]
    NotAbsolute,
    #[error("the installer is a link, not a file")]
    Link,
    #[error("the installer is not a file")]
    NotAFile,
    #[error("the installer is larger than PCTwin checks ({0} bytes)")]
    TooLarge(u64),
    #[error("the installer cannot be read ({0})")]
    Unreadable(#[from] io::Error),
}

fn open_held(path: &Path) -> io::Result<File> {
    let mut o = std::fs::OpenOptions::new();
    o.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // Others may read it, but not write, rename or delete it while it is held.
        const FILE_SHARE_READ: u32 = 1;
        // Do not follow a link at the final name.
        const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
        o.share_mode(FILE_SHARE_READ)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.custom_flags(libc::O_NOFOLLOW);
    }
    o.open(path)
}

/// One pass over every byte: the fingerprint and size.
fn fingerprint(f: &mut File) -> io::Result<Bytes> {
    f.seek(SeekFrom::Start(0))?;
    let mut h = Sha256Hasher::new();
    let mut buf = vec![0u8; 1024 * 1024];
    let mut size = 0u64;
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
        size += n as u64;
    }
    Ok(Bytes {
        size,
        sha256: Sha256(h.finalize().into()),
    })
}

fn ends(f: &mut File, size: u64) -> io::Result<(Vec<u8>, Vec<u8>)> {
    let mut head = vec![0u8; usize::try_from(size.min(HEAD as u64)).unwrap_or(HEAD)];
    f.seek(SeekFrom::Start(0))?;
    f.read_exact(&mut head)?;
    let tail_len = size.min(TAIL as u64);
    let mut tail = vec![0u8; usize::try_from(tail_len).unwrap_or(TAIL)];
    f.seek(SeekFrom::Start(size - tail_len))?;
    f.read_exact(&mut tail)?;
    Ok((head, tail))
}

/// Who signed it, as PCTwin's own reader sees it (Windows files only).
#[expect(
    clippy::large_enum_variant,
    reason = "made once per check; boxing buys nothing"
)]
enum OwnRead {
    NotRead,
    None,
    Broken(String),
    Signed {
        signer: Result<crate::facts::WindowsSigner, String>,
        /// None for an MSI: PCTwin's own reader does not work out a package's digest; Windows
        /// does, reading it by name, so only the helper's own copy counts ([`check_staged`]).
        digest_matches: Option<bool>,
    },
}

/// The administrator helper's own copy of a package: in its administrators-only folder, opened by
/// the helper and held so nobody can write, rename or delete it while it is checked and
/// installed. Windows' installer-package checker reads a package by its name, not through the
/// held file (third review, finding 3), so its answer is about the bytes the helper holds only
/// when nobody else can change what that name leads to: here, and nowhere else. Implemented only
/// by the helper's staging (`pctwin-helper`, `broker::staging::VerifiedArtifact`): it is sealed
/// ([`pctwin_staged_seal::Sealed`]), and only this crate and the helper may depend on the seal.
pub trait HelperStaged: pctwin_staged_seal::Sealed {
    /// Where the copy is, in the helper's administrators-only folder.
    fn staged_path(&self) -> &Path;
}

/// Why an installer package's signature is not counted outside the helper's own copy.
pub const PACKAGE_READ_BY_NAME: &str = "Windows reads an installer package by its name, not the copy PCTwin holds, so its signature is checked only on the administrator helper's own copy";

/// Checks one installer file against what is expected of it. A Windows Installer package checked
/// here is never trusted ("not checked": see [`HelperStaged`]); the administrator helper checks
/// its own copy with [`check_staged`].
pub fn check(
    path: &Path,
    expected: &Expected,
    system: &dyn SystemChecks,
    limits: &Limits,
) -> Result<Verdict, CheckError> {
    check_at(path, false, expected, system, limits)
}

/// Checks the administrator helper's own copy of a package, the only place a Windows Installer
/// package's signature counts.
pub fn check_staged(
    staged: &dyn HelperStaged,
    expected: &Expected,
    system: &dyn SystemChecks,
    limits: &Limits,
) -> Result<Verdict, CheckError> {
    check_at(staged.staged_path(), true, expected, system, limits)
}

fn check_at(
    path: &Path,
    staged: bool,
    expected: &Expected,
    system: &dyn SystemChecks,
    limits: &Limits,
) -> Result<Verdict, CheckError> {
    if !path.is_absolute() {
        return Err(CheckError::NotAbsolute);
    }
    if std::fs::symlink_metadata(path)?.file_type().is_symlink() {
        return Err(CheckError::Link);
    }
    let mut file = open_held(path)?;
    let meta = file.metadata()?;
    if !meta.is_file() {
        return Err(CheckError::NotAFile);
    }
    if meta.len() > limits.max_size {
        return Err(CheckError::TooLarge(meta.len()));
    }
    let held = same_file::Handle::from_file(file.try_clone()?)?;
    let first = fingerprint(&mut file)?;
    if first.size > limits.max_size {
        return Err(CheckError::TooLarge(first.size));
    }
    let (head, tail) = ends(&mut file, first.size)?;
    let sniffed = sniff(&head, &tail);
    let mut format = sniffed.format;
    let mut arch = sniffed.arch;

    let mut own = OwnRead::NotRead;
    if sniffed.compound {
        if let Ok(m) = read_msi(&file)
            && m.is_package
        {
            format = Format::WindowsInstaller;
            arch = m.arch;
            own = match m.signature {
                MsiSignature::None => OwnRead::None,
                MsiSignature::Broken(why) => OwnRead::Broken(why),
                MsiSignature::Signed(signer) => OwnRead::Signed {
                    signer,
                    digest_matches: None,
                },
            };
        }
    } else if format == Format::WindowsProgram {
        own = match read_pe(&mut file, first.size)? {
            PeSignature::None => OwnRead::None,
            PeSignature::Broken(why) => OwnRead::Broken(why),
            PeSignature::Signed {
                signer,
                digest_matches,
            } => OwnRead::Signed {
                signer,
                digest_matches: Some(digest_matches),
            },
        };
    } else if format == Format::FlatpakRef {
        // A downloaded reference is a pointer only (second review, finding B1): it can name any
        // remote, address and key, so it is never trusted and never installed. A Flatpak app is
        // installed by its ID from the old laptop's own remote and key
        // ([`crate::linux::flatpak_route`]); here the reference is only read for the report.
        let maker = if first.size > crate::linux::MAX_FLATPAKREF as u64 {
            Err("the Flatpak reference is too large".to_owned())
        } else {
            crate::linux::parse_flatpakref(&head).map(Maker::Linux)
        };
        let found = maker.as_ref().ok().cloned();
        let signature = match &maker {
            Ok(_) => Signature::NotChecked {
                why: "a Flatpak reference is only a pointer; PCTwin installs the app by its name \
                      from the old laptop's own source instead"
                    .into(),
            },
            Err(why) => Signature::Broken { why: why.clone() },
        };
        return finish(
            path,
            expected,
            &mut file,
            &held,
            first,
            Facts {
                format,
                arch,
                signature: signature.clone(),
                same_maker: same_maker(&expected.maker, maker.map_err(|e| e.to_string())),
                malware: crate::linux::malware_from(&signature),
                found_maker: found,
            },
        );
    }

    let format_fact = if expected.formats.contains(&format) {
        FormatFact::Matches { found: format }
    } else {
        FormatFact::Wrong { found: format }
    };
    if let FormatFact::Wrong { .. } = format_fact {
        let why = "not checked: the download is not an installer of the expected kind".to_owned();
        return finish(
            path,
            expected,
            &mut file,
            &held,
            first,
            Facts {
                format,
                arch,
                signature: Signature::NotChecked { why: why.clone() },
                same_maker: SameMaker::Unknown { why: why.clone() },
                malware: Malware::NotFullyChecked { why },
                found_maker: None,
            },
        );
    }

    let mut sys = system.signature(&file, path, format, limits);
    if format == Format::WindowsInstaller && !staged {
        // Windows' answer is about whatever the name leads to, not the bytes held: only PCTwin's
        // own reading of the held bytes counts here. No signature in them is still "missing";
        // one in them is not checked.
        sys.signature = match own {
            OwnRead::None => Signature::Missing,
            _ => Signature::NotChecked {
                why: PACKAGE_READ_BY_NAME.into(),
            },
        };
        sys.embedded = false;
        sys.maker = None;
    }
    let (signature, found) = match own {
        OwnRead::NotRead => (sys.signature.clone(), sys.maker.clone()),
        own => combine_windows(own, &sys),
    };
    let arch = arch.or(sys.arch);
    let malware = system.malware(path, format, &signature, limits);
    let found_maker = found.as_ref().and_then(|m| m.as_ref().ok().cloned());
    let same = match found {
        Some(m) => same_maker(&expected.maker, m),
        None => SameMaker::Unknown {
            why: "no maker could be read from the download".into(),
        },
    };
    finish(
        path,
        expected,
        &mut file,
        &held,
        first,
        Facts {
            format,
            arch,
            signature,
            same_maker: same,
            malware,
            found_maker,
        },
    )
}

fn same_maker(old: &Maker, found: Result<Maker, String>) -> SameMaker {
    match found {
        Ok(m) => compare(old, &m),
        Err(why) => SameMaker::Unknown { why },
    }
}

/// PCTwin's own reading and Windows' together. Both must agree for the signature to be trusted.
fn combine_windows(
    own: OwnRead,
    sys: &SystemSignature,
) -> (Signature, Option<Result<Maker, String>>) {
    let sys_signer = match &sys.maker {
        Some(Ok(Maker::Windows(s))) => Some(Ok(s.clone())),
        Some(Ok(_)) => Some(Err("Windows named a maker of another system".to_owned())),
        Some(Err(e)) => Some(Err(e.clone())),
        None => None,
    };
    match own {
        OwnRead::NotRead | OwnRead::None => {
            let signature = match &sys.signature {
                Signature::Missing => Signature::Missing,
                Signature::Trusted if !sys.embedded => Signature::NotTrusted {
                    why: "the file is signed only through a Windows catalog, not by its maker inside the file".into(),
                },
                Signature::Trusted | Signature::NotTrusted { .. } | Signature::Broken { .. } => {
                    Signature::Broken {
                        why: "Windows found a signature PCTwin's own reader did not".into(),
                    }
                }
                Signature::NotChecked { why } => Signature::NotChecked { why: why.clone() },
            };
            (signature, None)
        }
        OwnRead::Broken(why) => (Signature::Broken { why }, None),
        OwnRead::Signed {
            signer,
            digest_matches,
        } => {
            let ours = signer.map(Maker::Windows);
            if digest_matches == Some(false) {
                let why = "the file's bytes are not the bytes its maker signed".into();
                return (Signature::Broken { why }, Some(ours));
            }
            let signature = match &sys.signature {
                // A Windows part can carry its own signature and also be listed in a Windows
                // catalog; Windows then checks the catalog only (seen on MpCmdRun.exe, 26
                // October 2026), so the maker's own signature was not checked by Windows.
                Signature::Trusted if !sys.embedded => Signature::NotTrusted {
                    why: "Windows checked it through its own catalog, not through the maker's signature inside the file".into(),
                },
                Signature::Trusted => match (&ours, &sys_signer) {
                    (Ok(Maker::Windows(a)), Some(Ok(b))) if a.certificate == b.certificate => {
                        Signature::Trusted
                    }
                    _ => Signature::Broken {
                        why: "Windows and PCTwin's own reader name different signers".into(),
                    },
                },
                Signature::Missing => Signature::Broken {
                    why: "PCTwin's own reader found a signature Windows did not".into(),
                },
                other => other.clone(),
            };
            (signature, Some(ours))
        }
    }
}

struct Facts {
    format: Format,
    arch: Option<Arch>,
    signature: Signature,
    same_maker: SameMaker,
    malware: Malware,
    found_maker: Option<Maker>,
}

fn finish(
    path: &Path,
    expected: &Expected,
    file: &mut File,
    held: &same_file::Handle,
    first: Bytes,
    facts: Facts,
) -> Result<Verdict, CheckError> {
    // The name must still lead to the file held, and its bytes must be the same.
    let same_file = same_file::Handle::from_path(path).is_ok_and(|now| now == *held);
    let second = fingerprint(file)?;
    let steady = if same_file && second == first {
        Steady::Yes
    } else {
        Steady::Changed
    };
    let format = if expected.formats.contains(&facts.format) {
        FormatFact::Matches {
            found: facts.format,
        }
    } else {
        FormatFact::Wrong {
            found: facts.format,
        }
    };
    let arch = match facts.arch {
        Some(a) if a.runs_on(expected.arch) => ArchFact::Matches,
        Some(found) => ArchFact::Wrong { found },
        None => ArchFact::Unknown,
    };
    Ok(Verdict {
        system: expected.system,
        bytes: first,
        steady,
        expected_bytes: expected.bytes,
        format,
        arch,
        signature: facts.signature,
        same_maker: facts.same_maker,
        malware: facts.malware,
        found_maker: facts.found_maker,
    })
}

/// Reads the maker of the old laptop's own copy of a Windows program or installer, by PCTwin's
/// own reader only (the old laptop is read-only and may be offline). The copy's bytes must be
/// the bytes signed, or there is no maker to anchor on.
pub fn windows_maker_of(path: &Path) -> Result<crate::facts::WindowsSigner, String> {
    let mut f = open_held(path).map_err(|e| e.to_string())?;
    let size = f.metadata().map_err(|e| e.to_string())?.len();
    let (head, tail) = ends(&mut f, size).map_err(|e| e.to_string())?;
    let s = sniff(&head, &tail);
    if s.compound {
        return match read_msi(&f)?.signature {
            MsiSignature::Signed(signer) => signer,
            MsiSignature::None => Err("the old copy is not signed".into()),
            MsiSignature::Broken(why) => Err(why),
        };
    }
    if s.format != Format::WindowsProgram {
        return Err("the old copy is not a Windows program".into());
    }
    match read_pe(&mut f, size).map_err(|e| e.to_string())? {
        PeSignature::Signed {
            signer,
            digest_matches: true,
        } => signer,
        PeSignature::Signed { .. } => Err("the old copy was changed after it was signed".into()),
        PeSignature::None => Err("the old copy is not signed".into()),
        PeSignature::Broken(why) => Err(why),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::{MacTeam, Subject, WindowsSigner};

    fn signer(cert: u8) -> WindowsSigner {
        WindowsSigner {
            subject: Subject::default(),
            certificate: Sha256([cert; 32]),
            issuer: String::new(),
            issuer_organization: None,
            public_key: None,
        }
    }

    fn sys(
        signature: Signature,
        embedded: bool,
        maker: Option<Result<Maker, String>>,
    ) -> SystemSignature {
        SystemSignature {
            signature,
            embedded,
            maker,
            arch: None,
        }
    }

    fn signed(cert: u8, digest: Option<bool>) -> OwnRead {
        OwnRead::Signed {
            signer: Ok(signer(cert)),
            digest_matches: digest,
        }
    }

    fn win(cert: u8) -> Option<Result<Maker, String>> {
        Some(Ok(Maker::Windows(signer(cert))))
    }

    fn is_broken(s: &Signature) -> bool {
        matches!(s, Signature::Broken { .. })
    }

    #[test]
    fn both_readings_must_agree_for_trust() {
        for digest in [Some(true), None] {
            let (s, m) = combine_windows(signed(1, digest), &sys(Signature::Trusted, true, win(1)));
            assert_eq!(s, Signature::Trusted);
            assert_eq!(m, Some(Ok(Maker::Windows(signer(1)))));
        }
        let (s, _) = combine_windows(
            signed(1, Some(true)),
            &sys(Signature::Trusted, true, win(2)),
        );
        assert!(is_broken(&s));
        let (s, _) = combine_windows(signed(1, Some(true)), &sys(Signature::Trusted, true, None));
        assert!(is_broken(&s));
        let (s, _) = combine_windows(
            signed(1, Some(true)),
            &sys(Signature::Trusted, true, Some(Err("x".into()))),
        );
        assert!(is_broken(&s));
        let mac = Some(Ok(Maker::Mac(MacTeam {
            team_id: "ABCDE12345".into(),
        })));
        let (s, _) = combine_windows(signed(1, Some(true)), &sys(Signature::Trusted, true, mac));
        assert!(is_broken(&s));
        let own_err = OwnRead::Signed {
            signer: Err("x".into()),
            digest_matches: Some(true),
        };
        let (s, m) = combine_windows(own_err, &sys(Signature::Trusted, true, win(1)));
        assert!(is_broken(&s));
        assert_eq!(m, Some(Err("x".into())));
    }

    #[test]
    fn a_catalog_check_is_not_the_makers_signature() {
        let (s, _) = combine_windows(
            signed(1, Some(true)),
            &sys(Signature::Trusted, false, win(1)),
        );
        assert!(matches!(s, Signature::NotTrusted { why } if why.contains("catalog")));
        let (s, m) = combine_windows(OwnRead::None, &sys(Signature::Trusted, false, win(1)));
        assert!(matches!(s, Signature::NotTrusted { why } if why.contains("catalog")));
        assert_eq!(m, None);
    }

    #[test]
    fn changed_bytes_are_broken_whatever_windows_says() {
        let (s, m) = combine_windows(
            signed(1, Some(false)),
            &sys(Signature::Trusted, true, win(1)),
        );
        assert!(matches!(s, Signature::Broken { why } if why.contains("not the bytes")));
        assert!(m.is_some());
    }

    #[test]
    fn windows_answers_pass_through_when_not_trusted() {
        let nt = Signature::NotTrusted {
            why: "expired".into(),
        };
        let (s, _) = combine_windows(signed(1, Some(true)), &sys(nt.clone(), true, win(1)));
        assert_eq!(s, nt);
        let (s, _) = combine_windows(signed(1, Some(true)), &sys(Signature::Missing, false, None));
        assert!(matches!(s, Signature::Broken { why } if why.contains("did not")));
        let nc = Signature::NotChecked { why: "x".into() };
        let (s, _) = combine_windows(signed(1, Some(true)), &sys(nc.clone(), false, None));
        assert_eq!(s, nc);
    }

    #[test]
    fn no_signature_of_our_own() {
        let (s, m) = combine_windows(OwnRead::None, &sys(Signature::Missing, false, None));
        assert_eq!((s, m), (Signature::Missing, None));
        for other in [
            Signature::Trusted,
            Signature::NotTrusted { why: "x".into() },
            Signature::Broken { why: "x".into() },
        ] {
            let (s, _) = combine_windows(OwnRead::None, &sys(other, true, win(1)));
            assert!(matches!(s, Signature::Broken { why } if why.contains("did not")));
        }
        let nc = Signature::NotChecked { why: "x".into() };
        let (s, _) = combine_windows(OwnRead::NotRead, &sys(nc.clone(), false, None));
        assert_eq!(s, nc);
        let (s, m) = combine_windows(
            OwnRead::Broken("bad".into()),
            &sys(Signature::Trusted, true, win(1)),
        );
        assert_eq!(s, Signature::Broken { why: "bad".into() });
        assert_eq!(m, None);
    }

    #[test]
    fn the_same_bytes_in_another_file_are_a_change() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.exe");
        let b = dir.path().join("b.exe");
        std::fs::write(&a, b"same").unwrap();
        std::fs::write(&b, b"same").unwrap();
        let mut held_file = open_held(&a).unwrap();
        let first = fingerprint(&mut held_file).unwrap();
        let held = same_file::Handle::from_file(held_file.try_clone().unwrap()).unwrap();
        let facts = || Facts {
            format: Format::WindowsProgram,
            arch: None,
            signature: Signature::Missing,
            same_maker: SameMaker::Unknown { why: String::new() },
            malware: Malware::Unavailable { why: String::new() },
            found_maker: None,
        };
        let e = Expected {
            system: System::Windows,
            maker: Maker::Mac(MacTeam {
                team_id: "ABCDE12345".into(),
            }),
            formats: vec![Format::WindowsProgram],
            arch: Arch::X64,
            bytes: None,
        };
        let same = finish(&a, &e, &mut held_file, &held, first, facts()).unwrap();
        assert_eq!(same.steady, Steady::Yes);
        let other = finish(&b, &e, &mut held_file, &held, first, facts()).unwrap();
        assert_eq!(other.steady, Steady::Changed);
    }
}
