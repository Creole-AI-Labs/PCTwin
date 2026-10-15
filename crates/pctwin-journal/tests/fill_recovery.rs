//! Recovery of a file filled in under its real name (a drive with no safe rename, such as exFAT
//! on a Mac; security design notes, "Drives with no safe rename" and "Proving a file is PCTwin's,
//! per drive", decided 15 October 2026). A crash can stop the fill between any two steps; after
//! it, the file at the real name is finished or removed only when it is proven PCTwin's, and
//! otherwise left as it is and reported in plain words. Checked here on a pretend disk at every
//! step boundary.

use std::collections::{HashMap, HashSet};

use pctwin_journal::recovery::{self, Action, Look, Seen};
use pctwin_journal::{
    Actor, Born, Entry, FileId, Fill, FillTag, Permission, PlannedWrite, Proof, State,
};
use pctwin_record::{ItemId, LaptopId};

const FP: [u8; 32] = [5; 32];
const SIZE: u64 = 100;
const TEMP: &str = "d/.pctwin-j-1.part";
const NAME: &str = "d/f1.txt";
/// The creation time PCTwin gave the file it made under the real name.
const TOKEN: Born = Born {
    secs: 1_000_000_000,
    nanos: 70_000_000,
};
/// The checked temporary file.
const SOURCE: FileId = id(1);
/// The file PCTwin made under the real name.
const MADE: FileId = id(2);

const fn id(n: u64) -> FileId {
    FileId {
        volume: 1,
        index: std::num::NonZeroU64::new(n).unwrap(),
        born: None,
    }
}

/// One stored file on the pretend disk.
#[derive(Clone)]
struct Stored {
    len: u64,
    /// Its bytes are the whole file with fingerprint FP.
    whole: bool,
    /// Its bytes are the first `len` bytes of the temporary file.
    start_of_temp: bool,
    id: FileId,
    born: Option<Born>,
}

#[derive(Default)]
struct Disk {
    files: HashMap<String, Stored>,
    cannot: HashSet<String>,
    unreadable: HashSet<String>,
}

impl Disk {
    fn put(mut self, path: &str, f: Stored) -> Self {
        self.files.insert(path.into(), f);
        self
    }
    /// The checked temporary file, whole and as recorded (unless changed below).
    fn temp(self) -> Self {
        self.put(
            TEMP,
            Stored {
                len: SIZE,
                whole: true,
                start_of_temp: true,
                id: SOURCE,
                born: None,
            },
        )
    }
    /// At the real name: `len` bytes, the start of the temporary file (whole if `len` is all of
    /// it), with identity `id` and creation time `born`.
    fn at_name(self, len: u64, id: FileId, born: Option<Born>) -> Self {
        self.put(
            NAME,
            Stored {
                len,
                whole: len == SIZE,
                start_of_temp: true,
                id,
                born,
            },
        )
    }
    /// Another program's file at the real name: other bytes, another identity and time.
    fn theirs(self, len: u64) -> Self {
        self.put(
            NAME,
            Stored {
                len,
                whole: false,
                start_of_temp: false,
                id: id(99),
                born: Some(Born {
                    secs: 1_700_000_000,
                    nanos: 0,
                }),
            },
        )
    }
    fn changed(mut self, path: &str) -> Self {
        let f = self.files.get_mut(path).unwrap();
        f.whole = false;
        f.start_of_temp = false;
        self
    }
    fn cannot(mut self, path: &str) -> Self {
        self.cannot.insert(path.into());
        self
    }
    fn unreadable(mut self, path: &str) -> Self {
        self.unreadable.insert(path.into());
        self
    }
}

impl Look for Disk {
    fn look(&self, _: &Entry, stored: &str) -> Seen {
        if self.cannot.contains(stored) {
            return Seen::CannotLook("the drive is not plugged in".into());
        }
        self.files
            .get(stored)
            .map_or(Seen::Missing, |f| Seen::File { len: f.len })
    }
    fn matches(&self, _: &Entry, stored: &str, fp: &[u8; 32]) -> Result<bool, String> {
        if self.unreadable.contains(stored) {
            return Err("could not read it".into());
        }
        Ok(fp == &FP && self.files.get(stored).is_some_and(|f| f.whole))
    }
    fn identity(&self, _: &Entry, stored: &str) -> Result<Option<FileId>, String> {
        Ok(self.files.get(stored).map(|f| f.id))
    }
    fn temp_path(&self, _: &Entry, _: Option<&str>) -> Option<String> {
        Some(TEMP.into())
    }
    fn born(&self, _: &Entry, stored: &str) -> Result<Option<Born>, String> {
        if self.unreadable.contains(stored) {
            return Err("could not read it".into());
        }
        Ok(self.files.get(stored).and_then(|f| f.born))
    }
    fn same_start(&self, _: &Entry, stored: &str, temp: &str, len: u64) -> Result<bool, String> {
        if self.unreadable.contains(stored) || self.unreadable.contains(temp) {
            return Err("could not read it".into());
        }
        let (Some(f), Some(t)) = (self.files.get(stored), self.files.get(temp)) else {
            return Ok(false);
        };
        Ok(f.len == len && f.start_of_temp && t.whole && len <= t.len)
    }
}

