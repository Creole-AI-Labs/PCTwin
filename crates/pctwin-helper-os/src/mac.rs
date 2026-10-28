//! macOS: who is calling on the helper's socket, and switching the helper's launch daemon on and
//! off through SMAppService.
//!
//! The four `os_daemon_*` / `os_open_login_items` functions, `os_od_create_user` and
//! `os_authorization_holds` are the only unsafe code here, one block each, listed in
//! scripts/unsafe-fence.sh: the Service Management and Open Directory bindings mark every method
//! unsafe, and the security-framework wrapper has no way to check a right on an approval made
//! elsewhere. Reading the caller and asking an administrator need none (nix, libproc and
//! security-framework wrap them safely).

use crate::daemon::{DaemonService, DaemonState};
use crate::{Account, Caller, Error};
use nix::sys::socket::{getsockopt, sockopt};
use objc2::runtime::AnyObject;
use objc2_foundation::NSString;
use objc2_open_directory::{
    ODNode, ODSession, kODAttributeTypeFullName, kODAttributeTypeNFSHomeDirectory,
    kODAttributeTypePrimaryGroupID, kODAttributeTypeUniqueID, kODAttributeTypeUserShell,
    kODNodeTypeLocalNodes, kODRecordTypeUsers,
};
use objc2_service_management::{SMAppService, SMAppServiceStatus};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub(crate) fn peer(stream: &UnixStream) -> Result<Caller, Error> {
    let errno = |e: nix::errno::Errno| Error::Io(std::io::Error::from_raw_os_error(e as i32));
    let pid = getsockopt(stream, sockopt::LocalPeerPid).map_err(errno)?;
    let cred = getsockopt(stream, sockopt::LocalPeerCred).map_err(errno)?;
    let process_id = u32::try_from(pid).map_err(|_| Error::CallerGone)?;
    let program = libproc::proc_pid::pidpath(pid).map_err(|_| Error::CallerGone)?;
    Ok(Caller {
        process_id,
        account: Account::Uid(cred.uid()),
        program_hint: PathBuf::from(program),
    })
}

/// Registers the launch daemon whose property list the app carries in
/// `Contents/Library/LaunchDaemons/<plist>`. Errors come back as the `NSError` code and text.
fn os_daemon_register(plist: &str) -> Result<(), (i64, String)> {
    let name = NSString::from_str(plist);
    // SAFETY: `name` is a valid NSString for the call; the service object is retained by the
    // binding and released on drop; the error, if any, is a retained NSError read only here.
    #[expect(
        unsafe_code,
        reason = "the SMAppService bindings mark every method unsafe"
    )]
    let r = unsafe { SMAppService::daemonServiceWithPlistName(&name).registerAndReturnError() };
    r.map_err(|e| (e.code() as i64, e.localizedDescription().to_string()))
}

/// Unregisters it (the move has ended).
fn os_daemon_unregister(plist: &str) -> Result<(), (i64, String)> {
    let name = NSString::from_str(plist);
    // SAFETY: as in os_daemon_register.
    #[expect(
        unsafe_code,
        reason = "the SMAppService bindings mark every method unsafe"
    )]
    let r = unsafe { SMAppService::daemonServiceWithPlistName(&name).unregisterAndReturnError() };
    r.map_err(|e| (e.code() as i64, e.localizedDescription().to_string()))
}

/// Its state: 0 not registered, 1 enabled, 2 needs an administrator's approval, 3 not found.
fn os_daemon_status(plist: &str) -> isize {
    let name = NSString::from_str(plist);
    // SAFETY: `name` is a valid NSString for the call; `status` only reads.
    #[expect(
        unsafe_code,
        reason = "the SMAppService bindings mark every method unsafe"
    )]
    let r = unsafe { SMAppService::daemonServiceWithPlistName(&name).status() };
    r.0
}

/// Opens System Settings at Login Items, where an administrator allows the daemon.
fn os_open_login_items() {
    // SAFETY: a class method with no arguments and no result.
    #[expect(
        unsafe_code,
        reason = "the SMAppService bindings mark every method unsafe"
    )]
    unsafe {
        SMAppService::openSystemSettingsLoginItems()
    };
}

