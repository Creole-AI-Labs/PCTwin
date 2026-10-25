//! The hand-over at a person's own next sign-in, set up for real on CI as an administrator
//! (gated by PCTWIN_HELPER_OS_ADMIN_TESTS=1): the entry is made in the system's own place, the
//! held folder becomes the person's to read, and once the folder is gone the entry is cleared.

#[test]
#[ignore = "needs PCTWIN_HELPER_OS_ADMIN_TESTS=1 (CI); makes a real account and sign-in entry"]
fn a_hand_over_at_sign_in_is_set_up_and_cleared_once_done() {
    assert!(
        std::env::var("PCTWIN_HELPER_OS_ADMIN_TESTS").as_deref() == Ok("1"),
        "set PCTWIN_HELPER_OS_ADMIN_TESTS=1 (CI only)"
    );
    #[cfg(unix)]
    {
        let uid = std::process::Command::new("id").arg("-u").output().unwrap();
        if String::from_utf8_lossy(&uid.stdout).trim() != "0" {
            let st = std::process::Command::new("sudo")
                .args(["-n", "env", "PCTWIN_HELPER_OS_ADMIN_TESTS=1"])
                .arg(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "a_hand_over_at_sign_in_is_set_up_and_cleared_once_done",
                    "--ignored",
                    "--nocapture",
                ])
                .status()
                .unwrap();
            assert!(st.success(), "run as root, it failed: {st}");
            return;
        }
    }
    #[cfg(unix)]
    use pctwin_helper_os::Account;
    use pctwin_helper_os::{add_standard_account, holding, signin};
    let name = format!("pctwinsi{}", std::process::id() % 10_000);
    let person = add_standard_account(&name, "PCTwin Sign-in", "Temporary-Pass-123!").unwrap();
    #[cfg(windows)]
    let mover = pctwin_helper_os::current_account().unwrap();
    #[cfg(unix)]
    let mover = {
        let id = std::process::Command::new("id")
            .args(["-u", "nobody"])
            .output()
            .unwrap();
        Account::Uid(
            String::from_utf8(id.stdout)
                .unwrap()
                .trim()
                .parse::<i64>()
                .unwrap() as u32,
        )
    };
    let area = holding::create_holding_area(&mover).unwrap();
    let folder = holding::person_folder(&area, "p1", &mover, Some(&person)).unwrap();
    std::fs::create_dir_all(folder.join("documents")).unwrap();
    std::fs::write(folder.join("documents/letter.txt"), b"Dear person").unwrap();
    let program = std::env::current_exe().unwrap();
    let check = || {
        signin::schedule(&person, &name, &mover, &folder, &program, "t1").unwrap();
        #[cfg(windows)]
        {
            let q = std::process::Command::new("schtasks")
                .args(["/Query", "/TN", r"PCTwin\pctwin-handover-t1"])
                .output()
                .unwrap();
            assert!(q.status.success(), "the sign-in task was made");
        }
        #[cfg(target_os = "macos")]
        assert!(
            std::path::Path::new("/Library/LaunchAgents/com.creoleailabs.pctwin.handover.t1.plist")
                .exists()
        );
        #[cfg(all(unix, not(target_os = "macos")))]
        assert!(std::path::Path::new("/etc/xdg/autostart/pctwin-handover-t1.desktop").exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let Account::Uid(uid) = person else {
                unreachable!()
            };
            assert_eq!(std::fs::metadata(&folder).unwrap().uid(), uid);
            assert_eq!(
                std::fs::metadata(folder.join("documents/letter.txt"))
                    .unwrap()
                    .uid(),
                uid
            );
        }
        // Handed over (the folder gone): the entry is cleared.
        std::fs::remove_dir_all(&folder).unwrap();
        signin::clean(&holding::staging_root().unwrap());
        #[cfg(windows)]
        {
            let q = std::process::Command::new("schtasks")
                .args(["/Query", "/TN", r"PCTwin\pctwin-handover-t1"])
                .output()
                .unwrap();
            assert!(!q.status.success(), "the sign-in task was cleared");
        }
        #[cfg(target_os = "macos")]
        assert!(
            !std::path::Path::new(
                "/Library/LaunchAgents/com.creoleailabs.pctwin.handover.t1.plist"
            )
            .exists()
        );
        #[cfg(all(unix, not(target_os = "macos")))]
        assert!(!std::path::Path::new("/etc/xdg/autostart/pctwin-handover-t1.desktop").exists());
    };
    let result = std::panic::catch_unwind(check);
    let _ = std::fs::remove_dir_all(&area);
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("schtasks")
            .args(["/Delete", "/TN", r"PCTwin\pctwin-handover-t1", "/F"])
            .status();
        let _ = std::process::Command::new("net")
            .args(["user", &name, "/delete"])
            .status();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::fs::remove_file("/etc/xdg/autostart/pctwin-handover-t1.desktop");
        let _ = std::process::Command::new("userdel")
            .args(["--remove", &name])
            .status();
    }
    #[cfg(target_os = "macos")]
    {
        let _ =
            std::fs::remove_file("/Library/LaunchAgents/com.creoleailabs.pctwin.handover.t1.plist");
        let _ = std::process::Command::new("sysadminctl")
            .args(["-deleteUser", &name])
            .status();
    }
    result.unwrap();
}
