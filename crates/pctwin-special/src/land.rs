//! Landing a captured component on the new laptop (B25-03; plan sections 12 and 13).
//!
//! - **Through the safety gate**: every file is written inside the approved folder through the
//!   gate's handle (no link can redirect it), under a temporary name the journal can find again,
//!   read back from the drive and checked block by block against the capture before it gets its
//!   name. Nothing is ever overwritten.
//! - **Never over an existing profile** (T25-016): the component lands in a folder made by this
//!   very landing. When the app's folder is already there (the person used the app on the new
//!   laptop), it lands beside it under a new name ("keep both"), and the result says the app does
//!   not see it until the person chooses ([`Landed::beside`]); or the person skips it. Replacing
//!   the person's folder is not offered yet.
//! - **Journaled** (the imports role's own recorder): the intent (the place, nothing there
//!   before) is on disk before the first byte; then what is there is observed; then the honest
//!   receipt. After a crash, [`reconcile`] looks at what is really there and finishes the record
//!   from that, never by writing again; PCTwin's own unfinished temporary files are removed.
//! - **Secrets** ([`land_secret`]): only with a secret grant naming exactly the component, into a
//!   new folder only the person can open (made through [`OwnerOnly`], which the app hands in: on
//!   Windows the administrator helper library's owner-only folder; on Linux and macOS mode
//!   0700), and each file is then owner-only (0600) on Linux and macOS.
//! - **Stopping** (T25-007): a stop (cancel, or a revoked grant) is honoured before the next file;
//!   what landed so far is observed and the effect ends failed, so undo knows exactly what is
//!   there.

use std::io::Read;
use std::path::Path;

use pctwin_gate::{Destination, Fresh, GateError, IncomingPath};
use pctwin_grant::{
    CaptureEffect, CaptureGrant, Now, PlanSeal, RootRef, SecretEffect, SecretGrant,
};
use pctwin_journal::{
    EffectState, FileHash, Found, ImportIntent, Imports, JournalError, PlannedEffect, Receipt,
    Recorder, StoredEffect,
};
use pctwin_recipe::{RelPath, SettingsPlace, SettingsRoot};
use pctwin_types::Reason;

use crate::capture::{Approved, Capture, Stop};

/// The block size files are checked in when read back.
const BLOCK: u64 = 1 << 20;
/// How many new names are tried beside a folder that is already there.
const BESIDE_TRIES: u32 = 50;

/// Makes a new folder only this account can open. The app implements it with the helper library's
/// owner-only folder on Windows; [`UnixOwnerOnly`] is the Linux and macOS form.
pub trait OwnerOnly {
    /// A new folder at `path` (never one that exists) that only this account may open.
    fn private_folder(&self, path: &Path) -> std::io::Result<()>;
}

/// Mode 0700 from the folder's first moment.
#[cfg(unix)]
#[derive(Debug, Default, Clone, Copy)]
pub struct UnixOwnerOnly;

#[cfg(unix)]
impl OwnerOnly for UnixOwnerOnly {
    fn private_folder(&self, path: &Path) -> std::io::Result<()> {
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new().mode(0o700).create(path)
    }
}

/// Where a component lands: a folder (one or more plain names, `/` between them) inside the
/// approved destination, which is the person's settings root `root` on the new laptop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub root: SettingsRoot,
    pub folder: String,
    /// The folder this move already made for the same target (an earlier component of the same
    /// app landed there): later components join it instead of landing beside it.
    pub made: Option<Made>,
}

/// A folder this move made for a target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Made {
    pub folder: String,
    /// It was made beside a folder that was already there.
    pub beside: bool,
}

/// What to do when the target folder is already there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conflict {
    /// Land beside it under a new name: both are kept.
    KeepBoth,
    /// Leave this component out.
    Skip,
}

/// A component that landed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Landed {
    /// Its effect in the journal.
    pub effect: u64,
    /// The folder it landed in, inside the approved destination.
    pub folder: String,
    /// It landed beside a folder that was already there: the app does not see it until the
    /// person chooses.
    pub beside: bool,
    /// Every byte was read back and matched.
    pub verified: bool,
}

