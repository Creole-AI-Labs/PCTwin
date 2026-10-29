//! Clean-up of PCTwin's own files (Task List 2.7 R07; acceptance T27-015).

mod common;

use common::file;
use pctwin_readiness::cleanup::{CleanupError, Entry, Ownership, Plan, What, approve};

fn entry(name: &str, what: What, ownership: Ownership) -> Entry {
    Entry {
        name: name.into(),
        what,
        bytes: 10,
        ownership,
        item: None,
        in_use: false,
    }
}

fn found() -> Vec<Entry> {
    let mut chosen = entry(
        "model.gguf",
        What::ModelAndCache,
        Ownership::RecordedByPctwin,
    );
    chosen.item = Some(file("model.gguf").id);
    let mut busy = entry(
        "pctwin-capture-01",
        What::TemporaryFiles,
        Ownership::RecordedByPctwin,
    );
    busy.in_use = true;
    vec![
        entry(
            "pctwin-installs-00",
            What::TemporaryFiles,
            Ownership::RecordedByPctwin,
        ),
        entry(
            "pctwin-installs-ff",
            What::TemporaryFiles,
            Ownership::LooksTemporary,
        ),
        busy,
        entry(
            "catalogue-cache",
            What::ModelAndCache,
            Ownership::RecordedByPctwin,
        ),
        chosen,
    ]
}

/// T27-015: only what PCTwin recorded making is removed; what only looks temporary, what is in
/// use and what is the person's chosen work is kept and named; each question is its own.
#[test]
fn t27_015_only_pctwin_s_own_files_and_each_question_on_its_own() {
    let f = found();
    let t = Plan::of(What::TemporaryFiles, &f);
    let names = |v: &[Entry]| v.iter().map(|e| e.name.clone()).collect::<Vec<_>>();
    assert_eq!(names(&t.removes), ["pctwin-installs-00"]);
    assert_eq!(names(&t.keeps), ["pctwin-installs-ff", "pctwin-capture-01"]);
    let m = Plan::of(What::ModelAndCache, &f);
    assert_eq!(names(&m.removes), ["catalogue-cache"]);
    assert_eq!(names(&m.keeps), ["model.gguf"]);
    // A confirmation of one question never approves the other.
    assert_eq!(
        approve(What::ModelAndCache, &f, t.digest()),
        Err(CleanupError::Changed)
    );
    assert_eq!(
        approve(What::TemporaryFiles, &f, t.digest()).unwrap(),
        t.removes
    );
}

/// What is there changed since the person confirmed: refused, nothing removed; nothing of
/// PCTwin's: refused.
#[test]
fn a_changed_or_empty_list_is_refused() {
    let mut f = found();
    let t = Plan::of(What::TemporaryFiles, &f);
    f.push(entry(
        "pctwin-installs-11",
        What::TemporaryFiles,
        Ownership::RecordedByPctwin,
    ));
    assert_eq!(
        approve(What::TemporaryFiles, &f, t.digest()),
        Err(CleanupError::Changed)
    );
    let none: Vec<Entry> = vec![];
    let empty = Plan::of(What::TemporaryFiles, &none);
    assert_eq!(
        approve(What::TemporaryFiles, &none, empty.digest()),
        Err(CleanupError::Nothing)
    );
}
