//! Headless move for the test lab. Run the two sides as two user accounts:
//!
//!     pctwin-labmove send    <handoff> <report.json> [--lab <folder> <account>]
//!     pctwin-labmove receive <handoff> <report.json> <journal> [--lab <folder> <account>]
//!
//! Without `--lab`, each side uses the signed-in person's real special folders. The handoff
//! folder must be writable by both accounts. Exit 0 when the side finished and reported every
//! item, 1 if it could not finish.

use std::path::PathBuf;
use std::process::ExitCode;

use pctwin_labmove::{Folders, receive, send};

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

async fn run(args: &[String]) -> Result<(), String> {
    let report = match args {
        [side, handoff, out, rest @ ..] if side == "send" => {
            let r = send(folders(rest)?, handoff.as_ref(), "0.0.0.0:0").await?;
            (r, out)
        }
        [side, handoff, out, journal, rest @ ..] if side == "receive" => {
            let r = receive(folders(rest)?, handoff.as_ref(), journal.as_ref()).await?;
            (r, out)
        }
        _ => return Err("usage: pctwin-labmove send|receive ...".into()),
    };
    let json = serde_json::to_vec_pretty(&report.0).map_err(|e| e.to_string())?;
    std::fs::write(report.1, json).map_err(|e| e.to_string())
}

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
