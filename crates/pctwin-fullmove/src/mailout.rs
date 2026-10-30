//! Apple Mail on the new laptop (Task List 2.5, email; design of 30 October 2026).
//!
//! The old laptop sent Apple Mail's raw files, carried as they were ([`crate::extras`]). Here,
//! after they arrived checked in PCTwin's own staging folder ([`crate::data::Sink`]):
//!
//! 1. the received mail index is read in the parser process ([`MailOut::kinds`]); only
//!    `(account folder, kind of address)` pairs come back, matched to the received folders by
//!    exact equality, never used as a path;
//! 2. each account folder is kept only on the old Mac or left to its server
//!    ([`pctwin_special::applemail::local_only`]: every doubt keeps mail);
//! 3. the mail kept only on the old Mac is written as one mbox file per mailbox
//!    ([`pctwin_special::mail::export_from`], each message read in the parser process), each
//!    file read back and its messages counted against the number written;
//! 4. the files land through the safety gate and the imports journal, with undo, in the
//!    person's Documents folder, in a new folder named for the old laptop (beside one already
//!    there, never over it);
//! 5. only then is the staged mail removed.
//!
//! Every mailbox ends with its own honest result: `converted` with how many messages are ready
//! (the import stays the person's step, never claimed done); needs-action with what is missing
//! counted; an account whose mail stays on its server is "sign in on the new laptop", never
//! "copied".

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use pctwin_grant::{PlanSeal, RootRef};
use pctwin_special::api::MadeFolders;
use pctwin_special::applemail::{self, AccountScheme, Words};
use pctwin_special::capture::{Capture, CapturedFile, Method, Stop, generation};
use pctwin_special::component::{Component, Custody, Evidence, Preservation};
use pctwin_special::content::ContentError;
use pctwin_special::mail::{MailExport, MailSource, Reader};
use pctwin_special::roots::RootKind;
use pctwin_types::{ItemResult, ItemStatus, Reason};

use crate::data::{
    Arrived, DataWhy, Landing, NewLaptopOf, NotCaptured, PlacementWire, Record, Sink, arrived_whole,
};

/// Reads the received mail index (and its log) in the parser process.
pub type KindsReader<'a> =
    &'a dyn Fn(&[u8], Option<&[u8]>) -> Result<Vec<AccountScheme>, ContentError>;

/// The folder inside PCTwin's staging folder the mailbox files are written in first.
const OUT: &str = "mail-out";

/// What the new laptop needs to write Apple Mail out.
pub struct MailOut<'a> {
    /// The plan's line for it (its name, as the catalogue names it).
    pub line: &'a str,
    /// The line's components, and the folder inside `V<n>` each one holds (same order).
    pub ids: &'a [String],
    pub parts: &'a [String],
    /// The person's Documents folder, inside their home (`/` between folders).
    pub documents: &'a str,
    /// The old laptop's name.
    pub old_laptop: &'a str,
    pub words: Words,
    /// Reads one saved message in the parser process.
    pub emlx: Reader<'a>,
    /// Reads the mail index in the parser process.
    pub kinds: KindsReader<'a>,
    pub deadline: Instant,
}

/// One line of how the mail ended.
#[derive(Debug, Clone, PartialEq)]
pub struct MailLine {
    /// A mailbox's name (`On My Mac/Trips`), or the plan line's name.
    pub title: String,
    pub result: ItemResult,
    pub why: Option<DataWhy>,
}

/// How the mail ended.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MailDone {
    pub lines: Vec<MailLine>,
    /// The mailbox files' landing in the journal (for undo).
    pub effect: Option<u64>,
}

/// Why a part that did not arrive whole did not.
fn why_not(a: Option<&Arrived>, lost: bool) -> DataWhy {
    match a.and_then(|a| a.not_captured) {
        Some(NotCaptured::AppOpen) => DataWhy::AppOpen,
        Some(NotCaptured::Changing) => DataWhy::Changing,
        Some(NotCaptured::Unreadable) => DataWhy::Unreadable,
        Some(NotCaptured::Stopped) => DataWhy::NotRead,
        None if a.is_none() && lost => DataWhy::Lost,
        None if a.is_none() => DataWhy::DidNotArrive,
        None => DataWhy::NotWhole,
    }
}

/// The parser's answers about the received index (component `id`, arrived whole), as the engine
/// accepts them: read with its log, and read without it (the database alone). A log that does
/// not belong to its database (copied at another moment, or damaged) could name an account
/// "on a server" that is not: an account is taken as kept on its server only when both readings
/// say so. `None` where there is no answer to use.
type Answers = (Option<Vec<AccountScheme>>, Option<Vec<AccountScheme>>);

