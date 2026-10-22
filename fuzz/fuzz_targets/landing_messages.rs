//! The app's move messages from the other laptop (what it is, the mover, the other people, the
//! landing rules) and the hand-over's note: any bytes are read as one of them or refused, never a crash,
//! and what is read writes back to something that reads the same.
#![no_main]

use libfuzzer_sys::fuzz_target;
use pctwin_move::greeting::parse_about;
use pctwin_move::handover::ReadyNote;
use pctwin_move::landing::{parse_mover, parse_people, parse_rules};

fuzz_target!(|data: &[u8]| {
    if let Ok(about) = parse_about(data) {
        let mut again = vec![data[0]];
        again.extend(serde_json::to_vec(&about).unwrap());
        assert_eq!(parse_about(&again).unwrap(), about, "what it is read back differently");
    }
    if let Ok(mover) = parse_mover(data) {
        assert!(!mover.is_empty() && !mover.chars().any(char::is_control));
    }
    if let Ok(people) = parse_people(data) {
        assert!(people.len() <= pctwin_move::landing::MAX_PEOPLE);
    }
    if let Ok(rules) = parse_rules(data) {
        let mut again = vec![data[0]];
        again.extend(serde_json::to_vec(&rules).unwrap());
        assert_eq!(parse_rules(&again).unwrap(), rules, "rules read back differently");
    }
    if let Some(note) = ReadyNote::read(data) {
        assert_eq!(ReadyNote::read(&serde_json::to_vec(&note).unwrap()), Some(note));
    }
});
