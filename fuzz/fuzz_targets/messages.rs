//! The move's messages from the other laptop: any bytes decode to a message or an error, never a
//! crash, and a message that decodes encodes back to the very same bytes.
#![no_main]

use libfuzzer_sys::fuzz_target;
use pctwin_transfer::Message;

fuzz_target!(|data: &[u8]| {
    if let Ok(m) = Message::decode(data) {
        assert_eq!(m.encode(), data, "a message reads back to other bytes");
    }
});
