//! Undo asks the old laptop whether it still has the original, unchanged (Security Design part B,
//! "Undo removes only the file it checked, through one handle"). The old laptop answers read-only
//! and only for what it sent; the new laptop trusts an answer only if it is exactly one answer to
//! a question it asked, and removes a copy only if the original is the very same file, with the
//! same size and the same modified time as when it was read.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use pctwin_gate::{Approved, Destinations};
use pctwin_journal::{Actor, FileId, Journal, PartialKeep, Permission, PlannedWrite};
use pctwin_record::{ItemId, LaptopId};
use pctwin_transfer::{
    Channel, ChannelError, Confirmed, MAX_ORIGINALS_PER_REQUEST, Message, NONCE_LEN, Nonce,
    OriginalNow, OriginalsBudget, OriginalsFault, ReceiverSession, SendJob, SenderSession, Tier,
    TransferError, answer_originals, check_originals, original_unchanged, serve_originals,
};
use tokio::sync::mpsc;

mod common;

struct Mem {
    tx: mpsc::UnboundedSender<Vec<u8>>,
    rx: mpsc::UnboundedReceiver<Vec<u8>>,
}

fn mem_pair() -> (Mem, Mem) {
    let (a_tx, b_rx) = mpsc::unbounded_channel();
    let (b_tx, a_rx) = mpsc::unbounded_channel();
    (Mem { tx: a_tx, rx: a_rx }, Mem { tx: b_tx, rx: b_rx })
}

impl Channel for Mem {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        tokio::task::yield_now().await;
        self.tx.send(data.to_vec()).map_err(|_| ChannelError)
    }
    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        self.rx.recv().await.ok_or(ChannelError)
    }
}

fn id(n: u8) -> ItemId {
    ItemId::from_hex(&format!("{n:02x}{}", "0".repeat(30))).unwrap()
}

fn bytes(len: usize, seed: u8) -> Vec<u8> {
    (0..len)
        .map(|i| (i as u8).wrapping_mul(31).wrapping_add(seed))
        .collect()
}

use common::identity_of;

fn modified_ns(path: &Path) -> i64 {
    std::fs::metadata(path)
        .unwrap()
        .modified()
        .unwrap()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as i64
}

fn set_mtime(path: &Path, t: SystemTime) {
    let f = std::fs::OpenOptions::new().write(true).open(path).unwrap();
    f.set_modified(t).unwrap();
}

struct World {
    old: tempfile::TempDir,
    _mine: tempfile::TempDir,
    _jdir: tempfile::TempDir,
    journal: Journal,
    table: Destinations,
}

fn world() -> World {
    let old = tempfile::tempdir().unwrap();
    let mine = tempfile::tempdir().unwrap();
    let jdir = tempfile::tempdir().unwrap();
    let journal = Journal::open(&jdir.path().join("journal.redb")).unwrap();
    let mut table = Destinations::new();
    table
        .approve("me", Approved::MyFolders, mine.path())
        .unwrap();
    World {
        old,
        _mine: mine,
        _jdir: jdir,
        journal,
        table,
    }
}

impl World {
    fn job(&self, n: u8, len: usize) -> (SendJob, Vec<u8>) {
        let source = self.old.path().join(format!("f{n}.bin"));
        let data = bytes(len, n);
        std::fs::write(&source, &data).unwrap();
        (
            SendJob {
                item: id(n),
                source,
                destination: "me".into(),
                path: format!("Documents/f{n}.bin"),
                compressible: false,
                tier: Tier::Rest,
            },
            data,
        )
    }

