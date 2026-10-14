//! A whole move of a synthetic laptop through the real engine, both sides in one process: every
//! file ends with an outcome on both sides, each one that landed matches its original byte for
//! byte, and the old laptop's folder is unchanged.

use std::collections::BTreeMap;

use pctwin_lab::{Entry, Spec, build, compare, snapshot};
use pctwin_labmove::{Folders, receive, send};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_synthetic_laptop_moves_and_the_source_stays_unchanged() {
    let old = tempfile::tempdir().unwrap();
    let new = tempfile::tempdir().unwrap();
    let handoff = tempfile::tempdir().unwrap();
    let mut made = build(
        old.path(),
        &Spec {
            seed: 11,
            ordinary_files: 60,
            big_file: 3 * 1024 * 1024,
        },
    )
    .unwrap();
    let before = snapshot(old.path()).unwrap();
    let journal = new.path().join("journal.redb");

    let sending = send(
        Folders::Lab {
            root: old.path().to_path_buf(),
            account: "old-person".into(),
        },
        handoff.path(),
        "127.0.0.1:0",
    );
    let receiving = receive(
        Folders::Lab {
            root: new.path().join("home"),
            account: "new-person".into(),
        },
        handoff.path(),
        &journal,
    );
    let (sent, received) = tokio::join!(sending, receiving);
    let (sent, received) = (sent.unwrap(), received.unwrap());

    // The old laptop is read-only.
    assert_eq!(compare(&before, &snapshot(old.path()).unwrap()), Vec::<String>::new());

    // Both sides account for the same files, each with an outcome.
    assert!(!sent.lines.is_empty());
    assert_eq!(sent.lines.len(), received.lines.len());
    let got: BTreeMap<_, _> = received.lines.iter().map(|l| (l.item.clone(), l)).collect();
    for line in &sent.lines {
        assert_ne!(line.outcome, "no outcome", "{line:?}");
        assert!(got.contains_key(&line.item), "{line:?} missing on the new laptop");
    }

    // Every file that landed matches its original.
    let after = snapshot(&new.path().join("home")).unwrap();
    let mut landed = 0;
    for line in &received.lines {
        let Some(final_path) = line.outcome.strip_prefix("landed: ") else {
            continue;
        };
        landed += 1;
        let Some(original) = before.get(&line.path) else {
            // Only a name that is not valid text is shown differently from its key.
            assert!(line.path.contains('\u{FFFD}'), "{line:?} has no original");
            continue;
        };
        let original = Some(original);
        let copy = after.get(final_path);
        match (original, copy) {
            (
                Some(Entry::File { size: a, blake3: ha, .. }),
                Some(Entry::File { size: b, blake3: hb, .. }),
            ) => {
                assert_eq!((a, ha), (b, hb), "{line:?}");
            }
            other => panic!("{line:?}: {other:?}"),
        }
    }
    assert!(landed >= 60, "only {landed} files landed: {:#?}", received.lines);
    made.release().unwrap();
}
