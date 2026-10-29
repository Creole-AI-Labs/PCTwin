//! Handover gates (Task List 2.7 "Handing it on" and "Safe goodbye"; Security Design S14
//! "Retirement ceremony" and B "Undo removes only proven unchanged copies"; plan 2.7 sections 7
//! and 8; acceptance T27-016 to T27-024, T27-026).
//!
//! Two separate requests, none following from another:
//!
//! - [`Action::RemoveMoversAccount`]: the mover's own account on the old laptop, removed by the
//!   person with the system's own tools; every other account is left as it is;
//! - [`Action::EraseOldLaptop`]: erasing the old laptop, with a method qualified for that exact
//!   drive if one ever is, and otherwise by the person with the system's own reset.
//!
//! PCTwin never removes an account or erases a drive on the old laptop itself: the admin helper
//! never writes or deletes there (Security Design C, "Never"), and no erase method is qualified
//! for any drive yet. So both requests end at the system's own route ([`Method::SystemRoute`]),
//! and what happened is recorded as the person's word ([`ActionResult::PersonSaidDone`]), never as
//! a certificate.
//!
//! Each request passes one gate, checked when it is previewed and again, in full, when it is
//! confirmed:
//!
//! 1. the exact laptop: the old laptop paired and scanned now is the one the move came from; and
//!    when a drive is named, that exact drive, never one that merely has the same letter or label
//!    (T27-019);
//! 2. the before-wipe list built from the old laptop as it is now, with every exception resolved
//!    or accepted (T27-012, T27-014);
//! 3. for an action PCTwin would run itself, no other program can write to what it removes
//!    (T27-020);
//! 4. a fresh, separate confirmation from the person of exactly the preview shown, for this action,
//!    used once (T27-017). A finished move is never that confirmation;
//! 5. then, and only then, undo is closed for good, on disk, through the journal's own close:
//!    both actions remove the originals undo relies on. If it cannot be closed (an undo is part
//!    way, another closer is busy), the request is refused and undo stays as it was (T27-018).
//!
//! Results are recorded as observed ([`ActionResult`]): part-way, interrupted or cancelled says what
//! may already be gone (T27-021 to T27-024, T27-026).

use std::collections::BTreeSet;

use pctwin_journal::{ClosedToken, Journal, Resolved, UndoGate};
use pctwin_record::{LaptopId, Record};
use serde::{Deserialize, Serialize};

use crate::goodbye::Checklist;

/// How long a confirmation stays fresh.
pub const FRESH: u64 = 10 * 60;

/// One of the two separate requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Action {
    RemoveMoversAccount,
    EraseOldLaptop,
}

/// A drive, exactly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriveIdentity {
    /// The drive's serial number as the system reports it.
    pub serial: String,
    pub model: String,
    pub bytes: u64,
}

/// The old laptop, and its drive when the system named it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceIdentity {
    pub laptop: LaptopId,
    pub drive: Option<DriveIdentity>,
}

/// Whether other programs are kept from writing while an action PCTwin runs itself goes on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteExclusion {
    Held,
    CannotExclude,
}

/// An account on the old laptop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    /// The system's own account ID.
    pub id: String,
    /// Whether it holds files of its own.
    pub has_own_files: bool,
}

/// An erase method qualified for one exact drive (none is yet).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualifiedMethod {
    pub drive: DriveIdentity,
    pub method: String,
}

/// How the action is done.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "method")]
pub enum Method {
    /// A method qualified for this exact drive, which PCTwin would run (`name`, the method's key).
    Qualified { name: String },
    /// The person does it with the system's own tool; PCTwin opens its page and says the steps.
    SystemRoute,
}

/// Something the person must know before confirming (a key: the screens hold the words).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Note {
    /// Deleting files on an SSD is not a secure wipe (Task List 2.7).
    SsdNotSecureWipe,
    /// Removing an account does not cancel subscriptions or sign out elsewhere.
    DoesNotCancelSubscriptions,
    /// Everyone else's account stays as it is.
    OthersStay,
    /// This cannot be undone.
    CannotBeUndone,
    /// Confirming closes undo for this move for good.
    ClosesUndo,
    /// An account cannot remove itself: sign in with another administrator account first.
    SignInAsAnotherAdministrator,
    /// What PCTwin records is what was asked and what was said to happen, not a certificate.
    NotACertificate,
    /// Turning on the drive's encryption before the reset makes leftovers far harder to read.
    EncryptFirstHelps,
}

