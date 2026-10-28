//! Windows: starting PCTwin's parser process walled in (Phase 2 adoption, change 1; the system's
//! walls approved by the user, 28 October 2026).
//!
//! The parser is started:
//! - in an AppContainer with no capabilities: no network, none of the person's files, low
//!   integrity (the profile and its SID through `rappct`; the start itself here);
//! - with a restricted token: the administrators group deny-only and every privilege but
//!   bypass-traverse removed, so a parser started by the administrator helper never holds an
//!   administrator's rights;
//! - forbidden to start any program of its own (the child-process policy), with only its two
//!   pipes inherited, only Windows' own environment values and no console (so no console host
//!   process either);
//! - inside a job object that ends it when the job's handle closes (with PCTwin, or when PCTwin
//!   ends it), allows one process, caps its memory and ends it on a crash without a dialog.
//!
//! It starts suspended and runs only once it is in the job.

use std::ffi::OsStr;
use std::fs::File;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::{FromRawHandle, OwnedHandle};
use std::path::Path;

use windows::Win32::Foundation::{
    HANDLE, HANDLE_FLAG_INHERIT, HLOCAL, LocalFree, SetHandleInformation,
};
use windows::Win32::Security::Authorization::ConvertStringSidToSidW;
use windows::Win32::Security::{
    CreateRestrictedToken, CreateWellKnownSid, DISABLE_MAX_PRIVILEGE, PSID, SECURITY_ATTRIBUTES,
    SECURITY_CAPABILITIES, SECURITY_MAX_SID_SIZE, SID_AND_ATTRIBUTES, TOKEN_ADJUST_DEFAULT,
    TOKEN_ASSIGN_PRIMARY, TOKEN_DUPLICATE, TOKEN_QUERY, WinBuiltinAdministratorsSid,
};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_ACTIVE_PROCESS,
    JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOB_OBJECT_LIMIT_PROCESS_MEMORY, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JobObjectExtendedLimitInformation, SetInformationJobObject,
};
use windows::Win32::System::Pipes::CreatePipe;
use windows::Win32::System::Threading::{
    CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, CreateProcessAsUserW, DETACHED_PROCESS,
    DeleteProcThreadAttributeList, EXTENDED_STARTUPINFO_PRESENT, GetCurrentProcess,
    InitializeProcThreadAttributeList, LPPROC_THREAD_ATTRIBUTE_LIST, OpenProcessToken,
    PROC_THREAD_ATTRIBUTE_CHILD_PROCESS_POLICY, PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
    PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES, PROCESS_INFORMATION, ResumeThread,
    STARTF_USESTDHANDLES, STARTUPINFOEXW, UpdateProcThreadAttribute,
};
use windows::core::{HRESULT, PCWSTR, PWSTR};

use crate::Error;

/// The AppContainer the parser runs in (its profile, made once per account).
pub const PARSER_CONTAINER: &str = "CreoleAILabs.PCTwin.Parser";

/// `PROCESS_CREATION_CHILD_PROCESS_RESTRICTED`: the process may start no program.
const CHILD_PROCESS_RESTRICTED: u32 = 0x01;

/// A parser process walled in. Dropping it ends the process (its job's last handle closes).
#[derive(Debug)]
pub struct Contained {
    /// What the parser reads (its standard input).
    pub to_child: File,
    /// What the parser says (its standard output).
    pub from_child: File,
    _process: OwnedHandle,
    _job: OwnedHandle,
}

/// What keeps a walled-in parser alive: dropping it ends the process.
#[derive(Debug)]
pub struct Guard {
    _process: OwnedHandle,
    _job: OwnedHandle,
}

impl Contained {
    /// Its two pipes, and the guard that ends it when dropped.
    pub fn split(self) -> (File, File, Guard) {
        (
            self.to_child,
            self.from_child,
            Guard {
                _process: self._process,
                _job: self._job,
            },
        )
    }
}

