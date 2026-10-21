//! What the mover is shown at the end of a move (Product Spec, outcome and report; Security
//! Design part C, decided 21 October 2026): their own files and the shared ones line by line, each
//! with its honest result; every other person as one line of counts. Another person's files pass
//! through PCTwin's engine but are never named here, so nothing built from this report (a screen,
//! a saved report, a problem code) can show them.

use std::collections::{BTreeMap, BTreeSet};

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
    /// Of those that failed, how many the person had open ("in use by Bea", with Try again).
    pub in_use: u64,
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
                    FileResult::Failed => {
                        line.failed += 1;
                        if matches!(why, Some((Cause::InUse, _))) {
                            line.in_use += 1;
                        }
                    }
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

/// The files a "Try again" can bring: those another program had open, or that kept changing
/// while they were read (closing the program is often enough).
pub fn to_try_again(received: &Received) -> BTreeSet<ItemId> {
    received
        .files
        .iter()
        .filter(|f| {
            matches!(
                &f.outcome,
                Some(ReceiveOutcome::Failed(why))
                    if matches!(why.cause(), Cause::InUse | Cause::ChangedDuringMove)
            )
        })
        .map(|f| f.item)
        .collect()
}

/// The first move's report with the files `tried` again given their new outcome: what the
/// person sees after "Try again". A file the second move did not reach keeps its first outcome.
pub fn tried_again(first: &Received, again: &Received, tried: &BTreeSet<ItemId>) -> Received {
    let mut merged = first.clone();
    for line in &mut merged.files {
        if !tried.contains(&line.item) {
            continue;
        }
        if let Some(new) = again.files.iter().find(|f| f.item == line.item)
            && new.outcome.is_some()
        {
            *line = new.clone();
        }
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Ended, ReceivedFile};
    use pctwin_record::{
        FolderRole, Inclusion, ItemKind, ItemName, ItemPath, LaptopId, ManagedBy, Place,
        Portability, Record, Storage,
    };
    use pctwin_transfer::Failure;

    fn item(owner: &str, name: &str) -> Item {
        let laptop = LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap();
        let owner = Owner::Person {
            account_id: owner.into(),
        };
        let place = Place {
            role: FolderRole::Documents,
            storage: Storage::SystemDrive,
        };
        let path = ItemPath::new(vec![ItemName::from_text(name)]).unwrap();
        Item {
            id: ItemId::derive(&laptop, &owner, &place, &path),
            kind: ItemKind::File,
            owner,
            place,
            path,
            size_bytes: 10,
            modified_ns: None,
            portability: Portability::Portable,
            managed_by: ManagedBy::Personal,
            download_bytes: None,
            inclusion: Inclusion::Included,
        }
    }

    #[test]
    fn another_person_s_open_files_are_counted_as_in_use_never_named() {
        let laptop = LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap();
        let mut record = Record::new(laptop);
        let (open, gone, mine) = (
            item("1001", "bea-open.txt"),
            item("1001", "bea-gone.txt"),
            item("1000", "ada.txt"),
        );
        record.items = vec![open.clone(), gone.clone(), mine.clone()];
        let failed = |cause, why: &str| Some(ReceiveOutcome::Failed(Failure::new(cause, why)));
        let received = Received {
            record,
            files: vec![
                ReceivedFile {
                    item: open.id,
                    outcome: failed(Cause::InUse, "another program has it open"),
                    changed_since_scan: false,
                },
                ReceivedFile {
                    item: gone.id,
                    outcome: failed(Cause::NoLongerThere, "it was removed"),
                    changed_since_scan: false,
                },
                ReceivedFile {
                    item: mine.id,
                    outcome: failed(Cause::InUse, "another program has it open"),
                    changed_since_scan: false,
                },
            ],
            ended: Ended::Finished,
        };
        let report = for_the_mover(&received, "1000");
        let bea = &report.people["1001"];
        assert_eq!((bea.failed, bea.in_use, bea.copied), (2, 1, 0));
        assert_eq!(report.own.len(), 1);
        assert_eq!(report.own[0].why.as_ref().map(|w| w.0), Some(Cause::InUse));
        assert!(!format!("{report:?}").contains("bea-"));
        // Both open files can be tried again; the removed one cannot.
        assert_eq!(to_try_again(&received), BTreeSet::from([open.id, mine.id]));
    }
}