/// Why a landing did not happen.
#[derive(Debug, thiserror::Error)]
pub enum LandError {
    #[error("this permission is not for this step: {0}")]
    Denied(&'static str),
    #[error("the folder is already there and you chose to skip it")]
    Skipped,
    #[error("no free name beside the folder that is already there")]
    NoFreeName,
    #[error("the place is not one PCTwin can name: {0}")]
    BadPlace(String),
    #[error(transparent)]
    Gate(#[from] GateError),
    #[error(transparent)]
    Journal(#[from] JournalError),
    #[error("the captured copy could not be read: {0}")]
    Capture(String),
    #[error("stopped as asked; what landed so far is recorded")]
    Stopped { effect: u64 },
}

fn check_import(
    grant: &CaptureGrant,
    seal: PlanSeal,
    root: RootRef,
    now: Now,
) -> Result<(), LandError> {
    if grant.effect() != CaptureEffect::Import {
        return Err(LandError::Denied("it allows another kind of work"));
    }
    if grant.seal() != seal {
        return Err(LandError::Denied("it belongs to another plan"));
    }
    if !grant.roots().contains(&root) {
        return Err(LandError::Denied("it does not cover this folder"));
    }
    if grant.expiry().is_over(now) {
        return Err(LandError::Denied("it has ended"));
    }
    Ok(())
}

/// The folder name to land in: the target's, or a new one beside it.
fn choose(
    dest: &Destination,
    target: &Target,
    conflict: Conflict,
) -> Result<(String, bool), LandError> {
    if let Some(m) = &target.made {
        return Ok((m.folder.clone(), m.beside));
    }
    let (parent, top) = match target.folder.rsplit_once('/') {
        Some((p, t)) => (format!("{p}/"), t.to_string()),
        None => (String::new(), target.folder.clone()),
    };
    let taken = |name: &str| -> Result<bool, LandError> {
        let p = IncomingPath::parse(&format!("{parent}{name}"))
            .map_err(|e| LandError::BadPlace(e.to_string()))?;
        Ok(dest.meet(&p).taken)
    };
    if !taken(&top)? {
        return Ok((format!("{parent}{top}"), false));
    }
    if conflict == Conflict::Skip {
        return Err(LandError::Skipped);
    }
    for n in 1..=BESIDE_TRIES {
        let name = if n == 1 {
            format!("{top} (from old laptop)")
        } else {
            format!("{top} (from old laptop {n})")
        };
        if !taken(&name)? {
            return Ok((format!("{parent}{name}"), true));
        }
    }
    Err(LandError::NoFreeName)
}

fn place_of(target: &Target, folder: &str) -> Result<SettingsPlace, LandError> {
    Ok(SettingsPlace {
        root: target.root,
        path: RelPath::new(folder).map_err(|e| LandError::BadPlace(e.to_string()))?,
    })
}

/// Lands the ordinary component `capture` (its files in `work`) under `target` in `dest`, the
/// approved folder `root`, with the import grant of plan `seal`.
#[allow(clippy::too_many_arguments)]
pub fn land(
    rec: &Recorder<'_, Imports>,
    grant: CaptureGrant,
    seal: PlanSeal,
    root: RootRef,
    planned: &PlannedEffect,
    dest: &Destination,
    target: &Target,
    capture: &Capture,
    work: &Path,
    conflict: Conflict,
    stop: &Stop,
    now: Now,
) -> Result<Landed, LandError> {
    check_import(&grant, seal, root, now)?;
    let (folder, beside) = choose(dest, target, conflict)?;
    write(
        rec, planned, dest, target, &folder, beside, capture, work, stop, None,
    )
}

/// Lands a secret component the person chose, with a secret grant naming exactly it, into a new
/// owner-only folder. `dest_path` is the approved folder's own path (where the private folder is
/// made).
#[allow(clippy::too_many_arguments)]
pub fn land_secret(
    rec: &Recorder<'_, Imports>,
    grant: SecretGrant,
    approved: &Approved,
    planned: &PlannedEffect,
    dest: &Destination,
    dest_path: &Path,
    target: &Target,
    capture: &Capture,
    work: &Path,
    owner_only: &dyn OwnerOnly,
    stop: &Stop,
    now: Now,
) -> Result<Landed, LandError> {
    if grant.effect() != SecretEffect::Land {
        return Err(LandError::Denied("it allows another kind of work"));
    }
    if grant.seal() != approved.seal || !grant.components().contains(&approved.object()) {
        return Err(LandError::Denied(
            "it does not name this secret as approved",
        ));
    }
    if grant.expiry().is_over(now) {
        return Err(LandError::Denied("it has ended"));
    }
    if capture.component != approved.component {
        return Err(LandError::Denied("the capture is of another component"));
    }
    // A secret never joins a folder made earlier (it may not be owner-only): its own private
    // folder, beside one that is already there.
    let fresh = Target {
        made: None,
        ..target.clone()
    };
    let (folder, beside) = choose(dest, &fresh, Conflict::KeepBoth)?;
    // The folder exists before any file: only the person can open it from its first moment.
    let mut parent = dest_path.to_path_buf();
    let parts: Vec<&str> = folder.split('/').collect();
    for p in &parts[..parts.len() - 1] {
        parent = parent.join(p);
    }
    std::fs::create_dir_all(&parent).map_err(|e| LandError::Capture(e.to_string()))?;
    let made = parent.join(parts[parts.len() - 1]);
    owner_only
        .private_folder(&made)
        .map_err(|e| LandError::Capture(e.to_string()))?;
    write(
        rec,
        planned,
        dest,
        target,
        &folder,
        beside,
        capture,
        work,
        stop,
        Some(dest_path),
    )
}

/// Reads the file `rel` of the capture and its block fingerprints.
fn read_captured(work: &Path, rel: &str) -> Result<(Vec<u8>, Vec<[u8; 32]>), LandError> {
    let path = rel
        .split('/')
        .fold(work.to_path_buf(), |p, part| p.join(part));
    let mut bytes = Vec::new();
    std::fs::File::open(&path)
        .and_then(|mut f| f.read_to_end(&mut bytes))
        .map_err(|e| LandError::Capture(e.kind().to_string()))?;
    let hashes = bytes
        .chunks(BLOCK as usize)
        .map(|c| *blake3::hash(c).as_bytes())
        .collect();
    Ok((bytes, hashes))
}

#[allow(clippy::too_many_arguments)]
fn write(
    rec: &Recorder<'_, Imports>,
    planned: &PlannedEffect,
    dest: &Destination,
    target: &Target,
    folder: &str,
    beside: bool,
    capture: &Capture,
    work: &Path,
    stop: &Stop,
    owner_only_files: Option<&Path>,
) -> Result<Landed, LandError> {
    let place = place_of(target, folder)?;
    let effect = rec.intend(
        planned,
        &ImportIntent::Files {
            to: place,
            before: Vec::new(),
        },
    )?;
    let depth = folder.split('/').count();
    // A secret's owner-only folder was made by this landing just before: it is this move's own.
    // (A folder this move made for an earlier component of the same app is its own too.)
    let mut top: Option<String> =
        (owner_only_files.is_some() || target.made.is_some()).then(|| folder.to_string());
    let mut files = capture.files.clone();
    files.sort_by(|a, b| a.rel.cmp(&b.rel));
    for (i, f) in files.iter().enumerate() {
        if stop.stopped() {
            finish(rec, dest, folder, capture, effect, false)?;
            return Err(LandError::Stopped { effect });
        }
        let (bytes, hashes) = read_captured(work, &f.rel)?;
        if *blake3::hash(&bytes).as_bytes() != f.hash {
            finish(rec, dest, folder, capture, effect, false)?;
            return Err(LandError::Capture("the captured copy changed".into()));
        }
        let path = IncomingPath::parse(&format!("{folder}/{}", f.rel))
            .map_err(|e| LandError::BadPlace(e.to_string()))?;
        let ours_name = top.clone();
        let ours = move |stored: &str| ours_name.as_deref() == Some(stored);
        let fresh = Fresh { depth, ours: &ours };
        let tag = format!("s25-{effect}-{i}");
        let result = (|| -> Result<pctwin_gate::Finished, GateError> {
            let mut file =
                dest.create_file_tagged_fresh(&path, bytes.len() as u64, &tag, Some(fresh))?;
            std::io::Write::write_all(&mut file, &bytes).map_err(GateError::Io)?;
            let check = file.verify(BLOCK, &hashes)?;
            if !check.passed() {
                return Err(GateError::Io(std::io::Error::other(
                    "the copy did not read back as sent",
                )));
            }
            file.finish()
        })();
        let done = match result {
            Ok(d) => d,
            Err(e) => {
                finish(rec, dest, folder, capture, effect, false)?;
                return Err(LandError::Gate(e));
            }
        };
        if top.is_none() {
            top = Some(
                done.final_path
                    .split('/')
                    .take(depth)
                    .collect::<Vec<_>>()
                    .join("/"),
            );
        }
        #[cfg(unix)]
        if let Some(base) = owner_only_files {
            use std::os::unix::fs::PermissionsExt;
            let p = done
                .final_path
                .split('/')
                .fold(base.to_path_buf(), |p, part| p.join(part));
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600))
                .map_err(|e| LandError::Capture(e.to_string()))?;
        }
        #[cfg(not(unix))]
        let _ = owner_only_files;
    }
    let verified = finish(rec, dest, folder, capture, effect, true)?;
    Ok(Landed {
        effect,
        folder: top.unwrap_or_else(|| folder.to_string()),
        beside,
        verified,
    })
}

/// What is at `folder` now, for each captured file: its path inside the folder and content hash.
fn look(dest: &Destination, folder: &str, capture: &Capture) -> Vec<FileHash> {
    let mut out = Vec::new();
    for f in &capture.files {
        // Found as the gate stores it (names converted exactly as for writing).
        let Some((stored, _)) = IncomingPath::parse(&format!("{folder}/{}", f.rel))
            .ok()
            .and_then(|p| dest.find(&p))
        else {
            continue;
        };
        if let Ok(mut file) = dest.open_read(&stored) {
            let mut bytes = Vec::new();
            if file.read_to_end(&mut bytes).is_ok() {
                out.push(FileHash {
                    path: f.rel.clone(),
                    hash: pctwin_journal::content_hash(&bytes),
                });
            }
        }
    }
    out
}

/// Records what is there and the honest receipt; returns whether every file matched.
fn finish(
    rec: &Recorder<'_, Imports>,
    dest: &Destination,
    folder: &str,
    capture: &Capture,
    effect: u64,
    completed: bool,
) -> Result<bool, LandError> {
    let found = look(dest, folder, capture);
    rec.observed(effect, &Found::Files(found.clone()), false)?;
    let all = matches_capture(&found, capture);
    let receipt = if completed && all {
        Receipt::Done
    } else {
        Receipt::Failed {
            reason: Reason::NotVerified,
        }
    };
    rec.receipt(effect, &receipt)?;
    Ok(completed && all)
}

fn matches_capture(found: &[FileHash], capture: &Capture) -> bool {
    capture.files.len() == found.len()
        && capture
            .files
            .iter()
            .all(|f| found.iter().any(|h| h.path == f.rel && h.hash == f.hash))
}

/// How a landing interrupted by a crash ended, found by looking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reconciled {
    /// Every captured file is there and matches.
    Complete,
    /// Some are missing or differ: the effect ends failed; undo removes what PCTwin placed.
    Partial,
    /// The effect was already finished.
    AlreadyFinished,
}

