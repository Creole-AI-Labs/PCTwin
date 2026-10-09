//! The transfer crate changes nobody's files itself (Security Design B, decided 8 October 2026):
//! every file it writes, names or removes goes through the safety gate (`pctwin-gate`), which
//! checks what it acts on through a handle. This test reads the crate's own source and fails if
//! any of the standard library's ways of changing files appears there, so a new one cannot slip
//! in unnoticed. Reading files (as the old laptop does to answer "is the original still there?")
//! is allowed.

/// The ways to change, make or remove a file or folder without the gate.
const CHANGES: &[&str] = &[
    "remove_file",
    "remove_dir",
    "fs::rename",
    ".rename(",
    "fs::write",
    "File::create",
    "create_new",
    ".create(",
    ".truncate(",
    ".append(true",
    ".write(true",
    "set_len",
    "fs::copy",
    "create_dir",
    "hard_link",
    "symlink(",
    "symlink_file",
    "symlink_dir",
    "set_permissions",
    "set_modified",
    "set_times",
    "unlinkat",
    "renameat",
    "delete_by_handle",
];

/// Each way of changing a file found in `text`, with its line number. A file's unit tests (the
/// module marked `#[cfg(test)]`, at the end by convention) make files to test on and are never
/// built into PCTwin, so the scan stops there; anything else marked for tests alone is still
/// scanned.
fn changes_in(text: &str) -> Vec<(usize, &'static str)> {
    let mut found = Vec::new();
    let mut previous = "";
    for (n, line) in text.lines().enumerate() {
        if previous == "#[cfg(test)]" && line.starts_with("mod ") {
            break;
        }
        previous = line.trim_end();
        let code = line.split("//").next().unwrap_or("");
        for change in CHANGES {
            if code.contains(change) {
                found.push((n + 1, *change));
            }
        }
    }
    found
}

#[test]
fn no_file_is_changed_except_through_the_gate() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut found = Vec::new();
    let mut files = 0;
    for entry in std::fs::read_dir(&src).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "rs") {
            continue;
        }
        files += 1;
        let text = std::fs::read_to_string(&path).unwrap();
        for (line, change) in changes_in(&text) {
            found.push(format!("{}:{line}: {change}", path.display()));
        }
    }
    assert!(files > 10, "the source was not found");
    assert!(
        found.is_empty(),
        "files changed outside the gate:\n{}",
        found.join("\n")
    );
}

/// The scan finds a change written in any of the usual ways, and ignores one only mentioned in a
/// comment.
#[test]
fn the_scan_finds_a_change_and_ignores_comments() {
    let sample = "fn f(p: &Path) {
    // std::fs::remove_file is never used here
    let _ = std::fs::remove_file(p);
    let _ = std::fs::OpenOptions::new().write(true).open(p);
}
";
    assert_eq!(changes_in(sample), [(3, "remove_file"), (4, ".write(true")]);
    // The unit tests at the end are not scanned; a function only for tests elsewhere still is.
    let with_tests = "#[cfg(test)]
fn hook(p: &Path) { let _ = std::fs::remove_file(p); }
fn f() {}
#[cfg(test)]
mod tests {
    fn t(p: &Path) { std::fs::write(p, b\"x\").unwrap(); }
}
";
    assert_eq!(changes_in(with_tests), [(2, "remove_file")]);
    for change in CHANGES {
        assert_eq!(changes_in(&format!("x{change}y")).len(), 1, "{change}");
    }
}
