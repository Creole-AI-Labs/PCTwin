//! Regression tests for the fresh-context security review of 2.2 and 2.3 (26 October 2026):
//! each began as the reviewer's proof of a fault, and now proves it is gone.

mod common;
#[allow(unused_imports)]
use common::{keys, rebuild};
use redb::ReadableTable;

use common::{Laptop, SCOPE, copy, install, journal, place, plan_of, recipe};
use pctwin_executor::{Adapter, Undone, undo_effects};
use pctwin_journal::{EffectUndoOutcome, Journal, Presence, SettingsFile};
use pctwin_recipe::{Catalog, Check, Platform, SettingValue, SettingsPlace, Step};
use pctwin_types::Reason;

fn moved(j: &Journal, laptop: &mut Laptop, recipes: Vec<pctwin_recipe::Recipe>) {
    let plan = plan_of(recipes);
    let run = plan.run(laptop, &rebuild(j), SCOPE, &common::actor());
    assert_eq!(run.stopped, None);
    laptop.done.clear();
}

fn tamper(path: &std::path::Path, edit: impl Fn(String) -> String) {
    let db = redb::Database::open(path).unwrap();
    let tx = db.begin_write().unwrap();
    {
        let mut t = tx
            .open_table(redb::TableDefinition::<u64, &[u8]>::new("effects"))
            .unwrap();
        let ids: Vec<u64> = { t.iter().unwrap().map(|r| r.unwrap().0.value()).collect() };
        for id in ids {
            // The seal (32 bytes) is kept; the words after it are changed by hand.
            let stored = t.get(id).unwrap().unwrap().value().to_vec();
            let (seal, json) = stored.split_at(32);
            let new = edit(String::from_utf8(json.to_vec()).unwrap());
            t.insert(id, [seal, new.as_bytes()].concat().as_slice())
                .unwrap();
        }
    }
    tx.commit().unwrap();
}

/// Review P1: a journal record changed by hand (another package, removal allowed) must never make
/// undo remove an app the person already had.
#[test]
fn a_record_changed_by_hand_never_makes_undo_remove_an_app() {
    let (_d, path, j) = journal();
    let mut laptop = Laptop::ok();
    laptop.installed.insert("Person.App".into(), "9.0".into());
    // Recipe does NOT allow removal (default declaration).
    moved(&j, &mut laptop, vec![recipe(vec![install("A.B")])]);
    drop(j);
    tamper(&path, |r| {
        r.replace("A.B", "Person.App")
            .replace("\"uninstall\":false", "\"uninstall\":true")
    });
    let j = common::open(&path).unwrap();
    let permit = j.begin_undo().unwrap();
    let got: Vec<_> = undo_effects(&rebuild(&j), &permit, &mut laptop, SCOPE, &common::book())
        .unwrap()
        .into_iter()
        .map(|Undone { outcome, .. }| outcome)
        .collect();
    // The changed record never drives undo: the person's own app stays.
    assert!(
        !laptop.done.iter().any(|d| d.starts_with("uninstall")),
        "{:?}",
        laptop.done
    );
    assert!(laptop.installed.contains_key("Person.App"));
    assert!(
        got.iter()
            .all(|o| !matches!(o, EffectUndoOutcome::Uninstalled)),
        "{got:?}"
    );
}

/// Review P2: the person saves a settings file between undo's check and its write.
struct Racy {
    inner: Laptop,
    place: SettingsPlace,
    armed: bool,
    /// The person sets the setting `k` to this after undo looked and before it writes.
    before_write: Option<SettingValue>,
    /// ... or right after undo wrote it.
    after_write: Option<SettingValue>,
    /// The settings files are on the real file system here (the place's own folder).
    real: Option<std::path::PathBuf>,
    /// The laptop loses power at this moment of putting a file back.
    crash_at: Option<pctwin_executor::files::Moment>,
    /// The person removes the file just before it is put back.
    remove_before_put: bool,
    /// The person saves on the real file just before it is put back (second review, S1).
    person_saves_real: bool,
    /// Windows: another program (a backup, an indexer) reads the moved copy without letting it
    /// be moved, from the moment it is aside until putting it back gives up (second review, S1).
    lock_aside: bool,
    /// Putting a file back fails this many times: the first moves it aside and then fails (a
    /// full disk writing the earlier version), the rest fail at once.
    fail_after_aside: u32,
    /// How many times putting a file back was asked for.
    asked: u32,
    /// The other program's open copies (dropped by the test when it lets go).
    held: Vec<std::fs::File>,
}

