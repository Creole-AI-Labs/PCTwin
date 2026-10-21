//! macOS and Linux: the helper's socket sits in a folder only the server's own account can
//! write, only the client's account can connect to it, the client checks who serves it, and
//! every connection says who made it.
#![cfg(unix)]

use pctwin_helper_os::{
    Account, ChannelDir, Error, SocketServer, caller_of, connect, current_account, socket_name,
};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn my_uid() -> u32 {
    match current_account().unwrap() {
        Account::Uid(u) => u,
        _ => panic!("this system names accounts by user ID"),
    }
}

/// A short parent folder (socket paths are limited to about 100 bytes), private to this test.
fn parent() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("pt")
        .tempdir_in("/tmp")
        .unwrap()
}

fn same_file(a: &Path, b: &Path) -> bool {
    std::fs::canonicalize(a).unwrap() == std::fs::canonicalize(b).unwrap()
}

#[test]
fn the_folder_is_made_0711_for_the_server_and_read_back() {
    let p = parent();
    let dir = ChannelDir::create(&p.path().join("d")).unwrap();
    let m = std::fs::symlink_metadata(dir.path()).unwrap();
    assert!(m.is_dir());
    assert_eq!(m.mode() & 0o7777, 0o711, "only the server may write in it");
    assert_eq!(m.uid(), my_uid());
    assert_eq!(dir.owner(), my_uid());
    assert!(ChannelDir::open(dir.path()).is_ok());
}

#[test]
fn a_folder_with_any_other_mode_is_refused() {
    let p = parent();
    for mode in [0o700, 0o755, 0o751, 0o733, 0o777, 0o713] {
        let path = p.path().join(format!("m{mode:o}"));
        std::fs::create_dir(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
        assert!(
            matches!(ChannelDir::open(&path), Err(Error::NotPrivate(_))),
            "{mode:o}"
        );
    }
}

#[test]
fn a_link_to_a_channel_folder_is_refused() {
    let p = parent();
    let real = p.path().join("real");
    ChannelDir::create(&real).unwrap();
    let link = p.path().join("link");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    assert!(matches!(ChannelDir::open(&link), Err(Error::NotPrivate(_))));
}

#[test]
fn a_parent_others_can_write_is_refused_and_left_alone() {
    let p = parent();
    let shared = p.path().join("shared");
    std::fs::create_dir(&shared).unwrap();
    std::fs::set_permissions(&shared, std::fs::Permissions::from_mode(0o777)).unwrap();
    let path = shared.join("d");
    assert!(matches!(
        ChannelDir::create(&path),
        Err(Error::NotPrivate(_))
    ));
    assert_eq!(std::fs::read_dir(&shared).unwrap().count(), 0);
    // Sticky, as /tmp is: others cannot rename the folder away.
    std::fs::set_permissions(&shared, std::fs::Permissions::from_mode(0o1777)).unwrap();
    assert!(ChannelDir::create(&path).is_ok());
}

#[test]
fn a_parent_that_is_a_link_is_refused_and_left_alone() {
    let p = parent();
    let real = p.path().join("real");
    std::fs::create_dir(&real).unwrap();
    let link = p.path().join("link");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    assert!(matches!(
        ChannelDir::create(&link.join("d")),
        Err(Error::NotPrivate(_))
    ));
    assert_eq!(std::fs::read_dir(&real).unwrap().count(), 0);
}

#[test]
fn a_name_already_taken_is_never_made_or_followed() {
    let p = parent();
    let elsewhere = p.path().join("elsewhere");
    std::fs::create_dir(&elsewhere).unwrap();
    let path = p.path().join("d");
    std::os::unix::fs::symlink(&elsewhere, &path).unwrap();
    assert!(ChannelDir::create(&path).is_err());
    let m = std::fs::metadata(&elsewhere).unwrap();
    assert_ne!(
        m.mode() & 0o777,
        0o711,
        "the link's target must not be changed"
    );
}

#[test]
fn a_folder_replaced_after_its_check_is_refused_at_bind() {
    let p = parent();
    let path = p.path().join("d");
    let dir = ChannelDir::create(&path).unwrap();
    std::fs::rename(&path, p.path().join("moved")).unwrap();
    ChannelDir::create(&path).unwrap();
    let r = SocketServer::bind(&dir, &socket_name().unwrap(), my_uid());
    assert!(matches!(r, Err(Error::NotPrivate(_))), "{r:?}");
    assert_eq!(std::fs::read_dir(&path).unwrap().count(), 0);
}

#[test]
fn the_socket_is_0600_for_the_client_and_a_name_is_never_reused() {
    let p = parent();
    let dir = ChannelDir::create(&p.path().join("d")).unwrap();
    let name = socket_name().unwrap();
    let server = SocketServer::bind(&dir, &name, my_uid()).unwrap();
    let m = std::fs::symlink_metadata(server.path()).unwrap();
    assert_eq!(m.mode() & 0o777, 0o600);
    assert_eq!(m.uid(), my_uid());
    assert!(matches!(
        SocketServer::bind(&dir, &name, my_uid()),
        Err(Error::NameTaken)
    ));
    for bad in ["", "../x", "a/b", ".hidden", "x y"] {
        assert!(
            matches!(
                SocketServer::bind(&dir, bad, my_uid()),
                Err(Error::BadInput(_))
            ),
            "{bad}"
        );
    }
}

#[test]
fn the_client_checks_who_serves_the_socket_before_sending() {
    let p = parent();
    let dir = ChannelDir::create(&p.path().join("d")).unwrap();
    let server = SocketServer::bind(&dir, &socket_name().unwrap(), my_uid()).unwrap();
    // Served by this account: accepted when this account is the one expected.
    assert!(connect(server.path(), my_uid()).is_ok());
    // Expecting another account (root, for the helper) refuses this server.
    let other = if my_uid() == 0 { 1 } else { 0 };
    assert!(matches!(
        connect(server.path(), other),
        Err(Error::UnexpectedPeer)
    ));
}

#[test]
fn a_caller_in_this_process_is_this_process() {
    let p = parent();
    let dir = ChannelDir::create(&p.path().join("d")).unwrap();
    let server = SocketServer::bind(&dir, &socket_name().unwrap(), my_uid()).unwrap();
    let mut client = connect(server.path(), my_uid()).unwrap();
    let (mut stream, caller) = server.accept().unwrap();
    assert_eq!(caller.process_id, std::process::id());
    assert_eq!(caller.account, Account::Uid(my_uid()));
    assert!(same_file(
        &caller.program_hint,
        &std::env::current_exe().unwrap()
    ));
    assert_eq!(caller_of(&client).unwrap().process_id, std::process::id());
    client.write_all(b"hi").unwrap();
    let mut b = [0u8; 2];
    stream.read_exact(&mut b).unwrap();
    assert_eq!(&b, b"hi");
}

#[test]
fn a_caller_in_another_process_is_that_process() {
    let p = parent();
    let dir = ChannelDir::create(&p.path().join("d")).unwrap();
    let server = SocketServer::bind(&dir, &socket_name().unwrap(), my_uid()).unwrap();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "child_client", "--nocapture", "--test-threads=1"])
        .env("PCTWIN_HELPER_OS_CHILD_SOCKET", server.path())
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    let (mut stream, caller) = server.accept().unwrap();
    let mut b = [0u8; 1];
    stream.read_exact(&mut b).unwrap();
    stream.write_all(b"k").unwrap();
    assert!(child.wait().unwrap().success());
    assert_eq!(caller.process_id, child.id());
    assert_eq!(caller.account, Account::Uid(my_uid()));
    assert!(same_file(
        &caller.program_hint,
        &std::env::current_exe().unwrap()
    ));
}

