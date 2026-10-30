//! The 2.5 API end to end, as the move calls it: plan on the old laptop, approve, capture, apply on
//! the new laptop through the journal, undo; with honest result words throughout.

mod common;

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use common::*;
use pctwin_catalogue::Catalogue;
use pctwin_gate::Destination;
use pctwin_grant::{CaptureEffect, Now, PlanSeal, RoleKeys, RootRef, SecretEffect};
use pctwin_journal::{Actor, Journal, Permission, PlannedEffect, Scope};
use pctwin_readiness::goodbye::ExceptionKind;
use pctwin_recipe::{RecipeId, RecipeRef};
use pctwin_record::{ItemId, Owner};
use pctwin_special::api::*;
use pctwin_special::capture::{Holders, Holding, Stop};
use pctwin_special::catalogue::{SHIPPED, SpecialRow};
use pctwin_special::component::Custody;
use pctwin_special::handoff::Step;
use pctwin_special::land::{Conflict, OwnerOnly, Undone};
use pctwin_special::roots::System;
use pctwin_types::{ItemStatus, Reason};

fn keys() -> &'static RoleKeys {
    static K: OnceLock<RoleKeys> = OnceLock::new();
    K.get_or_init(|| RoleKeys::mint().expect("minted once in this test program"))
}

const MIN: Duration = Duration::from_secs(60);

fn seal() -> PlanSeal {
    PlanSeal::new(1, [6; 32])
}

struct Free;
impl Holders for Free {
    fn holding(&self, _: &[PathBuf]) -> Holding {
        Holding::Free
    }
}

struct AppOwnerOnly;
impl OwnerOnly for AppOwnerOnly {
    fn private_folder(&self, path: &Path) -> std::io::Result<()> {
        #[cfg(windows)]
        return pctwin_helper_os::private_folder(path)
            .map_err(|e| std::io::Error::other(format!("{e:?}")));
        #[cfg(unix)]
        return pctwin_special::land::UnixOwnerOnly.private_folder(path);
    }
}

fn effect(step: u32) -> PlannedEffect {
    PlannedEffect {
        scope: Scope([2; 16]),
        item: ItemId::from_hex(&format!("cc{}", "0".repeat(30))).unwrap(),
        recipe: RecipeRef {
            id: RecipeId::new("special.move").unwrap(),
            version: 1,
        },
        step,
        actor: Actor {
            acting_account: "1001".into(),
            for_account: "1001".into(),
            permission: Permission::OwnFolders,
        },
    }
}

