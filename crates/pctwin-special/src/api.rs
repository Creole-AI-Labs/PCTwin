//! The 2.5 API the move calls. Stable once published: change it only by adding.
//!
//! **On the old laptop**
//! 1. [`plan`]: one person's app data, found by general rules and classified by content, given
//!    their home folder and 2.1's app list. Every component comes back with its class (P, S, E, D
//!    or unresolved), the person's step, a plain-words reason and whether it is proposed by
//!    default (secrets never are). Everything not moved is listed with its reason
//!    ([`Plan::not_moved`]), and what was searched ([`Plan::coverage`]).
//! 2. [`approve`]: binds a component to the approved plan at its current generation.
//! 3. [`capture_planned`]: captures it consistently into a private PCTwin work folder, with the
//!    2.5 grant ([`Grant::Ordinary`] for data, [`Grant::Secret`] for a secret the person chose).
//!
//! **On the new laptop**
//! 4. [`apply`]: lands the captured component under the person's home through the safety gate,
//!    journaled (intent, observed, receipt) by the imports role, never over an existing folder
//!    (keep both, or skip), secrets owner-only; returns the one result word and the six outcome
//!    dimensions ([`Applied`]).
//! 5. [`undo`]: undoes a landing, keeping anything the person changed since (Windows first).
//! 6. [`result_without_landing`]: the result word of a component that has nothing to land (a
//!    sign-in, a relink, an export to make, something unresolved, or left out by the person).
//! 7. [`before_wipe`]: the steps that must be settled before the old laptop is wiped, for the
//!    2.7 checklist ([`pctwin_readiness`'s `Inputs::special`]).
//!
//! Windows first: on a Mac or Linux the discovery and classification run, but whether an app is
//! still open cannot be told on a Mac (a store is then kept as a labelled copy, never "copied"),
//! and undo of a landing is not built there yet ("not done this time").

use std::path::{Path, PathBuf};

use pctwin_catalogue::Catalogue;
use pctwin_gate::Destination;
use pctwin_grant::{CaptureGrant, Now, PlanSeal, RootRef, SecretGrant};
use pctwin_journal::{Imports, PlannedEffect, Recorder, UndoRight};
use pctwin_readiness::goodbye::{ExceptionKind, SpecialStep};
use pctwin_recipe::SettingsRoot;
use pctwin_record::{FoundApp, Owner};
use pctwin_types::ItemResult;

use crate::capture::{self, Approved, Capture, CaptureError, Holders, Method, Stop};
use crate::catalogue::SpecialRow;
use crate::classify::{FolderReport, Placement, Skip, placement};
use crate::component::{
    Access, Component, Custody, Evidence, Import, Outcome, Preservation, Readiness, Reversal,
    Transport,
};
use crate::handoff::{Step, step_for};
use crate::land::{self, Conflict, LandError, Landed, Made, OwnerOnly, Target, Undone};
use crate::roots::{Coverage, RootKind, System, find_folders, roots_of};
use crate::select::Selection;

/// One component, planned.
#[derive(Debug, Clone)]
pub struct Planned {
    /// The app's name to show (a catalogue row's, else its folder's).
    pub app: String,
    pub component: Component,
    /// The app data folder on the old laptop.
    pub base: PathBuf,
    /// The person's step (none when it simply copies).
    pub step: Step,
    /// Why, in plain words.
    pub why: String,
    /// Proposed in the default selection (never a secret; never anything that does not copy).
    pub proposed: bool,
    /// Where it goes on the new laptop.
    pub placement: Placement,
    /// The folder's root on the old laptop.
    pub root: RootKind,
}

/// A thing found and not moved, with why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotMovedItem {
    pub app: String,
    /// One of them, as an example (its path inside the app's folder).
    pub rel: String,
    pub why: String,
    /// How many there are in this app's folder for this reason.
    pub count: usize,
}

/// The most time [`plan`] takes in all, across every root; folders not reached are reported
/// "not fully checked" (the least recently used ones, since folders are taken newest first).
pub const PLAN_TIME: std::time::Duration = std::time::Duration::from_secs(270);

