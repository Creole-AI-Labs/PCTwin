//! The system's own checks on this laptop, through its own tools.

use std::fs::File;
use std::path::Path;

use crate::check::{Limits, SystemChecks, SystemSignature};
use crate::facts::{Format, LinuxSource, Maker, Malware, Signature};
use crate::oracle::{NoOracle, TrustOracle};
use crate::run::{Runner, SystemRunner};

/// The real checks: Windows' signature trust and Defender, a Mac's pkgutil and Gatekeeper,
/// Linux's rpm. What Windows itself answers comes through `oracle`, which the caller gives (the
/// administrator helper gives the real one; the default knows nothing, so every Windows check is
/// "not checked").
pub struct NativeChecks<R: Runner = SystemRunner> {
    pub runner: R,
    pub oracle: Box<dyn TrustOracle>,
}

impl Default for NativeChecks {
    fn default() -> Self {
        Self {
            runner: SystemRunner,
            oracle: Box::new(NoOracle),
        }
    }
}

impl NativeChecks {
    /// The real tools, with Windows' own answers from `oracle`.
    pub fn with_oracle(oracle: Box<dyn TrustOracle>) -> Self {
        Self {
            runner: SystemRunner,
            oracle,
        }
    }
}

fn not_checked(why: &str) -> SystemSignature {
    SystemSignature {
        signature: Signature::NotChecked { why: why.into() },
        embedded: false,
        maker: None,
        arch: None,
    }
}

impl<R: Runner> SystemChecks for NativeChecks<R> {
    fn signature(
        &self,
        file: &File,
        path: &Path,
        format: Format,
        limits: &Limits,
    ) -> SystemSignature {
        let t = limits.signature_timeout;
        match format {
            Format::WindowsProgram | Format::WindowsInstaller => {
                #[cfg(windows)]
                {
                    let w = crate::windows::check_signature(&*self.oracle, file, path);
                    SystemSignature {
                        signature: w.signature,
                        embedded: w.embedded,
                        maker: w.signer.map(|s| s.map(Maker::Windows)),
                        arch: None,
                    }
                }
                #[cfg(not(windows))]
                {
                    let _ = file;
                    not_checked("a Windows installer is checked on Windows")
                }
            }
            Format::MacInstaller => {
                if !cfg!(target_os = "macos") {
                    return not_checked("a Mac installer is checked on a Mac");
                }
                let f = crate::mac::check_pkg(&self.runner, path, t);
                SystemSignature {
                    signature: f.signature,
                    embedded: true,
                    maker: Some(
                        f.team
                            .map(Maker::Mac)
                            .ok_or_else(|| "the package names no Apple Team ID".to_owned()),
                    ),
                    arch: f.arch,
                }
            }
            Format::RpmPackage => {
                if !cfg!(target_os = "linux") {
                    return not_checked("an RPM package is checked on Linux");
                }
                let (c, arch) = crate::linux::check_rpm(&self.runner, path, t);
                SystemSignature {
                    signature: c.signature,
                    embedded: true,
                    maker: Some(
                        c.key
                            .map(|key| Maker::Linux(LinuxSource::Rpm { key }))
                            .ok_or_else(|| "the package names no signing key".to_owned()),
                    ),
                    arch,
                }
            }
            Format::DebPackage | Format::LinuxProgram => SystemSignature {
                signature: Signature::Missing,
                embedded: false,
                maker: None,
                arch: None,
            },
            Format::MacDiskImage => {
                not_checked("a disk image is checked through the app inside it, once it is opened")
            }
            _ => not_checked("PCTwin does not check this kind of file"),
        }
    }