#[test]
fn a_person_s_app_data_moves_through_the_api_with_honest_words() {
    pctwin_special::content::install_reader(&OWN_READERS);
    let t = temp();
    let old = t.path().join("old");
    sqlite(
        &old.join("AppData/Roaming/Journal App/journal.db"),
        "CREATE TABLE notes(body TEXT); INSERT INTO notes VALUES('a'),('b');",
    );
    chromium_profile(&old.join("AppData/Local/Chromey/User Data"));
    ssh_folder(&old.join(".ssh"));
    let owner = Owner::Person {
        account_id: "1001".into(),
    };
    let cat = Catalogue::<SpecialRow>::shipped(SHIPPED);
    let plan = plan(&owner, &old, &[], System::Windows, System::Windows, &cat);
    assert!(!plan.components.is_empty());
    assert!(plan.not_moved.iter().any(|n| n.why.contains("cache")));
    for p in &plan.components {
        assert!(!p.why.is_empty());
        if p.component.secret {
            assert!(!p.proposed, "no secret is proposed");
        }
        if !matches!(
            p.component.custody,
            Custody::Portable | Custody::NeedsExport
        ) {
            assert!(!p.proposed);
            assert_ne!(p.step, Step::None);
        }
    }

    // The new laptop.
    let new = t.path().join("new");
    std::fs::create_dir_all(new.join("AppData/Roaming")).unwrap();
    std::fs::create_dir_all(new.join("AppData/Local")).unwrap();
    let home = Destination::open(&new).unwrap();
    let j = Journal::open_with_record_key(
        &t.path().join("j.redb"),
        pctwin_journal::keystore::TagKey::from_bytes([0x5a; 32]),
    )
    .unwrap();
    let rec = j.imports(&keys().capture);
    let stop = Stop::new();
    let made = MadeFolders::default();
    let n = NewLaptop {
        rec: &rec,
        home: &home,
        home_path: &new,
        root: RootRef::new(20),
        seal: seal(),
        system: System::Windows,
        owner_only: &AppOwnerOnly,
        stop: &stop,
        made: &made,
    };
    let k = &keys().capture;
    let mut step = 0;
    let mut results = Vec::new();
    let mut journal_effect = None;
    // A secret the person chose lands first (its own grants, owner-only); its app's other data
    // then joins its folder.
    let key = plan
        .components
        .iter()
        .find(|p| p.component.members == vec!["work_key".to_string()])
        .unwrap();
    let approved = approve(key, seal(), RootRef::new(10), 99);
    let work = t.path().join("work-key");
    std::fs::create_dir(&work).unwrap();
    let read = Grant::Secret(
        k.secret(
            seal(),
            vec![approved.object()],
            SecretEffect::Read,
            MIN,
            Now::real(),
        )
        .unwrap(),
    );
    let cap = capture_planned(key, read, &approved, &work, &Free, &stop, Now::real()).unwrap();
    let land = Grant::Secret(
        k.secret(
            seal(),
            vec![approved.object()],
            SecretEffect::Land,
            MIN,
            Now::real(),
        )
        .unwrap(),
    );
    let applied = apply(
        &n,
        land,
        &approved,
        &effect(500),
        key,
        &cap,
        &work,
        Conflict::KeepBoth,
        Now::real(),
    );
    // The person's keys land in `~/.ssh` itself (decided with the founder, 31 October 2026).
    assert_eq!(
        applied.result.status(),
        ItemStatus::Copied,
        "{}",
        applied.words
    );
    assert!(new.join(".ssh/work_key").is_file());
    for (i, p) in plan.landing_order().into_iter().enumerate() {
        if p.component.id == key.component.id {
            continue;
        }
        if !p.proposed {
            results.push((p, result_without_landing(p, false)));
            continue;
        }
        let approved = approve(p, seal(), RootRef::new(10), i as u64);
        let work = t.path().join(format!("work{i}"));
        std::fs::create_dir(&work).unwrap();
        let read = Grant::Ordinary(
            k.grant(
                seal(),
                vec![RootRef::new(10)],
                CaptureEffect::ReadPrivate,
                MIN,
                Now::real(),
            )
            .unwrap(),
        );
        let cap = capture_planned(p, read, &approved, &work, &Free, &stop, Now::real()).unwrap();
        let import = Grant::Ordinary(
            k.grant(
                seal(),
                vec![RootRef::new(20)],
                CaptureEffect::Import,
                MIN,
                Now::real(),
            )
            .unwrap(),
        );
        let applied = apply(
            &n,
            import,
            &approved,
            &effect(step),
            p,
            &cap,
            &work,
            Conflict::KeepBoth,
            Now::real(),
        );
        step += 1;
        if p.component.members.contains(&"journal.db".to_string()) {
            journal_effect = applied.landed.as_ref().map(|l| l.effect);
        }
        results.push((p, applied));
    }
    let word = |member: &str| {
        results
            .iter()
            .find(|(p, _)| p.component.members.iter().any(|m| m == member))
            .map(|(_, a)| (a.result.status(), a.result.reason()))
            .unwrap()
    };
    assert_eq!(word("journal.db"), (ItemStatus::Copied, None));
    assert_eq!(
        word("User Data/Default/History"),
        (ItemStatus::Copied, None)
    );
    assert_eq!(
        word("User Data/Default/Bookmarks"),
        (ItemStatus::Copied, None)
    );
    let db = new.join("AppData/Roaming/Journal App/journal.db");
    assert!(db.is_file());
    assert!(
        new.join("AppData/Local/Chromey/User Data/Default/History")
            .is_file()
    );
    // The public keys joined the secret's owner-only folder.
    assert!(new.join(".ssh/id_ed25519.pub").is_file());
    // Nothing machine-bound arrived.
    assert!(
        !new.join("AppData/Local/Chromey/User Data/Default/Cookies")
            .exists()
    );
    assert!(
        !new.join("AppData/Local/Chromey/User Data/Local State")
            .exists()
    );
    // The steps' words.
    let not_moved = results
        .iter()
        .filter(|(p, _)| p.component.custody == Custody::ServiceHeld)
        .map(|(_, a)| a.result)
        .next()
        .unwrap();
    assert_eq!(not_moved.status(), ItemStatus::SkippedByUser);
    let p = plan
        .components
        .iter()
        .find(|p| p.component.custody == Custody::ServiceHeld)
        .unwrap();
    assert_eq!(
        result_without_landing(p, true).result.reason(),
        Some(Reason::SignIn)
    );
    let cookies = plan
        .components
        .iter()
        .find(|p| p.component.members.iter().any(|m| m.ends_with("Cookies")))
        .unwrap();
    assert_eq!(
        result_without_landing(cookies, true).result.reason(),
        Some(Reason::FinishByHand)
    );

    // The before-wipe list holds everything that did not simply copy.
    let wipe = before_wipe(&plan, &|p| {
        results
            .iter()
            .find(|(q, _)| q.component.id == p.component.id)
            .map(|(_, a)| a.result)
    });
    assert!(
        wipe.iter()
            .any(|s| s.kind == ExceptionKind::TiedToOldLaptop)
    );
    assert!(
        wipe.iter()
            .any(|s| s.kind == ExceptionKind::DoFirstOnOldLaptop)
    );
    assert!(!wipe.iter().any(|s| s.place.contains("journal.db")));

    // The copied database's landing is recorded, so it can be undone.
    assert!(journal_effect.is_some());

    // Undo: the person changed nothing, so the landing is removed.
    #[cfg(windows)]
    {
        let permit = j.begin_undo().unwrap();
        let undone = undo(&rec, &permit, &home, journal_effect.unwrap()).unwrap();
        assert_eq!(undone, Undone::Removed);
        assert!(!db.exists());
        assert_eq!(
            undo(&rec, &permit, &home, journal_effect.unwrap()).unwrap(),
            Undone::AlreadyUndone
        );
    }
}

