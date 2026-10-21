//! The holding area on the new laptop: made fresh inside a folder only administrators can make
//! things in, for the mover's account alone, never with the right to change who may. Made for
//! real only on CI, as an administrator (gated by PCTWIN_HELPER_OS_ADMIN_TESTS=1), and removed.

#[test]
#[ignore = "needs PCTWIN_HELPER_OS_ADMIN_TESTS=1 (CI); makes a real holding area"]
fn a_holding_area_is_the_mover_s_alone_and_inside_an_administrators_folder() {
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
                    "a_holding_area_is_the_mover_s_alone_and_inside_an_administrators_folder",
                    "--ignored",
                    "--nocapture",
                ])
                .status()
                .unwrap();
            assert!(st.success(), "run as root, it failed: {st}");
            return;
        }
    }
    #[cfg(windows)]
    let mover = pctwin_helper_os::current_account().unwrap();
    #[cfg(unix)]
    let mover = {
        let id = std::process::Command::new("id")
            .args(["-u", "nobody"])
            .output()
            .unwrap();
        pctwin_helper_os::Account::Uid(
            String::from_utf8(id.stdout)
                .unwrap()
                .trim()
                .parse::<i64>()
                .unwrap() as u32,
        )
    };
    let area = pctwin_helper_os::holding::create_holding_area(&mover).unwrap();
    let other_area = pctwin_helper_os::holding::create_holding_area(&mover).unwrap();
    assert_ne!(area, other_area, "a fresh one each move");
    let first = pctwin_helper_os::holding::person_folder(&area, "bea", &mover).unwrap();
    // Never one that exists, and never a key that is not plain.
    assert!(pctwin_helper_os::holding::person_folder(&area, "bea", &mover).is_err());
    assert!(pctwin_helper_os::holding::person_folder(&area, "../x", &mover).is_err());
    let check = || {
        #[cfg(windows)]
        {
            let out = std::process::Command::new("icacls")
                .arg(&first)
                .output()
                .unwrap();
            let text = String::from_utf8_lossy(&out.stdout);
            println!("{text}");
            // No one but the system, administrators, the mover and owner rights.
            assert!(
                !text.contains("Everyone") && !text.contains("BUILTIN\\Users"),
                "{text}"
            );
            // The mover may write files there.
            std::fs::write(first.join("probe.txt"), b"x").unwrap();
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let meta = std::fs::metadata(&first).unwrap();
            let pctwin_helper_os::Account::Uid(uid) = mover else {
                unreachable!()
            };
            assert_eq!(meta.uid(), uid);
            assert_eq!(meta.mode() & 0o777, 0o700);
            let parent = std::fs::metadata(&area).unwrap();
            assert_eq!(parent.uid(), 0);
            assert_eq!(parent.mode() & 0o777, 0o711);
            // Given to the person at the hand-over, by handle.
            std::fs::write(first.join("probe.txt"), b"x").unwrap();
            pctwin_helper_os::holding::give_to(&first, 4242, 4242).unwrap();
            assert_eq!(std::fs::metadata(&first).unwrap().uid(), 4242);
            assert_eq!(
                std::fs::metadata(first.join("probe.txt")).unwrap().uid(),
                4242
            );
        }
    };
    let result = std::panic::catch_unwind(check);
    let _ = std::fs::remove_dir_all(&area);
    let _ = std::fs::remove_dir_all(&other_area);
    result.unwrap();
}
