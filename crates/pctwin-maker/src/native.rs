//! The system's own checks on this laptop, through its own tools.

use std::path::Path;

use crate::check::{Limits, SystemChecks, SystemSignature};
use crate::facts::{Format, LinuxSource, Maker, Malware, Signature};
use crate::run::{Runner, SystemRunner};

/// The real checks: Windows' signature trust and Defender, a Mac's pkgutil and Gatekeeper,
/// Linux's rpm.
pub struct NativeChecks<R: Runner = SystemRunner> {
    pub runner: R,
}

impl Default for NativeChecks {
    fn default() -> Self {
        Self {
            runner: SystemRunner,
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
    fn signature(&self, path: &Path, format: Format, limits: &Limits) -> SystemSignature {
        let t = limits.signature_timeout;
        match format {
            Format::WindowsProgram | Format::WindowsInstaller => {
                if !cfg!(windows) {
                    return not_checked("a Windows installer is checked on Windows");
                }
                let w = crate::windows::check_signature(&self.runner, path, t);
                SystemSignature {
                    signature: w.signature,
                    embedded: w.embedded,
                    maker: w.signer.map(|s| s.map(Maker::Windows)),
                    arch: None,
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
            Format::WindowsProgram | Format::WindowsInstaller if cfg!(windows) => {
                crate::windows::scan(&self.runner, path, limits.scan_timeout)
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