/// What PCTwin knows now, read again for every preview and every confirmation.
pub struct Facts<'a> {
    pub checklist: &'a Checklist,
    /// A fresh scan of the old laptop (`None`: it was not checked since the move).
    pub old_laptop_now: Option<&'a Record>,
    /// This move's journal on the new laptop (its undo gate is closed by a confirmed request).
    pub journal: &'a Journal,
    /// The old laptop as recorded when the move was made.
    pub expected: &'a DeviceIdentity,
    /// The old laptop as it is now.
    pub device_now: &'a DeviceIdentity,
    pub write_exclusion: WriteExclusion,
    /// The accounts known on the old laptop.
    pub accounts: &'a [Account],
    /// The mover's account ID on the old laptop.
    pub mover: &'a str,
    pub qualified: &'a [QualifiedMethod],
    pub now: u64,
}

/// Why a request was refused. Nothing was done.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "kebab-case", tag = "refused")]
pub enum Refusal {
    #[error("this is not the same laptop or drive")]
    WrongDevice,
    #[error("undo could not be closed for good")]
    UndoNotClosed,
    #[error("the old laptop was not checked again")]
    OldLaptopNotReachable,
    #[error("the old laptop changed since the list was made")]
    ChecklistStale,
    #[error("some things on the list are not resolved or accepted")]
    ExceptionsOpen { open: usize },
    #[error("other programs could write while this runs")]
    NoWriteExclusion,
    #[error("the mover's account is not on this laptop")]
    NoSuchAccount,
    #[error("other people keep their own files on this laptop")]
    OtherPeopleUseIt,
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
    /// What is kept, by name (another person's account).
    pub keeps: Vec<String>,
    pub method: Method,
    pub notes: Vec<Note>,
}

impl Preview {
    pub fn digest(&self) -> [u8; 32] {
        let mut h = blake3::Hasher::new();
        h.update(b"pctwin/v2/handover-preview\0");
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

/// The person confirms the preview with digest `preview` for `action`, at `made_at` (`nonce`: new
/// random bytes for each confirmation).
pub fn confirm(action: Action, preview: [u8; 32], made_at: u64, nonce: [u8; 16]) -> Confirmation {
    Confirmation {
        action,
        preview,
        made_at,
        nonce,
    }
}

/// Confirmations already used.
#[derive(Debug, Default)]
pub struct Used(BTreeSet<[u8; 16]>);

/// The gate's request, made only when every check passed and undo is closed for good.
#[derive(Debug)]
pub struct Request {
    pub action: Action,
    pub device: DeviceIdentity,
    pub generation: u64,
    pub removes: Vec<String>,
    pub method: Method,
    pub confirmed_at: u64,
    /// The journal's own proof that undo is closed for good, on disk.
    pub undo_closed: ClosedToken,
}

fn same_device(f: &Facts<'_>) -> bool {
    let laptop = f.device_now.laptop == f.expected.laptop
        && f.checklist.move_id.source == f.expected.laptop
        && f.old_laptop_now
            .is_none_or(|r| r.source_laptop == f.expected.laptop);
    // A drive named when the move was made must be that exact drive now.
    let drive = match (&f.expected.drive, &f.device_now.drive) {
        (Some(was), Some(now)) => !was.serial.trim().is_empty() && was == now,
        (Some(_), None) => false,
        (None, _) => true,
    };
    laptop && drive
}

fn qualified<'a>(f: &Facts<'a>) -> Option<&'a QualifiedMethod> {
    let drive = f.device_now.drive.as_ref()?;
    if f.expected.drive.as_ref() != Some(drive) || drive.serial.trim().is_empty() {
        return None;
    }
    f.qualified.iter().find(|m| &m.drive == drive)
}

