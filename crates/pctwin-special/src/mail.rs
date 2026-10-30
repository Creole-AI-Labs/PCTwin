//! Mail in a standard format (Task List 2.5, email; plan section 6): saved messages in the Apple
//! Mail format (`emlx`) written out as mbox files, the format Apple Mail's own "Import
//! Mailboxes" and other mail apps take, for a move to a laptop where the Apple Mail folder itself
//! cannot be used (another system).
//!
//! - **Read only in the parser process** ([`emlx_facts`], it takes an [`InChild`]): a message
//!   file's bytes are handed over, bounded ([`MAX_MESSAGE`]); the parser answers only facts:
//!   where the message starts and how long it is (the file's first line says it), which of its
//!   lines must be marked so they are not read as the start of the next message, and whether
//!   its attachments were kept outside the file (Apple Mail keeps a large one apart and marks
//!   its part with an `X-Apple-Content-Length` header).
//! - **Written here, without reading** ([`export_mailboxes`]): the message's own bytes are
//!   copied as they are, a `>` put in front of each line the parser named, after a standard
//!   envelope line. The parser's facts are checked against the file before use; facts that do
//!   not fit end the message "could not be written", never a guess.
//! - **Honest results** ([`MailboxExport::result`]): a mailbox whose every message was written
//!   whole is `converted`; one with messages whose attachments were not on the laptop, or with
//!   messages that could not be written, needs the person (they are counted); one with nothing
//!   written failed. The original files are never changed or removed; nothing is sent.
//!
//! Which folder holds the person's own mailboxes (those kept only on the laptop) is the
//! caller's: a signed catalogue row names it. A mailbox is the nearest folder above a message
//! whose name ends `.mbox` (Apple Mail's own layout), else the folder the message is in.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use pctwin_parse_seal::InChild;
use pctwin_types::{ItemResult, ItemStatus, Reason};
use serde::{Deserialize, Serialize};

use crate::content::ContentError;
use crate::guard::{self, Kind};

/// The largest message file read.
pub const MAX_MESSAGE: u64 = 64 * 1024 * 1024;
/// The most lines one message may need marked.
pub const MAX_MARKED: usize = 100_000;
/// The most message files looked at under one folder.
pub const MAX_MESSAGES: usize = 500_000;
/// The deepest folder looked into.
const MAX_DEPTH: usize = 16;
/// The envelope line put before each message (the sender and date stay in its own headers).
const ENVELOPE: &[u8] = b"From - Thu Jan  1 00:00:00 1970\n";

/// What the parser read in one saved message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmlxFacts {
    /// Where the message starts in the file.
    pub start: u64,
    /// The message's length.
    pub len: u64,
    /// Where each line that must be marked starts (in the file, increasing).
    pub marked: Vec<u64>,
    /// Some of its attachments were kept outside the file: they are not in what is written.
    pub partial: bool,
}

