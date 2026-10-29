//! Windows: the UAC prompt, the helper's named pipe and who is calling.
//!
//! Every `unsafe` call is in a function named `os_*`, one block each, listed in
//! scripts/unsafe-fence.sh. Everything around them is safe and does the checking.

use crate::{Account, Caller, Ended, Error, LaunchError, LaunchRequest, Process, Sid};
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::{AsHandle, AsRawHandle, BorrowedHandle, FromRawHandle, OwnedHandle};
use std::path::PathBuf;

use windows::Win32::Foundation::{
    ERROR_ACCESS_DENIED, ERROR_CANCELLED, ERROR_PIPE_BUSY, ERROR_PIPE_CONNECTED, ERROR_SUCCESS,
    GetLastError, HANDLE, HLOCAL, INVALID_HANDLE_VALUE, LocalFree, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows::Win32::NetworkManagement::NetManagement::{
    NetApiBufferFree, NetUserAdd, NetUserGetInfo, NetUserSetInfo, UF_NORMAL_ACCOUNT, UF_SCRIPT,
    USER_ACCOUNT_FLAGS, USER_INFO_1, USER_INFO_4, USER_PRIV_USER,
};
use windows::Win32::Security::Authorization::{
    ConvertSecurityDescriptorToStringSecurityDescriptorW, ConvertSidToStringSidW,
    ConvertStringSecurityDescriptorToSecurityDescriptorW, GetSecurityInfo, SDDL_REVISION_1,
    SE_KERNEL_OBJECT,
};
use windows::Win32::Security::WinTrust::{
    WINTRUST_ACTION_GENERIC_VERIFY_V2, WINTRUST_DATA, WINTRUST_DATA_0, WINTRUST_FILE_INFO,
    WTD_CHOICE_FILE, WTD_REVOCATION_CHECK_CHAIN_EXCLUDE_ROOT, WTD_REVOKE_WHOLECHAIN,
    WTD_STATEACTION_CLOSE, WTD_STATEACTION_VERIFY, WTD_UI_NONE, WTHelperGetProvCertFromChain,
    WTHelperGetProvSignerFromChain, WTHelperProvDataFromStateData, WinVerifyTrust,
};
use windows::Win32::Security::{
    DACL_SECURITY_INFORMATION, GetTokenInformation, PSECURITY_DESCRIPTOR, RevertToSelf,
    SECURITY_ATTRIBUTES, SetKernelObjectSecurity, TOKEN_ELEVATION, TOKEN_QUERY, TOKEN_USER,
    TokenElevation, TokenUser,
};
use windows::Win32::Storage::FileSystem::{
    CreateDirectoryW, FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_FLAGS_AND_ATTRIBUTES, GetDriveTypeW,
    GetVolumePathNameW, PIPE_ACCESS_DUPLEX, SECURITY_IDENTIFICATION,
};
use windows::Win32::System::IO::DeviceIoControl;
use windows::Win32::System::Ioctl::{IOCTL_STORAGE_GET_HOTPLUG_INFO, STORAGE_HOTPLUG_INFO};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, GetNamedPipeClientProcessId, GetNamedPipeServerProcessId,
    ImpersonateNamedPipeClient, NAMED_PIPE_MODE, PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS,
    PIPE_TYPE_BYTE, PIPE_WAIT,
};
use windows::Win32::System::Registry::{
    HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RRF_SUBKEY_WOW6464KEY, RegGetValueW,
};
use windows::Win32::System::RemoteDesktop::{
    WTS_CONNECTSTATE_CLASS, WTS_CURRENT_SERVER_HANDLE, WTS_CURRENT_SESSION, WTSActive,
    WTSConnectState, WTSFreeMemory, WTSQuerySessionInformationW,
};
use windows::Win32::System::SystemInformation::GetSystemWindowsDirectoryW;
use windows::Win32::System::SystemInformation::IMAGE_FILE_MACHINE;
use windows::Win32::System::Threading::{GetCurrentProcess, IsWow64Process2};
use windows::Win32::System::Threading::{
    GetCurrentThread, GetExitCodeProcess, GetProcessId, INFINITE, OpenProcess, OpenProcessToken,
    OpenThreadToken, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
    QueryFullProcessImageNameW, WaitForSingleObject,
};
use windows::Win32::UI::Shell::{
    AssocIsDangerous, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW,
};
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
use windows::core::{HRESULT, PCWSTR, PWSTR, w};

/// What the mover's account and SYSTEM may do with the pipe: read, and write data. Never
/// `FILE_CREATE_PIPE_INSTANCE` (which `GENERIC_WRITE` and `FILE_GENERIC_WRITE` include, as
/// `FILE_APPEND_DATA`): a client that could make instances could stand in for the helper.
/// `FILE_GENERIC_READ` (0x120089) plus `FILE_WRITE_DATA` (0x2).
pub(crate) const CLIENT_ACCESS: u32 = 0x0012_008B;

const NAME_PREFIX: &str = "pctwin-helper-";
const BUFFER: u32 = 64 * 1024;

/// The pipe's access list, in the system's text form: protected (nothing inherited), allowing
/// [`CLIENT_ACCESS`] to the mover and to SYSTEM, and nothing to anyone else.
pub(crate) fn pipe_security(mover: &Sid) -> String {
    format!("D:P(A;;0x{CLIENT_ACCESS:x};;;{mover})(A;;0x{CLIENT_ACCESS:x};;;SY)")
}

/// Duplex, and fail if any pipe of this name already exists (someone squatting on it).
pub(crate) fn open_mode() -> FILE_FLAGS_AND_ATTRIBUTES {
    PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE
}

/// A byte stream that refuses clients on other computers.
pub(crate) fn pipe_mode() -> NAMED_PIPE_MODE {
    PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS
}

fn wide(s: &OsStr) -> Vec<u16> {
    s.encode_wide().chain(std::iter::once(0)).collect()
}

fn os_err(call: &'static str, code: HRESULT) -> Error {
    Error::Os {
        call,
        code: i64::from(code.0),
    }
}

fn raw(h: BorrowedHandle<'_>) -> HANDLE {
    HANDLE(h.as_raw_handle())
}

// ---- The unsafe calls. One block each; see the crate header and scripts/unsafe-fence.sh. ----

/// Starts `file` with `parameters` through the UAC prompt; returns its handle and process ID.
fn os_shell_execute_runas(file: &[u16], parameters: &[u16]) -> Result<(OwnedHandle, u32), HRESULT> {
    assert!(
        file.ends_with(&[0]) && parameters.ends_with(&[0]),
        "NUL-terminated"
    );
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC,
        lpVerb: w!("runas"),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: PCWSTR(parameters.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };
    // SAFETY: `info` is a correctly sized SHELLEXECUTEINFOW whose strings are NUL-terminated
    // (checked above) and outlive the call. With SEE_MASK_NOCLOSEPROCESS a successful call
    // leaves a process handle we now own (checked non-null before it is wrapped, so it is closed
    // exactly once, on drop); GetProcessId only reads it.
    #[expect(unsafe_code, reason = "ShellExecuteExW has no safe binding")]
    let r = unsafe {
        match ShellExecuteExW(&mut info) {
            Err(e) => Err(e.code()),
            Ok(()) if info.hProcess.is_invalid() => Err(HRESULT::from_win32(ERROR_CANCELLED.0)),
            Ok(()) => {
                let id = GetProcessId(info.hProcess);
                Ok((OwnedHandle::from_raw_handle(info.hProcess.0), id))
            }
        }
    };
    r
}

/// Waits up to `millis` for a process; its exit code once ended, `None` while it runs.
fn os_wait(process: BorrowedHandle<'_>, millis: u32) -> Result<Option<u32>, HRESULT> {
    let h = raw(process);
    // SAFETY: `h` is a live process handle borrowed for the call; the exit code is written into
    // a local.
    #[expect(
        unsafe_code,
        reason = "WaitForSingleObject and GetExitCodeProcess have no safe binding"
    )]
    let r = unsafe {
        match WaitForSingleObject(h, millis) {
            WAIT_OBJECT_0 => {
                let mut code = 0u32;
                GetExitCodeProcess(h, &mut code)
                    .map(|()| Some(code))
                    .map_err(|e| e.code())
            }
            WAIT_TIMEOUT => Ok(None),
            _ => Err(GetLastError().to_hresult()),
        }
    };
    r
}

