//! Before leaving the old laptop (Task List 2.7 "Before any wipe"; Product Spec "After the move";
//! plan 2.7 section 6; acceptance T27-011 to T27-014).
//!
//! The [`Checklist`] is built from what is known, never from a box labelled "all files moved":
//!
//! - what is still only on the old laptop (the ledger's lines not on the new laptop, and what was
//!   left there to fit a smaller new laptop);
//! - keys and recovery material not checked to work on the new laptop, and things tied to the old
//!   laptop (passkeys, authenticator apps) to set up again;
//! - licences and sign-ins to deal with on the old laptop first (a licence with an install limit,
//!   an app that needs signing in or activating);
//! - steps of the move that did not finish;
//! - places on the old laptop PCTwin did not search;
//! - whether undo is still open.
//!
//! Each [`Exception`] is resolved by evidence (the list is built again and it is gone), done by
//! the person where only the person can do it, or accepted as a loss after its exact warning. An
//! old laptop that cannot be reached cannot be accepted away: the list cannot be checked again,
//! so no automated clean-up may follow (T27-014). The list is bound to the old laptop's latest scan
//! ([`Checklist::is_current`]): when the old laptop has changed since, it must be built again
//! (T27-012). It is saved on the new laptop ([`Checklist::save`]).

use std::collections::BTreeSet;
use std::path::Path;

use pctwin_orchestrate::jobs::Missing as MissingJob;
use pctwin_record::{Fingerprint, ItemId, LeftOutReason, Portability, Record};
use pctwin_types::Reason;
use serde::{Deserialize, Serialize};

use crate::ledger::{EvidenceKind, Ledger, MoveId, Selection};
use crate::store;

/// The saved list's format.
pub const CHECKLIST_FORMAT: u32 = 2;
/// The keep-until date PCTwin suggests: this long after the list is made.
pub const SUGGESTED_KEEP: u64 = 30 * 24 * 60 * 60;

/// What one exception is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExceptionKind {
    OnlyOnOldLaptop,
    KeyNotChecked,
    TiedToOldLaptop,
    DoFirstOnOldLaptop,
    StepsNotFinished,
    NotSearched,
    OldLaptopNotReachable,
}

/// How an exception stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "resolution")]
pub enum Resolution {
    Open,
    /// The person says they did it (a step only they can do): recorded as their word.
    PersonDidIt {
        at: u64,
    },
    /// The person accepted losing it, after its exact warning: a decision, not a recovery.
    AcceptedLoss {
        at: u64,
    },
}

/// One thing to resolve or accept before leaving the old laptop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exception {
    /// Names exactly this kind and these items, places and steps.
    pub id: [u8; 32],
    pub kind: ExceptionKind,
    pub items: Vec<ItemId>,
    pub places: Vec<String>,
    pub steps: Vec<String>,
    /// For a special-category step, what it asks (the screens put it in words).
    pub what: Option<StepWhat>,
    pub resolution: Resolution,
}

/// What a special-category step (2.5) asks of the person. A key: the screens hold the words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StepWhat {
    /// Sign in to it again on the new laptop.
    SignInAgain,
    /// Link it again (to a phone, a security key) on the new laptop.
    Relink,
    /// Make its export on the old laptop first.
    ExportFirst,
    /// Look at it yourself: PCTwin could not tell what kind of data it is.
    LookAtIt,
}

impl Exception {
    fn new(
        kind: ExceptionKind,
        items: Vec<ItemId>,
        places: Vec<String>,
        steps: Vec<String>,
        what: Option<StepWhat>,
    ) -> Self {
        let mut h = blake3::Hasher::new();
        h.update(b"pctwin/v2/goodbye-exception\0");
        h.update(&serde_json::to_vec(&(kind, &items, &places, &steps, what)).expect("serialises"));
        Self {
            id: *h.finalize().as_bytes(),
            kind,
            items,
            places,
            steps,
            what,
            resolution: Resolution::Open,
        }
    }

    /// Whether the person may accept it as a loss. An old laptop that cannot be reached cannot be.
    pub fn acceptable(&self) -> bool {
        self.kind != ExceptionKind::OldLaptopNotReachable
    }

    /// Whether only the person can do it, so their word resolves it.
    pub fn person_can_do(&self) -> bool {
        matches!(
            self.kind,
            ExceptionKind::DoFirstOnOldLaptop | ExceptionKind::TiedToOldLaptop
        )
    }

    pub fn is_open(&self) -> bool {
        self.resolution == Resolution::Open
    }
}

/// The old laptop now.
#[derive(Debug, Clone, Copy)]
pub enum OldLaptop<'a> {
    /// Connected, with a fresh scan made now.
    Scanned(&'a Record),
    NotReachable,
}