/// After a restart: finishes the record of landing `effect` from what is really at its place now
/// (never by writing again), and removes PCTwin's own unfinished temporary files there.
pub fn reconcile(
    rec: &Recorder<'_, Imports>,
    dest: &Destination,
    effect: u64,
    capture: &Capture,
) -> Result<Reconciled, LandError> {
    let entry = rec
        .journal()
        .effect(effect)?
        .ok_or(LandError::BadPlace("no such effect".into()))?;
    let StoredEffect::Import { to, .. } = &entry.effect else {
        return Err(LandError::Denied("not an import"));
    };
    let folder = to.path.parts().collect::<Vec<_>>().join("/");
    let mut files = capture.files.clone();
    files.sort_by(|a, b| a.rel.cmp(&b.rel));
    for (i, f) in files.iter().enumerate() {
        let dir = f
            .rel
            .rsplit_once('/')
            .map_or(String::new(), |(d, _)| format!("/{d}"));
        let temp = format!(
            "{folder}{dir}/{}",
            pctwin_gate::temp_name(&format!("s25-{effect}-{i}"))
        );
        let _ = dest.remove_temp(&temp);
    }
    match &entry.state {
        EffectState::Receipted { .. } => Ok(Reconciled::AlreadyFinished),
        EffectState::Observed { found, .. } => {
            let all = matches!(found, Found::Files(f) if matches_capture(f, capture));
            rec.receipt(
                effect,
                &if all {
                    Receipt::Done
                } else {
                    Receipt::Failed {
                        reason: Reason::NotVerified,
                    }
                },
            )?;
            Ok(if all {
                Reconciled::Complete
            } else {
                Reconciled::Partial
            })
        }
        EffectState::Intended => {
            let found = look(dest, &folder, capture);
            let all = matches_capture(&found, capture);
            rec.observed(effect, &Found::Files(found), true)?;
            rec.receipt(
                effect,
                &if all {
                    Receipt::Done
                } else {
                    Receipt::Failed {
                        reason: Reason::NotVerified,
                    }
                },
            )?;
            Ok(if all {
                Reconciled::Complete
            } else {
                Reconciled::Partial
            })
        }
    }
}

