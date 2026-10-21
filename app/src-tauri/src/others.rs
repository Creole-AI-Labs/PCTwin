//! Other people in the move, on the old laptop (M06, M25): who lives here, and the administrator
//! helper started with the system's own prompt for the people the mover ticked (Security Design
//! part C). The helper reads their files read-only and gives the app totals and handles, never
//! names of files; the mover sees only how many of their things moved.
//!
//! Windows asks through UAC and Linux through polkit. A Mac starts the helper as a launch daemon
//! that, as built, cannot yet be told which people were approved, so there other people are not
//! moved in this version and the screens say so.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use pctwin_helper::{Client, ClientError, Refusal, Scanned};
use pctwin_record::LaptopId;
use pctwin_scan::{Census, PersonKind};
use serde::Serialize;

use crate::moving::MoveProblem;

/// How long the person has to answer the system's prompt and the helper to open its channel.
const HELPER_WAIT: Duration = Duration::from_secs(180);
/// How often the channel is tried while waiting.
const TRY_EVERY: Duration = Duration::from_millis(250);

/// One person with an account on this laptop, as M06 shows them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonHere {
    /// Their account ID: what the screens send back to tick them.
    pub id: String,
    /// The name their account shows.
    pub name: String,
    pub is_you: bool,
}

/// The people M06 lists from `census`: the mover first, then everyone else with an account and a
/// home folder. Accounts never used, work accounts and leftovers are not people to move.
pub fn people_in(census: &Census) -> Vec<PersonHere> {
    census
        .residents
        .iter()
        .filter(|r| matches!(r.kind, PersonKind::Me | PersonKind::Other))
        .filter_map(|r| {
            Some(PersonHere {
                id: r.account_id.clone()?,
                name: r.suggested_name.clone(),
                is_you: r.kind == PersonKind::Me,
            })
        })
        .collect()
}

/// Whether this laptop can move other people in this version.
pub const CAN_MOVE_OTHERS: bool = cfg!(any(windows, target_os = "linux"));

/// The people on this laptop (M06).
#[tauri::command]
pub async fn move_people_here() -> Result<PeopleHere, MoveProblem> {
    let census = tauri::async_runtime::spawn_blocking(pctwin_scan::census_this_laptop)
        .await
        .map_err(|_| MoveProblem::Failed)?;
    Ok(PeopleHere {
        people: people_in(&census),
        can_move_others: CAN_MOVE_OTHERS,
    })
}

/// What M06 shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeopleHere {
    pub people: Vec<PersonHere>,
    /// False on a Mac in this version: others are listed but cannot be ticked.
    pub can_move_others: bool,
}

/// The ticked people, kept only if each is another person with an account here.
pub fn checked_others(census: &Census, ticked: &[String]) -> Result<Vec<String>, MoveProblem> {
    let here: BTreeSet<String> = people_in(census)
        .into_iter()
        .filter(|p| !p.is_you)
        .map(|p| p.id)
        .collect();
    let mut seen = BTreeSet::new();
    for id in ticked {
        if !here.contains(id) || !seen.insert(id.clone()) {
            return Err(MoveProblem::Failed);
        }
    }
    Ok(ticked.to_vec())
}

/// The helper, connected, for one move.
pub type Helper = Arc<Mutex<Client<pctwin_helper::client::Channel>>>;

/// How asking for the other people went.
pub enum Asked {
    /// The administrator said yes: each approved person's scan, by account ID.
    Approved(Helper, Vec<(String, Scanned)>),
    /// The administrator said no at the system's prompt (or this system cannot ask): only the
    /// mover's things move.
    NotApproved,
}

fn hex(bytes: usize) -> Result<String, MoveProblem> {
    let token = pctwin_helper_os::random_token().map_err(|_| MoveProblem::Failed)?;
    Ok(token.chars().take(bytes * 2).collect())
}

