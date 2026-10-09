//! The transfer's messages over the encrypted link (Task List 1.5): each fits the link's 64 KiB
//! limit, blocks travel as pieces and are rejoined with a size cap, and anything malformed is
//! refused rather than guessed at.

use pctwin_journal::FileId;
use pctwin_record::ItemId;
use pctwin_transfer::{
    BlockMap, Header, MAX_BLOCK, MAX_ORIGINALS_PER_REQUEST, Message, OriginalNow, PIECE_MAX,
    PieceBuffer, ResumeTicket, Stamp, TransferError, split_into_pieces,
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
                source_file: Some(FileId {
                    volume: 0xdead_beef_0102_0304,
                    index: std::num::NonZeroU64::new(0x0011_2233_4455_6677).unwrap(),
                    born: None,
                }),
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
            item: ItemId::from_hex("0f0e0d0c0b0a09080706050403020100").unwrap(),
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
        Message::Refused { stream: 7 },
        Message::CheckOriginals {
            items: vec![
                id(),
                ItemId::from_hex("0f0e0d0c0b0a09080706050403020100").unwrap(),
            ],
        },
        Message::CheckOriginals { items: Vec::new() },
        Message::Originals {
            answers: vec![
                (
                    id(),
                    OriginalNow::Present {
                        size: 5_000_000,
                        modified_ns: Some(-5),
                        file: Some(FileId {
                            volume: 9,
                            index: std::num::NonZeroU64::new(8).unwrap(),
                            born: None,
                        }),
                    },
                ),
                (
                    id(),
                    OriginalNow::Present {
                        size: 0,
                        modified_ns: None,
                        file: None,
                    },
                ),
                (id(), OriginalNow::Missing),
                (id(), OriginalNow::CannotLook),
            ],
        },
    ]
}

#[test]
fn a_header_with_and_without_the_originals_identity_survives_the_trip() {
    for source_file in [
        None,
        Some(FileId {
            volume: u64::MAX,
            index: std::num::NonZeroU64::MAX,
            born: None,
        }),
        Some(FileId {
            volume: 0,
            index: std::num::NonZeroU64::MIN,
            born: Some(pctwin_journal::Born {
                secs: -1,
                nanos: 999_999_999,
            }),
        }),
    ] {
        let m = Message::StartFile {
            stream: 1,
            item: id(),
            destination: "me".into(),
            path: "a".into(),
            header: Header {
                size: 1,
                block_size: 131_072,
                block_count: 1,
                stamp: stamp(),
                source_file,
            },
            resumed_done: 0,
        };
        let wire = m.encode();
        assert_eq!(Message::decode(&wire).unwrap(), m);
        // A strict reader: the identity's flag is 0 or 1 and nothing else.
        let mut bad = wire.clone();
        // Header: kind(1) stream(4) item(16) size(8) block_size(8) block_count(8) stamp(8+1+8).
        let flag_at = 1 + 4 + 16 + 24 + 17;
        bad[flag_at] = 2;
        assert!(Message::decode(&bad).is_err());
        // Cut inside the identity.
        if source_file.is_some() {
            assert!(Message::decode(&wire[..flag_at + 5]).is_err());
        }
    }
}

fn items(n: usize) -> Vec<ItemId> {
    (0..n)
        .map(|i| ItemId::from_hex(&format!("{i:032x}")).unwrap())
        .collect()
}

#[test]
fn the_check_about_originals_is_bounded() {
    let at_limit = Message::CheckOriginals {
        items: items(MAX_ORIGINALS_PER_REQUEST),
    };
    assert!(at_limit.encode().len() <= 60 * 1024);
    assert_eq!(Message::decode(&at_limit.encode()).unwrap(), at_limit);
    let over = Message::CheckOriginals {
        items: items(MAX_ORIGINALS_PER_REQUEST + 1),
    };
    assert!(Message::decode(&over.encode()).is_err());
}

#[test]
fn the_answer_about_originals_is_bounded_and_fits_the_link() {
    let answer = |n: usize| Message::Originals {
        answers: items(n)
            .into_iter()
            .map(|i| {
                (
                    i,
                    OriginalNow::Present {
                        size: 1,
                        modified_ns: Some(1),
                        file: Some(FileId {
                            volume: 1,
                            index: std::num::NonZeroU64::new(1).unwrap(),
                            born: None,
                        }),
                    },
                )
            })
            .collect(),
    };
    let at_limit = answer(MAX_ORIGINALS_PER_REQUEST);
    assert!(
        at_limit.encode().len() <= 60 * 1024,
        "{}",
        at_limit.encode().len()
    );
    assert_eq!(Message::decode(&at_limit.encode()).unwrap(), at_limit);
    assert!(Message::decode(&answer(MAX_ORIGINALS_PER_REQUEST + 1).encode()).is_err());
}

#[test]
fn damaged_originals_messages_are_refused() {
    let check = Message::CheckOriginals { items: items(2) }.encode();
    // Cut short, trailing bytes, and a count that promises more than there is.
    assert!(Message::decode(&check[..check.len() - 1]).is_err());
    let mut extra = check.clone();
    extra.push(0);
    assert!(Message::decode(&extra).is_err());
    let mut lying = check.clone();
    lying[6] = 3; // count (after kind and the 4-byte stream) says 3 items
    assert!(Message::decode(&lying).is_err());
    // These two have no stream of their own: a stream number is refused.
    let mut streamed = check.clone();
    streamed[4] = 1;
    assert!(Message::decode(&streamed).is_err());
    let answers = Message::Originals {
        answers: vec![(id(), OriginalNow::Missing)],
    }
    .encode();
    let mut streamed = answers.clone();
    streamed[4] = 1;
    assert!(Message::decode(&streamed).is_err());
    // An unknown answer kind, a bad flag inside a present answer, and trailing bytes.
    let mut unknown = answers.clone();
    *unknown.last_mut().unwrap() = 9;
    assert!(Message::decode(&unknown).is_err());
    let mut extra = answers.clone();
    extra.push(0);
    assert!(Message::decode(&extra).is_err());
    let present = Message::Originals {
        answers: vec![(
            id(),
            OriginalNow::Present {
                size: 1,
                modified_ns: None,
                file: None,
            },
        )],
    }
    .encode();
    assert!(Message::decode(&present).is_ok());
    for cut in 1..present.len() {
        assert!(Message::decode(&present[..cut]).is_err(), "cut at {cut}");
    }
    let mut bad_flag = present.clone();
    let at = bad_flag.len() - 2; // modified-time flag
    bad_flag[at] = 7;
    assert!(Message::decode(&bad_flag).is_err());
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
        item: ItemId::from_hex("0f0e0d0c0b0a09080706050403020100").unwrap(),
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
            source_file: None,
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
