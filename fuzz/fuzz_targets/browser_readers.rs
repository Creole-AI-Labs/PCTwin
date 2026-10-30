//! The browser readers the parser process runs (2.5 browsers): a bookmarks tree, an add-ons
//! list or manifest in JSON, a places database with its log, a property list of bookmarks and a
//! Mozilla session file, each on any bytes, through the parser's own code path. The first byte
//! picks the reader; the second says where the second file starts.
#![no_main]

use libfuzzer_sys::fuzz_target;
use pctwin_parse::testing::answer_in_memory;
use pctwin_parse::wire::Request;
use pctwin_special::browser_read::BrowserRead;

fuzz_target!(|data: &[u8]| {
    let Some((&pick, bytes)) = data.split_first() else {
        return;
    };
    let what = match pick % 4 {
        0 => BrowserRead::Json,
        1 => BrowserRead::Places,
        2 => BrowserRead::PropertyList,
        _ => BrowserRead::Session,
    };
    let size = bytes.len() as u64;
    let split = (data.get(1).copied().unwrap_or(255) as u64 * size / 255).min(size);
    let request = Request::Browser {
        size: split,
        second: size - split,
        what,
    };
    let answer = answer_in_memory(pctwin_parse_seal::in_child(), request, bytes);
    assert!(answer.answers(&request));
});