/// Makes the user record `name` in the local directory with its attributes and sets its
/// `password` in memory (never on a command line). Errors come back as the step and the
/// `NSError` code.
fn os_od_create_user(
    name: &str,
    full_name: &str,
    uid: u32,
    password: &str,
) -> Result<(), (&'static str, i64)> {
    let record_name = NSString::from_str(name);
    let values = [
        (1, NSString::from_str(full_name)),
        (2, NSString::from_str(&uid.to_string())),
        // staff: every person's group on a Mac.
        (3, NSString::from_str("20")),
        (4, NSString::from_str("/bin/zsh")),
        (5, NSString::from_str(&format!("/Users/{name}"))),
    ];
    let new_password = NSString::from_str(password);
    // SAFETY: every NSString lives for the whole block; the directory constants are statics
    // Open Directory provides for the life of the process; every object the calls return is
    // retained by the binding and released on drop; each error is a retained NSError read only
    // here. Nothing is passed that Open Directory documents as not allowing nil, except the
    // attributes of the new record, which it documents as optional.
    #[expect(
        unsafe_code,
        reason = "the Open Directory bindings mark every method unsafe"
    )]
    let r = unsafe {
        let code = |e: Option<objc2::rc::Retained<objc2_foundation::NSError>>| {
            e.map_or(-1, |e| e.code() as i64)
        };
        let session = ODSession::defaultSession().ok_or(("session", -1))?;
        let mut error = None;
        let node = ODNode::nodeWithSession_type_error(
            Some(&session),
            kODNodeTypeLocalNodes,
            Some(&mut error),
        )
        .ok_or_else(|| ("node", code(error.take())))?;
        let record = node
            .createRecordWithRecordType_name_attributes_error(
                kODRecordTypeUsers,
                Some(&record_name),
                None,
                Some(&mut error),
            )
            .ok_or_else(|| ("record", code(error.take())))?;
        for (which, value) in &values {
            let attribute = match which {
                1 => kODAttributeTypeFullName,
                2 => kODAttributeTypeUniqueID,
                3 => kODAttributeTypePrimaryGroupID,
                4 => kODAttributeTypeUserShell,
                _ => kODAttributeTypeNFSHomeDirectory,
            };
            let any: &AnyObject = value.as_ref();
            if !record.setValue_forAttribute_error(Some(any), attribute, Some(&mut error)) {
                return Err(("attribute", code(error.take())));
            }
        }
        if !record.changePassword_toPassword_error(None, Some(&new_password), Some(&mut error)) {
            return Err(("password", code(error.take())));
        }
        if !record.synchronizeAndReturnError(Some(&mut error)) {
            return Err(("save", code(error.take())));
        }
        Ok(())
    };
    r
}

/// The system's tools by their full paths, never looked up on the search path.
const DSCL: &str = "/usr/bin/dscl";
const CREATEHOMEDIR: &str = "/usr/sbin/createhomedir";
const PWPOLICY: &str = "/usr/bin/pwpolicy";

fn run(program: &str, args: &[&str]) -> Result<Vec<u8>, Error> {
    let out = pctwin_types::process::quiet(program)
        .args(args)
        .stdin(Stdio::null())
        .output()?;
    if out.status.success() {
        Ok(out.stdout)
    } else {
        Err(Error::Os {
            call: "a directory tool",
            code: i64::from(out.status.code().unwrap_or(-1)),
        })
    }
}

/// The next user ID for a person: one more than the highest from 501 up.
fn next_uid() -> Result<u32, Error> {
    let listed =
        String::from_utf8_lossy(&run(DSCL, &[".", "-list", "/Users", "UniqueID"])?).into_owned();
    let highest = listed
        .lines()
        .filter_map(|l| l.split_whitespace().last()?.parse::<u32>().ok())
        .filter(|uid| (501..60_000).contains(uid))
        .max()
        .unwrap_or(500);
    Ok(highest + 1)
}