/// Starts the helper for `others` with the system's prompt, connects, and scans each one.
pub fn ask_for(mover: &str, others: &[String], laptop: LaptopId) -> Result<Asked, MoveProblem> {
    if !CAN_MOVE_OTHERS || others.is_empty() {
        return Ok(Asked::NotApproved);
    }
    let token = pctwin_helper_os::random_token().map_err(|_| MoveProblem::Failed)?;
    let channel = format!("pctwin-helper-{}", hex(8)?);
    let dir = if cfg!(windows) {
        None
    } else {
        Some(std::path::PathBuf::from(format!(
            "/var/run/pctwin-{}",
            hex(8)?
        )))
    };
    let approve = others.join(",");
    let laptop = laptop.to_hex();
    let mut args = vec![
        "--session",
        token.as_str(),
        "--mover",
        mover,
        "--approve",
        approve.as_str(),
        "--channel",
        channel.as_str(),
        "--laptop",
        laptop.as_str(),
    ];
    let dir_text = dir.as_ref().map(|d| d.display().to_string());
    if let Some(d) = &dir_text {
        args.extend(["--dir", d.as_str()]);
    }
    let program = if cfg!(windows) {
        "pctwin-helper.exe"
    } else {
        "pctwin-helper"
    };
    let request = pctwin_helper_os::LaunchRequest::beside_current_exe(program, &args)
        .map_err(|_| MoveProblem::Failed)?;
    use pctwin_helper_os::Launcher;
    let mut process = match pctwin_helper_os::SystemLauncher.start(&request) {
        Ok(p) => p,
        Err(pctwin_helper_os::LaunchError::Cancelled) => return Ok(Asked::NotApproved),
        Err(pctwin_helper_os::LaunchError::Failed(_)) => return Err(MoveProblem::Failed),
    };
    // The prompt may still be open (Linux asks after starting): try the channel until the helper
    // answers, the prompt is refused, or the time is up.
    let until = Instant::now() + HELPER_WAIT;
    let client = loop {
        match pctwin_helper::connect(&channel, dir.as_deref(), &token) {
            Ok(c) => break c,
            Err(_) if Instant::now() < until => {
                if let Ok(Some(ended)) = process.try_wait() {
                    return match ended {
                        pctwin_helper_os::Ended::Refused
                        | pctwin_helper_os::Ended::NotAuthorized => Ok(Asked::NotApproved),
                        _ => Err(MoveProblem::Failed),
                    };
                }
                std::thread::sleep(TRY_EVERY);
            }
            Err(_) => return Err(MoveProblem::Failed),
        }
    };
    let mut client = client;
    let mut scanned = Vec::new();
    for person in others {
        match client.scan_person(person) {
            Ok(s) => scanned.push((person.clone(), s)),
            // The helper refuses anyone not on its approved list: never expected here.
            Err(ClientError::Refused(Refusal::NotApproved)) => {}
            Err(_) => return Err(MoveProblem::Failed),
        }
    }
    Ok(Asked::Approved(Arc::new(Mutex::new(client)), scanned))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pctwin_scan::Resident;

    fn census() -> Census {
        let r = |id: Option<&str>, kind, name: &str| Resident {
            account_id: id.map(str::to_string),
            kind,
            suggested_name: name.into(),
            home: None,
            found_by: Vec::new(),
        };
        Census {
            me: "1000".into(),
            profiles_root: None,
            residents: vec![
                r(Some("1000"), PersonKind::Me, "Ada"),
                r(Some("1001"), PersonKind::Other, "Bea"),
                r(Some("1002"), PersonKind::NeverUsed, "Guest"),
                r(None, PersonKind::Leftover, "old"),
                r(Some("1003"), PersonKind::Managed, "Work"),
            ],
            account_list_read: true,
            system: pctwin_scan::SystemKind::Linux,
            uid_range: (1000, 60000),
        }
    }

    #[test]
    fn m06_lists_the_mover_first_then_the_other_people_only() {
        let people = people_in(&census());
        let names: Vec<(&str, bool)> = people.iter().map(|p| (p.name.as_str(), p.is_you)).collect();
        assert_eq!(names, [("Ada", true), ("Bea", false)]);
    }

    #[test]
    fn only_other_people_here_can_be_ticked_and_each_once() {
        let c = census();
        assert_eq!(checked_others(&c, &["1001".into()]).unwrap(), ["1001"]);
        assert!(checked_others(&c, &[]).unwrap().is_empty());
        for bad in [
            vec!["1000".to_string()],
            vec!["1002".into()],
            vec!["9999".into()],
            vec!["1001".into(), "1001".into()],
        ] {
            assert!(checked_others(&c, &bad).is_err(), "{bad:?}");
        }
    }
}
