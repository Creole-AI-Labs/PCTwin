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

/// Attempts in the race below. The flaw it guards against (the old layout: the folder belonged
/// to the mover and the socket's mode was changed by name after it was checked) was hit within
/// about 1,700 attempts on Linux; this runs many times that.
const RACE_ATTEMPTS: u32 = 20_000;

/// Mode, owner and group of a path, not following a link.
fn stamp(p: &Path) -> Option<(u32, u32, u32)> {
    std::fs::symlink_metadata(p)
        .ok()
        .map(|m| (m.mode(), m.uid(), m.gid()))
}

/// Empties `arena` of whatever the racer or an attempt left, but the racer's stop sign.
fn clear(arena: &Path) {
    for e in std::fs::read_dir(arena).unwrap().flatten() {
        if e.file_name() == "stop" {
            continue;
        }
        let p = e.path();
        match std::fs::symlink_metadata(&p) {
            Ok(m) if m.is_dir() => {
                let _ = std::fs::remove_dir_all(&p);
            }
            Ok(_) => {
                let _ = std::fs::remove_file(&p);
            }
            Err(_) => {}
        }
    }
}

/// Stops the racer and removes the test's folders however the test ends.
struct RaceGuard {
    racer: Option<std::process::Child>,
    dirs: Vec<PathBuf>,
    arena: PathBuf,
}