#[test]
fn undo_keeps_what_the_person_changed_since() {
    pctwin_special::content::install_reader(&OWN_READERS);
    // T25-054. On Linux and macOS on a drive undo removes from (Cargo's own folder for test files,
    // `.noindex` so Spotlight never holds a file open), and on a Mac with a test Trash on that
    // drive, never the person's own.
    let t = undo_temp();
    let old = t.path().join("old");
    put(&old.join("AppData/Roaming/Notes/a.txt"), b"one");
    put(&old.join("AppData/Roaming/Notes/b.txt"), b"two");
    let owner = Owner::Person {
        account_id: "1001".into(),
    };
    let cat = Catalogue::<SpecialRow>::shipped(SHIPPED);
    let plan = plan(&owner, &old, &[], System::Windows, System::Windows, &cat);
    let p = &plan.components[0];
    let new = t.path().join("new");
    std::fs::create_dir_all(new.join("AppData/Roaming")).unwrap();
    let home = Destination::open(&new).unwrap();
    let j = Journal::open_with_record_key(
        &t.path().join("j.redb"),
        pctwin_journal::keystore::TagKey::from_bytes([0x5a; 32]),
    )
    .unwrap();
    let rec = j.imports(&keys().capture);
    let stop = Stop::new();
    let made = MadeFolders::default();
    let n = NewLaptop {
        rec: &rec,
        home: &home,
        home_path: &new,
        root: RootRef::new(20),
        seal: seal(),
        system: System::Windows,
        owner_only: &AppOwnerOnly,
        stop: &stop,
        made: &made,
    };
    let k = &keys().capture;
    let approved = approve(p, seal(), RootRef::new(10), 1);
    let work = t.path().join("work");
    std::fs::create_dir(&work).unwrap();
    let cap = capture_planned(
        p,
        Grant::Ordinary(
            k.grant(
                seal(),
                vec![RootRef::new(10)],
                CaptureEffect::ReadPrivate,
                MIN,
                Now::real(),
            )
            .unwrap(),
        ),
        &approved,
        &work,
        &Free,
        &stop,
        Now::real(),
    )
    .unwrap();
    let applied = apply(
        &n,
        Grant::Ordinary(
            k.grant(
                seal(),
                vec![RootRef::new(20)],
                CaptureEffect::Import,
                MIN,
                Now::real(),
            )
            .unwrap(),
        ),
        &approved,
        &effect(0),
        p,
        &cap,
        &work,
        Conflict::KeepBoth,
        Now::real(),
    );
    assert_eq!(applied.result.status(), ItemStatus::Copied);
    let effect_id = applied.landed.unwrap().effect;
    // The person edits one file on the new laptop.
    std::fs::write(
        new.join("AppData/Roaming/Notes/a.txt"),
        b"the person's newer words",
    )
    .unwrap();
    let permit = j.begin_undo().unwrap();
    let undone = undo(&rec, &permit, &home, effect_id).unwrap();
    // The person's file stays, as they left it, and is said kept, on every system.
    assert_eq!(
        std::fs::read(new.join("AppData/Roaming/Notes/a.txt")).unwrap(),
        b"the person's newer words"
    );
    let b = new.join("AppData/Roaming/Notes/b.txt");
    // A drive undo cannot remove from on Linux or macOS (tmpfs, a container's own file system)
    // keeps both, said as kept; that is allowed only where the drive test is not required.
    if !cfg!(windows) && b.exists() {
        assert!(
            std::env::var_os("PCTWIN_REQUIRE_UNIX_UNDO").is_none(),
            "undo must remove here: {undone:?}"
        );
        assert_eq!(undone, Undone::KeptChanged { kept: 2 });
        return;
    }
    assert_eq!(undone, Undone::KeptChanged { kept: 1 });
    // What PCTwin wrote and nobody changed is gone from the app's place (on a Mac, in the Trash).
    assert!(!b.exists());
    #[cfg(target_os = "macos")]
    assert!(
        std::fs::read_dir(trash_home().join(".Trash")).is_ok_and(|entries| entries
            .flatten()
            .any(|e| std::fs::read(e.path()).is_ok_and(|bytes| bytes == b"two"))),
        "a Mac moves it into the Trash, never deletes it"
    );
}

