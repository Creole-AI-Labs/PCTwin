//! Headless move for the test lab. Run the two sides as two user accounts:
//!
//!     pctwin-labmove send    <handoff> <report.json> [--lab <folder> <account>]
//!         (PCTWIN_LAB_BIND=<address:port> to listen on one address, as on two machines)
//!     pctwin-labmove receive <handoff> <report.json> <journal> [--lab <folder> <account>]
//!     pctwin-labmove check   <before.json> <source-after.json> <dest.json> <sent.json> <received.json> [--only <made.json>]
//!     pctwin-labmove fill-profile [seed] [ordinary files] [big file bytes]
//!     pctwin-labmove snapshot-profile <out.json>
//!     pctwin-labmove probe <out.json>
//!     pctwin-labmove pack <lab folder> <account> <package folder>
//!     pctwin-labmove send <handoff> <report.json> --package <package folder>
//!     pctwin-labmove everyone <report.json> --laptop <id> --new-home <folder> --journal <file>
//!         --new-names <prefix> [--session <token> --channel <name> [--dir <folder>]]
//!     pctwin-labmove everyone ... [--hostile yes]
//!     pctwin-labmove hello <token> <channel> [folder]
//!     pctwin-labmove check-everyone <key.json> <evidence.json> <what the mover can read>...
//!
//! `fill-profile` builds a lab laptop into the signed-in person's real special folders, wherever
//! they are; `snapshot-profile` records those folders the way reports name them. `probe` compares
//! where each special folder really is with the plain guess and checks item IDs stay the same
//! across two scans; it exits 1 on any disagreement.
//!
//! Without `--lab`, each side uses the signed-in person's real special folders. The handoff
//! folder must be writable by both accounts. `send` and `receive` exit 0 when the side finished
//! and reported every item. `check` takes `pctwin-lab snapshot` files and both reports, prints
//! what the move came to, and exits 1 with every problem if anything is wrong.

use std::path::PathBuf;
use std::process::ExitCode;

use pctwin_labmove::{
    Folders, Report, check_move, probe, profile_places, receive, send, snapshot_profile,
};

fn folders(rest: &[String]) -> Result<Folders, String> {
    match rest {
        [] => Ok(Folders::Profile),
        [flag, root, account] if flag == "--lab" => Ok(Folders::Lab {
            root: PathBuf::from(root),
            account: account.clone(),
        }),
        [flag, dir] if flag == "--package" => Ok(Folders::Package {
            dir: PathBuf::from(dir),
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
            // PCTWIN_LAB_BIND: the address to listen on (two machines on a virtual network, where
            // loopback is not shared); by default every address, reached through loopback.
            let bind = std::env::var("PCTWIN_LAB_BIND").unwrap_or_else(|_| "0.0.0.0:0".into());
            let r = send(folders(rest)?, handoff.as_ref(), &bind).await?;
            write(&r, out)?;
            Ok(true)
        }
        [side, handoff, out, journal, rest @ ..] if side == "receive" => {
            let r = receive(folders(rest)?, handoff.as_ref(), journal.as_ref()).await?;
            write(&r, out)?;
            Ok(true)
        }
        [side, before, after, dest, sent, received, rest @ ..] if side == "check" => {
            // `--only made.json`: judge only the lab's own files (`made[].path`).
            let only = match rest {
                [] => None,
                [flag, made] if flag == "--only" => {
                    let made: serde_json::Value = read(made)?;
                    Some(
                        made["made"]
                            .as_array()
                            .ok_or("the made file has no list of made files")?
                            .iter()
                            .filter_map(|m| m["path"].as_str().map(String::from))
                            .collect::<std::collections::BTreeSet<String>>(),
                    )
                }
                _ => return Err("expected --only <made.json> or nothing".into()),
            };
            match check_move(
                &read(before)?,
                &read(after)?,
                &read(dest)?,
                &read(sent)?,
                &read(received)?,
                only.as_ref(),
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
        [side, rest @ ..] if side == "fill-profile" => {
            let num = |i: usize, d: u64| -> Result<u64, String> {
                rest.get(i)
                    .map(|s| s.parse::<u64>().map_err(|e| format!("{s}: {e}")))
                    .unwrap_or(Ok(d))
            };
            let d = pctwin_lab::Spec::default();
            let spec = pctwin_lab::Spec {
                seed: num(0, d.seed)?,
                ordinary_files: usize::try_from(num(1, d.ordinary_files as u64)?)
                    .map_err(|e| e.to_string())?,
                big_file: num(2, d.big_file)?,
            };
            let places = profile_places();
            if places.is_empty() {
                return Err("this account has no special folders".into());
            }
            let home = places
                .values()
                .next()
                .and_then(|p| p.parent())
                .map(PathBuf::from)
                .ok_or("no home folder")?;
            let mut made =
                pctwin_lab::build_with(&home, &spec, &places).map_err(|e| e.to_string())?;
            let report =
                serde_json::json!({ "places": places, "made": made.made, "skipped": made.skipped });
            println!(
                "{}",
                serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
            );
            made.release().map_err(|e| e.to_string())?;
            Ok(true)
        }
        [side, out] if side == "snapshot-profile" => {
            let snap = snapshot_profile().map_err(|e| e.to_string())?;
            std::fs::write(
                out,
                serde_json::to_vec_pretty(&snap).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            Ok(true)
        }
        [side, root, account, out] if side == "pack" => {
            let held = pctwin_labmove::pack(root.as_ref(), account, out.as_ref())?;
            println!("packed {held} files");
            Ok(true)
        }
        [side, report, rest @ ..] if side == "everyone" => {
            // The mover's report to their own file; the lab's evidence on standard output, which
            // the lab keeps where only an administrator can read it.
            let ran =
                pctwin_labmove::everyone::everyone(pctwin_labmove::everyone::parse(rest)?).await?;
            std::fs::write(
                report,
                serde_json::to_vec_pretty(&ran.report).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            println!(
                "{}",
                serde_json::to_string_pretty(&ran.evidence).map_err(|e| e.to_string())?
            );
            Ok(true)
        }
        [side, token, channel, rest @ ..] if side == "hello" => {
            let dir = match rest {
                [] => None,
                [dir] => Some(std::path::Path::new(dir)),
                _ => return Err("hello <token> <channel> [folder]".into()),
            };
            pctwin_labmove::everyone::hello(token, channel, dir)?;
            Ok(true)
        }
        [side, key, evidence, visible @ ..] if side == "check-everyone" => {
            let key: serde_json::Value = read(key)?;
            let evidence: serde_json::Value = read(evidence)?;
            let mut seen = Vec::new();
            for v in visible {
                let text = std::fs::read(v).map_err(|e| format!("{v}: {e}"))?;
                seen.push((v.clone(), String::from_utf8_lossy(&text).into_owned()));
            }
            let problems = pctwin_labmove::everyone::check(&key, &evidence, &seen);
            for p in &problems {
                println!("PROBLEM: {p}");
            }
            Ok(problems.is_empty())
        }
        [side, out] if side == "probe" => {
            let (found, problems) = probe()?;
            let json = serde_json::json!({ "probe": found, "problems": problems });
            std::fs::write(
                out,
                serde_json::to_vec_pretty(&json).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            for p in &problems {
                println!("PROBLEM: {p}");
            }
            Ok(problems.is_empty())
        }
        _ => Err(
            "usage: pctwin-labmove send|receive|check|fill-profile|snapshot-profile|probe ..."
                .into(),
        ),
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
