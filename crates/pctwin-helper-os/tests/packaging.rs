//! The files the packages install for the helper: the Linux polkit policy and the macOS launch
//! daemon's property list. Checked on every system.

use pctwin_helper_os::daemon::HELPER_PLIST;
use std::path::Path;

fn read(rel: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)).unwrap()
}

fn between<'a>(s: &'a str, open: &str, close: &str) -> Vec<&'a str> {
    s.split(open)
        .skip(1)
        .map(|part| part.split(close).next().unwrap().trim())
        .collect()
}

#[test]
fn the_polkit_policy_names_one_program_and_asks_every_time() {
    let p = read("packaging/linux/com.creoleailabs.pctwin.helper.policy");
    assert_eq!(between(&p, "<action ", ">").len(), 1, "one action only");
    assert_eq!(
        between(
            &p,
            "<annotate key=\"org.freedesktop.policykit.exec.path\">",
            "</annotate>"
        ),
        ["/usr/lib/pctwin/pctwin-helper"]
    );
    assert_eq!(between(&p, "<allow_any>", "</allow_any>"), ["no"]);
    assert_eq!(between(&p, "<allow_inactive>", "</allow_inactive>"), ["no"]);
    // auth_admin, never auth_admin_keep: approval lasts one move.
    assert_eq!(
        between(&p, "<allow_active>", "</allow_active>"),
        ["auth_admin"]
    );
    assert!(!p.contains("allow_gui"), "nothing beyond the one program");
}

#[test]
fn the_launch_daemon_runs_the_bundled_helper_for_this_app() {
    let file = format!("packaging/macos/{HELPER_PLIST}");
    let p = read(&file);
    let conf = read("../../app/src-tauri/tauri.conf.json");
    let app_id = between(&conf, "\"identifier\": \"", "\"")[0];
    let label = HELPER_PLIST.strip_suffix(".plist").unwrap();
    let strings = between(&p, "<string>", "</string>");
    let keys = between(&p, "<key>", "</key>");
    assert_eq!(strings[0], label, "Label matches the file name");
    assert!(keys.contains(&"BundleProgram"));
    assert!(
        strings.contains(&"Contents/MacOS/pctwin-helper"),
        "inside the signed bundle"
    );
    assert!(strings.contains(&app_id), "tied to the app {app_id}");
    // Started once when registered (and at start-up while registered), never kept alive: the
    // helper exits unless a move is in progress, and the app unregisters it afterwards.
    let compact: String = p.split_whitespace().collect();
    assert!(compact.contains("<key>RunAtLoad</key><true/>"));
    assert!(compact.contains("<key>KeepAlive</key><false/>"));
    // Started with no arguments (launchd names the program only), it is the daemon; it serves
    // one approved move and ends, or ends after ten minutes with none.
    assert!(p.contains("Started with no"));
    assert!(p.contains("arguments, the helper is the launch daemon"));
    assert!(p.contains("ends after ten minutes with none"));
    for k in [
        "Program",
        "ProgramArguments",
        "MachServices",
        "Sockets",
        "UserName",
    ] {
        assert!(
            !keys.contains(&k),
            "{k}: the daemon runs only the bundled helper, nothing else"
        );
    }
}
