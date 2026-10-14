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
    PersonTarget, Place, Portability, Record, SharedTarget, Storage,
};
use pctwin_transfer::{
    Channel, ChannelError, INVENTORY_PIECE_WAIT, InventoryFrame, MAX_INVENTORY_BYTES, Message,
    PIECE_MAX, TransferError, receive_inventory, send_inventory,
};
use tokio::sync::mpsc;

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

/// Sends `frames` from a pretend old laptop, then returns what the new laptop made of them and
/// how many messages it read.
async fn hostile(frames: Vec<Vec<u8>>) -> (Result<Record, TransferError>, usize) {
    let (mut old, mut new) = pair();
    for f in &frames {
        old.send(f).await.unwrap();
    }
    drop(old);
    let got = receive_inventory(&mut new).await;
    (got, new.read.load(Ordering::SeqCst))
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

/// The frames an honest old laptop sends for these JSON bytes.
fn honest(json: &[u8]) -> Vec<Vec<u8>> {
    let packed = zstd::bulk::compress(json, 3).unwrap();
    let mut frames = vec![start(json.len() as u64, packed.len() as u64)];
    frames.extend(packed.chunks(PIECE_MAX).map(part));
    frames
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
    let (s, r) = tokio::join!(send_inventory(&mut old, &sent), receive_inventory(&mut new));
    s.unwrap();
    let got = r.unwrap();
    assert_eq!(got, sent);
    assert_eq!(got.fingerprint(), sent.fingerprint());
}

#[tokio::test]
async fn a_large_record_travels_in_pieces_that_each_fit_the_link() {
    // Unique names barely compress, so this needs many pieces.
    let sent = record(40_000);
    let (mut old, mut new) = pair();
    let (s, r) = tokio::join!(send_inventory(&mut old, &sent), receive_inventory(&mut new));
    s.unwrap();
    assert_eq!(r.unwrap(), sent);
    assert!(
        new.read.load(Ordering::SeqCst) > 3,
        "expected several pieces"
    );
}

#[tokio::test]
async fn every_message_of_the_exchange_fits_the_link() {
    let sent = record(40_000);
    let (mut old, mut new) = pair();
    send_inventory(&mut old, &sent).await.unwrap();
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
    let mut old_link = pending_host.choose(number, Instant::now()).await.unwrap();
    let mut new_link = pending_guest.approval().await.unwrap();

    let sent = record(20_000);
    let (s, r) = tokio::time::timeout(Duration::from_secs(60), async {
        tokio::join!(
            send_inventory(&mut old_link, &sent),
            receive_inventory(&mut new_link)
        )
    })
    .await
    .unwrap();
    s.unwrap();
    assert_eq!(r.unwrap(), sent);
}

#[tokio::test]
async fn a_record_announced_over_the_limit_is_refused_before_any_piece_is_read() {
    let (got, read) = hostile(vec![start(MAX_INVENTORY_BYTES + 1, 1000), part(&[0; 1000])]).await;
    refused(got);
    assert_eq!(read, 1);
}

#[tokio::test]
async fn packed_bytes_announced_beyond_what_compression_can_make_are_refused() {
    let (got, read) = hostile(vec![start(100, 10_000_000), part(&[0; 1000])]).await;
    refused(got);
    assert_eq!(read, 1);
}

#[tokio::test]
async fn an_empty_record_announcement_is_refused_at_once() {
    let (got, read) = hostile(vec![start(0, 0)]).await;
    refused(got);
    assert_eq!(read, 1);
    // Refused without waiting for pieces that would never make a record.
    let (got, read) = hostile(vec![start(0, 10)]).await;
    refused(got);
    assert_eq!(read, 1);
}

#[tokio::test]
async fn an_empty_packed_record_is_refused() {
    let (got, _) = hostile(vec![start(10, 0)]).await;
    refused(got);
}

/// The old laptop sends full pieces until the last, so the number of pieces is fixed by the
/// announced size and a sender cannot drag the exchange out over countless tiny ones.
#[tokio::test]
async fn a_short_piece_before_the_last_is_refused_at_once() {
    let json = record(40_000).to_json();
    let packed = zstd::bulk::compress(&json, 3).unwrap();
    assert!(packed.len() > 2 * PIECE_MAX);
    let mut frames = vec![
        start(json.len() as u64, packed.len() as u64),
        part(&packed[..PIECE_MAX - 1]),
    ];
    frames.extend(packed[PIECE_MAX - 1..].chunks(PIECE_MAX).map(part));
    let (got, read) = hostile(frames).await;
    refused(got);
    assert_eq!(read, 2);

    let small = record(3).to_json();
    let packed = zstd::bulk::compress(&small, 3).unwrap();
    let mut frames = vec![start(small.len() as u64, packed.len() as u64)];
    frames.extend(packed.chunks(1).map(part));
    let (got, read) = hostile(frames).await;
    refused(got);
    assert_eq!(read, 2);
}

#[tokio::test]
async fn a_second_start_part_way_through_is_refused() {
    let json = record(40_000).to_json();
    let mut frames = honest(&json);
    let announce = frames[0].clone();
    frames.insert(2, announce);
    let (got, read) = hostile(frames).await;
    refused(got);
    assert_eq!(read, 3);
}

#[tokio::test]
async fn more_bytes_than_announced_are_refused_at_the_piece_that_overflows() {
    let json = record(5).to_json();
    let mut frames = honest(&json);
    let declared = match InventoryFrame::decode(&frames[0]).unwrap() {
        InventoryFrame::Start {
            raw_len,
            packed_len,
        } => (raw_len, packed_len),
        InventoryFrame::Part(_) => unreachable!(),
    };
    frames[0] = start(declared.0, declared.1 - 1);
    let (got, read) = hostile(frames).await;
    refused(got);
    assert_eq!(read, 2);
}

#[tokio::test]
async fn a_connection_that_ends_before_every_piece_arrived_is_a_dropped_connection() {
    let json = record(40_000).to_json();
    let mut frames = honest(&json);
    frames.truncate(2);
    let (got, _) = hostile(frames).await;
    assert!(
        matches!(got, Err(TransferError::ConnectionDropped)),
        "{got:?}"
    );
}

#[tokio::test]
async fn an_empty_piece_is_refused() {
    let json = record(5).to_json();
    let mut frames = honest(&json);
    frames.insert(1, part(&[]));
    let (got, _) = hostile(frames).await;
    refused(got);
}

#[tokio::test]
async fn a_piece_larger_than_the_link_allows_is_refused() {
    let (got, _) = hostile(vec![start(200_000, 200_000), part(&vec![7; PIECE_MAX + 1])]).await;
    refused(got);
}

#[tokio::test]
async fn a_record_that_unpacks_larger_than_announced_is_refused() {
    // A compression bomb: a few dozen bytes that unpack to a megabyte, announced as 100 bytes.
    let packed = zstd::bulk::compress(&vec![b'x'; 1_000_000], 19).unwrap();
    assert!(packed.len() <= zstd::zstd_safe::compress_bound(100));
    let (got, _) = hostile(vec![start(100, packed.len() as u64), part(&packed)]).await;
    refused(got);
}

#[tokio::test]
async fn a_whole_record_one_byte_longer_than_announced_is_refused() {
    let json = record(5).to_json();
    let packed = zstd::bulk::compress(&json, 3).unwrap();
    let mut frames = vec![start(json.len() as u64 - 1, packed.len() as u64)];
    frames.extend(packed.chunks(PIECE_MAX).map(part));
    let (got, _) = hostile(frames).await;
    refused(got);
}

#[tokio::test]
async fn a_record_that_unpacks_smaller_than_announced_is_refused() {
    let json = record(5).to_json();
    let packed = zstd::bulk::compress(&json, 3).unwrap();
    let mut frames = vec![start(json.len() as u64 + 1, packed.len() as u64)];
    frames.extend(packed.chunks(PIECE_MAX).map(part));
    let (got, _) = hostile(frames).await;
    refused(got);
}

#[tokio::test]
async fn bytes_that_are_not_compressed_data_are_refused() {
    let (got, _) = hostile(vec![start(1000, 1000), part(&[0x55; 1000])]).await;
    refused(got);
}

#[tokio::test]
async fn a_transfer_message_in_place_of_the_inventory_is_refused() {
    let (got, read) = hostile(vec![Message::Ready.encode()]).await;
    refused(got);
    assert_eq!(read, 1);
}

#[tokio::test]
async fn a_transfer_message_between_pieces_is_refused() {
    let json = record(40_000).to_json();
    let mut frames = honest(&json);
    assert!(frames.len() > 2);
    frames.insert(2, Message::AllSent.encode());
    let (got, read) = hostile(frames).await;
    refused(got);
    assert_eq!(read, 3);
}

#[tokio::test]
async fn text_that_is_not_a_record_is_refused() {
    let (got, _) = hostile(honest(b"{\"hello\": \"world\"}")).await;
    refused(got);
}

#[tokio::test]
async fn deeply_nested_input_is_refused_without_crashing() {
    let mut json = "[".repeat(100_000);
    json.push_str(&"]".repeat(100_000));
    let (got, _) = hostile(honest(json.as_bytes())).await;
    refused(got);
}

#[tokio::test]
async fn a_record_from_a_newer_app_is_refused() {
    let json = String::from_utf8(record(1).to_json()).unwrap();
    let newer = json.replacen("\"format\":1", "\"format\":999", 1);
    assert_ne!(json, newer, "the test must change the format");
    let (got, _) = hostile(honest(newer.as_bytes())).await;
    refused(got);
}

#[tokio::test]
async fn a_record_that_breaks_the_record_rules_is_refused() {
    let mut r = record(2);
    let first = r.items[0].id;
    r.items[1].id = first;
    let (got, _) = hostile(honest(&r.to_json())).await;
    refused(got);
}

#[tokio::test]
async fn the_old_laptop_cannot_decide_where_people_land() {
    let mut r = record(2);
    r.mapping.people.insert(
        "1000".into(),
        PersonTarget::ExistingAccount {
            account_id: "1001".into(),
        },
    );
    let (got, _) = hostile(honest(&r.to_json())).await;
    let why = refused(got);
    assert!(why.contains("people"), "{why}");
}

#[tokio::test]
async fn a_record_that_is_not_a_fresh_scan_is_refused() {
    let later = record(2).next_revision();
    let (got, _) = hostile(honest(&later.to_json())).await;
    refused(got);
    // A first revision that claims an earlier one.
    let json = String::from_utf8(later.to_json()).unwrap();
    let claimed = json.replacen("\"revision\":2", "\"revision\":1", 1);
    assert_ne!(json, claimed, "the test must change the revision");
    let (got, _) = hostile(honest(claimed.as_bytes())).await;
    refused(got);
    // A later revision with no earlier one named.
    let fresh = String::from_utf8(record(2).to_json()).unwrap();
    let skipped = fresh.replacen("\"revision\":1", "\"revision\":5", 1);
    assert_ne!(fresh, skipped, "the test must change the revision");
    let (got, _) = hostile(honest(skipped.as_bytes())).await;
    refused(got);
}

#[tokio::test]
async fn the_old_laptop_cannot_decide_where_shared_things_go() {
    let mut r = record(2);
    r.mapping.shared = SharedTarget::Skip;
    let (got, _) = hostile(honest(&r.to_json())).await;
    refused(got);
}

#[tokio::test]
async fn sending_a_record_with_choices_about_people_is_refused_on_the_old_laptop_too() {
    let mut r = record(2);
    r.mapping.people.insert("1000".into(), PersonTarget::Skip);
    let (mut old, _new) = pair();
    let got = send_inventory(&mut old, &r).await;
    assert!(matches!(got, Err(TransferError::Inventory(_))), "{got:?}");
}

#[tokio::test]
async fn nothing_arriving_is_a_dropped_connection() {
    let (got, _) = hostile(Vec::new()).await;
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

/// An old laptop that connects and then says nothing is given up on, instead of leaving the new
/// laptop waiting for ever.
#[tokio::test(start_paused = true)]
async fn an_old_laptop_that_never_starts_is_given_up_on() {
    let (_old, mut new) = pair();
    let t = tokio::time::Instant::now();
    let got = receive_inventory(&mut new).await;
    assert!(no_answer(&got), "{got:?}");
    assert!(t.elapsed() >= INVENTORY_PIECE_WAIT, "{:?}", t.elapsed());
    assert!(t.elapsed() < INVENTORY_PIECE_WAIT + Duration::from_secs(1));
}

#[tokio::test(start_paused = true)]
async fn an_old_laptop_that_stops_part_way_is_given_up_on() {
    let json = record(40_000).to_json();
    let frames = honest(&json);
    let (mut old, mut new) = pair();
    for f in &frames[..2] {
        old.send(f).await.unwrap();
    }
    let t = tokio::time::Instant::now();
    let got = receive_inventory(&mut new).await;
    assert!(no_answer(&got), "{got:?}");
    assert!(t.elapsed() < INVENTORY_PIECE_WAIT + Duration::from_secs(1));
    drop(old);
}

/// Sends a start for `pieces` full pieces, then one full piece every `every`, from a pretend old
/// laptop that never hangs up.
fn steady(pieces: u64, every: Duration) -> Mem {
    let (mut old, new) = pair();
    tokio::spawn(async move {
        let n = pieces * PIECE_MAX as u64;
        old.send(&start(n, n)).await.unwrap();
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
    let got = receive_inventory(&mut new).await;
    refused(got);
    assert!(t.elapsed() >= (INVENTORY_PIECE_WAIT - Duration::from_secs(1)) * pieces as u32);
}

/// A piece that comes later than the wait ends the exchange, even after many on time.
#[tokio::test(start_paused = true)]
async fn one_piece_later_than_the_wait_is_given_up_on() {
    let mut new = steady(3, INVENTORY_PIECE_WAIT + Duration::from_secs(1));
    let got = receive_inventory(&mut new).await;
    assert!(no_answer(&got), "{got:?}");
}