/// macOS: a standard account in the local directory (Open Directory, the password in memory
/// only), its home folder (`createhomedir`), then marked to be changed at the first sign-in
/// (`pwpolicy newPasswordRequired`). Its Secure Token, so it can unlock FileVault, is given
/// later through the administrator's own password in the Mac's own window. Returns its user ID.
pub(crate) fn add_standard_account(
    name: &str,
    full_name: &str,
    password: &str,
) -> Result<u32, Error> {
    let path = format!("/Users/{name}");
    if run(DSCL, &[".", "-read", &path, "UniqueID"]).is_ok() {
        return Err(Error::AlreadyExists);
    }
    let uid = next_uid()?;
    let home_was_there = std::path::Path::new(&path).exists();
    if let Err((call, code)) = os_od_create_user(name, full_name, uid, password) {
        // A step after the record was made failed: the record is PCTwin's own (none was at the
        // name, and making one at a taken name fails), taken away again even if its user ID was
        // not set yet.
        if crate::od_step_left_a_record(call) {
            let _ = remove_account(name, uid, home_was_there, true);
        }
        return Err(Error::Os {
            call: match call {
                "password" => "Open Directory: setting the password",
                "record" => "Open Directory: making the account",
                _ => "Open Directory",
            },
            code,
        });
    }
    crate::finish_or_undo(
        uid,
        || {
            run(CREATEHOMEDIR, &["-c", "-u", name])?;
            run(
                PWPOLICY,
                &["-u", name, "-setpolicy", "newPasswordRequired=1"],
            )?;
            Ok(())
        },
        |made| remove_account(name, made, home_was_there, false),
    )
}

/// Takes away the account `name` PCTwin just made: only if its record still has the user ID
/// `uid` it was made with, or (`unset_is_ours`: made in this very call, failing before its user
/// ID was set) has none, never another account of that name; and its home folder only if
/// PCTwin's own step made it (it was not there before, and it is that account's).
fn remove_account(
    name: &str,
    uid: u32,
    home_was_there: bool,
    unset_is_ours: bool,
) -> Result<(), Error> {
    use std::os::unix::fs::MetadataExt;
    let path = format!("/Users/{name}");
    let record = String::from_utf8_lossy(&run(DSCL, &[".", "-read", &path])?).into_owned();
    if !crate::is_the_made_account(crate::listed_uid(&record), uid, unset_is_ours) {
        return Err(Error::BadInput("the account is not the one PCTwin made"));
    }
    run(DSCL, &[".", "-delete", &path])?;
    if std::fs::symlink_metadata(&path)
        .is_ok_and(|m| crate::is_the_made_home(home_was_there, m.is_dir(), m.uid(), uid))
    {
        std::fs::remove_dir_all(&path)?;
    }
    Ok(())
}

const SYSADMINCTL: &str = "/usr/sbin/sysadminctl";

