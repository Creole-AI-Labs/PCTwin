//! Big files split across lanes (Security Design A, "Lanes share one plan"): every piece of work
//! runs in order on one lane. A big file is cut into sections; a lane that runs out of work takes
//! the back half of the section with the most left to send (the aria2 approach), and never a range
//! smaller than twice [`MIN_SECTION_BYTES`]. Done blocks are recorded block by block, so a dropped
//! lane or a resume loses nothing and nothing is confirmed twice.

use pctwin_transfer::{FileSections, MIN_SECTION_BYTES, SectionError};
use proptest::prelude::*;

const KIB128: u64 = 128 * 1024;

/// A file of `mib` MiB in 128 KiB blocks.
fn file(mib: u64) -> FileSections {
    let blocks = u32::try_from(mib * 1024 * 1024 / KIB128).unwrap();
    FileSections::new(blocks, KIB128, &[])
}

/// Sends `n` blocks on `lane`, returning them.
fn send(f: &mut FileSections, lane: u32, n: usize) -> Vec<u32> {
    (0..n).map_while(|_| f.next_block(lane)).collect()
}

#[test]
fn the_minimum_section_is_32_mib() {
    assert_eq!(MIN_SECTION_BYTES, 32 * 1024 * 1024);
}

#[test]
fn a_file_under_twice_the_minimum_is_never_split() {
    let mut f = file(60);
    assert_eq!(f.claim(1), Some(0..480));
    assert_eq!(f.claim(2), None);
}

#[test]
fn a_second_lane_takes_the_back_half() {
    let mut f = file(200);
    assert_eq!(f.claim(1), Some(0..1600));
    assert_eq!(f.claim(2), Some(800..1600));
    // Lane 1 now stops where lane 2 begins.
    let sent = send(&mut f, 1, 2000);
    assert_eq!(sent, (0..800).collect::<Vec<_>>());
    assert_eq!(send(&mut f, 2, 3), [800, 801, 802]);
}

#[test]
fn the_split_is_measured_from_what_was_already_sent() {
    let mut f = file(200);
    f.claim(1);
    // 600 blocks are on their way but none confirmed yet: they must stay with lane 1.
    send(&mut f, 1, 600);
    assert_eq!(f.claim(2), Some(1100..1600));
    assert_eq!(send(&mut f, 1, 2000), (600..1100).collect::<Vec<_>>());
}

#[test]
fn too_little_left_to_send_is_not_split() {
    let mut f = file(200);
    f.claim(1);
    // 1,089 sent: 511 blocks (just under 64 MiB) left.
    send(&mut f, 1, 1089);
    assert_eq!(f.claim(2), None);
    let mut f = file(200);
    f.claim(1);
    // 1,088 sent: exactly 64 MiB (512 blocks) left, which is enough.
    send(&mut f, 1, 1088);
    assert_eq!(f.claim(2), Some(1344..1600));
}

#[test]
fn an_idle_lane_splits_the_section_with_the_most_left() {
    let mut f = file(400);
    assert_eq!(f.claim(1), Some(0..3200));
    assert_eq!(f.claim(2), Some(1600..3200));
    send(&mut f, 1, 1000); // lane 1 has 600 left; lane 2 has 1,600 left
    assert_eq!(f.claim(3), Some(2400..3200));
}

#[test]
fn between_equal_sections_the_earlier_one_is_split_so_the_file_fills_from_the_front() {
    let mut f = file(400);
    f.claim(1);
    f.claim(2);
    // Both lanes have 1,600 blocks left.
    assert_eq!(f.claim(3), Some(800..1600));
}

#[test]
fn a_lane_works_on_one_section_of_a_file_at_a_time() {
    let mut f = file(400);
    f.claim(1);
    assert_eq!(f.claim(1), None);
}

#[test]
fn confirmations_come_in_order_from_the_lane_that_sent_them() {
    let mut f = file(200);
    f.claim(1);
    f.claim(2);
    send(&mut f, 1, 3);
    send(&mut f, 2, 1);
    assert_eq!(f.confirmed(1, 1), Err(SectionError::OutOfOrder));
    assert_eq!(f.confirmed(2, 0), Err(SectionError::OutOfOrder));
    assert_eq!(f.confirmed(1, 0), Ok(()));
    // Not yet sent.
    assert_eq!(f.confirmed(2, 801), Err(SectionError::OutOfOrder));
    let mut g = file(200);
    g.claim(1);
    assert_eq!(g.confirmed(1, 0), Err(SectionError::OutOfOrder));
    // A lane with no section.
    assert_eq!(f.confirmed(3, 5), Err(SectionError::NoSection));
    // Confirming a block twice.
    assert_eq!(f.confirmed(1, 0), Err(SectionError::OutOfOrder));
}

#[test]
fn a_lost_lanes_unconfirmed_blocks_are_sent_again_and_confirmed_ones_are_not() {
    let mut f = file(200);
    f.claim(1);
    f.claim(2);
    send(&mut f, 2, 10);
    for b in 800..805 {
        f.confirmed(2, b).unwrap();
    }
    // Lane 2 drops with 805..810 sent but not confirmed.
    f.lane_lost(2);
    assert_eq!(f.confirmed(2, 805), Err(SectionError::NoSection));
    assert_eq!(f.claim(3), Some(805..1600));
    assert_eq!(send(&mut f, 3, 2), [805, 806]);
}

