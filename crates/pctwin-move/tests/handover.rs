//! Handing another person's files over from the holding area into their own folders, through
//! the move's own write path: every file lands, read back; a file already at a name is never
//! replaced; anything that is not a plain file is left and listed, never followed.
// Tests make files to test on; the no-file-changes fence is for the shipped code.
#![allow(clippy::disallowed_methods)]

use std::collections::BTreeMap;

use pctwin_journal::Journal;
use pctwin_move::handover::hand_over;
use pctwin_transfer::ReceiveOutcome;

#[tokio::test]
async fn held_files_land_in_the_person_s_folders_and_nothing_is_replaced() {
    let holding = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let jdir = tempfile::tempdir().unwrap();
    let docs_held = holding.path().join("documents");
    std::fs::create_dir_all(docs_held.join("Trips")).unwrap();
    std::fs::write(docs_held.join("letter.txt"), b"Dear Bea").unwrap();
    std::fs::write(docs_held.join("Trips/beach.jpg"), vec![7u8; 200_000]).unwrap();
    std::fs::create_dir_all(holding.path().join("pictures")).unwrap();
    std::fs::write(holding.path().join("pictures/cat.png"), b"meow").unwrap();
    // Already in Bea's Documents: never replaced.
    let docs = home.path().join("Documents");
    let pics = home.path().join("Pictures");
    // Documents exists already; Pictures does not, and is made.
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::write(docs.join("letter.txt"), b"Bea's own, newer").unwrap();
    let places = BTreeMap::from([
        ("documents".to_string(), "Documents".to_string()),
        ("pictures".to_string(), "Pictures".to_string()),
    ]);
    let journal = Journal::open(&jdir.path().join("journal.redb")).unwrap();
    let done = hand_over(holding.path(), home.path(), &places, &journal, "1001")
        .await
        .unwrap();
    assert_eq!(done.files.len(), 3);
    for f in &done.files {
        assert!(
            matches!(f.outcome, Some(ReceiveOutcome::Finished(_))),
            "{:?}: {:?}",
            f.held,
            f.outcome
        );
    }
    assert_eq!(
        std::fs::read(docs.join("letter.txt")).unwrap(),
        b"Bea's own, newer"
    );
    let landed_letter = done
        .files
        .iter()
        .find(|f| f.held.ends_with("letter.txt"))
        .and_then(|f| match &f.outcome {
            Some(ReceiveOutcome::Finished(done)) => Some(done.final_path.clone()),
            _ => None,
        })
        .unwrap();
    // Inside the home: Documents/<another name>.
    assert_ne!(landed_letter, "Documents/letter.txt");
    assert!(landed_letter.starts_with("Documents/"), "{landed_letter}");
    assert_eq!(
        std::fs::read(home.path().join(&landed_letter)).unwrap(),
        b"Dear Bea"
    );
    assert_eq!(
        std::fs::read(docs.join("Trips/beach.jpg")).unwrap(),
        vec![7u8; 200_000]
    );
    assert_eq!(std::fs::read(pics.join("cat.png")).unwrap(), b"meow");
    assert!(done.not_plain.is_empty());
}

/// A person whose held folders hold nothing is handed nothing, without a fault (found on Linux CI,
/// where a held folder with only a link failed the whole hand-over).
#[tokio::test]
async fn an_empty_holding_area_hands_over_nothing() {
    let holding = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let jdir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(holding.path().join("documents")).unwrap();
    let places = BTreeMap::from([("documents".to_string(), "Documents".to_string())]);
    let journal = Journal::open(&jdir.path().join("journal.redb")).unwrap();
    let done = hand_over(holding.path(), home.path(), &places, &journal, "1001")
        .await
        .unwrap();
    assert!(done.files.is_empty());
    assert!(done.not_plain.is_empty());
}

/// A link in the holding area is never followed: it is listed and left as it is.
#[cfg(unix)]
#[tokio::test]
async fn a_link_in_the_holding_area_is_never_followed() {
    let holding = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let jdir = tempfile::tempdir().unwrap();
    let secret = tempfile::tempdir().unwrap();
    std::fs::write(secret.path().join("shadow"), b"not yours").unwrap();
    std::fs::create_dir_all(holding.path().join("documents")).unwrap();
    std::os::unix::fs::symlink(
        secret.path().join("shadow"),
        holding.path().join("documents/shadow"),
    )
    .unwrap();
    let docs = home.path().join("Documents");
    std::fs::create_dir_all(&docs).unwrap();
    let places = BTreeMap::from([("documents".to_string(), "Documents".to_string())]);
    let journal = Journal::open(&jdir.path().join("journal.redb")).unwrap();
    let done = hand_over(holding.path(), home.path(), &places, &journal, "1001")
        .await
        .unwrap();
    assert!(done.files.is_empty());
    assert_eq!(done.not_plain.len(), 1);
    assert!(!docs.join("shadow").exists());
}