/// How undoing a landing ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Undone {
    /// Every file PCTwin placed is gone; nothing was there before (it landed in a new folder).
    Removed,
    /// Files changed since the move (the person's later work) are kept; the rest are gone.
    KeptChanged { kept: usize },
    /// Not done this time, and why; asking again tries again.
    NotDone(String),
    /// Already undone.
    AlreadyUndone,
}

/// Undoes landing `effect` (T25-054): each file PCTwin placed is removed only while it is still
/// exactly what PCTwin wrote (compare before removing, through the gate); a file the person changed
/// since is kept, and they are told. Folders are removed only when empty. Recorded in the journal
/// (part way first, then how it ended), with leave from its undo gate. Windows first: the gate's
/// checked removal is built for Windows; elsewhere it is "not done this time".
pub fn undo_landing(
    rec: &Recorder<'_, Imports>,
    right: &impl pctwin_journal::UndoRight,
    dest: &Destination,
    effect: u64,
) -> Result<Undone, LandError> {
    use pctwin_journal::{EffectUndo, EffectUndoOutcome};
    let journal = rec.journal();
    if let Some(EffectUndo::Done { outcome }) = journal.effect_undo_of(effect)?
        && outcome.is_final()
    {
        return Ok(Undone::AlreadyUndone);
    }
    let entry = journal
        .effect(effect)?
        .ok_or(LandError::BadPlace("no such effect".into()))?;
    let StoredEffect::Import { to, .. } = &entry.effect else {
        return Err(LandError::Denied("not an import"));
    };
    let EffectState::Receipted {
        found: Found::Files(placed),
        ..
    } = &entry.state
    else {
        return Ok(Undone::NotDone(
            "the landing is not finished; reconcile it first".into(),
        ));
    };
    let folder = to.path.parts().collect::<Vec<_>>().join("/");
    if !matches!(journal.effect_undo_of(effect)?, Some(EffectUndo::Reversing)) {
        rec.record_undo(right, effect, &EffectUndo::Reversing)?;
    }
    let result = remove_placed(right, dest, &folder, placed);
    let (undone, outcome) = match result {
        Ok(0) => (Undone::Removed, EffectUndoOutcome::Restored),
        Ok(kept) => (
            Undone::KeptChanged { kept },
            EffectUndoOutcome::Kept {
                why: format!("{kept} file(s) changed since the move were kept"),
            },
        ),
        Err(why) => (
            Undone::NotDone(why.clone()),
            EffectUndoOutcome::NotDone { why },
        ),
    };
    rec.record_undo(right, effect, &EffectUndo::Done { outcome })?;
    Ok(undone)
}