/// One person's special-category plan.
#[derive(Debug, Clone)]
pub struct Plan {
    pub components: Vec<Planned>,
    pub not_moved: Vec<NotMovedItem>,
    pub coverage: Coverage,
    /// The classified folders themselves (for the screens' detail views).
    pub reports: Vec<FolderReport>,
}

impl Plan {
    /// The order to land components in: every secret first, then the rest, each in plan order
    /// (a secret makes its app's owner-only folder; the app's other data then joins it).
    pub fn landing_order(&self) -> Vec<&Planned> {
        let (mut secrets, rest): (Vec<&Planned>, Vec<&Planned>) =
            self.components.iter().partition(|p| p.component.secret);
        secrets.extend(rest);
        secrets
    }
}

/// Plans `owner`'s app data: `home` is their home folder on this (old) laptop, `apps` 2.1's app
/// list, `from` this laptop's system and `to` the new laptop's.
pub fn plan(
    owner: &Owner,
    home: &Path,
    apps: &[FoundApp],
    from: System,
    to: System,
    catalogue: &Catalogue<SpecialRow>,
) -> Plan {
    plan_within(
        owner,
        home,
        apps,
        from,
        to,
        catalogue,
        std::time::Instant::now() + PLAN_TIME,
    )
}

/// [`plan`] finished by `deadline`, across every root: app folders are taken most recently
/// written first (by the folder's own last-write time), each within its own time
/// ([`crate::classify::FOLDER_TIME`]) and never past `deadline`; a folder reached after it is
/// reported "not fully checked" without being read.
#[allow(clippy::too_many_arguments)]
pub fn plan_within(
    owner: &Owner,
    home: &Path,
    apps: &[FoundApp],
    from: System,
    to: System,
    catalogue: &Catalogue<SpecialRow>,
    deadline: std::time::Instant,
) -> Plan {
    let roots = roots_of(from, home, &|v| std::env::var_os(v).map(PathBuf::from));
    let (folders, coverage) = find_folders(owner, home, &roots, apps);
    let reports = crate::classify::classify_newest_first(folders, from, catalogue, deadline);
    let selection = Selection::default_for(&reports);
    let mut components = Vec::new();
    let mut not_moved = Vec::new();
    for r in &reports {
        let app = r.name();
        let place = placement(r, from, to);
        for c in &r.components {
            components.push(Planned {
                app: app.clone(),
                why: why_words(c),
                step: step_for(r, c),
                proposed: selection.contains(c.id),
                component: c.clone(),
                base: r.folder.path(),
                placement: place.clone(),
                root: r.folder.root,
            });
        }
        // One line per reason in this app's folder, with how many.
        let mut by_why: std::collections::BTreeMap<&'static str, (String, usize)> =
            std::collections::BTreeMap::new();
        for n in &r.not_moved {
            let e = by_why
                .entry(n.why.words())
                .or_insert_with(|| (n.rel.clone(), 0));
            e.1 += 1;
        }
        for (why, (rel, count)) in by_why {
            not_moved.push(NotMovedItem {
                app: app.clone(),
                rel,
                why: why.to_string(),
                count,
            });
        }
    }
    for (path, why) in &coverage.skipped {
        not_moved.push(NotMovedItem {
            app: String::new(),
            rel: path.to_string_lossy().into_owned(),
            why: why.clone(),
            count: 1,
        });
    }
    Plan {
        components,
        not_moved,
        coverage,
        reports,
    }
}

fn why_words(c: &Component) -> String {
    let facts: Vec<String> = c
        .evidence
        .iter()
        .filter(|e| !matches!(e, Evidence::Catalogue { .. }))
        .map(Evidence::words)
        .collect();
    let what = facts.join("; ");
    let then = match c.custody {
        Custody::Portable if c.secret => "it moves only if you choose it",
        Custody::Portable => "it is copied",
        Custody::ServiceHeld if c.preserve => {
            "sign in again on the new laptop; a copy is kept in case something never reached the service"
        }
        Custody::ServiceHeld => "sign in again on the new laptop",
        Custody::NeedsExport => "use the app's own export on the old laptop, then its import",
        Custody::MachineBound => "it cannot be copied; set it up again on the new laptop",
        Custody::Unresolved => "PCTwin could not tell what it is, so it is not moved; look at it",
    };
    format!("{what}: {then}")
}

