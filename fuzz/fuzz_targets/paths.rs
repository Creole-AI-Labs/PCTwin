//! A path inside the move as the old laptop sends it: anything is accepted as a safe path or
//! refused, never a crash, and an accepted path never climbs out or names a drive.
#![no_main]

use libfuzzer_sys::fuzz_target;
use pctwin_gate::IncomingPath;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    if let Ok(path) = IncomingPath::parse(text) {
        for part in path.components() {
            assert!(!part.is_empty() && part != "." && part != "..", "{part:?}");
            assert!(!part.contains('/') && !part.contains('\\'), "{part:?}");
        }
    }
});
