//! Recovery decides what each unfinished write needs after a restart, looking at the disk only
//! through `Look`: here a pretend disk, so every case is checked exactly.

use std::collections::{HashMap, HashSet};

use pctwin_journal::recovery::{self, Action, Look, Seen, Tidy};
use pctwin_journal::{Actor, Entry, FileId, Landed, Permission, PlannedWrite, State};
use pctwin_record::{ItemId, LaptopId};

const FP: [u8; 32] = [5; 32];
const SIZE: u64 = 100;

#[derive(Default)]
struct Disk {
    files: HashMap<String, Seen>,
    /// Stored paths whose contents have the fingerprint FP.
    good: HashSet<String>,
    /// Which file each stored path is (every file its own, unless made the sealed one).
    ids: HashMap<String, FileId>,
    unreadable: HashSet<String>,
}

/// The file the write sealed (recorded in the journal).
const OURS: FileId = FileId {
    volume: 1,
    index: 1,
};

impl Disk {
    fn file(mut self, path: &str, len: u64, good: bool) -> Self {
        self.files.insert(path.into(), Seen::File { len });
        let index = 100 + self.ids.len() as u64;
        self.ids.insert(path.into(), FileId { volume: 1, index });
        if good {
            self.good.insert(path.into());
        }
        self
    }
    fn cannot(mut self, path: &str) -> Self {
        self.files
            .insert(path.into(), Seen::CannotLook("drive not plugged in".into()));
        self
    }
    /// This path is the very file the write sealed.
    fn ours(mut self, path: &str) -> Self {
        self.ids.insert(path.into(), OURS);
        self
    }
    fn unreadable(mut self, path: &str) -> Self {
        self.unreadable.insert(path.into());
        self
    }
}

impl Look for Disk {
    fn look(&self, _: &Entry, stored: &str) -> Seen {
        self.files.get(stored).cloned().unwrap_or(Seen::Missing)
    }
    fn matches(&self, _: &Entry, stored: &str, fp: &[u8; 32]) -> Result<bool, String> {
        if self.unreadable.contains(stored) {
            return Err("could not read it".into());
        }
        Ok(fp == &FP && self.good.contains(stored))
    }
    fn identity(&self, _: &Entry, stored: &str) -> Result<Option<FileId>, String> {
        Ok(self
            .files
            .get(stored)
            .and_then(|_| self.ids.get(stored).copied()))
    }
    fn temp_path(&self, entry: &Entry, folder: Option<&str>) -> Option<String> {
        let folder = folder.map_or_else(|| "d".to_string(), str::to_string);
        Some(format!("{folder}/.pctwin-j-{}.part", entry.id))
    }
}

fn entry(id: u64, state: State) -> Entry {
    Entry {
        id,
        write: PlannedWrite {
            item: ItemId::from_hex(&format!("{id:02x}{}", "0".repeat(30))).unwrap(),
            source_laptop: LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap(),
            destination: "me".into(),
            path: format!("d/f{id}.txt"),
            size: SIZE,
            actor: Actor {
                acting_account: "1001".into(),
                for_account: "1001".into(),
                permission: Permission::OwnFolders,
            },
            block_size: 128 * 1024,
            source_modified_ns: None,
            place: None,
            source_file: None,
            partial_keep: Default::default(),
        },
        state,
    }
}

const TEMP: &str = "d/.pctwin-j-1.part";
const NAME: &str = "d/f1.txt";

fn staged() -> State {
    State::Staged { temp: TEMP.into() }
}
fn verified() -> State {
    State::Verified {
        temp: TEMP.into(),
        fingerprint: FP,
        file: Some(OURS),
    }
}
fn applied() -> State {
    State::Applied {
        temp: TEMP.into(),
        final_path: NAME.into(),
        fingerprint: FP,
        file: Some(OURS),
    }
}

fn decide(state: State, disk: &Disk) -> Action {
    recovery::decide(&entry(1, state), disk)
}

fn fails(a: &Action) -> bool {
    matches!(a, Action::Fail(_))
}

#[test]
fn a_planned_write_is_failed_whatever_is_on_disk() {
    let disk = Disk::default().file(TEMP, SIZE, true);
    assert!(fails(&decide(State::Planned, &disk)));
}

