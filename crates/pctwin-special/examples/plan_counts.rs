//! How long planning this laptop's own app data takes, read only. Prints counts and times only:
//! never a folder or file name, never a value.

use std::collections::BTreeMap;

use pctwin_catalogue::Catalogue;
use pctwin_record::Owner;
use pctwin_special::api::plan;
use pctwin_special::catalogue::{SHIPPED, SpecialRow};
use pctwin_special::roots::System;

fn main() {
    let home = std::path::PathBuf::from(
        std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .unwrap_or_default(),
    );
    let t = std::time::Instant::now();
    let p = plan(
        &Owner::Shared,
        &home,
        &[],
        System::here(),
        System::here(),
        &Catalogue::<SpecialRow>::shipped(SHIPPED),
    );
    let took = t.elapsed();
    let mut by_class: BTreeMap<String, usize> = BTreeMap::new();
    for c in &p.components {
        *by_class
            .entry(format!("{:?}", c.component.custody))
            .or_default() += 1;
    }
    let mut by_reason: BTreeMap<String, usize> = BTreeMap::new();
    for n in &p.not_moved {
        *by_reason.entry(n.why.clone()).or_default() += 1;
    }
    println!("planned in {took:?}");
    println!("app folders: {}", p.reports.len());
    println!(
        "components: {} (secrets {}, proposed {})",
        p.components.len(),
        p.components.iter().filter(|c| c.component.secret).count(),
        p.components.iter().filter(|c| c.proposed).count()
    );
    for (k, v) in by_class {
        println!("  {k}: {v}");
    }
    println!("found, not moved: {}", p.not_moved.len());
    for (k, v) in by_reason {
        println!("  {v} x {k}");
    }
    let c = &p.coverage;
    println!(
        "roots searched {}, absent {}, unreadable {}, caches {}, skipped {}",
        c.searched.len(),
        c.absent.len(),
        c.unreadable.len(),
        c.caches.len(),
        c.skipped.len()
    );
}
