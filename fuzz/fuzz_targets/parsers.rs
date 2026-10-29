//! Every reader the parser process runs (adoption change 1): an installer's signer, the old
//! laptop's own copy, a font's names, a ZIP index, a SQLite header, a JSON file's facts, a
//! database with its log, each on any bytes, read
//! piece by piece through the parser's own code path. The first byte picks the reader.
#![no_main]

use libfuzzer_sys::fuzz_target;
use pctwin_parse::testing::answer_in_memory;
use pctwin_parse::wire::Request;

fuzz_target!(|data: &[u8]| {
    let Some((&pick, bytes)) = data.split_first() else {
        return;
    };
    let size = bytes.len() as u64;
    let request = match pick % 7 {
        0 => Request::Installer { size },
        1 => Request::OldCopy { size },
        2 => Request::Font { size },
        3 => Request::Archive { size },
        4 => Request::Database { size },
        5 => Request::Json { size },
        // A database and its log: the second byte says where the log starts.
        _ => {
            let split = (data.get(1).copied().unwrap_or(0) as u64 * size / 255).min(size);
            Request::DatabaseFacts {
                size: split,
                log: size - split,
                millis: 1000,
            }
        }
    };
    let answer = answer_in_memory(pctwin_parse_seal::in_child(), request, bytes);
    assert!(answer.answers(&request));
});
