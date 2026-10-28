//! The helper on the new laptop: the holding area for other people's files (`HoldFor`) and
//! handing each person's files over into their own folders (`HandOverNow`). The system's side is
//! in [`System`]; the session decides who may.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pctwin_helper_os::Account;

use crate::protocol::Refusal;

/// How a hand-over came out, file by file counted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counted {
    pub landed: u64,
    pub kept: u64,
    pub failed: u64,
}

/// What the new laptop's helper asks of the system.
pub trait NewLaptop {
    /// This move's holding area, made once.
    fn area(&mut self) -> Result<PathBuf, Refusal>;
    /// One person's folder in it, named `key`, for the mover to fill (and, on Windows, for
    /// `person` to read at their sign-in).
    fn person_folder(
        &mut self,
        area: &Path,
        key: &str,
        person: &Account,
    ) -> Result<PathBuf, Refusal>;
    /// Makes the hand-over of `folder` run at `person`'s next sign-in, as them (`name`: the
    /// account's name, when the helper made it; `key`: the entry's name).
    fn hand_over_at_sign_in(
        &mut self,
        person: &Account,
        name: &str,
        folder: &Path,
        key: &str,
    ) -> Result<(), Refusal>;
    /// Hands `folder` over into the home of `account` (named `name`, when the helper made it).
    fn hand_over(
        &mut self,
        account: &Account,
        name: &str,
        folder: &Path,
    ) -> Result<Counted, Refusal>;
}

/// The refusal `why`, with what lay behind it written to the helper's own error output, which
/// only an administrator reads (the app sees only the refusal; no file names are in it).
fn logged(why: Refusal, what: &str, cause: &dyn std::fmt::Display) -> Refusal {
    eprintln!("pctwin-helper: {what}: {cause}");
    why
}

/// The places a person's files land in inside their home, by the move's place labels: the
/// system's own folder names for a new account, and one folder for everything else.
pub fn places() -> BTreeMap<String, String> {
    let videos = if cfg!(target_os = "macos") {
        "Movies"
    } else {
        "Videos"
    };
    [
        ("desktop", "Desktop"),
        ("documents", "Documents"),
        ("downloads", "Downloads"),
        ("pictures", "Pictures"),
        ("music", "Music"),
        ("videos", videos),
        ("home", "From your old laptop"),
        ("public", "From your old laptop"),
        ("other", "From your old laptop"),
    ]
    .into_iter()
    .map(|(l, f)| (l.to_string(), f.to_string()))
    .collect()
}

/// `pctwin-handover`, beside this program (as the system resolves it, no link left), and only if
/// nobody but the administrators can change it or the folders it is in: it runs with an
/// administrator's rights (Windows, hand-over now) or as each person who signs in (the entries
/// made for the hand-over at sign-in), so a program the mover could replace would run as them.
fn handover_program() -> Result<PathBuf, Refusal> {
    let program = std::env::current_exe()
        .and_then(std::fs::canonicalize)
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf))
        .map(|d| d.join(format!("pctwin-handover{}", std::env::consts::EXE_SUFFIX)))
        .ok_or(Refusal::Unreadable)?;
    pctwin_helper_os::only_administrators_can_change(&program)
        .map_err(|e| logged(Refusal::Unreadable, "the hand-over program", &e))?;
    Ok(program)
}

/// The real system: the holding area, people's homes, and the hand-over program.
pub struct System {
    pub mover: Account,
}

impl NewLaptop for System {
    fn area(&mut self) -> Result<PathBuf, Refusal> {
        pctwin_helper_os::holding::create_holding_area(&self.mover)
            .map_err(|e| logged(Refusal::NoHome, "the holding area", &e))
    }

    fn person_folder(
        &mut self,
        area: &Path,
        key: &str,
        person: &Account,
    ) -> Result<PathBuf, Refusal> {
        pctwin_helper_os::holding::person_folder(area, key, &self.mover, Some(person))
            .map_err(|e| logged(Refusal::NoHome, "a person's held folder", &e))
    }

    fn hand_over_at_sign_in(
        &mut self,
        person: &Account,
        name: &str,
        folder: &Path,
        key: &str,
    ) -> Result<(), Refusal> {
        pctwin_helper_os::signin::schedule(
            person,
            name,
            &self.mover,
            folder,
            &handover_program()?,
            key,
        )
        .map_err(|e| logged(Refusal::Unreadable, "the hand-over at sign-in", &e))
    }