/// Reads one saved message's facts, in the parser process (it takes an [`InChild`]).
pub fn emlx_facts(_: InChild, bytes: &[u8]) -> Result<EmlxFacts, ContentError> {
    if bytes.len() as u64 > MAX_MESSAGE {
        return Err(ContentError::TooLarge);
    }
    let damaged = |why: &str| ContentError::Damaged(why.into());
    let nl = bytes
        .iter()
        .take(64)
        .position(|b| *b == b'\n')
        .ok_or_else(|| damaged("its first line does not say its length"))?;
    let first = std::str::from_utf8(&bytes[..nl])
        .map_err(|_| damaged("its first line does not say its length"))?
        .trim();
    if first.len() > 12 || !first.bytes().all(|b| b.is_ascii_digit()) {
        return Err(damaged("its first line does not say its length"));
    }
    let len: u64 = first
        .parse()
        .map_err(|_| damaged("its first line does not say its length"))?;
    let start = nl as u64 + 1;
    let end = start
        .checked_add(len)
        .filter(|e| *e <= bytes.len() as u64)
        .ok_or_else(|| damaged("it is shorter than its first line says"))?;
    if len == 0 {
        return Err(damaged("it holds no message"));
    }
    let message = &bytes[start as usize..end as usize];
    let mut marked = Vec::new();
    let mut partial = false;
    let mut at = 0usize;
    while at < message.len() {
        let line_end = message[at..]
            .iter()
            .position(|b| *b == b'\n')
            .map_or(message.len(), |p| at + p + 1);
        let line = &message[at..line_end];
        // mboxrd: any number of `>` then "From " is marked once more, so readers undo it exactly.
        let quoted = line.iter().take_while(|b| **b == b'>').count();
        if line[quoted..].starts_with(b"From ") {
            if marked.len() == MAX_MARKED {
                return Err(damaged("too many lines to mark"));
            }
            marked.push(start + at as u64);
        }
        if line.len() > 23 && line[..23].eq_ignore_ascii_case(b"X-Apple-Content-Length:") {
            partial = true;
        }
        at = line_end;
    }
    Ok(EmlxFacts {
        start,
        len,
        marked,
        partial,
    })
}

/// Whether `facts` fit a file of `size` bytes: the message inside it, the marks inside the
/// message and increasing. A parser's answer is checked before it is used.
pub fn facts_fit(facts: &EmlxFacts, size: u64) -> bool {
    let Some(end) = facts.start.checked_add(facts.len) else {
        return false;
    };
    end <= size
        && facts.len > 0
        && facts.marked.len() <= MAX_MARKED
        && facts.marked.windows(2).all(|w| w[0] < w[1])
        && facts.marked.iter().all(|m| (facts.start..end).contains(m))
}

/// Writes one message from `bytes` (its file) as an mbox entry into `out`, by `facts` (already
/// checked with [`facts_fit`]): the envelope line, the message with its marked lines marked, and
/// a blank line.
pub fn write_entry(out: &mut impl Write, bytes: &[u8], facts: &EmlxFacts) -> std::io::Result<()> {
    let end = (facts.start + facts.len) as usize;
    out.write_all(ENVELOPE)?;
    let mut from = facts.start as usize;
    for m in &facts.marked {
        let m = *m as usize;
        out.write_all(&bytes[from..m])?;
        out.write_all(b">")?;
        from = m;
    }
    out.write_all(&bytes[from..end])?;
    if !bytes[..end].ends_with(b"\n") {
        out.write_all(b"\n")?;
    }
    out.write_all(b"\n")
}

/// What became of one mailbox.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MailboxExport {
    /// The mailbox's name, as its folders say (`/` between them).
    pub name: String,
    /// The mbox file written, if any message was.
    pub file: Option<PathBuf>,
    /// Messages written whole.
    pub written: u32,
    /// Messages written whose attachments were kept outside the file (not on the laptop).
    pub partial: u32,
    /// Messages that could not be written (unreadable, damaged, too large).
    pub failed: u32,
}

impl MailboxExport {
    /// The mailbox's result word: `converted` only when every message was written whole; some
    /// written is needs-action (what is missing is counted); none written failed.
    pub fn result(&self) -> ItemResult {
        let wrote = self.written + self.partial;
        if wrote == 0 {
            ItemResult::because(Reason::Unreadable)
        } else if self.partial == 0 && self.failed == 0 {
            ItemResult::done(ItemStatus::Converted)
                .unwrap_or(ItemResult::because(Reason::NotVerified))
        } else {
            // Written, but not every message, or not every attachment: the missing ones listed.
            ItemResult::because(Reason::PartsMissing)
        }
    }
}

/// What [`export_mailboxes`] did.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MailExport {
    pub mailboxes: Vec<MailboxExport>,
    /// Not every message was looked at (too many, too deep, or out of time): said, never
    /// "everything".
    pub not_all_looked_at: bool,
}