impl Racy {
    fn new(inner: Laptop, place: SettingsPlace) -> Self {
        Self {
            inner,
            place,
            armed: false,
            before_write: None,
            after_write: None,
            real: None,
            crash_at: None,
            remove_before_put: false,
            person_saves_real: false,
            lock_aside: false,
            fail_after_aside: 0,
            asked: 0,
            held: Vec::new(),
        }
    }
}

impl Adapter for Racy {
    fn platform(&self) -> Platform {
        self.inner.platform()
    }
    fn install(
        &mut self,
        g: pctwin_grant::InstallGrant,
        c: Catalog,
        p: &str,
    ) -> Result<(), Reason> {
        self.inner.install(g, c, p)
    }
    fn copy_settings(
        &mut self,
        g: pctwin_grant::SettingGrant,
        f: &SettingsPlace,
        t: &SettingsPlace,
    ) -> Result<(), Reason> {
        self.inner.copy_settings(g, f, t)
    }
    fn apply_setting(
        &mut self,
        g: pctwin_grant::SettingGrant,
        k: &str,
        v: &SettingValue,
    ) -> Result<(), Reason> {
        self.inner.apply_setting(g, k, v)
    }
    fn verify(&mut self, c: &Check) -> bool {
        self.inner.verify(c)
    }
    fn app_presence(&mut self, c: Catalog, p: &str) -> Result<Presence, String> {
        self.inner.app_presence(c, p)
    }
    fn install_running(&mut self, c: Catalog, p: &str) -> Result<bool, String> {
        self.inner.install_running(c, p)
    }
    fn read_setting(&mut self, k: &str) -> Result<Option<SettingValue>, String> {
        self.inner.read_setting(k)
    }
    fn read_settings_files(&mut self, p: &SettingsPlace) -> Result<Vec<SettingsFile>, String> {
        if let Some(dir) = &self.real {
            // Undo's private names are never settings files.
            let mut out = Vec::new();
            for e in std::fs::read_dir(dir).unwrap() {
                let name = e.unwrap().file_name().to_string_lossy().into_owned();
                if !name.starts_with(".pctwin-undo-") {
                    let bytes = std::fs::read(dir.join(&name)).unwrap();
                    out.push(SettingsFile { path: name, bytes });
                }
            }
            return Ok(out);
        }
        self.inner.read_settings_files(p)
    }
    fn restore_point(&mut self) -> pctwin_journal::RestorePoint {
        self.inner.restore_point()
    }
    fn uninstall(&mut self, c: Catalog, p: &str) -> Result<(), String> {
        self.inner.uninstall(c, p)
    }
    fn write_setting_if(
        &mut self,
        k: &str,
        e: &SettingValue,
        v: Option<&SettingValue>,
    ) -> Result<pctwin_executor::SettingWrite, String> {
        if let Some(theirs) = self.before_write.take() {
            self.inner.settings.insert(k.into(), theirs);
        }
        let wrote = self.inner.write_setting_if(k, e, v);
        if let Some(theirs) = self.after_write.take() {
            self.inner.settings.insert(k.into(), theirs);
        }
        wrote
    }
    fn replace_settings_file(
        &mut self,
        p: &SettingsPlace,
        path: &str,
        swap: &pctwin_executor::files::Swap<'_>,
    ) -> Result<pctwin_executor::files::Swapped, String> {
        if self.armed {
            // The person saves, after undo looked at the file and before it is put back.
            self.inner
                .put_file(&self.place, "vlcrc", b"the person's new edit");
            self.armed = false;
        }
        self.asked += 1;
        if let Some(dir) = self.real.clone() {
            if std::mem::take(&mut self.remove_before_put) {
                std::fs::remove_file(dir.join(path)).unwrap();
            }
            if std::mem::take(&mut self.person_saves_real) {
                std::fs::write(dir.join(path), b"the person's new edit").unwrap();
            }
            if self.fail_after_aside > 0 {
                self.fail_after_aside -= 1;
                if dir.join(path).exists() && !dir.join(swap.private).exists() {
                    std::fs::rename(dir.join(path), dir.join(swap.private)).unwrap();
                }
                return Err("There is not enough space on the disk.".into());
            }
            let crash_at = self.crash_at.take();
            let lock_aside = self.lock_aside;
            #[cfg_attr(not(windows), expect(unused_mut, reason = "only Windows holds it"))]
            let mut lock: Option<std::fs::File> = None;
            let private = swap.private.to_string();
            let got = pctwin_executor::files::replace_if_unchanged(&dir, path, swap, &mut |m| {
                if Some(m) == crash_at {
                    panic!("the laptop lost power");
                }
                #[cfg(windows)]
                if lock_aside && m == pctwin_executor::files::Moment::Aside {
                    use std::os::windows::fs::OpenOptionsExt;
                    // Others may read it, but not move or remove it.
                    lock = std::fs::OpenOptions::new()
                        .read(true)
                        .share_mode(0x1)
                        .open(dir.join(&private))
                        .ok();
                }
            });
            self.held.extend(lock);
            #[cfg(not(windows))]
            let _ = (lock_aside, &private);
            return got.map_err(|e| e.to_string());
        }
        self.inner.replace_settings_file(p, path, swap)
    }
}

