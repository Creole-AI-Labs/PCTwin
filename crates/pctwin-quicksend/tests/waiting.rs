//! Waiting sends: approve this delivery only (Security Design Part T, Quick Send Stage 1).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use pctwin_known::{MemoryStore, SecretStore};
use pctwin_quicksend::waiting::{
    DEFAULT_DAYS, Delivery, LEASE, MOST_DAYS, State, WaitingError, WaitingStore,
};
use pctwin_quicksend::{Arrival, Outcome, receive};
use pctwin_transfer::{Channel, ChannelError};
use tokio::sync::mpsc;

const T0: u64 = 1_800_000_000;
const DAY: u64 = 86_400;

struct Mem {
    tx: mpsc::UnboundedSender<Vec<u8>>,
    rx: mpsc::UnboundedReceiver<Vec<u8>>,
}

impl Channel for Mem {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        self.tx.send(data.to_vec()).map_err(|_| ChannelError)
    }
    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        self.rx.recv().await.ok_or(ChannelError)
    }
}

fn pair() -> (Mem, Mem) {
    let (a_tx, b_rx) = mpsc::unbounded_channel();
    let (b_tx, a_rx) = mpsc::unbounded_channel();
    (Mem { tx: a_tx, rx: a_rx }, Mem { tx: b_tx, rx: b_rx })
}

fn chosen(old: &Path) -> Vec<PathBuf> {
    std::fs::create_dir_all(old.join("Trip")).unwrap();
    std::fs::write(old.join("Trip/day1.jpg"), b"sunrise").unwrap();
    std::fs::write(old.join("menu.txt"), b"rice and beans").unwrap();
    vec![old.join("Trip"), old.join("menu.txt")]
}

fn alex() -> (String, [u8; 32]) {
    ("alex".into(), [1; 32])
}

fn bo() -> (String, [u8; 32]) {
    ("bo".into(), [2; 32])
}

/// A store of waiting sends at `root`, keys in a store of their own (returned too).
fn store_at(root: &Path) -> (WaitingStore, Arc<MemoryStore>) {
    let keys = Arc::new(MemoryStore::default());
    (WaitingStore::new(root, keys.clone()), keys)
}

fn new_store(root: &Path) -> WaitingStore {
    store_at(root).0
}

fn journal(dir: &Path) -> pctwin_journal::Journal {
    pctwin_journal::Journal::open(&dir.join("journal.redb")).unwrap()
}

/// Runs a delivery of `id` to `who`, the receiver accepting into `dest` (or declining).
async fn deliver(
    store: &WaitingStore,
    id: &str,
    who: &str,
    dest: Option<&Path>,
    jdir: &Path,
    now: u64,
) -> (Result<pctwin_quicksend::Receipt, WaitingError>, Vec<String>) {
    let (mut a, mut b) = pair();
    let live = pctwin_move::Live::default();
    let (offer, _) = match store.offer_for(id, who, now) {
        Ok(o) => o,
        Err(e) => return (Err(e), Vec::new()),
    };
    let (sent, got) = tokio::join!(store.deliver(id, who, offer, &mut a, &live, now), async {
        let Ok(Arrival::Files(incoming)) = receive(&mut b).await else {
            panic!("files")
        };
        let ids: Vec<String> = incoming.files().iter().map(|f| f.id.to_hex()).collect();
        match dest {
            Some(d) => {
                incoming
                    .accept(&mut b, d, None, &journal(jdir), "1001")
                    .await
                    .unwrap();
            }
            None => incoming.decline(&mut b).await.unwrap(),
        }
        ids
    });
    (sent, got)
}

#[tokio::test]
async fn an_approved_send_is_copied_then_waits_then_delivers_the_copy() {
    let (old, keep, dest, j) = (dirs(), dirs(), dirs(), dirs());
    let store = new_store(keep.path());
    let w = store
        .prepare(&chosen(old.path()), &[alex()], DEFAULT_DAYS, T0, None)
        .unwrap();
    assert_eq!(w.state, State::Waiting);
    assert_eq!(w.files, 2);
    assert_eq!(w.bytes, 7 + 14);
    assert_eq!(w.deadline, T0 + 7 * DAY);
    assert_eq!(store.used_bytes(), 21, "shown as used space");
    assert!(w.newer.is_empty());
    let (sent, _) = deliver(&store, &w.id, "alex", Some(dest.path()), j.path(), T0 + 60).await;
    let r = sent.unwrap();
    assert!(
        r.lines
            .iter()
            .all(|l| matches!(l.outcome, Outcome::Arrived { .. })),
        "{r:?}"
    );
    assert_eq!(
        std::fs::read(dest.path().join("Trip/day1.jpg")).unwrap(),
        b"sunrise"
    );
    assert!(matches!(
        store.get(&w.id).unwrap().recipients[0].delivery,
        Delivery::Delivered {
            arrived: 2,
            needs_action: 0,
            ..
        }
    ));
}

