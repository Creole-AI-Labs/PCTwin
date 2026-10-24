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

/// `pctwin-handover`, beside this program.
fn handover_program() -> Result<PathBuf, Refusal> {
    std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf))
        .map(|d| d.join(format!("pctwin-handover{}", std::env::consts::EXE_SUFFIX)))
        .ok_or(Refusal::Unreadable)
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
        let mut command = std::process::Command::new(&program);
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
        // change the held folder (the mover and the person lose their access to it), and nothing
        // in it can be swapped for a link while it is read.
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
        let mut counted = Counted::default();
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            match line.split('\t').next() {
                Some("landed") => counted.landed += 1,
                Some("kept") => counted.kept += 1,
                _ => counted.failed += 1,
            }
        }
        // Emptied once everything is in the person's folders; else kept for another try.
        if out.status.success() && counted.kept == 0 && counted.failed == 0 {
            let _ = std::fs::remove_dir_all(folder);
        }
        Ok(counted)
    }
}
