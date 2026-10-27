//! Handover and clean-up gates (Task List 2.7 "Handing it on", "Source cleanup", "Safe goodbye";
//! Security Design S14 "Retirement ceremony"; plan 2.7 sections 7 and 8; acceptance T27-015 to
//! T27-024, T27-026).
//!
//! Three separate requests, none following from another, and none run here:
//!
//! - [`Action::RemoveTemporaryFiles`]: PCTwin's own temporary files and downloads, only those
//!   PCTwin recorded making; anything that merely looks temporary, and anything that is the
//!   person's selected work, is kept;
//! - [`Action::RemoveMoversAccount`]: the mover's own account on the old laptop, with the
//!   system's own tools; every other account is left as it is;
//! - [`Action::EraseOldLaptop`]: erasing the old laptop's drive. Only a method qualified for that
//!   exact drive could be asked for, and none is qualified yet: the request ends "not supported"
//!   with the system's own route in plain words. The erase itself is a later fixed broker action
//!   ("not built yet").
//!
//! Every request passes the same gate ([`request`]), checked again at the moment it is made:
//!
//! 1. the exact device: the old laptop and its drive's identity as recorded, never a drive that
//!    merely has the same letter or label (T27-019);
//! 2. undo closed for good, on disk, through the journal's own close: its [`ClosedToken`] for
//!    this very journal, and the journal's gate read back as closed (T27-018);
//! 3. the before-wipe list built from the old laptop as it is now, with every exception resolved
//!    or accepted (T27-012, T27-014);
//! 4. no other program can write to what would be removed (T27-020);
//! 5. a fresh, separate confirmation from the person of exactly the preview shown, for this
//!    action, used once (T27-017). A finished move is never that confirmation.
//!
//! Results are recorded as what was observed ([`ActionResult`]): a part-way, interrupted or
//! cancelled action says what may already be gone, and nothing here is a certificate that a drive
//! can never be read (T27-021 to T27-024, T27-026).

use std::collections::BTreeSet;

use pctwin_journal::{ClosedToken, Journal, UndoGate};
use pctwin_record::{ItemId, LaptopId, Record};
use serde::{Deserialize, Serialize};

use crate::goodbye::Checklist;

/// How long a confirmation stays fresh.
pub const FRESH: u64 = 10 * 60;

/// Deleting files on an SSD is not a secure wipe (Task List 2.7).
pub const SSD_NOTE: &str = "Deleting files on an SSD is not a secure wipe: the drive can keep \
    copies that PCTwin cannot reach. To hand the laptop on, use its own reset with drive \
    cleaning, or the drive maker's secure erase.";

/// One of the three separate requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Action {
    RemoveTemporaryFiles,
    RemoveMoversAccount,
    EraseOldLaptop,
}

/// The old laptop and its drive, exactly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceIdentity {
    pub laptop: LaptopId,
    /// The drive's serial number as the system reports it.
    pub drive_serial: String,
    pub drive_model: String,
    pub drive_bytes: u64,
}

/// Whether other programs are kept from writing while the action runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteExclusion {
    Held,
    CannotExclude,
}

/// An account on the old laptop now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    /// The system's own account ID.
    pub id: String,
    /// Whether it holds files of its own.
    pub has_own_files: bool,
}

/// How PCTwin knows a temporary file is its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Ownership {
    /// PCTwin recorded making it.
    RecordedByPctwin,
    /// It only looks like PCTwin's (by its name or folder).
    LooksTemporary,
}

/// One temporary file or folder found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TempEntry {
    pub name: String,
    pub ownership: Ownership,
    /// The person's selected work it is, if it is (a model they chose to keep, say).
    pub item: Option<ItemId>,
}

/// A method of erasing qualified for one exact drive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualifiedMethod {
    pub drive: DeviceIdentity,
    pub method: String,
}

/// What PCTwin knows now, read again for every request.
pub struct Facts<'a> {
    pub checklist: &'a Checklist,
    /// A fresh scan of the old laptop made now (`None`: it cannot be reached).
    pub old_laptop_now: Option<&'a Record>,
    pub journal: &'a Journal,
    pub undo_closed: Option<&'a ClosedToken>,
    /// The device as recorded when the laptops were paired.
    pub expected: &'a DeviceIdentity,
    /// The device as the system reports it now.
    pub device_now: &'a DeviceIdentity,
    pub write_exclusion: WriteExclusion,
    pub accounts: &'a [Account],
    /// The mover's account ID on the old laptop.
    pub mover: &'a str,
    pub temp: &'a [TempEntry],
    pub qualified: &'a [QualifiedMethod],
    pub now: u64,
}