#[tokio::test]
async fn a_changed_source_only_says_review_again_and_the_approved_copy_is_what_goes() {
    let (old, keep, dest, j) = (dirs(), dirs(), dirs(), dirs());
    let store = new_store(keep.path());
    let w = store
        .prepare(&chosen(old.path()), &[alex()], DEFAULT_DAYS, T0, None)
        .unwrap();
    std::fs::write(old.path().join("menu.txt"), b"a new menu, longer").unwrap();
    assert_eq!(
        store.get(&w.id).unwrap().newer,
        vec!["menu.txt".to_string()]
    );
    deliver(&store, &w.id, "alex", Some(dest.path()), j.path(), T0)
        .await
        .0
        .unwrap();
    assert_eq!(
        std::fs::read(dest.path().join("menu.txt")).unwrap(),
        b"rice and beans",
        "the approved version, never swapped"
    );
}

#[tokio::test]
async fn a_copy_that_no_longer_matches_is_never_sent() {
    let (old, keep) = (dirs(), dirs());
    let store = new_store(keep.path());
    let w = store
        .prepare(&chosen(old.path()), &[alex()], DEFAULT_DAYS, T0, None)
        .unwrap();
    let files = keep.path().join(&w.id).join("files");
    let copy = files.join("0.sealed");
    let sealed = std::fs::read(&copy).unwrap();
    let mut changed = sealed.clone();
    changed[3] ^= 0x20;
    std::fs::write(&copy, &changed).unwrap();
    assert!(matches!(
        store.offer_for(&w.id, "alex", T0),
        Err(WaitingError::CopyDamaged)
    ));
    std::fs::write(&copy, &sealed).unwrap();
    assert!(store.offer_for(&w.id, "alex", T0).is_ok());
    // Something added beside the approved copies.
    std::fs::write(files.join("2.sealed"), b"MZ").unwrap();
    assert!(matches!(
        store.offer_for(&w.id, "alex", T0),
        Err(WaitingError::CopyDamaged)
    ));
    std::fs::remove_file(files.join("2.sealed")).unwrap();
    // Two copies swapped between their places.
    let other = std::fs::read(files.join("1.sealed")).unwrap();
    std::fs::write(files.join("1.sealed"), &sealed).unwrap();
    std::fs::write(&copy, &other).unwrap();
    assert!(matches!(
        store.offer_for(&w.id, "alex", T0),
        Err(WaitingError::CopyDamaged)
    ));
    // One taken away.
    std::fs::remove_file(files.join("1.sealed")).unwrap();
    assert!(matches!(
        store.offer_for(&w.id, "alex", T0),
        Err(WaitingError::CopyDamaged)
    ));
}

/// The drive alone says nothing about what is waiting: no name, folder, plain byte or
/// fingerprint of the approved files in any file name or file of the send.
#[test]
fn the_drive_shows_nothing_of_what_is_waiting() {
    let (old, keep) = (dirs(), dirs());
    let store = new_store(keep.path());
    let w = store
        .prepare(&chosen(old.path()), &[alex()], 7, T0, None)
        .unwrap();
    let fingerprints = [
        blake3::hash(b"sunrise").to_hex().to_string(),
        blake3::hash(b"rice and beans").to_hex().to_string(),
    ];
    let mut stack = vec![keep.path().join(&w.id)];
    let mut seen = 0;
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).unwrap() {
            let e = e.unwrap();
            let name = e.file_name().to_string_lossy().to_lowercase();
            for word in ["menu", "trip", "day1", "jpg", "txt"] {
                assert!(!name.contains(word), "{name}");
            }
            if e.file_type().unwrap().is_dir() {
                stack.push(e.path());
                continue;
            }
            seen += 1;
            let bytes = std::fs::read(e.path()).unwrap();
            let text = String::from_utf8_lossy(&bytes).to_lowercase();
            for word in ["menu", "trip", "day1", "sunrise", "rice", "beans"] {
                assert!(!text.contains(word), "{word} in {name}");
            }
            for fp in &fingerprints {
                assert!(!text.contains(fp.as_str()), "a fingerprint in {name}");
            }
        }
    }
    assert!(seen >= 4, "the record, the list and two copies");
    // With the key, everything is still known.
    let info = store.get(&w.id).unwrap();
    assert_eq!((info.files, info.bytes), (2, 21));
}

