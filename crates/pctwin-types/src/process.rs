//! Starting a system tool without a window (found by the first real test, 28 October 2026: every
//! registry read and account check during the scan flashed a black command window, which looked
//! like the app glitching).
//!
//! Every program PCTwin starts goes through [`quiet`]: on Windows it carries `CREATE_NO_WINDOW`, so
//! a console tool (reg, whoami, icacls, PowerShell) never opens a window of its own; a program with
//! its own windows (an installer, an app being checked) still shows them. On other systems it is
//! `Command::new`.

use std::ffi::OsStr;
use std::process::Command;

/// Windows' `CREATE_NO_WINDOW` process creation flag.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// A command for `program` that opens no console window of its own.
pub fn quiet(program: impl AsRef<OsStr>) -> Command {
    #[allow(unused_mut)]
    let mut command = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quiet_command_still_runs_and_answers() {
        let out = if cfg!(windows) {
            quiet("cmd").args(["/c", "echo", "here"]).output()
        } else {
            quiet("sh").args(["-c", "echo here"]).output()
        }
        .unwrap();
        assert!(out.status.success());
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "here");
    }
}
