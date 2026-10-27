//! The system programs the helper's adapters may start, each by its fixed place in Windows' own
//! folder (from the kernel, never from `PATH`, the current folder or anything the app sends),
//! with only the minimal environment worked out from Windows itself, in a working folder of the
//! system's, and with arguments the compiled adapter builds from typed choices.

use std::path::PathBuf;
use std::process::Command;

use super::actions::{ExeKind, InstallScope};
use super::staging::VerifiedArtifact;

/// A system program an adapter may start.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrustedTool {
    /// Windows Installer, `System32\msiexec.exe`.
    Msiexec,
}

/// Why a program could not be prepared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolError {
    /// Not on this system.
    NotHere,
    /// Windows' own folder could not be found.
    NoSystemFolder,
    /// The program is not a plain file where Windows keeps it.
    Missing,
}

impl TrustedTool {
    /// Where it is: Windows' own `System32`, as the kernel names it.
    #[cfg(windows)]
    pub fn path(self) -> Result<PathBuf, ToolError> {
        let windows = pctwin_helper_os::system_folder(pctwin_helper_os::SystemFolder::Windows)
            .map_err(|_| ToolError::NoSystemFolder)?;
        let name = match self {
            TrustedTool::Msiexec => "msiexec.exe",
        };
        let path = windows.join("System32").join(name);
        let meta = std::fs::symlink_metadata(&path).map_err(|_| ToolError::Missing)?;
        if !meta.is_file() {
            return Err(ToolError::Missing);
        }
        Ok(path)
    }

    #[cfg(not(windows))]
    pub fn path(self) -> Result<PathBuf, ToolError> {
        Err(ToolError::NotHere)
    }

    /// The program, ready to start: its full path, only the minimal environment, and
    /// `System32` (which only administrators can change) as its working folder. No arguments
    /// yet: each adapter adds its own fixed ones.
    #[cfg(windows)]
    pub fn command(self) -> Result<Command, ToolError> {
        let path = self.path()?;
        let mut command =
            pctwin_helper_os::clean_command(&path).map_err(|_| ToolError::NoSystemFolder)?;
        if let Some(folder) = path.parent() {
            command.current_dir(folder);
        }
        Ok(command)
    }

    #[cfg(not(windows))]
    pub fn command(self) -> Result<Command, ToolError> {
        Err(ToolError::NotHere)
    }
}

/// Windows Installer's command for a checked package held open: install it for everyone,
/// quietly, without restarting the laptop (the result says when a restart is needed). Only for
/// [`InstallScope::Machine`]; a package for the person alone is never installed by the helper.
pub fn msiexec_install(
    package: &VerifiedArtifact,
    scope: InstallScope,
) -> Result<Command, ToolError> {
    if scope != InstallScope::Machine {
        return Err(ToolError::NotHere);
    }
    let mut command = TrustedTool::Msiexec.command()?;
    command
        .arg("/i")
        .arg(package.path())
        .args(["/qn", "/norestart", "ALLUSERS=1"]);
    Ok(command)
}

/// The fixed switches for an installer program of `kind`: quiet, for everyone, never restarting
/// the laptop, and saying when a restart is needed (exit code 3010). Documented by the makers:
/// Inno Setup's "Setup Command Line Parameters" (`/RESTARTEXITCODE` sets the code for "restart
/// needed", 0 by default); WiX Burn's standard bundle switches. Nothing here comes from a
/// manifest, a recipe or a website.
pub fn exe_switches(kind: ExeKind) -> &'static [&'static str] {
    match kind {
        ExeKind::Inno => &[
            "/VERYSILENT",
            "/SUPPRESSMSGBOXES",
            "/NORESTART",
            "/RESTARTEXITCODE=3010",
            "/SP-",
            "/ALLUSERS",
        ],
        ExeKind::Burn => &["/quiet", "/norestart"],
    }
}

/// An installer program's command: the helper's own held copy, started by its full path with
/// only the minimal environment, in the helper's administrators-only folder, with
/// [`exe_switches`]. Only for [`InstallScope::Machine`].
pub fn exe_install(
    package: &VerifiedArtifact,
    kind: ExeKind,
    scope: InstallScope,
) -> Result<Command, ToolError> {
    if scope != InstallScope::Machine {
        return Err(ToolError::NotHere);
    }
    #[cfg(windows)]
    {
        let mut command = pctwin_helper_os::clean_command(package.path())
            .map_err(|_| ToolError::NoSystemFolder)?;
        if let Some(folder) = package.path().parent() {
            command.current_dir(folder);
        }
        command.args(exe_switches(kind));
        Ok(command)
    }
    #[cfg(not(windows))]
    {
        let _ = (package, kind);
        Err(ToolError::NotHere)
    }
}

/// What an installer's exit code means, by the kind of installer: done (with or without a
/// restart needed), another install running (wait and try again: T21-039), or failed. Windows
/// Installer's codes are documented ("MsiExec.exe and InstMsi.exe error messages"); a Burn bundle
/// passes them on; Inno Setup returns 0 or, with `/RESTARTEXITCODE`, 3010.
pub fn meaning(kind: Option<ExeKind>, code: i32) -> ExitMeaning {
    match (kind, code) {
        (_, 0) => ExitMeaning::Done {
            restart_needed: false,
        },
        (_, 3010) => ExitMeaning::Done {
            restart_needed: true,
        },
        // Restart started: never asked for (/norestart), so it is said, not hidden.
        (None | Some(ExeKind::Burn), 1641) => ExitMeaning::Done {
            restart_needed: true,
        },
        (None | Some(ExeKind::Burn), 1618) => ExitMeaning::Busy,
        (None | Some(ExeKind::Burn), 1602) | (Some(ExeKind::Inno), 2 | 5) => ExitMeaning::Cancelled,
        _ => ExitMeaning::Failed,
    }
}