/// Makes the first and only instance of the pipe `name` with the access list `sddl`.
fn os_create_pipe(name: &[u16], sddl: &[u16]) -> Result<OwnedHandle, HRESULT> {
    assert!(
        name.ends_with(&[0]) && sddl.ends_with(&[0]),
        "NUL-terminated"
    );
    // SAFETY: both strings are NUL-terminated (checked above). The security descriptor the
    // system allocates is used only during CreateNamedPipeW and freed once, after it. A valid
    // pipe handle is ours alone and wrapped exactly once; the error is read before LocalFree can
    // change it.
    #[expect(
        unsafe_code,
        reason = "named-pipe creation with an access list has no safe binding"
    )]
    let r = unsafe {
        let mut sd = PSECURITY_DESCRIPTOR::default();
        match ConvertStringSecurityDescriptorToSecurityDescriptorW(
            PCWSTR(sddl.as_ptr()),
            SDDL_REVISION_1,
            &mut sd,
            None,
        ) {
            Err(e) => Err(e.code()),
            Ok(()) => {
                let attributes = SECURITY_ATTRIBUTES {
                    nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
                    lpSecurityDescriptor: sd.0,
                    bInheritHandle: false.into(),
                };
                let h = CreateNamedPipeW(
                    PCWSTR(name.as_ptr()),
                    open_mode(),
                    pipe_mode(),
                    1,
                    BUFFER,
                    BUFFER,
                    0,
                    Some(&attributes),
                );
                let err = GetLastError();
                LocalFree(Some(HLOCAL(sd.0)));
                if h == INVALID_HANDLE_VALUE || h.is_invalid() {
                    Err(err.to_hresult())
                } else {
                    Ok(OwnedHandle::from_raw_handle(h.0))
                }
            }
        }
    };
    r
}

/// The access list the system holds for `object`, in its text form.
fn os_access_list(object: BorrowedHandle<'_>) -> Result<String, HRESULT> {
    let h = raw(object);
    // SAFETY: `h` is borrowed for the call. Both buffers the system allocates are freed once,
    // after their last use; the text is copied out before its buffer is freed.
    #[expect(
        unsafe_code,
        reason = "reading an object's access list has no safe binding"
    )]
    let r = unsafe {
        let mut sd = PSECURITY_DESCRIPTOR::default();
        let e = GetSecurityInfo(
            h,
            SE_KERNEL_OBJECT,
            DACL_SECURITY_INFORMATION,
            None,
            None,
            None,
            None,
            Some(&mut sd),
        );
        if e != ERROR_SUCCESS {
            Err(e.to_hresult())
        } else {
            let mut text = PWSTR::null();
            let out = ConvertSecurityDescriptorToStringSecurityDescriptorW(
                sd,
                SDDL_REVISION_1,
                DACL_SECURITY_INFORMATION,
                &mut text,
                None,
            )
            .map_err(|e| e.code())
            .map(|()| String::from_utf16_lossy(text.as_wide()));
            if !text.is_null() {
                LocalFree(Some(HLOCAL(text.0.cast())));
            }
            LocalFree(Some(HLOCAL(sd.0)));
            out
        }
    };
    r
}

/// Waits for the pipe's one client (at once if it is already connected).
fn os_accept(pipe: BorrowedHandle<'_>) -> Result<(), HRESULT> {
    let h = raw(pipe);
    // SAFETY: `h` is our pipe's server handle, borrowed for the call; no OVERLAPPED is passed,
    // so the call completes before it returns.
    #[expect(unsafe_code, reason = "ConnectNamedPipe has no safe binding")]
    let r = unsafe { ConnectNamedPipe(h, None) };
    match r {
        Ok(()) => Ok(()),
        Err(e) if e.code() == ERROR_PIPE_CONNECTED.to_hresult() => Ok(()),
        Err(e) => Err(e.code()),
    }
}

/// The process ID of the pipe's client, as the system recorded it when it connected.
fn os_client_process_id(pipe: BorrowedHandle<'_>) -> Result<u32, HRESULT> {
    let h = raw(pipe);
    let mut id = 0u32;
    // SAFETY: `h` is borrowed for the call; the ID is written into a local.
    #[expect(
        unsafe_code,
        reason = "GetNamedPipeClientProcessId has no safe binding"
    )]
    let r = unsafe { GetNamedPipeClientProcessId(h, &mut id) };
    r.map(|()| id).map_err(|e| e.code())
}

/// The process ID of the pipe's server, seen from the client's end.
fn os_server_process_id(pipe: BorrowedHandle<'_>) -> Result<u32, HRESULT> {
    let h = raw(pipe);
    let mut id = 0u32;
    // SAFETY: `h` is borrowed for the call; the ID is written into a local.
    #[expect(
        unsafe_code,
        reason = "GetNamedPipeServerProcessId has no safe binding"
    )]
    let r = unsafe { GetNamedPipeServerProcessId(h, &mut id) };
    r.map(|()| id).map_err(|e| e.code())
}

/// Opens process `id` to read its identity and whether it still runs, nothing more.
fn os_open_process(id: u32) -> Result<OwnedHandle, HRESULT> {
    // SAFETY: OpenProcess takes plain values; a returned handle is valid (the binding turns a
    // null one into an error), ours alone, and wrapped exactly once.
    #[expect(unsafe_code, reason = "OpenProcess has no safe binding")]
    let r = unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
            false,
            id,
        )
        .map(|h| OwnedHandle::from_raw_handle(h.0))
    };
    r.map_err(|e| e.code())
}

/// The process's own access token, to read its account.
fn os_process_token(process: BorrowedHandle<'_>) -> Result<OwnedHandle, HRESULT> {
    let h = raw(process);
    let mut token = HANDLE::default();
    // SAFETY: `h` is borrowed for the call; a token handle the call returns is ours alone and
    // wrapped exactly once.
    #[expect(unsafe_code, reason = "OpenProcessToken has no safe binding")]
    let r = unsafe {
        OpenProcessToken(h, TOKEN_QUERY, &mut token).map(|()| OwnedHandle::from_raw_handle(token.0))
    };
    r.map_err(|e| e.code())
}

/// The token the pipe's client connected with, as the system holds it for this connection
/// (not looked up by process ID). Taken by impersonating the client on this thread for the
/// length of one call, at the level the client allowed (identification), and reverted at once.
fn os_client_token(pipe: BorrowedHandle<'_>) -> Result<OwnedHandle, HRESULT> {
    let h = raw(pipe);
    let mut token = HANDLE::default();
    // SAFETY: `h` is our pipe's server handle, borrowed for the call. While impersonating, the
    // only call made is OpenThreadToken on this thread's pseudo-handle (opened as self, so the
    // client's identity-only token never opens anything); a returned handle is wrapped exactly
    // once. RevertToSelf runs whatever OpenThreadToken returned; if it ever failed, this thread
    // would go on as the client, so the process stops instead.
    #[expect(
        unsafe_code,
        reason = "ImpersonateNamedPipeClient and OpenThreadToken have no safe binding"
    )]
    let r = unsafe {
        ImpersonateNamedPipeClient(h)
            .map_err(|e| e.code())
            .and_then(|()| {
                let opened = OpenThreadToken(GetCurrentThread(), TOKEN_QUERY, true, &mut token);
                if RevertToSelf().is_err() {
                    std::process::abort();
                }
                opened
                    .map(|()| OwnedHandle::from_raw_handle(token.0))
                    .map_err(|e| e.code())
            })
    };
    r
}

/// Whether a token has an administrator's full rights (`TokenElevation`).
fn os_token_elevated(token: BorrowedHandle<'_>) -> Result<bool, HRESULT> {
    let h = raw(token);
    let mut elevation = TOKEN_ELEVATION::default();
    let mut needed = 0u32;
    // SAFETY: `h` is a token handle borrowed for the call; the answer is written into
    // `elevation`, whose true size is passed, so the call cannot write past it.
    #[expect(
        unsafe_code,
        reason = "reading a token's elevation has no safe binding"
    )]
    let r = unsafe {
        GetTokenInformation(
            h,
            TokenElevation,
            Some((&raw mut elevation).cast()),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut needed,
        )
    };
    r.map(|()| elevation.TokenIsElevated != 0)
        .map_err(|e| e.code())
}

/// Whether this program's own session is the one on the screen and keyboard (its connection
/// state is `WTSActive`): after switching to another person, it is not.
fn os_session_on_screen() -> Result<bool, HRESULT> {
    let mut buffer = PWSTR::null();
    let mut bytes = 0u32;
    // SAFETY: the current server and current session name this program's own session. On
    // success the system allocates the buffer and says its size; one connection state (a 32-bit
    // value) is read from it only if the size holds one, and the buffer is freed exactly once.
    #[expect(
        unsafe_code,
        reason = "WTSQuerySessionInformationW and WTSFreeMemory have no safe binding"
    )]
    let r = unsafe {
        WTSQuerySessionInformationW(
            Some(WTS_CURRENT_SERVER_HANDLE),
            WTS_CURRENT_SESSION,
            WTSConnectState,
            &mut buffer,
            &mut bytes,
        )
        .map_err(|e| e.code())
        .map(|()| {
            let holds = !buffer.is_null()
                && bytes as usize >= std::mem::size_of::<WTS_CONNECTSTATE_CLASS>();
            let state = holds.then(|| buffer.0.cast::<WTS_CONNECTSTATE_CLASS>().read_unaligned());
            WTSFreeMemory(buffer.0.cast());
            state
        })
    };
    match r {
        Ok(Some(state)) => Ok(state == WTSActive),
        Ok(None) => Err(HRESULT::from_win32(ERROR_ACCESS_DENIED.0)),
        Err(code) => Err(code),
    }
}