    /// Moves the jobs for real and returns what the new laptop's journal planned for each.
    async fn move_all(&self, jobs: &[(SendJob, Vec<u8>)]) -> HashMap<ItemId, PlannedWrite> {
        let files: Vec<(ItemId, u64)> =
            jobs.iter().map(|(j, d)| (j.item, d.len() as u64)).collect();
        let mut receiver =
            ReceiverSession::new(&self.table, common::approved(&files), &self.journal, "1001");
        let mut sender = SenderSession::new(jobs.iter().map(|(j, _)| j.clone()).collect(), 2);
        let (a, b) = mem_pair();
        let send_side = async {
            let mut a = a;
            sender.run(&mut a).await
        };
        let receive_side = async {
            let mut b = b;
            receiver.run(&mut b).await
        };
        let (s, r) = tokio::time::timeout(Duration::from_secs(30), async {
            tokio::join!(send_side, receive_side)
        })
        .await
        .expect("the move hung");
        s.unwrap();
        r.unwrap();
        self.journal
            .entries()
            .unwrap()
            .into_iter()
            .map(|e| (e.write.item, e.write))
            .collect()
    }
}

fn sent_of(jobs: &[(SendJob, Vec<u8>)]) -> HashMap<ItemId, PathBuf> {
    jobs.iter()
        .map(|(j, _)| (j.item, j.source.clone()))
        .collect()
}

/// One round: the new laptop asks, the old laptop answers from `sent`.
async fn ask(sent: &HashMap<ItemId, PathBuf>, items: &[ItemId]) -> HashMap<ItemId, OriginalNow> {
    let (mut new, mut old) = mem_pair();
    let chunks = items.len().div_ceil(MAX_ORIGINALS_PER_REQUEST).max(1);
    let journal = common::journal();
    let serve = async {
        let mut budget = OriginalsBudget::new();
        for _ in 0..chunks {
            if items.is_empty() {
                break;
            }
            serve_originals(&mut old, sent, &mut budget).await.unwrap();
        }
    };
    let (answers, ()) = tokio::time::timeout(Duration::from_secs(30), async {
        tokio::join!(check_originals(&mut new, journal, items), serve)
    })
    .await
    .expect("the check hung");
    flatten(&answers.unwrap(), journal, items)
}

/// What a token says for each of `items` (those it has no answer for are left out).
fn flatten(c: &Confirmed, journal: &Journal, items: &[ItemId]) -> HashMap<ItemId, OriginalNow> {
    items
        .iter()
        .filter_map(|i| c.answer(journal, i).map(|n| (*i, *n)))
        .collect()
}

fn planned(source_file: Option<FileId>, modified: Option<i64>, size: u64) -> PlannedWrite {
    PlannedWrite {
        item: id(1),
        source_laptop: LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap(),
        destination: "me".into(),
        path: "a".into(),
        size,
        actor: Actor {
            acting_account: "1001".into(),
            for_account: "1001".into(),
            permission: Permission::OwnFolders,
        },
        block_size: 131_072,
        source_modified_ns: modified,
        place: None,
        source_file,
        partial_keep: PartialKeep::default(),
    }
}

// ---- the original's identity is recorded at move time ----

#[tokio::test]
async fn a_real_move_records_the_originals_identity_in_the_journal() {
    let w = world();
    let jobs = vec![w.job(1, 300_000), w.job(2, 10)];
    let identities: Vec<FileId> = jobs.iter().map(|(j, _)| identity_of(&j.source)).collect();
    let planned = w.move_all(&jobs).await;
    for ((job, _), ident) in jobs.iter().zip(identities) {
        assert_eq!(planned[&job.item].source_file, Some(ident));
        assert_eq!(
            planned[&job.item].source_modified_ns,
            Some(modified_ns(&job.source))
        );
    }
}

// ---- the whole check, end to end ----

#[tokio::test]
async fn an_unchanged_original_is_confirmed() {
    let w = world();
    let jobs = vec![w.job(1, 300_000)];
    let planned = w.move_all(&jobs).await;
    let now = ask(&sent_of(&jobs), &[id(1)]).await;
    assert!(
        original_unchanged(&planned[&id(1)], &now[&id(1)]),
        "{now:?}"
    );
}