/// Gives the account `name` a Secure Token, so it can unlock FileVault: the administrator
/// approves in the Mac's own window (`sysadminctl interactive`), and the account's `password`
/// goes to the tool's own prompt, never on a command line. Then the token is checked: `true` only
/// if the system says it is on (a prompt answered wrongly is never taken as done).
pub(crate) fn secure_token_on(name: &str, password: &str) -> Result<bool, Error> {
    use std::io::Write;
    // With FileVault off there is nothing to unlock: no window is shown, and the token is as the
    // system says. With it on, the window waits at most a few minutes for the administrator, so
    // a Mac nobody is watching (or a CI runner) never holds the move on it for ever.
    if !filevault_on() {
        return secure_token_status(name);
    }
    // Ended (with anything it started) and reaped whatever happens below, an error included.
    let mut tool = crate::ToolRun::spawn(
        pctwin_types::process::quiet(SYSADMINCTL)
            .args(["interactive", "-secureTokenOn", name, "-password", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null()),
    )?;
    if let Some(mut stdin) = tool.stdin() {
        let mut line = format!("{password}\n").into_bytes();
        let written = stdin.write_all(&line);
        line.iter_mut().for_each(|b| *b = 0);
        written?;
    }
    tool.wait_at_most(SECURE_TOKEN_WAIT)?;
    secure_token_status(name)
}

/// The longest the Mac's own window for a Secure Token waits for the administrator.
const SECURE_TOKEN_WAIT: std::time::Duration = std::time::Duration::from_secs(180);

/// Whether FileVault is on, as `fdesetup status` says ("FileVault is On."). Unknown is taken as
/// on (the window is then shown, for a limited time).
fn filevault_on() -> bool {
    match pctwin_types::process::quiet("/usr/bin/fdesetup")
        .arg("status")
        .stdin(Stdio::null())
        .output()
    {
        Ok(out) => !String::from_utf8_lossy(&out.stdout).contains("FileVault is Off"),
        Err(_) => true,
    }
}

/// Whether the account `name` has a Secure Token, as the system says.
pub(crate) fn secure_token_status(name: &str) -> Result<bool, Error> {
    let out = pctwin_types::process::quiet(SYSADMINCTL)
        .args(["-secureTokenStatus", name])
        .stdin(Stdio::null())
        .output()?;
    // The tool says it on its error stream: "Secure token is ENABLED for user ...".
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    Ok(said.contains("ENABLED") && !said.contains("DISABLED"))
}

/// The system's Service Management.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemDaemons;

impl DaemonService for SystemDaemons {
    fn register(&self, plist: &str) -> Result<(), Error> {
        os_daemon_register(plist).map_err(|(code, _)| Error::Os {
            call: "SMAppService.register",
            code,
        })
    }

    fn unregister(&self, plist: &str) -> Result<(), Error> {
        os_daemon_unregister(plist).map_err(|(code, _)| Error::Os {
            call: "SMAppService.unregister",
            code,
        })
    }

    fn status(&self, plist: &str) -> DaemonState {
        DaemonState::from_raw(os_daemon_status(plist))
    }

    fn open_approval_settings(&self) {
        os_open_login_items();
    }
}

// The raw states, as Apple numbers them.
const _: () = {
    assert!(SMAppServiceStatus::NotRegistered.0 == 0);
    assert!(SMAppServiceStatus::Enabled.0 == 1);
    assert!(SMAppServiceStatus::RequiresApproval.0 == 2);
    assert!(SMAppServiceStatus::NotFound.0 == 3);
};

/// The administrator's approval for one move (see `crate::approval`): the app's side, through the
/// safe wrapper. macOS shows its own window with `prompt`; PCTwin never sees the password.
pub(crate) fn ask_administrator(prompt: &str) -> Result<Option<crate::approval::Approval>, Error> {
    use security_framework::authorization::{Authorization, AuthorizationItemSetBuilder, Flags};
    const CANCELLED: i32 = -60006; // errAuthorizationCanceled
    const DENIED: i32 = -60005; // errAuthorizationDenied
    let fail = |call: &'static str| {
        move |e: security_framework::base::Error| Error::Os {
            call,
            code: i64::from(e.code()),
        }
    };
    let rights = AuthorizationItemSetBuilder::new()
        .add_right(crate::approval::MOVE_RIGHT)
        .map_err(fail("AuthorizationItemSet"))?
        .build();
    let environment = AuthorizationItemSetBuilder::new()
        .add_string("prompt", prompt)
        .map_err(fail("AuthorizationItemSet"))?
        .build();
    let flags = Flags::INTERACTION_ALLOWED | Flags::EXTEND_RIGHTS | Flags::PREAUTHORIZE;
    let approved = match Authorization::new(Some(rights), Some(environment), flags) {
        Ok(a) => a,
        Err(e) if e.code() == CANCELLED || e.code() == DENIED => return Ok(None),
        Err(e) => return Err(fail("AuthorizationCreate")(e)),
    };
    let form = approved
        .make_external_form()
        .map_err(fail("AuthorizationMakeExternalForm"))?;
    let mut bytes = [0u8; 32];
    for (to, from) in bytes.iter_mut().zip(form.bytes.iter()) {
        *to = *from as u8;
    }
    Ok(Some(crate::approval::Approval(bytes)))
}

/// The helper, as root: writes the right `name` as `definition` (a property list) into the
/// authorization database with Apple's own tool.
pub(crate) fn define_right(name: &str, definition: &str) -> Result<(), Error> {
    use std::io::Write;
    let mut child = pctwin_types::process::quiet("/usr/bin/security")
        .args(["authorizationdb", "write", name])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    if let Some(mut input) = child.stdin.take() {
        input.write_all(definition.as_bytes())?;
    }
    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::Os {
            call: "security authorizationdb write",
            code: i64::from(status.code().unwrap_or(-1)),
        })
    }
}