/// Removes the placed files still unchanged; returns how many were kept.
#[cfg(windows)]
fn remove_placed(
    right: &impl pctwin_journal::UndoRight,
    dest: &Destination,
    folder: &str,
    placed: &[pctwin_journal::FileHash],
) -> Result<usize, String> {
    use pctwin_gate::Removed;
    let mut kept = 0usize;
    let mut folders = std::collections::BTreeSet::new();
    for h in placed {
        let Some((stored, _)) = IncomingPath::parse(&format!("{folder}/{}", h.path))
            .ok()
            .and_then(|p| dest.find(&p))
        else {
            continue;
        };
        let Some(stat) = dest.stat(&stored).map_err(|e| e.kind().to_string())? else {
            continue;
        };
        let want = h.hash;
        let verify = |f: &mut std::fs::File| -> std::io::Result<bool> {
            let mut bytes = Vec::new();
            f.read_to_end(&mut bytes)?;
            Ok(pctwin_journal::content_hash(&bytes) == want)
        };
        match dest
            .remove_if_unchanged(right, &stored, stat.id, verify, || Ok(()))
            .map_err(|e| e.to_string())?
        {
            Removed::Removed | Removed::Gone => {}
            _ => kept += 1,
        }
        let mut parts: Vec<&str> = stored.split('/').collect();
        while parts.len() > 1 {
            parts.pop();
            folders.insert(parts.join("/"));
        }
    }
    // Deepest first; only empty folders go.
    let mut folders: Vec<String> = folders.into_iter().collect();
    folders.sort_by_key(|f| std::cmp::Reverse(f.matches('/').count()));
    for f in folders {
        if f.matches('/').count() + 1 >= folder.split('/').count() {
            let _ = dest.remove_empty_folder(&f);
        }
    }
    Ok(kept)
}

#[cfg(not(windows))]
fn remove_placed(
    _right: &impl pctwin_journal::UndoRight,
    _dest: &Destination,
    _folder: &str,
    _placed: &[pctwin_journal::FileHash],
) -> Result<usize, String> {
    Err("undo of brought-in app data is built for Windows first".into())
}
