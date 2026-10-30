//! Landing on the new laptop (B25-03): through the safety gate, journaled, never over an existing
//! profile, secrets owner-only, and a crash finished by looking, not by writing again.

mod common;

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use common::*;
use pctwin_catalogue::Catalogue;
use pctwin_gate::Destination;
use pctwin_grant::{CaptureEffect, Now, PlanSeal, RoleKeys, RootRef, SecretEffect};
use pctwin_journal::{
    Actor, EffectState, Found, ImportIntent, Journal, Permission, PlannedEffect, Receipt, Scope,
};
use pctwin_recipe::{RecipeId, RecipeRef, RelPath, SettingsPlace, SettingsRoot};
use pctwin_record::{ItemId, Owner};
use pctwin_special::capture::*;
use pctwin_special::catalogue::{SHIPPED, SpecialRow};
use pctwin_special::classify::{FolderReport, classify_all};
use pctwin_special::component::Component;
use pctwin_special::land::*;
use pctwin_special::roots::{System, find_folders, roots_of};

fn keys() -> &'static RoleKeys {
    static K: OnceLock<RoleKeys> = OnceLock::new();
    K.get_or_init(|| RoleKeys::mint().expect("minted once in this test program"))
}

const MINUTE: Duration = Duration::from_secs(60);

fn seal() -> PlanSeal {
    PlanSeal::new(2, [8; 32])
}

fn source_root() -> RootRef {
    RootRef::new(1)
}

fn target_root() -> RootRef {
    RootRef::new(2)
}