#[tokio::test]
async fn an_edited_original_with_the_same_size_is_not_confirmed() {
    let w = world();
    let jobs = vec![w.job(1, 5000)];
    let planned = w.move_all(&jobs).await;
    // Same size, different bytes, a later modified time.
    let mut edited = bytes(5000, 99);
    edited[0] ^= 1;
    std::fs::write(&jobs[0].0.source, &edited).unwrap();
    set_mtime(
        &jobs[0].0.source,
        SystemTime::UNIX_EPOCH
            + Duration::from_nanos(modified_ns(&jobs[0].0.source) as u64 + 5_000_000_000),
    );
    let now = ask(&sent_of(&jobs), &[id(1)]).await;
    assert!(matches!(
        now[&id(1)],
        OriginalNow::Present { size: 5000, .. }
    ));
    assert!(!original_unchanged(&planned[&id(1)], &now[&id(1)]));
}

#[tokio::test]
async fn a_replaced_original_with_the_same_bytes_and_time_is_not_confirmed() {
    let w = world();
    let jobs = vec![w.job(1, 5000)];
    let planned = w.move_all(&jobs).await;
    // A new file, made while the old one still exists (so it cannot reuse its identity), with the
    // same bytes and the same modified time, renamed over it.
    let source = &jobs[0].0.source;
    let when = SystemTime::UNIX_EPOCH + Duration::from_nanos(modified_ns(source) as u64);
    let other = w.old.path().join("replacement.tmp");
    std::fs::write(&other, &jobs[0].1).unwrap();
    set_mtime(&other, when);
    std::fs::rename(&other, source).unwrap();
    let now = ask(&sent_of(&jobs), &[id(1)]).await;
    let OriginalNow::Present {
        size,
        modified_ns: seen,
        file,
    } = now[&id(1)]
    else {
        panic!("{now:?}")
    };
    assert_eq!(size, 5000);
    assert_eq!(seen, planned[&id(1)].source_modified_ns);
    assert_ne!(file, planned[&id(1)].source_file);
    assert!(!original_unchanged(&planned[&id(1)], &now[&id(1)]));
}

#[tokio::test]
async fn a_removed_original_is_missing_and_not_confirmed() {
    let w = world();
    let jobs = vec![w.job(1, 5000)];
    let planned = w.move_all(&jobs).await;
    std::fs::remove_file(&jobs[0].0.source).unwrap();
    let now = ask(&sent_of(&jobs), &[id(1)]).await;
    assert_eq!(now[&id(1)], OriginalNow::Missing);
    assert!(!original_unchanged(&planned[&id(1)], &now[&id(1)]));
}

#[tokio::test]
async fn an_item_the_old_laptop_never_sent_cannot_be_looked_at() {
    let w = world();
    let jobs = vec![w.job(1, 5000)];
    // A real file the old laptop did not send: the new laptop cannot make it look there.
    let (stranger, _) = w.job(7, 100);
    assert!(stranger.source.exists());
    let now = ask(&sent_of(&jobs), &[id(1), id(7)]).await;
    assert_eq!(now[&id(7)], OriginalNow::CannotLook);
    assert!(matches!(now[&id(1)], OriginalNow::Present { .. }));
}

#[tokio::test]
async fn many_items_go_in_several_requests_and_every_one_is_answered() {
    let items: Vec<ItemId> = (0..(MAX_ORIGINALS_PER_REQUEST * 2 + 5))
        .map(|i| ItemId::from_hex(&format!("{i:032x}")).unwrap())
        .collect();
    let now = ask(&HashMap::new(), &items).await;
    assert_eq!(now.len(), items.len());
    assert!(now.values().all(|n| *n == OriginalNow::CannotLook));
}

#[tokio::test]
async fn asking_about_nothing_sends_nothing() {
    let (mut new, mut old) = mem_pair();
    let journal = common::journal();
    let now = check_originals(&mut new, journal, &[]).await.unwrap();
    assert_eq!(now.answer(journal, &id(1)), None);
    drop(new);
    assert!(old.recv().await.is_err(), "nothing was sent");
}

#[tokio::test]
async fn the_same_item_asked_twice_is_asked_once_and_answered() {
    let w = world();
    let jobs = vec![w.job(1, 100)];
    let now = ask(&sent_of(&jobs), &[id(1), id(1)]).await;
    assert!(matches!(now[&id(1)], OriginalNow::Present { .. }));
}

// ---- a hostile old laptop ----

