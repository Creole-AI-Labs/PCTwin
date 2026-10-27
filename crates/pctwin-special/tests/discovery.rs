//! Discovery first, by content alone (ADOPTION change 8; the user, 27 October 2026: "for special
//! categories no hard coding"). With ZERO catalogue rows, synthetic app data of the kinds the plan
//! names as examples (a profile-based mail program, an NSS-based browser, a Chromium-based
//! browser, a password manager, SSH keys, game saves, a local model library) is found, split into
//! components and classified correctly from the files' bytes and how they relate. A second test
//! shows a new app gains a name, an export route and a cross-system place from one signed
//! catalogue row, with no code change.

mod common;

use common::*;
use pctwin_catalogue::Catalogue;
use pctwin_record::Owner;
use pctwin_special::catalogue::{NativeRoute, SHIPPED, SpecialRow};
use pctwin_special::classify::{FolderReport, Placement, Skip, classify_all, placement};
use pctwin_special::component::{Component, Custody, Evidence};
use pctwin_special::roots::{RootKind, System, find_folders, roots_of};

fn me() -> Owner {
    Owner::Person {
        account_id: "S-1-5-21-0-0-0-1001".into(),
    }
}

fn discover(home: &std::path::Path, cat: &Catalogue<SpecialRow>) -> Vec<FolderReport> {
    let roots = roots_of(System::Windows, home, &|_| None);
    let (folders, cov) = find_folders(&me(), home, &roots, &[]);
    assert!(cov.unreadable.is_empty(), "{:?}", cov.unreadable);
    classify_all(folders, System::Windows, cat)
}

fn folder<'a>(r: &'a [FolderReport], rel: &str) -> &'a FolderReport {
    r.iter()
        .find(|f| f.folder.rel == rel)
        .unwrap_or_else(|| panic!("folder {rel} not found"))
}

fn holding<'a>(f: &'a FolderReport, member: &str) -> &'a Component {
    f.components
        .iter()
        .find(|c| c.members.iter().any(|m| m == member))
        .unwrap_or_else(|| panic!("{member} is in no component of {}", f.folder.rel))
}

fn skipped(f: &FolderReport, rel: &str) -> Skip {
    f.not_moved
        .iter()
        .find(|n| n.rel == rel)
        .unwrap_or_else(|| panic!("{rel} not reported"))
        .why
        .clone()
}

/// Every file found is in exactly one component or reported not moved: nothing silent.
fn accounted(home: &std::path::Path, f: &FolderReport) {
    let base = f.folder.path();
    let mut all = Vec::new();
    fn files(dir: &std::path::Path, rel: &str, out: &mut Vec<String>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let e = e.unwrap();
            let name = e.file_name().into_string().unwrap();
            let child = if rel.is_empty() {
                name
            } else {
                format!("{rel}/{name}")
            };
            if e.file_type().unwrap().is_dir() {
                files(&e.path(), &child, out);
            } else {
                out.push(child);
            }
        }
    }
    files(&base, "", &mut all);
    for file in all {
        let in_components = f
            .components
            .iter()
            .filter(|c| c.members.contains(&file))
            .count();
        let reported = f
            .not_moved
            .iter()
            .any(|n| file == n.rel || file.starts_with(&format!("{}/", n.rel)));
        let in_held_folder = f
            .components
            .iter()
            .any(|c| c.members.iter().any(|m| file.starts_with(&format!("{m}/"))));
        assert!(
            in_components + usize::from(reported) + usize::from(in_held_folder) == 1,
            "{file} in {} is accounted for {in_components} + {reported} + {in_held_folder} times ({})",
            f.folder.rel,
            home.display()
        );
    }
}