/// Run as a child by the test above; does nothing otherwise.
#[test]
fn child_client() {
    let Some(path) = std::env::var_os("PCTWIN_HELPER_OS_CHILD_SOCKET") else {
        return;
    };
    let mut s = UnixStream::connect(PathBuf::from(path)).unwrap();
    s.write_all(b"x").unwrap();
    let mut b = [0u8; 1];
    s.read_exact(&mut b).unwrap();
}

/// A real other account (`nobody`, through sudo as the lab's two-accounts job uses it) can
/// neither connect to the socket (0600, this account's) nor add anything to the folder (0711);
/// the same account can run Python, so the refusals are the permissions' doing.
#[test]
#[ignore = "needs PCTWIN_HELPER_OS_ADMIN_TESTS=1 and password-free sudo (CI)"]
fn another_account_can_neither_connect_nor_write_in_the_folder() {
    assert!(
        std::env::var("PCTWIN_HELPER_OS_ADMIN_TESTS").as_deref() == Ok("1"),
        "set PCTWIN_HELPER_OS_ADMIN_TESTS=1 (CI only)"
    );
    // /tmp itself is a link on macOS; the folder's parent must be the real folder.
    let tmp = std::fs::canonicalize("/tmp").unwrap();
    let path = tmp.join(format!("pctwin-h-{}", &socket_name().unwrap()[2..10]));
    let dir = ChannelDir::create(&path).unwrap();
    let server = SocketServer::bind(&dir, "s.sock", my_uid()).unwrap();
    let as_nobody = |code: &str| {
        Command::new("sudo")
            .args(["-n", "-u", "nobody", "python3", "-c", code])
            .output()
            .unwrap()
    };
    let control = as_nobody("print('ok')");
    assert!(
        control.status.success(),
        "sudo as nobody must work here: {control:?}"
    );
    let connect = format!(
        "import socket,sys\ns=socket.socket(socket.AF_UNIX)\ntry:\n s.connect({:?})\nexcept PermissionError:\n sys.exit(13)\nsys.exit(0)",
        server.path().to_str().unwrap()
    );
    let write = format!(
        "import sys\ntry:\n open({:?}, 'w')\nexcept PermissionError:\n sys.exit(13)\nsys.exit(0)",
        path.join("added").to_str().unwrap()
    );
    let r = as_nobody(&connect);
    let w = as_nobody(&write);
    drop(server);
    let _ = std::fs::remove_dir(&path);
    assert_eq!(
        r.status.code(),
        Some(13),
        "nobody reached the socket: {r:?}"
    );
    assert_eq!(
        w.status.code(),
        Some(13),
        "nobody wrote in the folder: {w:?}"
    );
}
