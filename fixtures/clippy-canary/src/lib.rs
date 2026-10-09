//! Every function below changes a file the forbidden way. Clippy must refuse each one.
#![forbid(clippy::disallowed_methods)]

use std::fs as renamed; // an alias must not hide the call: clippy resolves the real function
use std::fs::OpenOptions;

pub fn by_name(p: &std::path::Path) {
    let _ = std::fs::remove_file(p);
}

pub fn through_an_alias(p: &std::path::Path) {
    let _ = renamed::rename(p, p);
}

pub fn make_a_file(p: &std::path::Path) {
    let _ = std::fs::File::create(p);
}

pub fn open_for_writing(p: &std::path::Path) {
    let _ = OpenOptions::new().write(true).open(p);
}