#[test]
fn a_resumed_file_skips_blocks_already_done() {
    let blocks = 1600u32;
    let mut done = vec![false; blocks as usize];
    for d in done.iter_mut().take(500) {
        *d = true;
    }
    for d in &mut done[700..900] {
        *d = true;
    }
    let mut f = FileSections::new(blocks, KIB128, &done);
    assert_eq!(f.done_count(), 700);
    assert_eq!(f.claim(1), Some(500..700));
    assert_eq!(f.claim(2), Some(900..1600));
    assert!(!f.is_done());
}

#[test]
fn every_block_is_confirmed_once_and_the_file_is_then_done() {
    let mut f = file(200);
    f.claim(1);
    f.claim(2);
    let mut seen = std::collections::BTreeSet::new();
    for lane in [1, 2] {
        while let Some(b) = f.next_block(lane) {
            f.confirmed(lane, b).unwrap();
            assert!(seen.insert(b));
        }
    }
    assert_eq!(seen.len(), 1600);
    assert!(f.is_done());
    assert_eq!(f.claim(3), None);
}

#[derive(Debug, Clone)]
enum Op {
    Claim(u32),
    Send(u32, u8),
    Confirm(u32, u8),
    Lose(u32),
}

fn op() -> impl Strategy<Value = Op> {
    let lane = 1u32..=4;
    prop_oneof![
        lane.clone().prop_map(Op::Claim),
        (lane.clone(), 1u8..60).prop_map(|(l, n)| Op::Send(l, n)),
        (lane.clone(), 1u8..60).prop_map(|(l, n)| Op::Confirm(l, n)),
        lane.prop_map(Op::Lose),
    ]
}

proptest! {
    /// Whatever lanes do (claim, send, confirm, drop) in any order: no block is ever being sent by
    /// two lanes at once, no block is confirmed twice, done blocks are never sent again, and driving
    /// the rest to the end finishes the file with every block confirmed exactly once.
    #[test]
    fn lanes_never_overlap_and_every_block_is_confirmed_exactly_once(
        mib in 1u64..300,
        resumed in proptest::collection::vec(any::<bool>(), 0..64),
        ops in proptest::collection::vec(op(), 0..120),
    ) {
        let blocks = u32::try_from(mib * 8).unwrap();
        let mut done: Vec<bool> = (0..blocks as usize)
            .map(|i| resumed.get(i * resumed.len().max(1) / blocks as usize).copied().unwrap_or(false))
            .collect();
        let mut f = FileSections::new(blocks, KIB128, &done);
        // Blocks each lane has sent and not yet had confirmed, in order.
        let mut flying: [Vec<u32>; 5] = Default::default();
        let mut confirmations = vec![0u32; blocks as usize];
        let check_disjoint = |flying: &[Vec<u32>; 5]| {
            let mut all: Vec<u32> = flying.iter().flatten().copied().collect();
            let n = all.len();
            all.sort_unstable();
            all.dedup();
            assert_eq!(all.len(), n, "a block was being sent on two lanes at once");
        };
        for op in ops {
            match op {
                Op::Claim(l) => { f.claim(l); }
                Op::Send(l, n) => {
                    for _ in 0..n {
                        let Some(b) = f.next_block(l) else { break };
                        prop_assert!(!done[b as usize], "a done block was sent again");
                        flying[l as usize].push(b);
                    }
                }
                Op::Confirm(l, n) => {
                    for _ in 0..n {
                        if flying[l as usize].is_empty() { break; }
                        let b = flying[l as usize].remove(0);
                        prop_assert_eq!(f.confirmed(l, b), Ok(()));
                        done[b as usize] = true;
                        confirmations[b as usize] += 1;
                    }
                }
                Op::Lose(l) => {
                    f.lane_lost(l);
                    flying[l as usize].clear();
                }
            }
            check_disjoint(&flying);
        }
        // Drive everything to the end on lanes 1 to 4.
        for l in 1..=4u32 {
            while let Some(b) = flying[l as usize].first().copied() {
                prop_assert_eq!(f.confirmed(l, b), Ok(()));
                flying[l as usize].remove(0);
                done[b as usize] = true;
                confirmations[b as usize] += 1;
            }
        }
        let mut rounds = 0;
        while !f.is_done() {
            rounds += 1;
            prop_assert!(rounds < 10_000, "the file never finished");
            for l in 1..=4u32 {
                f.claim(l);
                if let Some(b) = f.next_block(l) {
                    prop_assert!(!done[b as usize]);
                    prop_assert_eq!(f.confirmed(l, b), Ok(()));
                    done[b as usize] = true;
                    confirmations[b as usize] += 1;
                }
            }
        }
        prop_assert!(done.iter().all(|d| *d));
        prop_assert!(confirmations.iter().all(|c| *c <= 1));
        prop_assert_eq!(f.done_count(), blocks);
    }
}
