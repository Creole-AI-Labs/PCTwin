//! Pairing messages from a stranger on the network: whatever arrives as message 1 at the new
//! laptop, or as message 2 at the old laptop, is refused or handled, never a crash.
#![no_main]

use std::time::Instant;

use libfuzzer_sys::fuzz_target;
use pctwin_pairing::{PairingCode, ReceiverSession, SenderSession};

fuzz_target!(|data: &[u8]| {
    let code = PairingCode::parse("123456").expect("a valid code");
    let now = Instant::now();
    let _ = ReceiverSession::respond(&code, data, now);
    let (sender, _msg1) = SenderSession::start(&code, now);
    let _ = sender.receive(data, now);
});
