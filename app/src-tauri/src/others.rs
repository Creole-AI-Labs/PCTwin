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
use serde::{Deserialize, Serialize};

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

/// Starts the helper with the system's prompt for `approve` (old laptop: the other people's
/// account IDs; new laptop: the names of accounts to make and the accounts to hold for), and
/// connects to it. `None`: the administrator said no.
fn launch(
    mover: &str,
    approve: &[String],
    laptop: LaptopId,
) -> Result<Option<Client<pctwin_helper::client::Channel>>, MoveProblem> {
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
    let approve = approve.join(",");
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
        Err(pctwin_helper_os::LaunchError::Cancelled) => return Ok(None),
        Err(pctwin_helper_os::LaunchError::Failed(_)) => return Err(MoveProblem::Failed),
    };
    // The prompt may still be open (Linux asks after starting): try the channel until the helper
    // answers, the prompt is refused, or the time is up.
    let until = Instant::now() + HELPER_WAIT;
    loop {
        match pctwin_helper::connect(&channel, dir.as_deref(), &token) {
            Ok(c) => return Ok(Some(c)),
            Err(_) if Instant::now() < until => {
                if let Ok(Some(ended)) = process.try_wait() {
                    return match ended {
                        pctwin_helper_os::Ended::Refused
                        | pctwin_helper_os::Ended::NotAuthorized => Ok(None),
                        _ => Err(MoveProblem::Failed),
                    };
                }
                std::thread::sleep(TRY_EVERY);
            }
            Err(_) => return Err(MoveProblem::Failed),
        }
    }
}

/// Starts the helper for `others` with the system's prompt, connects, and scans each one.
pub fn ask_for(mover: &str, others: &[String], laptop: LaptopId) -> Result<Asked, MoveProblem> {
    if !CAN_MOVE_OTHERS || others.is_empty() {
        return Ok(Asked::NotApproved);
    }
    let Some(mut client) = launch(mover, others, laptop)? else {
        return Ok(Asked::NotApproved);
    };
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

/* ---------- The new laptop ---------- */

/// An account already on this laptop that another person's things may go to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AccountHere {
    pub id: String,
    pub name: String,
}

/// The accounts on this laptop other than the signed-in person's.
pub fn accounts_here() -> Vec<AccountHere> {
    pctwin_scan::list_people()
        .into_iter()
        .filter(|p| !p.is_me)
        .map(|p| AccountHere {
            id: p.account_id,
            name: p.suggested_name,
        })
        .collect()
}

/// Where one other person's things go, as the mover chose.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum PlaceChoice {
    /// Into an account already on this laptop.
    Existing {
        #[serde(rename = "accountId")]
        account_id: String,
    },
    /// Into a new account PCTwin makes, named after them.
    New,
    /// Not moved.
    LeaveOut,
}

/// One other person's choice, by their account ID on the old laptop.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersonChoice {
    pub id: String,
    pub to: PlaceChoice,
}

/// The words of the system window that shows a new account's temporary password, in the
/// person's language: `{name}`, `{account}` and `{password}` are filled in.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PasswordWords {
    pub title: String,
    pub body: String,
}

impl PasswordWords {
    /// Short plain text with each place to fill in exactly once.
    pub fn fine(&self) -> bool {
        let plain = |t: &str, max: usize| {
            !t.is_empty() && t.len() <= max && !t.chars().any(|c| c.is_control() && c != '\n')
        };
        plain(&self.title, 200)
            && plain(&self.body, 1000)
            && ["{name}", "{account}", "{password}"]
                .iter()
                .all(|k| self.body.matches(k).count() == 1)
    }
}

/// Checks the mover's choices: exactly one for each person `asked`, an existing account only
/// from `accounts`, and one account never given to two people.
pub fn checked_choices(
    asked: &BTreeSet<String>,
    accounts: &[AccountHere],
    choices: &[PersonChoice],
) -> Result<std::collections::BTreeMap<String, PlaceChoice>, MoveProblem> {
    let mut out = std::collections::BTreeMap::new();
    let mut used = BTreeSet::new();
    for c in choices {
        if !asked.contains(&c.id) {
            return Err(MoveProblem::Failed);
        }
        if let PlaceChoice::Existing { account_id } = &c.to
            && (!accounts.iter().any(|a| &a.id == account_id) || !used.insert(account_id.clone()))
        {
            return Err(MoveProblem::Failed);
        }
        if out.insert(c.id.clone(), c.to.clone()).is_some() {
            return Err(MoveProblem::Failed);
        }
    }
    if out.len() == asked.len() {
        Ok(out)
    } else {
        Err(MoveProblem::Failed)
    }
}

/// A sign-in name for a new account made from `name`: its letters and digits, lower case,
/// starting with a letter, at most 20, and none of `taken` (a number is added if needed).
pub fn login_name(name: &str, taken: &BTreeSet<String>) -> String {
    let mut base: String = name
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .skip_while(char::is_ascii_digit)
        .take(16)
        .collect();
    if base.is_empty() {
        base = "person".into();
    }
    if !taken.contains(&base) {
        return base;
    }
    (2..1000)
        .map(|n| format!("{base}{n}"))
        .find(|n| !taken.contains(n))
        .unwrap_or(base)
}

