//! A whole move of a synthetic laptop through the real engine, both sides in one process: every
//! file ends with an outcome on both sides, each one that landed matches its original byte for
//! byte, and the old laptop's folder is unchanged.

use pctwin_lab::{Spec, build, snapshot};
use pctwin_labmove::{Folders, check_move, receive, send};

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

    let checked = check_move(
        &before,
        &snapshot(old.path()).unwrap(),
        &snapshot(&new.path().join("home")).unwrap(),
        &sent,
        &received,
    )
    .unwrap_or_else(|problems| panic!("{problems:#?}"));
    assert!(checked.landed >= 60, "{checked:?}");
    // The file held open by another program cannot be read, so it must end failed.
    if made.cases().contains(&pctwin_lab::Case::Locked) {
        assert_eq!(checked.unreadable, 1, "{checked:?}");
    }
    made.release().unwrap();
}
