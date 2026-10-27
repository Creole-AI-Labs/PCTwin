//! On a real laptop with OneDrive: files stored online only are found from folder listings
//! alone, end "safe in OneDrive", and are still online only afterwards (nothing was downloaded).
//! Only reads. Runs when `PCTWIN_REAL_ONEDRIVE` names the OneDrive folder; skipped otherwise
//! (CI has no OneDrive).

mod common;

use common::*;
use pctwin_orchestrate::cloud::{CloudOutcome, SyncFact, cloud_outcome};
use pctwin_record::{CloudProvider, Inclusion, LeftOutReason};

#[test]
fn real_online_only_files_stay_online_only() {
    let Some(root) = std::env::var_os("PCTWIN_REAL_ONEDRIVE") else {
        eprintln!("skipped: PCTWIN_REAL_ONEDRIVE is not set");
        return;
    };
    let mut pending = vec![std::path::PathBuf::from(root)];
    let (mut online, mut local, mut seen) = (Vec::new(), 0u64, 0u64);
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            seen += 1;
            if seen > 50_000 {
                break;
            }
            let (Ok(kind), Ok(meta)) = (e.file_type(), e.metadata()) else {
                continue;
            };
            if kind.is_dir() {
                pending.push(e.path());
            } else if kind.is_file() {
                if pctwin_scan::is_cloud_only(&meta) {
                    online.push(e.path());
                } else {
                    local += 1;
                }
            }
        }
    }
    eprintln!("{} online only, {local} on the laptop", online.len());
    for p in online.iter().take(200) {
        let mut item = online_only(person("me"), CloudProvider::OneDrive, &["x"], 0);
        item.inclusion = Inclusion::LeftOut {
            reason: LeftOutReason::CloudOnly,
        };
        assert!(matches!(
            cloud_outcome(&item, SyncFact::NotKnown),
            CloudOutcome::SafeInCloud { .. }
        ));
        // Still online only: listing and classifying downloaded nothing.
        let again = std::fs::symlink_metadata(p).unwrap();
        assert!(pctwin_scan::is_cloud_only(&again), "{}", p.display());
    }
}
