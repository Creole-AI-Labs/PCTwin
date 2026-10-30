//! Phase 2 fresh-context security review, finding 1 (HIGH, 31 October 2026): a hostile old laptop
//! must never choose where app data lands. Here the old laptop is played by forged records sent
//! straight to the new laptop's landing, each naming a place the system runs or trusts at the
//! next sign-in (the desktop's autostart folder, systemd's user units, `~/.ssh`, Windows' Start
//! menu with its Startup folder), and each claiming the same kind of system on both sides, so the
//! old laptop's own word for the folder is all there is. Every one ends kept aside or refused,
//! said as such: nothing lands where the system would run or trust it.
// Tests make files to test on.
#![allow(clippy::disallowed_methods)]

use std::collections::BTreeMap;
use std::path::Path;

use pctwin_fullmove::data::{Arrived, Choice, Landing, PlacementWire, Record, Sink, land_all};
use pctwin_gate::Destination;
use pctwin_journal::{Actor, Journal, Permission, Scope};
use pctwin_record::Owner;
use pctwin_special::capture::{Capture, CapturedFile, Method, generation};
use pctwin_special::component::{Component, Custody, Evidence, Preservation};
use pctwin_special::roots::{RootKind, System};
use pctwin_types::ItemStatus;

fn keys() -> &'static pctwin_grant::RoleKeys {
    static KEYS: std::sync::OnceLock<pctwin_grant::RoleKeys> = std::sync::OnceLock::new();
    KEYS.get_or_init(|| pctwin_grant::RoleKeys::mint().expect("the keys are made once"))
}

struct AppOwnerOnly;
impl pctwin_special::land::OwnerOnly for AppOwnerOnly {
    fn private_folder(&self, path: &Path) -> std::io::Result<()> {
        #[cfg(windows)]
        return pctwin_helper_os::private_folder(path)
            .map_err(|e| std::io::Error::other(format!("{e:?}")));
        #[cfg(unix)]
        return pctwin_special::land::UnixOwnerOnly.private_folder(path);
    }
}

/// One forged component: `app` (as the plan named it), at `folder` of `root` "on the same kind
/// of system", holding `file` (its path inside the folder) with `bytes`.
struct Forged {
    app: &'static str,
    root: RootKind,
    folder: &'static str,
    file: &'static str,
    bytes: &'static [u8],
    /// Where it would be on the new laptop if the old laptop's word were taken (home-relative).
    danger: &'static str,
}

