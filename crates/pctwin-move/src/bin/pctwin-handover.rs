//! Hands another person's files over from the holding area into their own folders (see
//! `pctwin_move::handover`). Started by the administrator helper:
//!
//! `pctwin-handover --holding <folder> --home <folder> --account <id> --journal <file>
//!  --into <label>=<folder name>...`
//!
//! On Mac and Linux it runs as the person themselves. It prints one line per file (`landed`,
//! `kept` or `failed`, then the held path) and ends with 0 only if every file landed.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;

use pctwin_journal::Journal;
use pctwin_transfer::ReceiveOutcome;

struct Args {
    holding: PathBuf,
    home: PathBuf,
    account: String,
    journal: PathBuf,
    places: BTreeMap<String, String>,
}

fn parse(mut args: impl Iterator<Item = String>) -> Result<Args, String> {
    let (mut holding, mut home, mut account, mut journal) = (None, None, None, None);
    let mut places = BTreeMap::new();
    while let Some(flag) = args.next() {
        let value = args.next().ok_or(format!("{flag} needs a value"))?;
        match flag.as_str() {
            "--holding" if holding.is_none() => holding = Some(PathBuf::from(value)),
            "--home" if home.is_none() => home = Some(PathBuf::from(value)),
            "--account" if account.is_none() => account = Some(value),
            "--journal" if journal.is_none() => journal = Some(PathBuf::from(value)),
            "--into" => {
                let (label, folder) = value
                    .split_once('=')
                    .ok_or("--into takes <label>=<folder name>")?;
                // One plain folder name inside the home, never a path.
                if folder.is_empty()
                    || folder == "."
                    || folder == ".."
                    || folder.contains(['/', '\\', '\0'])
                {
                    return Err(format!("--into {label}: {folder} is not one folder name"));
                }
                if label.is_empty()
                    || !label.bytes().all(|b| b.is_ascii_lowercase() || b == b'-')
                    || places
                        .insert(label.to_string(), folder.to_string())
                        .is_some()
                {
                    return Err(format!(
                        "--into {label} is not a place label, or given twice"
                    ));
                }
            }
            _ => return Err(format!("unknown or repeated {flag}")),
        }
    }
    if places.is_empty() {
        return Err("--into is needed".into());
    }
    Ok(Args {
        holding: holding.ok_or("--holding is needed")?,
        home: home.ok_or("--home is needed")?,
        account: account.ok_or("--account is needed")?,
        journal: journal.ok_or("--journal is needed")?,
        places,
    })
}

fn main() -> ExitCode {
    let args = match parse(std::env::args().skip(1)) {
        Ok(a) => a,
        Err(why) => {
            eprintln!("pctwin-handover: {why}");
            return ExitCode::from(2);
        }
    };
    let journal = match Journal::open(&args.journal) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("pctwin-handover: the change journal: {e}");
            return ExitCode::FAILURE;
        }
    };
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("pctwin-handover: {e}");
            return ExitCode::FAILURE;
        }
    };
    let done = match runtime.block_on(pctwin_move::handover::hand_over(
        &args.holding,
        &args.home,
        &args.places,
        &journal,
        &args.account,
    )) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("pctwin-handover: {e}");
            return ExitCode::FAILURE;
        }
    };
    let mut all = true;
    for f in &done.files {
        let word = match &f.outcome {
            Some(ReceiveOutcome::Finished(_)) => "landed",
            Some(ReceiveOutcome::NeedsAction { .. } | ReceiveOutcome::AlreadyThere(_)) => {
                all = false;
                "kept"
            }
            _ => {
                all = false;
                "failed"
            }
        };
        println!("{word}\t{}", f.held.display());
    }
    for p in &done.not_plain {
        all = false;
        println!("not-plain\t{}", p.display());
    }
    if all {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a(v: &[&str]) -> Result<Args, String> {
        parse(v.iter().map(|s| s.to_string()))
    }

    const BASE: [&str; 8] = [
        "--holding",
        "h",
        "--home",
        "m",
        "--account",
        "1001",
        "--journal",
        "j",
    ];

    fn with(extra: &[&str]) -> Vec<&'static str> {
        let mut v: Vec<&str> = BASE.to_vec();
        v.extend(
            extra
                .iter()
                .map(|s| -> &'static str { Box::leak(s.to_string().into_boxed_str()) }),
        );
        v
    }

    #[test]
    fn the_command_line_is_read_strictly() {
        assert!(a(&with(&["--into", "documents=Documents"])).is_ok());
        // A place is needed, and each part once.
        assert!(a(&BASE).is_err());
        assert!(a(&with(&["--into", "documents=Documents", "--home", "n"])).is_err());
        assert!(
            a(&with(&[
                "--into",
                "documents=Documents",
                "--into",
                "documents=Docs"
            ]))
            .is_err()
        );
        // A label is plain; a folder is one name inside the home, never a path.
        assert!(a(&with(&["--into", "../x=Documents"])).is_err());
        for folder in ["../etc", "a/b", "a\\b", ".", "..", ""] {
            let value = format!("documents={folder}");
            assert!(a(&with(&["--into", &value])).is_err(), "{folder}");
        }
        assert!(a(&with(&["--into", "documents=Documents", "--run", "x"])).is_err());
    }
}
