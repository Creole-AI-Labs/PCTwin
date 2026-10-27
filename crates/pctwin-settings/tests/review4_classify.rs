//! The run-or-fetch rule of the fourth review (finding F1), case by case: what is plain data,
//! what could run something, what could fetch something, and what points at a place PCTwin
//! cannot check. With no catalogue hints at all, and with the starter catalogue's.

use pctwin_settings::catalogue::{Catalogue, HintKind};
use pctwin_settings::classify::{
    Action, Why, action_kind, danger, file_shaped_name, may_write_text, run_or_fetch,
};
use pctwin_settings::judge::Facts;

const STORE: &str = r"Software\Vendor\App";

/// No catalogue hints, and a system that runs no file ending and opens no scheme: the shape
/// rule alone.
fn bare(t: &str) -> Option<Why> {
    danger(&Facts::new([], Some(vec![])), &[], STORE, "Value", t)
}

#[test]
fn plain_data_passes() {
    for t in [
        "ocean",
        "Segoe UI",
        "12:30",
        "Arial:12",
        "abc:1",
        "abc: x",
        "1.5",
        "-3",
        "125",
        "",
        "   ",
        "and/or",
        "x.y",
        "a%b",
        "Tom and Jerry",
        "-",
        "a - b",
        "en-US",
        "100%",
    ] {
        assert_eq!(bare(t), None, "{t:?}");
    }
}

#[test]
fn one_full_local_path_is_data_for_the_caller_to_point_at_the_moved_file() {
    for t in [
        r"C:\Users\ada\x.txt",
        r"C:\Program Files\App\data.txt",
        r#""C:\Program Files\App\data.txt""#,
        "C:/Users/ada/x.txt",
        "/home/ada/x",
        "file:///home/ada/x",
        "file:///c:/Users/ada/x",
    ] {
        assert_eq!(bare(t), None, "{t:?}");
    }
}

#[test]
fn another_computer_or_an_address_could_fetch_something() {
    let long = format!("{}:x/y", "a".repeat(32));
    for t in [
        r"\\srv\s",
        r"\\srv\s\x.jpg",
        "//srv/s",
        "https://example.com/u.xml",
        "ftp://x",
        "file://host/s",
        "FILE://host/s",
        r"see \\srv\s now",
        "x=https://y",
    ] {
        assert_eq!(bare(t), Some(Why::Fetches), "{t:?}");
    }
    // A scheme followed by a name is held back even when no handler is registered for it (the
    // shell opens `shell:` names no registry lists; fifth review, finding 2), and so is a time
    // format that has the same shape: PCTwin cannot tell them apart.
    for t in [
        "ab:x/y",
        "mailto:a@b",
        "search-ms:query=x",
        "javascript:alert(1)",
        long.as_str(),
        "shell:startup",
        "HH:mm:ss",
        "mm:ss",
        "ms-settings:display",
    ] {
        assert_eq!(bare(t), Some(Why::PlaceUnknown), "{t:?}");
    }
    // A scheme longer than any is not one; digits after the colon are a time or a pair.
    assert_eq!(bare(&format!("{}:x?y", "a".repeat(33))), None);
    assert_eq!(bare("1a:x?y"), None);
    assert_eq!(bare("a_b:x?y"), None);
    assert_eq!(bare("ab:1/2"), None);
}

#[test]
fn a_command_line_could_run_something() {
    for t in [
        "cmd /c calc",
        "cmd /k:x",
        "cmd /? x",
        "sh -c id",
        "app --run",
        "powershell -ExecutionPolicy Bypass",
        "a && b",
        "a || b",
        "a | b",
        "a > f",
        "a < f",
        "`id`",
        "$(id)",
        "${X}",
        r#""C:\Program Files\x\y.exe" --tray"#,
    ] {
        assert_eq!(bare(t), Some(Why::StartsProgram), "{t:?}");
    }
    // A word that only looks like a switch is not one.
    for t in ["a /home", "a /1", "a --", "a -1", "a -", "a /abcd"] {
        assert_ne!(bare(t), Some(Why::StartsProgram), "{t:?}");
    }
}

#[test]
fn a_place_written_so_it_cannot_be_checked_is_kept_back() {
    for t in [
        r"%SystemRoot%\System32\cmd.exe",
        r"%USERPROFILE%\Exports",
        "a%b%c",
        "$HOME/x",
        "~/x",
        "~",
        "./x",
        "../x",
        r"bin\x",
        "C:x",
        "C:",
        r"C:..\x",
    ] {
        assert_eq!(bare(t), Some(Why::PlaceUnknown), "{t:?}");
    }
}

