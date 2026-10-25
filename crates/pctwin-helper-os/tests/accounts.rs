//! Making a new person's account on the new laptop (`CreateAccount`): a standard account, never
//! an administrator, its temporary password passed to the system in memory only and marked to be
//! changed at the first sign-in; an existing account is never changed. The real account is made
//! only where it cannot harm anyone: a CI runner, as an administrator (gated by
//! PCTWIN_HELPER_OS_ADMIN_TESTS=1, set only in CI), and removed after.

use pctwin_helper_os::{Error, add_standard_account, is_account_name, is_full_name};

#[test]
fn names_are_checked_strictly_before_the_system_is_asked() {
    for good in ["bea", "bea-2", "b_x9", "abcdefghijklmnopqrst"] {
        assert!(is_account_name(good), "{good}");
    }
    for bad in [
        "",
        "Bea",
        "2bea",
        "-bea",
        "bea smith",
        "bea/../x",
        "bea:x",
        "abcdefghijklmnopqrstu",
        "béa",
    ] {
        assert!(!is_account_name(bad), "{bad}");
    }
    for good in ["Bea Smith", "Béatrice Ngozi Okafor", "李小龍"] {
        assert!(is_full_name(good), "{good}");
    }
    for bad in [
        "",
        "   ",
        "Bea:Smith",
        "Bea, Jr",
        "Bea\nSmith",
        &"x".repeat(65),
    ] {
        assert!(!is_full_name(bad), "{bad:?}");
    }
    let refused = |r: Result<_, Error>| matches!(r, Err(Error::BadInput(_)));
    assert!(refused(add_standard_account(
        "Bea",
        "Bea",
        "a-long-password-1"
    )));
    assert!(refused(add_standard_account(
        "bea",
        "Bea:x",
        "a-long-password-1"
    )));
    assert!(refused(add_standard_account("bea", "Bea", "short")));
    assert!(refused(add_standard_account(
        "bea",
        "Bea",
        "a long\npassword-1"
    )));
}

/// Without an administrator's rights the system refuses, and nothing is made. Run only on a CI
/// runner's ordinary test step (`CI` set, no administrator tests): on a person's own laptop, run
/// as an administrator, it would make a real account.
#[test]
fn without_an_administrator_nothing_is_made() {
    let ci = std::env::var_os("CI").is_some();
    let admin_tests = std::env::var("PCTWIN_HELPER_OS_ADMIN_TESTS").as_deref() == Ok("1");
    if !ci || admin_tests || is_admin() {
        return;
    }
    let r = add_standard_account("pctwinnoadmin", "PCTwin test", "a-long-password-1");
    assert!(r.is_err(), "{r:?}");
}

/// CI, as an administrator: the account is made, is not an administrator, must change its
/// password at the first sign-in, and a second make of the same name changes nothing.
#[test]
#[ignore = "needs PCTWIN_HELPER_OS_ADMIN_TESTS=1 (CI); makes and removes a real account"]
fn a_real_standard_account_is_made_once() {
    assert!(
        std::env::var("PCTWIN_HELPER_OS_ADMIN_TESTS").as_deref() == Ok("1"),
        "set PCTWIN_HELPER_OS_ADMIN_TESTS=1 (CI only)"
    );
    // Mac and Linux: CI's job is not root, so the test runs itself again through password-free
    // sudo.
    #[cfg(unix)]
    {
        let uid = std::process::Command::new("id").arg("-u").output().unwrap();
        if String::from_utf8_lossy(&uid.stdout).trim() != "0" {
            let st = std::process::Command::new("sudo")
                .args(["-n", "env", "PCTWIN_HELPER_OS_ADMIN_TESTS=1"])
                .arg(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "a_real_standard_account_is_made_once",
                    "--ignored",
                    "--nocapture",
                ])
                .status()
                .unwrap();
            assert!(st.success(), "run as root, it failed: {st}");
            return;
        }
    }
    let name = format!("pctwin{}", std::process::id() % 100_000);
    let password = "Temporary-Pass-123!";
    let made = add_standard_account(&name, "PCTwin Test", password);
    let check = || -> Result<(), String> {
        let account = made.as_ref().map_err(|e| e.to_string())?;
        println!("made {name}: {account:?}");
        assert!(matches!(
            add_standard_account(&name, "PCTwin Test", password),
            Err(Error::AlreadyExists)
        ));
        #[cfg(windows)]
        {
            let out = std::process::Command::new("net")
                .args(["user", &name])
                .output()
                .unwrap();
            let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
            println!("{text}");
            assert!(!text.contains("*administrators"), "never an administrator");
            assert!(text.contains("password expires") || text.contains("password last set"));
        }
        #[cfg(target_os = "linux")]
        {
            let groups = std::process::Command::new("id")
                .args(["-Gn", &name])
                .output()
                .unwrap();
            let groups = String::from_utf8_lossy(&groups.stdout);
            assert!(
                !groups
                    .split_whitespace()
                    .any(|g| g == "sudo" || g == "wheel" || g == "admin")
            );
            let chage = std::process::Command::new("chage")
                .args(["-l", &name])
                .output()
                .unwrap();
            let text = String::from_utf8_lossy(&chage.stdout);
            println!("{text}");
            assert!(text.contains("password must be changed"), "{text}");
        }
        #[cfg(target_os = "macos")]
        {
            let admins = std::process::Command::new("dscl")
                .args([".", "-read", "/Groups/admin", "GroupMembership"])
                .output()
                .unwrap();
            assert!(!String::from_utf8_lossy(&admins.stdout).contains(&name));
        }
        Ok(())
    };
    let result = check();
    // Removed whatever happened.
    #[cfg(windows)]
    let _ = std::process::Command::new("net")
        .args(["user", &name, "/delete"])
        .status();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("userdel")
        .args(["--remove", &name])
        .status();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("sysadminctl")
        .args(["-deleteUser", &name])
        .status();
    result.unwrap();
}

/// Whether this test runs with an administrator's rights (CI's Windows runners do): then an
/// account is rightly made, and "nothing is made" cannot be checked here.
fn is_admin() -> bool {
    #[cfg(windows)]
    let admin = std::process::Command::new("net")
        .arg("session")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    #[cfg(unix)]
    let admin = std::process::Command::new("id")
        .arg("-u")
        .output()
        .is_ok_and(|o| String::from_utf8_lossy(&o.stdout).trim() == "0");
    admin
}