/// The account (its security identifier, as text) a token belongs to.
fn os_token_user(token: BorrowedHandle<'_>) -> Result<String, HRESULT> {
    let h = raw(token);
    // Room for TOKEN_USER and the largest identifier (68 bytes), aligned for TOKEN_USER.
    let mut buffer = [0u64; 64];
    let mut needed = 0u32;
    // SAFETY: `h` is a token handle borrowed for the call. The buffer is aligned and its true
    // size is passed, so GetTokenInformation cannot overrun it, and on success it holds a
    // TOKEN_USER whose identifier points inside the buffer, read while the buffer lives. The
    // text the system allocates is copied out, then freed once.
    #[expect(unsafe_code, reason = "reading a token's user has no safe binding")]
    let r = unsafe {
        GetTokenInformation(
            h,
            TokenUser,
            Some(buffer.as_mut_ptr().cast()),
            std::mem::size_of_val(&buffer) as u32,
            &mut needed,
        )
        .map_err(|e| e.code())
        .and_then(|()| {
            let user = &*buffer.as_ptr().cast::<TOKEN_USER>();
            let mut text = PWSTR::null();
            ConvertSidToStringSidW(user.User.Sid, &mut text).map_err(|e| e.code())?;
            let sid = String::from_utf16_lossy(text.as_wide());
            LocalFree(Some(HLOCAL(text.0.cast())));
            Ok(sid)
        })
    };
    r
}

/// The kind of drive (as `GetDriveTypeW` numbers it; 3 is fixed) of the volume that holds
/// `file`: its mount point is found first (`GetVolumePathNameW`), so a volume mounted in a
/// folder is asked about itself, not about the drive letter it hangs from.
fn os_volume_drive_type(file: &[u16]) -> Result<u32, HRESULT> {
    assert!(file.ends_with(&[0]), "NUL-terminated");
    let mut root = vec![0u16; 32_768];
    // SAFETY: `file` is NUL-terminated (checked above); the binding passes the root buffer's
    // true length, so the system cannot overrun it, and on success the buffer holds a
    // NUL-terminated mount point, which GetDriveTypeW reads while the buffer lives.
    #[expect(
        unsafe_code,
        reason = "GetVolumePathNameW and GetDriveTypeW have no safe binding"
    )]
    let r = unsafe {
        GetVolumePathNameW(PCWSTR(file.as_ptr()), &mut root)
            .map(|()| GetDriveTypeW(PCWSTR(root.as_ptr())))
    };
    r.map_err(|e| e.code())
}

/// Makes the standard (never administrator) local account `name`, shown as `full_name`, with
/// `password`, which must be changed at its first sign-in; returns its SID. The password is
/// passed to the system in memory only, never on a command line.
pub(crate) fn add_standard_account(
    name: &str,
    full_name: &str,
    password: &str,
) -> Result<String, Error> {
    let name_w = wide(OsStr::new(name));
    let mut password_w = wide(OsStr::new(password));
    let added = os_net_user_add(&name_w, &password_w, &wide(OsStr::new(full_name)));
    // The password's copy here is cleared whatever happened.
    password_w.iter_mut().for_each(|c| *c = 0);
    added.map_err(net_error)?;
    // The account is made (NetUserAdd refuses a name that is taken, so it is PCTwin's own). If
    // marking its password fails, it is taken away again by its SID, never left half-made.
    let (sid, expired) = os_expire_password(&name_w);
    let (read, failed) = match (sid, expired) {
        (Some(sid), Ok(())) => return Ok(sid),
        // ERROR_NONE_MAPPED: the account's SID could not be read.
        (None, Ok(())) => (None, net_error(1332)),
        (sid, Err(status)) => (sid, net_error(status)),
    };
    if let Some(sid) = crate::made_sid(read, || sid_of_name(name).ok()) {
        let _ = remove_account(&sid);
    }
    Err(failed)
}

/// PowerShell, by its full path in the system folder.
fn powershell() -> Result<PathBuf, Error> {
    system_program(r"WindowsPowerShell\v1.0\powershell.exe")
}

/// Runs one PowerShell command (built only from checked SIDs and account names), its output.
fn run_powershell(command: &str) -> Result<String, Error> {
    let out = clean_command(&powershell()?)?
        .args(["-NoProfile", "-NonInteractive", "-Command", command])
        .stdin(std::process::Stdio::null())
        .output()?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(Error::Os {
            call: "a system tool",
            code: i64::from(out.status.code().unwrap_or(-1)),
        })
    }
}

/// The SID the system gives the local account `name` (checked to be a plain account name first,
/// so nothing else can be said to the shell).
fn sid_of_name(name: &str) -> Result<String, Error> {
    if !crate::is_account_name(name) {
        return Err(Error::BadInput("an account name"));
    }
    let sid = run_powershell(&format!("(Get-LocalUser -Name '{name}').SID.Value"))?;
    crate::Sid::parse(&sid).map(|s| s.as_str().to_string())
}

/// Takes away the local account PCTwin just made, by its SID (checked first), never by a name.
fn remove_account(sid: &str) -> Result<(), Error> {
    let sid = crate::Sid::parse(sid)?;
    run_powershell(&format!("Remove-LocalUser -SID '{}'", sid.as_str())).map(|_| ())
}

fn net_error(status: u32) -> Error {
    match status {
        // NERR_UserExists, NERR_GroupExists
        2224 | 2223 => Error::AlreadyExists,
        // ERROR_ACCESS_DENIED
        5 => Error::AccessDenied,
        other => Error::Io(std::io::Error::from_raw_os_error(other as i32)),
    }
}

/// `NetUserAdd` at level 1: a normal account with the user privilege (never administrator).
fn os_net_user_add(name: &[u16], password: &[u16], comment: &[u16]) -> Result<(), u32> {
    assert!(name.ends_with(&[0]) && password.ends_with(&[0]) && comment.ends_with(&[0]));
    let info = USER_INFO_1 {
        usri1_name: PWSTR(name.as_ptr().cast_mut()),
        usri1_password: PWSTR(password.as_ptr().cast_mut()),
        usri1_password_age: 0,
        usri1_priv: USER_PRIV_USER,
        usri1_home_dir: PWSTR::null(),
        usri1_comment: PWSTR(comment.as_ptr().cast_mut()),
        usri1_flags: USER_ACCOUNT_FLAGS(UF_SCRIPT.0 | UF_NORMAL_ACCOUNT),
        usri1_script_path: PWSTR::null(),
    };
    // SAFETY: every string is NUL-terminated (checked above) and outlives the call; the system
    // only reads them (the binding's PWSTR is not written through by NetUserAdd), and `info` is
    // a USER_INFO_1 as level 1 requires, passed by address for the call only.
    #[expect(unsafe_code, reason = "NetUserAdd has no safe binding")]
    let status = unsafe { NetUserAdd(PCWSTR::null(), 1, std::ptr::from_ref(&info).cast(), None) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Marks `name`'s password expired (it must be changed at the next sign-in), read back first so
/// nothing else of the account is reset. Returns its SID as text, when it could be read (before
/// the change, so a failed change still says which account to take away), and how the change
/// went.
fn os_expire_password(name: &[u16]) -> (Option<String>, Result<(), u32>) {
    assert!(name.ends_with(&[0]));
    let mut buffer: *mut u8 = std::ptr::null_mut();
    // SAFETY: `name` is NUL-terminated; NetUserGetInfo allocates a USER_INFO_4 at `buffer`,
    // which is read and changed only while it lives and freed exactly once below, whatever
    // happens. The SID it holds points inside the same allocation and is turned into text (the
    // text's own allocation freed once) before the buffer is changed or freed. Its password
    // field is set to null, so NetUserSetInfo leaves the password as it is.
    #[expect(
        unsafe_code,
        reason = "NetUserGetInfo, NetUserSetInfo and ConvertSidToStringSidW have no safe binding"
    )]
    let r = unsafe {
        let got = NetUserGetInfo(PCWSTR::null(), PCWSTR(name.as_ptr()), 4, &mut buffer);
        if got != 0 {
            return (None, Err(got));
        }
        // Success always gives a buffer; a null one is refused rather than read.
        if buffer.is_null() {
            return (None, Err(1));
        }
        let info = &mut *buffer.cast::<USER_INFO_4>();
        let mut text = PWSTR::null();
        let sid = match ConvertSidToStringSidW(info.usri4_user_sid, &mut text) {
            Ok(()) => {
                let s = String::from_utf16_lossy(text.as_wide());
                LocalFree(Some(HLOCAL(text.0.cast())));
                Some(s)
            }
            Err(_) => None,
        };
        info.usri4_password_expired = 1;
        info.usri4_password = PWSTR::null();
        let set = NetUserSetInfo(PCWSTR::null(), PCWSTR(name.as_ptr()), 4, buffer, None);
        NetApiBufferFree(Some(buffer.cast()));
        (sid, if set == 0 { Ok(()) } else { Err(set) })
    };
    r
}

/// Makes the new folder `path` with exactly the access list `sddl` from its first moment (never
/// a folder that already exists: that fails, so nothing anyone planted is ever used).
pub(crate) fn create_folder_with_access(path: &std::path::Path, sddl: &str) -> Result<(), Error> {
    let path_w = wide(path.as_os_str());
    let sddl_w = wide(OsStr::new(sddl));
    os_create_dir_with_sddl(&path_w, &sddl_w).map_err(|code| {
        // ERROR_ALREADY_EXISTS as an HRESULT
        if code.0 as u32 == 0x8007_00B7 {
            Error::NameTaken
        } else {
            Error::Os {
                call: "CreateDirectoryW",
                code: i64::from(code.0),
            }
        }
    })
}