#[test]
fn an_edit_saved_between_check_and_write_is_kept() {
    let (_d, _p, j) = journal();
    let mut laptop = Laptop::ok();
    let p = place("vlc");
    laptop.put_file(&p, "vlcrc", b"defaults");
    laptop.put_source(&p, "vlcrc", b"old laptop vlcrc");
    moved(&j, &mut laptop, vec![recipe(vec![copy("vlc")])]);
    let mut racy = Racy::new(laptop, p.clone());
    racy.armed = true;
    let permit = j.begin_undo().unwrap();
    let got = undo_effects(&rebuild(&j), &permit, &mut racy, SCOPE, &common::book()).unwrap();
    // The person's edit is still there, and undo says it kept it.
    assert_eq!(
        racy.inner.file(&p, "vlcrc").unwrap().as_slice(),
        b"the person's new edit"
    );
    assert!(
        matches!(got[0].outcome, EffectUndoOutcome::Kept { .. }),
        "{got:?}"
    );
}

/// Review finding 1: each change reaches the adapter only with its own role's permission, which
/// names the approved plan and exactly that step.
#[test]
fn each_change_is_handed_its_roles_permission_for_that_step() {
    let (_d, _p, j) = journal();
    let mut laptop = Laptop::ok();
    laptop.put_source(&place("vlc"), "vlcrc", b"old laptop vlcrc");
    let steps = vec![
        install("A.B"),
        common::set("k", SettingValue::Bool(true)),
        copy("vlc"),
    ];
    let plan = plan_of(vec![recipe(steps.clone())]);
    let run = plan.run(&mut laptop, &rebuild(&j), SCOPE, &common::actor());
    assert_eq!(run.stopped, None);
    let want: Vec<(&str, u64, [u8; 32])> = ["install", "set", "copy"]
        .into_iter()
        .zip(&steps)
        .enumerate()
        .map(|(n, (what, step))| {
            (
                what,
                n as u64,
                pctwin_executor::object_of(n as u32, step).digest(),
            )
        })
        .collect();
    assert_eq!(laptop.grants, want);
    // Two steps never share a digest, and the seal names the approved revision.
    assert_ne!(want[0].2, want[1].2);
    assert_eq!(plan.seal().generation(), 1);
}

/// The seal names the revision the person approved, and another approval another digest.
#[test]
fn the_seal_names_the_approved_revision() {
    let (record, approval) = common::approved(vec![common::app("a")]);
    let next = record.next_revision();
    let next_approval = next.approve().unwrap();
    let one = pctwin_executor::seal_of(&approval);
    let two = pctwin_executor::seal_of(&next_approval);
    assert_eq!(one.generation(), record.revision);
    assert_eq!(two.generation(), next.revision);
    assert_ne!(one.generation(), two.generation());
    assert_ne!(one.digest(), two.digest());
}

/// Finding 3: undo's rules come from the checked recipe looked up again, never from the record.
/// The move ran with a recipe that allowed removal; the checked recipe of that ID and version now
/// says it does not: undo keeps the app.
#[test]
fn undo_follows_the_checked_recipe_not_the_record() {
    let (_d, _p, j) = journal();
    let mut laptop = Laptop::ok();
    let r = common::removable(vec![install("A.B")]);
    moved(&j, &mut laptop, vec![r.clone()]);
    let mut strict = r;
    strict.undo = Default::default();
    let permit = j.begin_undo().unwrap();
    let got = undo_effects(&rebuild(&j), &permit, &mut laptop, SCOPE, &vec![strict]).unwrap();
    assert!(laptop.installed.contains_key("A.B"));
    assert!(
        matches!(got[0].outcome, EffectUndoOutcome::Kept { .. }),
        "{got:?}"
    );
}

