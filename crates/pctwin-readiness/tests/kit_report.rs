//! The recovery kit (Engineering Plan A02) and the shareable report (Task List 2.7; plan 2.7
//! section 9; acceptance T27-025, T27-026, T27-028, T27-030).

// Fixtures are written with std::fs; only the crate itself is fenced.
#![allow(clippy::disallowed_methods)]

mod common;

use common::*;
use pctwin_readiness::checks::Summary;
use pctwin_readiness::goodbye::{Checklist, Inputs, OldLaptop};
use pctwin_readiness::kit::{KIT_FILE, Kit, KitError, README_FILE, date};
use pctwin_readiness::ledger::{Observed, Role};
use pctwin_readiness::report::{
    Disclosure, FEEDBACK_WORDS, NOT_A_CERTIFICATE, scrub, share_preview,
};
use pctwin_record::{Inclusion, ItemKind};
use pctwin_types::{ItemResult, Reason};

fn moved() -> (
    pctwin_record::Record,
    pctwin_readiness::ledger::Ledger,
    Checklist,
) {
    let (record, mut l) = ledger(vec![
        file("Taxes 2025 ada@example.com.pdf"),
        file("notes ghp_abcdefghijklmnopqrstuvwxyz0123456789.txt"),
        app("Secret Project Editor"),
        item("1001", "bea-diary.txt", ItemKind::File, Inclusion::Included),
    ]);
    let ids: Vec<_> = record.items.iter().map(|i| i.id).collect();
    for id in &ids[..2] {
        l.accept(
            receipt(
                &l,
                Role::Transfer,
                *id,
                Some(ItemResult::because(Reason::Unreadable)),
            ),
            &Sees(Observed::Matches),
            T0 + 20,
        )
        .unwrap();
    }
    l.accept(
        receipt(
            &l,
            Role::Install,
            ids[2],
            Some(ItemResult::because(Reason::ManualInstall)),
        ),
        &Sees(Observed::Matches),
        T0 + 20,
    )
    .unwrap();
    l.searched("Documents");
    let c = Checklist::build(
        &Inputs {
            ledger: &l,
            approved: &record,
            old_laptop: OldLaptop::Scanned(&record),
            left_on_old_laptop: &[],
            steps_not_finished: &[],
            undo_open: true,
            special: &[],
        },
        None,
        T0 + 100,
    );
    (record, l, c)
}

/// A02: the kit is exported and read back with no account, and says exactly what it holds.
#[test]
fn the_recovery_kit_is_kept_without_an_account_and_says_what_it_holds() {
    let (record, l, c) = moved();
    let kit = Kit::new(record.clone(), l.clone(), Some(c.clone()), T0);
    let dir = tempfile::tempdir().unwrap();
    let (json, readme) = kit.export(dir.path()).unwrap();
    assert_eq!(json.file_name().unwrap(), KIT_FILE);
    assert_eq!(readme.file_name().unwrap(), README_FILE);
    assert_eq!(Kit::read(&json).unwrap(), kit);
    let words = std::fs::read_to_string(&readme).unwrap();
    assert!(words.contains("made on 2027-01-15"), "{words}");
    assert!(words.contains("cannot bring back missing files"));
    assert!(words.contains("You need no account"));
    assert!(words.contains("- the result of every item, and how each result was checked;"));
    assert!(words.contains("- the steps to set your apps and settings up again."));
    assert!(words.contains("- a tool to restore anything"));
    assert!(words.contains("the list of what is still only on the old laptop"));
    // Exported again: beside the first, never over it.
    let (json2, _) = kit.export(dir.path()).unwrap();
    assert_ne!(json, json2);
    assert_eq!(
        std::fs::read(&json).unwrap(),
        std::fs::read(&json2).unwrap()
    );
    // Without the list, it says so by leaving that line out.
    let bare = Kit::new(record.clone(), l.clone(), None, T0);
    assert!(!bare.readme().contains("only on the old laptop"));
    assert!(bare.readme().contains("does not hold"));
    // Fresh-context review F11: it names files, so it says where not to keep it.
    assert!(bare.readme().contains("names your files"));
    assert!(bare.readme().contains("Do not save it in a shared folder"));
}

