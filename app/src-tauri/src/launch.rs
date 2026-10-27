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
use std::process::{Child, Command};

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
    Command::new(program).spawn()
}

#[cfg(test)]
mod tests {
    use super::*;

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