/// Asks about `items`, and lets a scripted old laptop answer with `reply`.
async fn ask_hostile(
    items: &[ItemId],
    reply: impl FnOnce(Nonce) -> Vec<u8>,
) -> Result<HashMap<ItemId, OriginalNow>, TransferError> {
    ask_hostile_with(items, reply)
        .await
        .map(|(now, _faults)| now)
}

/// [`ask_hostile`], also returning what the token says went wrong.
async fn ask_hostile_with(
    items: &[ItemId],
    reply: impl FnOnce(Nonce) -> Vec<u8>,
) -> Result<(HashMap<ItemId, OriginalNow>, Vec<OriginalsFault>), TransferError> {
    let (mut new, mut old) = mem_pair();
    let script = async {
        let Message::CheckOriginals { nonce, .. } =
            Message::decode(&old.recv().await.unwrap()).unwrap()
        else {
            panic!("not a request")
        };
        old.send(&reply(nonce)).await.unwrap();
    };
    let journal = common::journal();
    let (answers, ()) = tokio::join!(check_originals(&mut new, journal, items), script);
    answers.map(|c| (flatten(&c, journal, items), c.faults().to_vec()))
}

fn present(n: u64) -> OriginalNow {
    OriginalNow::Present {
        size: n,
        modified_ns: Some(1),
        file: Some(FileId {
            volume: 1,
            index: std::num::NonZeroU64::new(n).unwrap(),
            born: None,
        }),
    }
}

#[tokio::test]
async fn an_answer_for_an_item_not_asked_confirms_nothing_and_is_dropped() {
    let reply = |nonce| {
        Message::Originals {
            nonce,
            answers: vec![(id(1), present(1)), (id(9), present(9))],
        }
        .encode()
    };
    let now = ask_hostile(&[id(1), id(2)], reply).await.unwrap();
    assert_eq!(now[&id(1)], present(1));
    assert_eq!(now[&id(2)], OriginalNow::CannotLook, "no answer given");
    assert!(!now.contains_key(&id(9)));
}

#[tokio::test]
async fn a_duplicate_answer_spoils_that_item() {
    let reply = |nonce| {
        Message::Originals {
            nonce,
            answers: vec![
                (id(1), present(1)),
                (id(2), present(2)),
                (id(1), present(1)),
            ],
        }
        .encode()
    };
    let now = ask_hostile(&[id(1), id(2)], reply).await.unwrap();
    assert_eq!(now[&id(1)], OriginalNow::CannotLook);
    assert_eq!(now[&id(2)], present(2));
}

#[tokio::test]
async fn a_dropped_answer_is_not_confirmed() {
    let reply = |nonce| {
        Message::Originals {
            nonce,
            answers: vec![(id(2), present(2))],
        }
        .encode()
    };
    let now = ask_hostile(&[id(1), id(2), id(3)], reply).await.unwrap();
    assert_eq!(now[&id(1)], OriginalNow::CannotLook);
    assert_eq!(now[&id(2)], present(2));
    assert_eq!(now[&id(3)], OriginalNow::CannotLook);
    assert_eq!(now.len(), 3);
}

#[tokio::test]
async fn an_empty_answer_confirms_nothing() {
    let reply = |nonce| {
        Message::Originals {
            nonce,
            answers: vec![],
        }
        .encode()
    };
    let now = ask_hostile(&[id(1)], reply).await.unwrap();
    assert_eq!(now[&id(1)], OriginalNow::CannotLook);
}

#[tokio::test]
async fn an_oversized_damaged_or_wrong_reply_is_an_error() {
    let big = |nonce| {
        Message::Originals {
            nonce,
            answers: (0..=MAX_ORIGINALS_PER_REQUEST)
                .map(|i| {
                    (
                        ItemId::from_hex(&format!("{i:032x}")).unwrap(),
                        OriginalNow::Missing,
                    )
                })
                .collect(),
        }
        .encode()
    };
    assert!(ask_hostile(&[id(1)], big).await.is_err());
    assert!(
        ask_hostile(&[id(1)], |_| vec![13, 0, 0, 0, 0, 0])
            .await
            .is_err()
    );
    assert!(ask_hostile(&[id(1)], |_| vec![]).await.is_err());
    // A valid message of the wrong kind.
    assert!(
        ask_hostile(&[id(1)], |_| Message::Ready.encode())
            .await
            .is_err()
    );
    // The reply to a request must not be another request.
    let echo = |nonce| {
        Message::CheckOriginals {
            nonce,
            items: vec![id(1)],
        }
        .encode()
    };
    assert!(ask_hostile(&[id(1)], echo).await.is_err());
}