/// Finding 3: a change whose recipe cannot be found, or whose step is not the recipe's, is left.
#[test]
fn undo_without_the_checked_recipe_leaves_the_change() {
    for book in [vec![], vec![recipe(vec![install("Other.App")])]] {
        let (_d, _p, j) = journal();
        let mut laptop = Laptop::ok();
        let r = common::removable(vec![install("A.B")]);
        moved(&j, &mut laptop, vec![r.clone()]);
        // The same ID and version, another step (when the book holds one).
        let book: Vec<_> = book
            .into_iter()
            .map(|mut other| {
                other.id = r.id.clone();
                other.undo = r.undo.clone();
                other
            })
            .collect();
        let permit = j.begin_undo().unwrap();
        let got = undo_effects(&rebuild(&j), &permit, &mut laptop, SCOPE, &book).unwrap();
        let want = if book.is_empty() {
            pctwin_executor::NO_RECIPE
        } else {
            pctwin_executor::NOT_THE_RECIPE
        };
        assert_eq!(got[0].outcome, EffectUndoOutcome::Kept { why: want.into() });
        assert!(laptop.installed.contains_key("A.B"));
        assert!(laptop.done.is_empty());
    }
}

/// Finding 3: one damaged record is reported and drives nothing; the rest are still undone.
#[test]
fn one_damaged_record_does_not_stop_the_rest() {
    let (_d, path, j) = journal();
    let mut laptop = Laptop::ok();
    moved(
        &j,
        &mut laptop,
        vec![
            common::removable(vec![install("A.B")]),
            common::removable(vec![install("C.D")]),
        ],
    );
    drop(j);
    let db = redb::Database::open(&path).unwrap();
    let tx = db.begin_write().unwrap();
    {
        let mut t = tx
            .open_table(redb::TableDefinition::<u64, &[u8]>::new("effects"))
            .unwrap();
        let mut first = t.get(1).unwrap().unwrap().value().to_vec();
        first[0] ^= 1;
        t.insert(1, first.as_slice()).unwrap();
    }
    tx.commit().unwrap();
    drop(db);
    let j = common::open(&path).unwrap();
    let permit = j.begin_undo().unwrap();
    let got = undo_effects(&rebuild(&j), &permit, &mut laptop, SCOPE, &common::book()).unwrap();
    assert_eq!(
        got[0],
        Undone {
            id: 1,
            outcome: EffectUndoOutcome::Kept {
                why: pctwin_executor::DAMAGED_RECORD.into()
            }
        }
    );
    assert_eq!(got[1].outcome, EffectUndoOutcome::Uninstalled);
    assert!(laptop.installed.contains_key("A.B"));
    assert!(!laptop.installed.contains_key("C.D"));
}

/// Finding 3 and T23-007: with its key gone, a journal holding effects makes no new key, erases
/// nothing, and drives no undo; with the key back, all is as it was.
#[test]
fn a_missing_key_is_waited_for_never_replaced() {
    let (_d, path, j) = journal();
    let mut laptop = Laptop::ok();
    moved(
        &j,
        &mut laptop,
        vec![common::removable(vec![install("A.B")])],
    );
    drop(j);
    let wrong = pctwin_journal::Journal::open_with_record_key(
        &path,
        pctwin_journal::keystore::TagKey::from_bytes([1; 32]),
    )
    .unwrap();
    let permit = wrong.begin_undo().unwrap();
    let got = undo_effects(
        &rebuild(&wrong),
        &permit,
        &mut laptop,
        SCOPE,
        &common::book(),
    );
    // With another key nothing in it reads as PCTwin's, its counter included (second review,
    // S2): the whole record is refused as damaged, and nothing is done.
    assert!(
        matches!(got, Err(pctwin_journal::JournalError::Damaged(_))),
        "{got:?}"
    );
    assert!(laptop.installed.contains_key("A.B"));
    drop(permit);
    drop(wrong);
    let j = common::open(&path).unwrap();
    assert_eq!(j.effects().unwrap().len(), 1, "nothing was erased");
}

