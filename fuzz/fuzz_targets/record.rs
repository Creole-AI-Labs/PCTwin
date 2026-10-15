//! The record (the move's manifest) as JSON from the other laptop: any bytes are read as a record
//! or refused, never a crash; a record that is read saves to JSON that reads back the same.
#![no_main]

use libfuzzer_sys::fuzz_target;
use pctwin_record::Record;

fuzz_target!(|data: &[u8]| {
    if let Ok(r) = Record::from_json(data) {
        let again = Record::from_json(&r.to_json()).expect("a saved record reads back");
        assert_eq!(again, r);
    }
});
