//! The maker check of a package about to be installed with an administrator's rights, made by the
//! helper itself on its own copy (second security review, findings B2 and B3, 27 October 2026).
//!
//! The person's app may have checked the download already, but its answer is never used for an
//! elevated install: anything running as the person could have changed that answer, the file it
//! looked at, or the code that answered. So the helper checks again, here, on the very file the
//! installer will be given:
//!
//! - the package is the helper's own copy, in its administrators-only folder, opened without
//!   following a link and held so nobody can write, rename or delete it until the installer has
//!   finished ([`super::staging::VerifiedArtifact`]);
//! - the maker check reads and hashes that file, asks Windows itself (not PowerShell) whether it
//!   trusts the signature in the bytes it holds, and has Microsoft Defender scan it
//!   ([`pctwin_maker::check_staged`]: Windows reads an installer package by its name, so only this
//!   copy, which nobody else can change, is checked that way; third review, finding 3);
//! - only "install" for exactly the approved bytes, by the old laptop's maker named in the
//!   approved plan, lets the install go on.
//!
//! What this does not prove: the old laptop's maker comes from the plan the app sealed (see
//! [`super::LaunchSeal`]); the check stops a changed or swapped package and a forged answer, not a
//! plan that names another maker.

use pctwin_maker::facts::{Arch, Decision, Format, Maker, Sha256, System};
use pctwin_maker::{Expected, Limits, NativeChecks, check_staged};

use super::actions::{InstallVerifiedPackage, PackageKind};
use super::staging::VerifiedArtifact;

/// What the helper's own maker check said.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MakerCheck {
    /// Every fact holds, for exactly the approved bytes.
    Approved,
    /// Not installed by this route.
    Refused,
}

/// This laptop's own chip, as the maker check names it (`None`: the system would not say, or
/// PCTwin does not know it).
pub fn laptop_arch() -> Option<Arch> {
    use pctwin_helper_os::chip::{Chip, native_chip};
    match native_chip()?.native {
        Chip::X86_64 => Some(Arch::X64),
        Chip::Arm64 => Some(Arch::Arm64),
        Chip::X86 => Some(Arch::X86),
        Chip::Other => None,
    }
}

/// What the approved action says the package must be.
pub fn expected_of(action: &InstallVerifiedPackage, arch: Arch) -> Expected {
    Expected {
        system: System::Windows,
        maker: Maker::Windows((*action.maker).clone()),
        formats: match action.kind {
            PackageKind::Msi => vec![Format::WindowsInstaller],
        },
        arch,
        bytes: Some(Sha256(action.sha256.0)),
    }
}

/// The helper's own maker check of `package` for `action`.
pub fn check_package(
    package: &VerifiedArtifact,
    action: &InstallVerifiedPackage,
    limits: &Limits,
) -> MakerCheck {
    let Some(arch) = laptop_arch() else {
        return MakerCheck::Refused;
    };
    let checks = NativeChecks::with_oracle(Box::new(crate::os_probes::OsTrust));
    let Ok(verdict) = check_staged(package, &expected_of(action, arch), &checks, limits) else {
        return MakerCheck::Refused;
    };
    match verdict.decide() {
        Decision::Install { bytes }
            if bytes.sha256.0 == action.sha256.0 && bytes.sha256.0 == package.sha256() =>
        {
            MakerCheck::Approved
        }
        _ => MakerCheck::Refused,
    }
}
