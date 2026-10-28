//! PCTwin's own reading of an installer's insides, run only in the locked-down parser process
//! (Phase 2 adoption, change 1).
//!
//! A program's embedded signature (its layout, the Authenticode digest, the CMS and X.509 inside
//! it), an installer package's compound file and a Flatpak reference are read from bytes that
//! arrived from a download or another laptop. A mistake in reading them must not reach the app
//! or the administrator helper, so the reading happens in a separate process
//! (`pctwin-parse`), asked through [`OwnReader`]. The readers here take an [`InChild`]: only
//! the parser process (and this crate's own tests) can call them in their own process.
//!
//! A reader that could not answer (the parser process stopped, ran out of time, or asked for
//! more than the file) gives `Err(why)`; the maker check then says "not checked", never
//! "signed".

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};

use pctwin_parse_seal::InChild;
use serde::{Deserialize, Serialize};

use crate::check::{OldCopy, OldMaker};
use crate::facts::{Arch, Format, LinuxSource, WindowsSigner};
use crate::msi::{MsiSignature, read_msi};
use crate::pe::{PeSignature, read_pe};
use crate::sniff::{HEAD, TAIL, sniff};

/// Who signed an installer, as PCTwin's own reader sees it.
#[expect(
    clippy::large_enum_variant,
    reason = "made once per check; boxing buys nothing"
)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum OwnRead {
    /// Not a kind of file PCTwin reads the signature of.
    NotRead,
    /// Read, and it carries no signature.
    None,
    /// It carries one, but it or the file is malformed: why.
    Broken(String),
    /// It carries one. `digest_matches` is None for an installer package: PCTwin's own reader
    /// does not work out a package's digest.
    Signed {
        signer: Result<WindowsSigner, String>,
        digest_matches: Option<bool>,
    },
    /// A Flatpak reference: the source it names, or why it could not be read.
    Flatpak(Result<LinuxSource, String>),
}

/// What PCTwin's own reader found in an installer: its kind, what it is built for, its signer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstallerRead {
    pub format: Format,
    pub arch: Option<Arch>,
    pub own: OwnRead,
}

/// PCTwin's own reading of an installer, asked of the parser process (`pctwin-parse`'s
/// `Parser`). `file` is held open by the caller, `size` its length; the reader reads it only
/// through the caller.
pub trait OwnReader: Sync {
    /// The kind, build and signer of an installer. Err: it could not be checked (why).
    fn installer(&self, file: &File, size: u64) -> Result<InstallerRead, String>;
    /// The maker and build of the old laptop's own copy of a Windows program or package. Err: it
    /// could not be checked (why).
    fn old_copy(&self, file: &File, size: u64) -> Result<OldCopy, String>;
}

/// The readers, run in this process: only for the parser process and this crate's own tests
/// (it takes an [`InChild`]).
#[derive(Debug, Clone, Copy)]
pub struct InThisProcess(InChild);

impl InThisProcess {
    pub fn new(seal: InChild) -> Self {
        Self(seal)
    }
}

impl OwnReader for InThisProcess {
    fn installer(&self, mut file: &File, size: u64) -> Result<InstallerRead, String> {
        read_installer(self.0, &mut file, size).map_err(|e| e.to_string())
    }

    fn old_copy(&self, mut file: &File, size: u64) -> Result<OldCopy, String> {
        Ok(read_old_copy(self.0, &mut file, size))
    }
}

/// The first and last bytes of a file of `size` bytes.
pub(crate) fn ends<R: Read + Seek>(r: &mut R, size: u64) -> io::Result<(Vec<u8>, Vec<u8>)> {
    let mut head = vec![0u8; usize::try_from(size.min(HEAD as u64)).unwrap_or(HEAD)];
    r.seek(SeekFrom::Start(0))?;
    r.read_exact(&mut head)?;
    let tail_len = size.min(TAIL as u64);
    let mut tail = vec![0u8; usize::try_from(tail_len).unwrap_or(TAIL)];
    r.seek(SeekFrom::Start(size - tail_len))?;
    r.read_exact(&mut tail)?;
    Ok((head, tail))
}