/// Reads one message's facts in the parser process (the program's installed reader).
pub type Reader<'a> = &'a dyn Fn(&[u8]) -> Result<EmlxFacts, ContentError>;

/// Writes every mailbox of saved messages under `dir` as one mbox file each in `out` (a folder
/// PCTwin made for this, empty), each message read in the parser process by `read`, by
/// `deadline`. Files are never followed through links and never changed.
pub fn export_mailboxes(
    dir: &Path,
    out: &Path,
    read: Reader<'_>,
    deadline: Instant,
) -> std::io::Result<MailExport> {
    let mut found: std::collections::BTreeMap<String, Vec<PathBuf>> = Default::default();
    let mut cut = false;
    let mut count = 0usize;
    let mut stack = vec![(dir.to_path_buf(), 0usize)];
    while let Some((d, depth)) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            cut = true;
            continue;
        };
        for e in entries.flatten() {
            let Ok(m) = std::fs::symlink_metadata(e.path()) else {
                cut = true;
                continue;
            };
            match guard::kind_of(&m) {
                Kind::Folder if depth < MAX_DEPTH => stack.push((e.path(), depth + 1)),
                Kind::Folder => cut = true,
                Kind::File if is_message(&e.path()) => {
                    if count == MAX_MESSAGES {
                        cut = true;
                        continue;
                    }
                    count += 1;
                    found
                        .entry(mailbox_of(dir, &e.path()))
                        .or_default()
                        .push(e.path());
                }
                _ => {}
            }
        }
    }
    let mut result = MailExport {
        not_all_looked_at: cut,
        ..Default::default()
    };
    for (n, (name, mut files)) in found.into_iter().enumerate() {
        files.sort();
        let mut b = MailboxExport {
            name: name.clone(),
            file: None,
            written: 0,
            partial: 0,
            failed: 0,
        };
        let path = out.join(format!("{}.mbox", file_name(&name, n)));
        let mut w: Option<std::io::BufWriter<std::fs::File>> = None;
        for f in files {
            if Instant::now() >= deadline {
                result.not_all_looked_at = true;
                b.failed += 1;
                continue;
            }
            let Ok(bytes) = guard::read_all(&f, MAX_MESSAGE) else {
                b.failed += 1;
                continue;
            };
            let facts = match read(&bytes) {
                Ok(facts) if facts_fit(&facts, bytes.len() as u64) => facts,
                _ => {
                    b.failed += 1;
                    continue;
                }
            };
            if w.is_none() {
                // Never over a file that is there already.
                let file = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)?;
                w = Some(std::io::BufWriter::new(file));
                b.file = Some(path.clone());
            }
            if let Some(w) = w.as_mut() {
                write_entry(w, &bytes, &facts)?;
            }
            if facts.partial {
                b.partial += 1;
            } else {
                b.written += 1;
            }
        }
        if let Some(w) = w {
            w.into_inner()
                .map_err(std::io::IntoInnerError::into_error)?
                .sync_all()?;
        }
        result.mailboxes.push(b);
    }
    Ok(result)
}

/// Whether a file is a saved message, by its first bytes.
fn is_message(path: &Path) -> bool {
    use std::io::Read;
    let Ok(f) = guard::open(path) else {
        return false;
    };
    let mut head = Vec::new();
    if f.take(4096).read_to_end(&mut head).is_err() {
        return false;
    }
    crate::sniff::shape(&head, &[], head.len() as u64) == crate::sniff::Shape::Emlx
}