fn planned(step: u32) -> PlannedEffect {
    PlannedEffect {
        scope: Scope([5; 16]),
        item: ItemId::from_hex(&format!("bb{}", "0".repeat(30))).unwrap(),
        recipe: RecipeRef {
            id: RecipeId::new("special.synthetic").unwrap(),
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

struct Free;
impl Holders for Free {
    fn holding(&self, _: &[PathBuf]) -> Holding {
        Holding::Free
    }
}

fn found(home: &Path) -> Vec<FolderReport> {
    let owner = Owner::Person {
        account_id: "1001".into(),
    };
    let (folders, _) = find_folders(
        &owner,
        home,
        &roots_of(System::Windows, home, &|_| None),
        &[],
    );
    classify_all(
        folders,
        System::Windows,
        &Catalogue::<SpecialRow>::shipped(SHIPPED),
    )
}

fn component<'a>(r: &'a [FolderReport], member: &str) -> (&'a FolderReport, &'a Component) {
    for f in r {
        if let Some(c) = f
            .components
            .iter()
            .find(|c| c.members.iter().any(|m| m == member))
        {
            return (f, c);
        }
    }
    panic!("{member} not found")
}

fn approve(f: &FolderReport, c: &Component) -> Approved {
    Approved {
        seal: seal(),
        component: c.id,
        root: source_root(),
        generation: generation(&f.folder.path(), &c.members),
        handle: 7,
    }
}

/// A captured journal-app database (three rows) and a settings file, in `work`.
fn captured(t: &tempfile::TempDir) -> (Capture, PathBuf) {
    let home = t.path().join("old");
    let d = home.join("AppData/Roaming/Journal App");
    sqlite(
        &d.join("data/journal.db"),
        "CREATE TABLE notes(body TEXT); INSERT INTO notes VALUES('a'),('b'),('c');",
    );
    let r = found(&home);
    let (f, c) = component(&r, "data/journal.db");
    let work = t.path().join("work");
    std::fs::create_dir(&work).unwrap();
    let g = keys()
        .capture
        .grant(
            seal(),
            vec![source_root()],
            CaptureEffect::ReadPrivate,
            MINUTE,
            Now::real(),
        )
        .unwrap();
    let cap = capture(
        g,
        &approve(f, c),
        c,
        &f.folder.path(),
        &work,
        &Free,
        &Stop::new(),
        Now::real(),
    )
    .unwrap();
    (cap, work)
}

fn import() -> pctwin_grant::CaptureGrant {
    keys()
        .capture
        .grant(
            seal(),
            vec![target_root()],
            CaptureEffect::Import,
            MINUTE,
            Now::real(),
        )
        .unwrap()
}

fn target() -> Target {
    Target {
        root: SettingsRoot::Config,
        folder: "Journal App".into(),
        made: None,
    }
}

#[test]
fn a_component_lands_through_the_gate_journaled_and_verified() {
    pctwin_special::content::install_reader(&OWN_READERS);
    let t = temp();
    let (cap, work) = captured(&t);
    let new = t.path().join("new");
    std::fs::create_dir(&new).unwrap();
    let dest = Destination::open(&new).unwrap();
    let j = Journal::open_with_record_key(
        &t.path().join("j.redb"),
        pctwin_journal::keystore::TagKey::from_bytes([0x5a; 32]),
    )
    .unwrap();
    let rec = j.imports(&keys().capture);
    let landed = land(
        &rec,
        import(),
        seal(),
        target_root(),
        &planned(0),
        &dest,
        &target(),
        &cap,
        &work,
        Conflict::KeepBoth,
        &Stop::new(),
        Now::real(),
    )
    .unwrap();
    assert!(landed.verified && !landed.beside);
    assert_eq!(landed.folder, "Journal App");
    let db = new.join("Journal App/data/journal.db");
    let c = rusqlite::Connection::open(&db).unwrap();
    let n: i64 = c
        .query_row("SELECT count(*) FROM notes", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 3);
    let e = j.effect(landed.effect).unwrap().unwrap();
    assert!(matches!(
        e.state,
        EffectState::Receipted {
            receipt: Receipt::Done,
            after_restart: false,
            ..
        }
    ));
}

#[test]
fn an_existing_profile_is_never_written_over() {
    pctwin_special::content::install_reader(&OWN_READERS);
    // T25-016: the person already uses the app on the new laptop.
    let t = temp();
    let (cap, work) = captured(&t);
    let new = t.path().join("new");
    put(
        &new.join("Journal App/data/journal.db"),
        b"the person's own new data",
    );
    let dest = Destination::open(&new).unwrap();
    let j = Journal::open_with_record_key(
        &t.path().join("j.redb"),
        pctwin_journal::keystore::TagKey::from_bytes([0x5a; 32]),
    )
    .unwrap();
    let rec = j.imports(&keys().capture);
    let skipped = land(
        &rec,
        import(),
        seal(),
        target_root(),
        &planned(0),
        &dest,
        &target(),
        &cap,
        &work,
        Conflict::Skip,
        &Stop::new(),
        Now::real(),
    );
    assert!(matches!(skipped, Err(LandError::Skipped)));
    assert!(j.effects().unwrap().is_empty(), "nothing was intended");
    let both = land(
        &rec,
        import(),
        seal(),
        target_root(),
        &planned(0),
        &dest,
        &target(),
        &cap,
        &work,
        Conflict::KeepBoth,
        &Stop::new(),
        Now::real(),
    )
    .unwrap();
    assert!(both.beside && both.verified);
    assert_eq!(both.folder, "Journal App (from old laptop)");
    assert_eq!(
        std::fs::read(new.join("Journal App/data/journal.db")).unwrap(),
        b"the person's own new data"
    );
    assert!(
        new.join("Journal App (from old laptop)/data/journal.db")
            .is_file()
    );
    // A third time: another new name, still nothing overwritten.
    let third = land(
        &rec,
        import(),
        seal(),
        target_root(),
        &planned(1),
        &dest,
        &target(),
        &cap,
        &work,
        Conflict::KeepBoth,
        &Stop::new(),
        Now::real(),
    )
    .unwrap();
    assert_eq!(third.folder, "Journal App (from old laptop 2)");
}

#[test]
fn landing_checks_its_grant() {
    pctwin_special::content::install_reader(&OWN_READERS);
    let t = temp();
    let (cap, work) = captured(&t);
    let new = t.path().join("new");
    std::fs::create_dir(&new).unwrap();
    let dest = Destination::open(&new).unwrap();
    let j = Journal::open_with_record_key(
        &t.path().join("j.redb"),
        pctwin_journal::keystore::TagKey::from_bytes([0x5a; 32]),
    )
    .unwrap();
    let rec = j.imports(&keys().capture);
    let k = &keys().capture;
    for g in [
        k.grant(
            seal(),
            vec![target_root()],
            CaptureEffect::ReadPrivate,
            MINUTE,
            Now::real(),
        )
        .unwrap(),
        k.grant(
            seal(),
            vec![source_root()],
            CaptureEffect::Import,
            MINUTE,
            Now::real(),
        )
        .unwrap(),
        k.grant(
            PlanSeal::new(3, [8; 32]),
            vec![target_root()],
            CaptureEffect::Import,
            MINUTE,
            Now::real(),
        )
        .unwrap(),
        k.grant(
            seal(),
            vec![target_root()],
            CaptureEffect::NativeExport,
            MINUTE,
            Now::real(),
        )
        .unwrap(),
    ] {
        let got = land(
            &rec,
            g,
            seal(),
            target_root(),
            &planned(0),
            &dest,
            &target(),
            &cap,
            &work,
            Conflict::KeepBoth,
            &Stop::new(),
            Now::real(),
        );
        assert!(matches!(got, Err(LandError::Denied(_))));
    }
    assert!(j.effects().unwrap().is_empty());
}

#[test]
fn a_stop_ends_the_landing_with_an_honest_record() {
    pctwin_special::content::install_reader(&OWN_READERS);
    // T25-007: revoked or cancelled while importing: no new work; what is there is recorded.
    let t = temp();
    let (cap, work) = captured(&t);
    let new = t.path().join("new");
    std::fs::create_dir(&new).unwrap();
    let dest = Destination::open(&new).unwrap();
    let j = Journal::open_with_record_key(
        &t.path().join("j.redb"),
        pctwin_journal::keystore::TagKey::from_bytes([0x5a; 32]),
    )
    .unwrap();
    let rec = j.imports(&keys().capture);
    let stop = Stop::new();
    stop.stop();
    let got = land(
        &rec,
        import(),
        seal(),
        target_root(),
        &planned(0),
        &dest,
        &target(),
        &cap,
        &work,
        Conflict::KeepBoth,
        &stop,
        Now::real(),
    );
    let Err(LandError::Stopped { effect }) = got else {
        panic!("{got:?}")
    };
    let e = j.effect(effect).unwrap().unwrap();
    assert!(matches!(
        e.state,
        EffectState::Receipted {
            receipt: Receipt::Failed { .. },
            found: Found::Files(ref f),
            ..
        } if f.is_empty()
    ));
}

#[test]
fn after_a_crash_the_record_is_finished_by_looking() {
    pctwin_special::content::install_reader(&OWN_READERS);
    // T25-056: the intent is on disk, the files may or may not be there.
    let t = temp();
    let (cap, work) = captured(&t);
    let new = t.path().join("new");
    std::fs::create_dir(&new).unwrap();
    let dest = Destination::open(&new).unwrap();
    let j = Journal::open_with_record_key(
        &t.path().join("j.redb"),
        pctwin_journal::keystore::TagKey::from_bytes([0x5a; 32]),
    )
    .unwrap();
    let rec = j.imports(&keys().capture);
    let place = SettingsPlace {
        root: SettingsRoot::Config,
        path: RelPath::new("Journal App").unwrap(),
    };
    let intend = |step| {
        rec.intend(
            &planned(step),
            &ImportIntent::Files {
                to: place.clone(),
                before: Vec::new(),
            },
        )
        .unwrap()
    };
    // Crashed before anything landed, with a half-written temporary file of its own.
    let a = intend(0);
    put(
        &new.join(format!(
            "Journal App/data/{}",
            pctwin_gate::temp_name(&format!("s25-{a}-0"))
        )),
        b"half",
    );
    assert_eq!(
        reconcile(&rec, &dest, a, &cap).unwrap(),
        Reconciled::Partial
    );
    assert!(
        !new.join(format!(
            "Journal App/data/{}",
            pctwin_gate::temp_name(&format!("s25-{a}-0"))
        ))
        .exists()
    );
    assert!(matches!(
        j.effect(a).unwrap().unwrap().state,
        EffectState::Receipted {
            after_restart: true,
            receipt: Receipt::Failed { .. },
            ..
        }
    ));
    // Crashed after every file landed, before it was observed.
    let b = intend(1);
    put(
        &new.join("Journal App/data/journal.db"),
        &std::fs::read(work.join("data/journal.db")).unwrap(),
    );
    assert_eq!(
        reconcile(&rec, &dest, b, &cap).unwrap(),
        Reconciled::Complete
    );
    assert!(matches!(
        j.effect(b).unwrap().unwrap().state,
        EffectState::Receipted {
            after_restart: true,
            receipt: Receipt::Done,
            ..
        }
    ));
    // Asked again: already finished, nothing changes.
    assert_eq!(
        reconcile(&rec, &dest, b, &cap).unwrap(),
        Reconciled::AlreadyFinished
    );
}

/// The owner-only folder, as the app makes it.
struct AppOwnerOnly;
impl OwnerOnly for AppOwnerOnly {
    fn private_folder(&self, path: &Path) -> std::io::Result<()> {
        #[cfg(windows)]
        return pctwin_helper_os::private_folder(path)
            .map_err(|e| std::io::Error::other(format!("{e:?}")));
        #[cfg(unix)]
        return UnixOwnerOnly.private_folder(path);
    }
}

#[test]
fn a_chosen_secret_lands_owner_only() {
    pctwin_special::content::install_reader(&OWN_READERS);
    let t = temp();
    let home = t.path().join("old");
    ssh_folder(&home.join(".ssh"));
    let r = found(&home);
    let (f, key) = component(&r, "work_key");
    let approved = approve(f, key);
    let work = t.path().join("work");
    std::fs::create_dir(&work).unwrap();
    let read = keys()
        .capture
        .secret(
            seal(),
            vec![approved.object()],
            SecretEffect::Read,
            MINUTE,
            Now::real(),
        )
        .unwrap();
    let cap = capture_secret(
        read,
        &approved,
        key,
        &f.folder.path(),
        &work,
        &Free,
        &Stop::new(),
        Now::real(),
    )
    .unwrap();
    let new = t.path().join("newhome");
    std::fs::create_dir(&new).unwrap();
    let dest = Destination::open(&new).unwrap();
    let j = Journal::open_with_record_key(
        &t.path().join("j.redb"),
        pctwin_journal::keystore::TagKey::from_bytes([0x5a; 32]),
    )
    .unwrap();
    let rec = j.imports(&keys().capture);
    // The person's keys land in `~/.ssh` itself, owner-only (decided with the founder, 31
    // October 2026); only the files that grant access or run something are kept aside.
    let target = Target {
        root: SettingsRoot::Home,
        folder: ".ssh".into(),
        made: None,
    };
    // A read grant does not land it.
    let wrong = keys()
        .capture
        .secret(
            seal(),
            vec![approved.object()],
            SecretEffect::Read,
            MINUTE,
            Now::real(),
        )
        .unwrap();
    assert!(matches!(
        land_secret(
            &rec,
            wrong,
            &approved,
            &planned(0),
            &dest,
            &new,
            &target,
            &cap,
            &work,
            &AppOwnerOnly,
            &Stop::new(),
            Now::real()
        ),
        Err(LandError::Denied(_))
    ));
    let g = keys()
        .capture
        .secret(
            seal(),
            vec![approved.object()],
            SecretEffect::Land,
            MINUTE,
            Now::real(),
        )
        .unwrap();
    let landed = land_secret(
        &rec,
        g,
        &approved,
        &planned(0),
        &dest,
        &new,
        &target,
        &cap,
        &work,
        &AppOwnerOnly,
        &Stop::new(),
        Now::real(),
    )
    .unwrap();
    assert!(landed.verified);
    let folder = new.join(".ssh");
    let file = folder.join("work_key");
    assert_eq!(
        std::fs::read(&file).unwrap(),
        std::fs::read(home.join(".ssh/work_key")).unwrap(),
        "the key is unchanged: its passphrase protection travels with it"
    );
    #[cfg(windows)]
    for p in [&folder, &file] {
        let acl = pctwin_helper_os::folder_access_list(p).unwrap();
        assert!(acl.contains(";;;SY)"), "{acl}");
        for others in [";;;BU)", ";;;AU)", ";;;WD)", ";;;BA)", ";;;IU)"] {
            assert!(!acl.contains(others), "{p:?}: {acl}");
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&folder).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            std::fs::metadata(&file).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

/// PCTwin's readers of files' contents, run in the test's own process (only a parser crate's
/// tests may).
static OWN_READERS: pctwin_special::content::InThisProcess =
    pctwin_special::content::InThisProcess::new(pctwin_parse_seal::in_child());

/// Phase 2 fresh-context security review, finding 1 (31 October 2026), defence in depth: whatever
/// a caller asks, the landing itself never writes where the system runs or trusts something at the
/// next sign-in (one shared table, `roots::trusted_place`), nor into a folder holding one; nothing
/// is made there.
#[test]
fn the_landing_refuses_every_place_the_system_runs_or_trusts_at_sign_in() {
    pctwin_special::content::install_reader(&OWN_READERS);
    let t = temp();
    let (cap, work) = captured(&t);
    let new = t.path().join("new");
    std::fs::create_dir(&new).unwrap();
    let dest = Destination::open(&new).unwrap();
    let j = Journal::open_with_record_key(
        &t.path().join("j.redb"),
        pctwin_journal::keystore::TagKey::from_bytes([0x5a; 32]),
    )
    .unwrap();
    let rec = j.imports(&keys().capture);
    for (root, folder) in [
        (SettingsRoot::Config, "autostart"),
        (SettingsRoot::Config, "systemd/user"),
        (SettingsRoot::Config, "environment.d"),
        (SettingsRoot::Home, ".config/autostart"),
        (SettingsRoot::Home, ".config"),
        (SettingsRoot::Home, "Library/LaunchAgents"),
        (
            SettingsRoot::Config,
            "Microsoft/Windows/Start Menu/Programs/Startup",
        ),
        (SettingsRoot::Home, "AppData/Roaming/Microsoft/Windows"),
        (SettingsRoot::LocalData, "systemd/user"),
    ] {
        let got = land(
            &rec,
            import(),
            seal(),
            target_root(),
            &planned(0),
            &dest,
            &Target {
                root,
                folder: folder.into(),
                made: None,
            },
            &cap,
            &work,
            Conflict::KeepBoth,
            &Stop::new(),
            Now::real(),
        );
        assert!(
            matches!(got, Err(LandError::Denied(_))),
            "{root:?} {folder}: {got:?}"
        );
    }
    assert_eq!(
        std::fs::read_dir(&new).unwrap().count(),
        0,
        "nothing was made"
    );
}

/// The files in `~/.ssh` that decide who may sign in remotely or can run a program when connecting
/// (`authorized_keys`, `config`...) are a component of their own, apart from the person's keys,
/// and the landing refuses to put them in `~/.ssh` by itself, whoever asks; kept in PCTwin's own
/// folder of things brought over, they land.
#[test]
fn ssh_files_that_grant_access_or_run_something_never_land_in_ssh() {
    pctwin_special::content::install_reader(&OWN_READERS);
    let t = temp();
    let home = t.path().join("old");
    ssh_folder(&home.join(".ssh"));
    put(
        &home.join(".ssh/authorized_keys"),
        b"ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAISyntheticKey friend@elsewhere\n",
    );
    put(
        &home.join(".ssh/config"),
        b"Host work\n  ProxyCommand /usr/bin/nc %h %p\n",
    );
    let r = found(&home);
    let (f, kept) = component(&r, "authorized_keys");
    assert!(kept.members.iter().any(|m| m == "config"), "{kept:?}");
    assert!(
        !kept
            .members
            .iter()
            .any(|m| m.starts_with("id_") || m == "known_hosts")
    );
    let (_, keys_c) = component(&r, "id_ed25519.pub");
    assert!(
        !keys_c
            .members
            .iter()
            .any(|m| m == "authorized_keys" || m == "config")
    );
    let work = t.path().join("work");
    std::fs::create_dir(&work).unwrap();
    let g = keys()
        .capture
        .grant(
            seal(),
            vec![source_root()],
            CaptureEffect::ReadPrivate,
            MINUTE,
            Now::real(),
        )
        .unwrap();
    let cap = capture(
        g,
        &approve(f, kept),
        kept,
        &f.folder.path(),
        &work,
        &Free,
        &Stop::new(),
        Now::real(),
    )
    .unwrap();
    let new = t.path().join("new");
    std::fs::create_dir(&new).unwrap();
    let dest = Destination::open(&new).unwrap();
    let j = Journal::open_with_record_key(
        &t.path().join("j.redb"),
        pctwin_journal::keystore::TagKey::from_bytes([0x5a; 32]),
    )
    .unwrap();
    let rec = j.imports(&keys().capture);
    let to = |folder: &str| Target {
        root: SettingsRoot::Home,
        folder: folder.into(),
        made: None,
    };
    let got = land(
        &rec,
        import(),
        seal(),
        target_root(),
        &planned(0),
        &dest,
        &to(".ssh"),
        &cap,
        &work,
        Conflict::KeepBoth,
        &Stop::new(),
        Now::real(),
    );
    assert!(matches!(got, Err(LandError::Denied(_))), "{got:?}");
    assert!(!new.join(".ssh").exists());
    let aside = land(
        &rec,
        import(),
        seal(),
        target_root(),
        &planned(1),
        &dest,
        &to("PCTwin brought over/.ssh"),
        &cap,
        &work,
        Conflict::KeepBoth,
        &Stop::new(),
        Now::real(),
    )
    .unwrap();
    assert!(aside.verified);
    assert!(
        new.join("PCTwin brought over/.ssh/authorized_keys")
            .is_file()
    );
}

/// GnuPG's `random_seed` never crosses (it must never be shared between laptops); its keyrings
/// are found as data, and its settings files (`*.conf`, `sshcontrol`) are a component of their own.
#[test]
fn gnupg_keyrings_move_its_settings_are_apart_and_its_random_seed_never_crosses() {
    pctwin_special::content::install_reader(&OWN_READERS);
    let t = temp();
    let home = t.path().join("old");
    let g = home.join(".gnupg");
    put(&g.join("pubring.kbx"), b"KBXf synthetic keyring");
    put(&g.join("trustdb.gpg"), b"synthetic trust db");
    put(&g.join("random_seed"), &[7u8; 600]);
    put(
        &g.join("gpg-agent.conf"),
        b"pinentry-program /usr/bin/pinentry-gtk\n",
    );
    put(&g.join("sshcontrol"), b"# none\n");
    let r = found(&home);
    let every: Vec<&String> = r
        .iter()
        .flat_map(|f| f.components.iter().flat_map(|c| c.members.iter()))
        .collect();
    assert!(
        !every.iter().any(|m| m.ends_with("random_seed")),
        "{every:?}"
    );
    let (_, conf) = component(&r, "gpg-agent.conf");
    assert!(conf.members.iter().any(|m| m == "sshcontrol"));
    assert!(!conf.members.iter().any(|m| m == "pubring.kbx"));
    let (_, ring) = component(&r, "pubring.kbx");
    assert!(!ring.members.iter().any(|m| m.ends_with(".conf")));
}