    fn hand_over(
        &mut self,
        account: &Account,
        name: &str,
        folder: &Path,
    ) -> Result<Counted, Refusal> {
        let home = pctwin_helper_os::holding::home_folder(account, name)
            .map_err(|e| logged(Refusal::NoHome, "the person's home", &e))?;
        let program = handover_program()?;
        let account_id = match account {
            Account::Sid(sid) => sid.as_str().to_string(),
            Account::Uid(uid) => uid.to_string(),
        };
        let mut command = pctwin_types::process::quiet(&program);
        command
            .arg("--holding")
            .arg(folder)
            .arg("--home")
            .arg(&home)
            .arg("--account")
            .arg(&account_id)
            .arg("--journal")
            .arg(folder.join("handover.journal"));
        for (label, into) in places() {
            command.arg("--into").arg(format!("{label}={into}"));
        }
        // Windows: the hand-over runs with the administrator's rights, so first nobody else may
        // change the held folder (the mover and the person lose their access to it). The lock
        // alone cannot be the whole guard: it walks the folder once, a handle opened before it
        // keeps its access, and a folder made while it runs keeps the mover's. So the hand-over
        // itself trusts nothing in the folder: it walks it and opens every file through folder
        // handles, never through a link, refuses a file with a second name, and reads a file
        // only if it is still the very one it found (`pctwin_move::handover`). What the mover can
        // still put there is a plain file of its own, which was the mover's to hand over anyway.
        #[cfg(windows)]
        {
            // Started with an administrator's rights: nothing of the starter's environment.
            command.env_clear().envs(
                pctwin_helper_os::minimal_env()
                    .map_err(|e| logged(Refusal::Unreadable, "the hand-over's environment", &e))?,
            );
            let (Account::Sid(mover), Account::Sid(person)) = (&self.mover, account) else {
                return Err(Refusal::NoHome);
            };
            pctwin_helper_os::signin::lock_to_administrators(folder, &[mover, person])
                .map_err(|e| logged(Refusal::Unreadable, "locking the held folder", &e))?;
        }
        // Mac and Linux: the person's held folder is theirs now, and the hand-over runs as them,
        // writing with their rights only.
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            let Account::Uid(uid) = account else {
                return Err(Refusal::NoHome);
            };
            let (gid, _) = pctwin_helper_os::holding::passwd_entry(*uid)
                .map_err(|e| logged(Refusal::NoHome, "the person's account record", &e))?;
            pctwin_helper_os::holding::give_to(folder, *uid, gid).map_err(|e| {
                logged(
                    Refusal::Unreadable,
                    "giving the held folder to the person",
                    &e,
                )
            })?;
            command
                .uid(*uid)
                .gid(gid)
                .env_clear()
                .env("HOME", &home)
                .current_dir(&home);
        }
        let out = command
            .output()
            .map_err(|e| logged(Refusal::Unreadable, "starting the hand-over", &e))?;
        let counted = tally(&out.stdout, out.status.success()).map_err(|why| {
            logged(
                why,
                "the hand-over",
                &String::from_utf8_lossy(&out.stderr).trim(),
            )
        })?;
        // Emptied once everything is in the person's folders; else kept for another try.
        if out.status.success() && counted.kept == 0 && counted.failed == 0 {
            let _ = std::fs::remove_dir_all(folder);
        }
        Ok(counted)
    }
}

/// What a hand-over's output (`stdout`, one line per file) says, given whether it ended well
/// (`ended_well`). A hand-over that failed without naming a file it did not hand over (it could
/// not start, open its journal or read the holding area, or it stopped part way) is a failure,
/// never "nothing to do": what it did not say is not known to have landed.
fn tally(stdout: &[u8], ended_well: bool) -> Result<Counted, Refusal> {
    let mut counted = Counted::default();
    for line in String::from_utf8_lossy(stdout).lines() {
        match line.split('\t').next() {
            Some("landed") => counted.landed += 1,
            Some("kept") => counted.kept += 1,
            _ => counted.failed += 1,
        }
    }
    if !ended_well && counted.kept == 0 && counted.failed == 0 {
        return Err(Refusal::Unreadable);
    }
    Ok(counted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hand_over_that_failed_is_a_failure() {
        // Crashed, or its journal would not open: nothing said, and it ended badly.
        assert_eq!(tally(b"", false), Err(Refusal::Unreadable));
        // Some landed, then it stopped without saying what became of the rest.
        assert_eq!(
            tally(b"landed\ta\nlanded\tb\n", false),
            Err(Refusal::Unreadable)
        );
        // Ended badly and said why, file by file: those counts.
        assert_eq!(
            tally(b"landed\ta\nkept\tb\nnot-plain\tc\n", false),
            Ok(Counted {
                landed: 1,
                kept: 1,
                failed: 1
            })
        );
        // Ended well: nothing held, or everything landed.
        assert_eq!(tally(b"", true), Ok(Counted::default()));
        assert_eq!(
            tally(b"landed\ta\n", true),
            Ok(Counted {
                landed: 1,
                kept: 0,
                failed: 0
            })
        );
    }
}