/// A delivery's result and a recipient forgotten at the same moment: neither is lost.
#[tokio::test]
async fn a_recipient_forgotten_during_a_delivery_is_not_written_back() {
    let (old, keep, dest, j) = (dirs(), dirs(), dirs(), dirs());
    let store = new_store(keep.path());
    let w = store
        .prepare(&chosen(old.path()), &[alex(), bo()], 7, T0, None)
        .unwrap();
    let (offer, _) = store.offer_for(&w.id, "alex", T0).unwrap();
    let (mut a, mut b) = pair();
    let live = pctwin_move::Live::default();
    let (sent, ()) = tokio::join!(
        store.deliver(&w.id, "alex", offer, &mut a, &live, T0),
        async {
            let Ok(Arrival::Files(incoming)) = receive(&mut b).await else {
                panic!("files")
            };
            // Bo is forgotten while Alex's delivery runs.
            assert_eq!(store.forget_recipient("bo"), 1);
            incoming
                .accept(&mut b, dest.path(), None, &journal(j.path()), "1001")
                .await
                .unwrap();
        }
    );
    sent.unwrap();
    let r = store.get(&w.id).unwrap().recipients;
    assert_eq!(r.len(), 1, "Bo stays forgotten: {r:?}");
    assert!(matches!(r[0].delivery, Delivery::Delivered { .. }));
}

#[test]
fn the_cap_and_free_space_are_checked_before_anything_is_copied() {
    let (old, keep) = (dirs(), dirs());
    let store = new_store(keep.path()).with_cap(20);
    let c = chosen(old.path());
    assert!(matches!(
        store.prepare(&c, &[alex()], 7, T0, None),
        Err(WaitingError::OverCap {
            needed: 21,
            room: 20
        })
    ));
    let store = new_store(keep.path());
    assert!(matches!(
        store.prepare(&c, &[alex()], 7, T0, Some(1000)),
        Err(WaitingError::NoRoom { needed: 21, .. })
    ));
    assert!(store.list().is_empty());
    assert_eq!(
        std::fs::read_dir(keep.path()).unwrap().count(),
        0,
        "nothing copied"
    );
}

#[test]
fn the_deadline_is_bounded_renewed_only_when_asked_and_ends_it() {
    let (old, keep) = (dirs(), dirs());
    let store = new_store(keep.path());
    let c = chosen(old.path());
    for bad in [0, MOST_DAYS + 1] {
        assert!(matches!(
            store.prepare(&c, &[alex()], bad, T0, None),
            Err(WaitingError::Days)
        ));
    }
    let w = store.prepare(&c, &[alex()], 7, T0, None).unwrap();
    let end = T0 + 7 * DAY;
    assert!(store.offer_for(&w.id, "alex", end - 1).is_ok());
    assert!(matches!(
        store.offer_for(&w.id, "alex", end),
        Err(WaitingError::PastDeadline)
    ));
    assert!(store.sweep(end - 1).is_empty());
    store.renew(&w.id, 3, end).unwrap();
    assert!(store.offer_for(&w.id, "alex", end).is_ok());
    assert_eq!(store.sweep(end + 3 * DAY), vec![w.id.clone()]);
    assert!(
        !keep.path().join(&w.id).exists(),
        "the copy is deleted at the deadline"
    );
    assert_eq!(store.used_bytes(), 0);
}

#[test]
fn cancel_deletes_the_copy() {
    let (old, keep) = (dirs(), dirs());
    let store = new_store(keep.path());
    let w = store
        .prepare(&chosen(old.path()), &[alex()], 7, T0, None)
        .unwrap();
    store.cancel(&w.id).unwrap();
    assert!(!keep.path().join(&w.id).exists());
    assert_eq!(store.used_bytes(), 0);
    assert!(matches!(store.cancel(&w.id), Err(WaitingError::NotFound)));
    assert!(matches!(
        store.cancel("../../etc"),
        Err(WaitingError::NotFound)
    ));
}

