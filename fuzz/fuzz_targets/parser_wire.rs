//! What the app reads from the parser process: any bytes it says are read as frames of at most
//! the limit, and each frame's body as the typed messages, never anything else.
#![no_main]

use libfuzzer_sys::fuzz_target;
use pctwin_parse::wire::{Answer, Hello, MAX_FRAME, ReadAt, Request, json, read_frame};

fuzz_target!(|data: &[u8]| {
    let mut r = data;
    while let Ok(Some((_, body))) = read_frame(&mut r, MAX_FRAME) {
        assert!(body.len() <= MAX_FRAME);
        let _ = json::<Answer>(&body);
        let _ = json::<ReadAt>(&body);
        let _ = json::<Hello>(&body);
        let _ = json::<Request>(&body);
    }
});
