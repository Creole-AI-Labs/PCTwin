//! Which blocks of a file have arrived (Task List 1.5, lanes, step 3): kept as runs of blocks,
//! joined when they touch, so the map stays small however sections were split, and travels in the
//! resume message to tell the old laptop exactly what is still needed.

use pctwin_transfer::{BlockMap, MAX_TICKET_RUNS};
use proptest::prelude::*;

fn runs(m: &BlockMap) -> Vec<(u64, u64)> {
    m.runs().map(|r| (r.start, r.end)).collect()
}

fn missing(m: &BlockMap) -> Vec<(u64, u64)> {
    m.missing().map(|r| (r.start, r.end)).collect()
}

#[test]
fn a_new_map_has_nothing_done() {
    let m = BlockMap::new(10);
    assert_eq!(m.count(), 10);
    assert_eq!(m.done_count(), 0);
    assert!(!m.is_full());
    assert!(runs(&m).is_empty());
    assert_eq!(missing(&m), [(0, 10)]);
}

#[test]
fn blocks_that_touch_join_into_one_run() {
    let mut m = BlockMap::new(20);
    for b in [3, 4, 5] {
        assert_eq!(m.insert(b), Ok(true));
    }
    assert_eq!(runs(&m), [(3, 6)]);
    m.insert(2).unwrap();
    m.insert(6).unwrap();
    assert_eq!(runs(&m), [(2, 7)]);
    m.insert(0).unwrap();
    m.insert(19).unwrap();
    assert_eq!(runs(&m), [(0, 1), (2, 7), (19, 20)]);
    m.insert(1).unwrap();
    assert_eq!(runs(&m), [(0, 7), (19, 20)]);
    assert_eq!(m.done_count(), 8);
    assert_eq!(missing(&m), [(7, 19)]);
    assert!(m.contains(0) && m.contains(6) && m.contains(19));
    assert!(!m.contains(7) && !m.contains(18) && !m.contains(20));
}

#[test]
fn a_block_counts_once() {
    let mut m = BlockMap::new(5);
    assert_eq!(m.insert(2), Ok(true));
    assert_eq!(m.insert(2), Ok(false));
    assert_eq!(m.done_count(), 1);
}

#[test]
fn a_block_outside_the_file_is_refused() {
    let mut m = BlockMap::new(5);
    assert!(m.insert(5).is_err());
    assert!(m.insert(u64::MAX).is_err());
    assert_eq!(m.done_count(), 0);
}

#[test]
fn a_full_map_is_one_run() {
    let mut m = BlockMap::new(4);
    for b in [3, 1, 0, 2] {
        m.insert(b).unwrap();
    }
    assert!(m.is_full());
    assert_eq!(runs(&m), [(0, 4)]);
    assert!(m.missing().next().is_none());
    assert_eq!(m.to_bools(), [true; 4]);
    assert!(BlockMap::new(0).is_full());
}

#[test]
fn the_map_travels_and_is_read_back_exactly() {
    let mut m = BlockMap::new(1000);
    for b in (0..300).chain(500..501).chain(990..1000) {
        m.insert(b).unwrap();
    }
    let back = BlockMap::decode(&m.encode()).unwrap();
    assert_eq!(back, m);
    assert_eq!(back.done_count(), 311);
}

#[test]
fn a_very_scattered_map_sends_only_its_earliest_runs() {
    // Every other block: more runs than one message carries.
    let count = 4 * MAX_TICKET_RUNS as u64;
    let mut m = BlockMap::new(count);
    for b in (0..count).step_by(2) {
        m.insert(b).unwrap();
    }
    let back = BlockMap::decode(&m.encode()).unwrap();
    assert_eq!(back.runs().count(), MAX_TICKET_RUNS);
    // What travels is never more than what is really there: a block left out is sent again.
    assert!(back.runs().all(|r| (r.start..r.end).all(|b| m.contains(b))));
    assert_eq!(
        back.runs().last().unwrap().start,
        2 * (MAX_TICKET_RUNS as u64 - 1)
    );
    assert!(m.encode().len() < 40 * 1024);
}

