//! The Apple Mail index reader the parser process runs on the new laptop (2.5 mail): any bytes
//! as a received mail index and its log, through the parser's own code path. The first byte says
//! where the log starts. Whatever the bytes, the answer is of the kind asked, and every account
//! folder it names is within its bounds (the engine only compares those names; it never builds a
//! path from them).
#![no_main]

use libfuzzer_sys::fuzz_target;
use pctwin_parse::testing::answer_in_memory;
use pctwin_parse::wire::{Answer, Request};
use pctwin_special::applemail::{MAX_ACCOUNTS, MAX_NAME};

fuzz_target!(|data: &[u8]| {
    let Some((&cut, bytes)) = data.split_first() else {
        return;
    };
    let size = bytes.len() as u64;
    let split = (cut as u64 * size / 255).min(size);
    let request = Request::MailKinds {
        size: split,
        log: size - split,
    };
    let answer = answer_in_memory(pctwin_parse_seal::in_child(), request, bytes);
    assert!(answer.answers(&request));
    if let Answer::MailKinds(Ok(kinds)) = answer {
        assert!(kinds.len() <= MAX_ACCOUNTS);
        assert!(kinds.iter().all(|k| !k.folder.is_empty()
            && k.folder.len() <= MAX_NAME
            && !k.folder.chars().any(char::is_control)));
    }
});
