//! Where planning this laptop's own app data spends its time, read only: each app folder by its
//! number and root kind, with seconds and counts. Never a folder or file name, never a value.

use std::io::Write;

use pctwin_record::Owner;
use pctwin_special::classify::classify_folder;
use pctwin_special::roots::{System, find_folders, roots_of};

fn main() {
    let home = std::path::PathBuf::from(
        std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .unwrap_or_default(),
    );
    let t = std::time::Instant::now();
    let roots = roots_of(System::here(), &home, &|v| {
        std::env::var_os(v).map(std::path::PathBuf::from)
    });
    let (folders, cov) = find_folders(&Owner::Shared, &home, &roots, &[]);
    println!(
        "found {} app folders in {:?} (skipped {}, unreadable {})",
        folders.len(),
        t.elapsed(),
        cov.skipped.len(),
        cov.unreadable.len()
    );
    let mut slowest = std::time::Duration::ZERO;
    for (i, f) in folders.iter().enumerate() {
        print!("{i} {:?} ... ", f.root);
        let _ = std::io::stdout().flush();
        let s = std::time::Instant::now();
        let r = classify_folder(f, None);
        let took = s.elapsed();
        slowest = slowest.max(took);
        let not_read = r
            .not_moved
            .iter()
            .filter(|n| n.why.words().starts_with("found, not fully checked"))
            .count();
        println!(
            "{:.1}s components {} not moved {} not fully checked {}",
            took.as_secs_f64(),
            r.components.len(),
            r.not_moved.len(),
            not_read
        );
    }
    println!("all in {:?}, slowest folder {:?}", t.elapsed(), slowest);
}