#[test]
fn a_staged_write_is_kept_to_continue_only_while_its_partial_file_is_there() {
    assert_eq!(
        decide(staged(), &Disk::default().file(TEMP, 40, false)),
        Action::Resume
    );
    // Reserved to full size: still only partly arrived, checked block by block on resume.
    assert_eq!(
        decide(staged(), &Disk::default().file(TEMP, SIZE, false)),
        Action::Resume
    );
    assert!(fails(&decide(staged(), &Disk::default())));
    assert!(fails(&decide(
        staged(),
        &Disk::default().file(TEMP, SIZE + 1, false)
    )));
    assert!(matches!(
        decide(staged(), &Disk::default().cannot(TEMP)),
        Action::Leave(_)
    ));
}

#[test]
fn a_verified_write_is_named_only_once_its_whole_file_is_proven_again() {
    let ok = Disk::default().file(TEMP, SIZE, true).ours(TEMP);
    assert_eq!(decide(verified(), &ok), Action::Name);
    // Changed since it was checked, or the wrong size: never committed.
    assert!(fails(&decide(
        verified(),
        &Disk::default().file(TEMP, SIZE, false).ours(TEMP)
    )));
    assert!(fails(&decide(
        verified(),
        &Disk::default().file(TEMP, SIZE - 1, true).ours(TEMP)
    )));
    // Another file under its temporary name, however alike: not the file it sealed.
    assert!(fails(&decide(
        verified(),
        &Disk::default().file(TEMP, SIZE, true)
    )));
    // Gone: nothing proves which file is its own.
    assert!(fails(&decide(verified(), &Disk::default())));
    // Could not read it now: left for next time rather than failed.
    assert!(matches!(
        decide(
            verified(),
            &Disk::default()
                .file(TEMP, SIZE, true)
                .ours(TEMP)
                .unreadable(TEMP)
        ),
        Action::Leave(_)
    ));
    assert!(matches!(
        decide(verified(), &Disk::default().cannot(TEMP)),
        Action::Leave(_)
    ));
}

#[test]
fn an_applied_write_is_committed_only_when_the_file_at_its_name_is_the_very_file_it_sealed() {
    // It got its name just before the crash: the same file by both names.
    let both = Disk::default()
        .file(NAME, SIZE, true)
        .ours(NAME)
        .file(TEMP, SIZE, true)
        .ours(TEMP);
    assert_eq!(decide(applied(), &both), Action::Commit);
    // Its temporary name was already removed: still the very file it sealed.
    let named = Disk::default().file(NAME, SIZE, true).ours(NAME);
    assert_eq!(decide(applied(), &named), Action::Commit);
    // Its own file, changed afterwards: not committed, and kept as it is.
    let changed = Disk::default().file(NAME, SIZE, false).ours(NAME);
    assert!(fails(&decide(applied(), &changed)));
    let grown = Disk::default()
        .file(NAME, SIZE + 5, false)
        .ours(NAME)
        .file(TEMP, SIZE + 5, false)
        .ours(TEMP);
    assert!(fails(&decide(applied(), &grown)));
}

#[test]
fn a_file_at_its_name_that_is_not_the_one_it_sealed_is_never_taken() {
    // A copy with exactly its contents, and the temporary file gone: a person's copy, never
    // adopted as the move's own.
    let copy = Disk::default().file(NAME, SIZE, true);
    assert!(fails(&decide(applied(), &copy)));
    // An empty file at the name (a person's, or another write's): never replaced; this write
    // gets another name.
    let empty = Disk::default()
        .file(NAME, 0, false)
        .file(TEMP, SIZE, true)
        .ours(TEMP);
    assert_eq!(decide(applied(), &empty), Action::Name);
    // A different file of the same size and contents: passed over the same way.
    let alike = Disk::default()
        .file(NAME, SIZE, true)
        .file(TEMP, SIZE, true)
        .ours(TEMP);
    assert_eq!(decide(applied(), &alike), Action::Name);
    // Its own file changed meanwhile: failed, the other file untouched.
    let bad = Disk::default()
        .file(NAME, 0, false)
        .file(TEMP, SIZE, false)
        .ours(TEMP);
    assert!(fails(&decide(applied(), &bad)));
}

#[test]
fn without_a_recorded_identity_nothing_is_ever_proven_its_own() {
    let no_id = |state: State| match state {
        State::Verified {
            temp, fingerprint, ..
        } => State::Verified {
            temp,
            fingerprint,
            file: None,
        },
        State::Applied {
            temp,
            final_path,
            fingerprint,
            ..
        } => State::Applied {
            temp,
            final_path,
            fingerprint,
            file: None,
        },
        other => other,
    };
    let disk = Disk::default()
        .file(NAME, SIZE, true)
        .ours(NAME)
        .file(TEMP, SIZE, true)
        .ours(TEMP);
    assert!(fails(&decide(no_id(verified()), &disk)));
    assert!(fails(&decide(no_id(applied()), &disk)));
}