/// `CreateDirectoryW` with the security descriptor `sddl` describes.
fn os_create_dir_with_sddl(path: &[u16], sddl: &[u16]) -> Result<(), HRESULT> {
    assert!(
        path.ends_with(&[0]) && sddl.ends_with(&[0]),
        "NUL-terminated"
    );
    // SAFETY: both strings are NUL-terminated (checked above). The security descriptor the
    // system allocates is used only during CreateDirectoryW and freed once, after it; the error
    // is read before LocalFree can change it.
    #[expect(
        unsafe_code,
        reason = "making a folder with an access list has no safe binding"
    )]
    let r = unsafe {
        let mut sd = PSECURITY_DESCRIPTOR::default();
        match ConvertStringSecurityDescriptorToSecurityDescriptorW(
            PCWSTR(sddl.as_ptr()),
            SDDL_REVISION_1,
            &mut sd,
            None,
        ) {
            Err(e) => Err(e.code()),
            Ok(()) => {
                let attributes = SECURITY_ATTRIBUTES {
                    nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
                    lpSecurityDescriptor: sd.0,
                    bInheritHandle: false.into(),
                };
                let made = CreateDirectoryW(PCWSTR(path.as_ptr()), Some(&attributes));
                LocalFree(Some(HLOCAL(sd.0)));
                made.map_err(|e| e.code())
            }
        }
    };
    r
}

/// The access list this process gets while it carries other people's things (Security Design
/// part C): SYSTEM keeps everything; the account it runs as, and the owner's implicit rights
/// (`OW`, so the owner cannot simply change the list back), may only see that it runs, wait for it
/// and end it (`PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION | SYNCHRONIZE`): never read
/// or write its memory, start a thread in it, or copy its handles.
fn hardened_process_sddl(user: &Sid) -> String {
    format!(
        "D:P(A;;GA;;;SY)(A;;0x101001;;;OW)(A;;0x101001;;;{})",
        user.as_str()
    )
}

/// Gives this process [`hardened_process_sddl`]'s access list.
pub(crate) fn harden_process() -> Result<(), Error> {
    let me = current_sid()?;
    let sddl = wide(OsStr::new(&hardened_process_sddl(&me)));
    os_set_process_sddl(&sddl).map_err(|code| os_err("SetKernelObjectSecurity", code))
}

/// `SetKernelObjectSecurity` on this process, with the access list `sddl` describes.
fn os_set_process_sddl(sddl: &[u16]) -> Result<(), HRESULT> {
    assert!(sddl.ends_with(&[0]), "NUL-terminated");
    // SAFETY: the string is NUL-terminated (checked above). GetCurrentProcess returns a
    // pseudo-handle that needs no closing. The security descriptor the system allocates is used
    // only during SetKernelObjectSecurity and freed once, after it; the result is read before
    // LocalFree can change the last error.
    #[expect(
        unsafe_code,
        reason = "changing a process's access list has no safe binding"
    )]
    let r = unsafe {
        let mut sd = PSECURITY_DESCRIPTOR::default();
        match ConvertStringSecurityDescriptorToSecurityDescriptorW(
            PCWSTR(sddl.as_ptr()),
            SDDL_REVISION_1,
            &mut sd,
            None,
        ) {
            Err(e) => Err(e.code()),
            Ok(()) => {
                let set =
                    SetKernelObjectSecurity(GetCurrentProcess(), DACL_SECURITY_INFORMATION, sd);
                LocalFree(Some(HLOCAL(sd.0)));
                set.map_err(|e| e.code())
            }
        }
    };
    r
}

/// The home folder of the account `sid` (named `name`): made now if the person has never
/// signed in (`CreateProfile`), else the one the system's profile list names.
pub(crate) fn profile_folder(sid: &str, name: &str) -> Result<std::path::PathBuf, Error> {
    // MAX_PATH exactly: CreateProfile refuses a longer buffer with RPC_X_BAD_STUB_DATA
    // (0x800706F7), which the two-accounts lab found.
    let mut path = vec![0u16; 260];
    match os_create_profile(&wide(OsStr::new(sid)), &wide(OsStr::new(name)), &mut path) {
        Ok(()) => {
            let len = path.iter().position(|c| *c == 0).unwrap_or(path.len());
            Ok(std::path::PathBuf::from(String::from_utf16_lossy(
                &path[..len],
            )))
        }
        // ERROR_ALREADY_EXISTS: the person has signed in before.
        Err(code) if code.0 as u32 == 0x8007_00B7 => listed_profile(sid),
        Err(code) => Err(Error::Os {
            call: "CreateProfile",
            code: i64::from(code.0),
        }),
    }
}

/// The profile folder the system's profile list names for `sid`, read with the system's own
/// `reg` tool (read only).
fn listed_profile(sid: &str) -> Result<std::path::PathBuf, Error> {
    Sid::parse(sid)?;
    let key = format!(r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\ProfileList\{sid}");
    let system = system_folder(SystemFolder::Windows)?;
    let out = clean_command(&system.join("System32").join("reg.exe"))?
        .args(["query", &key, "/v", "ProfileImagePath"])
        .output()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let value = text
        .lines()
        .find_map(|l| {
            let l = l.trim();
            l.strip_prefix("ProfileImagePath")
                .and_then(|r| {
                    r.trim_start()
                        .split_once("REG_EXPAND_SZ")
                        .or_else(|| r.trim_start().split_once("REG_SZ"))
                })
                .map(|(_, v)| v.trim().to_string())
        })
        .ok_or(Error::NotSupported("no profile listed for that account"))?;
    // %SystemDrive% is the drive Windows is on, taken from where Windows itself says it is.
    let Some(std::path::Component::Prefix(drive)) = system.components().next() else {
        return Err(Error::NotSupported("the Windows folder has no drive"));
    };
    let drive = drive.as_os_str().to_string_lossy();
    Ok(std::path::PathBuf::from(
        value.replace("%SystemDrive%", &drive),
    ))
}

/// `CreateProfile`: the profile folder of a person who has never signed in.
fn os_create_profile(sid: &[u16], name: &[u16], path: &mut [u16]) -> Result<(), HRESULT> {
    assert!(
        sid.ends_with(&[0]) && name.ends_with(&[0]),
        "NUL-terminated"
    );
    // SAFETY: both strings are NUL-terminated (checked above) and only read; the binding passes
    // the path buffer's true length, so the system cannot overrun it.
    #[expect(unsafe_code, reason = "CreateProfile has no safe binding")]
    let r = unsafe {
        windows::Win32::UI::Shell::CreateProfile(PCWSTR(sid.as_ptr()), PCWSTR(name.as_ptr()), path)
    };
    r.map_err(|e| e.code())
}

/// The machine's own architecture (`IMAGE_FILE_MACHINE_*`), whatever this process runs as.
pub(crate) fn native_machine() -> Result<u16, HRESULT> {
    os_native_machine()
}

/// `IsWow64Process2` for this process: the native machine.
fn os_native_machine() -> Result<u16, HRESULT> {
    let mut process = IMAGE_FILE_MACHINE(0);
    let mut native = IMAGE_FILE_MACHINE(0);
    // SAFETY: GetCurrentProcess returns a pseudo-handle that needs no closing and is valid for
    // the call; both out-pointers point to live locals of the right type.
    #[expect(
        unsafe_code,
        reason = "GetCurrentProcess and IsWow64Process2 have no safe binding"
    )]
    let r = unsafe { IsWow64Process2(GetCurrentProcess(), &mut process, Some(&mut native)) };
    r.map(|()| native.0).map_err(|e| e.code())
}

/// The full path of the program the process runs.
fn os_image_path(process: BorrowedHandle<'_>) -> Result<PathBuf, HRESULT> {
    let h = raw(process);
    let mut buffer = vec![0u16; 32_768];
    let mut len = buffer.len() as u32;
    // SAFETY: `h` is borrowed for the call; the buffer's true length is passed and the system
    // writes back how much of it it filled.
    #[expect(unsafe_code, reason = "QueryFullProcessImageNameW has no safe binding")]
    let r = unsafe {
        QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buffer.as_mut_ptr()), &mut len)
    };
    r.map_err(|e| e.code())?;
    buffer.truncate(len as usize);
    Ok(PathBuf::from(OsString::from_wide(&buffer)))
}

/// Where Windows is, as the kernel holds it (not the environment's `SystemRoot`).
fn os_windows_dir() -> Result<PathBuf, HRESULT> {
    let mut buffer = vec![0u16; 32_768];
    // SAFETY: the binding passes the buffer's true length; the system writes at most that much
    // and returns how many characters it wrote (0 on failure, more than the length if too small).
    #[expect(unsafe_code, reason = "GetSystemWindowsDirectoryW has no safe binding")]
    let len = unsafe { GetSystemWindowsDirectoryW(Some(&mut buffer)) } as usize;
    if len == 0 || len >= buffer.len() {
        return Err(windows::core::Error::from_thread().code());
    }
    buffer.truncate(len);
    Ok(PathBuf::from(OsString::from_wide(&buffer)))
}