#[test]
fn a_key_named_like_a_file_keeps_back_any_path_with_no_hints_at_all() {
    for n in ["SCRNSAVE.EXE", "x.cur", "a.B", "Icon.ico"] {
        assert!(file_shaped_name(n), "{n}");
        assert_eq!(action_kind(&[], STORE, n), Some(Action::File), "{n}");
    }
    for n in [
        "editor.fontSize",
        "1.5",
        "v1.2",
        ".exe",
        "a.b c",
        "a\\b.exe",
        "plain",
        "a.",
    ] {
        assert!(!file_shaped_name(n), "{n}");
    }
    let s = |v: &str| run_or_fetch(&[], r"Control Panel\Desktop", "SCRNSAVE.EXE", v);
    assert_eq!(s(r"C:\Users\Ada\Documents\x.dat"), Some(Why::StartsProgram));
    assert_eq!(s("x.scr"), Some(Why::StartsProgram));
    assert_eq!(s("1"), None);
    assert_eq!(s(""), None);
}

#[test]
fn the_catalogue_hints_only_add_caution() {
    let cat = Catalogue::starter().unwrap();
    let h = &cat.action_hints;
    assert!(
        h.iter()
            .any(|x| x.kind == HintKind::Run && x.ending.is_none())
    );
    assert!(h.iter().any(|x| x.kind == HintKind::File));
    assert!(h.iter().any(|x| x.ending.is_some()));
    // Endings of programs: a program's file anywhere in the value.
    for t in [
        r"C:\Tools\edit.exe",
        "/home/ada/run.sh",
        "notepad.exe",
        "a b.exe",
        r#""x.bat" y"#,
    ] {
        assert_eq!(
            run_or_fetch(h, STORE, "Value", t),
            Some(Why::StartsProgram),
            "{t:?}"
        );
    }
    // The same values with no hints and a system that names no program ending: a full local
    // path is data, a bare name is text (on a real laptop its own facts keep them back).
    assert_eq!(bare(r"C:\Tools\edit.exe"), None);
    assert_eq!(bare("notepad.exe"), None);
    // A run hint: anything but a number is kept back.
    assert_eq!(
        run_or_fetch(h, STORE, "Editor", "notepad"),
        Some(Why::StartsProgram)
    );
    assert_eq!(run_or_fetch(h, STORE, "ProxyEnable", "1"), None);
    assert_eq!(run_or_fetch(h, STORE, "ProxyPort", "8080"), None);
    assert_eq!(run_or_fetch(h, STORE, "EditorZoom", "-1.5"), None);
    // Not a number: two dots, a leading dot, a lone sign.
    for t in ["1.2.3", ".5", "-", "1e3"] {
        assert_eq!(
            run_or_fetch(h, STORE, "Editor", t),
            Some(Why::StartsProgram),
            "{t}"
        );
    }
    // An ending alone is not a program's file.
    assert_eq!(run_or_fetch(h, STORE, "Value", ".exe"), None);
    assert_eq!(
        run_or_fetch(h, "/org/vendor/app/", "command", "id"),
        Some(Why::StartsProgram)
    );
    // A file hint: a file name or a path is kept back, a theme's name is not.
    let cur = r"Control Panel\Cursors";
    assert_eq!(
        run_or_fetch(h, cur, "Arrow", "aero_arrow.cur"),
        Some(Why::StartsProgram)
    );
    assert_eq!(
        run_or_fetch(h, cur, "Arrow", r"C:\Users\ada\a.cur"),
        Some(Why::StartsProgram)
    );
    assert_eq!(run_or_fetch(h, cur, "Scheme Source", "2"), None);
    assert_eq!(run_or_fetch(h, cur, "Theme", "Papirus"), None);
    assert_eq!(action_kind(h, cur, "Arrow"), Some(Action::File));
    assert_eq!(action_kind(h, STORE, "Theme"), None);
    // Hints never loosen: what the shape rule keeps back stays back with any hints.
    for t in [r"\\srv\s", "cmd /c x", "%X%\\y"] {
        assert_eq!(
            bare(t).is_some(),
            run_or_fetch(h, STORE, "Value", t).is_some(),
            "{t}"
        );
    }
}

#[test]
fn writing_checks_the_same_rule_again() {
    let h = &Catalogue::starter().unwrap().action_hints;
    assert_eq!(may_write_text(h, STORE, "Theme", "ocean"), Ok(()));
    assert_eq!(
        may_write_text(h, STORE, "Last", r"C:\Users\ada2\x.txt"),
        Ok(())
    );
    assert_eq!(
        may_write_text(h, STORE, "Theme", r"\\srv\s"),
        Err(Why::Fetches)
    );
    assert_eq!(
        may_write_text(h, STORE, "Theme", "a\u{202E}b"),
        Err(Why::HiddenText)
    );
    assert_eq!(
        may_write_text(h, STORE, "Theme", &"a".repeat(4001)),
        Err(Why::TooLarge)
    );
    assert_eq!(may_write_text(h, STORE, "Theme", &"a".repeat(4000)), Ok(()));
}