/// Binds `p` to the approved plan `seal`, read from the approved root `root`, as plan item
/// `handle`, at its current generation.
pub fn approve(p: &Planned, seal: PlanSeal, root: RootRef, handle: u64) -> Approved {
    Approved {
        seal,
        component: p.component.id,
        root,
        generation: capture::generation(&p.base, &p.component.members),
        handle,
    }
}

/// The 2.5 grant for one step: ordinary data, or a secret the person chose.
#[derive(Debug)]
pub enum Grant {
    Ordinary(CaptureGrant),
    Secret(SecretGrant),
}

/// Captures `p` on the old laptop into the private folder `work`.
pub fn capture_planned(
    p: &Planned,
    grant: Grant,
    approved: &Approved,
    work: &Path,
    holders: &dyn Holders,
    stop: &Stop,
    now: Now,
) -> Result<Capture, CaptureError> {
    match grant {
        Grant::Ordinary(g) => {
            capture::capture(g, approved, &p.component, &p.base, work, holders, stop, now)
        }
        Grant::Secret(g) => {
            capture::capture_secret(g, approved, &p.component, &p.base, work, holders, stop, now)
        }
    }
}

/// The new laptop, for one person.
pub struct NewLaptop<'a> {
    /// The imports role's recorder.
    pub rec: &'a Recorder<'a, Imports>,
    /// The person's home folder on the new laptop, opened as an approved destination.
    pub home: &'a Destination,
    pub home_path: &'a Path,
    /// The approved root the import grant names for that home.
    pub root: RootRef,
    pub seal: PlanSeal,
    pub system: System,
    pub owner_only: &'a dyn OwnerOnly,
    pub stop: &'a Stop,
    /// The folders this move made, so each app's components land together. One per move.
    pub made: &'a MadeFolders,
}

/// The folders a move made on the new laptop, by target, so later components of the same app
/// join the folder the first one made. Apply an app's chosen secrets first: they make an
/// owner-only folder the rest then join (a secret never joins a folder made by ordinary data).
#[derive(Debug, Default)]
pub struct MadeFolders(std::sync::Mutex<std::collections::BTreeMap<String, Made>>);

impl MadeFolders {
    fn get(&self, target: &str) -> Option<Made> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(target)
            .cloned()
    }

    fn put(&self, target: &str, made: Made) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .entry(target.to_string())
            .or_insert(made);
    }
}

/// How one component ended on the new laptop.
#[derive(Debug, Clone)]
pub struct Applied {
    pub result: ItemResult,
    pub outcome: Outcome,
    pub landed: Option<Landed>,
    /// What happened, in plain words.
    pub words: String,
}

/// Where `p` lands under the new laptop's home: its root's place there and its folder.
pub fn target_of(p: &Planned, n: &NewLaptop<'_>) -> Option<Target> {
    let (root, folder) = match &p.placement {
        Placement::Same { root, folder } | Placement::Translated { root, folder } => {
            (*root, folder.clone())
        }
        // Kept, not handed to the app: in PCTwin's own folder of things brought over.
        Placement::PreservedOnly => {
            let folder = format!("PCTwin brought over/{}", p.app.replace('/', " - "));
            return Some(Target {
                root: SettingsRoot::Home,
                made: n.made.get(&folder),
                folder,
            });
        }
    };
    let roots = roots_of(n.system, n.home_path, &|_| None);
    let at = roots.iter().find(|r| r.kind == root)?;
    let rel = at.path.strip_prefix(n.home_path).ok()?;
    let mut parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    parts.push(folder);
    let folder = parts.join("/");
    Some(Target {
        root: SettingsRoot::Home,
        made: n.made.get(&folder),
        folder,
    })
}

