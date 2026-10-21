//! What the mover is shown at the end of a move (Product Spec, outcome and report; Security
//! Design part C, decided 21 October 2026): their own files and the shared ones line by line, each
//! with its honest result; every other person as one line of counts. Another person's files pass
//! through PCTwin's engine but are never named here, so nothing built from this report (a screen,
//! a saved report, a problem code) can show them.

use std::collections::BTreeMap;

use pctwin_record::{Item, ItemId, Owner};
use pctwin_transfer::{Cause, ReceiveOutcome, shown_path};

use crate::Received;

/// The honest result of one file (every item ends one of these; a move copies, so reinstalled
/// and converted come with apps and settings).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileResult {
    /// On the new laptop, checked (or an identical file was already there).
    Copied,
    /// On the new laptop but not finished: the report says what to do.
    NeedsAction,
    /// Not on the new laptop.
    Failed,
}

/// One of the mover's own files, or a shared one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnLine {
    pub item: ItemId,
    /// Where it was, as the person knows it (`Letters/ada.txt`).
    pub path: String,
    pub result: FileResult,
    /// Why, for a file not copied: its typed cause and the words.
    pub why: Option<(Cause, String)>,
    /// It was sent as it is now, not as it was at the scan.
    pub changed_since_scan: bool,
}

/// Another person: how their files ended, counted, never named.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PersonLine {
    pub copied: u64,
    pub needs_action: u64,
    pub failed: u64,
    /// Bytes of theirs that are on the new laptop.
    pub bytes_copied: u64,
}

/// The mover's report.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MoverReport {
    pub own: Vec<OwnLine>,
    /// Every other person whose things were in the move, by their account ID on the old laptop.
    pub people: BTreeMap<String, PersonLine>,
}

fn result_of(outcome: Option<&ReceiveOutcome>) -> (FileResult, Option<(Cause, String)>) {
    match outcome {
        Some(ReceiveOutcome::Finished(_) | ReceiveOutcome::AlreadyThere(_)) => {
            (FileResult::Copied, None)
        }
        Some(ReceiveOutcome::NeedsAction { why, .. }) => (
            FileResult::NeedsAction,
            Some((Cause::NotVerified, why.clone())),
        ),
        Some(ReceiveOutcome::Failed(f)) => {
            (FileResult::Failed, Some((f.cause(), f.why().to_string())))
        }
        // Never a blank: a file with no outcome did not arrive.
        None => (
            FileResult::Failed,
            Some((Cause::NotAccepted, "it did not arrive".into())),
        ),
    }
}

/// The report for `mover` (their account ID on the old laptop) from what the new laptop
/// received.
pub fn for_the_mover(received: &Received, mover: &str) -> MoverReport {
    let items: BTreeMap<ItemId, &Item> = received.record.items.iter().map(|i| (i.id, i)).collect();
    let mut report = MoverReport::default();
    for file in &received.files {
        let Some(item) = items.get(&file.item) else {
            continue;
        };
        let (result, why) = result_of(file.outcome.as_ref());
        match &item.owner {
            Owner::Person { account_id } if account_id != mover => {
                let line = report.people.entry(account_id.clone()).or_default();
                match result {
                    FileResult::Copied => {
                        line.copied += 1;
                        line.bytes_copied += item.size_bytes;
                    }
                    FileResult::NeedsAction => line.needs_action += 1,
                    FileResult::Failed => line.failed += 1,
                }
            }
            _ => report.own.push(OwnLine {
                item: item.id,
                path: shown_path(&item.path),
                result,
                why,
                changed_since_scan: file.changed_since_scan,
            }),
        }
    }
    report
}