    fn malware(
        &self,
        path: &Path,
        format: Format,
        signature: &Signature,
        limits: &Limits,
    ) -> Malware {
        match format {
            #[cfg(windows)]
            Format::WindowsProgram | Format::WindowsInstaller => {
                crate::windows::scan(&self.runner, &*self.oracle, path, limits.scan_timeout)
            }
            Format::MacInstaller if cfg!(target_os = "macos") => {
                let call = crate::mac::gatekeeper_call(path, true, limits.signature_timeout);
                crate::mac::malware_from(&crate::mac::parse_gatekeeper(&self.runner.run(&call)))
            }
            Format::RpmPackage if cfg!(target_os = "linux") => {
                crate::linux::malware_from(signature)
            }
            _ => Malware::Unavailable {
                why: "this system's malware check cannot look at this kind of file".into(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::{Call, Ran, RunError};

    /// Answers every call with the same output.
    struct Always(String);
    impl Runner for Always {
        fn run(&self, _: &Call) -> Result<Ran, RunError> {
            Ok(Ran {
                code: Some(0),
                stdout: self.0.clone(),
                stderr: String::new(),
            })
        }
    }

    fn native(out: &str) -> NativeChecks<Always> {
        NativeChecks {
            runner: Always(out.into()),
            oracle: Box::new(NoOracle),
        }
    }

    const P: &str = "/x/setup";

    /// An open file to ask about (its bytes do not matter to these stand-in tools).
    fn file() -> File {
        tempfile::tempfile().unwrap()
    }

    /// Windows itself is asked, about the open file, whatever any tool would say: a file with no
    /// signature is never trusted (second review, finding B3).
    #[test]
    fn windows_files_are_asked_of_windows_itself_only_on_windows() {
        let json = r#"{"status":"Valid","kind":"Authenticode","message":"","signer":""}"#;
        let mut f = file();
        std::io::Write::write_all(&mut f, b"MZ not really a program").unwrap();
        let s = native(json).signature(
            &f,
            Path::new(P),
            Format::WindowsInstaller,
            &Limits::default(),
        );
        assert_ne!(s.signature, Signature::Trusted);
        // With no answers from Windows given, nothing is checked (third review, finding 8).
        assert!(matches!(s.signature, Signature::NotChecked { .. }), "{s:?}");
        if !cfg!(windows) {
            assert!(matches!(s.signature, Signature::NotChecked { .. }));
            assert!(!s.embedded);
        }
    }

    #[test]
    fn packages_without_a_checkable_signature() {
        let n = native("");
        for f in [Format::DebPackage, Format::LinuxProgram] {
            let s = n.signature(&file(), Path::new(P), f, &Limits::default());
            assert_eq!(s.signature, Signature::Missing);
            assert!(!s.embedded);
            assert!(s.maker.is_none());
        }
        for f in [Format::MacDiskImage, Format::Zip, Format::Unknown] {
            let s = n.signature(&file(), Path::new(P), f, &Limits::default());
            assert!(matches!(s.signature, Signature::NotChecked { .. }));
            assert!(!s.embedded);
        }
    }

    #[test]
    fn rpm_is_asked_of_linux_only_on_linux() {
        let ok = "x.rpm:\n    Header V4 RSA/SHA256 Signature, key ID 24901d0c: OK\n    Header SHA256 digest: OK\n";
        let n = native(ok);
        let s = n.signature(
            &file(),
            Path::new(P),
            Format::RpmPackage,
            &Limits::default(),
        );
        let m = n.malware(
            Path::new(P),
            Format::RpmPackage,
            &s.signature,
            &Limits::default(),
        );
        if cfg!(target_os = "linux") {
            assert_eq!(s.signature, Signature::Trusted);
            assert!(s.embedded);
            assert!(matches!(m, Malware::CoveredBySystemTrust { .. }));
        } else {
            assert!(matches!(s.signature, Signature::NotChecked { .. }));
            assert!(matches!(m, Malware::Unavailable { .. }));
        }
    }

    #[test]
    fn mac_packages_are_asked_of_a_mac_only_on_a_mac() {
        // Output that pkgutil and spctl would both read as signed, notarised and accepted.
        let yes = "/x.pkg: accepted\nsource=Notarized Developer ID\n   Status: signed by a developer certificate issued by Apple for distribution\n    1. Developer ID Installer: Example Ltd (ABCDE12345)\n";
        let n = native(yes);
        let s = n.signature(
            &file(),
            Path::new(P),
            Format::MacInstaller,
            &Limits::default(),
        );
        let m = n.malware(
            Path::new(P),
            Format::MacInstaller,
            &s.signature,
            &Limits::default(),
        );
        if cfg!(target_os = "macos") {
            assert_eq!(s.signature, Signature::Trusted);
            assert!(s.embedded);
            assert!(matches!(m, Malware::CoveredBySystemTrust { .. }));
        } else {
            assert!(matches!(s.signature, Signature::NotChecked { .. }));
            assert!(matches!(m, Malware::Unavailable { .. }));
        }
    }

    #[test]
    fn other_kinds_have_no_malware_check() {
        let m = native("").malware(
            Path::new(P),
            Format::Zip,
            &Signature::Trusted,
            &Limits::default(),
        );
        assert!(matches!(m, Malware::Unavailable { .. }));
    }
}