/// Why a request was refused. Nothing was done.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "kebab-case", tag = "refused")]
pub enum Refusal {
    #[error("this is not the same laptop or drive")]
    WrongDevice,
    #[error("undo is not closed for good")]
    UndoNotClosed,
    #[error("the old laptop cannot be reached")]
    OldLaptopNotReachable,
    #[error("the old laptop changed since the list was made")]
    ChecklistStale,
    #[error("some things on the list are not resolved or accepted")]
    ExceptionsOpen { open: usize },
    #[error("other programs could write while this runs")]
    NoWriteExclusion,
    #[error("nothing here is PCTwin's to remove")]
    NothingToRemove,
    #[error("the mover's account is not on this laptop")]
    NoSuchAccount,
    #[error("other people keep their own files on this laptop")]
    OtherPeopleUseIt,
    #[error("no erase method is qualified for this drive")]
    NotQualified,
    #[error("there is no fresh confirmation for this")]
    NoFreshConfirmation,
    #[error("the confirmation is for something else")]
    WrongConfirmation,
    #[error("something changed since the person confirmed")]
    ChangedSinceConfirmed,
    #[error("the confirmation is too old")]
    Expired,
    #[error("that confirmation was already used")]
    AlreadyUsed,
}

/// What the person is shown before confirming: the exact, irreversible scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preview {
    pub action: Action,
    pub device: DeviceIdentity,
    pub generation: u64,
    /// What would be removed, by name.
    pub removes: Vec<String>,
    /// What is kept, by name (another person's account, a file that only looks temporary).
    pub keeps: Vec<String>,
    pub words: String,
}

impl Preview {
    pub fn digest(&self) -> [u8; 32] {
        let mut h = blake3::Hasher::new();
        h.update(b"pctwin/v1/handover-preview\0");
        h.update(&serde_json::to_vec(self).expect("serialises"));
        *h.finalize().as_bytes()
    }
}

/// The person's fresh, separate confirmation of exactly one preview. Only [`confirm`] makes one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Confirmation {
    action: Action,
    preview: [u8; 32],
    made_at: u64,
    nonce: [u8; 16],
}

/// The person confirms `preview`, as shown, at `made_at` (`nonce`: new random bytes for each).
pub fn confirm(preview: &Preview, made_at: u64, nonce: [u8; 16]) -> Confirmation {
    Confirmation {
        action: preview.action,
        preview: preview.digest(),
        made_at,
        nonce,
    }
}

/// Confirmations already used.
#[derive(Debug, Default)]
pub struct Used(BTreeSet<[u8; 16]>);

/// The gate's request, made only when every check passed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub action: Action,
    pub device: DeviceIdentity,
    pub generation: u64,
    pub removes: Vec<String>,
    pub confirmed_at: u64,
}

/// What asking the system to do a request gives now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeOutcome {
    /// The fixed broker action that would do it is not built yet; nothing was done.
    NotBuiltYet,
}

impl Request {
    /// Hands the request to the system. No destructive action exists yet: nothing is done.
    pub fn run(self) -> NativeOutcome {
        NativeOutcome::NotBuiltYet
    }
}

fn undo_closed(f: &Facts<'_>) -> bool {
    f.undo_closed.is_some_and(|t| t.is_for(f.journal))
        && matches!(f.journal.undo_gate(), Ok(UndoGate::Closed))
}

