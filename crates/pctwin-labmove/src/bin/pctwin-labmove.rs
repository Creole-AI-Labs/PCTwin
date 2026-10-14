//! Headless move for the test lab. Run the two sides as two user accounts:
//!
//!     pctwin-labmove send    <handoff> <report.json> [--lab <folder> <account>]
//!     pctwin-labmove receive <handoff> <report.json> <journal> [--lab <folder> <account>]
//!     pctwin-labmove check   <before.json> <source-after.json> <dest.json> <sent.json> <received.json>
//!
//! Without `--lab`, each side uses the signed-in person's real special folders. The handoff
//! folder must be writable by both accounts. `send` and `receive` exit 0 when the side finished
//! and reported every item. `check` takes `pctwin-lab snapshot` files and both reports, prints
//! what the move came to, and exits 1 with every problem if anything is wrong.

use std::path::PathBuf;
use std::process::ExitCode;

use pctwin_labmove::{Folders, Report, check_move, receive, send};

fn folders(rest: &[String]) -> Result<Folders, String> {
    match rest {
        [] => Ok(Folders::Profile),
        [flag, root, account] if flag == "--lab" => Ok(Folders::Lab {
            root: PathBuf::from(root),
            account: account.clone(),
        }),
        _ => Err("expected --lab <folder> <account> or nothing".into()),
    }
}

fn read<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("{path}: {e}"))
}

fn write(report: &Report, out: &str) -> Result<(), String> {
    let json = serde_json::to_vec_pretty(report).map_err(|e| e.to_string())?;
    std::fs::write(out, json).map_err(|e| e.to_string())
}

async fn run(args: &[String]) -> Result<bool, String> {
    match args {
        [side, handoff, out, rest @ ..] if side == "send" => {
            let r = send(folders(rest)?, handoff.as_ref(), "0.0.0.0:0").await?;
            write(&r, out)?;
            Ok(true)
        }
        [side, handoff, out, journal, rest @ ..] if side == "receive" => {
            let r = receive(folders(rest)?, handoff.as_ref(), journal.as_ref()).await?;
            write(&r, out)?;
            Ok(true)
        }
        [side, before, after, dest, sent, received] if side == "check" => {
            match check_move(
                &read(before)?,
                &read(after)?,
                &read(dest)?,
                &read(sent)?,
                &read(received)?,
            ) {
                Ok(checked) => {
                    println!(
                        "{}",
                        serde_json::to_string(&checked).map_err(|e| e.to_string())?
                    );
                    Ok(true)
                }
                Err(problems) => {
                    for p in problems {
                        println!("PROBLEM: {p}");
                    }
                    Ok(false)
                }
            }
        }
        _ => Err("usage: pctwin-labmove send|receive|check ...".into()),
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args).await {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
