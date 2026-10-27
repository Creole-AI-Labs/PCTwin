//! One writer per settings domain (T24-017).

use pctwin_settings::SettingId;
use pctwin_settings::ownership::{App, Domain, DomainLocks, LockError, Writer, writer_of};

fn vs(p: &str) -> Domain {
    Domain::AppFile(App::VsCode, p.into())
}

fn jb(p: &str) -> Domain {
    Domain::AppFile(App::JetBrains, p.into())
}

#[test]
fn each_domain_has_exactly_the_agreed_writer() {
    assert_eq!(
        writer_of(&Domain::System(SettingId::Theme)),
        Writer::Settings
    );
    assert_eq!(writer_of(&vs("User/settings.json")), Writer::Settings);
    assert_eq!(writer_of(&vs("User/keybindings.json")), Writer::Settings);
    assert_eq!(writer_of(&vs("User/tasks.json")), Writer::Settings);
    assert_eq!(writer_of(&vs("User/snippets/rust.json")), Writer::Settings);
    // Recent files and extension sign-ins share this database: 2.5 writes it.
    assert_eq!(
        writer_of(&vs("User/globalStorage/state.vscdb")),
        Writer::Data
    );
    assert_eq!(
        writer_of(&vs("User/workspaceStorage/abc/state.vscdb")),
        Writer::Data
    );
    assert_eq!(
        writer_of(&vs("extensions/ms-python.python-2026.1/package.json")),
        Writer::Install
    );
    assert_eq!(writer_of(&vs("Cache/data_0")), Writer::Nobody);
    assert_eq!(writer_of(&vs("machineid")), Writer::Nobody);
    assert_eq!(writer_of(&jb("options/editor.xml")), Writer::Settings);
    assert_eq!(
        writer_of(&jb("options/recentProjects.xml")),
        Writer::Settings
    );
    assert_eq!(writer_of(&jb("keymaps/Mine.xml")), Writer::Settings);
    assert_eq!(writer_of(&jb("tools/External Tools.xml")), Writer::Settings);
    assert_eq!(writer_of(&jb("options/security.xml")), Writer::Nobody);
    assert_eq!(writer_of(&jb("options/trusted-paths.xml")), Writer::Nobody);
    assert_eq!(writer_of(&jb("options/window.state.xml")), Writer::Nobody);
    assert_eq!(writer_of(&jb("options/Window.State.xml")), Writer::Nobody);
    assert_eq!(writer_of(&jb("idea.key")), Writer::Nobody);
    assert_eq!(writer_of(&jb("options/x.key")), Writer::Nobody);
    assert_eq!(writer_of(&jb("options/")), Writer::Nobody);
    assert_eq!(writer_of(&jb("options/../idea.key")), Writer::Nobody);
    assert_eq!(writer_of(&jb("plugins/foo/lib/foo.jar")), Writer::Install);
    assert_eq!(writer_of(&jb("workspace/123.xml")), Writer::Data);
    assert_eq!(writer_of(&jb("port")), Writer::Nobody);
}

#[test]
fn t24_017_a_role_that_is_not_the_writer_cannot_take_the_lock() {
    let locks = DomainLocks::default();
    assert_eq!(
        locks
            .acquire(vs("User/settings.json"), Writer::Data)
            .unwrap_err(),
        LockError::NotTheWriter
    );
    assert_eq!(
        locks
            .acquire(vs("User/globalStorage/state.vscdb"), Writer::Settings)
            .unwrap_err(),
        LockError::NotTheWriter
    );
    assert_eq!(
        locks
            .acquire(vs("Cache/data_0"), Writer::Nobody)
            .unwrap_err(),
        LockError::NotTheWriter
    );
}

#[test]
fn t24_017_two_writers_never_hold_one_domain_and_a_lock_is_given_back() {
    let locks = DomainLocks::default();
    let a = locks
        .acquire(vs("User/settings.json"), Writer::Settings)
        .unwrap();
    assert_eq!(
        locks
            .clone()
            .acquire(vs("User/settings.json"), Writer::Settings)
            .unwrap_err(),
        LockError::Busy
    );
    // Different domains at once: 2.4 the settings file, 2.5 the database beside it.
    let b = locks
        .acquire(vs("User/globalStorage/state.vscdb"), Writer::Data)
        .unwrap();
    drop(a);
    let again = locks.acquire(vs("User/settings.json"), Writer::Settings);
    assert!(again.is_ok());
    drop(b);
}
