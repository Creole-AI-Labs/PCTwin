//! The recovery kit (Engineering Plan A02) and the shareable report (Task List 2.7; plan 2.7
//! section 9; acceptance T27-025, T27-026, T27-028, T27-030).

// Fixtures are written with std::fs; only the crate itself is fenced.
#![allow(clippy::disallowed_methods)]

mod common;

use common::*;
use pctwin_readiness::checks::Summary;
use pctwin_readiness::goodbye::{Checklist, Inputs, OldLaptop};
use pctwin_readiness::kit::{KIT_FILE, Kit, KitError, README_FILE, date};
use pctwin_readiness::ledger::Standing;
use pctwin_readiness::ledger::{Observed, Role};
use pctwin_readiness::report::{Disclosure, Leak, check_text, scrub, share_preview};
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
/// never named. Text the screens make from it is refused if it holds anything kept out.
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
    let json = serde_json::to_string(&p).unwrap();
    for private in [
        "Taxes",
        "ada@example.com",
        "ghp_",
        "Secret Project",
        "bea",
        sid,
    ] {
        assert!(!json.contains(private), "{private} in:\n{json}");
    }
    assert!(p.lines.iter().all(|l| l.name.is_none()));
    assert_eq!(p.lines.len(), 3);
    assert!(p.lines.iter().any(|l| !l.is_app
        && l.standing == Standing::Failed
        && l.reason == Some(Reason::Unreadable)));
    assert!(p.lines.iter().any(|l| l.is_app
        && l.standing == Standing::NeedsAction
        && l.reason == Some(Reason::ManualInstall)));
    assert_eq!(p.others_not_finished, 1);
    assert_eq!(p.account_id, None);
    assert_eq!((p.withheld.names, p.withheld.account_ids), (3, 1));

    // Text holding a name kept out, by its whole path or its last part, in any case, is refused.
    assert_eq!(
        check_text(&p, "Not finished: Taxes 2025 ada@example.com.pdf"),
        Err(Leak::KeptOut)
    );
    assert_eq!(check_text(&p, "SECRET PROJECT EDITOR"), Err(Leak::KeptOut));
    assert_eq!(
        check_text(&p, "a file of bea-diary.txt"),
        Err(Leak::KeptOut)
    );
    assert_eq!(
        check_text(&p, &format!("account {sid}")),
        Err(Leak::KeptOut)
    );
    assert_eq!(
        check_text(&p, "see https://example.com"),
        Err(Leak::SecretLike)
    );
    assert_eq!(
        check_text(&p, "1 file failed: it could not be read."),
        Ok(())
    );

    let all = Disclosure {
        file_names: true,
        app_names: true,
        account_id: true,
    };
    let p = share_preview(&l, Summary::FinishedWithTasks, &[], Some(&c), sid, all);
    let names: Vec<&str> = p.lines.iter().filter_map(|l| l.name.as_deref()).collect();
    assert!(names.contains(&"Taxes 2025 [removed]"), "{names:?}");
    assert!(names.contains(&"notes [removed]"));
    assert!(names.contains(&"Secret Project Editor"));
    assert_eq!(p.account_id.as_deref(), Some(sid));
    let json = serde_json::to_string(&p).unwrap();
    for never in ["ada@example.com", "ghp_", "bea-diary"] {
        assert!(!json.contains(never));
    }
    assert_eq!(p.withheld.secrets, 2);
    // Chosen names may be in the text; another person's never.
    assert_eq!(
        check_text(
            &p,
            &format!("Secret Project Editor needs you. Account {sid}.")
        ),
        Ok(())
    );
    assert_eq!(check_text(&p, "bea-diary.txt"), Err(Leak::KeptOut));
    // Only file names, not app names.
    let files_only = Disclosure {
        file_names: true,
        ..Disclosure::default()
    };
    let p = share_preview(&l, Summary::FinishedWithTasks, &[], None, sid, files_only);
    let names: Vec<&str> = p.lines.iter().filter_map(|l| l.name.as_deref()).collect();
    assert!(!names.iter().any(|n| n.contains("Secret Project")));
    assert!(names.iter().any(|n| n.contains("Taxes 2025")));
    assert_eq!(check_text(&p, "Secret Project Editor"), Err(Leak::KeptOut));
    // A kept-out name inside a longer word is not that name.
    assert_eq!(check_text(&p, "Secret Project Editors club"), Ok(()));
}

/// The digest binds a save to exactly the report shown.
#[test]
fn the_report_digest_changes_with_what_is_disclosed() {
    let (_, l, c) = moved();
    let a = share_preview(
        &l,
        Summary::FinishedWithTasks,
        &[],
        Some(&c),
        "1000",
        Disclosure::default(),
    );
    let b = share_preview(
        &l,
        Summary::FinishedWithTasks,
        &[],
        Some(&c),
        "1000",
        Disclosure {
            app_names: true,
            ..Disclosure::default()
        },
    );
    assert_ne!(a.digest(), b.digest());
    let again = share_preview(
        &l,
        Summary::FinishedWithTasks,
        &[],
        Some(&c),
        "1000",
        Disclosure::default(),
    );
    assert_eq!(a.digest(), again.digest());
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
    assert_eq!(p.others_not_finished, 0);
    assert!(p.checks.is_empty());
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
    let r = run(&c);
    let p = share_preview(
        &l,
        Summary::FinishedWithTasks,
        &r,
        None,
        "1000",
        Disclosure::default(),
    );
    assert_eq!(
        p.checks,
        [pctwin_readiness::checks::Label::StillNeedsAttention]
    );
}