#[test]
fn a_newer_damaged_mixed_or_partial_kit_is_reported_as_such() {
    let (record, l, c) = moved();
    let dir = tempfile::tempdir().unwrap();
    let kit = Kit::new(record.clone(), l.clone(), Some(c.clone()), T0);
    let (json, _) = kit.export(dir.path()).unwrap();
    let text = std::fs::read_to_string(&json).unwrap();
    let write = |name: &str, t: &str| {
        let p = dir.path().join(name);
        std::fs::write(&p, t).unwrap();
        p
    };
    let newer = write(
        "newer.json",
        &text.replacen("\"format\": 1", "\"format\": 2", 1),
    );
    assert!(matches!(
        Kit::read(&newer),
        Err(KitError::NewerFormat { found: 2 })
    ));
    let damaged = write("damaged.json", &text[..text.len() / 2]);
    assert!(matches!(Kit::read(&damaged), Err(KitError::Damaged)));
    assert!(matches!(
        Kit::read(&write("nf.json", "{}")),
        Err(KitError::Damaged)
    ));
    let claims = write(
        "claims.json",
        &text.replacen("\"file_data\": false", "\"file_data\": true", 1),
    );
    assert!(matches!(Kit::read(&claims), Err(KitError::Partial)));
    for claim in ["setup_recipe", "recovery_tool"] {
        let c = write(
            "claim.json",
            &text.replacen(
                &format!("\"{claim}\": false"),
                &format!("\"{claim}\": true"),
                1,
            ),
        );
        assert!(matches!(Kit::read(&c), Err(KitError::Partial)), "{claim}");
    }
    let no_list = Kit::new(record.clone(), l.clone(), None, T0);
    let mut v = serde_json::to_value(&no_list).unwrap();
    v["contents"]["before_wipe_list"] = true.into();
    let p = write("nolist.json", &v.to_string());
    assert!(matches!(Kit::read(&p), Err(KitError::Partial)));
    let mut other = record.clone();
    other.source_laptop = other_laptop();
    let mixed = Kit::new(other, l.clone(), Some(c), T0);
    let p = write("mixed.json", &serde_json::to_string(&mixed).unwrap());
    assert!(matches!(Kit::read(&p), Err(KitError::Mixed)));
    assert!(matches!(
        Kit::read(&dir.path().join("missing.json")),
        Err(KitError::Io(_))
    ));
}

#[test]
fn dates_are_written_plainly() {
    assert_eq!(date(0), "1970-01-01");
    assert_eq!(date(951_782_400), "2000-02-29");
    assert_eq!(date(1_800_000_000), "2027-01-15");
    assert_eq!(date(4_102_444_800), "2100-01-01");
}

/// T27-025: the shared report holds no private names, account IDs or codes unless the person
/// selects them; even selected names lose what looks like a secret; another person's things are
/// never named.
#[test]
fn t27_025_the_share_preview_strips_private_things_unless_selected() {
    let (_, l, c) = moved();
    let sid = "S-1-5-21-3623811015-3361044348-30300820-1013";
    let p = share_preview(
        &l,
        Summary::FinishedWithTasks,
        &[],
        Some(&c),
        sid,
        Disclosure::default(),
    );
    for private in [
        "Taxes",
        "ada@example.com",
        "ghp_",
        "Secret Project",
        "bea",
        sid,
    ] {
        assert!(!p.text.contains(private), "{private} in:\n{}", p.text);
    }
    assert!(p.text.contains("- a file: failed (unreadable)"));
    assert!(
        p.text
            .contains("- an app or setting: needs action (manual-install)")
    );
    assert!(p.text.contains("other people's items not finished: 1"));
    assert_eq!((p.withheld.names, p.withheld.account_ids), (3, 1));

    let all = Disclosure {
        file_names: true,
        app_names: true,
        account_id: true,
    };
    let p = share_preview(&l, Summary::FinishedWithTasks, &[], Some(&c), sid, all);
    assert!(p.text.contains("Taxes 2025 [removed]"), "{}", p.text);
    assert!(p.text.contains("notes [removed]"));
    assert!(p.text.contains("Secret Project Editor"));
    assert!(p.text.contains(sid));
    for never in ["ada@example.com", "ghp_", "bea-diary"] {
        assert!(!p.text.contains(never));
    }
    assert_eq!(p.withheld.secrets, 2);
    // Only file names, not app names.
    let files_only = Disclosure {
        file_names: true,
        ..Disclosure::default()
    };
    let p = share_preview(&l, Summary::FinishedWithTasks, &[], None, sid, files_only);
    assert!(!p.text.contains("Secret Project"));
    assert!(p.text.contains("Taxes 2025"));
}