/// macOS: the home whose `.Trash` undo uses in these tests (on the build's drive), never the
/// person's own.
#[cfg(target_os = "macos")]
fn trash_home() -> &'static std::path::Path {
    static HOME: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();
    HOME.get_or_init(|| {
        tempfile::Builder::new()
            .suffix(".noindex")
            .tempdir_in(env!("CARGO_TARGET_TMPDIR"))
            .unwrap()
    })
    .path()
}

/// A folder on a drive undo removes from (see the test above).
fn undo_temp() -> tempfile::TempDir {
    #[cfg(unix)]
    {
        #[cfg(target_os = "macos")]
        pctwin_gate::use_trash_home(trash_home().to_path_buf());
        tempfile::Builder::new()
            .suffix(".noindex")
            .tempdir_in(env!("CARGO_TARGET_TMPDIR"))
            .unwrap()
    }
    #[cfg(not(unix))]
    {
        temp()
    }
}

struct CannotTell;
impl Holders for CannotTell {
    fn holding(&self, _: &[PathBuf]) -> Holding {
        Holding::CannotTell
    }
}

#[test]
fn a_store_kept_only_as_found_never_lands_where_the_app_opens_it() {
    pctwin_special::content::install_reader(&OWN_READERS);
    let t = temp();
    let old = t.path().join("old");
    let d = old.join("AppData/Roaming/Store App/db");
    put(&d.join("CURRENT"), b"MANIFEST-000003\n");
    put(&d.join("MANIFEST-000003"), b"manifest");
    let owner = Owner::Person {
        account_id: "1001".into(),
    };
    let cat = Catalogue::<SpecialRow>::shipped(SHIPPED);
    let plan = plan(&owner, &old, &[], System::Windows, System::Windows, &cat);
    let p = &plan.components[0];
    let new = t.path().join("new");
    std::fs::create_dir_all(new.join("AppData/Roaming")).unwrap();
    let home = Destination::open(&new).unwrap();
    let j = Journal::open_with_record_key(
        &t.path().join("j.redb"),
        pctwin_journal::keystore::TagKey::from_bytes([0x5a; 32]),
    )
    .unwrap();
    let rec = j.imports(&keys().capture);
    let stop = Stop::new();
    let made = MadeFolders::default();
    let n = NewLaptop {
        rec: &rec,
        home: &home,
        home_path: &new,
        root: RootRef::new(20),
        seal: seal(),
        system: System::Windows,
        owner_only: &AppOwnerOnly,
        stop: &stop,
        made: &made,
    };
    let k = &keys().capture;
    let approved = approve(p, seal(), RootRef::new(10), 1);
    let work = t.path().join("work");
    std::fs::create_dir(&work).unwrap();
    let cap = capture_planned(
        p,
        Grant::Ordinary(
            k.grant(
                seal(),
                vec![RootRef::new(10)],
                CaptureEffect::ReadPrivate,
                MIN,
                Now::real(),
            )
            .unwrap(),
        ),
        &approved,
        &work,
        &CannotTell,
        &stop,
        Now::real(),
    )
    .unwrap();
    let applied = apply(
        &n,
        Grant::Ordinary(
            k.grant(
                seal(),
                vec![RootRef::new(20)],
                CaptureEffect::Import,
                MIN,
                Now::real(),
            )
            .unwrap(),
        ),
        &approved,
        &effect(0),
        p,
        &cap,
        &work,
        Conflict::KeepBoth,
        Now::real(),
    );
    assert_eq!(applied.result.status(), ItemStatus::NeedsAction);
    assert!(!new.join("AppData/Roaming/Store App").exists());
    assert!(
        new.join("PCTwin brought over/Store App/db/CURRENT")
            .is_file()
    );
}

