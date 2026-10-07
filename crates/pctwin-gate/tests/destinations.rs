//! The table of approved destinations (Security Design Part B): the new laptop lists the only
//! places it will write to. The old laptop can only name an entry; an unknown entry is refused like
//! `..`, its IDs are never used as paths, and another person's account opens only through the admin
//! helper.

use std::io::Write;

use pctwin_gate::{Approved, Destinations, Dir, GateError, IncomingPath};

fn write(table: &Destinations, id: &str, p: &str, bytes: &[u8]) -> Result<String, GateError> {
    let dest = table.get(id)?;
    let mut file = dest.create_file(&IncomingPath::parse(p).unwrap(), bytes.len() as u64)?;
    file.write_all(bytes).map_err(GateError::Io)?;
    Ok(file.finish()?.final_path)
}

/// Every file and folder under `root`, as sorted relative paths.
fn everything_under(root: &std::path::Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            found.push(
                path.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
            if path.is_dir() {
                stack.push(path);
            }
        }
    }
    found.sort();
    found
}

#[test]
fn each_approved_place_receives_only_its_own_files() {
    let mine = tempfile::tempdir().unwrap();
    let shared = tempfile::tempdir().unwrap();
    let drive = tempfile::tempdir().unwrap();
    let offload = tempfile::tempdir().unwrap();
    let mut table = Destinations::new();
    table
        .approve("me", Approved::MyFolders, mine.path())
        .unwrap();
    table
        .approve("shared", Approved::SharedFolder, shared.path())
        .unwrap();
    table
        .approve("drive-d", Approved::ChosenDrive, drive.path())
        .unwrap();
    table
        .approve("offload", Approved::OffloadDrive, offload.path())
        .unwrap();

    assert_eq!(
        write(&table, "me", "Documents/a.txt", b"a").unwrap(),
        "Documents/a.txt"
    );
    assert_eq!(
        write(&table, "shared", "Public/b.txt", b"b").unwrap(),
        "Public/b.txt"
    );
    assert_eq!(
        write(&table, "drive-d", "Games/c.txt", b"c").unwrap(),
        "Games/c.txt"
    );
    assert_eq!(
        write(&table, "offload", "Old/d.txt", b"d").unwrap(),
        "Old/d.txt"
    );

    assert_eq!(
        everything_under(mine.path()),
        ["Documents", "Documents/a.txt"]
    );
    assert_eq!(everything_under(shared.path()), ["Public", "Public/b.txt"]);
    assert_eq!(everything_under(drive.path()), ["Games", "Games/c.txt"]);
    assert_eq!(everything_under(offload.path()), ["Old", "Old/d.txt"]);
}

#[test]
fn the_same_name_in_two_places_is_not_a_clash() {
    let mine = tempfile::tempdir().unwrap();
    let shared = tempfile::tempdir().unwrap();
    let mut table = Destinations::new();
    table
        .approve("me", Approved::MyFolders, mine.path())
        .unwrap();
    table
        .approve("shared", Approved::SharedFolder, shared.path())
        .unwrap();
    assert_eq!(write(&table, "me", "notes.txt", b"1").unwrap(), "notes.txt");
    assert_eq!(
        write(&table, "shared", "notes.txt", b"2").unwrap(),
        "notes.txt"
    );
}

#[test]
fn an_unknown_destination_is_refused_and_nothing_is_written() {
    let parent = tempfile::tempdir().unwrap();
    let mine = parent.path().join("mine");
    std::fs::create_dir(&mine).unwrap();
    let mut table = Destinations::new();
    table.approve("me", Approved::MyFolders, &mine).unwrap();

    let parent_path = parent.path().to_string_lossy().into_owned();
    let mine_path = mine.to_string_lossy().into_owned();
    // IDs that look like paths are only ever looked up in the table, never opened.
    for id in [
        "",
        "you",
        "ME",
        "me ",
        "..",
        "../mine",
        "me/..",
        "/",
        "C:\\",
        "C:\\Windows",
        "\\\\server\\share",
        parent_path.as_str(),
        mine_path.as_str(),
    ] {
        assert!(
            matches!(table.get(id), Err(GateError::UnknownDestination)),
            "{id:?} was accepted"
        );
        assert!(matches!(
            write(&table, id, "x.txt", b"x"),
            Err(GateError::UnknownDestination)
        ));
    }
    assert_eq!(everything_under(parent.path()), ["mine"]);
}

#[test]
fn an_empty_table_refuses_everything() {
    let table = Destinations::new();
    assert!(matches!(
        table.get("me"),
        Err(GateError::UnknownDestination)
    ));
    assert_eq!(table.ids().count(), 0);
}