/// A plain text value (`REG_SZ`, never one with variables to fill in) from the 64-bit view of
/// `HKEY_LOCAL_MACHINE`, which only administrators can change.
fn os_machine_text(subkey: &[u16], value: &[u16]) -> Result<OsString, HRESULT> {
    assert!(
        subkey.ends_with(&[0]) && value.ends_with(&[0]),
        "NUL-terminated"
    );
    let mut buffer = vec![0u16; 32_768];
    let mut bytes = (buffer.len() * 2) as u32;
    // SAFETY: both names are NUL-terminated (checked above) and only read; the data pointer is
    // the buffer's and `bytes` its true size in bytes, which the system updates to what it wrote.
    #[expect(unsafe_code, reason = "RegGetValueW has no safe binding")]
    let e = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(subkey.as_ptr()),
            PCWSTR(value.as_ptr()),
            RRF_RT_REG_SZ | RRF_SUBKEY_WOW6464KEY,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&mut bytes),
        )
    };
    if e != ERROR_SUCCESS {
        return Err(e.to_hresult());
    }
    buffer.truncate(bytes as usize / 2);
    while buffer.last() == Some(&0) {
        buffer.pop();
    }
    Ok(OsString::from_wide(&buffer))
}

/// Windows' own signature check (`WinVerifyTrust`, Authenticode policy, the whole chain checked
/// for withdrawn certificates except the root) of the signature inside the open file `file`,
/// named `path` (NUL-terminated). Returns Windows' answer (0: trusted) and, when it is trusted,
/// the signing certificate's bytes as Windows read them.
///
/// Which bytes Windows checks depends on the kind of file. For a Windows program (PE) Windows
/// reads through the handle, so the file checked is the very file held (second review, finding
/// B2). For a Windows Installer package it does not: the package checker opens the file again by
/// `path` and ignores the handle (third review, finding 3, shown by a folder link swapped while
/// Windows looked), so that answer is about whatever `path` names at that moment. The maker check
/// counts a package's answer only for the administrator helper's own copy, in its
/// administrators-only folder, held so nobody can change it (`pctwin_maker::check::check_staged`).
/// Added 27 October 2026 (second review, finding B3: before, the answer came from PowerShell,
/// which loads code from the person's own folders); approved by the user, 27 October 2026.
fn os_win_verify_trust(file: BorrowedHandle<'_>, path: &[u16]) -> (i32, Option<Vec<u8>>) {
    assert!(path.ends_with(&[0]), "NUL-terminated");
    let mut info = WINTRUST_FILE_INFO {
        cbStruct: std::mem::size_of::<WINTRUST_FILE_INFO>() as u32,
        pcwszFilePath: PCWSTR(path.as_ptr()),
        hFile: raw(file),
        pgKnownSubject: std::ptr::null_mut(),
    };
    let mut data = WINTRUST_DATA {
        cbStruct: std::mem::size_of::<WINTRUST_DATA>() as u32,
        dwUIChoice: WTD_UI_NONE,
        fdwRevocationChecks: WTD_REVOKE_WHOLECHAIN,
        dwUnionChoice: WTD_CHOICE_FILE,
        Anonymous: WINTRUST_DATA_0 {
            pFile: &raw mut info,
        },
        dwStateAction: WTD_STATEACTION_VERIFY,
        dwProvFlags: WTD_REVOCATION_CHECK_CHAIN_EXCLUDE_ROOT,
        ..Default::default()
    };
    let mut action = WINTRUST_ACTION_GENERIC_VERIFY_V2;
    let no_window = windows::Win32::Foundation::HWND(INVALID_HANDLE_VALUE.0);
    // SAFETY: `info` and `data` are fully set (sizes true, the rest zero) and live across both
    // calls; the path is NUL-terminated (checked above) and the handle is borrowed for the call.
    // The second call (close) is always made with the state the first left, so Windows frees
    // it. The signer's certificate is read only when the check passed, through Windows' own
    // helpers, each result checked for null before it is read, and its bytes are copied out
    // before the state is closed.
    #[expect(
        unsafe_code,
        reason = "WinVerifyTrust and its provider helpers have no safe binding"
    )]
    let r = unsafe {
        let code = WinVerifyTrust(no_window, &mut action, (&raw mut data).cast());
        let mut signer = None;
        if code == 0 {
            let provider = WTHelperProvDataFromStateData(data.hWVTStateData);
            if !provider.is_null() {
                let sgnr = WTHelperGetProvSignerFromChain(provider, 0, false, 0);
                if !sgnr.is_null() {
                    let cert = WTHelperGetProvCertFromChain(sgnr, 0);
                    if !cert.is_null() && !(*cert).pCert.is_null() {
                        let ctx = &*(*cert).pCert;
                        if !ctx.pbCertEncoded.is_null() {
                            signer = Some(
                                std::slice::from_raw_parts(
                                    ctx.pbCertEncoded,
                                    ctx.cbCertEncoded as usize,
                                )
                                .to_vec(),
                            );
                        }
                    }
                }
            }
        }
        data.dwStateAction = WTD_STATEACTION_CLOSE;
        WinVerifyTrust(no_window, &mut action, (&raw mut data).cast());
        (code, signer)
    };
    r
}

/// Whether a disk (opened for no access, only to be asked) says it can be removed: its hotplug
/// information (`IOCTL_STORAGE_GET_HOTPLUG_INFO`), which needs no administrator. Added
/// 27 October 2026 (second review, low finding: an NVMe disk in a Thunderbolt or USB4 case reads
/// as a bus inside the laptop); approved by the user, 27 October 2026.
fn os_disk_hotplug(disk: BorrowedHandle<'_>) -> Result<STORAGE_HOTPLUG_INFO, HRESULT> {
    let mut info = STORAGE_HOTPLUG_INFO {
        Size: std::mem::size_of::<STORAGE_HOTPLUG_INFO>() as u32,
        ..Default::default()
    };
    let mut returned = 0u32;
    // SAFETY: the handle is borrowed for the call; no input is passed; the output is `info`,
    // whose true size is passed, so the system cannot write past it; the call is synchronous
    // (no overlapped structure), so nothing is written after it returns.
    #[expect(unsafe_code, reason = "DeviceIoControl has no safe binding")]
    let r = unsafe {
        DeviceIoControl(
            raw(disk),
            IOCTL_STORAGE_GET_HOTPLUG_INFO,
            None,
            0,
            Some((&raw mut info).cast()),
            std::mem::size_of::<STORAGE_HOTPLUG_INFO>() as u32,
            Some(&mut returned),
            None,
        )
    };
    r.map_err(|e| e.code())?;
    if returned as usize != std::mem::size_of::<STORAGE_HOTPLUG_INFO>() {
        return Err(HRESULT::from_win32(ERROR_ACCESS_DENIED.0));
    }
    Ok(info)
}

// ---- Safe code from here on. ----

/// What a disk says about being removed ([`disk_hotplug`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiskHotplug {
    /// Its medium can be taken out (a card reader).
    pub media_removable: bool,
    /// The disk itself can be unplugged while running (USB, Thunderbolt, USB4, ExpressCard).
    pub device_hotplug: bool,
}

/// Asks disk number `n` (as Windows numbers its disks) whether it can be removed. Needs no
/// administrator: the disk is opened for no access, only to be asked.
pub fn disk_hotplug(n: u32) -> Result<DiskHotplug, Error> {
    let disk = std::fs::OpenOptions::new()
        .access_mode(0)
        .share_mode(0x1 | 0x2)
        .open(format!(r"\\.\PhysicalDrive{n}"))
        .map_err(Error::Io)?;
    let info = os_disk_hotplug(disk.as_handle())
        .map_err(|c| os_err("IOCTL_STORAGE_GET_HOTPLUG_INFO", c))?;
    Ok(DiskHotplug {
        media_removable: info.MediaRemovable,
        device_hotplug: info.DeviceHotplug,
    })
}

/// Windows' own answer about the signature inside one open file ([`verify_file_trust`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileTrust {
    /// Windows' result: 0 when it trusts the signature, otherwise its error code (for example
    /// `TRUST_E_NOSIGNATURE`, `TRUST_E_BAD_DIGEST`, `CERT_E_UNTRUSTEDROOT`).
    pub code: i32,
    /// The signing certificate's bytes as Windows read them, when it trusts the signature.
    pub signer: Option<Vec<u8>>,
}

/// Whether Windows itself counts a file type as dangerous to open (`AssocIsDangerous`: its own
/// list plus the policy's high-risk types and what the type's registration says). `assoc` is the
/// ending with its dot, NUL-terminated. Added 28 October 2026 (fifth review, residual of finding
/// 1: types the registry does not mark, like `.ps1`, `.theme` or `.chm`); approved by the user,
/// 28 October 2026.
fn os_assoc_is_dangerous(assoc: &[u16]) -> bool {
    assert!(assoc.ends_with(&[0]), "NUL-terminated");
    // SAFETY: `assoc` is a NUL-terminated UTF-16 string that lives for the whole call; the call
    // only reads it and keeps no pointer to it.
    #[expect(unsafe_code, reason = "AssocIsDangerous has no safe binding")]
    let r = unsafe { AssocIsDangerous(PCWSTR(assoc.as_ptr())) };
    r.as_bool()
}

