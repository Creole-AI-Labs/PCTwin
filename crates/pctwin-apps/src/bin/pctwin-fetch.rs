//! PCTwin's download program: run by the app, locked down, for one download at a time
//! (`pctwin_apps::fetch`). `pctwin-fetch fetch <staging>` reads one request on standard input and
//! writes one answer on standard output. `pctwin-fetch probe <file>` tries to write `file` and
//! says whether it could (the lock-down's own test).

use std::io::{Read, Write};
use std::path::PathBuf;

fn main() {
    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    let answer = match args.first().and_then(|a| a.to_str()) {
        Some("fetch") if args.len() == 2 => {
            let staging = PathBuf::from(&args[1]);
            let mut input = Vec::new();
            let _ = std::io::stdin()
                .take(pctwin_apps::fetch::process::MAX_REQUEST as u64 + 1)
                .read_to_end(&mut input);
            let outcome = pctwin_apps::fetch::process::serve(&staging, &input);
            serde_json::to_vec(&outcome).unwrap_or_default()
        }
        Some("probe") if args.len() == 2 => {
            let target = PathBuf::from(&args[1]);
            match std::fs::write(&target, b"probe") {
                Ok(()) => {
                    let _ = std::fs::remove_file(&target);
                    b"wrote".to_vec()
                }
                Err(_) => b"denied".to_vec(),
            }
        }
        _ => b"{\"outcome\":\"bad\",\"why\":\"unknown command\"}".to_vec(),
    };
    let mut out = std::io::stdout();
    let _ = out.write_all(&answer);
    let _ = out.flush();
}