/// Finding 3: a setting or a settings copy is put back only when it is exactly the step of the
/// checked recipe it names, and only from a recipe that passes its own checks.
#[test]
fn settings_are_undone_only_by_their_own_checked_recipe() {
    let steps = [common::set("k", SettingValue::Bool(true)), copy("vlc")];
    let others = [
        common::set("k", SettingValue::Bool(false)),
        Step::CopySettings {
            from: place("vlc"),
            to: place("other"),
        },
    ];
    for (step, other) in steps.into_iter().zip(others) {
        for broken in [false, true] {
            let (_d, _p, j) = journal();
            let mut laptop = Laptop::ok();
            laptop
                .settings
                .insert("k".into(), SettingValue::Bool(false));
            laptop.put_source(&place("vlc"), "vlcrc", b"old laptop vlcrc");
            let r = recipe(vec![step.clone()]);
            moved(&j, &mut laptop, vec![r.clone()]);
            let mut book = r.clone();
            if broken {
                // Its own recipe, failing its checks (no system named).
                book.platforms.clear();
            } else {
                book.steps = vec![other.clone()];
            }
            let before = (laptop.settings.clone(), laptop.files.clone());
            let permit = j.begin_undo().unwrap();
            let got = undo_effects(&rebuild(&j), &permit, &mut laptop, SCOPE, &vec![book]).unwrap();
            let want = if broken {
                pctwin_executor::NO_RECIPE
            } else {
                pctwin_executor::NOT_THE_RECIPE
            };
            assert_eq!(got[0].outcome, EffectUndoOutcome::Kept { why: want.into() });
            assert_eq!((laptop.settings.clone(), laptop.files.clone()), before);
        }
    }
}

/// Finding 2 for a setting: a choice the person makes after undo looked and before it writes is
/// kept (the write compares first); one made right after it wrote is kept too (read back).
#[test]
fn a_setting_changed_around_the_write_is_kept() {
    for (after, theirs) in [
        (false, SettingValue::Int(9)),
        (true, SettingValue::Int(9)),
        // The person chose the earlier value again themselves: still their choice, kept.
        (false, SettingValue::Int(1)),
    ] {
        let (_d, _p, j) = journal();
        let mut laptop = Laptop::ok();
        laptop.settings.insert("k".into(), SettingValue::Int(1));
        moved(
            &j,
            &mut laptop,
            vec![recipe(vec![common::set("k", SettingValue::Int(2))])],
        );
        let mut racy = Racy::new(laptop, place("vlc"));
        if after {
            racy.after_write = Some(theirs.clone());
        } else {
            racy.before_write = Some(theirs.clone());
        }
        let permit = j.begin_undo().unwrap();
        let got = undo_effects(&rebuild(&j), &permit, &mut racy, SCOPE, &common::book()).unwrap();
        assert!(
            matches!(got[0].outcome, EffectUndoOutcome::Kept { .. }),
            "{got:?}"
        );
        assert_eq!(racy.inner.settings["k"], theirs);
    }
}

/// Finding 2: the laptop loses power while a settings file is moved aside to be put back; undo
/// asked again finds it under the private name it recorded first, and finishes: the earlier
/// version is back and nothing is left under a private name.
#[test]
fn a_file_moved_aside_survives_a_crash_and_is_finished() {
    use pctwin_executor::files::Moment;
    for at in [Moment::Aside, Moment::Checked, Moment::Written] {
        let (_d, _p, j) = journal();
        let mut laptop = Laptop::ok();
        let p = place("vlc");
        laptop.put_file(&p, "vlcrc", b"defaults");
        laptop.put_source(&p, "vlcrc", b"old laptop vlcrc");
        moved(&j, &mut laptop, vec![recipe(vec![copy("vlc")])]);
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("vlcrc"), b"old laptop vlcrc").unwrap();
        let mut real = Racy::new(laptop, p.clone());
        real.real = Some(dir.path().to_path_buf());
        real.crash_at = Some(at);
        let permit = j.begin_undo().unwrap();
        let crashed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            undo_effects(&rebuild(&j), &permit, &mut real, SCOPE, &common::book())
        }));
        assert!(crashed.is_err(), "{at:?}");
        // Still part way: closing undo waits for it.
        assert_eq!(j.effects_part_way().unwrap().len(), 1, "{at:?}");
        let got = undo_effects(&rebuild(&j), &permit, &mut real, SCOPE, &common::book()).unwrap();
        assert_eq!(got[0].outcome, EffectUndoOutcome::Restored, "{at:?}");
        assert_eq!(
            std::fs::read(dir.path().join("vlcrc")).unwrap(),
            b"defaults"
        );
        let left: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
        assert_eq!(left.len(), 1, "{at:?}: nothing left under a private name");
        assert!(j.effects_part_way().unwrap().is_empty());
    }
}

