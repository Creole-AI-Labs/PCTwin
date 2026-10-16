//! Asks Linux whether any other program has a file open, before PCTwin removes it.
//!
//! A write lease (`fcntl(F_SETLEASE, F_WRLCK)`) is granted on a handle only when no other open of
//! the same file exists anywhere on the system: any program, any user, a memory mapping, or a
//! second handle in PCTwin itself. While PCTwin holds the lease, another program that starts to
//! open the file waits, and PCTwin is told with the signal SIGIO, whose default would end the
//! process; [`write_lease`] makes sure that signal is caught first (through `signal-hook`, a
//! safe API). [`still_held`] then says whether anyone started to open it meanwhile.
//!
//! On a Mac it also marks a file PCTwin is filling in under its real name as hidden, through the
//! file's own handle ([`set_hidden`], `fchflags`; security design notes, "Drives with no safe
//! rename"), so it is never seen as an ordinary file before it is complete.
//!
//! This is the only crate in PCTwin that calls the system unsafely, in one function
//! (`sys_int`) with an open descriptor and whole numbers only (no pointers): `fcntl` for leases on
//! Linux, `fchflags` on a Mac. Leases proven on ext4, XFS and btrfs, 9 October 2026 (a local
//! experiment): granted alone; refused for another reader, writer, mapping, or PCTwin's own
//! second handle; a new open waits and SIGIO is caught; PCTwin's own rename and removal keep it.
//! On other systems nothing can be asked, and every answer is [`Lease::CannotCheck`]. On
//! Windows the crate is empty (a held file there blocks other programs outright).
#![cfg(unix)]

use std::io;
use std::os::fd::AsFd;

/// What asking for the lease found.
#[derive(Debug)]
pub enum Lease {
    /// No other program has the file open; PCTwin holds the lease until [`release`].
    Held,
    /// Another program (or another handle) has the file open.
    InUse,
    /// The system cannot say (leases switched off, a drive or system without them).
    CannotCheck(io::Error),
}

/// Asks for a write lease on `file`, opened read-only by PCTwin.
pub fn write_lease(file: &impl AsFd) -> Lease {
    imp::write_lease(file)
}

/// Whether the lease is still held with no other program waiting to open the file. False once
/// anyone has started to open it (the system then reports the lease as being given up).
pub fn still_held(file: &impl AsFd) -> bool {
    imp::still_held(file)
}

/// Gives the lease up (also given up when the handle is closed).
pub fn release(file: &impl AsFd) -> io::Result<()> {
    imp::release(file)
}

/// The system calls PCTwin makes unsafely: each on an open descriptor, with whole numbers only.
#[cfg(any(target_os = "linux", target_os = "macos"))]
enum Call {
    /// `fcntl(fd, command, arg)` with an integer argument (F_SETLEASE, F_GETLEASE).
    #[cfg(target_os = "linux")]
    Fcntl {
        command: libc::c_int,
        arg: libc::c_int,
    },
    /// `fchflags(fd, flags)`: sets the file's flags (UF_HIDDEN and the rest) through its handle.
    #[cfg(target_os = "macos")]
    Fchflags { flags: libc::c_uint },
}

/// The one unsafe call site.
/// SAFETY: `fd` is valid for the duration (borrowed through `AsFd`); every [`Call`] passes only
/// whole numbers (F_SETLEASE and F_GETLEASE take an integer argument, ignored for F_GETLEASE;
/// fchflags takes the flags word) and touches no memory.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn sys_int(file: &impl AsFd, call: Call) -> io::Result<i32> {
    use std::os::fd::AsRawFd;
    let fd = file.as_fd().as_raw_fd();
    #[expect(
        unsafe_code,
        reason = "no safe binding for F_SETLEASE / F_GETLEASE or fchflags exists"
    )]
    let r = unsafe {
        match call {
            #[cfg(target_os = "linux")]
            Call::Fcntl { command, arg } => libc::fcntl(fd, command, arg),
            #[cfg(target_os = "macos")]
            Call::Fchflags { flags } => libc::fchflags(fd, flags),
        }
    };
    if r == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(r)
    }
}

/// macOS: marks the open file `file` hidden, or clears the mark, through its own handle (never by
/// name), keeping every other flag as it is; then reads the flags back through the same handle
/// and fails if the mark is not as asked.
#[cfg(target_os = "macos")]
pub fn set_hidden(file: &impl AsFd, hidden: bool) -> io::Result<()> {
    const UF_HIDDEN: u32 = 0x0000_8000;
    let now = rustix::fs::fstat(file)?.st_flags;
    let flags = if hidden {
        now | UF_HIDDEN
    } else {
        now & !UF_HIDDEN
    };
    sys_int(file, Call::Fchflags { flags })?;
    let back = rustix::fs::fstat(file)?.st_flags & UF_HIDDEN != 0;
    if back == hidden {
        Ok(())
    } else {
        Err(io::Error::other("the drive did not keep the hidden mark"))
    }
}

/// macOS: whether the open file `file` is marked hidden.
#[cfg(target_os = "macos")]
pub fn is_hidden(file: &impl AsFd) -> io::Result<bool> {
    Ok(rustix::fs::fstat(file)?.st_flags & 0x0000_8000 != 0)
}

#[cfg(target_os = "linux")]
mod imp {
    use super::Lease;
    use std::io;
    use std::os::fd::AsFd;
    use std::sync::Arc;
    use std::sync::OnceLock;
    use std::sync::atomic::AtomicBool;

    /// SIGIO is caught (and only noted), once per process, before any lease is asked for.
    fn catch_sigio() -> io::Result<()> {
        static CAUGHT: OnceLock<Result<(), String>> = OnceLock::new();
        CAUGHT
            .get_or_init(|| {
                signal_hook::flag::register(
                    signal_hook::consts::SIGIO,
                    Arc::new(AtomicBool::new(false)),
                )
                .map(drop)
                .map_err(|e| e.to_string())
            })
            .clone()
            .map_err(io::Error::other)
    }

    fn lease_fcntl(file: &impl AsFd, command: libc::c_int, arg: libc::c_int) -> io::Result<i32> {
        super::sys_int(file, super::Call::Fcntl { command, arg })
    }

    pub(super) fn write_lease(file: &impl AsFd) -> Lease {
        if let Err(e) = catch_sigio() {
            return Lease::CannotCheck(e);
        }
        match lease_fcntl(file, libc::F_SETLEASE, libc::F_WRLCK) {
            Ok(_) => Lease::Held,
            Err(e) if e.raw_os_error() == Some(libc::EAGAIN) => Lease::InUse,
            Err(e) => Lease::CannotCheck(e),
        }
    }

    pub(super) fn still_held(file: &impl AsFd) -> bool {
        matches!(lease_fcntl(file, libc::F_GETLEASE, 0), Ok(l) if l == libc::F_WRLCK)
    }

    pub(super) fn release(file: &impl AsFd) -> io::Result<()> {
        lease_fcntl(file, libc::F_SETLEASE, libc::F_UNLCK).map(drop)
    }
}

#[cfg(not(target_os = "linux"))]
mod imp {
    use super::Lease;
    use std::io;
    use std::os::fd::AsFd;

    pub(super) fn write_lease(_file: &impl AsFd) -> Lease {
        Lease::CannotCheck(io::Error::from(io::ErrorKind::Unsupported))
    }

    pub(super) fn still_held(_file: &impl AsFd) -> bool {
        false
    }

    pub(super) fn release(_file: &impl AsFd) -> io::Result<()> {
        Ok(())
    }
}
