//! A path inside the move as the old laptop sends it: anything is accepted or refused, never a
//! crash. An accepted path never has an empty, "." or ".." part, and each part, once the gate
//! makes it storable for a system, is a plain name there: on Windows no character it forbids
//! (a backslash in a Linux name becomes a lookalike) and no trailing dot or space.
#![no_main]

use libfuzzer_sys::fuzz_target;
use pctwin_gate::{IncomingPath, MAX_COMPONENT_BYTES, Platform, convert_name};

/// What Windows forbids in a name, besides control characters.
const WINDOWS_FORBIDDEN: &str = "<>:\"/\\|?*";

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(path) = IncomingPath::parse(text) else {
        return;
    };
    for part in path.components() {
        assert!(!part.is_empty() && part != "." && part != "..", "{part:?}");
        assert!(!part.contains('/'), "{part:?}");
        for platform in [Platform::Windows, Platform::MacOs, Platform::Linux] {
            let name = convert_name(part, platform).name;
            assert!(!name.is_empty() && name != "." && name != "..", "{platform:?} {name:?}");
            assert!(!name.contains('/') && !name.contains('\0'), "{platform:?} {name:?}");
            assert!(name.len() <= MAX_COMPONENT_BYTES, "{platform:?} {name:?}");
            if platform == Platform::Windows {
                assert!(
                    !name
                        .chars()
                        .any(|c| WINDOWS_FORBIDDEN.contains(c) || u32::from(c) < 32),
                    "{name:?}"
                );
                assert!(!name.ends_with('.') && !name.ends_with(' '), "{name:?}");
            }
        }
    }
});