fn entry(state: State) -> Entry {
    Entry {
        id: 1,
        write: PlannedWrite {
            download_mark: None,
            item: ItemId::from_hex(&format!("01{}", "0".repeat(30))).unwrap(),
            source_laptop: LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap(),
            destination: "usb".into(),
            path: "d/f1.txt".into(),
            size: SIZE,
            actor: Actor {
                acting_account: "501".into(),
                for_account: "501".into(),
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

fn fill(proof: Proof, made: Option<FileId>, born: Option<Born>) -> Fill {
    Fill {
        final_path: NAME.into(),
        made,
        tag: FillTag { born, hidden: true },
        proof,
    }
}

fn filling(f: Fill) -> State {
    State::Filling {
        temp: TEMP.into(),
        fingerprint: FP,
        file: Some(SOURCE),
        fill: f,
    }
}

fn filled(f: Fill) -> State {
    State::Filled {
        temp: TEMP.into(),
        fingerprint: FP,
        file: Some(SOURCE),
        fill: f,
    }
}

fn decide(state: State, disk: &Disk) -> Action {
    recovery::decide(&entry(state), disk)
}

/// The mac's way: identities change, so the creation-time tag proves the file.
fn token(made: Option<FileId>) -> Fill {
    fill(Proof::Token, made, Some(TOKEN))
}

fn noting(a: &Action) -> bool {
    matches!(a, Action::NameNoting(why) if !why.is_empty())
}

/// Each step boundary of a fill, in order, on a drive proven by its tag. The file at the real
/// name grows through the steps; whatever the step, recovery either goes back to naming the
/// checked temporary file (removing PCTwin's own partial first) or finishes the whole file.
#[test]
fn a_crash_at_every_step_of_a_fill_ends_honestly() {
    let t = || Disk::default().temp();
    // 1. The step is recorded; nothing made yet.
    assert_eq!(decide(filling(token(None)), &t()), Action::Name);
    // 2. Made, before its tag: an empty file nothing proves is PCTwin's. Left, and said.
    assert!(noting(&decide(
        filling(token(None)),
        &t().at_name(0, MADE, None)
    )));
    // 3. Tagged, its identity not yet recorded: the tag proves it; empty, so removed.
    assert_eq!(
        decide(filling(token(None)), &t().at_name(0, MADE, Some(TOKEN))),
        Action::RemoveFillThenName
    );
    // 4. Its identity recorded; 5. part copied: still PCTwin's partial, removed.
    for len in [0, 1, 40, SIZE - 1] {
        assert_eq!(
            decide(
                filling(token(Some(MADE))),
                &t().at_name(len, MADE, Some(TOKEN))
            ),
            Action::RemoveFillThenName,
            "{len}"
        );
    }
    // 6. All copied, before it was made durable and read back: checked again whole, finished.
    assert_eq!(
        decide(
            filling(token(Some(MADE))),
            &t().at_name(SIZE, MADE, Some(TOKEN))
        ),
        Action::FinishFill
    );
    // 7. Recorded filled; 8. its time set; 9. shown (no longer hidden): finished.
    assert_eq!(
        decide(
            filled(token(Some(MADE))),
            &t().at_name(SIZE, MADE, Some(TOKEN))
        ),
        Action::FinishFill
    );
}

/// On a drive that keeps identities, the recorded identity proves the file too.
#[test]
fn a_kept_identity_proves_the_file_made() {
    let f = fill(Proof::StableId, Some(MADE), None);
    let t = || Disk::default().temp();
    assert_eq!(
        decide(filling(f.clone()), &t().at_name(40, MADE, None)),
        Action::RemoveFillThenName
    );
    assert_eq!(
        decide(filled(f.clone()), &t().at_name(SIZE, MADE, None)),
        Action::FinishFill
    );
    // Another file with the same bytes, but not the one made: someone else's.
    assert!(noting(&decide(filling(f), &t().at_name(SIZE, id(7), None))));
}

/// Where nothing lasts past a restart (an identity good only while the drive stays mounted, or
/// nothing at all), the file at the real name is never taken as PCTwin's.
#[test]
fn without_a_lasting_proof_the_file_is_left_and_reported() {
    let t = || Disk::default().temp();
    for proof in [Proof::SameMountId, Proof::None] {
        let f = fill(proof, Some(MADE), None);
        for len in [0, 40, SIZE] {
            let a = decide(filling(f.clone()), &t().at_name(len, MADE, None));
            assert!(noting(&a), "{proof:?} {len}: {a:?}");
        }
        let a = decide(filled(f.clone()), &t().at_name(SIZE, MADE, None));
        assert!(noting(&a), "{proof:?}: {a:?}");
    }
    // A tag recorded but never set by the drive (no creation time kept): no proof either.
    let f = fill(Proof::Token, Some(MADE), None);
    assert!(noting(&decide(filling(f), &t().at_name(40, MADE, None))));
}

/// Another program's file at the real name (it renamed its own over PCTwin's, say) is never
/// touched: the checked file is named again elsewhere, and the person told what was left.
#[test]
fn another_program_s_file_at_the_name_is_never_touched() {
    let t = || Disk::default().temp();
    for state in [filling(token(Some(MADE))), filled(token(Some(MADE)))] {
        for len in [0, 40, SIZE] {
            let a = decide(state.clone(), &t().theirs(len));
            assert!(noting(&a), "{len}: {a:?}");
        }
    }
}

/// PCTwin's own file, changed by someone after it was made: never removed or adopted.
#[test]
fn a_file_of_pctwin_s_changed_since_is_kept() {
    let t = || Disk::default().temp();
    // Part way: its bytes are not the start of the checked file.
    let a = decide(
        filling(token(Some(MADE))),
        &t().at_name(40, MADE, Some(TOKEN)).changed(NAME),
    );
    assert!(noting(&a), "{a:?}");
    // Filled, then changed: kept as it is, and the write is not counted as copied.
    let a = decide(
        filled(token(Some(MADE))),
        &t().at_name(SIZE, MADE, Some(TOKEN)).changed(NAME),
    );
    assert!(matches!(a, Action::Fail(_)), "{a:?}");
    // Filled, then cut short: kept too.
    let a = decide(
        filled(token(Some(MADE))),
        &t().at_name(40, MADE, Some(TOKEN)),
    );
    assert!(matches!(a, Action::Fail(_)), "{a:?}");
}

/// When the checked temporary file is gone or changed, nothing can be named again: PCTwin's own
/// partial is still removed (proven by tag, and by its bytes while the source can be read) or,
/// when its bytes cannot be checked, left and reported.
#[test]
fn without_the_checked_file_a_proven_partial_is_removed_or_left_never_named() {
    // The temporary file changed: the partial's bytes are not its start. Kept, and failed.
    let changed = Disk::default()
        .temp()
        .changed(TEMP)
        .at_name(40, MADE, Some(TOKEN));
    let a = decide(filling(token(Some(MADE))), &changed);
    assert!(matches!(a, Action::Fail(_)), "{a:?}");
    // Gone: the partial's bytes cannot be checked, so it is not removed.
    let gone = Disk::default().at_name(40, MADE, Some(TOKEN));
    let a = decide(filling(token(Some(MADE))), &gone);
    assert!(
        matches!(&a, Action::Fail(why) if why.contains("safe to delete")),
        "{a:?}"
    );
    // Nothing at either name.
    assert!(matches!(
        decide(filling(token(Some(MADE))), &Disk::default()),
        Action::Fail(_)
    ));
    // Filled and whole, but the temporary file gone: still finished (it no longer needs it).
    let whole = Disk::default().at_name(SIZE, MADE, Some(TOKEN));
    assert_eq!(
        decide(filled(token(Some(MADE))), &whole),
        Action::FinishFill
    );
}

/// A lost name, after it was filled (removed by another program, say), is given again from the
/// checked temporary file.
#[test]
fn a_filled_file_gone_from_its_name_is_named_again() {
    assert_eq!(
        decide(filled(token(Some(MADE))), &Disk::default().temp()),
        Action::Name
    );
}

/// A drive that cannot be looked at now is left exactly as it is, for next time.
#[test]
fn a_place_that_cannot_be_looked_at_is_left() {
    let t = || Disk::default().temp();
    for disk in [
        t().cannot(NAME),
        t().at_name(40, MADE, Some(TOKEN)).cannot(TEMP),
        t().at_name(40, MADE, Some(TOKEN)).unreadable(NAME),
    ] {
        let a = decide(filling(token(Some(MADE))), &disk);
        assert!(matches!(a, Action::Leave(_)), "{a:?}");
    }
}

/// On a drive proven by its tag, the checked temporary file is proven by its journal-given name
/// and whole fingerprint, since its identity may have changed (FAT after a remount).
#[test]
fn the_checked_file_is_proven_by_its_name_and_bytes_where_identities_change() {
    let moved = Disk::default()
        .put(
            TEMP,
            Stored {
                len: SIZE,
                whole: true,
                start_of_temp: true,
                id: id(55),
                born: None,
            },
        )
        .at_name(40, id(56), Some(TOKEN));
    assert_eq!(
        decide(filling(token(Some(MADE))), &moved),
        Action::RemoveFillThenName
    );
    // On a drive that keeps identities, the same change means it is not the file checked.
    let stable = fill(Proof::StableId, Some(MADE), None);
    let a = decide(filling(stable), &moved.at_name(40, MADE, None));
    assert!(matches!(a, Action::Fail(_)), "{a:?}");
}

/// The temporary file of a fill is the journal's own, cleaned up like any other.
#[test]
fn a_fill_s_temporary_file_is_known_for_clean_up() {
    for state in [filling(token(None)), filled(token(Some(MADE)))] {
        assert_eq!(
            recovery::temp_of(&entry(state), &Disk::default()),
            Some(TEMP.into())
        );
    }
}
