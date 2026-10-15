//! The old laptop's list arrives in frames: any bytes decode to a frame or an error, never a
//! crash, and a frame that decodes encodes back to the same bytes.
#![no_main]

use libfuzzer_sys::fuzz_target;
use pctwin_transfer::InventoryFrame;

fuzz_target!(|data: &[u8]| {
    if let Ok(f) = InventoryFrame::decode(data) {
        assert_eq!(f.encode(), data);
    }
});