fn answers_of(sink: &Sink, id: &str, r: &Record, kinds: KindsReader<'_>) -> Answers {
    let read = |log: bool| answer_of(sink, id, r, kinds, log);
    let has_log = r
        .capture
        .files
        .iter()
        .any(|f| f.rel == applemail::INDEX_LOG);
    let without = read(false);
    let with = if has_log { read(true) } else { without.clone() };
    (with, without)
}

fn answer_of(
    sink: &Sink,
    id: &str,
    r: &Record,
    kinds: KindsReader<'_>,
    with_log: bool,
) -> Option<Vec<AccountScheme>> {
    let has = |rel: &str| r.capture.files.iter().any(|f| f.rel == rel);
    if !has(applemail::INDEX) {
        return None;
    }
    let at = |rel: &str| rel.split('/').fold(sink.folder(id), |p, part| p.join(part));
    let db =
        pctwin_special::guard::read_all(&at(applemail::INDEX), pctwin_special::content::MAX_DB)
            .ok()?;
    let log = if with_log && has(applemail::INDEX_LOG) {
        Some(
            pctwin_special::guard::read_all(
                &at(applemail::INDEX_LOG),
                pctwin_special::content::MAX_LOG,
            )
            .ok()?,
        )
    } else {
        None
    };
    applemail::checked_answer(kinds(&db, log.as_deref()).ok()?)
}

/// Writes Apple Mail out for the person (the line `out` describes) from what arrived in `sink`,
/// and lands it through the safety gate. The staged mail is removed once every file was read
/// back and landed (or could not be).
pub fn convert_mail(sink: &Sink, n: &Landing<'_>, out: &MailOut<'_>) -> MailDone {
    let mut done = MailDone::default();
    let needs = |title: &str, why: DataWhy| MailLine {
        title: title.to_string(),
        result: ItemResult::because(Reason::FinishByHand),
        why: Some(why),
    };
    // What arrived, by the folder each part holds.
    let mut whole: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut not_whole: BTreeMap<&str, DataWhy> = BTreeMap::new();
    let mut index: Option<(&str, &Record)> = None;
    let mut owner = None;
    for (id, part) in out.ids.iter().zip(out.parts) {
        let a = sink.arrived.get(id);
        let rec = a
            .and_then(|a| a.record.as_ref())
            .filter(|r| r.root == RootKind::MacMail && r.component.custody == Custody::NeedsExport);
        let arrived = a
            .zip(rec)
            .is_some_and(|(a, r)| arrived_whole(a, r.capture.files.iter().map(|f| f.rel.as_str())))
            // A part holds only files of the folder the plan says it holds.
            && rec.is_some_and(|r| {
                r.component
                    .members
                    .iter()
                    .all(|m| m.split('/').next() == Some(part.as_str()))
            });
        if let Some(r) = rec {
            owner.get_or_insert_with(|| r.component.owner.clone());
        }
        match rec {
            Some(r) if arrived && r.component.members.iter().any(|m| m == applemail::INDEX) => {
                index = Some((id, r));
            }
            Some(_) if arrived => whole.entry(part).or_default().push(id),
            _ => {
                not_whole
                    .entry(part)
                    .or_insert_with(|| why_not(a, sink.lost));
            }
        }
    }
    let (with_log, without_log) = index
        .map(|(id, r)| answers_of(sink, id, r, out.kinds))
        .unwrap_or_default();
    let kinds = applemail::shipped_kinds();
    // Every folder of mail named in the plan (the index's own folder is not mail).
    let index_part = applemail::INDEX.split('/').next().unwrap_or_default();
    let tops: BTreeSet<&str> = out
        .parts
        .iter()
        .map(String::as_str)
        .filter(|p| *p != index_part)
        .collect();
    let mut server = 0u64;
    let mut sources = Vec::new();
    let mut accounts = 0usize;
    for top in &tops {
        let label = if *top == applemail::ON_THIS_MAC {
            out.words.on_this_mac.clone()
        } else {
            accounts += 1;
            out.words.account_for(accounts)
        };
        let local = applemail::local_only(top, with_log.as_deref(), &kinds)
            || applemail::local_only(top, without_log.as_deref(), &kinds);
        if !local {
            // Its mail is on its server: it comes back by signing in, whether or not it arrived.
            server += 1;
            continue;
        }
        if let Some(why) = not_whole.get(top) {
            // Some of it did not arrive: none of it is written, so nothing is said written whole.
            done.lines.push(needs(&label, why.clone()));
            continue;
        }
        for id in whole.get(top).into_iter().flatten() {
            sources.push(MailSource {
                dir: sink.folder(id).join(top),
                label: label.clone(),
            });
        }
    }
    if server > 0 {
        done.lines.push(MailLine {
            title: out.line.to_string(),
            result: ItemResult::because(Reason::SignIn),
            why: Some(DataWhy::MailSignIn { accounts: server }),
        });
    }
    let staged = sink.staging().join(OUT);
    let clean_up = || {
        let _ = std::fs::remove_dir_all(&staged);
        for id in out.ids {
            let _ = std::fs::remove_dir_all(sink.folder(id));
        }
    };
    if !sources.is_empty() {
        let labels: BTreeSet<String> = sources.iter().map(|s| s.label.clone()).collect();
        let written = std::fs::create_dir_all(sink.staging())
            .and_then(|()| std::fs::create_dir(&staged))
            .and_then(|()| {
                pctwin_special::mail::export_from(&sources, &staged, out.emlx, out.deadline)
            });
        match written {
            Ok(export) => {
                let landed = land_files(sink, n, out, &export, &staged, owner);
                done.effect = landed.as_ref().map(|(e, _)| *e);
                lines_of(&export, landed.map(|(_, f)| f), &mut done.lines);
                if export.not_all_looked_at {
                    done.lines.push(needs(out.line, DataWhy::NotWhole));
                }
            }
            Err(_) => {
                for l in labels {
                    done.lines.push(needs(&l, DataWhy::NotLanded));
                }
            }
        }
    }
    clean_up();
    if done.lines.is_empty() {
        // Nothing kept only on the old Mac and no account: nothing for the person to import.
        done.lines.push(MailLine {
            title: out.line.to_string(),
            result: ItemResult::done(ItemStatus::Converted)
                .unwrap_or(ItemResult::because(Reason::NotVerified)),
            why: None,
        });
    }
    done
}

