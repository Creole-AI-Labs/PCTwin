//! One writer per settings domain, as the catalogue declares it (T24-017).

use pctwin_settings::catalogue::{Catalogue, SettingId, Writer};
use pctwin_settings::ownership::{Domain, DomainLocks, LockError, writer_of};

fn file(app: &str, path: &str) -> Domain {
    Domain::AppFile {
        app: app.into(),
        path: path.into(),
    }
}

#[test]
fn each_domain_has_exactly_its_declared_writer() {
    let c = Catalogue::starter().unwrap();
    let w = |d: Domain| writer_of(&c, &d);
    assert_eq!(
        w(Domain::Setting(SettingId::new("appearance.theme").unwrap())),
        Writer::Settings
    );
    assert_eq!(
        w(Domain::Setting(SettingId::new("not.a-row").unwrap())),
        Writer::Nobody
    );
    assert_eq!(
        w(file("vscode", "Code/User/settings.json")),
        Writer::Settings
    );
    assert_eq!(
        w(file("vscode", "Code/User/snippets/rust.json")),
        Writer::Settings
    );
    assert_eq!(
        w(file("vscode", "Code/User/globalStorage/state.vscdb")),
        Writer::Data
    );
    assert_eq!(
        w(file("vscode", ".vscode/extensions/x/package.json")),
        Writer::Install
    );
    assert_eq!(w(file("vscode", "Code/machineid")), Writer::Nobody);
    assert_eq!(w(file("vscode", "Code/User/../machineid")), Writer::Nobody);
    assert_eq!(
        w(file(
            "jetbrains",
            "JetBrains/IntelliJIdea2025.2/options/editor.xml"
        )),
        Writer::Settings
    );
    assert_eq!(
        w(file(
            "jetbrains",
            "JetBrains/IntelliJIdea2025.2/options/security.xml"
        )),
        Writer::Nobody
    );
    assert_eq!(
        w(file(
            "jetbrains",
            "JetBrains/IntelliJIdea2025.2/options/Trusted-Paths.xml"
        )),
        Writer::Nobody
    );
    assert_eq!(
        w(file(
            "jetbrains",
            "JetBrains/IntelliJIdea2025.2/plugins/x/lib/x.jar"
        )),
        Writer::Install
    );
    assert_eq!(w(file("unknown-app", "x.json")), Writer::Nobody);
}

#[test]
fn t24_017_only_the_writer_takes_a_lock_and_never_two_at_once() {
    let c = Catalogue::starter().unwrap();
    let locks = DomainLocks::default();
    let s = file("vscode", "Code/User/settings.json");
    let db = file("vscode", "Code/User/globalStorage/state.vscdb");
    assert_eq!(
        locks.acquire(&c, s.clone(), Writer::Data).unwrap_err(),
        LockError::NotTheWriter
    );
    assert_eq!(
        locks.acquire(&c, db.clone(), Writer::Settings).unwrap_err(),
        LockError::NotTheWriter
    );
    assert_eq!(
        locks
            .acquire(&c, file("vscode", "Code/machineid"), Writer::Nobody)
            .unwrap_err(),
        LockError::NotTheWriter
    );
    let a = locks.acquire(&c, s.clone(), Writer::Settings).unwrap();
    assert_eq!(
        locks
            .clone()
            .acquire(&c, s.clone(), Writer::Settings)
            .unwrap_err(),
        LockError::Busy
    );
    let b = locks.acquire(&c, db, Writer::Data).unwrap();
    drop(a);
    assert!(locks.acquire(&c, s, Writer::Settings).is_ok());
    drop(b);
}