#[test]
fn a_destination_is_approved_only_once() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let mut table = Destinations::new();
    table.approve("me", Approved::MyFolders, a.path()).unwrap();
    assert!(matches!(
        table.approve("me", Approved::SharedFolder, b.path()),
        Err(GateError::DuplicateDestination)
    ));
    // The first approval still stands.
    assert_eq!(table.place("me").unwrap(), &Approved::MyFolders);
    write(&table, "me", "x.txt", b"x").unwrap();
    assert!(a.path().join("x.txt").exists());
    assert!(!b.path().join("x.txt").exists());
}

#[test]
fn destination_ids_are_short_plain_labels() {
    let dir = tempfile::tempdir().unwrap();
    let mut table = Destinations::new();
    let long = "a".repeat(65);
    for bad in [
        "",
        long.as_str(),
        "has space",
        "a/b",
        "a\\b",
        "..",
        "é",
        "a\u{0}",
    ] {
        assert!(
            matches!(
                table.approve(bad, Approved::MyFolders, dir.path()),
                Err(GateError::BadDestinationId)
            ),
            "{bad:?} was accepted as an ID"
        );
    }
    let longest = "a".repeat(64);
    for good in [
        "me",
        "shared",
        "drive-d",
        "person_2",
        "offload.1",
        longest.as_str(),
    ] {
        table
            .approve(good, Approved::MyFolders, dir.path())
            .unwrap();
    }
}

#[test]
fn another_persons_account_needs_the_admin_helper() {
    let theirs = tempfile::tempdir().unwrap();
    let mut table = Destinations::new();
    let place = Approved::AnotherAccount {
        account_id: "S-1-5-21-1-2-3-1002".into(),
    };
    assert!(matches!(
        table.approve("tunde", place.clone(), theirs.path()),
        Err(GateError::NeedsAdminHelper)
    ));
    assert!(matches!(
        table.get("tunde"),
        Err(GateError::UnknownDestination)
    ));

    // The admin helper opens the folder and hands over the handle.
    let handle = Dir::open_ambient_dir(theirs.path(), cap_std::ambient_authority()).unwrap();
    table
        .approve_through_helper("tunde", "S-1-5-21-1-2-3-1002", handle)
        .unwrap();
    assert_eq!(table.place("tunde").unwrap(), &place);
    assert_eq!(
        write(&table, "tunde", "Pictures/p.jpg", b"p").unwrap(),
        "Pictures/p.jpg"
    );
    assert!(theirs.path().join("Pictures/p.jpg").exists());
}

#[test]
fn the_helper_path_is_only_for_another_account() {
    let dir = tempfile::tempdir().unwrap();
    let mut table = Destinations::new();
    let handle = Dir::open_ambient_dir(dir.path(), cap_std::ambient_authority()).unwrap();
    assert!(matches!(
        table.approve_through_helper("x", "", handle),
        Err(GateError::BadDestinationId)
    ));
    let handle = Dir::open_ambient_dir(dir.path(), cap_std::ambient_authority()).unwrap();
    assert!(matches!(
        table.approve_through_helper("y", "S-1-5\n21", handle),
        Err(GateError::BadDestinationId)
    ));
    let handle = Dir::open_ambient_dir(dir.path(), cap_std::ambient_authority()).unwrap();
    let long = "S".repeat(257);
    assert!(matches!(
        table.approve_through_helper("z", &long, handle),
        Err(GateError::BadDestinationId)
    ));
    assert_eq!(table.ids().count(), 0);
}

#[test]
fn the_table_lists_its_ids_for_the_old_laptop() {
    let dir = tempfile::tempdir().unwrap();
    let mut table = Destinations::new();
    table
        .approve("shared", Approved::SharedFolder, dir.path())
        .unwrap();
    table
        .approve("me", Approved::MyFolders, dir.path())
        .unwrap();
    let mut ids: Vec<&str> = table.ids().collect();
    ids.sort_unstable();
    assert_eq!(ids, ["me", "shared"]);
}

#[test]
fn a_missing_folder_is_not_approved() {
    let parent = tempfile::tempdir().unwrap();
    let mut table = Destinations::new();
    assert!(matches!(
        table.approve("me", Approved::MyFolders, &parent.path().join("missing")),
        Err(GateError::Io(_))
    ));
    assert!(matches!(
        table.get("me"),
        Err(GateError::UnknownDestination)
    ));
    assert_eq!(everything_under(parent.path()), Vec::<String>::new());
}