#[tokio::test]
async fn several_recipients_each_get_their_own_session_result_and_offer_ids() {
    let (old, keep, d1, d2, j1, j2) = (dirs(), dirs(), dirs(), dirs(), dirs(), dirs());
    let store = new_store(keep.path());
    let w = store
        .prepare(&chosen(old.path()), &[alex(), bo()], 7, T0, None)
        .unwrap();
    assert_eq!(w.recipients.len(), 2);
    let (_, ids_alex) = deliver(&store, &w.id, "alex", Some(d1.path()), j1.path(), T0).await;
    let (_, ids_bo) = deliver(&store, &w.id, "bo", None, j2.path(), T0).await;
    for id in &ids_alex {
        assert!(
            !ids_bo.contains(id),
            "recipients cannot tell they got the same send"
        );
    }
    let r = store.get(&w.id).unwrap().recipients;
    assert!(matches!(r[0].delivery, Delivery::Delivered { .. }));
    assert!(matches!(r[1].delivery, Delivery::Declined { .. }));
    // Someone not on the approval gets nothing.
    assert!(matches!(
        store.offer_for(&w.id, "cy", T0),
        Err(WaitingError::NotFound)
    ));
    let _ = d2;
}

#[test]
fn an_approval_names_each_recipient_once() {
    let (old, keep) = (dirs(), dirs());
    let store = new_store(keep.path());
    let c = chosen(old.path());
    for bad in [
        vec![],
        vec![alex(), alex()],
        vec![alex(), ("other".into(), [1; 32])],
        (0..21).map(|i| (format!("c{i}"), [i as u8; 32])).collect(),
    ] {
        assert!(matches!(
            store.prepare(&c, &bad, 7, T0, None),
            Err(WaitingError::Recipients)
        ));
    }
}

#[test]
fn forgetting_a_connection_ends_its_deliveries() {
    let (old, keep) = (dirs(), dirs());
    let store = new_store(keep.path());
    let both = store
        .prepare(&chosen(old.path()), &[alex(), bo()], 7, T0, None)
        .unwrap();
    let only = store
        .prepare(&chosen(old.path()), &[alex()], 7, T0, None)
        .unwrap();
    assert_eq!(store.forget_recipient("alex"), 2);
    assert_eq!(store.get(&both.id).unwrap().recipients.len(), 1);
    assert!(
        store.get(&only.id).is_err(),
        "a send left with nobody is deleted"
    );
    assert!(!keep.path().join(&only.id).exists());
}

/// A send left Preparing (PCTwin stopped while copying) is never offered or listed, and is tidied
/// away only once its lease ran out: a long copy still running renews its lease and is never
/// deleted from under it, and listing deletes nothing.
#[test]
fn a_send_left_preparing_is_removed_only_when_its_lease_ran_out() {
    let (old, keep) = (dirs(), dirs());
    let store = new_store(keep.path());
    let w = store
        .prepare(&chosen(old.path()), &[alex()], 7, T0, None)
        .unwrap();
    let manifest = keep.path().join(&w.id).join("waiting.json");
    let text = std::fs::read_to_string(&manifest).unwrap();
    let lease = |until: u64| {
        let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
        v["state"] = "preparing".into();
        v["lease_until"] = until.into();
        std::fs::write(&manifest, v.to_string()).unwrap();
    };
    lease(T0 + LEASE);
    assert!(store.offer_for(&w.id, "alex", T0).is_err());
    assert!(store.list().is_empty());
    assert!(keep.path().join(&w.id).exists(), "listing deletes nothing");
    assert!(store.sweep(T0 + LEASE - 1).is_empty(), "still being copied");
    assert!(keep.path().join(&w.id).exists());
    assert_eq!(store.sweep(T0 + LEASE), vec![w.id.clone()]);
    assert!(!keep.path().join(&w.id).exists());
}

/// A send folder whose record is not written yet (a copy just beginning) is left alone.
#[test]
fn a_send_just_begun_is_not_swept() {
    let keep = dirs();
    let store = new_store(keep.path());
    let fresh = keep.path().join("00112233445566778899aabbccddeeff");
    std::fs::create_dir_all(fresh.join("files")).unwrap();
    assert!(store.sweep(T0).is_empty());
    assert!(fresh.exists());
}

