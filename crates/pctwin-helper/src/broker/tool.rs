//! The system programs the helper's adapters may start, each by its fixed place in Windows' own
//! folder (from the kernel, never from `PATH`, the current folder or anything the app sends),
//! with only the minimal environment worked out from Windows itself, in a working folder of the
//! system's, and with arguments the compiled adapter builds from typed choices.

use std::path::PathBuf;
use std::process::Command;

use super::actions::InstallScope;
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
        let mut s = Staging::new(staged.path().to_path_buf());
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
        let mut s = Staging::new(staged.path().to_path_buf());
        let digest: [u8; 32] = Sha256::digest(b"x").into();
        let h = s.admit(&mut &b"x"[..], digest).unwrap();
        let package = s.open_verified(h, digest).unwrap();
        assert_eq!(
            msiexec_install(&package, InstallScope::User).unwrap_err(),
            ToolError::NotHere
        );
    }
}