/// See [`meaning`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitMeaning {
    Done {
        restart_needed: bool,
    },
    /// Another install was running; nothing was changed.
    Busy,
    /// The installer was cancelled (by a person at its own window).
    Cancelled,
    Failed,
}

#[cfg(test)]
mod meaning_tests {
    use super::*;

    #[test]
    fn exit_codes_mean_what_their_makers_document() {
        assert_eq!(
            meaning(None, 0),
            ExitMeaning::Done {
                restart_needed: false
            }
        );
        assert_eq!(
            meaning(None, 3010),
            ExitMeaning::Done {
                restart_needed: true
            }
        );
        assert_eq!(
            meaning(None, 1641),
            ExitMeaning::Done {
                restart_needed: true
            }
        );
        assert_eq!(meaning(None, 1618), ExitMeaning::Busy);
        assert_eq!(meaning(None, 1602), ExitMeaning::Cancelled);
        assert_eq!(meaning(None, 1603), ExitMeaning::Failed);
        assert_eq!(
            meaning(Some(ExeKind::Inno), 3010),
            ExitMeaning::Done {
                restart_needed: true
            }
        );
        // Inno Setup's own codes are not Windows Installer's.
        assert_eq!(meaning(Some(ExeKind::Inno), 1641), ExitMeaning::Failed);
        assert_eq!(meaning(Some(ExeKind::Inno), 1618), ExitMeaning::Failed);
        assert_eq!(meaning(Some(ExeKind::Inno), 5), ExitMeaning::Cancelled);
        assert_eq!(meaning(Some(ExeKind::Burn), 1618), ExitMeaning::Busy);
    }

    #[test]
    fn installer_switches_never_restart_and_never_come_from_outside() {
        for kind in [ExeKind::Inno, ExeKind::Burn] {
            let s = exe_switches(kind);
            assert!(
                s.iter().any(|a| a.eq_ignore_ascii_case("/norestart")),
                "{kind:?}"
            );
        }
        assert!(exe_switches(ExeKind::Inno).contains(&"/RESTARTEXITCODE=3010"));
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use crate::broker::staging::Staging;
    use sha2::{Digest, Sha256};

    /// T22-015: a rogue `msiexec.exe` in the current folder and first on `PATH` is never the one
    /// started; the environment is Windows' minimal one, never the starter's.
    #[test]
    fn a_rogue_tool_on_path_or_in_the_current_folder_is_never_used() {
        let rogue = tempfile::tempdir().unwrap();
        std::fs::write(rogue.path().join("msiexec.exe"), b"MZ rogue").unwrap();
        let staged = tempfile::tempdir().unwrap();
        let mut s = Staging::new(crate::broker::staging::HelperFolder::for_tests(
            staged.path().to_path_buf(),
        ));
        let bytes = b"msi".to_vec();
        let digest: [u8; 32] = Sha256::digest(&bytes).into();
        let h = s.admit(&mut bytes.as_slice(), digest).unwrap();
        let package = s.open_verified(h, digest).unwrap();

        // Whatever this process's PATH and current folder hold (setting them here would race the
        // other tests), the command names Windows' own msiexec by its full path from the kernel,
        // passes on none of this process's environment, and works in System32.
        let command = msiexec_install(&package, InstallScope::Machine).unwrap();
        let program = PathBuf::from(command.get_program());
        assert!(program.is_absolute(), "{program:?}");
        assert!(program.ends_with(r"System32\msiexec.exe"), "{program:?}");
        assert!(!program.starts_with(rogue.path()));
        let windows =
            pctwin_helper_os::system_folder(pctwin_helper_os::SystemFolder::Windows).unwrap();
        assert_eq!(program, windows.join("System32").join("msiexec.exe"));
        assert_eq!(
            command.get_current_dir(),
            Some(windows.join("System32").as_path())
        );
        let envs: Vec<(String, Option<String>)> = command
            .get_envs()
            .map(|(k, v)| {
                (
                    k.to_string_lossy().into_owned(),
                    v.map(|v| v.to_string_lossy().into_owned()),
                )
            })
            .collect();
        let names: Vec<&str> = envs.iter().map(|(k, _)| k.as_str()).collect();
        for name in [
            "SystemRoot",
            "windir",
            "SystemDrive",
            "ComSpec",
            "PATH",
            "TEMP",
            "TMP",
        ] {
            assert!(names.iter().any(|n| n.eq_ignore_ascii_case(name)), "{name}");
        }
        assert_eq!(names.len(), 7, "{names:?}");
        let path = envs
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("PATH"))
            .and_then(|(_, v)| v.clone())
            .unwrap();
        assert!(!path.contains(&*rogue.path().to_string_lossy()));
        let args: Vec<String> = command
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(args[0], "/i");
        assert_eq!(PathBuf::from(&args[1]), package.path());
        assert_eq!(&args[2..], ["/qn", "/norestart", "ALLUSERS=1"]);
    }

    #[test]
    fn a_package_for_the_person_alone_is_never_given_to_msiexec() {
        let staged = tempfile::tempdir().unwrap();
        let mut s = Staging::new(crate::broker::staging::HelperFolder::for_tests(
            staged.path().to_path_buf(),
        ));
        let digest: [u8; 32] = Sha256::digest(b"x").into();
        let h = s.admit(&mut &b"x"[..], digest).unwrap();
        let package = s.open_verified(h, digest).unwrap();
        assert_eq!(
            msiexec_install(&package, InstallScope::User).unwrap_err(),
            ToolError::NotHere
        );
    }
}
