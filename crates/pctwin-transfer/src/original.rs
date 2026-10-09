//! Asking the old laptop whether it still has the original, unchanged, before undo removes the
//! copy on the new laptop.
//!
//! The old laptop is read-only here: it opens the file for reading only, looks at its size,
//! modified time and identity, and lets go. It looks only at files it sent itself (a path the new
//! laptop names is never opened), and an answer it cannot give is "cannot look", never a guess.

use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use pctwin_journal::{FileId, Journal, PlannedWrite};
use pctwin_record::ItemId;

use crate::Stamp;

/// The most items in one request or answer; a longer one is refused when it is read.
pub const MAX_ORIGINALS_PER_REQUEST: usize = 1024;

/// What the old laptop sees now where an original was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalNow {
    /// A regular file is there, with this size, modified time and identity.
    Present {
        size: u64,
        modified_ns: Option<i64>,
        file: Option<FileId>,
    },
    /// Nothing is there any more.
    Missing,
    /// The old laptop could not look (it never sent this item, the place is not an ordinary
    /// file, or reading failed). This confirms nothing.
    CannotLook,
}

/// Which file this open handle is, on its drive, told apart exactly the way the gate tells files
/// apart (on Linux that includes the file's birth time, since a freed file number is handed to
/// the next new file at once). The move records it and the check compares with it, so both
/// always use this one function.
pub(crate) fn file_identity(file: &File) -> io::Result<FileId> {
    pctwin_gate::file_identity(file)
}

/// One fresh, read-only look at one path. Nothing is written, no time is set, and the file is
/// not held open after this returns.
fn look(path: &Path) -> OriginalNow {
    look_with(path, || {})
}

/// [`look`], with a step run between the check by name and the open (tests swap the name there).
fn look_with(path: &Path, between: impl FnOnce()) -> OriginalNow {
    // A link, folder or device is not what the scan saw, so it is never followed or opened.
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.is_file() => {}
        Ok(_) => return OriginalNow::CannotLook,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return OriginalNow::Missing,
        Err(_) => return OriginalNow::CannotLook,
    }
    between();
    let file = match pctwin_gate::open_original(path) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return OriginalNow::Missing,
        Err(_) => return OriginalNow::CannotLook,
    };
    // The handle itself must be a regular file (the name may have changed since the look above).
    let Ok(meta) = file.metadata() else {
        return OriginalNow::CannotLook;
    };
    if !meta.is_file() {
        return OriginalNow::CannotLook;
    }
    let stamp = Stamp::of(&meta);
    OriginalNow::Present {
        size: stamp.size,
        modified_ns: stamp.modified_ns,
        file: file_identity(&file).ok(),
    }
}

/// Old laptop: answers for `items` from its own list of what it sent (item to source path).
/// An item it did not send is `CannotLook`: a path the new laptop names is never looked at.
/// The answers come in the order asked.
pub fn answer_originals(
    sent: &HashMap<ItemId, PathBuf>,
    items: &[ItemId],
) -> Vec<(ItemId, OriginalNow)> {
    items
        .iter()
        .map(|item| {
            let now = match sent.get(item) {
                Some(path) => look(path),
                None => OriginalNow::CannotLook,
            };
            (*item, now)
        })
        .collect()
}

/// New laptop: is the original still exactly the file that was copied? True only if it is
/// present, its identity is known on both sides and equal, and its size and modified time are
/// known and equal to what was read at move time. Anything missing or different is false.
pub fn original_unchanged(write: &PlannedWrite, now: &OriginalNow) -> bool {
    let OriginalNow::Present {
        size,
        modified_ns,
        file,
    } = now
    else {
        return false;
    };
    let (Some(recorded), Some(seen)) = (write.source_file, file) else {
        return false;
    };
    let (Some(recorded_time), Some(seen_time)) = (write.source_modified_ns, modified_ns) else {
        return false;
    };
    recorded == *seen && *size == write.size && recorded_time == *seen_time
}

/// How long a [`Confirmed`] stays usable. Undo asks, then removes copies one by one; ten minutes
/// is far longer than a removal pass takes, yet short enough that an answer cannot outlive the
/// user stepping away, editing the old laptop's files and coming back. After that undo asks again.
pub const MAX_CONFIRMED_AGE: Duration = Duration::from_secs(10 * 60);

