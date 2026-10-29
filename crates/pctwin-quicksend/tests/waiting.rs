//! Waiting sends: approve this delivery only (Security Design Part T, Quick Send Stage 1).

use std::path::{Path, PathBuf};

use pctwin_quicksend::waiting::{
    DEFAULT_DAYS, Delivery, MOST_DAYS, State, WaitingError, WaitingStore,
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
    let (sent, got) = tokio::join!(store.deliver(id, who, &mut a, &live, now), async {
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
    let store = WaitingStore::new(keep.path());
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
    let store = WaitingStore::new(keep.path());
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
    let store = WaitingStore::new(keep.path());
    let w = store
        .prepare(&chosen(old.path()), &[alex()], DEFAULT_DAYS, T0, None)
        .unwrap();
    let copy = keep.path().join(&w.id).join("files").join("menu.txt");
    std::fs::write(&copy, b"rice and BEANS").unwrap();
    assert!(matches!(
        store.offer_for(&w.id, "alex", T0),
        Err(WaitingError::CopyDamaged)
    ));
    std::fs::write(&copy, b"rice and beans").unwrap();
    assert!(store.offer_for(&w.id, "alex", T0).is_ok());
    // Something added beside the approved files.
    std::fs::write(keep.path().join(&w.id).join("files/Trip/extra.exe"), b"MZ").unwrap();
    assert!(matches!(
        store.offer_for(&w.id, "alex", T0),
        Err(WaitingError::CopyDamaged)
    ));
    std::fs::remove_file(keep.path().join(&w.id).join("files/Trip/extra.exe")).unwrap();
    // One taken away.
    std::fs::remove_file(keep.path().join(&w.id).join("files/Trip/day1.jpg")).unwrap();
    assert!(matches!(
        store.offer_for(&w.id, "alex", T0),
        Err(WaitingError::CopyDamaged)
    ));
}

#[test]
fn the_cap_and_free_space_are_checked_before_anything_is_copied() {
    let (old, keep) = (dirs(), dirs());
    let store = WaitingStore::new(keep.path()).with_cap(20);
    let c = chosen(old.path());
    assert!(matches!(
        store.prepare(&c, &[alex()], 7, T0, None),
        Err(WaitingError::OverCap {
            needed: 21,
            room: 20
        })
    ));
    let store = WaitingStore::new(keep.path());
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
    let store = WaitingStore::new(keep.path());
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
    let store = WaitingStore::new(keep.path());
    let w = store
        .prepare(&chosen(old.path()), &[alex()], 7, T0, None)
        .unwrap();
    store.cancel(&w.id).unwrap();
    assert!(!keep.path().join(&w.id).exists());
    assert!(matches!(store.cancel(&w.id), Err(WaitingError::NotFound)));
    assert!(matches!(
        store.cancel("../../etc"),
        Err(WaitingError::NotFound)
    ));
}

#[tokio::test]
async fn several_recipients_each_get_their_own_session_result_and_offer_ids() {
    let (old, keep, d1, d2, j1, j2) = (dirs(), dirs(), dirs(), dirs(), dirs(), dirs());
    let store = WaitingStore::new(keep.path());
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
    let store = WaitingStore::new(keep.path());
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
    let store = WaitingStore::new(keep.path());
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

#[test]
fn a_send_left_preparing_is_removed_not_offered() {
    let (old, keep) = (dirs(), dirs());
    let store = WaitingStore::new(keep.path());
    let w = store
        .prepare(&chosen(old.path()), &[alex()], 7, T0, None)
        .unwrap();
    let manifest = keep.path().join(&w.id).join("waiting.json");
    let text = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(&manifest, text.replace("\"waiting\"", "\"preparing\"")).unwrap();
    assert!(store.offer_for(&w.id, "alex", T0).is_err());
    assert!(store.list().is_empty());
    assert!(!keep.path().join(&w.id).exists());
}

fn dirs() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}