/// Lands the captured component `p` (its files in `work`) on the new laptop.
#[allow(clippy::too_many_arguments)]
pub fn apply(
    n: &NewLaptop<'_>,
    grant: Grant,
    approved: &Approved,
    planned: &PlannedEffect,
    p: &Planned,
    capture: &Capture,
    work: &Path,
    conflict: Conflict,
    now: Now,
) -> Applied {
    // A capture whose consistency was not established, or that is partial, is kept in PCTwin's
    // own folder of things brought over, never where the app would open it.
    let kept_aside;
    let p = if matches!(
        capture.preservation,
        Preservation::PreservedOnly | Preservation::Partial
    ) {
        kept_aside = Planned {
            placement: Placement::PreservedOnly,
            ..p.clone()
        };
        &kept_aside
    } else {
        p
    };
    let Some(target) = target_of(p, n) else {
        return finished(
            p,
            Some(capture),
            None,
            "the new laptop has no place for it".into(),
        );
    };
    let landed = match grant {
        Grant::Ordinary(g) => land::land(
            n.rec, g, n.seal, n.root, planned, n.home, &target, capture, work, conflict, n.stop,
            now,
        ),
        Grant::Secret(g) => land::land_secret(
            n.rec,
            g,
            approved,
            planned,
            n.home,
            n.home_path,
            &target,
            capture,
            work,
            n.owner_only,
            n.stop,
            now,
        ),
    };
    match landed {
        Ok(l) => {
            n.made.put(
                &target.folder,
                Made {
                    folder: l.folder.clone(),
                    beside: l.beside,
                },
            );
            let words = if l.beside {
                format!(
                    "brought over beside the folder already on the new laptop, as {}; the app uses it once you choose it",
                    l.folder
                )
            } else {
                format!("brought over to {}", l.folder)
            };
            finished(p, Some(capture), Some(l), words)
        }
        Err(LandError::Skipped) => Applied {
            result: ItemResult::skipped_by_user(),
            outcome: Outcome {
                preservation: Preservation::NotSelected,
                ..Outcome::pending()
            },
            landed: None,
            words: "left out: the app's folder is already on the new laptop".into(),
        },
        Err(e) => finished(p, Some(capture), None, e.to_string()),
    }
}

fn finished(
    p: &Planned,
    capture: Option<&Capture>,
    landed: Option<Landed>,
    words: String,
) -> Applied {
    let outcome = outcome_of(p, capture, landed.as_ref());
    Applied {
        result: outcome.result(p.component.custody),
        outcome,
        landed,
        words,
    }
}

/// Undoes landing `effect`, keeping anything changed since.
pub fn undo(
    rec: &Recorder<'_, Imports>,
    right: &impl UndoRight,
    home: &Destination,
    effect: u64,
) -> Result<Undone, LandError> {
    land::undo_landing(rec, right, home, effect)
}

/// The result of a component with nothing to land: not chosen, a sign-in, a relink, an export,
/// something unresolved.
pub fn result_without_landing(p: &Planned, chosen: bool) -> Applied {
    if !chosen {
        return Applied {
            result: ItemResult::skipped_by_user(),
            outcome: Outcome {
                preservation: Preservation::NotSelected,
                ..Outcome::pending()
            },
            landed: None,
            words: "left out".into(),
        };
    }
    finished(p, None, None, p.why.clone())
}

fn has(c: &Component, f: impl Fn(&Evidence) -> bool) -> bool {
    c.evidence.iter().any(f)
}