/// Each mailbox's line, by what was written and whether its file landed (in `folder`).
fn lines_of(export: &MailExport, folder: Option<String>, lines: &mut Vec<MailLine>) {
    for b in &export.mailboxes {
        let messages = u64::from(b.written + b.partial);
        let written = b.result();
        let (result, why) = match &folder {
            _ if messages == 0 => (written, Some(DataWhy::Unreadable)),
            // Its file did not read back as written: nothing in it is vouched for.
            _ if !b.checked => (written, Some(DataWhy::NotLanded)),
            None => (
                ItemResult::because(Reason::FinishByHand),
                Some(DataWhy::NotLanded),
            ),
            Some(folder) if written.status() == ItemStatus::Converted => (
                written,
                Some(DataWhy::MailReady {
                    messages,
                    folder: folder.clone(),
                }),
            ),
            Some(folder) => (
                written,
                Some(DataWhy::MailPartsMissing {
                    messages,
                    missing: u64::from(b.partial),
                    failed: u64::from(b.failed),
                    folder: folder.clone(),
                }),
            ),
        };
        lines.push(MailLine {
            title: b.name.clone(),
            result,
            why,
        });
    }
}

/// Lands the mailbox files written in `staged` through the safety gate (a new folder in the
/// person's Documents, named for the old laptop); its journal effect and the folder it landed
/// in (inside the home), or `None` when it did not land read back whole.
fn land_files(
    sink: &Sink,
    n: &Landing<'_>,
    out: &MailOut<'_>,
    export: &MailExport,
    staged: &Path,
    owner: Option<pctwin_record::Owner>,
) -> Option<(u64, String)> {
    let owner = owner?;
    let members: Vec<String> = export
        .mailboxes
        .iter()
        .filter_map(|b| b.file.as_deref())
        .filter_map(|f| f.file_name()?.to_str().map(str::to_string))
        .collect();
    if members.is_empty() {
        return None;
    }
    let mut files = Vec::new();
    for m in &members {
        files.push(hashed(&staged.join(m), m).ok()?);
    }
    let component = Component::new(
        owner,
        "Mail",
        members.clone(),
        Custody::NeedsExport,
        false,
        vec![Evidence::MailStore],
    );
    let folder = format!(
        "{}/{}",
        out.documents.trim_matches('/'),
        out.words.folder_for(out.old_laptop)
    );
    let record = Record {
        app: out.line.to_string(),
        capture: Capture {
            component: component.id,
            method: Method::SteadyCopy,
            preservation: Preservation::Captured,
            generation: generation(staged, &members),
            files,
            folded: Vec::new(),
            evidence: Vec::new(),
            closed_unconfirmed: false,
        },
        component,
        placement: PlacementWire::Same {
            root: RootKind::HomeDot,
            folder,
        },
        root: RootKind::HomeDot,
    };
    let rec = n.journal.imports(n.key);
    let stop = Stop::new();
    let made = MadeFolders::default();
    let seal = PlanSeal::new(
        3,
        *blake3::Hasher::new_derive_key("PCTwin mail landing 2026-10-30 v1")
            .update(&sink.choice.encode())
            .finalize()
            .as_bytes(),
    );
    let laptop = NewLaptopOf {
        n,
        rec: &rec,
        stop: &stop,
        made: &made,
        seal,
        root: RootRef::new(3),
    };
    let id = crate::data::id_hex(&record.component);
    let applied = laptop.applied(&id, &record, 0, false, staged)?;
    let l = applied.landed?;
    l.verified.then_some((l.effect, l.folder))
}