#[test]
fn scrub_removes_addresses_and_codes_only() {
    assert_eq!(
        scrub("Holiday photos 2024"),
        ("Holiday photos 2024".into(), 0)
    );
    assert_eq!(
        scrub("see https://example.com/x?t=1"),
        ("see [removed]".into(), 1)
    );
    assert_eq!(scrub("www.example.com"), ("[removed]".into(), 1));
    assert_eq!(scrub("mail a@b.co now"), ("mail [removed] now".into(), 1));
    assert_eq!(scrub("@handle"), ("@handle".into(), 0));
    assert_eq!(scrub("a@nodot"), ("a@nodot".into(), 0));
    // 19 mixed characters stay; 20 go; letters only or digits only stay.
    assert_eq!(scrub("abcdefghij123456789").1, 0);
    assert_eq!(scrub("abcdefghij1234567890").1, 1);
    assert_eq!(scrub("abcdefghijabcdefghijabcdefghij").1, 0);
    assert_eq!(scrub("123456789012345678901234").1, 0);
    assert_eq!(scrub("Projects/key-AKIAABCDEFGHIJ1234567.txt").1, 1);
    // Folder and name parts are judged apart: two short parts are not one long code.
    assert_eq!(scrub("Photos2024/Summer-trip").1, 0);
    assert_eq!(scrub("report2024.final-version").1, 0);
}

/// T27-026: the report is never a certificate. T27-028: feedback needs no account.
#[test]
fn t27_026_028_the_report_is_not_a_certificate_and_feedback_needs_no_account() {
    let (_, l, _) = moved();
    let p = share_preview(
        &l,
        Summary::FinishedWithTasks,
        &[],
        None,
        "1000",
        Disclosure::default(),
    );
    assert!(p.text.ends_with(&format!("{NOT_A_CERTIFICATE}\n")));
    assert!(FEEDBACK_WORDS.contains("You do not need an account"));
    assert!(FEEDBACK_WORDS.contains("Nothing is sent until you choose Send"));
}

/// T27-030: nothing is sent automatically. The crate cannot reach the network: its dependencies
/// are only these, none of which opens a connection, and its clippy fence bans the standard
/// library's connections and starting programs.
#[test]
fn t27_030_the_crate_has_no_way_to_send() {
    let manifest =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap();
    let deps = manifest
        .split("[dependencies]")
        .nth(1)
        .unwrap()
        .split("\n[")
        .next()
        .unwrap();
    let names: Vec<&str> = deps
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.split([' ', '=']).next().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "blake3",
            "pctwin-catalogue",
            "pctwin-journal",
            "pctwin-orchestrate",
            "pctwin-record",
            "pctwin-parse-seal",
            "pctwin-types",
            "serde",
            "serde_json",
            "tempfile",
            "thiserror"
        ]
    );
    let fence =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/clippy.toml")).unwrap();
    for banned in [
        "std::net::TcpStream::connect",
        "std::net::UdpSocket::bind",
        "std::process::Command::new",
    ] {
        assert!(fence.contains(&format!("\"{banned}\"")), "{banned}");
    }
    let lib = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs")).unwrap();
    assert!(lib.contains("forbid(clippy::disallowed_methods)"));
}

/// Lines appear only when there is something for them.
#[test]
fn the_report_lists_checks_and_other_people_only_when_there_are_some() {
    use pctwin_readiness::checks::{Choices, Chosen, Goal, run};
    let (_, mut l) = ledger(vec![file("a.txt")]);
    let a = l.lines().next().map(|(id, _)| *id).unwrap();
    l.accept(
        receipt(
            &l,
            Role::Transfer,
            a,
            Some(ItemResult::because(Reason::Unreadable)),
        ),
        &Sees(Observed::Matches),
        T0 + 20,
    )
    .unwrap();
    let p = share_preview(
        &l,
        Summary::FinishedWithTasks,
        &[],
        None,
        "1000",
        Disclosure::default(),
    );
    assert!(!p.text.contains("other people"));
    assert!(!p.text.contains("Checks you picked"));
    let c = Choices::picked(
        l.generation(),
        vec![Chosen {
            item: a,
            owner: Role::Data,
            goal: Goal::PersonCheck {
                task: "Open it".into(),
            },
        }],
    )
    .unwrap();
    let r = run(&c, &own());
    let p = share_preview(
        &l,
        Summary::FinishedWithTasks,
        &r,
        None,
        "1000",
        Disclosure::default(),
    );
    assert!(
        p.text.contains(
            "Checks you picked:
- check 1: still-needs-attention
"
        ),
        "{}",
        p.text
    );
}

/// PCTwin's own readers, run in the test's own process (only a parser crate's tests may).
fn own() -> pctwin_readiness::checks::InThisProcess {
    pctwin_readiness::checks::InThisProcess::new(pctwin_parse_seal::in_child())
}
