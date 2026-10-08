//! The transfer's messages over the encrypted link (Task List 1.5): each fits the link's 64 KiB
//! limit, blocks travel as pieces and are rejoined with a size cap, and anything malformed is
//! refused rather than guessed at.

use pctwin_record::ItemId;
use pctwin_transfer::{
    BlockMap, Header, MAX_BLOCK, Message, PIECE_MAX, PieceBuffer, ResumeTicket, Stamp,
    TransferError, split_into_pieces,
};

fn id() -> ItemId {
    ItemId::from_hex("0123456789abcdef0123456789abcdef").unwrap()
}

fn stamp() -> Stamp {
    Stamp {
        size: 5_000_000,
        modified_ns: Some(1_800_000_000_123_456_789),
    }
}

/// A file of 39 blocks with 0 to 11 and 20 done.
fn map() -> BlockMap {
    let mut m = BlockMap::new(39);
    for b in (0..12).chain(20..21) {
        m.insert(b).unwrap();
    }
    m
}

fn all_kinds() -> Vec<Message> {
    vec![
        Message::StartFile {
            stream: 7,
            item: id(),
            destination: "me".into(),
            path: "Documents/Taxes/return.pdf".into(),
            header: Header {
                size: 5_000_000,
                block_size: 131_072,
                block_count: 39,
                stamp: stamp(),
            },
            resumed_done: 0,
        },
        Message::Piece {
            stream: 7,
            last: false,
            bytes: vec![1, 2, 3],
        },
        Message::Piece {
            stream: 7,
            last: true,
            bytes: Vec::new(),
        },
        Message::EndFile {
            stream: 7,
            stamp_after: Stamp {
                size: 5_000_000,
                modified_ns: None,
            },
            changed: true,
        },
        Message::Receipt {
            stream: 7,
            block: 12,
        },
        Message::ResumeFrom {
            stream: 7,
            ticket: ResumeTicket {
                done: map(),
                block_size: 131_072,
                stamp: stamp(),
            },
        },
        Message::FileDone {
            stream: 7,
            ok: false,
        },
        Message::Ready,
        Message::AllSent,
        Message::Have {
            stream: 7,
            same_size: None,
        },
        Message::Have {
            stream: 7,
            same_size: Some([9; 32]),
        },
        Message::Skip { stream: 7 },
    ]
}

#[test]
fn every_message_survives_the_trip() {
    for m in all_kinds() {
        let wire = m.encode();
        assert!(wire.len() <= u16::MAX as usize);
        assert_eq!(Message::decode(&wire).unwrap(), m);
    }
}

#[test]
fn the_most_scattered_resume_message_fits_the_link() {
    let mut m = BlockMap::new(1 << 20);
    for b in (0..1 << 20).step_by(2) {
        m.insert(b).unwrap();
    }
    let wire = Message::ResumeFrom {
        stream: 7,
        ticket: ResumeTicket {
            done: m,
            block_size: 131_072,
            stamp: stamp(),
        },
    }
    .encode();
    assert!(wire.len() <= 60 * 1024, "{}", wire.len());
    assert!(Message::decode(&wire).is_ok());
}

#[test]
fn a_whole_block_travels_as_pieces_and_is_rejoined() {
    let block: Vec<u8> = (0..(PIECE_MAX * 3 + 123))
        .map(|i| (i % 251) as u8)
        .collect();
    let pieces = split_into_pieces(9, &block);
    assert_eq!(pieces.len(), 4);
    let mut buffer = PieceBuffer::default();
    let mut whole = None;
    for p in pieces {
        let wire = p.encode();
        assert!(wire.len() <= u16::MAX as usize);
        if let Message::Piece { last, bytes, .. } = Message::decode(&wire).unwrap() {
            whole = buffer.add(&bytes, last).unwrap();
        }
    }
    assert_eq!(whole.unwrap(), block);
    // The buffer is ready for the next block.
    assert_eq!(buffer.add(b"next", true).unwrap().unwrap(), b"next");
}

#[test]
fn a_block_is_never_rejoined_past_the_largest_allowed() {
    let mut buffer = PieceBuffer::default();
    let piece = vec![0u8; PIECE_MAX];
    let mut refused = false;
    for _ in 0..(MAX_BLOCK as usize / PIECE_MAX + 2) {
        if let Err(TransferError::Damaged(_)) = buffer.add(&piece, false) {
            refused = true;
            break;
        }
    }
    assert!(refused, "the buffer grew past the largest block");
}

#[test]
fn malformed_messages_are_refused() {
    let good = all_kinds()[0].encode();
    assert!(Message::decode(&[]).is_err());
    assert!(Message::decode(&[99]).is_err(), "unknown kind");
    assert!(
        Message::decode(&good[..good.len() - 1]).is_err(),
        "cut short"
    );
    let mut extra = good.clone();
    extra.push(0);
    assert!(Message::decode(&extra).is_err(), "trailing bytes");
    let too_big = Message::Piece {
        stream: 1,
        last: false,
        bytes: vec![0; PIECE_MAX + 1],
    };
    assert!(Message::decode(&too_big.encode()).is_err());
}

#[test]
fn paths_and_labels_must_be_text_within_limits() {
    let start = |destination: &str, path: String| Message::StartFile {
        stream: 1,
        item: id(),
        destination: destination.into(),
        path,
        header: Header {
            size: 0,
            block_size: 131_072,
            block_count: 0,
            stamp: stamp(),
        },
        resumed_done: 0,
    };
    assert!(Message::decode(&start("me", "a".repeat(4096)).encode()).is_ok());
    assert!(Message::decode(&start("me", "a".repeat(4097)).encode()).is_err());
    assert!(Message::decode(&start(&"d".repeat(65), "a".into()).encode()).is_err());
    // Not valid text: refused.
    let mut wire = start("me", "ab".into()).encode();
    let n = wire.len();
    wire[n - 1] = 0xFF;
    assert!(Message::decode(&wire).is_err());
}

proptest::proptest! {
    #![proptest_config(proptest::prelude::ProptestConfig::with_cases(3000))]
    #[test]
    fn decoding_anything_never_crashes(bytes in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..300)) {
        let _ = Message::decode(&bytes);
    }
}