/// Whether Windows counts files ending in `ending` (with or without its dot) as dangerous to
/// open (see [`os_assoc_is_dangerous`]). An ending that is empty or holds anything but letters,
/// digits, `-` and `_` is answered `true`: it is not a plain file type, so it is not trusted.
pub fn file_type_is_dangerous(ending: &str) -> bool {
    let e = ending.trim_start_matches('.');
    if e.is_empty()
        || e.len() > 64
        || !e
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return true;
    }
    os_assoc_is_dangerous(&wide(OsStr::new(&format!(".{e}"))))
}

/// Asks Windows itself, with no tool or script in between, whether it trusts the signature
/// inside `file` (open, held by the caller) at `path`. For a Windows program the bytes checked are
/// the held file's; for a Windows Installer package Windows reads `path` again instead (third
/// review, finding 3), so a package's answer is only as good as the folder `path` is in: it
/// counts only for the administrator helper's own copy (see [`os_win_verify_trust`]). A file
/// signed only through a Windows catalog has no signature inside it and is not trusted here. The
/// answer is the system's own and cannot be replaced by code in the person's folders.
pub fn verify_file_trust(file: &File, path: &std::path::Path) -> FileTrust {
    let (code, signer) = os_win_verify_trust(file.as_handle(), &wide(path.as_os_str()));
    FileTrust { code, signer }
}

/// Microsoft Defender's command-line scanner, found where Windows itself records Defender
/// (`InstallLocation` under Defender's key in `HKEY_LOCAL_MACHINE`, which only administrators and
/// Defender can change), or else in `Program Files` as the registry names it: never from an
/// environment value (second review, finding B3).
pub fn defender_scanner() -> Result<PathBuf, Error> {
    let installed = os_machine_text(
        &wide(OsStr::new(r"SOFTWARE\Microsoft\Windows Defender")),
        &wide(OsStr::new("InstallLocation")),
    )
    .ok()
    .map(PathBuf::from)
    .filter(|p| p.is_absolute())
    .map(|p| p.join("MpCmdRun.exe"))
    .filter(|p| p.is_file());
    match installed {
        Some(p) => Ok(p),
        None => {
            let p = system_folder(SystemFolder::ProgramFiles)?
                .join("Windows Defender")
                .join("MpCmdRun.exe");
            if p.is_file() {
                Ok(p)
            } else {
                Err(Error::NotSupported(
                    "Microsoft Defender's scanner is not on this laptop",
                ))
            }
        }
    }
}

/// A folder of the system's own, as Windows names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemFolder {
    /// Where Windows is (`C:\Windows`).
    Windows,
    /// Where 64-bit programs are installed (`C:\Program Files`).
    ProgramFiles,
}

/// Where `which` is, from the kernel or the machine's registry: never from the environment, which
/// whoever starts a program sets, so a mover could point an elevated helper at folders of their
/// own (Security Design part C). Not the shell's known folders either: their paths are written
/// with variables (`%windir%`) the shell fills in from the environment.
pub fn system_folder(which: SystemFolder) -> Result<PathBuf, Error> {
    let path = match which {
        SystemFolder::Windows => {
            os_windows_dir().map_err(|c| os_err("GetSystemWindowsDirectoryW", c))?
        }
        SystemFolder::ProgramFiles => PathBuf::from(
            os_machine_text(
                &wide(OsStr::new(r"SOFTWARE\Microsoft\Windows\CurrentVersion")),
                &wide(OsStr::new("ProgramFilesDir")),
            )
            .map_err(|c| os_err("RegGetValueW", c))?,
        ),
    };
    if !path.is_absolute() {
        return Err(Error::NotSupported(
            "the system named a folder that is not a full path",
        ));
    }
    Ok(path)
}

/// The few environment values a system program needs, every one worked out from Windows' own
/// folder ([`system_folder`]), never passed on from whoever started the helper: the helper runs
/// with an administrator's rights, and a value such as `COR_PROFILER` or `SystemRoot` set by an
/// ordinary program could otherwise make a system program it starts load that program's code
/// (found by the fresh-context review).
pub fn minimal_env() -> Result<Vec<(OsString, OsString)>, Error> {
    let windows = system_folder(SystemFolder::Windows)?;
    let Some(std::path::Component::Prefix(drive)) = windows.components().next() else {
        return Err(Error::NotSupported("the Windows folder has no drive"));
    };
    let system32 = windows.join("System32");
    let path = std::env::join_paths([
        system32.clone(),
        windows.clone(),
        system32.join("Wbem"),
        system32.join(r"WindowsPowerShell\v1.0"),
    ])
    .map_err(|_| Error::NotSupported("a search path"))?;
    let temp = windows.join("Temp");
    Ok(vec![
        ("SystemRoot".into(), windows.clone().into_os_string()),
        ("windir".into(), windows.clone().into_os_string()),
        ("SystemDrive".into(), drive.as_os_str().to_os_string()),
        ("ComSpec".into(), system32.join("cmd.exe").into_os_string()),
        ("PATH".into(), path),
        ("TEMP".into(), temp.clone().into_os_string()),
        ("TMP".into(), temp.into_os_string()),
    ])
}

/// A command for `program` with nothing of this process's environment but [`minimal_env`].
pub fn clean_command(program: &std::path::Path) -> Result<std::process::Command, Error> {
    let mut command = pctwin_types::process::quiet(program);
    command.env_clear().envs(minimal_env()?);
    Ok(command)
}

/// A program in Windows' own `System32`.
pub(crate) fn system_program(name: &str) -> Result<PathBuf, Error> {
    Ok(system_folder(SystemFolder::Windows)?
        .join("System32")
        .join(name))
}

/// System and administrators only, protected from what the folder above allows.
pub(crate) const ADMIN_ONLY: &str = "D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)";

/// A folder `name` inside [`program_folder`], made administrators-only from its first moment
/// (system and administrators only, not inherited), or one that is there already: only
/// administrators can make anything in `Program Files\PCTwin`, so it was made by one. Nothing in
/// it can be opened by an ordinary account, not even to read (the helper's one-helper lock).
pub fn admin_only_folder(name: &str) -> Result<PathBuf, Error> {
    if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
        return Err(Error::BadInput("a folder name"));
    }
    let path = program_folder()?.join(name);
    match create_folder_with_access(&path, ADMIN_ONLY) {
        Ok(()) => Ok(path),
        Err(Error::NameTaken) => {
            let meta = std::fs::symlink_metadata(&path)?;
            if !meta.is_dir() || meta.file_type().is_symlink() {
                return Err(Error::NotPrivate(format!(
                    "{} is not a plain folder",
                    path.display()
                )));
            }
            Ok(path)
        }
        Err(e) => Err(e),
    }
}

/// `Program Files\PCTwin`, made administrators-only if it is missing. Only administrators can
/// make anything in Program Files, so one that exists was made by an administrator (the
/// installer, or an earlier move); it must still be a plain folder, never a link.
pub fn program_folder() -> Result<PathBuf, Error> {
    let path = system_folder(SystemFolder::ProgramFiles)?.join("PCTwin");
    match create_folder_with_access(&path, ADMIN_ONLY) {
        Ok(()) => Ok(path),
        Err(Error::NameTaken) => {
            let meta = std::fs::symlink_metadata(&path)?;
            if !meta.is_dir() || meta.file_type().is_symlink() {
                return Err(Error::NotPrivate(format!(
                    "{} is not a plain folder",
                    path.display()
                )));
            }
            Ok(path)
        }
        Err(e) => Err(e),
    }
}

/// Refuses a program outside Program Files. Elevating a program its own account can replace would
/// let any program that account runs swap it in the moment before the administrator approves; in
/// Program Files only administrators can change a file. The request's path is already the one
/// the system resolved (no link, junction or short name left), and is compared with Program
/// Files as the system resolves it, as found from the machine's registry (never the environment).
pub(crate) fn only_from_program_files(program: &std::path::Path) -> Result<(), Error> {
    let root = std::fs::canonicalize(system_folder(SystemFolder::ProgramFiles)?)?;
    let real = std::fs::canonicalize(program)?;
    let inside = real
        .strip_prefix(&root)
        .is_ok_and(|rest| rest.components().count() >= 2);
    if inside {
        Ok(())
    } else {
        Err(Error::NotPrivate(format!(
            "{} is not inside {}: only a program there is started with administrator rights",
            program.display(),
            root.display()
        )))
    }
}

pub(crate) fn start_runas(request: &LaunchRequest) -> Result<Process, LaunchError> {
    only_from_program_files(request.program()).map_err(LaunchError::Failed)?;
    let file = wide(request.program().as_os_str());
    let parameters = wide(OsStr::new(&request.windows_parameters()));
    match os_shell_execute_runas(&file, &parameters) {
        Ok((handle, id)) => Ok(Process::from_handle(handle, id)),
        Err(code) if code == ERROR_CANCELLED.to_hresult() => Err(LaunchError::Cancelled),
        Err(code) => Err(LaunchError::Failed(os_err("ShellExecuteExW", code))),
    }
}