#[test]
fn with_no_catalogue_rows_everything_is_classified_by_its_content() {
    let t = temp();
    let home = home_of(&t);
    let roaming = home.join("AppData/Roaming");
    mail_profile(&roaming.join("Synthetic Mail"));
    nss_browser_profile(&roaming.join("Fox Like/Browser"));
    chromium_profile(&home.join("AppData/Local/Chromey/User Data"));
    password_vault(&roaming.join("Vaulty"));
    ssh_folder(&home.join(".ssh"));
    game_saves(&home.join("Saved Games/Synthetic Game"));
    model_library(&home.join(".modelrunner"), false);

    let none = Catalogue::<SpecialRow>::shipped(SHIPPED);
    assert!(none.rows().is_empty());
    let r = discover(&home, &none);
    for f in &r {
        assert!(f.row.is_none(), "no row knows anything");
        accounted(&home, f);
    }

    // The mail program: local folders and the address book are portable; the saved passwords
    // travel only with the key store that opens them, as one secret group.
    let mail = folder(&r, "Synthetic Mail");
    let inbox = holding(mail, "Profiles/abc.default/Mail/Local Folders/Inbox");
    assert_eq!((inbox.custody, inbox.secret), (Custody::Portable, false));
    assert!(inbox.evidence.contains(&Evidence::MailStore));
    let abook = holding(mail, "Profiles/abc.default/abook.sqlite");
    assert_eq!(abook.custody, Custody::Portable);
    assert!(abook.evidence.contains(&Evidence::SqliteDatabase));
    let keys = holding(mail, "Profiles/abc.default/key4.db");
    assert_eq!((keys.custody, keys.secret), (Custody::Portable, true));
    assert!(
        keys.members
            .contains(&"Profiles/abc.default/logins.json".to_string())
    );
    assert!(keys.evidence.contains(&Evidence::EncryptedWithKeyDatabase));
    // The profile list and preferences are plain data.
    assert_eq!(holding(mail, "profiles.ini").custody, Custody::Portable);
    assert_eq!(skipped(mail, "Profiles/abc.default/cache2"), Skip::Cache);

    // The NSS-based browser: history portable; cookies machine-bound (never copied); passwords
    // with their key store; an account to sign in to again.
    let fox = folder(&r, "Fox Like");
    assert_eq!(
        holding(fox, "Browser/Profiles/xyz.default/places.sqlite").custody,
        Custody::Portable
    );
    let cookies = holding(fox, "Browser/Profiles/xyz.default/cookies.sqlite");
    assert_eq!(cookies.custody, Custody::MachineBound);
    assert!(cookies.evidence.contains(&Evidence::SessionStore));
    assert!(fox.components.iter().any(
        |c| c.custody == Custody::ServiceHeld && c.evidence.contains(&Evidence::ServiceAccount)
    ));
    assert_eq!(
        holding(fox, "Browser/Profiles/xyz.default/logins.json").custody,
        Custody::Portable
    );

    // T25-001: the Chromium-based browser holds all four classes, each decided on its own.
    let chromey = folder(&r, "Chromey");
    let class = |m: &str| holding(chromey, m).custody;
    assert_eq!(class("User Data/Default/Bookmarks"), Custody::Portable);
    assert_eq!(class("User Data/Default/History"), Custody::Portable);
    assert_eq!(class("User Data/Default/Cookies"), Custody::MachineBound);
    assert_eq!(class("User Data/Default/Web Data"), Custody::MachineBound);
    assert_eq!(class("User Data/Local State"), Custody::MachineBound);
    assert!(
        holding(chromey, "User Data/Local State")
            .evidence
            .contains(&Evidence::MachineBoundProtection)
    );
    let logins = holding(chromey, "User Data/Default/Login Data");
    assert_eq!(
        (logins.custody, logins.secret),
        (Custody::NeedsExport, true)
    );
    assert_eq!(
        class("User Data/Default/Local Storage"),
        Custody::MachineBound
    );
    assert!(
        chromey
            .components
            .iter()
            .any(|c| c.custody == Custody::ServiceHeld)
    );
    assert_eq!(skipped(chromey, "User Data/Default/Cache"), Skip::Cache);
    assert_eq!(
        skipped(chromey, "User Data/Default/Code Cache"),
        Skip::Cache
    );
    assert_eq!(
        skipped(chromey, "User Data/Default/Extensions/abc/1.0"),
        Skip::AddOn
    );
    assert_eq!(
        skipped(chromey, "User Data/Default/Tools/helper.exe"),
        Skip::Program
    );

    // The password manager: the database and its key file are separate secret components, so
    // each is chosen on its own (T25-028: never packaged together automatically).
    let vault = folder(&r, "Vaulty");
    let db = holding(vault, "Vault.kdbx");
    let keyfile = holding(vault, "unlock.keyx");
    assert_eq!((db.custody, db.secret), (Custody::Portable, true));
    assert_eq!((keyfile.custody, keyfile.secret), (Custody::Portable, true));
    assert_ne!(db.id, keyfile.id);
    assert!(!db.members.contains(&"unlock.keyx".to_string()));
    assert!(!holding(vault, "Settings.xml").secret);

    // SSH: private keys are secrets the person chooses; a security-key handle is machine-bound
    // (T25-029); public keys and known hosts are plain.
    let ssh = folder(&r, ".ssh");
    assert_eq!(ssh.folder.root, RootKind::HomeDot);
    let plain = holding(ssh, "id_ed25519");
    assert_eq!((plain.custody, plain.secret), (Custody::Portable, true));
    assert!(plain.evidence.contains(&Evidence::UnprotectedPrivateKey));
    assert!(
        holding(ssh, "work_key")
            .evidence
            .contains(&Evidence::EncryptedPrivateKey)
    );
    let sk = holding(ssh, "id_ed25519_sk");
    assert_eq!(sk.custody, Custody::MachineBound);
    assert!(sk.evidence.contains(&Evidence::HardwareKeyStub));
    for public in ["id_ed25519.pub", "known_hosts", "config"] {
        let c = holding(ssh, public);
        assert_eq!(
            (c.custody, c.secret),
            (Custody::Portable, false),
            "{public}"
        );
    }

    // Game saves: plain portable files in the system's own saved-games place.
    let game = folder(&r, "Synthetic Game");
    assert_eq!(game.folder.root, RootKind::SavedGames);
    assert_eq!(
        holding(game, "Profile 1/slot1.sav").custody,
        Custody::Portable
    );

    // The model library: blobs named by their hash and the manifest listing them, one component.
    let models = folder(&r, ".modelrunner");
    let lib = holding(models, "models/manifests/registry.example/library/m/latest");
    assert_eq!(lib.members.len(), 3);
    assert!(lib.evidence.contains(&Evidence::ModelData));
    assert!(
        !lib.evidence
            .iter()
            .any(|e| matches!(e, Evidence::MissingPart { .. }))
    );
}