/// Why a whole request's answers were thrown away (every item in it became `CannotLook`). The
/// caller logs these; this crate has no logger of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalsFault {
    /// The reply did not carry the nonce of the request it answered: a stale reply, a replay of an
    /// earlier one, or one that was not an answer to this question. `items` is how many items the
    /// request asked about.
    WrongNonce { items: usize },
}

impl std::fmt::Display for OriginalsFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WrongNonce { items } => write!(
                f,
                "the old laptop's reply was not for the question just asked, so its answers for {items} item(s) were thrown away"
            ),
        }
    }
}

/// What the old laptop said about the originals, tied to the one journal it was asked for and the
/// moment it was asked. Only [`check_originals`](crate::check_originals) (or [`Confirmed::none`])
/// makes one; undo takes it and cannot invent, extend or move an answer. It cannot be cloned or
/// defaulted, and its fields are private.
///
/// It is good for one undo pass: [`undo`](crate::undo) takes it by value, so a second pass needs a
/// fresh check. Using it twice does not compile:
///
/// ```compile_fail,E0382
/// # fn demo(journal: &pctwin_journal::Journal, table: &pctwin_gate::Destinations) {
/// let token = pctwin_transfer::Confirmed::none(journal);
/// let _ = pctwin_transfer::undo(journal, table, token);
/// let _ = pctwin_transfer::undo(journal, table, token);
/// # }
/// ```
#[derive(Debug)]
pub struct Confirmed {
    journal: u64,
    taken: Instant,
    answers: HashMap<ItemId, OriginalNow>,
    faults: Vec<OriginalsFault>,
}

impl Confirmed {
    /// The old laptop could not be asked: this confirms nothing.
    pub fn none(journal: &Journal) -> Self {
        Self::from_answers(
            journal.instance(),
            Instant::now(),
            HashMap::new(),
            Vec::new(),
        )
    }

    /// Requests whose answers were thrown away, and why (for the caller to log). Empty when every
    /// reply was an answer to the question asked.
    pub fn faults(&self) -> &[OriginalsFault] {
        &self.faults
    }

    /// What the old laptop said about `item`, if this is for `journal`, is no older than
    /// [`MAX_CONFIRMED_AGE`], and `item` was asked about. Otherwise `None`, which confirms nothing.
    pub fn answer(&self, journal: &Journal, item: &ItemId) -> Option<&OriginalNow> {
        self.answer_at(Instant::now(), journal, item)
    }

    pub(crate) fn from_answers(
        journal: u64,
        taken: Instant,
        answers: HashMap<ItemId, OriginalNow>,
        faults: Vec<OriginalsFault>,
    ) -> Self {
        Self {
            journal,
            taken,
            answers,
            faults,
        }
    }

    /// [`answer`](Self::answer) as of `now` (tests choose the time).
    pub(crate) fn answer_at(
        &self,
        now: Instant,
        journal: &Journal,
        item: &ItemId,
    ) -> Option<&OriginalNow> {
        if self.journal != journal.instance() {
            return None;
        }
        // A clock that did not move forward counts as no time having passed, never as an error.
        if now.saturating_duration_since(self.taken) > MAX_CONFIRMED_AGE {
            return None;
        }
        self.answers.get(item)
    }
}

