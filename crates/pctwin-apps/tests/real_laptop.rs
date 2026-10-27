//! Reads the apps on the laptop the test runs on (read only: nothing is installed, removed or
//! changed) and prints counts only, never names. Run by hand:
//! `cargo test -p pctwin-apps --test real_laptop -- --ignored --nocapture`.

#![cfg(windows)]

use std::process::Command;

use pctwin_record::{AppMaker, FoundWhere};

fn me() -> String {
    let out = Command::new(r"C:\Windows\System32\whoami.exe")
        .args(["/user", "/fo", "csv", "/nh"])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    text.trim()
        .rsplit(',')
        .next()
        .unwrap()
        .trim_matches('"')
        .to_string()
}

#[test]
#[ignore = "reads this laptop's apps; run by hand"]
fn this_laptops_apps_are_all_found_with_their_makers() {
    let me = me();
    assert!(me.starts_with("S-1-5-"), "{me}");
    let list = pctwin_apps::found::windows::read_windows_apps(
        std::slice::from_ref(&me),
        &me,
        &pctwin_maker::run::SystemRunner,
    );
    list.validate().unwrap();
    let count =
        |f: &dyn Fn(&pctwin_record::FoundApp) -> bool| list.found.iter().filter(|a| f(a)).count();
    println!("apps found: {}", list.found.len());
    println!(
        "  programs: {}  packaged: {}",
        count(&|a| matches!(a.found, FoundWhere::WindowsEntry { .. })),
        count(&|a| matches!(a.found, FoundWhere::WindowsPackage { .. }))
    );
    println!(
        "  maker signed: {}  package publisher: {}  unsigned: {}  not read: {}",
        count(&|a| matches!(a.maker, AppMaker::Windows(_))),
        count(&|a| matches!(a.maker, AppMaker::WindowsPackage { .. })),
        count(&|a| matches!(a.maker, AppMaker::Unsigned)),
        count(&|a| matches!(a.maker, AppMaker::NotRead { .. }))
    );
    println!(
        "  with a web address: {}  with a known processor: {}",
        count(&|a| a.web.is_some()),
        count(&|a| a.arch.known)
    );
    let mut why: std::collections::BTreeMap<String, usize> = Default::default();
    for a in &list.found {
        if let AppMaker::NotRead { why: w } = &a.maker {
            // Only the kind of reason: anything in brackets (a path) is cut off.
            let inner = w.split_once('(').map_or("", |x| x.1);
            let inner = if inner.contains(":\\") {
                "(a path)"
            } else {
                inner
            };
            *why.entry(format!(
                "{} {}",
                w.split('(').next().unwrap_or("").trim(),
                inner
            ))
            .or_default() += 1;
        }
    }
    println!("  why not read: {why:?}");
    println!("places not read: {}", list.not_read.len());
    assert!(!list.found.is_empty());
}

/// The download program, locked down at low integrity, cannot write a file in the person's own
/// folders. Run by hand (it needs the program built: `cargo build -p pctwin-apps --bin
/// pctwin-fetch`).
#[test]
#[ignore = "starts the download program; run by hand"]
fn the_download_program_cannot_write_the_persons_files() {
    let program = std::path::PathBuf::from(env!("CARGO_BIN_EXE_pctwin-fetch"));
    let home = pctwin_apps::fetch::process::default_home()
        .unwrap()
        .join("test");
    let f = pctwin_apps::fetch::process::Fetcher::new(program, home).unwrap();
    let mine = std::path::PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap())
        .join(format!("pctwin-probe-{}.txt", std::process::id()));
    let wrote = f.could_write(&mine).unwrap();
    let _ = std::fs::remove_file(&mine);
    println!("download program could write the person's folder: {wrote}");
    assert!(!wrote);
    // And it can write its own staging.
    let own = f.staging().join("probe.txt");
    assert!(f.could_write(&own).unwrap());
}

/// A real download through the locked-down download program: the public rules (https, public
/// addresses, the system's trust store), the bytes winget's manifest names, and the mark of the
/// web kept on the file. Run by hand.
#[test]
#[ignore = "downloads from the internet; run by hand"]
fn the_download_program_fetches_exact_bytes_and_marks_them() {
    use pctwin_apps::fetch::{Expected, Outcome, Request};
    let program = std::path::PathBuf::from(env!("CARGO_BIN_EXE_pctwin-fetch"));
    let home = pctwin_apps::fetch::process::default_home()
        .unwrap()
        .join("test-fetch");
    let f = pctwin_apps::fetch::process::Fetcher::new(program, home).unwrap();
    let file = f.staging().join("putty.msi");
    let _ = std::fs::remove_file(&file);
    let req = Request {
        url: "https://the.earth.li/~sgtatham/putty/0.85/w64/putty-64bit-0.85-installer.msi".into(),
        also_hosts: vec![],
        max_bytes: 64 * 1024 * 1024,
        expected: Some(Expected::Sha256(
            "2213ebacf962b709411d654a40087508a694756bc4e41c5a1c24ad86596b2511".into(),
        )),
        file: file.clone(),
        allow_metered: true,
    };
    let got = f.fetch(&req, std::time::Duration::from_secs(600)).unwrap();
    println!("download program: {got:?}");
    assert!(matches!(got, Outcome::Done { .. }), "{got:?}");
    assert!(pctwin_apps::fetch::marks::marked(&file));
    // The same address with other bytes named: refused, nothing kept.
    let mut wrong = req.clone();
    wrong.expected = Some(Expected::Sha256("00".repeat(32)));
    wrong.file = f.staging().join("wrong.msi");
    assert_eq!(
        f.fetch(&wrong, std::time::Duration::from_secs(600))
            .unwrap(),
        Outcome::NotExpectedBytes
    );
    assert!(!wrong.file.exists());
    // A private address is refused before any connection.
    let mut private = req.clone();
    private.url = "https://169.254.169.254/latest/meta-data/".into();
    private.file = f.staging().join("meta.bin");
    assert!(matches!(
        f.fetch(&private, std::time::Duration::from_secs(60))
            .unwrap(),
        Outcome::Refused { .. }
    ));
    let _ = std::fs::remove_file(&file);
}