/// A copy that takes long renews its lease as it goes, so a tidy during it deletes nothing.
#[test]
fn a_long_copy_keeps_renewing_its_lease() {
    use std::sync::atomic::{AtomicU64, Ordering};
    let (old, keep) = (dirs(), dirs());
    let (store, _) = store_at(keep.path());
    std::fs::create_dir_all(old.path().join("Big")).unwrap();
    for n in 0..40 {
        std::fs::write(
            old.path().join(format!("Big/part{n}.bin")),
            vec![n as u8; 70_000],
        )
        .unwrap();
    }
    // Each read of the clock is a minute later: the copy runs for about an hour.
    let t = AtomicU64::new(T0);
    let swept = std::cell::RefCell::new(Vec::new());
    let clock = || {
        let now = t.fetch_add(60, Ordering::Relaxed);
        // A tidy runs alongside, at the time the copy has reached.
        swept.borrow_mut().extend(store.sweep(now));
        now
    };
    let w = store
        .prepare_with_clock(&[old.path().join("Big")], &[alex()], 7, T0, None, &clock)
        .unwrap();
    assert!(t.load(Ordering::Relaxed) > T0 + 2 * LEASE, "a long copy");
    assert!(swept.borrow().is_empty(), "{:?}", swept.borrow());
    assert_eq!(store.get(&w.id).unwrap().files, 40);
}

/// Each send's copy is sealed with a key of its own, kept in the key store and deleted with the
/// send: once cancelled, swept or forgotten, nothing of it stays readable.
#[tokio::test]
async fn each_send_has_its_own_key_in_the_store_deleted_with_it() {
    let (old, keep, dest, j) = (dirs(), dirs(), dirs(), dirs());
    let (store, keys) = store_at(keep.path());
    let a = store
        .prepare(&chosen(old.path()), &[alex()], 7, T0, None)
        .unwrap();
    let b = store
        .prepare(&chosen(old.path()), &[alex(), bo()], 7, T0, None)
        .unwrap();
    let names = keys.names();
    assert_eq!(names.len(), 2, "{names:?}");
    let ka = keys.get(&format!("waiting-{}-key", a.id)).unwrap().unwrap();
    let kb = keys.get(&format!("waiting-{}-key", b.id)).unwrap().unwrap();
    assert_ne!(*ka, *kb);
    // Delivered through the seal, the plain bytes arrive.
    deliver(&store, &a.id, "alex", Some(dest.path()), j.path(), T0)
        .await
        .0
        .unwrap();
    assert_eq!(
        std::fs::read(dest.path().join("menu.txt")).unwrap(),
        b"rice and beans"
    );
    // Without its key the copy is refused, never sent.
    keys.delete(&format!("waiting-{}-key", b.id)).unwrap();
    assert!(matches!(
        store.offer_for(&b.id, "bo", T0),
        Err(WaitingError::CopyDamaged)
    ));
    store.cancel(&a.id).unwrap();
    assert!(keys.names().is_empty(), "{:?}", keys.names());
}

/// Cancel marks the send first: a delivery that was about to start finds nothing to send, and a
/// delivery in flight records nothing afterwards.
#[tokio::test]
async fn a_cancelled_send_is_never_delivered_again() {
    let (old, keep, dest, j) = (dirs(), dirs(), dirs(), dirs());
    let store = new_store(keep.path());
    let w = store
        .prepare(&chosen(old.path()), &[alex()], 7, T0, None)
        .unwrap();
    let (offer, _) = store.offer_for(&w.id, "alex", T0).unwrap();
    assert!(store.due(&w.id, "alex", T0));
    store.cancel(&w.id).unwrap();
    assert!(!store.due(&w.id, "alex", T0));
    assert!(store.offer_for(&w.id, "alex", T0).is_err());
    // An offer taken before the cancel cannot read the copy any more.
    let (mut a, mut b) = pair();
    let live = pctwin_move::Live::default();
    let (sent, _) = tokio::join!(
        store.deliver(&w.id, "alex", offer, &mut a, &live, T0),
        async {
            if let Ok(Arrival::Files(incoming)) = receive(&mut b).await {
                let _ = incoming
                    .accept(&mut b, dest.path(), None, &journal(j.path()), "1001")
                    .await;
            }
        }
    );
    if let Ok(r) = sent {
        assert!(
            r.lines
                .iter()
                .all(|l| !matches!(l.outcome, Outcome::Arrived { .. })),
            "{r:?}"
        );
    }
    assert!(!dest.path().join("menu.txt").exists());
    assert!(store.list().is_empty());
}

fn dirs() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

/// The record's file count must match the sealed list.
#[test]
fn a_record_that_disagrees_with_its_sealed_list_is_not_sent() {
    let (old, keep) = (dirs(), dirs());
    let store = new_store(keep.path());
    let w = store
        .prepare(&chosen(old.path()), &[alex()], 7, T0, None)
        .unwrap();
    let manifest = keep.path().join(&w.id).join("waiting.json");
    let mut v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&manifest).unwrap()).unwrap();
    v["files"] = 3.into();
    std::fs::write(&manifest, v.to_string()).unwrap();
    assert!(matches!(
        store.offer_for(&w.id, "alex", T0),
        Err(WaitingError::CopyDamaged)
    ));
}

