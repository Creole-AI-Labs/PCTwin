//! Room for open files. A Mac app starts allowed only 256 open files at once (Linux often 1024),
//! while a move holds its own files open and the old laptop's listener may hold up to
//! [`pctwin_link::MOST_HELD`] connections from strangers on the network: so at start the app asks
//! the system for more, up to what it allows, through the system's own call. Windows has no such
//! small limit.

/// What the app asks for: a Mac allows a process at most this many.
#[cfg(unix)]
pub const WANTED: u64 = 10_240;
/// What it asks for if the system refuses that.
#[cfg(any(unix, test))]
pub const AT_LEAST: u64 = 4_096;

/// Raises this process's limit on open files towards [`WANTED`] (at least [`AT_LEAST`] where the
/// system allows), never lowering it. Returns the limit now in force, when the system says.
#[cfg(unix)]
pub fn raise_open_files() -> Option<u64> {
    use rustix::process::{Resource, Rlimit, getrlimit, setrlimit};
    let now = getrlimit(Resource::Nofile);
    let have = now.current.unwrap_or(u64::MAX);
    let hard = now.maximum.unwrap_or(u64::MAX);
    for want in [WANTED.min(hard), AT_LEAST.min(hard)] {
        if want <= have {
            return Some(have);
        }
        let raised = Rlimit {
            current: Some(want),
            maximum: now.maximum,
        };
        if setrlimit(Resource::Nofile, raised).is_ok() {
            return Some(want);
        }
    }
    Some(have)
}

/// Windows: nothing to raise.
#[cfg(not(unix))]
pub fn raise_open_files() -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What the listener may hold for strangers is a small share of what is asked for, so the
    /// move's own files always have room.
    #[test]
    fn the_listener_s_worst_case_is_a_small_share_of_the_room_asked_for() {
        assert!(pctwin_link::MOST_HELD * 8 <= AT_LEAST as usize);
    }

    /// After the raise, the limit is at least what was asked for, as far as the system allows.
    #[cfg(unix)]
    #[test]
    fn the_limit_is_raised_as_far_as_the_system_allows() {
        let got = raise_open_files().unwrap();
        let hard = rustix::process::getrlimit(rustix::process::Resource::Nofile)
            .maximum
            .unwrap_or(u64::MAX);
        assert!(got >= AT_LEAST.min(hard), "{got} of {hard}");
        let now = rustix::process::getrlimit(rustix::process::Resource::Nofile).current;
        assert_eq!(now.unwrap_or(u64::MAX), got);
    }
}
