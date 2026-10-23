//! macOS: the administrator's approval for one move (Security Design part C, "The administrator's
//! approval on a Mac"). Switching the launch daemon on approves nothing; for each move the app
//! asks macOS for PCTwin's own right, [`MOVE_RIGHT`], which needs an administrator to type their
//! password in macOS's own window, every time. The app hands the helper the approval's external
//! form ([`Approval`]); the helper asks macOS, showing nothing, whether it holds the right.
//!
//! - The app: [`ask_administrator`] (the safe `security-framework` wrapper).
//! - The helper, as root: [`define_move_right`] (Apple's `security authorizationdb` tool) and
//!   [`approval_holds`] (the one fenced call, `os_authorization_holds` in `src/mac.rs`).
//!
//! Elsewhere these answer [`Error::NotSupported`]: Windows and Linux ask through UAC and polkit.

use crate::Error;

/// PCTwin's right in macOS's authorization database.
pub const MOVE_RIGHT: &str = "com.creoleailabs.pctwin.move-everyone";

/// How long an approval holds inside the one request it was given for, so the helper can check
/// it moments later without asking again. Never shared with any other request.
pub const HOLDS_FOR_SECS: u32 = 120;

/// The right's definition: an administrator authenticates; the approval is not shared with
/// other requests and lasts [`HOLDS_FOR_SECS`] within its own.
pub fn move_right_definition() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>class</key>
	<string>user</string>
	<key>group</key>
	<string>admin</string>
	<key>authenticate-user</key>
	<true/>
	<key>allow-root</key>
	<false/>
	<key>shared</key>
	<false/>
	<key>timeout</key>
	<integer>{HOLDS_FOR_SECS}</integer>
	<key>comment</key>
	<string>An administrator of this Mac approves PCTwin reading the files of the people named, for one move.</string>
</dict>
</plist>
"#
    )
}

/// An approval's external form: 32 opaque bytes macOS made, meaningful only on this Mac.
#[derive(Clone, PartialEq, Eq)]
pub struct Approval(pub [u8; 32]);

impl std::fmt::Debug for Approval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Approval(..)")
    }
}

/// The app: asks an administrator, in macOS's own window showing `prompt`, to approve this move.
/// `Ok(None)` when they cancel or are not an administrator.
pub fn ask_administrator(prompt: &str) -> Result<Option<Approval>, Error> {
    #[cfg(target_os = "macos")]
    {
        crate::mac::ask_administrator(prompt)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = prompt;
        Err(Error::NotSupported("this approval is macOS only"))
    }
}

/// The helper, as root: writes [`MOVE_RIGHT`]'s definition into the authorization database
/// (each start, so it is always PCTwin's own).
pub fn define_move_right() -> Result<(), Error> {
    #[cfg(target_os = "macos")]
    {
        crate::mac::define_right(MOVE_RIGHT, &move_right_definition())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err(Error::NotSupported("this approval is macOS only"))
    }
}

/// The helper: whether `approval` holds [`MOVE_RIGHT`] now, asked of macOS with nothing shown.
pub fn approval_holds(approval: &Approval) -> Result<bool, Error> {
    approval_holds_for(approval, MOVE_RIGHT)
}

/// [`approval_holds`] for any right (CI checks the call with a right of its own).
pub fn approval_holds_for(approval: &Approval, right: &str) -> Result<bool, Error> {
    #[cfg(target_os = "macos")]
    {
        crate::mac::approval_holds(approval, right)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (approval, right);
        Err(Error::NotSupported("this approval is macOS only"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_right_needs_an_administrator_every_time_and_is_never_shared() {
        let d = move_right_definition();
        for needed in [
            "<key>group</key>\n\t<string>admin</string>",
            "<key>authenticate-user</key>\n\t<true/>",
            "<key>allow-root</key>\n\t<false/>",
            "<key>shared</key>\n\t<false/>",
            "<key>timeout</key>\n\t<integer>120</integer>",
        ] {
            assert!(d.contains(needed), "{needed}");
        }
        assert!(MOVE_RIGHT.starts_with("com.creoleailabs.pctwin."));
    }

    #[test]
    fn an_approval_never_shows_its_bytes() {
        assert_eq!(format!("{:?}", Approval([7; 32])), "Approval(..)");
    }
}
