//! Starting an app PCTwin installed, always as the person (acceptance case T22-020: after an
//! install done with an administrator's rights, the app is started in the person's own context,
//! never elevated; the join of 2.2 and 2.3, (d), 27 October 2026).
//!
//! The 2.7 launch check (does the installed app open?) starts apps only through [`launch`], and
//! [`launch`] takes an [`AsPerson`]: proof, checked when it was made, that this program runs as
//! the person without an administrator's full rights. Nothing else can make one, so:
//!
//! - the administrator helper never starts an app: it does not have this module (it lives in the
//!   app), and its broker has exactly three actions, none of which starts a program the person
//!   uses;
//! - an app started as administrator (the person chose "Run as administrator") gets no
//!   [`AsPerson`] ([`NotAsPerson::Elevated`]), so it starts nothing and says why, instead of
//!   starting the new app with full rights.
//!
//! The walls, proven by the compiler: an [`AsPerson`] cannot be written out by hand,
//!
//! ```compile_fail,E0451
//! let _p = pctwin_app::launch::AsPerson { _checked: () };
//! ```
//!
//! and nothing is started without one:
//!
//! ```compile_fail,E0061
//! let _ = pctwin_app::launch::launch(std::path::Path::new("app.exe"));
//! ```

use std::path::Path;
use std::process::Child;

/// Proof that this program runs as the person, without an administrator's full rights. Made only
/// by [`AsPerson::here`].
#[derive(Debug)]
pub struct AsPerson {
    _checked: (),
}

/// Why this program may not start the person's apps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotAsPerson {
    /// It runs with an administrator's full rights.
    Elevated,
    /// The system could not say: in doubt, nothing is started.
    CannotTell,
}

impl NotAsPerson {
    /// What to tell the person, in plain words.
    pub fn why(self) -> &'static str {
        match self {
            NotAsPerson::Elevated => {
                "PCTwin is running as administrator, so it did not open your new app; open it yourself, or start PCTwin normally"
            }
            NotAsPerson::CannotTell => {
                "PCTwin could not check it would open your new app as you, so it did not open it; open it yourself"
            }
        }
    }
}

impl AsPerson {
    /// Proof for this program, if it runs as the person.
    pub fn here() -> Result<Self, NotAsPerson> {
        Self::from_check(pctwin_helper_os::this_process_elevated().map_err(|_| ()))
    }

    /// The decision, from the system's answer to "is this program elevated?".
    fn from_check(elevated: Result<bool, ()>) -> Result<Self, NotAsPerson> {
        match elevated {
            Ok(false) => Ok(Self { _checked: () }),
            Ok(true) => Err(NotAsPerson::Elevated),
            Err(()) => Err(NotAsPerson::CannotTell),
        }
    }
}

/// Starts `program`, an app PCTwin installed, as the person: the only way PCTwin starts one. It
/// inherits this program's own rights, which `as_person` proves are the person's.
pub fn launch(as_person: &AsPerson, program: &Path) -> std::io::Result<Child> {
    let _ = as_person;
    pctwin_types::process::quiet(program).spawn()
}

/// The 2.1 launch check (step 5): starts `program` as the person and says whether it opened. It
/// opened when it is still running after `wait`, or ended by itself without an error (a launcher
/// that hands over to its own window). It is left open: it is the person's app, and they see
/// it. Started only when the person asks for the check; never with an administrator's rights.
pub fn launch_check(
    as_person: Result<AsPerson, NotAsPerson>,
    program: &Path,
    wait: std::time::Duration,
) -> pctwin_apps::after::Opened {
    use pctwin_apps::after::Opened;
    let Ok(as_person) = as_person else {
        return Opened::NotChecked;
    };
    let Ok(mut child) = launch(&as_person, program) else {
        return Opened::No;
    };
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return if status.success() {
                    Opened::Yes
                } else {
                    Opened::No
                };
            }
            Ok(None) if start.elapsed() >= wait => return Opened::Yes,
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(100)),
            Err(_) => return Opened::NotChecked,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Not as the person: nothing is started and nothing is claimed.
    #[test]
    fn the_launch_check_starts_nothing_without_the_proof() {
        use pctwin_apps::after::Opened;
        let none = Path::new("this-program-is-never-started.exe");
        assert_eq!(
            launch_check(Err(NotAsPerson::Elevated), none, std::time::Duration::ZERO),
            Opened::NotChecked
        );
        // A program that is not there did not open.
        if let Ok(p) = AsPerson::here() {
            assert_eq!(
                launch_check(Ok(p), none, std::time::Duration::ZERO),
                Opened::No
            );
        }
    }

    #[test]
    fn only_a_program_running_as_the_person_may_start_apps() {
        assert!(AsPerson::from_check(Ok(false)).is_ok());
        assert_eq!(
            AsPerson::from_check(Ok(true)).unwrap_err(),
            NotAsPerson::Elevated
        );
        assert_eq!(
            AsPerson::from_check(Err(())).unwrap_err(),
            NotAsPerson::CannotTell
        );
        for e in [NotAsPerson::Elevated, NotAsPerson::CannotTell] {
            assert!(e.why().contains("open it yourself"));
        }
    }

    /// The real check on this machine agrees with the system: a test run as the person gets the
    /// proof; one run as administrator (a CI runner) does not.
    #[test]
    fn the_real_check_agrees_with_the_system() {
        let elevated = pctwin_helper_os::this_process_elevated().unwrap();
        assert_eq!(AsPerson::here().is_err(), elevated);
    }
}
