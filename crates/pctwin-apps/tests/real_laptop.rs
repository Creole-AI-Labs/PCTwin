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
