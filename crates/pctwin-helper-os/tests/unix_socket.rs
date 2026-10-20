//! macOS and Linux: the helper's socket sits in a folder only the mover's account can enter,
//! and every connection says who made it.
#![cfg(unix)]

use pctwin_helper_os::{
    Account, Error, PrivateDir, SocketServer, caller_of, current_account, socket_name,
};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn my_uid() -> u32 {
    match current_account().unwrap() {
        Account::Uid(u) => u,
        other => panic!("{other:?}"),
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
fn the_folder_is_made_0700_for_its_owner_and_read_back() {
    let p = parent();
    let path = p.path().join("d");
    let dir = PrivateDir::create(&path, my_uid()).unwrap();
    let m = std::fs::symlink_metadata(dir.path()).unwrap();
    assert!(m.is_dir());
    assert_eq!(m.mode() & 0o7777, 0o700);
    assert_eq!(m.uid(), my_uid());
}

#[test]
fn a_folder_others_can_enter_is_refused() {
    let p = parent();
    for mode in [0o755, 0o750, 0o711, 0o701, 0o777] {
        let path = p.path().join(format!("m{mode:o}"));
        std::fs::create_dir(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
        assert!(
            matches!(PrivateDir::open(&path, my_uid()), Err(Error::NotPrivate(_))),
            "{mode:o}"
        );
    }
}

#[test]
fn a_folder_for_another_account_is_refused() {
    let p = parent();
    let path = p.path().join("d");
    PrivateDir::create(&path, my_uid()).unwrap();
    assert!(matches!(
        PrivateDir::open(&path, my_uid() + 1),
        Err(Error::NotPrivate(_))
    ));
}

#[test]
fn a_link_to_a_private_folder_is_refused() {
    let p = parent();
    let real = p.path().join("real");
    PrivateDir::create(&real, my_uid()).unwrap();
    let link = p.path().join("link");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    assert!(matches!(
        PrivateDir::open(&link, my_uid()),
        Err(Error::NotPrivate(_))
    ));
}

#[test]
fn a_parent_others_can_write_is_refused() {
    let p = parent();
    let shared = p.path().join("shared");
    std::fs::create_dir(&shared).unwrap();
    std::fs::set_permissions(&shared, std::fs::Permissions::from_mode(0o777)).unwrap();
    let path = shared.join("d");
    assert!(matches!(
        PrivateDir::create(&path, my_uid()),
        Err(Error::NotPrivate(_))
    ));
    // Sticky, as /tmp is: others cannot rename it away.
    std::fs::set_permissions(&shared, std::fs::Permissions::from_mode(0o1777)).unwrap();
    assert!(PrivateDir::open(&path, my_uid()).is_ok());
}

#[test]
fn the_socket_is_0600_and_a_name_is_never_reused() {
    let p = parent();
    let dir = PrivateDir::create(&p.path().join("d"), my_uid()).unwrap();
    let name = socket_name().unwrap();
    let server = SocketServer::bind(&dir, &name).unwrap();
    let m = std::fs::symlink_metadata(server.path()).unwrap();
    assert_eq!(m.mode() & 0o777, 0o600);
    assert_eq!(m.uid(), my_uid());
    assert!(matches!(
        SocketServer::bind(&dir, &name),
        Err(Error::NameTaken)
    ));
    for bad in ["", "../x", "a/b", ".hidden", "x y"] {
        assert!(
            matches!(SocketServer::bind(&dir, bad), Err(Error::BadInput(_))),
            "{bad}"
        );
    }
}

#[test]
fn a_caller_in_this_process_is_this_process() {
    let p = parent();
    let dir = PrivateDir::create(&p.path().join("d"), my_uid()).unwrap();
    let server = SocketServer::bind(&dir, &socket_name().unwrap()).unwrap();
    let mut client = UnixStream::connect(server.path()).unwrap();
    let (mut stream, caller) = server.accept().unwrap();
    assert_eq!(caller.process_id, std::process::id());
    assert_eq!(caller.account, Account::Uid(my_uid()));
    assert!(same_file(
        &caller.program,
        &std::env::current_exe().unwrap()
    ));
    // And from the client's end: the server is this process too.
    assert_eq!(caller_of(&client).unwrap().process_id, std::process::id());
    client.write_all(b"hi").unwrap();
    let mut b = [0u8; 2];
    stream.read_exact(&mut b).unwrap();
    assert_eq!(&b, b"hi");
}

#[test]
fn a_caller_in_another_process_is_that_process() {
    let p = parent();
    let dir = PrivateDir::create(&p.path().join("d"), my_uid()).unwrap();
    let server = SocketServer::bind(&dir, &socket_name().unwrap()).unwrap();
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
        &caller.program,
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

/// A real other account (`nobody`, through sudo as the lab's two-accounts job uses it) cannot
/// reach the socket; the same account can run Python, so the refusal is the folder's doing.
#[test]
#[ignore = "needs PCTWIN_HELPER_OS_ADMIN_TESTS=1 and password-free sudo (CI)"]
fn another_account_cannot_reach_the_socket() {
    assert!(
        std::env::var("PCTWIN_HELPER_OS_ADMIN_TESTS").as_deref() == Ok("1"),
        "set PCTWIN_HELPER_OS_ADMIN_TESTS=1 (CI only)"
    );
    // The parent is /tmp itself so that `nobody` can reach the private folder's door.
    let path = PathBuf::from(format!("/tmp/pctwin-h-{}", &socket_name().unwrap()[2..10]));
    let dir = PrivateDir::create(&path, my_uid()).unwrap();
    let server = SocketServer::bind(&dir, "s.sock").unwrap();
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
    let r = as_nobody(&connect);
    drop(server);
    let _ = std::fs::remove_dir(&path);
    assert_eq!(
        r.status.code(),
        Some(13),
        "nobody reached the socket: {r:?}"
    );
}