#[tokio::test]
async fn a_dropped_connection_is_an_error_not_a_confirmation() {
    let (mut new, old) = mem_pair();
    drop(old);
    let r = check_originals(&mut new, common::journal(), &[id(1)]).await;
    assert!(matches!(r, Err(TransferError::ConnectionDropped)));
}

#[tokio::test]
async fn the_old_laptop_refuses_anything_but_a_request() {
    let sent = HashMap::new();
    let mut budget = OriginalsBudget::new();
    let (mut new, mut old) = mem_pair();
    new.send(&Message::Ready.encode()).await.unwrap();
    assert!(serve_originals(&mut old, &sent, &mut budget).await.is_err());
    new.send(&[255, 0, 0, 0, 0]).await.unwrap();
    assert!(serve_originals(&mut old, &sent, &mut budget).await.is_err());
    // An oversized request.
    let big = Message::CheckOriginals {
        nonce: [1; NONCE_LEN],
        items: (0..=MAX_ORIGINALS_PER_REQUEST)
            .map(|i| ItemId::from_hex(&format!("{i:032x}")).unwrap())
            .collect(),
    };
    new.send(&big.encode()).await.unwrap();
    assert!(serve_originals(&mut old, &sent, &mut budget).await.is_err());
}

// ---- the old laptop stays read-only ----

#[tokio::test]
async fn the_check_leaves_the_original_exactly_as_it_was() {
    let w = world();
    let jobs = vec![w.job(1, 300_000)];
    let source = jobs[0].0.source.clone();
    // An old modified time, so any "touch" would show.
    set_mtime(
        &source,
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_600_000_000),
    );
    let before = (
        std::fs::read(&source).unwrap(),
        std::fs::metadata(&source).unwrap().modified().unwrap(),
        identity_of(&source),
        std::fs::metadata(&source).unwrap().permissions().readonly(),
    );
    let sent = sent_of(&jobs);
    let now = ask(&sent, &[id(1)]).await;
    assert!(matches!(now[&id(1)], OriginalNow::Present { .. }));
    // Even a read-only original is looked at.
    let mut perms = std::fs::metadata(&source).unwrap().permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&source, perms).unwrap();
    assert!(matches!(
        answer_originals(&sent, &[id(1)])[0].1,
        OriginalNow::Present { .. }
    ));
    let mut perms = std::fs::metadata(&source).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    perms.set_readonly(false);
    std::fs::set_permissions(&source, perms).unwrap();
    let after = (
        std::fs::read(&source).unwrap(),
        std::fs::metadata(&source).unwrap().modified().unwrap(),
        identity_of(&source),
        std::fs::metadata(&source).unwrap().permissions().readonly(),
    );
    assert_eq!(before, after);
    // Nothing else appeared next to it, and it is not held open: it can be renamed and removed.
    assert_eq!(std::fs::read_dir(w.old.path()).unwrap().count(), 1);
    let moved = w.old.path().join("renamed.bin");
    std::fs::rename(&source, &moved).unwrap();
    std::fs::remove_file(&moved).unwrap();
}

#[test]
fn a_file_open_elsewhere_can_still_be_looked_at_and_does_not_block_its_owner() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("busy.bin");
    std::fs::write(&path, b"hello").unwrap();
    let busy = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    let sent = HashMap::from([(id(1), path.clone())]);
    let now = answer_originals(&sent, &[id(1)]);
    assert!(matches!(now[0].1, OriginalNow::Present { size: 5, .. }));
    // The owner can still write after the look.
    use std::io::Write;
    let mut busy = busy;
    busy.write_all(b"!").unwrap();
}