fn checks(action: Action, f: &Facts<'_>) -> Result<Preview, Refusal> {
    if f.expected.drive_serial.trim().is_empty()
        || f.device_now != f.expected
        || f.checklist.move_id.source != f.expected.laptop
    {
        return Err(Refusal::WrongDevice);
    }
    if !undo_closed(f) {
        return Err(Refusal::UndoNotClosed);
    }
    let Some(scan) = f.old_laptop_now else {
        return Err(Refusal::OldLaptopNotReachable);
    };
    if !f.checklist.is_current(scan) {
        return Err(Refusal::ChecklistStale);
    }
    let open = f.checklist.open().count();
    if open > 0 {
        return Err(Refusal::ExceptionsOpen { open });
    }
    if f.write_exclusion != WriteExclusion::Held {
        return Err(Refusal::NoWriteExclusion);
    }
    let (removes, keeps, words) = match action {
        Action::RemoveTemporaryFiles => {
            let (mine, not): (Vec<&TempEntry>, Vec<&TempEntry>) = f
                .temp
                .iter()
                .partition(|t| t.ownership == Ownership::RecordedByPctwin && t.item.is_none());
            if mine.is_empty() {
                return Err(Refusal::NothingToRemove);
            }
            (
                mine.iter().map(|t| t.name.clone()).collect(),
                not.iter().map(|t| t.name.clone()).collect(),
                "PCTwin will remove only the temporary files it made. Files that only look \
                 temporary, and anything you chose to keep, stay."
                    .to_string(),
            )
        }
        Action::RemoveMoversAccount => {
            if !f.accounts.iter().any(|a| a.id == f.mover) {
                return Err(Refusal::NoSuchAccount);
            }
            (
                vec![f.mover.to_string()],
                f.accounts
                    .iter()
                    .filter(|a| a.id != f.mover)
                    .map(|a| a.id.clone())
                    .collect(),
                format!(
                    "Your account and its files will be removed from the old laptop with the \
                     system's own tools. Everyone else's account stays as it is. Removing your \
                     account does not cancel subscriptions or sign you out elsewhere. {SSD_NOTE}"
                ),
            )
        }
        Action::EraseOldLaptop => {
            if f.accounts
                .iter()
                .any(|a| a.id != f.mover && a.has_own_files)
            {
                return Err(Refusal::OtherPeopleUseIt);
            }
            let Some(m) = f.qualified.iter().find(|m| &m.drive == f.device_now) else {
                return Err(Refusal::NotQualified);
            };
            (
                vec![format!("{} ({})", f.device_now.drive_model, m.method)],
                vec![],
                format!(
                    "Everything on this drive will be erased with {}. This cannot be undone. \
                     {SSD_NOTE}",
                    m.method
                ),
            )
        }
    };
    Ok(Preview {
        action,
        device: f.device_now.clone(),
        generation: f.checklist.generation,
        removes,
        keeps,
        words,
    })
}

/// What the person would confirm, after every check passes.
pub fn preview(action: Action, f: &Facts<'_>) -> Result<Preview, Refusal> {
    checks(action, f)
}

/// Makes the request, checking everything again now, with the person's fresh confirmation of
/// exactly the preview these facts give (used once).
pub fn request(
    action: Action,
    f: &Facts<'_>,
    confirmation: Option<&Confirmation>,
    used: &mut Used,
) -> Result<Request, Refusal> {
    let p = checks(action, f)?;
    let c = confirmation.ok_or(Refusal::NoFreshConfirmation)?;
    if c.action != action {
        return Err(Refusal::WrongConfirmation);
    }
    if c.preview != p.digest() {
        return Err(Refusal::ChangedSinceConfirmed);
    }
    if c.made_at > f.now || f.now - c.made_at > FRESH {
        return Err(Refusal::Expired);
    }
    if !used.0.insert(c.nonce) {
        return Err(Refusal::AlreadyUsed);
    }
    Ok(Request {
        action,
        device: p.device,
        generation: p.generation,
        removes: p.removes,
        confirmed_at: c.made_at,
    })
}

/// How the system says a native action went (recorded as observed; T27-021 to T27-024, T27-026).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActionResult {
    /// The system reported it finished, and the check after it passed.
    Finished,
    /// Some of it was done.
    Partial,
    /// What happened cannot be told.
    Inconclusive,
    /// It stopped part way (power lost, say).
    Interrupted,
    /// The person cancelled after some of it was done.
    CancelledAfterSomeEffects,
    /// No qualified method for this drive.
    NotSupported,
}

impl ActionResult {
    /// The plain words: what was observed, and what may already be gone. Never a certificate.
    pub fn words(&self) -> &'static str {
        match self {
            ActionResult::Finished => {
                "The system reported it finished. This is a record of what PCTwin asked for and \
                 what the system said, not a certificate that the drive can never be read."
            }
            ActionResult::Partial => {
                "Only part of it was done. What was removed cannot be brought back from the old \
                 laptop; the rest is still there."
            }
            ActionResult::Inconclusive => {
                "PCTwin cannot tell how far it got. Treat the old laptop as neither kept nor \
                 cleared: do not rely on it as a copy, and do not hand it on yet."
            }
            ActionResult::Interrupted => {
                "It stopped part way. Some things may already be gone. Do not rely on the old \
                 laptop as a copy."
            }
            ActionResult::CancelledAfterSomeEffects => {
                "You cancelled, but some things were already removed. They cannot be brought \
                 back from the old laptop."
            }
            ActionResult::NotSupported => {
                "PCTwin cannot erase this drive safely yet. Use the laptop's own reset with drive \
                 cleaning (on Windows: Reset this PC, Remove everything, Clean the drive; on a \
                 Mac: Erase All Content and Settings)."
            }
        }
    }
}