pub(crate) fn wait(process: &OwnedHandle, millis: Option<u32>) -> Result<Option<Ended>, Error> {
    os_wait(process.as_handle(), millis.unwrap_or(INFINITE))
        .map(|code| code.map(|c| Ended::Exited(c as i32)))
        .map_err(|c| os_err("WaitForSingleObject", c))
}

/// Whether this program runs with an administrator's full rights.
pub(crate) fn this_process_elevated() -> Result<bool, Error> {
    let me = os_open_process(std::process::id()).map_err(|c| os_err("OpenProcess", c))?;
    let token = os_process_token(me.as_handle()).map_err(|c| os_err("OpenProcessToken", c))?;
    os_token_elevated(token.as_handle()).map_err(|c| os_err("GetTokenInformation", c))
}

/// Whether this program's session is on the screen (see [`os_session_on_screen`]).
pub(crate) fn session_on_screen() -> Result<bool, Error> {
    os_session_on_screen().map_err(|c| os_err("WTSQuerySessionInformationW", c))
}

pub(crate) fn current_sid() -> Result<Sid, Error> {
    let me = os_open_process(std::process::id()).map_err(|c| os_err("OpenProcess", c))?;
    let token = os_process_token(me.as_handle()).map_err(|c| os_err("OpenProcessToken", c))?;
    Sid::parse(&os_token_user(token.as_handle()).map_err(|c| os_err("GetTokenInformation", c))?)
}

/// A fixed drive's kind, as `GetDriveTypeW` numbers it.
pub(crate) const DRIVE_FIXED: u32 = 3;

/// The drive path a resolved program path names, if Windows classes the volume holding it as a
/// fixed drive: `\\?\C:\...` becomes `C:\...`. Refused: a network (`\\?\UNC\...`), device or
/// volume path, a data stream, and any file on a volume Windows does not report as fixed
/// (removable, optical, network, RAM disk). This is Windows' own classification of the volume
/// the file is on (found through its mount point): a USB disk that reports itself as fixed
/// passes.
pub(crate) fn local_fixed_drive_path(resolved: &std::path::Path) -> Result<PathBuf, Error> {
    drive_letter_of_resolved(resolved).ok_or(Error::BadInput(
        "the program is not on a drive of this computer",
    ))?;
    let kind = os_volume_drive_type(&wide(resolved.as_os_str()))
        .map_err(|c| os_err("GetVolumePathNameW", c))?;
    if !drive_kind_allowed(kind) {
        return Err(Error::BadInput(
            "the program is not on a fixed drive of this computer",
        ));
    }
    let text = resolved.to_string_lossy();
    Ok(PathBuf::from(&text[4..]))
}

/// Only a fixed drive.
pub(crate) fn drive_kind_allowed(kind: u32) -> bool {
    kind == DRIVE_FIXED
}

/// The drive letter of a path as the system resolves it (`\\?\C:\...`), with no data stream.
pub(crate) fn drive_letter_of_resolved(resolved: &std::path::Path) -> Option<char> {
    use std::path::{Component, Prefix};
    let mut parts = resolved.components();
    let letter = match parts.next() {
        Some(Component::Prefix(p)) => match p.kind() {
            Prefix::VerbatimDisk(d) => char::from(d),
            _ => return None,
        },
        _ => return None,
    };
    if !matches!(parts.next(), Some(Component::RootDir)) {
        return None;
    }
    for c in parts {
        match c {
            Component::Normal(n) if !n.to_string_lossy().contains(':') => {}
            _ => return None,
        }
    }
    Some(letter)
}

/// The name of the helper's pipe: `pctwin-helper-` and 32 random hexadecimal characters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PipeName(String);

impl PipeName {
    /// A new name from the system's random source.
    pub fn random() -> Result<PipeName, Error> {
        Ok(PipeName(format!("{NAME_PREFIX}{}", crate::random_token()?)))
    }

    /// Checks a name passed to the helper on its command line.
    pub fn parse(s: &str) -> Result<PipeName, Error> {
        match s.strip_prefix(NAME_PREFIX) {
            Some(t)
                if t.len() == 32 && t.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) =>
            {
                Ok(PipeName(s.to_owned()))
            }
            _ => Err(Error::BadInput("not a helper pipe name")),
        }
    }

    /// The name alone.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The full local path, `\\.\pipe\<name>`.
    pub fn path(&self) -> String {
        format!(r"\\.\pipe\{}", self.0)
    }
}

/// The helper's end of the pipe: one instance, one client.
#[derive(Debug)]
pub struct PipeServer {
    file: File,
}

impl PipeServer {
    /// Makes the pipe, admitting only `mover` and SYSTEM. [`Error::NameTaken`] if a pipe of this
    /// name already exists: it is never joined.
    pub fn create(name: &PipeName, mover: &Sid) -> Result<PipeServer, Error> {
        let path = wide(OsStr::new(&name.path()));
        let sddl = wide(OsStr::new(&pipe_security(mover)));
        match os_create_pipe(&path, &sddl) {
            Ok(h) => Ok(PipeServer {
                file: File::from(h),
            }),
            Err(c)
                if c == ERROR_ACCESS_DENIED.to_hresult() || c == ERROR_PIPE_BUSY.to_hresult() =>
            {
                Err(Error::NameTaken)
            }
            Err(c) => Err(os_err("CreateNamedPipeW", c)),
        }
    }

    /// Waits until the client has connected.
    pub fn accept(&self) -> Result<(), Error> {
        os_accept(self.file.as_handle()).map_err(|c| os_err("ConnectNamedPipe", c))
    }

    /// The pipe's access list as the system holds it, in its text form (for checking).
    pub fn access_list(&self) -> Result<String, Error> {
        os_access_list(self.file.as_handle()).map_err(|c| os_err("GetSecurityInfo", c))
    }

    /// The pipe, to read from and write to.
    pub fn stream(&self) -> &File {
        &self.file
    }
}

/// Who is on the other end of the helper's pipe, as soon as the client has connected.
///
/// Only [`Caller::account`] is authoritative: it comes from the token the client connected
/// with, which the system holds for this connection, not from a lookup by process ID.
/// [`Caller::process_id`] is the ID the system recorded at connection, and
/// [`Caller::program_hint`] is read by that ID afterwards, so it is a hint for the record and
/// the report, never for allowing anything. Fails with [`Error::CallerGone`] if that process has
/// ended.
pub fn caller_of(pipe: &PipeServer) -> Result<Caller, Error> {
    let id = os_client_process_id(pipe.file.as_handle())
        .map_err(|c| os_err("GetNamedPipeClientProcessId", c))?;
    let token = os_client_token(pipe.file.as_handle());
    let process = os_open_process(id).map_err(|_| Error::CallerGone)?;
    let program = os_image_path(process.as_handle());
    // Asked last, so a hint read from a process that has meanwhile ended is never reported (an
    // ended process also makes the reads above fail).
    let running = os_wait(process.as_handle(), 0)
        .map_err(|c| os_err("WaitForSingleObject", c))?
        .is_none();
    if !running {
        return Err(Error::CallerGone);
    }
    let token = token.map_err(|c| os_err("ImpersonateNamedPipeClient", c))?;
    let sid = os_token_user(token.as_handle()).map_err(|c| os_err("GetTokenInformation", c))?;
    Ok(Caller {
        process_id: id,
        account: Account::Sid(Sid::parse(&sid)?),
        program_hint: program.map_err(|c| os_err("QueryFullProcessImageNameW", c))?,
    })
}

/// The mover's end of the pipe.
#[derive(Debug)]
pub struct PipeClient {
    file: File,
}

impl PipeClient {
    /// Connects to the helper's pipe, asking for exactly [`CLIENT_ACCESS`] (the pipe grants no
    /// more) and letting the server only identify this process, never act as it
    /// (`SECURITY_IDENTIFICATION`).
    pub fn connect(name: &PipeName) -> Result<PipeClient, Error> {
        let file = std::fs::OpenOptions::new()
            .access_mode(CLIENT_ACCESS)
            .security_qos_flags(SECURITY_IDENTIFICATION.0)
            .open(name.path())?;
        Ok(PipeClient { file })
    }

    /// The process ID of the pipe's server: the app checks it is the helper it started.
    pub fn server_process_id(&self) -> Result<u32, Error> {
        os_server_process_id(self.file.as_handle())
            .map_err(|c| os_err("GetNamedPipeServerProcessId", c))
    }

    /// The pipe, to read from and write to.
    pub fn stream(&self) -> &File {
        &self.file
    }
}

/// A new folder `path` that only this account (and the system itself) may open, made so from its
/// first moment: protected from the folder above (nothing inherited), never one that exists.
pub fn private_folder(path: &std::path::Path) -> Result<(), Error> {
    let me = current_sid()?;
    create_folder_with_access(
        path,
        &format!("D:P(A;OICI;FA;;;{})(A;OICI;FA;;;SY)", me.as_str()),
    )
}