#[test]
fn only_an_ordinary_file_is_looked_at() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("folder");
    std::fs::create_dir(&folder).unwrap();
    let sent = HashMap::from([(id(1), folder.clone()), (id(2), dir.path().join("nothing"))]);
    let answers = answer_originals(&sent, &[id(1), id(2)]);
    assert_eq!(answers[0].1, OriginalNow::CannotLook);
    assert_eq!(answers[1].1, OriginalNow::Missing);
}

#[cfg(unix)]
#[test]
fn a_link_is_never_followed() {
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("real.bin");
    std::fs::write(&real, b"data").unwrap();
    let link = dir.path().join("link.bin");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let sent = HashMap::from([(id(1), link)]);
    assert_eq!(
        answer_originals(&sent, &[id(1)])[0].1,
        OriginalNow::CannotLook
    );
}

#[test]
fn answers_come_in_the_order_asked() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a");
    std::fs::write(&a, b"a").unwrap();
    let sent = HashMap::from([(id(1), a)]);
    let answers = answer_originals(&sent, &[id(3), id(1), id(2)]);
    let order: Vec<ItemId> = answers.iter().map(|(i, _)| *i).collect();
    assert_eq!(order, vec![id(3), id(1), id(2)]);
}

// ---- the decision ----

#[test]
fn only_the_same_file_with_the_same_size_and_time_is_unchanged() {
    let file = FileId {
        volume: 7,
        index: std::num::NonZeroU64::new(42).unwrap(),
        born: None,
    };
    let write = planned(Some(file), Some(1000), 50);
    let same = OriginalNow::Present {
        size: 50,
        modified_ns: Some(1000),
        file: Some(file),
    };
    assert!(original_unchanged(&write, &same));

    let differs = |now: OriginalNow| assert!(!original_unchanged(&write, &now), "{now:?}");
    // Each field different.
    differs(OriginalNow::Present {
        size: 51,
        modified_ns: Some(1000),
        file: Some(file),
    });
    differs(OriginalNow::Present {
        size: 49,
        modified_ns: Some(1000),
        file: Some(file),
    });
    differs(OriginalNow::Present {
        size: 50,
        modified_ns: Some(1001),
        file: Some(file),
    });
    differs(OriginalNow::Present {
        size: 50,
        modified_ns: Some(999),
        file: Some(file),
    });
    differs(OriginalNow::Present {
        size: 50,
        modified_ns: Some(1000),
        file: Some(FileId {
            volume: 8,
            index: std::num::NonZeroU64::new(42).unwrap(),
            born: None,
        }),
    });
    differs(OriginalNow::Present {
        size: 50,
        modified_ns: Some(1000),
        file: Some(FileId {
            volume: 7,
            index: std::num::NonZeroU64::new(43).unwrap(),
            born: None,
        }),
    });
    // Each field unknown on the old laptop's side.
    differs(OriginalNow::Present {
        size: 50,
        modified_ns: None,
        file: Some(file),
    });
    differs(OriginalNow::Present {
        size: 50,
        modified_ns: Some(1000),
        file: None,
    });
    differs(OriginalNow::Missing);
    differs(OriginalNow::CannotLook);

    // Each field unknown on the new laptop's side: two unknowns are never "equal".
    let unknown_file = planned(None, Some(1000), 50);
    assert!(!original_unchanged(&unknown_file, &same));
    assert!(!original_unchanged(
        &unknown_file,
        &OriginalNow::Present {
            size: 50,
            modified_ns: Some(1000),
            file: None
        }
    ));
    let unknown_time = planned(Some(file), None, 50);
    assert!(!original_unchanged(&unknown_time, &same));
    assert!(!original_unchanged(
        &unknown_time,
        &OriginalNow::Present {
            size: 50,
            modified_ns: None,
            file: Some(file)
        }
    ));
    // A different size recorded.
    assert!(!original_unchanged(
        &planned(Some(file), Some(1000), 51),
        &same
    ));
}

// ---- the token the check hands to undo ----