#[test]
fn a_model_with_a_missing_part_is_never_complete() {
    // T25-048: one blob the manifest lists is not there; a split model lacks one part.
    let t = temp();
    let home = home_of(&t);
    model_library(&home.join(".modelrunner"), true);
    let d = home.join(".gguf-models/big");
    put(&d.join("big-00001-of-00003.gguf"), &gguf_part(0, 3));
    put(&d.join("big-00003-of-00003.gguf"), &gguf_part(2, 3));
    let st = d.join("../st");
    let header = br#"{"t":{"dtype":"F32","shape":[1],"data_offsets":[0,4]}}"#;
    let mut weights = (header.len() as u64).to_le_bytes().to_vec();
    weights.extend_from_slice(header);
    weights.extend_from_slice(&[0; 4]);
    put(&st.join("model-00001-of-00002.safetensors"), &weights);
    put(
        &st.join("model.safetensors.index.json"),
        br#"{"weight_map":{"a":"model-00001-of-00002.safetensors","b":"model-00002-of-00002.safetensors"}}"#,
    );
    let r = discover(&home, &Catalogue::shipped(SHIPPED));
    let missing = |c: &Component| {
        c.evidence.iter().find_map(|e| match e {
            Evidence::MissingPart { what } => Some(what.clone()),
            _ => None,
        })
    };
    let lib = holding(
        folder(&r, ".modelrunner"),
        "models/manifests/registry.example/library/m/latest",
    );
    assert!(missing(lib).unwrap().contains("1 file"));
    let split = holding(folder(&r, ".gguf-models"), "big/big-00001-of-00003.gguf");
    assert_eq!(missing(split).unwrap(), "part 2 of 3");
    let st = holding(
        folder(&r, ".gguf-models"),
        "st/model.safetensors.index.json",
    );
    assert_eq!(missing(st).unwrap(), "model-00002-of-00002.safetensors");
}