/// Finding 2: the person removes the settings file after undo looked and before it is put back:
/// nothing is written in its place, and undo says it kept their change.
#[test]
fn a_file_removed_before_it_is_put_back_is_left_removed() {
    let (_d, _p, j) = journal();
    let mut laptop = Laptop::ok();
    let p = place("vlc");
    laptop.put_file(&p, "vlcrc", b"defaults");
    laptop.put_source(&p, "vlcrc", b"old laptop vlcrc");
    moved(&j, &mut laptop, vec![recipe(vec![copy("vlc")])]);
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("vlcrc"), b"old laptop vlcrc").unwrap();
    let mut real = Racy::new(laptop, p.clone());
    real.real = Some(dir.path().to_path_buf());
    real.remove_before_put = true;
    let permit = j.begin_undo().unwrap();
    let got = undo_effects(&rebuild(&j), &permit, &mut real, SCOPE, &common::book()).unwrap();
    assert!(
        matches!(got[0].outcome, EffectUndoOutcome::Kept { .. }),
        "{got:?}"
    );
    assert!(!dir.path().join("vlcrc").exists());
}

fn names_in(dir: &std::path::Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

fn hidden_in(dir: &std::path::Path) -> Vec<String> {
    names_in(dir)
        .into_iter()
        .filter(|n| n.starts_with(".pctwin-undo-"))
        .collect()
}

/// A settings file moved aside, with the place on the real file system: the laptop after a copy
/// of VLC's settings, and the adapter that puts files back there.
fn vlc_moved_on_disk(j: &Journal) -> (tempfile::TempDir, Racy) {
    let mut laptop = Laptop::ok();
    let p = place("vlc");
    laptop.put_file(&p, "vlcrc", b"defaults");
    laptop.put_source(&p, "vlcrc", b"old laptop vlcrc");
    moved(j, &mut laptop, vec![recipe(vec![copy("vlc")])]);
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("vlcrc"), b"old laptop vlcrc").unwrap();
    let mut real = Racy::new(laptop, p);
    real.real = Some(dir.path().to_path_buf());
    (dir, real)
}

/// Second review, finding S1: the person saves between undo's look and putting the file back,
/// and another program holds the moved copy so it cannot be put back yet. Undo must not forget
/// where the person's file is: the record stays part way (closing undo waits), the person is told
/// the hidden name, and the next undo puts their file back under its name and says it was kept.
#[cfg(windows)]
#[test]
fn a_held_moved_copy_keeps_undo_part_way_until_the_persons_file_is_back() {
    let (_d, _p, j) = journal();
    let (dir, mut real) = vlc_moved_on_disk(&j);
    real.person_saves_real = true;
    real.lock_aside = true;
    let permit = j.begin_undo().unwrap();
    let first = undo_effects(&rebuild(&j), &permit, &mut real, SCOPE, &common::book()).unwrap();
    let id = first[0].id;
    let hidden = hidden_in(dir.path());
    assert_eq!(hidden.len(), 1, "{:?}", names_in(dir.path()));
    let EffectUndoOutcome::NotDone { why } = &first[0].outcome else {
        panic!("{first:?}")
    };
    assert!(why.contains(&hidden[0]) && why.contains("vlcrc"), "{why}");
    assert!(matches!(
        j.effect_undo_of(id).unwrap(),
        Some(pctwin_journal::EffectUndo::Swapping { private, .. }) if private == hidden[0]
    ));
    assert_eq!(j.effects_part_way().unwrap(), vec![id]);
    // It was tried a few times in a row before undo stopped for now.
    assert!(real.asked >= 3, "{}", real.asked);
    // Closing undo waits while the person's file is hidden.
    drop(permit);
    real.lock_aside = false;
    real.held.clear();
    assert!(matches!(
        j.close_undo(&mut |_, _| unreachable!()),
        Err(pctwin_journal::CloseError::EffectsPartWay { ids }) if ids == vec![id]
    ));
    let permit = j.begin_undo().unwrap();
    let second = undo_effects(&rebuild(&j), &permit, &mut real, SCOPE, &common::book()).unwrap();
    assert_eq!(
        std::fs::read(dir.path().join("vlcrc")).unwrap(),
        b"the person's new edit"
    );
    assert!(
        hidden_in(dir.path()).is_empty(),
        "{:?}",
        names_in(dir.path())
    );
    assert!(j.effects_part_way().unwrap().is_empty());
    assert!(
        matches!(second[0].outcome, EffectUndoOutcome::Kept { .. }),
        "{second:?}"
    );
}