fn wide(s: &OsStr) -> Vec<u16> {
    s.encode_wide().chain(std::iter::once(0)).collect()
}

/// Starts `program` walled in (see the module notes), its memory capped at `memory` bytes.
pub fn start_contained(program: &Path, memory: usize) -> Result<Contained, Error> {
    // The profile is made (or found) once per run: making it from several threads at once fails.
    static PROFILE: std::sync::OnceLock<
        Result<rappct::profile::AppContainerProfile, &'static str>,
    > = std::sync::OnceLock::new();
    let profile = PROFILE
        .get_or_init(|| {
            rappct::profile::AppContainerProfile::ensure(
                PARSER_CONTAINER,
                "PCTwin file checker",
                Some("Reads the insides of untrusted files for PCTwin, walled in"),
            )
            .map_err(|e| leak(format!("the container's profile: {e}")))
        })
        .as_ref()
        .map_err(|e| Error::NotSupported(e))?;
    // A program outside Program Files and Windows' own folder (a build being tested) must be
    // made readable from inside the container; those folders already are, to every container.
    // If it cannot be, the start below fails and the file is not checked.
    let windows_dir = crate::win::system_folder(crate::win::SystemFolder::Windows)?;
    let readable_already = crate::win::system_folder(crate::win::SystemFolder::ProgramFiles)
        .is_ok_and(|pf| program.starts_with(pf))
        || program.starts_with(&windows_dir);
    if !readable_already {
        let _ = rappct::acl::grant_to_package(
            rappct::acl::ResourcePath::File(program.to_path_buf()),
            &profile.sid,
            rappct::acl::AccessMask(0x0012_00A9),
        );
    }
    // Windows' own folders only (worked out from the kernel, never passed on).
    let mut env: Vec<u16> = Vec::new();
    let mut vars = crate::win::minimal_env()?;
    for k in ["LOCALAPPDATA", "USERPROFILE"] {
        if let Some(v) = std::env::var_os(k) {
            vars.push((k.into(), v));
        }
    }
    for (k, v) in vars {
        env.extend(k.encode_wide());
        env.push(u16::from(b'='));
        env.extend(v.encode_wide());
        env.push(0);
    }
    env.push(0);
    let (process, job, to_child, from_child) = os_start_contained(
        &wide(program.as_os_str()),
        &wide(windows_dir.join("System32").as_os_str()),
        &env,
        &wide(OsStr::new(profile.sid.as_string())),
        memory,
    )
    .map_err(|(call, code)| Error::Os {
        call,
        code: i64::from(code.0),
    })?;
    Ok(Contained {
        to_child: File::from(to_child),
        from_child: File::from(from_child),
        _process: process,
        _job: job,
    })
}

/// A message kept for the life of the program (errors name a fixed set of steps).
fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

type Started = (OwnedHandle, OwnedHandle, OwnedHandle, OwnedHandle);