/// What the list is built from.
pub struct Inputs<'a> {
    pub ledger: &'a Ledger,
    /// The approved plan (licences with install limits are read from it).
    pub approved: &'a Record,
    pub old_laptop: OldLaptop<'a>,
    /// Left on the old laptop to fit a smaller new laptop (2.6 capacity accounting).
    pub left_on_old_laptop: &'a [ItemId],
    /// Steps of the move that did not finish (2.6 scheduler).
    pub steps_not_finished: &'a [MissingJob],
    /// Whether undo is still open (the journal's own gate).
    pub undo_open: bool,
    /// Special-category steps (2.5): a sign-in or relink to do on the new laptop, an export to
    /// make on the old laptop first, data whose kind could not be told, a labelled copy of
    /// something a service holds, a secret not chosen to move.
    pub special: &'a [SpecialStep],
}

/// One special-category step (2.5) that must be settled before the old laptop is wiped. 2.5
/// builds these from each component's outcome; the list shows them like any other exception.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecialStep {
    pub kind: ExceptionKind,
    /// Where it is on the old laptop (the app data folder, as shown to the person).
    pub place: String,
    /// The component it is about (its identity, as text), so two steps never merge.
    pub component: String,
    /// What it asks.
    pub what: StepWhat,
}

/// Why a change to the list was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ChecklistError {
    #[error("no such thing on the list")]
    NoSuchException,
    #[error("the warning agreed to is not the one shown now")]
    NotTheWarningShown,
    #[error("this cannot be accepted: connect the old laptop so PCTwin can check it again")]
    CannotBeAccepted,
    #[error("only PCTwin can check this; it is not something to tick off")]
    NotThePersonsStep,
    #[error("the date must not be before today")]
    DateInThePast,
    #[error("the saved list is from a newer PCTwin")]
    NewerFormat,
}

/// The list made before leaving the old laptop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checklist {
    format: u32,
    pub move_id: MoveId,
    /// The ledger's plan generation it was built from.
    pub generation: u64,
    /// The old laptop's scan it was built from (`None`: it could not be reached).
    pub old_laptop_scan: Option<Fingerprint>,
    pub made_at: u64,
    pub suggested_keep_until: u64,
    pub keep_until: Option<u64>,
    pub undo_open: bool,
    pub exceptions: Vec<Exception>,
}

impl Checklist {
    /// Builds the list, keeping the person's earlier decisions on exactly the same exceptions.
    pub fn build(i: &Inputs<'_>, previous: Option<&Checklist>, now: u64) -> Self {
        let l = i.ledger;
        let mut ex = Vec::new();

        let mut only: BTreeSet<ItemId> = l
            .only_on_old_laptop()
            .into_iter()
            .filter(|id| !l.settled(*id))
            .collect();
        only.extend(
            i.left_on_old_laptop
                .iter()
                .copied()
                .filter(|id| !l.settled(*id)),
        );
        if !only.is_empty() {
            ex.push(Exception::new(
                ExceptionKind::OnlyOnOldLaptop,
                only.into_iter().collect(),
                vec![],
                vec![],
                None,
            ));
        }

        let keys: Vec<ItemId> = l
            .lines()
            .filter(|(_, line)| {
                line.mine
                    && line.needs_key
                    && !line
                        .evidence
                        .iter()
                        .any(|o| o.evidence.kind() == EvidenceKind::RecoveryChecked)
                    && line.accepted_loss.is_none()
            })
            .map(|(id, _)| *id)
            .collect();
        if !keys.is_empty() {
            ex.push(Exception::new(
                ExceptionKind::KeyNotChecked,
                keys.clone(),
                vec![],
                vec![],
                None,
            ));
        }

        let tied: Vec<ItemId> = l
            .lines()
            .filter(|(_, line)| {
                line.mine
                    && line.selection
                        == Selection::Excluded {
                            why: LeftOutReason::DeviceBound,
                        }
            })
            .map(|(id, _)| *id)
            .collect();
        if !tied.is_empty() {
            ex.push(Exception::new(
                ExceptionKind::TiedToOldLaptop,
                tied.clone(),
                vec![],
                vec![],
                None,
            ));
        }

        let mut first: BTreeSet<ItemId> = i
            .approved
            .items
            .iter()
            .filter(|it| it.portability == Portability::LimitedActivations)
            .filter(|it| l.line(it.id).is_some_and(|line| line.mine))
            .map(|it| it.id)
            .collect();
        first.extend(
            l.lines()
                .filter(|(_, line)| {
                    line.mine
                        && line
                            .result
                            .as_ref()
                            .and_then(|a| a.result.reason())
                            .is_some_and(|r| matches!(r, Reason::SignIn | Reason::Activation))
                })
                .map(|(id, _)| *id),
        );
        if !first.is_empty() {
            ex.push(Exception::new(
                ExceptionKind::DoFirstOnOldLaptop,
                first.into_iter().collect(),
                vec![],
                vec![],
                None,
            ));
        }

        if !i.steps_not_finished.is_empty() {
            let steps: Vec<String> = i
                .steps_not_finished
                .iter()
                .map(|m| m.job.0.clone())
                .collect();
            ex.push(Exception::new(
                ExceptionKind::StepsNotFinished,
                vec![],
                vec![],
                steps.clone(),
                None,
            ));
        }

        for s in i.special {
            ex.push(Exception::new(
                s.kind,
                vec![],
                vec![s.place.clone()],
                vec![s.component.clone()],
                Some(s.what),
            ));
        }

        let not_searched: Vec<String> = l.search().not_searched.iter().cloned().collect();
        if !not_searched.is_empty() {
            ex.push(Exception::new(
                ExceptionKind::NotSearched,
                vec![],
                not_searched.clone(),
                vec![],
                None,
            ));
        }

        let scan = match i.old_laptop {
            OldLaptop::Scanned(r) => Some(r.fingerprint()),
            OldLaptop::NotReachable => {
                ex.push(Exception::new(
                    ExceptionKind::OldLaptopNotReachable,
                    vec![],
                    vec![],
                    vec![],
                    None,
                ));
                None
            }
        };

        // The person's earlier decisions carry over to exactly the same exceptions.
        if let Some(prev) = previous.filter(|p| p.move_id == l.id()) {
            for e in &mut ex {
                if let Some(p) = prev.exceptions.iter().find(|p| p.id == e.id) {
                    e.resolution = p.resolution;
                }
            }
        }

        Self {
            format: CHECKLIST_FORMAT,
            move_id: l.id(),
            generation: l.generation(),
            old_laptop_scan: scan,
            made_at: now,
            suggested_keep_until: now + SUGGESTED_KEEP,
            keep_until: previous.and_then(|p| p.keep_until).filter(|t| *t >= now),
            undo_open: i.undo_open,
            exceptions: ex,
        }
    }