/// Reads an installer of `size` bytes: its kind from its first and last bytes, then its signer
/// (a Windows program or package) or the source it names (a Flatpak reference). Never runs it.
pub fn read_installer<R: Read + Seek>(
    _: InChild,
    r: &mut R,
    size: u64,
) -> io::Result<InstallerRead> {
    let (head, tail) = ends(r, size)?;
    let sniffed = sniff(&head, &tail);
    let mut format = sniffed.format;
    let mut arch = sniffed.arch;
    let mut own = OwnRead::NotRead;
    if sniffed.compound {
        if let Ok(m) = read_msi(&mut *r)
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
        own = match read_pe(r, size)? {
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
        own = OwnRead::Flatpak(if size > crate::linux::MAX_FLATPAKREF as u64 {
            Err("the Flatpak reference is too large".to_owned())
        } else {
            crate::linux::parse_flatpakref(&head)
        });
    }
    Ok(InstallerRead { format, arch, own })
}

/// Reads the old laptop's own copy of a Windows program or installer package of `size` bytes:
/// who made it and what it is built for, never running it.
pub fn read_old_copy<R: Read + Seek>(_: InChild, r: &mut R, size: u64) -> OldCopy {
    let not_read = |why: String| OldCopy {
        maker: OldMaker::NotRead(why),
        arch: None,
    };
    if size == 0 {
        return not_read("the old copy is empty".into());
    }
    let (head, tail) = match ends(r, size) {
        Ok(ends) => ends,
        Err(e) => return not_read(e.to_string()),
    };
    let s = sniff(&head, &tail);
    if s.compound {
        let read = match read_msi(&mut *r) {
            Ok(read) => read,
            Err(why) => return not_read(why),
        };
        let maker = match read.signature {
            MsiSignature::Signed(Ok(signer)) => OldMaker::Signed(signer),
            MsiSignature::Signed(Err(why)) | MsiSignature::Broken(why) => OldMaker::NotRead(why),
            MsiSignature::None => OldMaker::Unsigned,
        };
        return OldCopy {
            maker,
            arch: read.arch,
        };
    }
    if s.format != Format::WindowsProgram {
        return not_read("the old copy is not a Windows program".into());
    }
    let maker = match read_pe(r, size) {
        Err(e) => OldMaker::NotRead(e.to_string()),
        Ok(PeSignature::Signed {
            signer: Ok(signer),
            digest_matches: true,
        }) => OldMaker::Signed(signer),
        Ok(PeSignature::Signed {
            signer: Err(why), ..
        }) => OldMaker::NotRead(why),
        Ok(PeSignature::Signed { .. }) => {
            OldMaker::NotRead("the old copy was changed after it was signed".into())
        }
        Ok(PeSignature::None) => OldMaker::Unsigned,
        Ok(PeSignature::Broken(why)) => OldMaker::NotRead(why),
    };
    OldCopy {
        maker,
        arch: s.arch,
    }
}

/// The low-level readers, for the parser process's fuzz targets and this crate's tests only
/// (each takes an [`InChild`]).
pub mod raw {
    use super::*;

    pub use crate::msi::{MsiRead, MsiSignature};
    pub use crate::pe::PeSignature;
    pub use crate::signer::Signed;

    /// A Windows program's embedded signature ([`crate::pe::read_pe`]).
    pub fn pe_signature<R: Read + Seek>(
        _: InChild,
        r: &mut R,
        size: u64,
    ) -> io::Result<PeSignature> {
        read_pe(r, size)
    }

    /// The signer inside a PKCS #7 signature ([`crate::signer::read_signature`]).
    pub fn signature(_: InChild, bytes: &[u8]) -> Result<Signed, String> {
        crate::signer::read_signature(bytes)
    }

    /// The signer a certificate names ([`crate::signer::signer_from_certificate`]).
    pub fn signer_of_certificate(
        _: InChild,
        cert: &x509_cert::Certificate,
    ) -> Result<WindowsSigner, String> {
        crate::signer::signer_from_certificate(cert)
    }

    /// An installer package's compound file ([`crate::msi::read_msi`]).
    pub fn msi<F: Read + Seek>(_: InChild, f: F) -> Result<MsiRead, String> {
        read_msi(f)
    }

    /// A Flatpak reference's source ([`crate::linux::parse_flatpakref`]).
    pub fn flatpakref(_: InChild, bytes: &[u8]) -> Result<LinuxSource, String> {
        crate::linux::parse_flatpakref(bytes)
    }
}