/// A name the systems accept to show for an account, from what the old laptop said.
pub fn full_name(name: &str) -> String {
    let shown: String = name
        .chars()
        .filter(|c| !c.is_control() && *c != ':' && *c != ',')
        .take(64)
        .collect();
    if shown.trim().is_empty() {
        "PCTwin".into()
    } else {
        shown
    }
}

/// Shows a new account's temporary password once, in the system's own window (never in the
/// screens, which are a web page), and forgets it.
pub fn show_password(words: &PasswordWords, name: &str, account: &str, password: &str) {
    let body = zeroize::Zeroizing::new(
        words
            .body
            .replace("{name}", name)
            .replace("{account}", account)
            .replace("{password}", password),
    );
    let _ = rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Info)
        .set_title(&words.title)
        .set_description(body.as_str())
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

/// One other person placed on this laptop: their account here, and the holding place their
/// things land in until the hand-over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placed {
    pub old_id: String,
    pub account: String,
}

/// The helper on the new laptop, for the people the mover placed.
pub struct NewHelper {
    client: Client<pctwin_helper::client::Channel>,
}

impl NewHelper {
    /// Starts it with the system's prompt, for the accounts to make (`names`) and the existing
    /// accounts to hold for (`accounts`). `None`: the administrator said no.
    pub fn start(
        me: &str,
        names: &[String],
        accounts: &[String],
    ) -> Result<Option<NewHelper>, MoveProblem> {
        if !CAN_MOVE_OTHERS {
            return Ok(None);
        }
        let approve: Vec<String> = names.iter().chain(accounts).cloned().collect();
        let laptop = LaptopId::from_hex(&"0".repeat(32)).map_err(|_| MoveProblem::Failed)?;
        Ok(launch(me, &approve, laptop)?.map(|client| NewHelper { client }))
    }

    /// Makes the account `name`, shown as `full`; its temporary password is shown once with
    /// `words`. Returns the system's ID for it.
    pub fn make_account(
        &mut self,
        name: &str,
        full: &str,
        words: &PasswordWords,
    ) -> Result<String, MoveProblem> {
        let made = self
            .client
            .create_account(name, full)
            .map_err(|_| MoveProblem::Failed)?;
        show_password(words, full, name, made.password.as_str());
        Ok(made.account)
    }

    /// The folder `account`'s things are held in until the hand-over.
    pub fn hold_for(&mut self, account: &str) -> Result<std::path::PathBuf, MoveProblem> {
        self.client
            .hold_for(account)
            .map_err(|_| MoveProblem::Failed)
    }

    /// Hands each placed person's things over at their next sign-in, then ends the helper.
    /// Returns those it could not set up.
    pub fn finish(mut self, placed: &[Placed]) -> Vec<String> {
        let failed = placed
            .iter()
            .filter(|p| self.client.hand_over_at_sign_in(&p.account).is_err())
            .map(|p| p.old_id.clone())
            .collect();
        let _ = self.client.bye();
        failed
    }
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
    fn a_new_accounts_sign_in_name_is_plain_and_free() {
        let taken: BTreeSet<String> = ["bea".to_string(), "bea2".to_string()].into();
        assert_eq!(login_name("Carl Jones", &taken), "carljones");
        assert_eq!(login_name("Béa", &taken), "ba");
        assert_eq!(login_name("Bea", &taken), "bea3");
        assert_eq!(login_name("42", &taken), "person");
        assert_eq!(login_name("李明", &taken), "person");
        for name in [
            "Carl Jones",
            "Bea",
            "42",
            "李明",
            "A very long name indeed for anyone",
        ] {
            assert!(
                pctwin_helper_os::is_account_name(&login_name(name, &taken)),
                "{name}"
            );
        }
        assert!(pctwin_helper_os::is_full_name(&full_name(
            "Bea, Smith: home"
        )));
        assert!(pctwin_helper_os::is_full_name(&full_name("\u{7}")));
    }

    #[test]
    fn choices_must_cover_each_person_once_with_accounts_from_here() {
        let asked: BTreeSet<String> = ["S-2".to_string(), "S-3".to_string()].into();
        let accounts = vec![AccountHere {
            id: "N-1".into(),
            name: "Bea".into(),
        }];
        let c = |id: &str, to| PersonChoice { id: id.into(), to };
        let existing = |id: &str| PlaceChoice::Existing {
            account_id: id.into(),
        };
        assert!(
            checked_choices(
                &asked,
                &accounts,
                &[c("S-2", existing("N-1")), c("S-3", PlaceChoice::New)]
            )
            .is_ok()
        );
        for bad in [
            vec![c("S-2", PlaceChoice::New)],
            vec![c("S-2", existing("N-9")), c("S-3", PlaceChoice::New)],
            vec![c("S-2", existing("N-1")), c("S-3", existing("N-1"))],
            vec![
                c("S-2", PlaceChoice::New),
                c("S-2", PlaceChoice::LeaveOut),
                c("S-3", PlaceChoice::New),
            ],
            vec![c("S-2", PlaceChoice::New), c("S-9", PlaceChoice::New)],
        ] {
            assert!(checked_choices(&asked, &accounts, &bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn the_password_window_words_hold_each_place_once() {
        let w = |body: &str| PasswordWords {
            title: "New account".into(),
            body: body.into(),
        };
        assert!(w("{name}: sign in as {account} with {password}").fine());
        assert!(!w("{name} {account}").fine());
        assert!(!w("{name} {account} {password} {password}").fine());
        assert!(!w("{name}\u{1b}[2J {account} {password}").fine());
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