/// The start itself: the restricted token, the container's attributes, the job, the process
/// (suspended until it is in the job). Returns the process, the job, the pipe the parser reads
/// and the pipe it writes. Added 28 October 2026 for the parser's walls; approved by the user,
/// 28 October 2026.
fn os_start_contained(
    program: &[u16],
    cwd: &[u16],
    env: &[u16],
    sid: &[u16],
    memory: usize,
) -> Result<Started, (&'static str, HRESULT)> {
    assert!(
        program.ends_with(&[0])
            && cwd.ends_with(&[0])
            && sid.ends_with(&[0])
            && env.ends_with(&[0, 0]),
        "NUL-terminated"
    );
    // SAFETY: every string passed is NUL-terminated (checked above) and lives for the whole
    // call. Every handle the system gives is wrapped in an OwnedHandle at once, so each is closed
    // exactly once on every way out; the SIDs and the attribute list are freed once, after their
    // last use. The attribute list's buffer is the size the system asked for, and every value it
    // points to (the capabilities, the two inherited handles, the policy) lives until the process
    // is made. The process is made suspended, put in the job, and only then resumed.
    #[expect(
        unsafe_code,
        reason = "starting a process in an AppContainer with a restricted token has no safe binding"
    )]
    let r = unsafe {
        let fail = |call: &'static str| move |e: windows::core::Error| (call, e.code());
        let owned = |h: HANDLE| OwnedHandle::from_raw_handle(h.0);

        // The container's SID.
        let mut container = PSID::default();
        ConvertStringSidToSidW(PCWSTR(sid.as_ptr()), &mut container)
            .map_err(fail("ConvertStringSidToSidW"))?;
        let container_sid = container;
        let free_container = move || {
            let _ = LocalFree(Some(HLOCAL(container_sid.0)));
        };

        let made = (|| -> Result<Started, (&'static str, HRESULT)> {
            // The two pipes; only the parser's ends are inheritable.
            let sa = SECURITY_ATTRIBUTES {
                nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: std::ptr::null_mut(),
                bInheritHandle: true.into(),
            };
            let (mut in_r, mut in_w) = (HANDLE::default(), HANDLE::default());
            CreatePipe(&mut in_r, &mut in_w, Some(&sa), 0).map_err(fail("CreatePipe"))?;
            let (in_r, in_w) = (owned(in_r), owned(in_w));
            let (mut out_r, mut out_w) = (HANDLE::default(), HANDLE::default());
            CreatePipe(&mut out_r, &mut out_w, Some(&sa), 0).map_err(fail("CreatePipe"))?;
            let (out_r, out_w) = (owned(out_r), owned(out_w));
            for mine in [&in_w, &out_r] {
                SetHandleInformation(
                    HANDLE(std::os::windows::io::AsRawHandle::as_raw_handle(mine)),
                    HANDLE_FLAG_INHERIT.0,
                    Default::default(),
                )
                .map_err(fail("SetHandleInformation"))?;
            }
            let raw = |h: &OwnedHandle| HANDLE(std::os::windows::io::AsRawHandle::as_raw_handle(h));

            // The restricted token: administrators deny-only, privileges removed.
            let mut own = HANDLE::default();
            OpenProcessToken(
                GetCurrentProcess(),
                TOKEN_DUPLICATE | TOKEN_ASSIGN_PRIMARY | TOKEN_QUERY | TOKEN_ADJUST_DEFAULT,
                &mut own,
            )
            .map_err(fail("OpenProcessToken"))?;
            let own = owned(own);
            let mut admins = [0u8; SECURITY_MAX_SID_SIZE as usize];
            let mut admins_len = admins.len() as u32;
            CreateWellKnownSid(
                WinBuiltinAdministratorsSid,
                None,
                Some(PSID(admins.as_mut_ptr().cast())),
                &mut admins_len,
            )
            .map_err(fail("CreateWellKnownSid"))?;
            let deny = [SID_AND_ATTRIBUTES {
                Sid: PSID(admins.as_mut_ptr().cast()),
                Attributes: 0,
            }];
            let mut restricted = HANDLE::default();
            CreateRestrictedToken(
                raw(&own),
                DISABLE_MAX_PRIVILEGE,
                Some(&deny),
                None,
                None,
                &mut restricted,
            )
            .map_err(fail("CreateRestrictedToken"))?;
            let restricted = owned(restricted);

            // The attributes: the container, the two handles it inherits, no programs of its own.
            let caps = SECURITY_CAPABILITIES {
                AppContainerSid: container_sid,
                Capabilities: std::ptr::null_mut(),
                CapabilityCount: 0,
                Reserved: 0,
            };
            let inherit = [raw(&in_r), raw(&out_w)];
            let policy: u32 = CHILD_PROCESS_RESTRICTED;
            let mut size = 0usize;
            let _ = InitializeProcThreadAttributeList(None, 3, None, &mut size);
            let mut buffer = vec![0u8; size];
            let list = LPPROC_THREAD_ATTRIBUTE_LIST(buffer.as_mut_ptr().cast());
            InitializeProcThreadAttributeList(Some(list), 3, None, &mut size)
                .map_err(fail("InitializeProcThreadAttributeList"))?;
            let started = (|| -> Result<Started, (&'static str, HRESULT)> {
                UpdateProcThreadAttribute(
                    list,
                    0,
                    PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES as usize,
                    Some(std::ptr::from_ref(&caps).cast()),
                    std::mem::size_of::<SECURITY_CAPABILITIES>(),
                    None,
                    None,
                )
                .map_err(fail("UpdateProcThreadAttribute"))?;
                UpdateProcThreadAttribute(
                    list,
                    0,
                    PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
                    Some(inherit.as_ptr().cast()),
                    std::mem::size_of_val(&inherit),
                    None,
                    None,
                )
                .map_err(fail("UpdateProcThreadAttribute"))?;
                UpdateProcThreadAttribute(
                    list,
                    0,
                    PROC_THREAD_ATTRIBUTE_CHILD_PROCESS_POLICY as usize,
                    Some(std::ptr::from_ref(&policy).cast()),
                    std::mem::size_of::<u32>(),
                    None,
                    None,
                )
                .map_err(fail("UpdateProcThreadAttribute"))?;

                // The job: ends it when closed, one process, memory capped, no crash dialog.
                let job = owned(
                    CreateJobObjectW(None, PCWSTR::null()).map_err(fail("CreateJobObjectW"))?,
                );
                let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
                limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
                    | JOB_OBJECT_LIMIT_ACTIVE_PROCESS
                    | JOB_OBJECT_LIMIT_PROCESS_MEMORY
                    | JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION;
                limits.BasicLimitInformation.ActiveProcessLimit = 1;
                limits.ProcessMemoryLimit = memory;
                SetInformationJobObject(
                    raw(&job),
                    JobObjectExtendedLimitInformation,
                    std::ptr::from_ref(&limits).cast(),
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )
                .map_err(fail("SetInformationJobObject"))?;

                let mut si = STARTUPINFOEXW::default();
                si.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
                si.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
                si.StartupInfo.hStdInput = raw(&in_r);
                si.StartupInfo.hStdOutput = raw(&out_w);
                si.lpAttributeList = list;
                let mut line: Vec<u16> = std::iter::once(u16::from(b'"'))
                    .chain(program[..program.len() - 1].iter().copied())
                    .chain([u16::from(b'"'), 0])
                    .collect();
                let mut pi = PROCESS_INFORMATION::default();
                CreateProcessAsUserW(
                    Some(raw(&restricted)),
                    PCWSTR(program.as_ptr()),
                    Some(PWSTR(line.as_mut_ptr())),
                    None,
                    None,
                    true,
                    EXTENDED_STARTUPINFO_PRESENT
                        | CREATE_SUSPENDED
                        | CREATE_UNICODE_ENVIRONMENT
                        | DETACHED_PROCESS,
                    Some(env.as_ptr().cast()),
                    PCWSTR(cwd.as_ptr()),
                    &si.StartupInfo,
                    &mut pi,
                )
                .map_err(fail("CreateProcessAsUserW"))?;
                let process = owned(pi.hProcess);
                let thread = owned(pi.hThread);
                if let Err(e) = AssignProcessToJobObject(raw(&job), raw(&process)) {
                    // Not in the job: it never runs (the job is not closed on it, so end it here).
                    let _ = windows::Win32::System::Threading::TerminateProcess(raw(&process), 1);
                    return Err(("AssignProcessToJobObject", e.code()));
                }
                if ResumeThread(raw(&thread)) == u32::MAX {
                    return Err(("ResumeThread", windows::core::Error::from_thread().code()));
                }
                drop(thread);
                Ok((process, job, in_w, out_r))
            })();
            DeleteProcThreadAttributeList(list);
            drop(buffer);
            // The parser's own ends close here: it holds its copies.
            drop((in_r, out_w, restricted, own));
            started
        })();
        free_container();
        made
    };
    r
}