/// The access list a folder has now, in its text form (read back to check it).
pub fn folder_access_list(path: &std::path::Path) -> Result<String, Error> {
    // READ_CONTROL, and the flag that lets a folder be opened as a handle.
    const READ_CONTROL: u32 = 0x0002_0000;
    const BACKUP_SEMANTICS: u32 = 0x0200_0000;
    let f = std::fs::OpenOptions::new()
        .access_mode(READ_CONTROL)
        .custom_flags(BACKUP_SEMANTICS)
        .open(path)?;
    os_access_list(f.as_handle()).map_err(|c| os_err("GetSecurityInfo", c))
}

/// Each program holding a file: its process id and its name as Windows shows it (UTF-16).
type RmHolder = (u32, Vec<u16>);

/// Windows Restart Manager's own list of the running programs that have any of `files` open
/// (each a NUL-terminated UTF-16 full path): each program's id and the name Windows shows for it.
/// The session is started, the files registered, the list read (asked again, at most twice, when
/// more programs appeared meanwhile) and the session ended, all in one call; nothing is closed or
/// restarted. Needs no administrator: the person's own programs are listed. Added 29 October 2026
/// for the 2.5 decision on files live while their app is open (the person is asked to close only
/// the app holding a file, by its name); approved by the founder, 29 October 2026.
fn os_rm_holders(files: &[Vec<u16>]) -> Result<Vec<RmHolder>, (&'static str, u32)> {
    use windows::Win32::Foundation::ERROR_MORE_DATA;
    use windows::Win32::System::RestartManager::{
        CCH_RM_SESSION_KEY, RM_PROCESS_INFO, RmEndSession, RmGetList, RmRegisterResources,
        RmStartSession,
    };
    assert!(files.iter().all(|f| f.ends_with(&[0])), "NUL-terminated");
    let names: Vec<PCWSTR> = files.iter().map(|f| PCWSTR(f.as_ptr())).collect();
    let mut key = [0u16; CCH_RM_SESSION_KEY as usize + 1];
    let mut session = 0u32;
    let mut infos: Vec<RM_PROCESS_INFO> = Vec::new();
    // SAFETY: `key` has room for the session key and its NUL, as RmStartSession requires; every
    // name in `names` points into `files`, which outlives the call and is only read; RmGetList is
    // given `infos`' own length as its room, and a null list only when that length is zero; the
    // session is always ended before the block ends, whatever failed. No pointer is kept.
    #[expect(unsafe_code, reason = "Restart Manager has no safe binding")]
    let got = unsafe {
        let started = RmStartSession(&mut session, None, PWSTR(key.as_mut_ptr()));
        if started != ERROR_SUCCESS {
            Err(("RmStartSession", started.0))
        } else {
            let listed = 'list: {
                let registered = RmRegisterResources(session, Some(&names), None, None);
                if registered != ERROR_SUCCESS {
                    break 'list Err(("RmRegisterResources", registered.0));
                }
                for _ in 0..3 {
                    let mut needed = 0u32;
                    let mut count = u32::try_from(infos.len()).unwrap_or(0);
                    let mut reasons = 0u32;
                    let room = if infos.is_empty() {
                        None
                    } else {
                        Some(infos.as_mut_ptr())
                    };
                    let r = RmGetList(session, &mut needed, &mut count, room, &mut reasons);
                    if r == ERROR_SUCCESS {
                        infos.truncate(count as usize);
                        break 'list Ok(());
                    }
                    if r != ERROR_MORE_DATA {
                        break 'list Err(("RmGetList", r.0));
                    }
                    infos = vec![RM_PROCESS_INFO::default(); needed as usize + 4];
                }
                Err(("RmGetList", ERROR_MORE_DATA.0))
            };
            let _ = RmEndSession(session);
            listed
        }
    };
    got?;
    Ok(infos
        .iter()
        .map(|i| (i.Process.dwProcessId, i.strAppName.to_vec()))
        .collect())
}

/// The names Windows shows for the programs, other than PCTwin itself, that have any of `files`
/// open (Restart Manager; [`os_rm_holders`]), each once, in the order Windows gives them. A name
/// Windows leaves empty is not listed. Holding a file open is not the same as writing to it: a
/// caller that needs to know whether a file is being written asks that first (a share-mode open)
/// and asks this only for the names.
pub fn file_holders(files: &[std::path::PathBuf]) -> Result<Vec<String>, Error> {
    if files.is_empty() {
        return Ok(Vec::new());
    }
    let wide_names: Vec<Vec<u16>> = files.iter().map(|f| wide(f.as_os_str())).collect();
    let got = os_rm_holders(&wide_names).map_err(|(call, code)| Error::Os {
        call,
        code: i64::from(code),
    })?;
    let me = std::process::id();
    let mut names: Vec<String> = Vec::new();
    for (pid, raw) in got {
        if pid == me {
            continue;
        }
        let end = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
        let name = String::from_utf16_lossy(&raw[..end]).trim().to_string();
        if !name.is_empty() && !names.contains(&name) {
            names.push(name);
        }
    }
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pipe_refuses_remote_clients_and_squatted_names() {
        assert_ne!(pipe_mode().0 & PIPE_REJECT_REMOTE_CLIENTS.0, 0);
        assert_ne!(open_mode().0 & FILE_FLAG_FIRST_PIPE_INSTANCE.0, 0);
        assert_eq!(open_mode().0 & PIPE_ACCESS_DUPLEX.0, PIPE_ACCESS_DUPLEX.0);
    }

    #[test]
    fn clients_may_never_make_pipe_instances() {
        const FILE_CREATE_PIPE_INSTANCE: u32 = 0x4;
        const GENERIC_ALL: u32 = 0x1000_0000;
        const GENERIC_WRITE: u32 = 0x4000_0000;
        const WRITE_DAC: u32 = 0x0004_0000;
        const WRITE_OWNER: u32 = 0x0008_0000;
        for bit in [
            FILE_CREATE_PIPE_INSTANCE,
            GENERIC_ALL,
            GENERIC_WRITE,
            WRITE_DAC,
            WRITE_OWNER,
        ] {
            assert_eq!(CLIENT_ACCESS & bit, 0, "{bit:#x}");
        }
        let mover = Sid::parse("S-1-5-21-1-2-3-1001").unwrap();
        assert_eq!(
            pipe_security(&mover),
            "D:P(A;;0x12008b;;;S-1-5-21-1-2-3-1001)(A;;0x12008b;;;SY)"
        );
    }

    #[test]
    fn only_fixed_drive_paths_as_resolved_are_programs() {
        use std::path::Path;
        assert_eq!(
            drive_letter_of_resolved(Path::new(r"\\?\C:\Program Files\PCTwin\h.exe")),
            Some('C')
        );
        for bad in [
            r"\\?\UNC\server\share\h.exe",
            r"\\server\share\h.exe",
            r"C:\h.exe",
            r"\\.\C:\h.exe",
            r"\\?\Volume{0b1e2c3d-0000-0000-0000-100000000000}\h.exe",
            r"\\?\C:\dir\h.exe:stream",
            r"\\?\GLOBALROOT\Device\HarddiskVolume1\h.exe",
        ] {
            assert_eq!(drive_letter_of_resolved(Path::new(bad)), None, "{bad}");
        }
        assert!(drive_kind_allowed(DRIVE_FIXED));
        for kind in [0, 1, 2, 4, 5, 6] {
            assert!(!drive_kind_allowed(kind), "{kind}");
        }
    }

    #[test]
    fn pipe_names_are_strict() {
        let n = PipeName::random().unwrap();
        assert_eq!(PipeName::parse(n.as_str()).unwrap(), n);
        assert!(n.path().starts_with(r"\\.\pipe\pctwin-helper-"));
        for bad in [
            "pctwin-helper-",
            "pctwin-helper-0123456789abcdef0123456789abcde",
            "pctwin-helper-0123456789abcdef0123456789abcdef0",
            "pctwin-helper-0123456789ABCDEF0123456789abcdef",
            "pctwin-helper-0123456789abcdef0123456789abcde\\",
            "other-0123456789abcdef0123456789abcdef",
            r"..\pctwin-helper-0123456789abcdef0123456789abcdef",
        ] {
            assert!(PipeName::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn restart_manager_names_the_program_holding_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let free = dir.path().join("free.db");
        std::fs::write(&free, b"x").unwrap();
        // Nobody holds it: nobody is named (PCTwin's own process is never listed).
        assert_eq!(
            super::file_holders(std::slice::from_ref(&free)).unwrap(),
            Vec::<String>::new()
        );
        assert_eq!(super::file_holders(&[]).unwrap(), Vec::<String>::new());
        // A child program holds it open: Windows names it.
        let held = dir.path().join("held.db");
        std::fs::write(&held, b"x").unwrap();
        // A program writing into it for half a minute (its output goes there).
        let mut child = std::process::Command::new("ping")
            .args(["-n", "30", "127.0.0.1"])
            .stdout(std::fs::File::create(&held).unwrap())
            .spawn()
            .unwrap();
        let mut named = Vec::new();
        for _ in 0..50 {
            named = super::file_holders(std::slice::from_ref(&held)).unwrap();
            if !named.is_empty() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let _ = child.kill();
        let _ = child.wait();
        assert!(!named.is_empty(), "the holder is named");
    }
}
