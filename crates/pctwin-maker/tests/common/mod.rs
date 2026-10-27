//! What the tests check with: the system's own checks, with Windows' own answers given the way
//! the administrator helper gives them (the maker check itself never links the crate that makes
//! the system calls; third security review, finding 8).
#![allow(dead_code)]

use pctwin_maker::NativeChecks;

#[cfg(windows)]
mod os {
    use std::ffi::OsString;
    use std::fs::File;
    use std::path::{Path, PathBuf};

    use pctwin_maker::TrustOracle;

    /// Windows' own answers, from the helper's fenced system calls.
    pub struct OsTrust;

    impl TrustOracle for OsTrust {
        fn verify_file_trust(&self, file: &File, path: &Path) -> (i32, Option<Vec<u8>>) {
            let t = pctwin_helper_os::verify_file_trust(file, path);
            (t.code, t.signer)
        }
        fn defender_scanner(&self) -> Option<PathBuf> {
            pctwin_helper_os::defender_scanner().ok()
        }
        fn minimal_env(&self) -> Option<Vec<(OsString, OsString)>> {
            pctwin_helper_os::minimal_env().ok()
        }
        fn system_dir(&self) -> Option<PathBuf> {
            pctwin_helper_os::system_folder(pctwin_helper_os::SystemFolder::Windows)
                .ok()
                .map(|w| w.join("System32"))
        }
    }
}

/// The real checks of this laptop.
pub fn os_checks() -> NativeChecks {
    #[cfg(windows)]
    {
        NativeChecks::with_oracle(Box::new(os::OsTrust))
    }
    #[cfg(not(windows))]
    {
        NativeChecks::default()
    }
}