/// The six dimensions of `p` from what happened to it.
pub fn outcome_of(p: &Planned, capture: Option<&Capture>, landed: Option<&Landed>) -> Outcome {
    let c = &p.component;
    let access = match c.custody {
        Custody::ServiceHeld => Access::SignInNeeded,
        Custody::MachineBound => Access::NeedsReenrolment,
        Custody::Unresolved => Access::Unknown,
        Custody::Portable | Custody::NeedsExport => {
            if has(c, |e| matches!(e, Evidence::MissingPart { .. }) && c.secret) {
                Access::MissingKeyComponent
            } else if has(c, |e| {
                matches!(
                    e,
                    Evidence::PasswordDatabase
                        | Evidence::EncryptedPrivateKey
                        | Evidence::PasswordKeyFile
                )
            }) {
                Access::AwaitingUnlock
            } else if has(c, |e| matches!(e, Evidence::KeyDatabase)) {
                Access::KeyAvailable
            } else {
                Access::NotNeeded
            }
        }
    };
    let preservation = capture.map_or(Preservation::Pending, |k| k.preservation);
    let transport = match (c.custody, landed) {
        (Custody::ServiceHeld, None) if !c.preserve => Transport::NotApplicable,
        (Custody::MachineBound | Custody::Unresolved, None) => Transport::NotApplicable,
        (_, Some(l)) if l.verified => Transport::Verified,
        (_, Some(_)) => Transport::Interrupted,
        (_, None) => Transport::NotYet,
    };
    let import = match (landed, &p.placement) {
        (None, _) if matches!(c.custody, Custody::MachineBound | Custody::Unresolved) => {
            Import::NotApplicable
        }
        (None, _) => Import::NotYet,
        (Some(l), _) if !l.verified => Import::Failed,
        (Some(l), _) if l.beside => Import::PreservedOnly,
        (Some(_), Placement::PreservedOnly) => Import::PreservedOnly,
        (Some(_), _) if c.custody == Custody::ServiceHeld => Import::PreservedOnly,
        (Some(_), _) => Import::Placed,
    };
    // A dependency missing, files elsewhere not chosen, or settings that start programs: the
    // person has a step before it is ready.
    let blocked = has(c, |e| {
        matches!(
            e,
            Evidence::MissingPart { .. }
                | Evidence::LinksOutside { .. }
                | Evidence::EnablesCommands
        )
    });
    let readiness = if blocked {
        Readiness::NeedsAction
    } else if landed.is_some_and(|l| l.verified)
        && capture
            .is_some_and(|k| k.method == Method::OnlineBackup || k.method == Method::SteadyCopy)
    {
        Readiness::MachineChecked
    } else {
        Readiness::Untested
    };
    Outcome {
        preservation,
        transport,
        access,
        import,
        readiness,
        reversal: if landed.is_some() {
            Reversal::LocallyReversible
        } else {
            Reversal::NotApplicable
        },
    }
}

/// What must be settled before the old laptop is wiped, from each planned component and how it
/// ended (`ended`: the component's result; `None` when nothing was done with it).
pub fn before_wipe(
    plan: &Plan,
    ended: &dyn Fn(&Planned) -> Option<ItemResult>,
) -> Vec<SpecialStep> {
    use pctwin_types::ItemStatus;
    let mut out = Vec::new();
    for p in &plan.components {
        let done = ended(p)
            .is_some_and(|r| matches!(r.status(), ItemStatus::Copied | ItemStatus::Converted));
        if done {
            continue;
        }
        let c = &p.component;
        let kind = match c.custody {
            Custody::MachineBound | Custody::ServiceHeld if !c.preserve => {
                ExceptionKind::TiedToOldLaptop
            }
            Custody::NeedsExport => ExceptionKind::DoFirstOnOldLaptop,
            _ => ExceptionKind::OnlyOnOldLaptop,
        };
        out.push(SpecialStep {
            kind,
            place: p.app.clone(),
            component: c.id.0.iter().map(|b| format!("{b:02x}")).collect(),
            words: format!("{}: {}", p.app, p.why),
        });
    }
    for n in &plan.not_moved {
        if n.why == Skip::NotRead.words() {
            out.push(SpecialStep {
                kind: ExceptionKind::NotSearched,
                place: format!("{}/{}", n.app, n.rel),
                component: String::new(),
                words: format!("{} in {}: {}", n.rel, n.app, n.why),
            });
        }
    }
    out
}
