//! Whether the person moving is one of the laptop's administrators (the 20 October 2026 Wi-Fi
//! rule: saved Wi-Fi networks are offered ticked for an administrator, unticked with "ask
//! whoever looks after this laptop" otherwise), as the system's own tools say it.

use pctwin_settings::runner::{Output, Runner, Tool};
use pctwin_settings::wifi::{admin_in_groups, admin_in_whoami, mover_is_admin, offer};

#[test]
fn windows_says_it_by_the_administrators_sid_in_any_language() {
    // Not elevated: Windows lists the group "for deny only"; still an administrator.
    let admin = "\"Everyone\",\"Well-known group\",\"S-1-1-0\",\"Mandatory group\"\r\n\"BUILTIN\\Administrateurs\",\"Alias\",\"S-1-5-32-544\",\"Group used for deny only\"\r\n";
    assert!(admin_in_whoami(admin));
    // A group name with a comma and a doubled quote is one field.
    assert!(admin_in_whoami(
        "\"A, \"\"odd\"\" name\",\"Alias\",\"S-1-5-32-544\",\"Enabled group\""
    ));
    // Users only, or the SID anywhere but its own field: not an administrator.
    assert!(!admin_in_whoami(
        "\"BUILTIN\\Users\",\"Alias\",\"S-1-5-32-545\",\"Mandatory group\""
    ));
    assert!(!admin_in_whoami(
        "\"S-1-5-32-544\",\"Alias\",\"S-1-5-32-545\",\"Mandatory group\""
    ));
    assert!(!admin_in_whoami(
        "\"x\",\"Alias\",\"S-1-5-32-5440\",\"Mandatory group\""
    ));
    assert!(!admin_in_whoami(""));
}

#[test]
fn mac_and_linux_say_it_by_the_administrators_group() {
    assert!(admin_in_groups("20 12 61 79 80 81", true));
    assert!(!admin_in_groups("20 12 61 79 180", true));
    assert!(
        !admin_in_groups("sudo wheel", true),
        "a Mac goes by numbers"
    );
    assert!(admin_in_groups("ada adm sudo", false));
    assert!(admin_in_groups("ada wheel", false));
    assert!(admin_in_groups("ada admin", false));
    assert!(!admin_in_groups("ada adm sudoers", false));
    assert!(!admin_in_groups("80", false));
}

struct Says(Result<Output, String>, Vec<(Tool, Vec<String>)>);

impl Runner for Says {
    fn run(&mut self, tool: Tool, args: &[&str]) -> Result<Output, String> {
        self.1
            .push((tool, args.iter().map(|a| a.to_string()).collect()));
        self.0.clone()
    }
}

#[test]
fn the_systems_own_tool_is_asked_and_not_known_is_not_an_administrator() {
    let yes = if cfg!(windows) {
        "\"BUILTIN\\Administrators\",\"Alias\",\"S-1-5-32-544\",\"Group used for deny only\""
    } else if cfg!(target_os = "macos") {
        "20 80"
    } else {
        "ada sudo"
    };
    let mut r = Says(
        Ok(Output {
            ok: true,
            stdout: yes.into(),
        }),
        Vec::new(),
    );
    assert!(mover_is_admin(&mut r));
    let (tool, args) = &r.1[0];
    if cfg!(windows) {
        assert_eq!(*tool, Tool::Whoami);
        assert_eq!(args, &["/groups", "/fo", "csv", "/nh"]);
    } else {
        assert_eq!(*tool, Tool::Id);
    }
    // The tool failed, or could not run: not known, so not an administrator.
    let mut failed = Says(
        Ok(Output {
            ok: false,
            stdout: yes.into(),
        }),
        Vec::new(),
    );
    assert!(!mover_is_admin(&mut failed));
    assert!(!mover_is_admin(&mut Says(Err("no".into()), Vec::new())));
    // And the offer follows it.
    assert!(offer(true).ticked);
    assert!(!offer(false).ticked);
}

/// The real tool on this laptop answers (whatever the answer): no error, no prompt.
#[test]
fn the_real_tool_answers() {
    let _ = mover_is_admin(&mut pctwin_settings::runner::SystemRunner);
}