impl Drop for RaceGuard {
    fn drop(&mut self) {
        let _ = std::fs::write(self.arena.join("stop"), b"");
        if let Some(racer) = &mut self.racer {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            while racer.try_wait().ok().flatten().is_none() {
                if std::time::Instant::now() > deadline {
                    let _ = racer.kill();
                    let _ = racer.wait();
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        }
        for d in &self.dirs {
            let _ = std::fs::remove_dir_all(d);
        }
    }
}

/// The helper (root) makes its folder and binds its socket again and again while another
/// account (`nobody`, standing in for the mover the socket is handed to) races to swap the
/// socket entry and the folder: a link to a root-owned file, a hard link to it, the folder
/// renamed away and replaced by a link to a root-owned folder or by a folder of its own. Nothing
/// outside the folder may change mode or owner, nothing may appear beside the root-owned file,
/// and every bind either succeeds on the real folder (its socket answers through its path) or
/// refuses.
///
/// Runs as root: CI's two-accounts job is not root, so the test runs itself again through
/// password-free sudo.
#[test]
#[ignore = "needs PCTWIN_HELPER_OS_ADMIN_TESTS=1 and password-free sudo (CI)"]
fn a_racing_account_never_changes_anything_outside_the_folder() {
    assert!(
        std::env::var("PCTWIN_HELPER_OS_ADMIN_TESTS").as_deref() == Ok("1"),
        "set PCTWIN_HELPER_OS_ADMIN_TESTS=1 (CI only)"
    );
    let exe = std::env::current_exe().unwrap();
    if my_uid() != 0 {
        let st = Command::new("sudo")
            .args(["-n", "env", "PCTWIN_HELPER_OS_ADMIN_TESTS=1"])
            .arg(&exe)
            .args([
                "--exact",
                "a_racing_account_never_changes_anything_outside_the_folder",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .status()
            .unwrap();
        assert!(st.success(), "the race, run as root, failed: {st}");
        return;
    }
    let id = Command::new("id").args(["-u", "nobody"]).output().unwrap();
    // macOS prints nobody's ID as -2.
    let nobody = String::from_utf8(id.stdout)
        .unwrap()
        .trim()
        .parse::<i64>()
        .unwrap() as u32;
    assert_ne!(nobody, 0);

    // /tmp itself is a link on macOS; the folders' parent must be the real folder.
    let tmp = std::fs::canonicalize("/tmp").unwrap();
    let tag = socket_name().unwrap()[2..10].to_owned();
    let outside = tmp.join(format!("pctwin-race-out-{tag}"));
    let arena = tmp.join(format!("pctwin-race-{tag}"));
    let bin = tmp.join(format!("pctwin-race-bin-{tag}"));
    let set_mode = |p: &Path, m: u32| {
        std::fs::set_permissions(p, std::fs::Permissions::from_mode(m)).unwrap();
    };
    let mut guard = RaceGuard {
        racer: None,
        dirs: vec![outside.clone(), arena.clone(), bin.clone()],
        arena: arena.clone(),
    };
    for d in [&outside, &arena, &bin] {
        std::fs::create_dir(d).unwrap();
    }
    set_mode(&outside, 0o755);
    set_mode(&bin, 0o755);
    // Sticky and open to all, as /tmp is: the racer may add its own things beside the folder
    // but may rename or remove only its own.
    set_mode(&arena, 0o1777);
    let victim = outside.join("victim");
    std::fs::write(&victim, b"root's").unwrap();
    set_mode(&victim, 0o644);
    // The racer is this test program, copied where `nobody` may run it.
    let racer_exe = bin.join("racer");
    std::fs::copy(&exe, &racer_exe).unwrap();
    set_mode(&racer_exe, 0o755);

    guard.racer = Some(
        Command::new("sudo")
            .args(["-n", "-u", "nobody", "env"])
            .arg(format!(
                "PCTWIN_HELPER_OS_RACE={}\n{}",
                arena.display(),
                victim.display()
            ))
            .arg(&racer_exe)
            .args(["--exact", "racer", "--nocapture", "--test-threads=1"])
            .stdout(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while !arena.join("ready").exists() {
        let racer = guard.racer.as_mut().unwrap();
        assert!(
            racer.try_wait().unwrap().is_none(),
            "the racer stopped before it began"
        );
        assert!(
            std::time::Instant::now() < deadline,
            "the racer never began"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    let watched = [
        tmp.clone(),
        outside.clone(),
        victim.clone(),
        arena.clone(),
        bin.clone(),
        racer_exe.clone(),
    ];
    let before: Vec<_> = watched.iter().map(|p| stamp(p)).collect();
    let folder = arena.join("d");
    let (mut bound, mut refused) = (0u32, 0u32);
    let mut lost: Option<u32> = None;
    for attempt in 1..=RACE_ATTEMPTS {
        clear(&arena);
        let r = ChannelDir::create(&folder)
            .and_then(|d| SocketServer::bind(&d, "s.sock", nobody).map(|s| (d, s)));
        match &r {
            Ok((_, server)) => {
                // Succeeded on the real folder: its socket answers through its path, served
                // by this process. Counted, not stopped at, so a change outside the folder
                // (checked next, and stopped at) is still found and its attempt reported.
                if connect(server.path(), 0).is_err() {
                    lost.get_or_insert(attempt);
                }
                bound += 1;
            }
            Err(_) => refused += 1,
        }
        drop(r);
        for (p, was) in watched.iter().zip(&before) {
            assert_eq!(
                &stamp(p),
                was,
                "attempt {attempt}: {} changed (mode, owner, group)",
                p.display()
            );
        }
        let beside: Vec<_> = std::fs::read_dir(&outside)
            .unwrap()
            .flatten()
            .map(|e| e.file_name())
            .collect();
        assert_eq!(
            beside,
            ["victim"],
            "attempt {attempt}: something was made outside the folder"
        );
    }
    std::fs::write(arena.join("stop"), b"").unwrap();
    let out = guard.racer.take().unwrap().wait_with_output().unwrap();
    let report = String::from_utf8_lossy(&out.stdout);
    let line = report
        .lines()
        .find_map(|l| l.find("racer tries=").map(|i| &l[i..]))
        .unwrap_or_else(|| panic!("no report from the racer: {report}"));
    println!("{RACE_ATTEMPTS} attempts: {bound} bound, {refused} refused; {line}");
    let tries: u64 = line
        .split_whitespace()
        .find_map(|w| w.strip_prefix("tries="))
        .unwrap()
        .parse()
        .unwrap();
    assert!(
        tries >= u64::from(RACE_ATTEMPTS),
        "the racer barely ran: {line}"
    );
    assert_eq!(
        lost, None,
        "from this attempt on, a bind succeeded but its path did not reach it"
    );
    assert!(
        bound >= RACE_ATTEMPTS / 2,
        "too few binds to prove anything"
    );
}

/// Run as `nobody` by the race above; does nothing otherwise. Loops as fast as it can until told
/// to stop (ten minutes at most) and reports how often it tried and how often a swap took.
#[test]
fn racer() {
    let Some(spec) = std::env::var_os("PCTWIN_HELPER_OS_RACE") else {
        return;
    };
    let spec = spec.into_string().unwrap();
    let (arena, victim) = spec.split_once('\n').unwrap();
    let (arena, victim) = (PathBuf::from(arena), PathBuf::from(victim));
    let outside = victim.parent().unwrap().to_path_buf();
    let folder = arena.join("d");
    let sock = folder.join("s.sock");
    let stop = arena.join("stop");
    std::fs::write(arena.join("ready"), b"").unwrap();
    let end = std::time::Instant::now() + std::time::Duration::from_secs(600);
    let (mut tries, mut hits) = (0u64, 0u64);
    loop {
        tries += 1;
        if tries % 256 == 0 && (stop.exists() || std::time::Instant::now() > end) {
            break;
        }
        let took = match tries % 8 {
            // The socket entry swapped for a link to the root-owned file.
            0..=4 => {
                let _ = std::fs::remove_file(&sock);
                std::os::unix::fs::symlink(&victim, &sock).is_ok()
            }
            // ... or for a hard link to it.
            5 => {
                let _ = std::fs::remove_file(&sock);
                std::fs::hard_link(&victim, &sock).is_ok()
            }
            // The folder renamed away and replaced by a link to a root-owned folder.
            6 => {
                std::fs::rename(&folder, arena.join(format!("r{tries}"))).is_ok()
                    && std::os::unix::fs::symlink(&outside, &folder).is_ok()
            }
            // ... or by a folder of its own holding a link to the root-owned file.
            _ => {
                std::fs::rename(&folder, arena.join(format!("r{tries}"))).is_ok()
                    && std::fs::create_dir(&folder).is_ok()
                    && std::os::unix::fs::symlink(&victim, &sock).is_ok()
            }
        };
        hits += u64::from(took);
    }
    println!("racer tries={tries} hits={hits}");
}