/// A delivery that ends after the send was cancelled writes nothing back into its record.
#[tokio::test]
async fn a_delivery_ending_after_cancel_records_nothing() {
    let (old, keep, dest, j) = (dirs(), dirs(), dirs(), dirs());
    let store = new_store(keep.path());
    let w = store
        .prepare(&chosen(old.path()), &[alex()], 7, T0, None)
        .unwrap();
    let (offer, _) = store.offer_for(&w.id, "alex", T0).unwrap();
    store.mark_cancelled(&w.id).unwrap();
    let (mut a, mut b) = pair();
    let live = pctwin_move::Live::default();
    let (sent, ()) = tokio::join!(
        store.deliver(&w.id, "alex", offer, &mut a, &live, T0),
        async {
            if let Ok(Arrival::Files(incoming)) = receive(&mut b).await {
                let _ = incoming
                    .accept(&mut b, dest.path(), None, &journal(j.path()), "1001")
                    .await;
            }
        }
    );
    let _ = sent;
    let text = std::fs::read_to_string(keep.path().join(&w.id).join("waiting.json")).unwrap();
    assert!(text.contains("\"cancelled\""), "{text}");
    assert!(!text.contains("delivered"), "{text}");
}

/// A send cancelled (or its last recipient forgotten) while it is still being copied never turns
/// into a waiting send.
#[test]
fn a_send_cancelled_while_copying_never_starts_waiting() {
    let (old, keep) = (dirs(), dirs());
    let store = new_store(keep.path());
    let root = keep.path().to_path_buf();
    let clock = || {
        // Cancel whatever is being prepared, as Cancel or a Forget would.
        if let Ok(entries) = std::fs::read_dir(&root) {
            for e in entries.flatten() {
                let id = e.file_name().to_string_lossy().into_owned();
                let _ = store.mark_cancelled(&id);
            }
        }
        T0
    };
    let got = store.prepare_with_clock(&chosen(old.path()), &[alex()], 7, T0, None, &clock);
    assert!(got.is_err());
    assert!(store.list().is_empty());
}

/// A source rewritten at the same length, later: a newer version exists (said, never sent).
#[test]
fn a_source_rewritten_at_the_same_length_is_newer() {
    let (old, keep) = (dirs(), dirs());
    let store = new_store(keep.path());
    let c = chosen(old.path());
    let w = store.prepare(&c, &[alex()], 7, T0, None).unwrap();
    assert!(store.get(&w.id).unwrap().newer.is_empty());
    let menu = old.path().join("menu.txt");
    let was = std::fs::metadata(&menu).unwrap().modified().unwrap();
    std::fs::write(&menu, b"rice and BEANS").unwrap();
    let f = std::fs::File::options().write(true).open(&menu).unwrap();
    f.set_modified(was + std::time::Duration::from_secs(60))
        .unwrap();
    drop(f);
    assert_eq!(std::fs::metadata(&menu).unwrap().len(), 14);
    assert_eq!(
        store.get(&w.id).unwrap().newer,
        vec!["menu.txt".to_string()]
    );
}

/// A source rewritten at the same length while it is being copied ends the approval: nothing
/// is kept.
#[test]
fn a_source_rewritten_at_the_same_length_during_the_copy_ends_it() {
    use std::sync::atomic::{AtomicU32, Ordering};
    let (old, keep) = (dirs(), dirs());
    let store = new_store(keep.path());
    let big = old.path().join("big.bin");
    std::fs::write(&big, vec![1u8; 200_000]).unwrap();
    let calls = AtomicU32::new(0);
    let clock = || {
        // Part way through the copy: the same length, a later time.
        if calls.fetch_add(1, Ordering::Relaxed) == 3 {
            let was = std::fs::metadata(&big).unwrap().modified().unwrap();
            std::fs::write(&big, vec![2u8; 200_000]).unwrap();
            let f = std::fs::File::options().write(true).open(&big).unwrap();
            f.set_modified(was + std::time::Duration::from_secs(60))
                .unwrap();
        }
        T0
    };
    let got = store.prepare_with_clock(&[big.clone()], &[alex()], 7, T0, None, &clock);
    assert!(
        matches!(got, Err(WaitingError::ChangedWhileCopying(_))),
        "{got:?}"
    );
    assert!(store.list().is_empty());
}