/// PCTwin's readers of files' contents, run in the test's own process (only a parser crate's
/// tests may).
static OWN_READERS: pctwin_special::content::InThisProcess =
    pctwin_special::content::InThisProcess::new(pctwin_parse_seal::in_child());

/// Task List 2.9: a copy whose bytes all match is still "copied", but only a copy read while the
/// system confirmed the app was closed (or SQLite's own backup and check) is machine-checked as
/// usable; where nobody could say the app was closed, its readiness stays untested.
#[test]
fn matching_bytes_alone_never_label_app_data_machine_checked() {
    use pctwin_special::component::Readiness;
    pctwin_special::content::install_reader(&OWN_READERS);
    for (holders, want) in [
        (&Free as &dyn Holders, Readiness::MachineChecked),
        (&CannotTell as &dyn Holders, Readiness::Untested),
    ] {
        let t = temp();
        let old = t.path().join("old");
        put(
            &old.join("AppData/Roaming/Notes App/notes.json"),
            br#"{"notes":["a","b"]}"#,
        );
        let owner = Owner::Person {
            account_id: "1001".into(),
        };
        let cat = Catalogue::<SpecialRow>::shipped(SHIPPED);
        let plan = plan(&owner, &old, &[], System::Windows, System::Windows, &cat);
        let p = plan
            .components
            .iter()
            .find(|p| p.component.custody == Custody::Portable)
            .expect("a plain app data folder is portable");
        let new = t.path().join("new");
        std::fs::create_dir_all(new.join("AppData/Roaming")).unwrap();
        let home = Destination::open(&new).unwrap();
        let j = Journal::open_with_record_key(
            &t.path().join("j.redb"),
            pctwin_journal::keystore::TagKey::from_bytes([0x5a; 32]),
        )
        .unwrap();
        let rec = j.imports(&keys().capture);
        let stop = Stop::new();
        let made = MadeFolders::default();
        let n = NewLaptop {
            rec: &rec,
            home: &home,
            home_path: &new,
            root: RootRef::new(20),
            seal: seal(),
            system: System::Windows,
            owner_only: &AppOwnerOnly,
            stop: &stop,
            made: &made,
        };
        let k = &keys().capture;
        let approved = approve(p, seal(), RootRef::new(10), 1);
        let work = t.path().join("work");
        std::fs::create_dir(&work).unwrap();
        let grant = |root, effect| {
            Grant::Ordinary(
                k.grant(seal(), vec![RootRef::new(root)], effect, MIN, Now::real())
                    .unwrap(),
            )
        };
        let cap = capture_planned(
            p,
            grant(10, CaptureEffect::ReadPrivate),
            &approved,
            &work,
            holders,
            &stop,
            Now::real(),
        )
        .unwrap();
        let applied = apply(
            &n,
            grant(20, CaptureEffect::Import),
            &approved,
            &effect(0),
            p,
            &cap,
            &work,
            Conflict::KeepBoth,
            Now::real(),
        );
        assert_eq!(
            applied.result.status(),
            ItemStatus::Copied,
            "{}",
            applied.words
        );
        assert_eq!(applied.outcome.readiness, want, "{}", applied.words);
    }
}
