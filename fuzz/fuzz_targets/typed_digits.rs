//! What a person types as a pairing code: read as six digits or refused, never a crash.
#![no_main]

use libfuzzer_sys::fuzz_target;
use pctwin_pairing::PairingCode;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    if let Ok(code) = PairingCode::parse(text) {
        let digits = code.digits();
        assert_eq!(digits.len(), 6);
        assert!(digits.bytes().all(|b| b.is_ascii_digit()));
    }
});
