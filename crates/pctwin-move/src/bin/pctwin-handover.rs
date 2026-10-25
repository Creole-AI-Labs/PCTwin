//! Hands another person's files over from the holding area into their own folders (see
//! `pctwin_move::handover`). Started by the administrator helper (as the person themselves on Mac
//! and Linux), or by the person's own sign-in once the move is over:
//!
//! `pctwin-handover --holding <folder> [--home <folder>] [--account <id>] [--journal <file>]
//!  [--into <label>=<folder name>...]`
//!
//! Without `--home`, the signed-in person's own home; without `--account`, them; without
//! `--journal`, `handover.journal` in the held folder; without `--into`, the usual places
//! ([`pctwin_move::handover::usual_places`]). It prints one line per file (`landed`, `kept`,
//! `failed` or `not-plain`, then the held path) and ends with 0 only if every file landed. Run
//! again, it hands over only what is still held.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;

use pctwin_journal::Journal;
use pctwin_transfer::ReceiveOutcome;

#[derive(Debug)]
struct Args {
    holding: PathBuf,
    home: PathBuf,
    account: String,
    journal: PathBuf,
    places: BTreeMap<String, String>,
}

/// One plain folder name inside the home, never a path.
fn is_folder_name(folder: &str) -> bool {
    !folder.is_empty() && folder != "." && folder != ".." && !folder.contains(['/', '\\', '\0'])
}

fn parse(
    mut args: impl Iterator<Item = String>,
    own_home: Option<PathBuf>,
) -> Result<Args, String> {
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
                if !is_folder_name(folder) {
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
    let holding = holding.ok_or("--holding is needed")?;
    Ok(Args {
        journal: journal.unwrap_or_else(|| holding.join("handover.journal")),
        holding,
        home: home
            .or(own_home)
            .ok_or("--home is needed: the signed-in person's home is not known")?,
        account: account.unwrap_or_else(|| "self".into()),
        places: if places.is_empty() {
            pctwin_move::handover::usual_places()
        } else {
            places
        },
    })
}

/// The signed-in person's own home, as their session says.
fn own_home() -> Option<PathBuf> {
    let var = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    std::env::var_os(var).map(PathBuf::from)
}

/// A folder named by the environment variable `var`, when it is set.
fn env_path(var: &str) -> Option<PathBuf> {
    std::env::var_os(var)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

/// The person's own app data folder, as the system names it.
fn data_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        env_path("APPDATA")
    } else if cfg!(target_os = "macos") {
        env_path("HOME").map(|h| h.join("Library").join("Application Support"))
    } else {
        env_path("XDG_DATA_HOME")
            .or_else(|| env_path("HOME").map(|h| h.join(".local").join("share")))
    }
}

/// Writes the note for the person, adding to one they have not read yet. The only file this
/// program writes itself: in the person's own app data folder, as them.
#[expect(
    clippy::disallowed_methods,
    reason = "the person's own note, in their own app data folder; not a move write"
)]
fn write_note(
    path: &std::path::Path,
    note: pctwin_move::handover::ReadyNote,
) -> std::io::Result<()> {
    use pctwin_move::handover::ReadyNote;
    let before = std::fs::read(path)
        .ok()
        .and_then(|b| ReadyNote::read(&b))
        .unwrap_or_default();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let bytes = serde_json::to_vec(&before.and(note)).map_err(std::io::Error::other)?;
    let tmp = path.with_extension("json.part");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

/// Opens PCTwin, when it is installed beside this program, so the person sees their note.
fn open_pctwin() {
    let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(PathBuf::from))
    else {
        return;
    };
    let names: &[&str] = if cfg!(windows) {
        &["PCTwin.exe", "pctwin-app.exe"]
    } else {
        &["PCTwin", "pctwin", "pctwin-app"]
    };
    if let Some(app) = names.iter().map(|n| dir.join(n)).find(|p| p.is_file()) {
        let _ = std::process::Command::new(app)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }
}

fn main() -> ExitCode {
    let args = match parse(std::env::args().skip(1), own_home()) {
        Ok(a) => a,
        Err(why) => {
            eprintln!("pctwin-handover: {why}");
            return ExitCode::from(2);
        }
    };
    if !args.holding.is_dir() {
        // Handed over already (or never held): nothing to do.
        return ExitCode::SUCCESS;
    }
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
    let mut note = pctwin_move::handover::ReadyNote::default();
    for f in &done.files {
        let word = match &f.outcome {
            Some(ReceiveOutcome::Finished(_)) => {
                note.landed += 1;
                "landed"
            }
            Some(ReceiveOutcome::NeedsAction { .. } | ReceiveOutcome::AlreadyThere(_)) => {
                all = false;
                note.kept += 1;
                "kept"
            }
            _ => {
                all = false;
                note.failed += 1;
                "failed"
            }
        };
        println!("{word}\t{}", f.held.display());
    }
    note.failed += u64::try_from(done.not_plain.len() + done.unfinished.len()).unwrap_or(u64::MAX);
    // For the person, once they are signed in: a note PCTwin shows when it opens, and PCTwin
    // opened for them (only at their own sign-in, never as the administrator).
    if args.account == "self"
        && let Some(data) = data_dir()
    {
        let path = pctwin_move::handover::ready_note_path(&data);
        if write_note(&path, note).is_ok() {
            open_pctwin();
        }
    }
    // A stopped move's unfinished file: never handed over, so it is not handed over now.
    for p in &done.unfinished {
        all = false;
        println!("unfinished\t{}", p.display());
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
        parse(v.iter().map(|s| s.to_string()), Some(PathBuf::from("own")))
    }

    #[test]
    fn the_command_line_is_read_strictly() {
        // Only the held folder is needed; the rest has the signed-in person's defaults.
        let short = a(&["--holding", "h"]).unwrap();
        assert_eq!(short.home, PathBuf::from("own"));
        assert_eq!(short.account, "self");
        assert_eq!(short.journal, PathBuf::from("h").join("handover.journal"));
        assert_eq!(short.places, pctwin_move::handover::usual_places());
        assert!(parse(["--holding", "h"].iter().map(|s| s.to_string()), None).is_err());
        assert!(a(&[]).is_err());
        let full = a(&[
            "--holding",
            "h",
            "--home",
            "m",
            "--account",
            "1",
            "--journal",
            "j",
            "--into",
            "documents=Documents",
        ])
        .unwrap();
        assert_eq!(full.places.len(), 1);
        assert!(a(&["--holding", "h", "--holding", "h2"]).is_err());
        assert!(
            a(&[
                "--holding",
                "h",
                "--into",
                "documents=A",
                "--into",
                "documents=B"
            ])
            .is_err()
        );
        assert!(a(&["--holding", "h", "--into", "../x=Documents"]).is_err());
        for folder in ["../etc", "a/b", "a\\b", ".", "..", ""] {
            let value = format!("documents={folder}");
            assert!(
                a(&["--holding", "h", "--into", &value]).is_err(),
                "{folder}"
            );
        }
        assert!(a(&["--holding", "h", "--run", "x"]).is_err());
    }
}
