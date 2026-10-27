//! The inventory exchange (Task List 1.5): after the number match, the old laptop sends its scanned
//! record over the sealed connection, compressed and in pieces that each fit the link. The new
//! laptop treats it as coming from a stranger (Security Design A: someone holding the code could
//! pose as the old laptop): sizes are checked before anything is held or unpacked, every piece
//! must add up to exactly what was announced, the unpacked record must be exactly the announced
//! size, and the record must pass every rule a saved one does. The old laptop never decides where
//! people land on the new laptop, so a record that arrives with those choices made is refused.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use pctwin_record::{
    FolderRole, Inclusion, Item, ItemId, ItemKind, ItemName, ItemPath, LaptopId, ManagedBy, Owner,
    PersonTarget, Place, Portability, Record, Storage,
};

use pctwin_transfer::{
    Channel, ChannelError, INVENTORY_PIECE_WAIT, InventoryFrame, Message, PIECE_MAX, TransferError,
    receive_inventory, send_inventory,
};
use tokio::sync::mpsc;

mod common;

/// One end of an in-memory connection that counts what it has read.
struct Mem {
    tx: mpsc::UnboundedSender<Vec<u8>>,
    rx: mpsc::UnboundedReceiver<Vec<u8>>,
    read: Arc<AtomicUsize>,
}

impl Channel for Mem {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        self.tx.send(data.to_vec()).map_err(|_| ChannelError)
    }
    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        let m = self.rx.recv().await.ok_or(ChannelError)?;
        self.read.fetch_add(1, Ordering::SeqCst);
        Ok(m)
    }
}

fn pair() -> (Mem, Mem) {
    let (a_tx, b_rx) = mpsc::unbounded_channel();
    let (b_tx, a_rx) = mpsc::unbounded_channel();
    let end = |tx, rx| Mem {
        tx,
        rx,
        read: Arc::new(AtomicUsize::new(0)),
    };
    (end(a_tx, a_rx), end(b_tx, b_rx))
}