/// The mailbox a message belongs to: the nearest folder above it named `<name>.mbox`, else its
/// own folder, as a path inside `dir`.
fn mailbox_of(dir: &Path, file: &Path) -> String {
    let rel = file.strip_prefix(dir).unwrap_or(file);
    let parts: Vec<String> = rel
        .parent()
        .map(|p| {
            p.components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    let upto = parts
        .iter()
        .rposition(|p| p.len() > 5 && p.to_ascii_lowercase().ends_with(".mbox"));
    let kept: Vec<String> = match upto {
        Some(i) => parts[..=i]
            .iter()
            .map(|p| strip_mbox(p).to_string())
            .collect(),
        None => parts,
    };
    if kept.is_empty() {
        "Mail".into()
    } else {
        kept.join("/")
    }
}

fn strip_mbox(p: &str) -> &str {
    if p.to_ascii_lowercase().ends_with(".mbox") {
        &p[..p.len() - 5]
    } else {
        p
    }
}

/// A file name for mailbox `name` (number `n`): its letters kept, anything a file name cannot
/// hold replaced, never empty, never a dot name, and numbered so two never meet.
fn file_name(name: &str, n: usize) -> String {
    let clean: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, ' ' | '-' | '_') {
                c
            } else {
                '-'
            }
        })
        .take(80)
        .collect();
    let clean = clean.trim_matches([' ', '-']);
    let clean = if clean.is_empty() { "Mail" } else { clean };
    format!("{} {n:04}", clean)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seal() -> InChild {
        pctwin_parse_seal::in_child()
    }

    fn emlx(message: &[u8], trailer: &[u8]) -> Vec<u8> {
        let mut b = format!("{}       \n", message.len()).into_bytes();
        b.extend_from_slice(message);
        b.extend_from_slice(trailer);
        b
    }

    const PLIST: &[u8] = b"<?xml version=\"1.0\"?>\n<plist><dict><key>flags</key><integer>1</integer></dict></plist>\n";

    fn entry(bytes: &[u8]) -> Vec<u8> {
        let f = emlx_facts(seal(), bytes).unwrap();
        assert!(facts_fit(&f, bytes.len() as u64));
        let mut out = Vec::new();
        write_entry(&mut out, bytes, &f).unwrap();
        out
    }

    #[test]
    fn a_message_is_written_whole_without_its_length_line_or_the_apps_notes() {
        let msg = b"From: a@example.org\nSubject: hi\n\nhello\n";
        let out = entry(&emlx(msg, PLIST));
        let mut want = ENVELOPE.to_vec();
        want.extend_from_slice(msg);
        want.extend_from_slice(b"\n");
        assert_eq!(out, want);
    }

    #[test]
    fn lines_that_would_start_a_message_are_marked_once_more() {
        let msg =
            b"Subject: x\n\nFrom the start\n>From before\n>>From older\nFromage\n From x\nFrom ";
        let out = entry(&emlx(msg, b""));
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("\n>From the start\n"));
        assert!(text.contains("\n>>From before\n"));
        assert!(text.contains("\n>>>From older\n"));
        assert!(text.contains("\nFromage\n"));
        assert!(text.contains("\n From x\n"));
        // The last line, with no line end, is marked and ended.
        assert!(text.ends_with("\n>From \n\n"));
        // Only the envelope starts a line with "From ".
        assert_eq!(text.lines().filter(|l| l.starts_with("From ")).count(), 1);
    }

    #[test]
    fn a_message_whose_attachments_were_kept_apart_is_partial() {
        let msg = b"Subject: x\nContent-Type: multipart/mixed; boundary=b\n\n--b\nContent-Type: application/pdf\nX-Apple-Content-Length: 12345\n\n--b--\n";
        assert!(emlx_facts(seal(), &emlx(msg, PLIST)).unwrap().partial);
        let whole = b"Subject: x\n\nthe word X-Apple-Content-Length: inside a line\n";
        assert!(!emlx_facts(seal(), &emlx(whole, b"")).unwrap().partial);
    }

    #[test]
    fn a_file_that_does_not_say_its_length_truly_is_refused() {
        let msg = b"Subject: x\n\nhi\n";
        let mut short = emlx(msg, b"");
        short.truncate(short.len() - 1);
        for bad in [
            short,
            b"abc\nSubject: x\n".to_vec(),
            b"\nSubject: x\n".to_vec(),
            b"0\nSubject: x\n".to_vec(),
            b"99999999999999999999\nSubject: x\n".to_vec(),
            b"18446744073709551615\nx".to_vec(),
            b"12".to_vec(),
            vec![b'1'; 100],
        ] {
            assert!(emlx_facts(seal(), &bad).is_err(), "{bad:?}");
        }
        // The exact length is enough, with nothing after it.
        assert!(emlx_facts(seal(), &emlx(msg, b"")).is_ok());
    }

    #[test]
    fn facts_that_do_not_fit_the_file_are_never_used() {
        let ok = EmlxFacts {
            start: 4,
            len: 10,
            marked: vec![4, 8],
            partial: false,
        };
        assert!(facts_fit(&ok, 14));
        assert!(!facts_fit(&ok, 13));
        for bad in [
            EmlxFacts {
                len: 0,
                marked: vec![],
                ..ok.clone()
            },
            EmlxFacts {
                start: u64::MAX,
                ..ok.clone()
            },
            EmlxFacts {
                marked: vec![8, 4],
                ..ok.clone()
            },
            EmlxFacts {
                marked: vec![4, 4],
                ..ok.clone()
            },
            EmlxFacts {
                marked: vec![3],
                ..ok.clone()
            },
            EmlxFacts {
                marked: vec![14],
                ..ok.clone()
            },
        ] {
            assert!(!facts_fit(&bad, 14), "{bad:?}");
        }
    }

    fn put(p: &Path, b: &[u8]) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, b).unwrap();
    }

    fn reader(b: &[u8]) -> Result<EmlxFacts, ContentError> {
        emlx_facts(seal(), b)
    }

    fn later() -> Instant {
        Instant::now() + std::time::Duration::from_secs(60)
    }

    #[test]
    fn each_mailbox_becomes_one_mbox_with_an_honest_result() {
        let t = tempfile::tempdir().unwrap();
        let src = t.path().join("src");
        let out = t.path().join("out");
        std::fs::create_dir_all(&out).unwrap();
        let m = |s: &str| emlx(format!("Subject: {s}\n\nbody {s}\n").as_bytes(), PLIST);
        let deep = "Mailboxes/Trips.mbox/1F2E/Data/0/Messages";
        put(&src.join(deep).join("1.emlx"), &m("one"));
        put(&src.join(deep).join("2.emlx"), &m("two"));
        put(&src.join("Mailboxes/Trips.mbox/Info.plist"), PLIST);
        let part = emlx(b"Subject: p\nX-Apple-Content-Length: 9\n\n", b"");
        put(
            &src.join("Mailboxes/Bills.mbox/Messages/3.partial.emlx"),
            &part,
        );
        put(
            &src.join("Mailboxes/Bills.mbox/Messages/4.emlx"),
            &m("four"),
        );
        // Damaged: shorter than it says, but still shaped like a message.
        let mut cut = m("five");
        cut.truncate(cut.len() - PLIST.len() - 3);
        put(&src.join("Mailboxes/Old.mbox/Messages/5.emlx"), &cut);
        let r = export_mailboxes(&src, &out, &reader, later()).unwrap();
        assert!(!r.not_all_looked_at);
        let by = |n: &str| r.mailboxes.iter().find(|b| b.name == n).unwrap();

        let trips = by("Mailboxes/Trips");
        assert_eq!((trips.written, trips.partial, trips.failed), (2, 0, 0));
        assert_eq!(trips.result().status(), ItemStatus::Converted);
        let text = std::fs::read_to_string(trips.file.as_ref().unwrap()).unwrap();
        assert_eq!(text.matches("\nFrom - ").count() + 1, 2, "{text}");
        assert!(text.contains("body one") && text.contains("body two"));
        assert!(!text.contains("plist"));

        let bills = by("Mailboxes/Bills");
        assert_eq!((bills.written, bills.partial, bills.failed), (1, 1, 0));
        assert_eq!(bills.result(), ItemResult::because(Reason::PartsMissing));

        let old = by("Mailboxes/Old");
        assert_eq!((old.written, old.partial, old.failed), (0, 0, 1));
        assert_eq!(old.file, None);
        assert_eq!(old.result(), ItemResult::because(Reason::Unreadable));
        // Nothing beside the mailboxes' own files, and the originals are untouched.
        assert_eq!(std::fs::read_dir(&out).unwrap().count(), 2);
        assert_eq!(
            std::fs::read(src.join(deep).join("1.emlx")).unwrap(),
            m("one")
        );
    }

    #[test]
    fn a_mailbox_file_already_there_is_never_written_over() {
        let t = tempfile::tempdir().unwrap();
        let src = t.path().join("src");
        let out = t.path().join("out");
        put(
            &src.join("Inbox.mbox/Messages/1.emlx"),
            &emlx(b"Subject: a\n\nb\n", b""),
        );
        put(&out.join("Inbox 0000.mbox"), b"the person's own");
        assert!(export_mailboxes(&src, &out, &reader, later()).is_err());
        assert_eq!(
            std::fs::read(out.join("Inbox 0000.mbox")).unwrap(),
            b"the person's own"
        );
    }

    #[test]
    fn a_parser_answer_that_does_not_fit_fails_the_message() {
        let t = tempfile::tempdir().unwrap();
        let src = t.path().join("src");
        let out = t.path().join("out");
        std::fs::create_dir_all(&out).unwrap();
        put(
            &src.join("Inbox.mbox/1.emlx"),
            &emlx(b"Subject: a\n\nb\n", b""),
        );
        let liar = |_: &[u8]| {
            Ok(EmlxFacts {
                start: 0,
                len: 1 << 40,
                marked: vec![],
                partial: false,
            })
        };
        let r = export_mailboxes(&src, &out, &liar, later()).unwrap();
        assert_eq!(r.mailboxes[0].failed, 1);
        assert_eq!(r.mailboxes[0].file, None);
        // Out of time: the rest is said, never written as done.
        let r = export_mailboxes(&src, &t.path().join("none"), &reader, Instant::now()).unwrap();
        assert!(r.not_all_looked_at);
        assert_eq!(r.mailboxes[0].failed, 1);
    }

    #[test]
    fn a_mailbox_is_converted_only_when_every_message_was_written_whole() {
        let b = |written, partial, failed| MailboxExport {
            name: "m".into(),
            file: None,
            written,
            partial,
            failed,
        };
        assert_eq!(b(3, 0, 0).result().status(), ItemStatus::Converted);
        for (w, p, f) in [(3, 0, 1), (3, 1, 0), (0, 1, 0)] {
            assert_eq!(
                b(w, p, f).result(),
                ItemResult::because(Reason::PartsMissing),
                "{w} {p} {f}"
            );
        }
        assert_eq!(b(0, 0, 2).result(), ItemResult::because(Reason::Unreadable));
        assert_eq!(b(0, 0, 0).result(), ItemResult::because(Reason::Unreadable));
    }

    #[test]
    fn mailbox_names_and_file_names() {
        let d = Path::new("/m");
        assert_eq!(mailbox_of(d, Path::new("/m/A.mbox/x/Messages/1.emlx")), "A");
        assert_eq!(
            mailbox_of(d, Path::new("/m/V/A.mbox/B.mbox/M/1.emlx")),
            "V/A/B"
        );
        assert_eq!(mailbox_of(d, Path::new("/m/Folder/1.emlx")), "Folder");
        assert_eq!(mailbox_of(d, Path::new("/m/1.emlx")), "Mail");
        assert_eq!(mailbox_of(d, Path::new("/m/.mbox/1.emlx")), ".mbox");
        assert_eq!(file_name("V/Trips", 3), "V-Trips 0003");
        assert_eq!(file_name("../..", 0), "Mail 0000");
        assert_eq!(file_name("", 1), "Mail 0001");
        assert_eq!(file_name(&"x".repeat(200), 2).len(), 85);
    }
}
