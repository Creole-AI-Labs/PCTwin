//! The system's own answers the maker check and the change journal need, given to them from here
//! (third security review, finding 8): those two crates never link `pctwin-helper-os`, the one
//! crate allowed to make the system's unsafe calls; the helper and the app hand them these real
//! implementations, and their tests use stand-ins.

use std::ffi::OsString;
use std::fs::File;
use std::path::{Path, PathBuf};

use pctwin_journal::place::DiskProbe;
use pctwin_maker::TrustOracle;

/// Windows' own signature check, Defender's scanner and the system's own folders.
#[derive(Debug, Default, Clone, Copy)]
pub struct OsTrust;

impl TrustOracle for OsTrust {
    fn verify_file_trust(&self, file: &File, path: &Path) -> (i32, Option<Vec<u8>>) {
        #[cfg(windows)]
        {
            let t = pctwin_helper_os::verify_file_trust(file, path);
            (t.code, t.signer)
        }
        #[cfg(not(windows))]
        {
            pctwin_maker::NoOracle.verify_file_trust(file, path)
        }
    }

    fn defender_scanner(&self) -> Option<PathBuf> {
        #[cfg(windows)]
        {
            pctwin_helper_os::defender_scanner().ok()
        }
        #[cfg(not(windows))]
        {
            None
        }
    }

    fn minimal_env(&self) -> Option<Vec<(OsString, OsString)>> {
        #[cfg(windows)]
        {
            pctwin_helper_os::minimal_env().ok()
        }
        #[cfg(not(windows))]
        {
            None
        }
    }

    fn system_dir(&self) -> Option<PathBuf> {
        #[cfg(windows)]
        {
            pctwin_helper_os::system_folder(pctwin_helper_os::SystemFolder::Windows)
                .ok()
                .map(|w| w.join("System32"))
        }
        #[cfg(not(windows))]
        {
            None
        }
    }
}

/// Whether a disk can be unplugged, as the disk itself says (Windows).
#[derive(Debug, Default, Clone, Copy)]
pub struct OsDiskProbe;

impl DiskProbe for OsDiskProbe {
    fn hotplug(&self, disk: u32) -> Option<bool> {
        #[cfg(windows)]
        {
            pctwin_helper_os::disk_hotplug(disk)
                .ok()
                .map(|h| h.device_hotplug || h.media_removable)
        }
        #[cfg(not(windows))]
        {
            let _ = disk;
            None
        }
    }
}

/// Gives the change journal its disk probe. Called once at the start of the helper and the app,
/// before any journal is opened.
pub fn install() {
    pctwin_journal::place::install_disk_probe(Box::new(OsDiskProbe));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_journal_gets_the_real_probe() {
        install();
        assert!(pctwin_journal::place::disk_probe_installed());
    }

    #[cfg(windows)]
    #[test]
    fn windows_answers_come_from_windows() {
        assert!(
            OsTrust
                .system_dir()
                .is_some_and(|d| d.join("attrib.exe").is_file())
        );
        assert!(OsTrust.minimal_env().is_some());
    }
}
