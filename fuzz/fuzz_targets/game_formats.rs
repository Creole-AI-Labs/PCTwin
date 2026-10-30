//! The game readers the parser process runs (2.5 games): a launcher's record in Valve's text
//! key-values or JSON, and a save list in the Ludusavi manifest format, each on any bytes, read
//! piece by piece through the parser's own code path. The first byte picks the reader.
#![no_main]

use libfuzzer_sys::fuzz_target;
use pctwin_parse::testing::answer_in_memory;
use pctwin_parse::wire::Request;
use pctwin_special::gamefmt::RecordFormat;

fuzz_target!(|data: &[u8]| {
    let Some((&pick, bytes)) = data.split_first() else {
        return;
    };
    let size = bytes.len() as u64;
    let request = match pick % 3 {
        0 => Request::GameRecord {
            size,
            format: RecordFormat::Vdf,
        },
        1 => Request::GameRecord {
            size,
            format: RecordFormat::Json,
        },
        // The games asked about, then the list: the second byte says where the list starts.
        _ => {
            let split = (data.get(1).copied().unwrap_or(0) as u64 * size / 255).min(size);
            Request::SaveManifest {
                wanted: split,
                size: size - split,
            }
        }
    };
    let answer = answer_in_memory(pctwin_parse_seal::in_child(), request, bytes);
    assert!(answer.answers(&request));
});