#[tokio::test]
async fn the_helper_token_answers_exactly_what_the_scripted_old_laptop_said() {
    let j = common::journal();
    let c = common::confirmed(
        j,
        vec![
            (id(1), present(1)),
            (id(2), OriginalNow::Missing),
            (id(3), OriginalNow::CannotLook),
        ],
    )
    .await;
    assert_eq!(c.answer(j, &id(1)), Some(&present(1)));
    assert_eq!(c.answer(j, &id(2)), Some(&OriginalNow::Missing));
    assert_eq!(c.answer(j, &id(3)), Some(&OriginalNow::CannotLook));
    assert_eq!(c.answer(j, &id(4)), None, "never asked");
}

#[test]
fn the_blocking_helper_works_outside_a_runtime() {
    let j = common::journal();
    let c = common::confirmed_blocking(j, vec![(id(1), present(1))]);
    assert_eq!(c.answer(j, &id(1)), Some(&present(1)));
}

#[tokio::test]
async fn a_token_from_one_journal_gives_nothing_to_another() {
    let a = world();
    let b = world();
    let c = common::confirmed(&a.journal, vec![(id(1), present(1))]).await;
    assert!(c.answer(&a.journal, &id(1)).is_some());
    assert_eq!(c.answer(&b.journal, &id(1)), None);
}

#[tokio::test]
async fn an_answer_for_an_item_not_asked_is_not_in_the_token() {
    let j = common::journal();
    let reply = |nonce| {
        Message::Originals {
            nonce,
            answers: vec![(id(1), present(1)), (id(9), present(9))],
        }
        .encode()
    };
    let (mut new, mut old) = mem_pair();
    let script = async {
        let Message::CheckOriginals { nonce, .. } =
            Message::decode(&old.recv().await.unwrap()).unwrap()
        else {
            panic!("not a request")
        };
        old.send(&reply(nonce)).await.unwrap();
    };
    let asked = [id(1)];
    let (c, ()) = tokio::join!(check_originals(&mut new, j, &asked), script);
    let c = c.unwrap();
    assert_eq!(c.answer(j, &id(1)), Some(&present(1)));
    assert_eq!(c.answer(j, &id(9)), None);
}

#[tokio::test]
async fn nothing_asked_means_the_token_confirms_nothing() {
    let j = common::journal();
    let (mut new, _old) = mem_pair();
    let c = check_originals(&mut new, j, &[]).await.unwrap();
    assert_eq!(c.answer(j, &id(1)), None);
}

#[test]
fn a_token_for_no_answers_confirms_nothing() {
    let j = common::journal();
    assert_eq!(Confirmed::none(j).answer(j, &id(1)), None);
}

// ---- the nonce: an answer belongs to the one question just asked ----

fn answer_for(nonce: Nonce, items: &[ItemId]) -> Vec<u8> {
    Message::Originals {
        nonce,
        answers: items.iter().map(|i| (*i, present(1))).collect(),
    }
    .encode()
}

#[tokio::test]
async fn an_answer_that_echoes_the_nonce_is_believed() {
    let (now, faults) = ask_hostile_with(&[id(1)], |n| answer_for(n, &[id(1)]))
        .await
        .unwrap();
    assert_eq!(now[&id(1)], present(1));
    assert!(faults.is_empty());
}

#[tokio::test]
async fn an_answer_with_the_wrong_nonce_confirms_nothing_and_is_reported() {
    let (now, faults) = ask_hostile_with(&[id(1), id(2)], |_| {
        answer_for([0; NONCE_LEN], &[id(1), id(2)])
    })
    .await
    .unwrap();
    assert_eq!(now[&id(1)], OriginalNow::CannotLook);
    assert_eq!(now[&id(2)], OriginalNow::CannotLook);
    assert_eq!(faults, vec![OriginalsFault::WrongNonce { items: 2 }]);
}