fn forge(system: System, cases: &[Forged]) {
    let t = tempfile::tempdir().unwrap();
    let new = t.path().join("new");
    let staging = t.path().join("staging");
    std::fs::create_dir_all(&staging).unwrap();
    for r in pctwin_special::roots::roots_of(system, &new, &|_| None) {
        std::fs::create_dir_all(&r.path).unwrap();
    }
    let owner = Owner::Person {
        account_id: "1000".into(),
    };
    let mut records = Vec::new();
    for c in cases {
        let component = Component::new(
            owner.clone(),
            c.folder,
            vec![c.file.to_string()],
            Custody::Portable,
            false,
            vec![Evidence::Unreadable {
                why: "a forged record".into(),
            }],
        );
        let id = pctwin_fullmove::data::id_hex(&component);
        records.push((id, c, component));
    }
    let mut sink = Sink::new(
        Choice {
            copy: records.iter().map(|(id, _, _)| id.clone()).collect(),
            secrets: Vec::new(),
        },
        staging.clone(),
    );
    let mut apps = BTreeMap::new();
    for (id, c, component) in records {
        let work = sink.folder(&id);
        let at = c.file.split('/').fold(work.clone(), |p, part| p.join(part));
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
        std::fs::write(&at, c.bytes).unwrap();
        let members = vec![c.file.to_string()];
        let record = Record {
            app: c.app.into(),
            capture: Capture {
                component: component.id,
                method: Method::SteadyCopy,
                preservation: Preservation::Captured,
                generation: generation(&work, &members),
                files: vec![CapturedFile {
                    rel: c.file.into(),
                    len: c.bytes.len() as u64,
                    hash: *blake3::hash(c.bytes).as_bytes(),
                }],
                folded: Vec::new(),
                evidence: Vec::new(),
                closed_unconfirmed: false,
            },
            component,
            placement: PlacementWire::Same {
                root: c.root,
                folder: c.folder.into(),
            },
            root: c.root,
        };
        sink.arrived.insert(
            id.clone(),
            Arrived {
                record: Some(record),
                files: [(c.file.to_string(), true)].into_iter().collect(),
                not_captured: None,
                refused: false,
            },
        );
        apps.insert(id, c.app.to_string());
    }
    let journal = Journal::open_with_record_key(
        &t.path().join("journal.redb"),
        pctwin_journal::keystore::TagKey::from_bytes([0x5a; 32]),
    )
    .unwrap();
    let home = Destination::open(&new).unwrap();
    let landed = land_all(
        &sink,
        &Landing {
            journal: &journal,
            key: &keys().capture,
            home: &home,
            home_path: &new,
            owner_only: &AppOwnerOnly,
            scope: Scope::for_a_move().unwrap(),
            actor: Actor {
                acting_account: "1001".into(),
                for_account: "1001".into(),
                permission: Permission::OwnFolders,
            },
            system,
            from: system,
            fix: None,
        },
        &apps,
        &Default::default(),
    );
    assert_eq!(landed.len(), cases.len(), "{landed:#?}");
    for c in cases {
        let danger = c
            .danger
            .split('/')
            .fold(new.clone(), |p, part| p.join(part));
        assert!(
            !danger.exists(),
            "{system:?}: a forged {:?} landed where the system runs or trusts it: {danger:?}",
            c.folder
        );
        let line = landed.iter().find(|l| l.app == c.app).unwrap();
        // Kept aside (in PCTwin's own folder of things brought over), or not landed: never said
        // copied into the app's place.
        if line.result.status() == ItemStatus::Copied {
            let aside = new.join("PCTwin brought over");
            assert!(
                aside.exists(),
                "{:?} said copied, but not kept aside: {line:#?}",
                c.folder
            );
        }
    }
}

#[test]
fn a_forged_linux_record_never_lands_where_linux_runs_or_trusts_it_at_sign_in() {
    forge(
        System::Linux,
        &[
            Forged {
                app: "autostart",
                root: RootKind::XdgConfig,
                folder: "autostart",
                file: "evil.desktop",
                bytes: b"[Desktop Entry]\nType=Application\nExec=sh -c evil\n",
                danger: ".config/autostart/evil.desktop",
            },
            Forged {
                app: "environment.d",
                root: RootKind::XdgConfig,
                folder: "environment.d",
                file: "50-evil.conf",
                bytes: b"LD_PRELOAD=/tmp/evil.so\n",
                danger: ".config/environment.d/50-evil.conf",
            },
            Forged {
                app: "systemd",
                root: RootKind::XdgConfig,
                folder: "systemd",
                file: "user/evil.service",
                bytes: b"[Service]\nExecStart=/bin/sh -c evil\n",
                danger: ".config/systemd/user/evil.service",
            },
            Forged {
                app: ".ssh",
                root: RootKind::HomeDot,
                folder: ".ssh",
                file: "authorized_keys",
                bytes: b"ssh-ed25519 AAAAforged attacker\n",
                danger: ".ssh/authorized_keys",
            },
        ],
    );
}

#[test]
fn a_forged_windows_record_never_lands_in_the_start_menus_startup_folder() {
    forge(
        System::Windows,
        &[Forged {
            app: "Microsoft/Windows",
            root: RootKind::Roaming,
            folder: "Microsoft/Windows",
            file: "Start Menu/Programs/Startup/evil.cmd",
            bytes: b"@start evil\r\n",
            danger: "AppData/Roaming/Microsoft/Windows/Start Menu/Programs/Startup/evil.cmd",
        }],
    );
}

/// A folder that is not the app's own (here LabMail's component claiming the fish shell's folder,
/// which runs at every shell start) is kept aside, never landed there.
#[test]
fn a_folder_that_is_not_the_apps_own_is_kept_aside() {
    forge(
        System::Linux,
        &[Forged {
            app: "LabMail",
            root: RootKind::XdgConfig,
            folder: "fish",
            file: "conf.d/evil.fish",
            bytes: b"evil\n",
            danger: ".config/fish/conf.d/evil.fish",
        }],
    );
}