/// Second review, finding S1, on every system: putting the file back fails after it was moved
/// aside (a full disk writing the earlier version). The record stays part way with the same
/// private name, the person is told where their file is, and the next undo finishes the job with
/// that name: PCTwin's unchanged version is replaced by the earlier one.
#[test]
fn an_error_after_the_move_aside_is_finished_next_time_with_the_same_name() {
    let (_d, _p, j) = journal();
    let (dir, mut real) = vlc_moved_on_disk(&j);
    real.fail_after_aside = 3;
    let permit = j.begin_undo().unwrap();
    let first = undo_effects(&rebuild(&j), &permit, &mut real, SCOPE, &common::book()).unwrap();
    let id = first[0].id;
    assert_eq!(real.asked, 3);
    let hidden = hidden_in(dir.path());
    assert_eq!(hidden.len(), 1);
    assert!(!dir.path().join("vlcrc").exists());
    let EffectUndoOutcome::NotDone { why } = &first[0].outcome else {
        panic!("{first:?}")
    };
    assert!(why.contains(&hidden[0]), "{why}");
    assert_eq!(j.effects_part_way().unwrap(), vec![id]);
    // Asked again in the same run of undo (or after a restart): the same name is used.
    let second = undo_effects(&rebuild(&j), &permit, &mut real, SCOPE, &common::book()).unwrap();
    assert!(
        hidden_in(dir.path()).is_empty(),
        "{:?}",
        names_in(dir.path())
    );
    assert_eq!(
        std::fs::read(dir.path().join("vlcrc")).unwrap(),
        b"defaults"
    );
    assert_eq!(second[0].outcome, EffectUndoOutcome::Restored);
    assert!(j.effects_part_way().unwrap().is_empty());
}

/// Second review, finding S1: a transient error that clears within the tries is never seen.
#[test]
fn a_brief_error_is_tried_again_at_once() {
    let (_d, _p, j) = journal();
    let (dir, mut real) = vlc_moved_on_disk(&j);
    real.fail_after_aside = 2;
    let permit = j.begin_undo().unwrap();
    let got = undo_effects(&rebuild(&j), &permit, &mut real, SCOPE, &common::book()).unwrap();
    assert_eq!(got[0].outcome, EffectUndoOutcome::Restored);
    assert_eq!(
        std::fs::read(dir.path().join("vlcrc")).unwrap(),
        b"defaults"
    );
    assert!(hidden_in(dir.path()).is_empty());
}

/// Second review, finding S1: the journal itself refuses anything over a file being put back
/// except "settled" for that same file and name.
#[test]
fn only_a_settled_file_may_follow_one_being_put_back() {
    use pctwin_journal::EffectUndo::*;
    let (_d, _p, j) = journal();
    let mut laptop = Laptop::ok();
    let p = place("vlc");
    laptop.put_file(&p, "vlcrc", b"defaults");
    laptop.put_source(&p, "vlcrc", b"old laptop vlcrc");
    moved(&j, &mut laptop, vec![recipe(vec![copy("vlc")])]);
    let id = j.effects_checked(SCOPE).unwrap().effects[0].id;
    let permit = j.begin_undo().unwrap();
    let r = rebuild(&j);
    let a = ".pctwin-undo-0123456789abcdef0123456789abcdef".to_string();
    let b = ".pctwin-undo-fedcba9876543210fedcba9876543210".to_string();
    let swapping = |path: &str, private: &str| Swapping {
        path: path.into(),
        private: private.into(),
    };
    let put_back = |path: &str, private: &str| PutBack {
        path: path.into(),
        private: private.into(),
    };
    // Settled before anything was moved aside: refused.
    assert!(
        r.settings
            .record_undo(&permit, id, &put_back("vlcrc", &a))
            .is_err()
    );
    r.settings
        .record_undo(&permit, id, &swapping("vlcrc", &a))
        .unwrap();
    for next in [
        Done {
            outcome: EffectUndoOutcome::NotDone { why: "x".into() },
        },
        Done {
            outcome: EffectUndoOutcome::Restored,
        },
        Reversing,
        swapping("vlcrc", &b),
        swapping("other", &a),
        put_back("vlcrc", &b),
        put_back("other", &a),
    ] {
        assert!(
            r.settings.record_undo(&permit, id, &next).is_err(),
            "{next:?}"
        );
    }
    assert_eq!(j.effects_part_way().unwrap(), vec![id]);
    r.settings
        .record_undo(&permit, id, &put_back("vlcrc", &a))
        .unwrap();
    // Still part way until the end is recorded; the next file may follow.
    assert_eq!(j.effects_part_way().unwrap(), vec![id]);
    assert!(r.settings.record_undo(&permit, id, &Reversing).is_err());
    r.settings
        .record_undo(&permit, id, &swapping("vlcrc", &b))
        .unwrap();
    r.settings
        .record_undo(&permit, id, &put_back("vlcrc", &b))
        .unwrap();
    r.settings
        .record_undo(
            &permit,
            id,
            &Done {
                outcome: EffectUndoOutcome::Restored,
            },
        )
        .unwrap();
    assert!(j.effects_part_way().unwrap().is_empty());
}