/// Encodes `count` and raw runs the way the wire does, to try maps no honest laptop would send.
fn raw(count: u64, runs: &[(u64, u64)]) -> Vec<u8> {
    let mut w = count.to_be_bytes().to_vec();
    w.extend_from_slice(&u32::try_from(runs.len()).unwrap().to_be_bytes());
    for (s, e) in runs {
        w.extend_from_slice(&s.to_be_bytes());
        w.extend_from_slice(&e.to_be_bytes());
    }
    w
}

#[test]
fn a_map_that_does_not_add_up_is_refused() {
    assert!(BlockMap::decode(&raw(10, &[(0, 3), (5, 7)])).is_ok());
    for bad in [
        raw(10, &[(5, 7), (0, 3)]), // out of order
        raw(10, &[(0, 4), (3, 7)]), // overlapping
        raw(10, &[(0, 3), (3, 7)]), // touching: should have been one run
        raw(10, &[(4, 4)]),         // empty run
        raw(10, &[(6, 4)]),         // backwards
        raw(10, &[(8, 11)]),        // past the end of the file
        raw(10, &[(0, u64::MAX)]),  // far past the end
    ] {
        assert!(BlockMap::decode(&bad).is_err(), "{bad:?}");
    }
    let mut too_many = Vec::new();
    for i in 0..=MAX_TICKET_RUNS as u64 {
        too_many.push((3 * i, 3 * i + 1));
    }
    assert!(BlockMap::decode(&raw(u64::MAX, &too_many)).is_err());
    // Cut short, or with bytes left over.
    let good = raw(10, &[(0, 3)]);
    assert!(BlockMap::decode(&good[..good.len() - 1]).is_err());
    let mut extra = good.clone();
    extra.push(0);
    assert!(BlockMap::decode(&extra).is_err());
    // A count of runs that claims more than is there is refused without reading past the end.
    let mut lying = 10u64.to_be_bytes().to_vec();
    lying.extend_from_slice(&u32::MAX.to_be_bytes());
    assert!(BlockMap::decode(&lying).is_err());
}

proptest! {
    /// Any blocks done in any order: the runs are sorted, never touch, cover exactly the done
    /// blocks, the missing runs cover exactly the rest, and the map reads back the same.
    #[test]
    fn the_runs_are_exactly_the_done_blocks(
        count in 0u64..300,
        picks in proptest::collection::vec(0u64..320, 0..400),
    ) {
        let mut m = BlockMap::new(count);
        let mut done = vec![false; count as usize];
        for b in picks {
            let r = m.insert(b);
            if b >= count {
                prop_assert!(r.is_err());
            } else {
                prop_assert_eq!(r, Ok(!done[b as usize]));
                done[b as usize] = true;
            }
        }
        prop_assert_eq!(m.to_bools(), done.clone());
        prop_assert_eq!(m.is_full(), done.iter().all(|d| *d));
        prop_assert_eq!(m.done_count(), done.iter().filter(|d| **d).count() as u64);
        let mut last_end = None;
        for r in m.runs() {
            prop_assert!(r.start < r.end);
            if let Some(e) = last_end {
                prop_assert!(r.start > e, "runs touch or overlap");
            }
            last_end = Some(r.end);
        }
        let mut covered = vec![false; count as usize];
        for r in m.runs().chain(m.missing()) {
            for b in r {
                prop_assert!(!covered[b as usize]);
                covered[b as usize] = true;
            }
        }
        prop_assert!(covered.iter().all(|c| *c));
        for r in m.missing() {
            prop_assert!(r.clone().all(|b| !done[b as usize]));
        }
        prop_assert_eq!(BlockMap::decode(&m.encode()).unwrap(), m);
    }
}
