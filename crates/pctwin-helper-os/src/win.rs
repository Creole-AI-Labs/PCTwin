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
use windows::Win32::Security::Authorization::{
    ConvertSecurityDescriptorToStringSecurityDescriptorW, ConvertSidToStringSidW,
    ConvertStringSecurityDescriptorToSecurityDescriptorW, GetSecurityInfo, SDDL_REVISION_1,
    SE_KERNEL_OBJECT,
};
use windows::Win32::Security::{
    DACL_SECURITY_INFORMATION, GetTokenInformation, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES,
    TOKEN_QUERY, TOKEN_USER, TokenUser,
};
use windows::Win32::Storage::FileSystem::{
    FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_FLAGS_AND_ATTRIBUTES, PIPE_ACCESS_DUPLEX,
    SECURITY_IDENTIFICATION,
};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, GetNamedPipeClientProcessId, GetNamedPipeServerProcessId,
    NAMED_PIPE_MODE, PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_WAIT,
};
use windows::Win32::System::Threading::{
    GetExitCodeProcess, GetProcessId, INFINITE, OpenProcess, OpenProcessToken, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, QueryFullProcessImageNameW,
    WaitForSingleObject,
};
use windows::Win32::UI::Shell::{
    SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW,
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

/// The account (its security identifier, as text) the process runs as.
fn os_token_user(process: BorrowedHandle<'_>) -> Result<String, HRESULT> {
    let h = raw(process);
    // Room for TOKEN_USER and the largest identifier (68 bytes), aligned for TOKEN_USER.
    let mut buffer = [0u64; 64];
    let mut needed = 0u32;
    // SAFETY: the token handle is wrapped (closed once, on drop) as soon as it exists. The
    // buffer is aligned and its true size is passed, so GetTokenInformation cannot overrun it,
    // and on success it holds a TOKEN_USER whose identifier points inside the buffer, read while
    // the buffer lives. The text the system allocates is copied out, then freed once.
    #[expect(
        unsafe_code,
        reason = "reading a process token's user has no safe binding"
    )]
    let r = unsafe {
        let mut token = HANDLE::default();
        OpenProcessToken(h, TOKEN_QUERY, &mut token)
            .map_err(|e| e.code())
            .and_then(|()| {
                let token = OwnedHandle::from_raw_handle(token.0);
                GetTokenInformation(
                    HANDLE(token.as_raw_handle()),
                    TokenUser,
                    Some(buffer.as_mut_ptr().cast()),
                    std::mem::size_of_val(&buffer) as u32,
                    &mut needed,
                )
                .map_err(|e| e.code())?;
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

// ---- Safe code from here on. ----

pub(crate) fn start_runas(request: &LaunchRequest) -> Result<Process, LaunchError> {
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

pub(crate) fn current_sid() -> Result<Sid, Error> {
    let me = os_open_process(std::process::id()).map_err(|c| os_err("OpenProcess", c))?;
    Sid::parse(&os_token_user(me.as_handle()).map_err(|c| os_err("OpenProcessToken", c))?)
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

/// Who is on the other end of the helper's pipe: process ID, account and program, read from the
/// system for this connection. Fails with [`Error::CallerGone`] if that process has ended.
///
/// Not closed here (for the fresh-context review): a process ID is reused once its process has
/// ended and every handle to it is closed; this opens the process and then checks it still
/// runs, which narrows but does not close the gap of a client that ends just before it is
/// opened, leaving its pipe handle with a child.
pub fn caller_of(pipe: &PipeServer) -> Result<Caller, Error> {
    let id = os_client_process_id(pipe.file.as_handle())
        .map_err(|c| os_err("GetNamedPipeClientProcessId", c))?;
    let process = os_open_process(id).map_err(|_| Error::CallerGone)?;
    let sid = os_token_user(process.as_handle());
    let program = os_image_path(process.as_handle());
    // Asked last, so an answer read from a process that has meanwhile ended is never vouched
    // for (an ended process also makes the two reads above fail).
    let running = os_wait(process.as_handle(), 0)
        .map_err(|c| os_err("WaitForSingleObject", c))?
        .is_none();
    if !running {
        return Err(Error::CallerGone);
    }
    Ok(Caller {
        process_id: id,
        account: Account::Sid(Sid::parse(
            &sid.map_err(|c| os_err("OpenProcessToken", c))?,
        )?),
        program: program.map_err(|c| os_err("QueryFullProcessImageNameW", c))?,
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
}