    /// The digest binding an answer to exactly the exception shown, for this list: its ID (which
    /// names its kind, items, places, steps and what it asks) and the plan generation. The screens'
    /// words are made from exactly these, so nothing else is shown.
    pub fn shown(&self, id: &[u8; 32]) -> Option<[u8; 32]> {
        let e = self.exceptions.iter().find(|e| &e.id == id)?;
        let mut h = blake3::Hasher::new();
        h.update(b"pctwin/v2/goodbye-shown\0");
        h.update(&e.id);
        h.update(&self.generation.to_le_bytes());
        Some(*h.finalize().as_bytes())
    }

    fn find(&mut self, id: &[u8; 32], shown: [u8; 32]) -> Result<&mut Exception, ChecklistError> {
        let want = self.shown(id).ok_or(ChecklistError::NoSuchException)?;
        if want != shown {
            return Err(ChecklistError::NotTheWarningShown);
        }
        Ok(self
            .exceptions
            .iter_mut()
            .find(|e| &e.id == id)
            .expect("found above"))
    }

    /// The person accepts losing what one exception names, after its exact warning (T27-011).
    pub fn accept(
        &mut self,
        id: &[u8; 32],
        shown: [u8; 32],
        at: u64,
    ) -> Result<(), ChecklistError> {
        let e = self.find(id, shown)?;
        if !e.acceptable() {
            return Err(ChecklistError::CannotBeAccepted);
        }
        e.resolution = Resolution::AcceptedLoss { at };
        Ok(())
    }

    /// The person says they did a step only they can do (T27-013).
    pub fn person_did_it(
        &mut self,
        id: &[u8; 32],
        shown: [u8; 32],
        at: u64,
    ) -> Result<(), ChecklistError> {
        let e = self.find(id, shown)?;
        if !e.person_can_do() {
            return Err(ChecklistError::NotThePersonsStep);
        }
        e.resolution = Resolution::PersonDidIt { at };
        Ok(())
    }

    /// The person picks how long to keep the old laptop untouched.
    pub fn choose_keep_until(&mut self, at: u64, now: u64) -> Result<(), ChecklistError> {
        if at < now {
            return Err(ChecklistError::DateInThePast);
        }
        self.keep_until = Some(at);
        Ok(())
    }

    /// Exceptions still open.
    pub fn open(&self) -> impl Iterator<Item = &Exception> {
        self.exceptions.iter().filter(|e| e.is_open())
    }

    /// Every exception resolved or accepted, and the old laptop reachable when it was built.
    pub fn is_clear(&self) -> bool {
        self.open().next().is_none() && self.old_laptop_scan.is_some()
    }

    /// Whether the list was built from this scan of the old laptop (T27-012).
    pub fn is_current(&self, old_laptop_now: &Record) -> bool {
        self.old_laptop_scan == Some(old_laptop_now.fingerprint())
    }

    /// Saves the list on the new laptop.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        store::save(path, self)
    }

    /// Reads a saved list; a list from a newer PCTwin is refused.
    pub fn load(path: &Path) -> std::io::Result<Self> {
        let list: Self = store::load(path)?;
        if list.format > CHECKLIST_FORMAT {
            return Err(std::io::Error::other(ChecklistError::NewerFormat));
        }
        Ok(list)
    }
}
