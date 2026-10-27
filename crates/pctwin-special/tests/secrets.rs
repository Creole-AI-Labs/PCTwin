//! Secrets (plan section 10): chosen one by one, never swept up by default; never in any record
//! PCTwin keeps or shows; a readable password export lives in an owner-only place and is removed
//! only when PCTwin made it and the import was verified.

mod common;

use std::path::Path;

use common::*;
use pctwin_catalogue::Catalogue;
use pctwin_record::Owner;
use pctwin_special::catalogue::{SHIPPED, SpecialRow};
use pctwin_special::classify::{FolderReport, classify_all};
use pctwin_special::component::{Component, Custody};
use pctwin_special::export::{Cleanup, DISCLOSURE, ExportSlot};
use pctwin_special::land::OwnerOnly;
use pctwin_special::roots::{System, find_folders, roots_of};
use pctwin_special::select::{NotChosen, Selection};

const MARK: &str = "SYNTHETICSECRETMARK";

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

fn all(r: &[FolderReport]) -> Vec<&Component> {
    r.iter().flat_map(|f| f.components.iter()).collect()
}

fn with_secrets(home: &Path) {
    ssh_folder(&home.join(".ssh"));
    password_vault(&home.join("AppData/Roaming/Vaulty"));
    key_store(&home.join("AppData/Roaming/Mailer/p"));
    put(
        &home.join("AppData/Roaming/Tool/settings.json"),
        format!(r#"{{"token":"ghp_{MARK}0123456789abcdefABCDEF"}}"#).as_bytes(),
    );
    put(
        &home.join("AppData/Roaming/Tool/.env"),
        format!("API_KEY=sk-{MARK}0123456789abcdefghijk\n").as_bytes(),
    );
    put(
        &home.join(".ssh/id_marked"),
        format!("-----BEGIN PRIVATE KEY-----\n{MARK}AAAA\n-----END PRIVATE KEY-----\n").as_bytes(),
    );
    put(
        &home.join("AppData/Roaming/Plain/notes.json"),
        br#"{"a":1}"#,
    );
}

#[test]
fn secrets_move_only_when_chosen_one_by_one() {
    let t = temp();
    let home = home_of(&t);
    with_secrets(&home);
    let r = found(&home);
    let mut sel = Selection::default_for(&r);
    let comps = all(&r);
    let secrets: Vec<&&Component> = comps.iter().filter(|c| c.secret).collect();
    assert!(
        secrets.len() >= 6,
        "keys, vault, key file, key store are secrets"
    );
    for s in &secrets {
        assert!(!sel.contains(s.id), "no secret is proposed by default");
    }
    for c in comps
        .iter()
        .filter(|c| !c.secret && c.custody == Custody::Portable)
    {
        assert!(sel.contains(c.id));
    }
    for c in comps
        .iter()
        .filter(|c| matches!(c.custody, Custody::MachineBound | Custody::Unresolved))
    {
        assert!(!sel.contains(c.id), "never proposed for copying");
    }
    // One chosen: only that one.
    let key = comps
        .iter()
        .find(|c| c.members == vec!["work_key".to_string()])
        .unwrap();
    sel.choose_secret(&r, key.id).unwrap();
    assert!(sel.contains(key.id));
    assert_eq!(
        secrets.iter().filter(|s| sel.contains(s.id)).count(),
        1,
        "choosing one secret chooses no other"
    );
    let plain = comps.iter().find(|c| !c.secret).unwrap();
    assert_eq!(sel.choose_secret(&r, plain.id), Err(NotChosen::NotSecret));
    assert_eq!(
        sel.choose_secret(&r, pctwin_special::component::ComponentId([0; 32])),
        Err(NotChosen::Unknown)
    );
    sel.leave_out(key.id);
    assert!(!sel.contains(key.id));
}

#[test]
fn an_unopenable_secret_cannot_be_chosen() {
    // T25-055: saved passwords with no key store beside them: kept as found, access unresolved.
    let t = temp();
    let home = home_of(&t);
    put(
        &home.join("AppData/Roaming/Orphan/logins.json"),
        format!(r#"{{"logins":[{{"p":"{}"}}]}}"#, sdr_value()).as_bytes(),
    );
    let r = found(&home);
    let c = all(&r)
        .into_iter()
        .find(|c| c.members == vec!["logins.json".to_string()])
        .unwrap();
    assert_eq!((c.custody, c.secret), (Custody::Unresolved, true));
    let mut sel = Selection::default_for(&r);
    assert_eq!(sel.choose_secret(&r, c.id), Err(NotChosen::NotCopyable));
}

#[test]
fn no_secret_content_reaches_any_record() {
    let t = temp();
    let home = home_of(&t);
    with_secrets(&home);
    let r = found(&home);
    // Everything PCTwin keeps or shows about what it found.
    let shown = format!("{r:?}");
    let words: Vec<String> = all(&r)
        .iter()
        .flat_map(|c| c.evidence.iter().map(|e| e.words()))
        .collect();
    let json = serde_json::to_string(&all(&r)).unwrap();
    for text in [shown, words.join("\n"), json] {
        assert!(
            !text.contains(MARK),
            "a secret's content leaked into a record"
        );
        assert!(!text.contains("AAAA"), "key material leaked into a record");
    }
}

/// The owner-only folder, as the app makes it.
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

#[test]
fn a_readable_password_export_lives_owner_only_and_is_removed_only_when_safe() {
    // T25-022.
    assert!(DISCLOSURE.contains("anyone who opens it can read"));
    let t = temp();
    let base = t.path().join("PCTwin/exports");
    let csv = b"url,username,password\nhttps://example.org,person,invented\n";

    // Imported and verified, PCTwin's own, unchanged: removed, and its folder with it.
    let slot = ExportSlot::prepare(&base, "passwords.csv", &AppOwnerOnly).unwrap();
    assert!(slot.ours());
    let folder = slot.file().parent().unwrap().to_path_buf();
    #[cfg(windows)]
    {
        let acl = pctwin_helper_os::folder_access_list(&folder).unwrap();
        for others in [";;;BU)", ";;;AU)", ";;;WD)", ";;;BA)"] {
            assert!(!acl.contains(others), "{acl}");
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&folder).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }
    std::fs::write(slot.file(), csv).unwrap();
    let sealed = slot.seal().unwrap();
    assert_eq!(
        slot.clone().cleanup(None).unwrap(),
        Cleanup::KeptImportNotVerified
    );
    assert!(slot.file().exists());
    assert_eq!(
        slot.clone().cleanup(Some(sealed)).unwrap(),
        Cleanup::Removed
    );
    assert!(!slot.file().exists() && !folder.exists());
    assert_eq!(slot.cleanup(Some(sealed)).unwrap(), Cleanup::Gone);

    // Changed since it was imported: kept.
    let slot = ExportSlot::prepare(&base, "passwords.csv", &AppOwnerOnly).unwrap();
    std::fs::write(slot.file(), csv).unwrap();
    let sealed = slot.seal().unwrap();
    std::fs::write(slot.file(), b"edited by the person").unwrap();
    assert_eq!(
        slot.clone().cleanup(Some(sealed)).unwrap(),
        Cleanup::KeptChanged
    );
    assert!(slot.file().exists());

    // Saved by the person elsewhere: theirs, never removed.
    let theirs = t.path().join("Documents/my-passwords.csv");
    put(&theirs, csv);
    let adopted = ExportSlot::adopt(theirs.clone());
    let sealed = adopted.seal().unwrap();
    assert_eq!(adopted.cleanup(Some(sealed)).unwrap(), Cleanup::KeptNotOurs);
    assert!(theirs.exists());

    // A name with a folder in it is refused.
    assert!(ExportSlot::prepare(&base, "../x.csv", &AppOwnerOnly).is_err());
}
