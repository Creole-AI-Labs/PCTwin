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
    /// One person's folder in it, named `key`, for the mover to fill.
    fn person_folder(&mut self, area: &Path, key: &str) -> Result<PathBuf, Refusal>;
    /// Hands `folder` over into the home of `account` (named `name`, when the helper made it).
    fn hand_over(
        &mut self,
        account: &Account,
        name: &str,
        folder: &Path,
    ) -> Result<Counted, Refusal>;
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

/// The real system: the holding area, people's homes, and the hand-over program.
pub struct System {
    pub mover: Account,
}

impl NewLaptop for System {
    fn area(&mut self) -> Result<PathBuf, Refusal> {
        pctwin_helper_os::holding::create_holding_area(&self.mover).map_err(|_| Refusal::NoHome)
    }

    fn person_folder(&mut self, area: &Path, key: &str) -> Result<PathBuf, Refusal> {
        pctwin_helper_os::holding::person_folder(area, key, &self.mover)
            .map_err(|_| Refusal::NoHome)
    }

    fn hand_over(
        &mut self,
        account: &Account,
        name: &str,
        folder: &Path,
    ) -> Result<Counted, Refusal> {
        let home =
            pctwin_helper_os::holding::home_folder(account, name).map_err(|_| Refusal::NoHome)?;
        let program = std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(Path::to_path_buf))
            .map(|d| d.join(format!("pctwin-handover{}", std::env::consts::EXE_SUFFIX)))
            .ok_or(Refusal::Unreadable)?;
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
        // Mac and Linux: the person's held folder is theirs now, and the hand-over runs as them,
        // writing with their rights only.
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            let Account::Uid(uid) = account else {
                return Err(Refusal::NoHome);
            };
            let (gid, _) =
                pctwin_helper_os::holding::passwd_entry(*uid).map_err(|_| Refusal::NoHome)?;
            pctwin_helper_os::holding::give_to(folder, *uid, gid)
                .map_err(|_| Refusal::Unreadable)?;
            command
                .uid(*uid)
                .gid(gid)
                .env_clear()
                .env("HOME", &home)
                .current_dir(&home);
        }
        let out = command.output().map_err(|_| Refusal::Unreadable)?;
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
