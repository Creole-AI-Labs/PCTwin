//! What the maker check asks of Windows itself, handed to it from outside (third security review,
//! finding 8): the maker check never calls the system's unsafe functions itself and never links
//! the crate that does (`pctwin-helper-os`). The administrator helper gives it the real answers
//! (`pctwin_helper::os_probes::OsTrust`); tests give it stand-ins. With [`NoOracle`] every
//! Windows check is "not checked", never trusted.

use std::ffi::OsString;
use std::fs::File;
use std::path::{Path, PathBuf};

/// Windows' own answers the maker check needs.
pub trait TrustOracle: Send + Sync {
    /// Windows' own signature check (`WinVerifyTrust`) of the file held open as `file`, named
    /// `path`. Returns Windows' answer (0: trusted, otherwise its error code) and, when trusted,
    /// the signing certificate's bytes as Windows read them.
    ///
    /// For a Windows program Windows reads the bytes through the handle. For a Windows Installer
    /// package it does not: its package checker opens the file again by `path` (third review,
    /// finding 3), so that answer is about whatever `path` names at that moment. The maker check
    /// therefore counts an installer package's answer only for the administrator helper's own
    /// copy ([`crate::check::check_staged`]).
    fn verify_file_trust(&self, file: &File, path: &Path) -> (i32, Option<Vec<u8>>);
    /// Microsoft Defender's command-line scanner, where Windows itself records it.
    fn defender_scanner(&self) -> Option<PathBuf>;
    /// The few environment values a system program needs, worked out from Windows itself.
    fn minimal_env(&self) -> Option<Vec<(OsString, OsString)>>;
    /// Windows' own `System32` folder, from the kernel: the folder a system tool starts in, so it
    /// never starts in a folder of the person's own (third review, finding 7).
    fn system_dir(&self) -> Option<PathBuf>;
}

/// Knows nothing: every Windows check is "not checked".
#[derive(Debug, Default, Clone, Copy)]
pub struct NoOracle;

/// What [`NoOracle`] answers for a signature: not an answer Windows gives.
pub const NOT_ASKED: i32 = 1;

impl TrustOracle for NoOracle {
    fn verify_file_trust(&self, _file: &File, _path: &Path) -> (i32, Option<Vec<u8>>) {
        (NOT_ASKED, None)
    }
    fn defender_scanner(&self) -> Option<PathBuf> {
        None
    }
    fn minimal_env(&self) -> Option<Vec<(OsString, OsString)>> {
        None
    }
    fn system_dir(&self) -> Option<PathBuf> {
        None
    }
}