#[test]
fn a_lost_name_is_given_again_and_a_lost_file_is_failed() {
    assert_eq!(
        decide(
            applied(),
            &Disk::default().file(TEMP, SIZE, true).ours(TEMP)
        ),
        Action::Name
    );
    assert!(fails(&decide(
        applied(),
        &Disk::default().file(TEMP, SIZE, false).ours(TEMP)
    )));
    assert!(fails(&decide(applied(), &Disk::default())));
    assert!(matches!(
        decide(applied(), &Disk::default().cannot(NAME)),
        Action::Leave(_)
    ));
    assert!(matches!(
        decide(
            applied(),
            &Disk::default()
                .file(NAME, SIZE, true)
                .ours(NAME)
                .unreadable(NAME)
        ),
        Action::Leave(_)
    ));
}

#[test]
fn reasons_are_plain_words() {
    let disk = Disk::default();
    for state in [State::Planned, staged(), verified(), applied()] {
        let Action::Fail(why) = decide(state, &disk) else {
            panic!("not failed")
        };
        assert!(why.starts_with("the copy was interrupted"), "{why}");
    }
}

#[test]
fn finished_writes_are_never_planned_again() {
    let finished = [
        entry(
            1,
            State::Committed {
                final_path: NAME.into(),
                fingerprint: FP,
                landed: Landed {
                    size: SIZE,
                    modified_ns: None,
                    file: None,
                },
            },
        ),
        entry(
            2,
            State::Existing {
                stored_path: NAME.into(),
            },
        ),
        entry(
            3,
            State::Failed {
                why: "x".into(),
                reached: Box::new(State::Planned),
            },
        ),
        entry(4, State::Planned),
    ];
    let plan = recovery::plan(&finished, &Disk::default());
    assert_eq!(plan.len(), 1);
    assert_eq!(plan[0].id, 4);
}

#[test]
fn clean_up_lists_only_the_journal_s_own_temporary_names_and_skips_kept_ones() {
    let entries = [
        entry(1, State::Planned),
        entry(
            2,
            State::Staged {
                temp: "x/.pctwin-j-2.part".into(),
            },
        ),
        entry(
            3,
            State::Committed {
                final_path: "e/f3.txt".into(),
                fingerprint: FP,
                landed: Landed {
                    size: SIZE,
                    modified_ns: None,
                    file: None,
                },
            },
        ),
        entry(
            4,
            State::Failed {
                why: "x".into(),
                reached: Box::new(State::Verified {
                    temp: "y/.pctwin-j-4.part".into(),
                    fingerprint: FP,
                    file: None,
                }),
            },
        ),
        entry(
            5,
            State::Existing {
                stored_path: "d/f5.txt".into(),
            },
        ),
        entry(
            6,
            State::Failed {
                why: "x".into(),
                reached: Box::new(State::Applied {
                    temp: "z/.pctwin-j-6.part".into(),
                    final_path: "z/f6.txt".into(),
                    fingerprint: FP,
                    file: None,
                }),
            },
        ),
    ];
    let keep: HashSet<u64> = [2].into();
    let tidy = recovery::sweep(&entries, &keep, &Disk::default());
    let want = |id: u64, temp: &str| Tidy {
        id,
        destination: "me".into(),
        temp: temp.into(),
    };
    assert_eq!(
        tidy,
        [
            want(1, "d/.pctwin-j-1.part"),
            want(3, "e/.pctwin-j-3.part"),
            want(4, "y/.pctwin-j-4.part"),
            want(5, "d/.pctwin-j-5.part"),
            want(6, "z/.pctwin-j-6.part"),
        ]
    );
}

#[test]
fn clean_up_moves_on_but_never_past_an_entry_still_kept() {
    let none = HashSet::new();
    assert_eq!(recovery::swept_upto(3, 10, &none), 10);
    assert_eq!(recovery::swept_upto(12, 10, &none), 12);
    let kept: HashSet<u64> = [7, 9].into();
    assert_eq!(recovery::swept_upto(3, 10, &kept), 6);
    let first: HashSet<u64> = [1].into();
    assert_eq!(recovery::swept_upto(0, 10, &first), 0);
}
