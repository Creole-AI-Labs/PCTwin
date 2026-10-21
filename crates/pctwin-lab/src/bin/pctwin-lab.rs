//! Test-lab command line, for CI scripts:
//!
//!     pctwin-lab make <folder> [seed] [ordinary files] [big file bytes]
//!     pctwin-lab snapshot <folder> <out.json>
//!     pctwin-lab compare <before.json> <after.json>
//!     pctwin-lab plan <seed> <people> <spec.json>
//!     pctwin-lab plant <spec.json> <roots.json> <person label>
//!     pctwin-lab unlock <spec.json> <roots.json> <person label>
//!     pctwin-lab check-sweep <spec.json> <roots.json> <sweep.json> [<observed.json>]
//!
//! `plan` writes a seeded lab laptop (people and planted things, with the answer key); `plant`
//! makes one person's planted things, run as that person so the system records them as the
//! owner; `unlock` opens that person's refused folders again for clean-up and snapshots;
//! `check-sweep` checks a `pctwin-sweep` result against the answer key and prints each problem
//! (exit 1 if any). Run it as an administrator, who may look everywhere for the "no invented
//! items" check.
//!
//! `make` prints what it made and skipped as JSON. `compare` prints each difference and exits 1
//! if there is any.

use std::path::Path;
use std::process::ExitCode;

use pctwin_lab::{Snapshot, Spec, build, compare, snapshot};

fn arg_num(args: &[String], i: usize, default: u64) -> Result<u64, String> {
    args.get(i)
        .map(|s| s.parse::<u64>().map_err(|e| format!("{s}: {e}")))
        .unwrap_or(Ok(default))
}

fn run(args: &[String]) -> Result<ExitCode, String> {
    let s = |e: std::io::Error| e.to_string();
    match args.first().map(String::as_str) {
        Some("make") => {
            let dir = args.get(1).ok_or("make needs a folder")?;
            let d = Spec::default();
            let spec = Spec {
                seed: arg_num(args, 2, d.seed)?,
                ordinary_files: usize::try_from(arg_num(args, 3, d.ordinary_files as u64)?)
                    .map_err(|e| e.to_string())?,
                big_file: arg_num(args, 4, d.big_file)?,
            };
            std::fs::create_dir_all(dir).map_err(s)?;
            let mut made = build(Path::new(dir), &spec).map_err(s)?;
            let report = serde_json::json!({ "made": made.made, "skipped": made.skipped });
            println!(
                "{}",
                serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
            );
            // A command-line run cannot keep files locked after it ends.
            made.release().map_err(s)?;
            Ok(ExitCode::SUCCESS)
        }
        Some("snapshot") => {
            let (dir, out) = (
                args.get(1).ok_or("snapshot needs a folder")?,
                args.get(2).ok_or("snapshot needs an output file")?,
            );
            let snap = snapshot(Path::new(dir)).map_err(s)?;
            let json = serde_json::to_vec_pretty(&snap).map_err(|e| e.to_string())?;
            std::fs::write(out, json).map_err(s)?;
            Ok(ExitCode::SUCCESS)
        }
        Some("compare") => {
            let read = |p: &String| -> Result<Snapshot, String> {
                let bytes = std::fs::read(p).map_err(s)?;
                serde_json::from_slice(&bytes).map_err(|e| format!("{p}: {e}"))
            };
            let before = read(args.get(1).ok_or("compare needs two files")?)?;
            let after = read(args.get(2).ok_or("compare needs two files")?)?;
            let diff = compare(&before, &after);
            for line in &diff {
                println!("{line}");
            }
            Ok(if diff.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            })
        }
        Some("plan") => {
            let seed = arg_num(args, 1, 1)?;
            let people = usize::try_from(arg_num(args, 2, 1)?).map_err(|e| e.to_string())?;
            let out = args.get(3).ok_or("plan needs an output file")?;
            let spec = pctwin_lab::plants::plan(seed, people);
            let json = serde_json::to_vec_pretty(&spec).map_err(|e| e.to_string())?;
            std::fs::write(out, json).map_err(s)?;
            Ok(ExitCode::SUCCESS)
        }
        Some(cmd @ ("plant" | "unlock")) => {
            let spec: pctwin_lab::plants::LaptopSpec = read_json(args.get(1))?;
            let roots: pctwin_lab::plants::Roots = read_json(args.get(2))?;
            let who = args.get(3).ok_or("which person?")?;
            if cmd == "plant" {
                for made in pctwin_lab::plants::plant(&spec, &roots, Some(who)).map_err(s)? {
                    println!("{}", made.display());
                }
            } else {
                for p in spec.plants.iter().filter(|p| p.by == *who) {
                    if let Some(top) = roots.top_of(p) {
                        pctwin_lab::plants::unlock(&top).map_err(s)?;
                    }
                }
            }
            Ok(ExitCode::SUCCESS)
        }
        Some("check-sweep") => {
            let spec: pctwin_lab::plants::LaptopSpec = read_json(args.get(1))?;
            let roots: pctwin_lab::plants::Roots = read_json(args.get(2))?;
            let sweep: serde_json::Value = read_json(args.get(3))?;
            let observed: pctwin_lab::plants::Observed = match args.get(4) {
                Some(_) => read_json(args.get(4))?,
                None => Default::default(),
            };
            let exists = |p: &str| {
                let p = if cfg!(windows) {
                    p.replace('/', "\\")
                } else {
                    p.to_string()
                };
                std::fs::symlink_metadata(&p).is_ok()
            };
            let problems = pctwin_lab::plants::check(&spec, &roots, &sweep, &observed, &exists);
            for line in &problems {
                println!("{line}");
            }
            println!(
                "{} planted things checked, {} problems",
                spec.plants.len(),
                problems.len()
            );
            Ok(if problems.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            })
        }
        _ => {
            Err("usage: pctwin-lab make|snapshot|compare|plan|plant|unlock|check-sweep ...".into())
        }
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: Option<&String>) -> Result<T, String> {
    let path = path.ok_or("a file is missing")?;
    let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("{path}: {e}"))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    run(&args).unwrap_or_else(|e| {
        eprintln!("{e}");
        ExitCode::from(2)
    })
}
