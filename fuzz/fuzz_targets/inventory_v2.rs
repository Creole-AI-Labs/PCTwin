//! The scan inventory, version 2, from the other laptop: any manifest and pages are read or
//! refused, never a crash; an inventory that is read encodes back to exactly the same bytes.
#![no_main]

use libfuzzer_sys::fuzz_target;
use pctwin_record::{EncodedInventory, decode_inventory, encode_inventory};

fuzz_target!(|data: &[u8]| {
    // The first two bytes say where the manifest ends; the rest is cut into pages at 0xFF bytes.
    if data.len() < 2 {
        return;
    }
    let split = (usize::from(data[0]) << 8 | usize::from(data[1])).min(data.len() - 2);
    let (manifest, rest) = data[2..].split_at(split);
    let encoded = EncodedInventory {
        manifest: manifest.to_vec(),
        pages: rest.split(|b| *b == 0xFF).map(<[u8]>::to_vec).collect(),
    };
    if let Ok((laptop, items, fingerprint)) = decode_inventory(&encoded) {
        let again = encode_inventory(&laptop, &items).expect("what was read encodes");
        assert_eq!(again.fingerprint(), fingerprint, "one encoding of each inventory");
    }
});
