//! Windows' own judgement of file types (`AssocIsDangerous`), as the settings judge asks it.
#![cfg(windows)]

use pctwin_helper_os::file_type_is_dangerous;

#[test]
fn windows_calls_programs_and_scripts_dangerous_and_plain_documents_not() {
    for e in [
        "exe", ".exe", "bat", "cmd", "msi", "ps1", "vbs", "js", "reg", "chm",
    ] {
        assert!(file_type_is_dangerous(e), "{e} counts as dangerous");
    }
    for e in ["txt", ".png", "csv"] {
        assert!(!file_type_is_dangerous(e), "{e} is a plain document");
    }
}

#[test]
fn an_ending_that_is_not_a_plain_file_type_is_never_trusted() {
    for e in [
        "",
        ".",
        "ex e",
        "exe\0",
        "a/b",
        "a\\b",
        "x:y",
        &"a".repeat(65),
    ] {
        assert!(file_type_is_dangerous(e), "{e:?} is refused");
    }
}