/// A scanned record with `n` files, as the old laptop makes it: no choices about people.
fn record(n: usize) -> Record {
    let laptop = LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap();
    let mut record = Record::new(laptop);
    for i in 0..n {
        let mut id = [0u8; 16];
        id[..8].copy_from_slice(&(i as u64).to_be_bytes());
        record.items.push(Item {
            id: ItemId::from_hex(&hex(&id)).unwrap(),
            kind: ItemKind::File,
            owner: Owner::Person {
                account_id: "1000".into(),
            },
            place: Place {
                role: FolderRole::Documents,
                storage: Storage::SystemDrive,
            },
            path: ItemPath::new(vec![
                ItemName::from_text("Projects"),
                ItemName::from_text(&format!("report {i} draft.docx")),
            ])
            .unwrap(),
            size_bytes: 1000 + i as u64,
            modified_ns: None,
            portability: Portability::Portable,
            managed_by: ManagedBy::Personal,
            download_bytes: None,
            inclusion: Inclusion::Included,
        });
    }
    record
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn start(raw_len: u64, packed_len: u64) -> Vec<u8> {
    InventoryFrame::Start {
        raw_len,
        packed_len,
    }
    .encode()
}

fn part(bytes: &[u8]) -> Vec<u8> {
    InventoryFrame::Part(bytes.to_vec()).encode()
}

fn refused(got: Result<Record, TransferError>) -> String {
    match got {
        Err(TransferError::Inventory(why)) => why,
        other => panic!("expected the inventory to be refused, got {other:?}"),
    }
}

#[tokio::test]
async fn a_record_arrives_exactly_as_it_was_sent() {
    let sent = record(3);
    let (mut old, mut new) = pair();
    let (s, r) = tokio::join!(
        send_inventory(&mut old, &sent, PAGED),
        receive_inventory(&mut new, PAGED)
    );
    s.unwrap();
    let got = r.unwrap();
    assert_eq!(got, as_sent(&sent));
}

#[tokio::test]
async fn a_large_record_travels_in_pieces_that_each_fit_the_link() {
    // Unique names barely compress, so this needs many pieces.
    let sent = record(40_000);
    let (mut old, mut new) = pair();
    let (s, r) = tokio::join!(
        send_inventory(&mut old, &sent, PAGED),
        receive_inventory(&mut new, PAGED)
    );
    s.unwrap();
    assert_eq!(r.unwrap(), as_sent(&sent));
    assert!(
        new.read.load(Ordering::SeqCst) > 3,
        "expected several pieces"
    );
}

#[tokio::test]
async fn every_message_of_the_exchange_fits_the_link() {
    let sent = record(40_000);
    let (mut old, mut new) = pair();
    send_inventory(&mut old, &sent, PAGED).await.unwrap();
    drop(old);
    while let Ok(m) = new.recv().await {
        assert!(
            m.len() <= u16::MAX as usize,
            "a message of {} bytes",
            m.len()
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_record_crosses_a_real_paired_link() {
    let config = pctwin_link::LinkConfig {
        step_timeout: Duration::from_secs(5),
        handshake_timeout: Duration::from_secs(5),
        silence_penalty: Duration::ZERO,
        connect_timeout: Duration::from_secs(5),
    };
    let host = pctwin_link::Host::bind("127.0.0.1:0".parse().unwrap(), config)
        .await
        .unwrap();
    let addr = host.local_addr().unwrap();
    let rotating = Arc::new(std::sync::Mutex::new(
        pctwin_pairing::RotatingSender::new(Instant::now()).unwrap(),
    ));
    let code = {
        let mut s = rotating.lock().unwrap();
        s.tick(Instant::now()).unwrap();
        pctwin_pairing::PairingCode::parse(&s.code().unwrap()).unwrap()
    };
    let guest =
        tokio::spawn(async move { pctwin_link::connect(addr, &code, config).await.unwrap() });
    let pending_host = host.next_peer(&rotating).await.unwrap();
    let pending_guest = guest.await.unwrap();
    let number = pending_guest.match_number();
    // The old laptop pairs once the new laptop confirms, so both run together.
    let (old_link, new_link) = tokio::join!(
        pending_host.choose(number, Instant::now()),
        pending_guest.approval()
    );
    let (mut old_link, mut new_link) = (old_link.unwrap(), new_link.unwrap());

    let sent = record(20_000);
    let (s, r) = common::no_hang(async {
        tokio::join!(
            send_inventory(&mut old_link, &sent, PAGED),
            receive_inventory(&mut new_link, PAGED)
        )
    })
    .await;
    s.unwrap();
    assert_eq!(r.unwrap(), as_sent(&sent));
}

#[tokio::test]
async fn a_transfer_message_in_place_of_the_inventory_is_refused() {
    let (got, read) = hostile_paged_counted(vec![Message::Ready.encode()]).await;
    refused(got);
    assert_eq!(read, 1);
}

#[tokio::test]
async fn a_transfer_message_between_pieces_is_refused() {
    let mut frames = paged_frames(&record(40_000));
    assert!(frames.len() > 2);
    frames.insert(2, Message::AllSent.encode());
    let (got, read) = hostile_paged_counted(frames).await;
    refused(got);
    assert_eq!(read, 3);
}

#[tokio::test]
async fn nothing_arriving_is_a_dropped_connection() {
    let got = hostile_paged(Vec::new()).await;
    assert!(
        matches!(got, Err(TransferError::ConnectionDropped)),
        "{got:?}"
    );
}

#[test]
fn the_frames_on_the_wire_stay_exactly_as_released() {
    // Kinds 32 and 33, sizes as big-endian 64-bit numbers: older and newer apps must agree.
    let s = InventoryFrame::Start {
        raw_len: 0x0102,
        packed_len: 0x0304,
    };
    assert_eq!(
        s.encode(),
        [32, 0, 0, 0, 0, 0, 0, 1, 2, 0, 0, 0, 0, 0, 0, 3, 4]
    );
    assert_eq!(InventoryFrame::Part(vec![9, 8]).encode(), [33, 9, 8]);
}

#[test]
fn frames_decode_to_what_was_encoded_and_reject_damage() {
    let s = InventoryFrame::Start {
        raw_len: 12,
        packed_len: 34,
    };
    assert_eq!(InventoryFrame::decode(&s.encode()).unwrap(), s);
    let p = InventoryFrame::Part(vec![1, 2, 3]);
    assert_eq!(InventoryFrame::decode(&p.encode()).unwrap(), p);
    // Cut short, too long, empty, or of another kind.
    let enc = s.encode();
    assert!(InventoryFrame::decode(&enc[..enc.len() - 1]).is_err());
    let mut long = enc.clone();
    long.push(0);
    assert!(InventoryFrame::decode(&long).is_err());
    assert!(InventoryFrame::decode(&[]).is_err());
    assert!(InventoryFrame::decode(&Message::Ready.encode()).is_err());
    // The transfer never mistakes an inventory frame for one of its own.
    assert!(Message::decode(&enc).is_err());
    assert!(Message::decode(&p.encode()).is_err());
}

fn no_answer(got: &Result<Record, TransferError>) -> bool {
    matches!(got, Err(TransferError::NoAnswer(_)))
}

#[tokio::test(start_paused = true)]
async fn an_old_laptop_that_never_starts_is_given_up_on() {
    let (_old, mut new) = pair();
    let t = tokio::time::Instant::now();
    let got = receive_inventory(&mut new, PAGED).await;
    assert!(no_answer(&got), "{got:?}");
    assert!(t.elapsed() >= INVENTORY_PIECE_WAIT, "{:?}", t.elapsed());
    assert!(t.elapsed() < INVENTORY_PIECE_WAIT + Duration::from_secs(1));
}

#[tokio::test(start_paused = true)]
async fn an_old_laptop_that_stops_part_way_is_given_up_on() {
    let frames = paged_frames(&record(40_000));
    let (mut old, mut new) = pair();
    for f in &frames[..2] {
        old.send(f).await.unwrap();
    }
    let t = tokio::time::Instant::now();
    let got = receive_inventory(&mut new, PAGED).await;
    assert!(no_answer(&got), "{got:?}");
    assert!(t.elapsed() < INVENTORY_PIECE_WAIT + Duration::from_secs(1));
    drop(old);
}

fn steady(pieces: u64, every: Duration) -> Mem {
    let (mut old, new) = pair();
    tokio::spawn(async move {
        let n = pieces * PIECE_MAX as u64;
        old.send(&paged_head(u32::MAX, n as u32)).await.unwrap();
        for _ in 0..pieces {
            tokio::time::sleep(every).await;
            if old.send(&part(&vec![0u8; PIECE_MAX])).await.is_err() {
                return;
            }
        }
        std::future::pending::<()>().await;
    });
    new
}

/// A slow link is not a stall: an old laptop that sends a full piece just inside the wait each
/// time is never cut off, however long the whole list takes (it is then refused here only
/// because its bytes are not a record).
#[tokio::test(start_paused = true)]
async fn a_slow_but_steady_old_laptop_is_never_cut_off() {
    let pieces = 10;
    let mut new = steady(pieces, INVENTORY_PIECE_WAIT - Duration::from_secs(1));
    let t = tokio::time::Instant::now();
    let got = receive_inventory(&mut new, PAGED).await;
    refused(got);
    assert!(t.elapsed() >= (INVENTORY_PIECE_WAIT - Duration::from_secs(1)) * pieces as u32);
}

/// A piece that comes later than the wait ends the exchange, even after many on time.
#[tokio::test(start_paused = true)]
async fn one_piece_later_than_the_wait_is_given_up_on() {
    let mut new = steady(3, INVENTORY_PIECE_WAIT + Duration::from_secs(1));
    let got = receive_inventory(&mut new, PAGED).await;
    assert!(no_answer(&got), "{got:?}");
}

// ---------- move protocol 3: the list in pages (scan inventory version 2) ----------

/// The move protocol that sends the list in pages.
const PAGED: u16 = pctwin_transfer::PAGED_FROM;

/// A paged header: which piece of the list follows (the manifest is `u32::MAX`) and its length.
fn paged_head(index: u32, len: u32) -> Vec<u8> {
    let mut h = vec![35];
    h.extend_from_slice(&index.to_be_bytes());
    h.extend_from_slice(&len.to_be_bytes());
    h
}

/// What a well-behaved old laptop sends for `record`, as frames.
fn paged_frames(record: &Record) -> Vec<Vec<u8>> {
    let enc = pctwin_record::encode_inventory(&record.source_laptop, &record.items).unwrap();
    let mut frames = Vec::new();
    let mut blob = |index: u32, bytes: &[u8]| {
        frames.push(paged_head(index, bytes.len() as u32));
        for piece in bytes.chunks(PIECE_MAX) {
            frames.push(part(piece));
        }
    };
    blob(u32::MAX, &enc.manifest);
    for (n, page) in enc.pages.iter().enumerate() {
        blob(n as u32, page);
    }
    frames
}

async fn hostile_paged(frames: Vec<Vec<u8>>) -> Result<Record, TransferError> {
    let (mut old, mut new) = pair();
    for f in &frames {
        old.send(f).await.unwrap();
    }
    drop(old);
    receive_inventory(&mut new, PAGED).await
}

/// The record the new laptop builds from `sent`'s pages: the same, naming the list that crossed.
fn as_sent(sent: &Record) -> Record {
    let mut want = sent.clone();
    want.inventory = Some(
        pctwin_record::encode_inventory(&sent.source_laptop, &sent.items)
            .unwrap()
            .fingerprint(),
    );
    want
}

#[tokio::test]
async fn a_large_list_crosses_in_pages_and_arrives_exactly() {
    let sent = record(25_000);
    let (mut old, mut new) = pair();
    let (s, r) = tokio::join!(
        send_inventory(&mut old, &sent, PAGED),
        receive_inventory(&mut new, PAGED)
    );
    s.unwrap();
    assert_eq!(r.unwrap(), as_sent(&sent));
}

#[tokio::test]
async fn the_paged_list_is_what_a_good_sender_sends_frame_for_frame() {
    let sent = record(12_000);
    let frames = paged_frames(&sent);
    assert_eq!(hostile_paged(frames).await.unwrap(), as_sent(&sent));
}

#[tokio::test]
async fn a_paged_list_that_breaks_a_rule_on_the_way_is_refused() {
    let sent = record(12_000);
    let good = paged_frames(&sent);
    let refused = |r: Result<Record, TransferError>| {
        assert!(matches!(r, Err(TransferError::Inventory(_))), "{r:?}");
    };
    // Missing its last frame: the list stops arriving.
    let mut cut = good.clone();
    cut.pop();
    assert!(hostile_paged(cut).await.is_err());
    // A page announced under another number.
    let mut renumbered = good.clone();
    let i = renumbered
        .iter()
        .position(|f| f.len() == 9 && f[0] == 35 && f[1..5] == 0u32.to_be_bytes())
        .unwrap();
    renumbered[i][1..5].copy_from_slice(&1u32.to_be_bytes());
    refused(hostile_paged(renumbered).await);
    // A page announced larger than a page may be.
    let mut big = vec![paged_head(
        u32::MAX,
        (pctwin_record::MAX_PAGE_BYTES + 1) as u32,
    )];
    big.push(part(&[0; 10]));
    refused(hostile_paged(big).await);
    // An empty manifest.
    refused(hostile_paged(vec![paged_head(u32::MAX, 0)]).await);
    // A short piece before the last.
    let manifest = pctwin_record::encode_inventory(&sent.source_laptop, &sent.items)
        .unwrap()
        .manifest;
    let half = manifest.len() / 2;
    let short = vec![
        paged_head(u32::MAX, manifest.len() as u32),
        part(&manifest[..half]),
        part(&manifest[half..]),
    ];
    if manifest.len() > PIECE_MAX {
        refused(hostile_paged(short).await);
    }
    // More of a piece than announced.
    let over = vec![paged_head(u32::MAX, 4), part(&[1, 2, 3, 4, 5])];
    refused(hostile_paged(over).await);
    // The old JSON start where pages are expected.
    refused(hostile_paged(vec![start(10, 10)]).await);
}

#[tokio::test]
async fn the_old_laptop_never_sends_choices_about_people_in_pages_either() {
    let mut decided = record(3);
    decided.mapping.people.insert(
        "1000".into(),
        PersonTarget::ExistingAccount {
            account_id: "ada".into(),
        },
    );
    let (mut old, _new) = pair();
    let got = send_inventory(&mut old, &decided, PAGED).await;
    assert!(matches!(got, Err(TransferError::Inventory(_))), "{got:?}");
}

#[tokio::test]
async fn an_empty_list_is_refused_in_pages_as_in_json() {
    let empty = record(0);
    let (mut old, mut new) = pair();
    let (_, r) = tokio::join!(
        send_inventory(&mut old, &empty, PAGED),
        receive_inventory(&mut new, PAGED)
    );
    assert!(r.is_err());
}

/// A laptop one version behind (move protocol 3) and one on this release agree on 3, and the list
/// then crosses in pages without the personal order, as the older laptop expects.
#[tokio::test]
async fn a_laptop_one_version_behind_gets_the_list_it_reads() {
    use pctwin_transfer::{MOVE_PROTOCOL, Versions, agree_version};
    let older = Versions::new(PAGED, PAGED).unwrap();
    let mut sent = record(50);
    sent.personal = vec![sent.items[1].id];
    let (mut old, mut new) = pair();
    let (s, r) = tokio::join!(
        async {
            let v = agree_version(&mut old, MOVE_PROTOCOL).await?;
            assert_eq!(v, PAGED);
            send_inventory(&mut old, &sent, v).await
        },
        async {
            let v = agree_version(&mut new, older).await?;
            receive_inventory(&mut new, v).await
        }
    );
    s.unwrap();
    let mut want = as_sent(&sent);
    want.personal.clear();
    assert_eq!(r.unwrap(), want);
}

/// Review of scan record v2 (24 October 2026): a laptop that says it speaks only move protocol 2
/// cannot force the one-document route, which had none of the paged route's memory bounds: the
/// versions do not meet, and a list asked for on protocol 2 is refused before anything is read.
#[tokio::test]
async fn a_laptop_that_forces_the_old_one_document_route_is_not_met() {
    use pctwin_transfer::{MOVE_PROTOCOL, VersionGap, Versions, agree_version};
    assert_eq!(MOVE_PROTOCOL.oldest(), PAGED);
    let forced = Versions::new(2, 2).unwrap();
    let (mut old, mut new) = pair();
    let (ours, theirs) = tokio::join!(
        agree_version(&mut new, MOVE_PROTOCOL),
        agree_version(&mut old, forced)
    );
    assert!(
        matches!(
            ours,
            Err(TransferError::VersionGap(VersionGap::OtherLaptopOlder))
        ),
        "{ours:?}"
    );
    assert!(theirs.is_err());
    // Asked for directly anyway: refused, with nothing read and nothing sent.
    let (mut old, mut new) = pair();
    old.send(&paged_frames(&record(3))[0]).await.unwrap();
    let got = receive_inventory(&mut new, 2).await;
    assert!(matches!(got, Err(TransferError::VersionGap(_))), "{got:?}");
    assert_eq!(new.read.load(Ordering::SeqCst), 0);
    let got = send_inventory(&mut old, &record(3), 2).await;
    assert!(matches!(got, Err(TransferError::VersionGap(_))), "{got:?}");
}

// ---------- found by the fresh-context review of 20 October 2026 ----------

/// [`hostile_paged`], also returning how many messages the new laptop read.
async fn hostile_paged_counted(frames: Vec<Vec<u8>>) -> (Result<Record, TransferError>, usize) {
    let (mut old, mut new) = pair();
    for f in &frames {
        old.send(f).await.unwrap();
    }
    drop(old);
    let got = receive_inventory(&mut new, PAGED).await;
    (got, new.read.load(Ordering::SeqCst))
}

/// A header that claims more than a manifest or a page may hold is refused on sight: nothing
/// after it is read, so nothing is held.
#[tokio::test]
async fn a_header_claiming_too_much_is_refused_before_anything_is_held() {
    let full = part(&vec![7; PIECE_MAX]);
    // The manifest, one byte over its limit, and the largest a header can claim.
    for len in [
        pctwin_record::inventory::MAX_MANIFEST_BYTES as u32 + 1,
        u32::MAX,
    ] {
        let frames = vec![paged_head(u32::MAX, len), full.clone(), full.clone()];
        let (got, read) = hostile_paged_counted(frames).await;
        refused(got);
        assert_eq!(read, 1, "read past a header claiming {len} bytes");
    }
    // A page, one byte over its limit, after a good manifest.
    let sent = record(3);
    let good = paged_frames(&sent);
    let manifest_frames = good.len() - 2;
    let mut frames = good[..manifest_frames].to_vec();
    frames.push(paged_head(0, pctwin_record::MAX_PAGE_BYTES as u32 + 1));
    frames.push(full.clone());
    frames.push(full);
    let (got, read) = hostile_paged_counted(frames).await;
    refused(got);
    assert_eq!(read, manifest_frames + 1);
}

/// Every piece of a page but its last is full, so how many pieces a page takes is fixed by its
/// length. A page larger than one piece that starts with a short piece is refused, even though
/// its bytes add up.
#[tokio::test]
async fn a_short_piece_before_the_last_of_a_page_is_refused() {
    let sent = record(12_000);
    let enc = pctwin_record::encode_inventory(&sent.source_laptop, &sent.items).unwrap();
    let page = &enc.pages[0];
    assert!(page.len() > 2 * PIECE_MAX, "the page fits one piece");
    let mut frames = vec![paged_head(u32::MAX, enc.manifest.len() as u32)];
    frames.extend(enc.manifest.chunks(PIECE_MAX).map(part));
    frames.push(paged_head(0, page.len() as u32));
    frames.push(part(&page[..PIECE_MAX - 1]));
    frames.extend(page[PIECE_MAX - 1..].chunks(PIECE_MAX).map(part));
    let why = refused(hostile_paged(frames).await);
    assert!(why.contains("not full"), "{why}");
}

/// A list sent in pages passes every rule a saved record does: an item that only works on the
/// old laptop, marked to move, is refused, as it is in JSON.
#[tokio::test]
async fn a_device_bound_item_marked_to_move_is_refused_in_pages() {
    let mut sent = record(3);
    sent.items[1].portability = Portability::DeviceBound;
    let got = hostile_paged(paged_frames(&sent)).await;
    refused(got);
    // The same list with that item left out is read.
    sent.items[1].inclusion = Inclusion::LeftOut {
        reason: pctwin_record::LeftOutReason::DeviceBound,
    };
    assert_eq!(
        hostile_paged(paged_frames(&sent)).await.unwrap(),
        as_sent(&sent)
    );
}

/// A paired sender that keeps sending pages cannot grow the new laptop's memory without limit:
/// what the pages will hold is counted as they arrive, and the list is refused once it passes
/// what an inventory may hold (`pctwin_record::MAX_HELD`), long before its announced pages end.
/// (About 80,000 items 128 parts deep cross first: slow in a debug build, so it runs on request
/// with `cargo test --release -- --ignored`.)
#[tokio::test]
#[ignore = "slow in a debug build; run with --release -- --ignored"]
async fn a_sender_that_keeps_sending_pages_is_stopped_at_the_memory_limit() {
    const ROOT: u32 = u32::MAX;
    let pages = 1_000u32;
    let per_page = 1_500u32;
    // The manifest of a record of one person in Documents, announcing far more than will cross.
    let one = record(1);
    let mut manifest = pctwin_record::encode_inventory(&one.source_laptop, &one.items)
        .unwrap()
        .manifest;
    manifest[28..36].copy_from_slice(&(u64::from(pages) * u64::from(per_page)).to_be_bytes());
    manifest[36..40].copy_from_slice(&pages.to_be_bytes());
    let name = |p: &mut Vec<u8>, s: &str| {
        p.push(0);
        p.extend_from_slice(&(s.len() as u32).to_be_bytes());
        p.extend_from_slice(s.as_bytes());
    };
    // Page 0 defines a chain of 127 folders of 31 bytes; every page puts its items at the bottom.
    let page = |n: u32| {
        let mut p = Vec::new();
        let (first, defined) = if n == 0 { (0u32, 127u32) } else { (127, 0) };
        for v in [n, first, defined, per_page] {
            p.extend_from_slice(&v.to_be_bytes());
        }
        for i in 0..defined {
            p.extend_from_slice(&(if i == 0 { ROOT } else { i - 1 }).to_be_bytes());
            name(&mut p, &"x".repeat(31));
        }
        for k in 0..per_page {
            let mut id = [0u8; 16];
            id[..8].copy_from_slice(&(u64::from(n) << 32 | u64::from(k)).to_be_bytes());
            p.extend_from_slice(&id);
            p.push(0);
            p.extend_from_slice(&0u32.to_be_bytes());
            p.extend_from_slice(&0u32.to_be_bytes());
            p.extend_from_slice(&126u32.to_be_bytes());
            name(&mut p, "f");
            p.extend_from_slice(&5u64.to_be_bytes());
            p.extend_from_slice(&[0, 0, 0, 0]);
        }
        p
    };
    let (mut old, mut new) = pair();
    let sender = async move {
        let blob = |index: u32, bytes: Vec<u8>| {
            let mut frames = vec![paged_head(index, bytes.len() as u32)];
            frames.extend(bytes.chunks(PIECE_MAX).map(part));
            frames
        };
        for f in blob(u32::MAX, manifest) {
            if old.send(&f).await.is_err() {
                return 0;
            }
        }
        for n in 0..pages {
            for f in blob(n, page(n)) {
                if old.send(&f).await.is_err() {
                    return n;
                }
            }
        }
        pages
    };
    let receiver = async {
        let got = receive_inventory(&mut new, PAGED).await;
        drop(new);
        got
    };
    let (_, got) = tokio::join!(sender, receiver);
    let why = refused(got);
    assert!(why.contains("more memory than PCTwin allows"), "{why}");
}

/// The person's own essentials cross in order with the list from move protocol 4, so the new
/// laptop shows (and the person approves) the order the move sends in. An older new laptop is
/// sent none: the list it reads is the one it always read.
#[tokio::test]
async fn the_personal_order_crosses_from_protocol_4_and_never_before() {
    let mut sent = record(5);
    sent.personal = vec![sent.items[3].id, sent.items[0].id];
    for (version, crosses) in [(4u16, true), (3, false)] {
        let (mut old, mut new) = pair();
        let (s, r) = tokio::join!(
            send_inventory(&mut old, &sent, version),
            receive_inventory(&mut new, version)
        );
        s.unwrap();
        let got = r.unwrap();
        assert_eq!(got.items, sent.items, "protocol {version}");
        if crosses {
            assert_eq!(got.personal, sent.personal, "protocol {version}");
        } else {
            assert!(got.personal.is_empty(), "protocol {version}");
        }
    }
}

/// Scan record version 2, point 6: the inventory's fingerprint, taken over the bytes exactly as
/// they crossed, stays with the record the new laptop builds from it, so the plan the person
/// approves names the very inventory that arrived (not a re-encoding of it).
#[tokio::test]
async fn the_record_built_from_pages_names_the_inventory_that_crossed() {
    let sent = record(5);
    let (mut old, mut new) = pair();
    let (s, r) = tokio::join!(
        send_inventory(&mut old, &sent, PAGED),
        receive_inventory(&mut new, PAGED)
    );
    s.unwrap();
    let got = r.unwrap();
    let crossed = pctwin_record::encode_inventory(&sent.source_laptop, &sent.items)
        .unwrap()
        .fingerprint();
    assert_eq!(got.inventory, Some(crossed));
    // The plan's own fingerprint (what an approval holds) depends on it.
    let mut other = got.clone();
    other.inventory = None;
    assert_ne!(got.fingerprint(), other.fingerprint());
    // Kept through every revision the new laptop makes.
    assert_eq!(got.next_revision().inventory, Some(crossed));
}

/// A folder too big to list is a reason only move protocol 4 knows: an older new laptop is
/// asked to update, never sent a list it would refuse part way, and nothing is sent at all.
#[tokio::test]
async fn a_folder_too_big_to_list_asks_an_older_new_laptop_to_update() {
    let mut r = record(3);
    r.items[1].kind = ItemKind::Folder;
    r.items[1].inclusion = Inclusion::LeftOut {
        reason: pctwin_record::LeftOutReason::TooManyFiles,
    };
    for version in [PAGED] {
        let (mut old, new) = pair();
        let got = send_inventory(&mut old, &r, version).await;
        assert!(
            matches!(
                got,
                Err(TransferError::VersionGap(
                    pctwin_transfer::VersionGap::OtherLaptopOlder
                ))
            ),
            "{got:?}"
        );
        // Nothing was sent: the new laptop finds the line closed before any message.
        drop(old);
        let mut new = new;
        let heard = receive_inventory(&mut new, version).await;
        assert!(
            matches!(heard, Err(TransferError::ConnectionDropped)),
            "{heard:?}"
        );
    }
    // Protocol 4 carries it.
    let (mut old, mut new) = pair();
    let (s, got) = tokio::join!(
        send_inventory(&mut old, &r, pctwin_transfer::PERSONAL_SINCE),
        receive_inventory(&mut new, pctwin_transfer::PERSONAL_SINCE)
    );
    s.unwrap();
    assert_eq!(got.unwrap().items, r.items);
}

/// Phase 2.1, step 1: the apps found on the old laptop cross with the list from move protocol 5,
/// and count in the fingerprint of what crossed. A new laptop on 4 or 3 is asked to update
/// before anything is sent (it would otherwise refuse the list as damaged), whether the list
/// holds apps or a cloud service by its ID; a list with neither still crosses to it as before.
#[tokio::test]
async fn apps_cross_from_protocol_five_and_an_older_new_laptop_is_asked_to_update() {
    use pctwin_record::*;
    let mut r = record(3);
    r.apps.found.push(FoundApp {
        name: "Writer".into(),
        version: Some("2.0".into()),
        publisher: None,
        owner: Owner::Shared,
        system: AppSystem::Windows,
        found: FoundWhere::WindowsEntry {
            view: RegistryView::Machine,
            key: "Writer".into(),
            product_code: None,
        },
        arch: AppArch::UNKNOWN,
        maker: AppMaker::Unsigned,
        web: None,
        update_feed: None,
        installers: vec![],
        notes: vec![],
    });
    let (mut old, mut new) = pair();
    let (s, got) = tokio::join!(
        send_inventory(&mut old, &r, pctwin_transfer::APPS_SINCE),
        receive_inventory(&mut new, pctwin_transfer::APPS_SINCE)
    );
    s.unwrap();
    let got = got.unwrap();
    assert_eq!(got.apps, r.apps);
    assert_eq!(got.items, r.items);
    let crossed = encode_inventory_with_apps(&r.source_laptop, &r.items, &r.personal, &r.apps)
        .unwrap()
        .fingerprint();
    assert_eq!(got.inventory, Some(crossed));

    let mut cloud = record(3);
    cloud.items[0].place.storage = Storage::Cloud {
        provider: CloudProvider::new("pcloud").unwrap(),
    };
    for older in [pctwin_transfer::PERSONAL_SINCE, PAGED] {
        for asks in [&r, &cloud] {
            let (mut old, new) = pair();
            let sent = send_inventory(&mut old, asks, older).await;
            assert!(
                matches!(
                    sent,
                    Err(TransferError::VersionGap(
                        pctwin_transfer::VersionGap::OtherLaptopOlder
                    ))
                ),
                "{sent:?}"
            );
            drop(old);
            let mut new = new;
            assert!(matches!(
                receive_inventory(&mut new, older).await,
                Err(TransferError::ConnectionDropped)
            ));
        }
        let plain = record(3);
        let (mut old, mut new) = pair();
        let (s, got) = tokio::join!(
            send_inventory(&mut old, &plain, older),
            receive_inventory(&mut new, older)
        );
        s.unwrap();
        assert_eq!(got.unwrap().items, plain.items);
    }
}