#[tokio::test]
async fn a_nonce_off_by_one_bit_is_wrong() {
    for bit in 0..NONCE_LEN * 8 {
        let (now, faults) = ask_hostile_with(&[id(1)], |mut n| {
            n[bit / 8] ^= 1 << (bit % 8);
            answer_for(n, &[id(1)])
        })
        .await
        .unwrap();
        assert_eq!(now[&id(1)], OriginalNow::CannotLook, "bit {bit}");
        assert_eq!(faults.len(), 1, "bit {bit}");
    }
}

#[tokio::test]
async fn an_answer_cut_short_before_its_nonce_is_an_error() {
    // The old shape (no nonce at all) and every cut inside the nonce are damaged messages.
    for keep in 0..5 + NONCE_LEN {
        let r = ask_hostile(&[id(1)], |n| answer_for(n, &[id(1)])[..keep].to_vec()).await;
        assert!(
            matches!(r, Err(TransferError::Damaged(_))),
            "keep {keep}: {r:?}"
        );
    }
}

/// Plays the old laptop for two requests: the first answered honestly (and kept), the second
/// answered with the first's reply replayed, or with a fresh honest one.
async fn two_requests(replay: bool) -> (HashMap<ItemId, OriginalNow>, Vec<OriginalsFault>) {
    let items: Vec<ItemId> = (0..MAX_ORIGINALS_PER_REQUEST + 1)
        .map(|i| ItemId::from_hex(&format!("{i:032x}")).unwrap())
        .collect();
    let (mut new, mut old) = mem_pair();
    let journal = common::journal();
    let script = async {
        let mut first_reply: Option<Vec<u8>> = None;
        let mut nonces = Vec::new();
        for _ in 0..2 {
            let Message::CheckOriginals { nonce, items } =
                Message::decode(&old.recv().await.unwrap()).unwrap()
            else {
                panic!("not a request")
            };
            nonces.push(nonce);
            let reply = match (&first_reply, replay) {
                (Some(first), true) => first.clone(),
                _ => answer_for(nonce, &items),
            };
            first_reply.get_or_insert_with(|| reply.clone());
            old.send(&reply).await.unwrap();
        }
        nonces
    };
    let (token, nonces) = tokio::join!(check_originals(&mut new, journal, &items), script);
    let token = token.unwrap();
    assert_ne!(nonces[0], nonces[1], "every request has its own nonce");
    assert_ne!(nonces[0], [0; NONCE_LEN]);
    assert_ne!(nonces[1], [0; NONCE_LEN]);
    (flatten(&token, journal, &items), token.faults().to_vec())
}

#[tokio::test]
async fn each_request_has_a_fresh_nonce_and_honest_answers_to_both_are_believed() {
    let (now, faults) = two_requests(false).await;
    assert_eq!(now.len(), MAX_ORIGINALS_PER_REQUEST + 1);
    assert!(now.values().all(|n| *n == present(1)));
    assert!(faults.is_empty());
}

#[tokio::test]
async fn a_reply_to_an_earlier_request_replayed_later_confirms_nothing() {
    let (now, faults) = two_requests(true).await;
    // The first request was answered honestly; the second got the first's reply again.
    let believed = now.values().filter(|n| **n == present(1)).count();
    assert_eq!(believed, MAX_ORIGINALS_PER_REQUEST);
    let refused = now
        .values()
        .filter(|n| **n == OriginalNow::CannotLook)
        .count();
    assert_eq!(refused, 1);
    assert_eq!(faults, vec![OriginalsFault::WrongNonce { items: 1 }]);
}

#[tokio::test]
async fn the_old_laptop_echoes_the_nonce_it_was_asked_with() {
    let sent = HashMap::new();
    let (mut new, mut old) = mem_pair();
    let nonce: Nonce = [0x5C; NONCE_LEN];
    new.send(
        &Message::CheckOriginals {
            nonce,
            items: vec![id(1)],
        }
        .encode(),
    )
    .await
    .unwrap();
    let mut budget = OriginalsBudget::new();
    serve_originals(&mut old, &sent, &mut budget).await.unwrap();
    let Message::Originals { nonce: echoed, .. } =
        Message::decode(&new.recv().await.unwrap()).unwrap()
    else {
        panic!("not an answer")
    };
    assert_eq!(echoed, nonce);
}
