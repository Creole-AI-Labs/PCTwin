//! macOS: who is calling on the helper's socket, and switching the helper's launch daemon on and
//! off through SMAppService.
//!
//! The four `os_daemon_*` / `os_open_login_items` functions and `os_od_create_user` are the
//! only unsafe code here, one block each, listed in scripts/unsafe-fence.sh: the Service
//! Management and Open Directory bindings mark every method unsafe. Reading the caller needs
//! none (nix and libproc wrap it safely).

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
    let out = Command::new(program)
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
    os_od_create_user(name, full_name, uid, password).map_err(|(call, code)| Error::Os {
        call: match call {
            "password" => "Open Directory: setting the password",
            "record" => "Open Directory: making the account",
            _ => "Open Directory",
        },
        code,
    })?;
    run(CREATEHOMEDIR, &["-c", "-u", name])?;
    run(
        PWPOLICY,
        &["-u", name, "-setpolicy", "newPasswordRequired=1"],
    )?;
    Ok(uid)
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