#[cfg(test)]
// Tests set up files of their own directly (fixtures), as the integration tests do.
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    fn id(n: u8) -> ItemId {
        ItemId::from_hex(&format!("{n:02x}{}", "0".repeat(30))).unwrap()
    }

    fn journal() -> (tempfile::TempDir, Journal) {
        let dir = tempfile::tempdir().unwrap();
        let j = Journal::open(&dir.path().join("j.redb")).unwrap();
        (dir, j)
    }

    fn token(j: &Journal, at: Instant) -> Confirmed {
        Confirmed::from_answers(
            j.instance(),
            at,
            HashMap::from([(id(1), OriginalNow::Missing)]),
            Vec::new(),
        )
    }

    #[test]
    fn an_answer_is_given_while_fresh_and_only_for_items_asked() {
        let (_d, j) = journal();
        let t0 = Instant::now();
        let c = token(&j, t0);
        assert_eq!(c.answer_at(t0, &j, &id(1)), Some(&OriginalNow::Missing));
        assert_eq!(c.answer_at(t0, &j, &id(2)), None, "never asked");
    }

    #[test]
    fn a_token_older_than_the_limit_is_refused_and_one_at_the_limit_is_not() {
        let (_d, j) = journal();
        let t0 = Instant::now();
        let c = token(&j, t0);
        assert!(c.answer_at(t0 + MAX_CONFIRMED_AGE, &j, &id(1)).is_some());
        let late = t0 + MAX_CONFIRMED_AGE + Duration::from_nanos(1);
        assert_eq!(c.answer_at(late, &j, &id(1)), None);
        assert_eq!(
            c.answer_at(t0 + Duration::from_secs(3600), &j, &id(1)),
            None
        );
    }

    #[test]
    fn a_time_before_the_token_was_taken_is_not_an_error() {
        let (_d, j) = journal();
        let t0 = Instant::now();
        let c = token(&j, t0 + Duration::from_secs(5));
        assert!(c.answer_at(t0, &j, &id(1)).is_some());
    }

    #[test]
    fn a_token_for_another_journal_is_refused() {
        let (_d1, one) = journal();
        let (_d2, two) = journal();
        assert_ne!(one.instance(), two.instance());
        let t0 = Instant::now();
        let c = token(&one, t0);
        assert!(c.answer_at(t0, &one, &id(1)).is_some());
        assert_eq!(c.answer_at(t0, &two, &id(1)), None);
    }

    #[test]
    fn none_confirms_nothing_for_any_item() {
        let (_d, j) = journal();
        let c = Confirmed::none(&j);
        for n in 0..=255u8 {
            assert_eq!(c.answer(&j, &id(n)), None);
        }
    }

    #[test]
    fn the_public_answer_uses_the_real_clock() {
        let (_d, j) = journal();
        let c = token(&j, Instant::now());
        assert!(c.answer(&j, &id(1)).is_some());
    }

    #[cfg(unix)]
    mod unix {
        use super::*;
        use std::os::unix::fs::symlink;

        fn mkfifo(path: &Path) {
            // The mkfifo tool is on every Unix.
            let status = std::process::Command::new("mkfifo")
                .arg(path)
                .status()
                .unwrap();
            assert!(status.success());
        }

        fn remove(path: &Path) {
            std::fs::remove_file(path).unwrap();
        }

        #[test]
        fn a_link_swapped_in_after_the_check_is_refused_not_followed() {
            let dir = tempfile::tempdir().unwrap();
            let target = dir.path().join("secret.bin");
            std::fs::write(&target, b"not the original").unwrap();
            let name = dir.path().join("orig.bin");
            std::fs::write(&name, b"the original").unwrap();
            let answer = look_with(&name, || {
                remove(&name);
                symlink(&target, &name).unwrap();
            });
            assert_eq!(answer, OriginalNow::CannotLook, "{answer:?}");
        }

        #[test]
        fn a_pipe_swapped_in_after_the_check_never_blocks_the_look() {
            let dir = tempfile::tempdir().unwrap();
            let name = dir.path().join("orig.bin");
            std::fs::write(&name, b"the original").unwrap();
            let (tx, rx) = std::sync::mpsc::channel();
            let n2 = name.clone();
            std::thread::spawn(move || {
                let answer = look_with(&n2, || {
                    remove(&n2);
                    mkfifo(&n2);
                });
                let _ = tx.send(answer);
            });
            let answer = rx
                .recv_timeout(Duration::from_secs(20))
                .expect("the look waited on a pipe");
            assert_eq!(answer, OriginalNow::CannotLook);
        }

        #[test]
        fn a_pipe_is_not_looked_at() {
            let dir = tempfile::tempdir().unwrap();
            let name = dir.path().join("pipe");
            mkfifo(&name);
            assert_eq!(look(&name), OriginalNow::CannotLook);
        }
    }
}