/// One written file's size and BLAKE3 hash, as a capture names it.
fn hashed(path: &PathBuf, rel: &str) -> std::io::Result<CapturedFile> {
    let mut h = blake3::Hasher::new();
    let mut f = std::fs::File::open(path)?;
    let len = std::io::copy(&mut f, &mut h)?;
    Ok(CapturedFile {
        rel: rel.to_string(),
        len,
        hash: *h.finalize().as_bytes(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pctwin_special::mail::MailboxExport;

    fn export(written: u32, partial: u32, failed: u32, checked: bool) -> MailExport {
        MailExport {
            mailboxes: vec![MailboxExport {
                name: "On My Mac/Trips".into(),
                file: None,
                written,
                partial,
                failed,
                checked,
            }],
            not_all_looked_at: false,
        }
    }

    /// Task List 2.5: every mailbox ends with one of the six words, `converted` only when every
    /// message was written whole, read back, and landed; never "copied"; the import is never
    /// claimed.
    #[test]
    fn every_mailbox_ends_with_an_honest_word() {
        let landed = Some("Documents/Mail from Ada".to_string());
        let f = "Documents/Mail from Ada".to_string();
        type Row = ((u32, u32, u32, bool), Option<String>, ItemStatus, DataWhy);
        let table: Vec<Row> = vec![
            (
                (3, 0, 0, true),
                landed.clone(),
                ItemStatus::Converted,
                DataWhy::MailReady {
                    messages: 3,
                    folder: f.clone(),
                },
            ),
            (
                (3, 1, 0, true),
                landed.clone(),
                ItemStatus::NeedsAction,
                DataWhy::MailPartsMissing {
                    messages: 4,
                    missing: 1,
                    failed: 0,
                    folder: f.clone(),
                },
            ),
            (
                (3, 0, 2, true),
                landed.clone(),
                ItemStatus::NeedsAction,
                DataWhy::MailPartsMissing {
                    messages: 3,
                    missing: 0,
                    failed: 2,
                    folder: f.clone(),
                },
            ),
            (
                (3, 0, 0, false),
                landed.clone(),
                ItemStatus::Failed,
                DataWhy::NotLanded,
            ),
            (
                (0, 0, 2, true),
                landed.clone(),
                ItemStatus::Failed,
                DataWhy::Unreadable,
            ),
            (
                (3, 0, 0, true),
                None,
                ItemStatus::NeedsAction,
                DataWhy::NotLanded,
            ),
        ];
        for ((w, p, x, c), folder, status, why) in table {
            let mut lines = Vec::new();
            lines_of(&export(w, p, x, c), folder.clone(), &mut lines);
            assert_eq!(lines.len(), 1);
            let l = &lines[0];
            assert_eq!(l.result.status(), status, "{w} {p} {x} {c} {folder:?}");
            assert_eq!(l.why.as_ref(), Some(&why), "{w} {p} {x} {c} {folder:?}");
            assert_ne!(l.result.status(), ItemStatus::Copied);
        }
    }

    #[test]
    fn a_part_that_did_not_arrive_says_why() {
        let a = |n: Option<NotCaptured>| Arrived {
            not_captured: n,
            ..Arrived::default()
        };
        assert_eq!(why_not(None, true), DataWhy::Lost);
        assert_eq!(why_not(None, false), DataWhy::DidNotArrive);
        assert_eq!(why_not(Some(&a(None)), true), DataWhy::NotWhole);
        assert_eq!(
            why_not(Some(&a(Some(NotCaptured::Changing))), false),
            DataWhy::Changing
        );
        assert_eq!(
            why_not(Some(&a(Some(NotCaptured::AppOpen))), false),
            DataWhy::AppOpen
        );
        assert_eq!(
            why_not(Some(&a(Some(NotCaptured::Unreadable))), false),
            DataWhy::Unreadable
        );
        assert_eq!(
            why_not(Some(&a(Some(NotCaptured::Stopped))), false),
            DataWhy::NotRead
        );
    }
}
