//! macOS: the administrator's approval for one move (Security Design part C). CI cannot type an
//! administrator's password, so the check is proved with rights of the test's own: one macOS
//! always allows, and one that needs an administrator's password (which nobody types here, as in
//! the helper's own check, which may show nothing), written with password-free sudo (gated by
//! PCTWIN_HELPER_OS_ADMIN_TESTS=1, set only in CI) and removed after. PCTwin's own right is then
//! written as root and read back.
#![cfg(target_os = "macos")]

use pctwin_helper_os::approval::{Approval, MOVE_RIGHT, approval_holds_for, define_move_right};
use std::process::Command;

const ALLOW: &str = "com.creoleailabs.pctwin.ci-allow";
const DENY: &str = "com.creoleailabs.pctwin.ci-needs-admin";

fn admin_tests() -> bool {
    std::env::var("PCTWIN_HELPER_OS_ADMIN_TESTS").as_deref() == Ok("1")
}

fn sudo_security(args: &[&str]) -> bool {
    Command::new("sudo")
        .args(["-n", "/usr/bin/security", "authorizationdb"])
        .args(args)
        .status()
        .is_ok_and(|s| s.success())
}

#[test]
#[ignore = "needs PCTWIN_HELPER_OS_ADMIN_TESTS=1 (CI): writes and removes two rights"]
fn macos_confirms_an_approval_only_for_a_right_it_holds() {
    use security_framework::authorization::{Authorization, AuthorizationItemSetBuilder, Flags};
    assert!(
        admin_tests(),
        "set PCTWIN_HELPER_OS_ADMIN_TESTS=1 (CI only)"
    );
    assert!(sudo_security(&["write", ALLOW, "allow"]));
    // macOS will not store a right ruled plain "deny" (-60005).
    assert!(sudo_security(&["write", DENY, "authenticate-admin"]));
    let check = || {
        let rights = AuthorizationItemSetBuilder::new()
            .add_right(ALLOW)
            .unwrap()
            .build();
        // Kept alive while the helper's check runs: the external form refers to it.
        let approved = Authorization::new(
            Some(rights),
            None,
            Flags::EXTEND_RIGHTS | Flags::PREAUTHORIZE,
        )
        .expect("a right macOS always allows");
        let form = approved.make_external_form().unwrap();
        let approval = Approval(std::array::from_fn(|i| form.bytes[i] as u8));
        assert_eq!(approval_holds_for(&approval, ALLOW).ok(), Some(true));
        // An approval for one right never holds another that needs an administrator. (Not checked
        // against PCTwin's own right here: the other test in this program approves that one at
        // the same time, and in CI run 40089359575 this program then held it too, which looks like
        // macOS letting one approval in a process stand for another there; the app and the helper
        // are separate processes, and the right is never shared.)
        assert_eq!(approval_holds_for(&approval, DENY).ok(), Some(false));
        // Bytes that are no approval never hold anything.
        assert_ne!(
            approval_holds_for(&Approval([0; 32]), ALLOW).ok(),
            Some(true)
        );
        assert_ne!(
            approval_holds_for(&Approval([0x5a; 32]), ALLOW).ok(),
            Some(true)
        );
    };
    let r = std::panic::catch_unwind(check);
    sudo_security(&["remove", ALLOW]);
    sudo_security(&["remove", DENY]);
    r.unwrap();
}

#[test]
#[ignore = "needs PCTWIN_HELPER_OS_ADMIN_TESTS=1 (CI): writes PCTwin's right as root"]
fn pctwins_right_needs_an_administrator_and_is_never_shared() {
    assert!(
        admin_tests(),
        "set PCTWIN_HELPER_OS_ADMIN_TESTS=1 (CI only)"
    );
    let uid = Command::new("id").arg("-u").output().unwrap();
    if String::from_utf8_lossy(&uid.stdout).trim() != "0" {
        let st = Command::new("sudo")
            .args(["-n", "env", "PCTWIN_HELPER_OS_ADMIN_TESTS=1"])
            .arg(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "pctwins_right_needs_an_administrator_and_is_never_shared",
                "--ignored",
                "--nocapture",
            ])
            .status()
            .unwrap();
        assert!(st.success(), "run as root, it failed: {st}");
        return;
    }
    define_move_right().unwrap();
    let out = Command::new("/usr/bin/security")
        .args(["authorizationdb", "read", MOVE_RIGHT])
        .output()
        .unwrap();
    let _ = Command::new("/usr/bin/security")
        .args(["authorizationdb", "remove", MOVE_RIGHT])
        .status();
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    println!("{text}");
    let squeezed: String = text.split_whitespace().collect();
    for needed in [
        "<key>group</key><string>admin</string>",
        "<key>shared</key><false/>",
        "<key>allow-root</key><false/>",
        "<key>authenticate-user</key><true/>",
        "<key>class</key><string>user</string>",
    ] {
        assert!(squeezed.contains(needed), "{needed} in {text}");
    }
}