fn checks(action: Action, f: &Facts<'_>) -> Result<Preview, Refusal> {
    if !same_device(f) {
        return Err(Refusal::WrongDevice);
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
    let undo_open = !matches!(f.journal.undo_gate(), Ok(UndoGate::Closed));
    let mut notes = Vec::new();
    let (removes, keeps, method) = match action {
        Action::RemoveMoversAccount => {
            if !f.accounts.iter().any(|a| a.id == f.mover) {
                return Err(Refusal::NoSuchAccount);
            }
            notes.extend([
                Note::SignInAsAnotherAdministrator,
                Note::OthersStay,
                Note::DoesNotCancelSubscriptions,
                Note::SsdNotSecureWipe,
            ]);
            (
                vec![f.mover.to_string()],
                f.accounts
                    .iter()
                    .filter(|a| a.id != f.mover)
                    .map(|a| a.id.clone())
                    .collect(),
                Method::SystemRoute,
            )
        }
        Action::EraseOldLaptop => {
            if f.accounts
                .iter()
                .any(|a| a.id != f.mover && a.has_own_files)
            {
                return Err(Refusal::OtherPeopleUseIt);
            }
            let method = match qualified(f) {
                Some(m) => {
                    if f.write_exclusion != WriteExclusion::Held {
                        return Err(Refusal::NoWriteExclusion);
                    }
                    Method::Qualified {
                        name: m.method.clone(),
                    }
                }
                None => Method::SystemRoute,
            };
            notes.extend([
                Note::CannotBeUndone,
                Note::SsdNotSecureWipe,
                Note::EncryptFirstHelps,
            ]);
            let removes = match &f.device_now.drive {
                Some(d) => vec![d.model.clone()],
                None => vec![],
            };
            (removes, vec![], method)
        }
    };
    if undo_open {
        notes.push(Note::ClosesUndo);
    }
    notes.push(Note::NotACertificate);
    Ok(Preview {
        action,
        device: f.device_now.clone(),
        generation: f.checklist.generation,
        removes,
        keeps,
        method,
        notes,
    })
}

/// What the person would confirm, after every check passes.
pub fn preview(action: Action, f: &Facts<'_>) -> Result<Preview, Refusal> {
    checks(action, f)
}

/// Makes the request, checking everything again now, with the person's fresh confirmation of
/// exactly the preview these facts give (used once); then closes undo for good. Only an undo part
/// way, or another close under way, keeps it from closing: then nothing is done and undo carries
/// on as before.
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
    // Irreversible from here: every other check passed. An undo left part way is not finished
    // here (it is not this step's to finish): closing refuses, and nothing changes.
    let token = f
        .journal
        .close_undo(&mut |_, _| {
            Ok(Resolved::Unresolved {
                why: "an undo is part way; finish it first".into(),
            })
        })
        .map_err(|_| Refusal::UndoNotClosed)?;
    if !token.is_for(f.journal) || !matches!(f.journal.undo_gate(), Ok(UndoGate::Closed)) {
        return Err(Refusal::UndoNotClosed);
    }
    Ok(Request {
        action,
        device: p.device,
        generation: p.generation,
        removes: p.removes,
        method: p.method,
        confirmed_at: c.made_at,
        undo_closed: token,
    })
}

/// What the person says happened after following the system's route.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PersonSays {
    Done,
    Partly,
    CancelledPartWay,
    NotStarted,
    CannotTell,
}

/// How an action went, as recorded (T27-021 to T27-024, T27-026). Never a certificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActionResult {
    /// The person says the system's route finished: their word, not PCTwin's check.
    PersonSaidDone,
    /// Some of it was done.
    Partial,
    /// What happened cannot be told.
    Inconclusive,
    /// It stopped part way (power lost, say).
    Interrupted,
    /// The person cancelled after some of it was done.
    CancelledAfterSomeEffects,
    /// Nothing was started.
    NotStarted,
}

impl ActionResult {
    /// The record of what the person says happened. Undo stays closed whatever they say: what
    /// may already be gone cannot be brought back from the old laptop.
    pub fn from_person(says: PersonSays) -> Self {
        match says {
            PersonSays::Done => ActionResult::PersonSaidDone,
            PersonSays::Partly => ActionResult::Partial,
            PersonSays::CancelledPartWay => ActionResult::CancelledAfterSomeEffects,
            PersonSays::NotStarted => ActionResult::NotStarted,
            PersonSays::CannotTell => ActionResult::Inconclusive,
        }
    }
}