#[test]
fn names_never_decide_bytes_do() {
    // Files named like the examples but holding something else are what their bytes say.
    let t = temp();
    let home = home_of(&t);
    let d = home.join("AppData/Roaming/Mimic");
    put(
        &d.join("key4.db"),
        b"just text pretending to be a key store\n",
    );
    put(&d.join("Cookies"), b"[recipes]\nflour=2\n");
    put(&d.join("id_rsa"), b"not a key at all\n");
    put(&d.join("notes.kdbx"), b"plain notes\n");
    // And files with no telling name are found by their bytes.
    put(&d.join("a"), &openssh_key("none", "ssh-ed25519"));
    sqlite(
        &d.join("b"),
        "CREATE TABLE t(host TEXT, name TEXT, value TEXT, path TEXT, expiry INTEGER);",
    );
    let r = discover(&home, &Catalogue::shipped(SHIPPED));
    let m = folder(&r, "Mimic");
    for plain in ["key4.db", "Cookies", "id_rsa", "notes.kdbx"] {
        let c = holding(m, plain);
        assert_eq!((c.custody, c.secret), (Custody::Portable, false), "{plain}");
    }
    assert!(holding(m, "a").secret);
    assert_eq!(holding(m, "b").custody, Custody::MachineBound);
}

#[test]
fn a_new_app_works_by_adding_one_catalogue_row() {
    let t = temp();
    let home = home_of(&t);
    let d = home.join("AppData/Roaming/Synthetic Journal");
    sqlite(
        &d.join("journal.db"),
        "CREATE TABLE entries(id INTEGER, body TEXT);",
    );
    put(&d.join("tokens/session.json"), br#"{"theme":"dark"}"#);

    // Before the row: found, classified, not known; between systems it is only preserved.
    let none = Catalogue::<SpecialRow>::shipped(SHIPPED);
    let before = discover(&home, &none);
    let f = folder(&before, "Synthetic Journal");
    assert!(f.row.is_none());
    assert_eq!(f.name(), "Synthetic Journal");
    assert_eq!(holding(f, "tokens/session.json").custody, Custody::Portable);
    assert_eq!(
        placement(f, System::Windows, System::Mac),
        Placement::PreservedOnly
    );

    // One signed row, delivered as data.
    let text = br#"{"format": 1, "catalogue": "special", "version": 2, "rows": [
        {"system": "windows", "root": "roaming", "folder": "Synthetic Journal",
         "name": "Synthetic Journal", "checked_on": "2026-10-27",
         "elsewhere": [{"system": "mac", "root": "application-support", "folder": "Synthetic Journal"}],
         "export": {"kind": "tool", "tool": "journal-cli", "args": [{"kind": "switch", "value": "--export"}, {"kind": "output-file"}]},
         "exceptions": [{"members": "tokens/*", "custody": "D", "why": "the app keeps sign-in tokens here"}]}
    ]}"#;
    let sig = pctwin_catalogue::test_key::sign(text, "special", 2);
    let marks = pctwin_catalogue::MemoryMarks::new();
    let one =
        Catalogue::<SpecialRow>::signed(text, &sig, &pctwin_catalogue::test_key::keys(), &marks)
            .unwrap();
    assert_eq!(one.rows().len(), 1);
    let after = discover(&home, &one);
    let f = folder(&after, "Synthetic Journal");
    assert_eq!(f.name(), "Synthetic Journal");
    assert!(matches!(
        f.row.as_ref().unwrap().export,
        Some(NativeRoute::Tool { .. })
    ));
    // The exception makes handling stricter, with its reason.
    let tokens = holding(f, "tokens/session.json");
    assert_eq!(tokens.custody, Custody::MachineBound);
    assert!(
        tokens
            .evidence
            .iter()
            .any(|e| matches!(e, Evidence::Catalogue { row } if row.contains("sign-in tokens")))
    );
    // The database is still classified by its content, and now has a place on the Mac.
    assert_eq!(holding(f, "journal.db").custody, Custody::Portable);
    assert_eq!(
        placement(f, System::Windows, System::Mac),
        Placement::Translated {
            root: RootKind::ApplicationSupport,
            folder: "Synthetic Journal".into()
        }
    );
    // Same system: the same place, row or not.
    assert_eq!(
        placement(f, System::Windows, System::Windows),
        Placement::Same {
            root: RootKind::Roaming,
            folder: "Synthetic Journal".into()
        }
    );
}