/// Second review, finding S1: a file was moved aside, the laptop lost power, and then the checked
/// instructions can no longer be found. Undo leaves the change as it is, but first brings the
/// file out of hiding under its own name; only then is it finished.
#[test]
fn a_hidden_file_comes_back_even_when_undo_may_not_put_it_back() {
    use pctwin_executor::files::Moment;
    for (at, person) in [
        (Moment::Aside, false),
        (Moment::Checked, false),
        (Moment::Aside, true),
    ] {
        let (_d, _p, j) = journal();
        let (dir, mut real) = vlc_moved_on_disk(&j);
        real.crash_at = Some(at);
        real.person_saves_real = person;
        let permit = j.begin_undo().unwrap();
        let crashed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            undo_effects(&rebuild(&j), &permit, &mut real, SCOPE, &common::book())
        }));
        assert!(crashed.is_err());
        assert_eq!(hidden_in(dir.path()).len(), 1);
        let none: Vec<pctwin_recipe::Recipe> = Vec::new();
        let got = undo_effects(&rebuild(&j), &permit, &mut real, SCOPE, &none).unwrap();
        assert_eq!(
            got[0].outcome,
            EffectUndoOutcome::Kept {
                why: pctwin_executor::NO_RECIPE.into()
            },
            "{at:?}"
        );
        let want: &[u8] = if person {
            b"the person's new edit"
        } else {
            b"old laptop vlcrc"
        };
        assert_eq!(std::fs::read(dir.path().join("vlcrc")).unwrap(), want);
        assert_eq!(names_in(dir.path()), vec!["vlcrc".to_string()], "{at:?}");
        assert!(j.effects_part_way().unwrap().is_empty());
    }
}

/// Second review, finding S2: an app's undo finished (removed), the person installed it again
/// themselves, and then an older copy of the whole journal file was put back (a backup restored).
/// Every record in it is sealed right, but it is older than the last change PCTwin recorded:
/// it is refused, so undo never removes the person's own reinstall.
#[test]
fn a_journal_put_back_from_an_older_copy_is_refused() {
    let (d, path, j) = journal();
    let mut laptop = Laptop::ok();
    moved(
        &j,
        &mut laptop,
        vec![common::removable(vec![install("A.B")])],
    );
    drop(j);
    let snap = d.path().join("snapshot.redb");
    std::fs::copy(&path, &snap).unwrap();
    let j = common::open(&path).unwrap();
    {
        let permit = j.begin_undo().unwrap();
        let got = undo_effects(&rebuild(&j), &permit, &mut laptop, SCOPE, &common::book()).unwrap();
        assert_eq!(got[0].outcome, EffectUndoOutcome::Uninstalled);
    }
    drop(j);
    laptop.installed.insert("A.B".into(), "2.0".into());
    laptop.done.clear();
    let newest = d.path().join("newest.redb");
    std::fs::copy(&path, &newest).unwrap();
    std::fs::copy(&snap, &path).unwrap();
    let j = common::open(&path).unwrap();
    let permit = j.begin_undo().unwrap();
    let got = undo_effects(&rebuild(&j), &permit, &mut laptop, SCOPE, &common::book());
    assert!(
        matches!(got, Err(pctwin_journal::JournalError::RolledBack)),
        "{got:?}"
    );
    assert!(laptop.done.is_empty(), "{:?}", laptop.done);
    assert!(laptop.installed.contains_key("A.B"));
    // Nothing is trusted from it, not even what is part way, and it closes nothing.
    assert!(matches!(
        j.effects_part_way(),
        Err(pctwin_journal::JournalError::RolledBack)
    ));
    drop(permit);
    assert!(matches!(
        j.close_undo(&mut |_, _| unreachable!()),
        Err(pctwin_journal::CloseError::Journal(
            pctwin_journal::JournalError::RolledBack
        ))
    ));
    drop(j);
    // The newest copy put back is trusted again, and says the app's undo is finished.
    std::fs::copy(&newest, &path).unwrap();
    let j = common::open(&path).unwrap();
    let permit = j.begin_undo().unwrap();
    let got = undo_effects(&rebuild(&j), &permit, &mut laptop, SCOPE, &common::book()).unwrap();
    assert_eq!(got[0].outcome, EffectUndoOutcome::Uninstalled);
    assert!(laptop.done.is_empty(), "{:?}", laptop.done);
}