/// The helper: whether `approval` holds the right `right` now.
pub(crate) fn approval_holds(
    approval: &crate::approval::Approval,
    right: &str,
) -> Result<bool, Error> {
    let right = std::ffi::CString::new(right).map_err(|_| Error::BadInput("a right's name"))?;
    os_authorization_holds(&approval.0, &right).map_err(|code| Error::Os {
        call: "AuthorizationCreateFromExternalForm",
        code: i64::from(code),
    })
}

/// Turns the external form `form` back into an approval, asks macOS whether it holds `right`
/// with nothing shown (no interaction allowed), and lets it go. `Err` only when the form is not
/// an approval at all; a refused right is `Ok(false)`.
fn os_authorization_holds(form: &[u8; 32], right: &std::ffi::CStr) -> Result<bool, i32> {
    use security_framework_sys::authorization as sys;
    let mut external = sys::AuthorizationExternalForm { bytes: [0; 32] };
    for (to, from) in external.bytes.iter_mut().zip(form) {
        *to = *from as std::ffi::c_char;
    }
    // SAFETY: every pointer is to a live local (the form, the out-reference, the one item and
    // its set) or to `right`, NUL-terminated and borrowed for the call; the item set is only
    // read; no rights are asked to be copied back (null); the reference made here is freed once,
    // after its last use, and only if it was made.
    #[expect(
        unsafe_code,
        reason = "checking a right on an approval made in another process has no safe binding"
    )]
    let r = unsafe {
        let mut made: sys::AuthorizationRef = std::ptr::null_mut();
        let status = sys::AuthorizationCreateFromExternalForm(&external, &mut made);
        if status != sys::errAuthorizationSuccess {
            Err(status)
        } else {
            let mut item = sys::AuthorizationItem {
                name: right.as_ptr(),
                valueLength: 0,
                value: std::ptr::null_mut(),
                flags: 0,
            };
            let rights = sys::AuthorizationItemSet {
                count: 1,
                items: &mut item,
            };
            let held = sys::AuthorizationCopyRights(
                made,
                &rights,
                std::ptr::null(),
                sys::kAuthorizationFlagExtendRights,
                std::ptr::null_mut(),
            );
            sys::AuthorizationFree(made, sys::kAuthorizationFlagDefaults);
            Ok(held == sys::errAuthorizationSuccess)
        }
    };
    r
}

/// The parser process's Seatbelt profile: nothing at all but what it already holds (its two
/// pipes), ending itself and reading its own process facts. No file, no network, no program.
pub const PARSER_PROFILE: &str = "(version 1)\n(deny default)\n(allow process-info* (target self))\n(allow signal (target self))\n(allow sysctl-read)\n";

/// Walls this process in with `profile` (Seatbelt, `sandbox_init`; Apple marks it deprecated but
/// still honours it, as Chromium relies on). Once in, it cannot be undone. Added 28 October 2026
/// for the parser's walls; approved by the user, 28 October 2026.
fn os_sandbox_init(profile: &std::ffi::CStr) -> Result<(), String> {
    unsafe extern "C" {
        fn sandbox_init(
            profile: *const std::ffi::c_char,
            flags: u64,
            errorbuf: *mut *mut std::ffi::c_char,
        ) -> std::ffi::c_int;
        fn sandbox_free_error(errorbuf: *mut std::ffi::c_char);
    }
    // SAFETY: `profile` is a NUL-terminated string that lives for the call; flags 0 means it is
    // the profile's text. The error text the system may give is copied, then freed once.
    #[expect(unsafe_code, reason = "sandbox_init has no safe binding")]
    let r = unsafe {
        let mut error: *mut std::ffi::c_char = std::ptr::null_mut();
        if sandbox_init(profile.as_ptr(), 0, &mut error) == 0 {
            Ok(())
        } else {
            let why = if error.is_null() {
                "the profile was refused".to_owned()
            } else {
                let why = std::ffi::CStr::from_ptr(error)
                    .to_string_lossy()
                    .into_owned();
                sandbox_free_error(error);
                why
            };
            Err(why)
        }
    };
    r
}

/// Walls the parser process in with [`PARSER_PROFILE`].
pub fn sandbox_parser() -> Result<(), String> {
    let profile = std::ffi::CString::new(PARSER_PROFILE).map_err(|e| e.to_string())?;
    os_sandbox_init(&profile)
}
