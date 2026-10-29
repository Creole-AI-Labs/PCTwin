//! "Can I actually work?" checks (Task List 2.9; plan 2.7 section 4; acceptance T27-005 to
//! T27-007).

// Fixtures are written with std::fs; only the crate itself is fenced.
#![allow(clippy::disallowed_methods)]

mod common;

use std::path::{Path, PathBuf};

use common::*;
use pctwin_readiness::checks::{
    ChoiceError, Choices, Chosen, Damage, EffectApproval, ExternalEffect, Goal, Label, Missing,
    NotRun, Outcome, Summary, any_sample, person_answer, repair_requests, run, summarize,
};
use pctwin_readiness::ledger::{Coverage, Observed, Role};
use pctwin_record::ItemId;
use pctwin_types::{ItemResult, ItemStatus};

fn zip_bytes(comment: &[u8]) -> Vec<u8> {
    let name = b"a.txt";
    let data = b"hi";
    let mut z = Vec::new();
    // Local header.
    z.extend([0x50, 0x4b, 0x03, 0x04, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    z.extend(0u32.to_le_bytes()); // crc (not checked by an index check)
    z.extend(2u32.to_le_bytes());
    z.extend(2u32.to_le_bytes());
    z.extend((name.len() as u16).to_le_bytes());
    z.extend(0u16.to_le_bytes());
    z.extend(name);
    z.extend(data);
    let cd_offset = z.len() as u32;
    z.extend([0x50, 0x4b, 0x01, 0x02, 20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    z.extend(0u32.to_le_bytes());
    z.extend(2u32.to_le_bytes());
    z.extend(2u32.to_le_bytes());
    z.extend((name.len() as u16).to_le_bytes());
    z.extend([0u8; 12]); // extra, comment, disk, internal attrs, external attrs
    z.extend(0u32.to_le_bytes()); // local header offset
    z.extend(name);
    let cd_size = z.len() as u32 - cd_offset;
    z.extend([0x50, 0x4b, 0x05, 0x06, 0, 0, 0, 0, 1, 0, 1, 0]);
    z.extend(cd_size.to_le_bytes());
    z.extend(cd_offset.to_le_bytes());
    z.extend((comment.len() as u16).to_le_bytes());
    z.extend(comment);
    z
}

fn font_bytes(family: &str) -> Vec<u8> {
    let name: Vec<u8> = family.encode_utf16().flat_map(u16::to_be_bytes).collect();
    let mut table = Vec::new();
    table.extend(0u16.to_be_bytes()); // format
    table.extend(1u16.to_be_bytes()); // count
    table.extend(18u16.to_be_bytes()); // string offset
    for v in [3u16, 1, 0x409, 1, name.len() as u16, 0] {
        table.extend(v.to_be_bytes());
    }
    table.extend(&name);
    let mut f = Vec::new();
    f.extend(0x0001_0000u32.to_be_bytes());
    f.extend(1u16.to_be_bytes());
    f.extend([0u8; 6]);
    f.extend(*b"name");
    f.extend(0u32.to_be_bytes());
    f.extend(28u32.to_be_bytes());
    f.extend((table.len() as u32).to_be_bytes());
    f.extend(table);
    f
}

fn sqlite_bytes(page: u16, pages_in_header: u32, len: usize) -> Vec<u8> {
    let mut h = vec![0u8; len];
    h[..16].copy_from_slice(b"SQLite format 3\0");
    h[16..18].copy_from_slice(&page.to_be_bytes());
    h[18] = 1;
    h[19] = 1;
    h[21] = 64;
    h[22] = 32;
    h[23] = 32;
    h[24..28].copy_from_slice(&5u32.to_be_bytes());
    h[28..32].copy_from_slice(&pages_in_header.to_be_bytes());
    h[92..96].copy_from_slice(&5u32.to_be_bytes());
    h
}

fn id(n: &str) -> ItemId {
    file(n).id
}

fn pick(goals: Vec<Goal>) -> Choices {
    Choices::picked(
        1,
        goals
            .into_iter()
            .enumerate()
            .map(|(i, goal)| Chosen {
                item: id(&format!("item{i}")),
                owner: Role::Install,
                goal,
            })
            .collect(),
    )
    .unwrap()
}

fn only(choices: &Choices) -> Outcome {
    let r = run(choices, &own());
    assert_eq!(r.len(), 1);
    r[0].outcome.clone()
}

fn write(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, bytes).unwrap();
    p
}

#[test]
fn nothing_is_checked_unless_picked() {
    let c = Choices::picked(1, vec![]).unwrap();
    assert!(run(&c, &own()).is_empty());
    let many: Vec<Chosen> = (0..201)
        .map(|_| Chosen {
            item: id("x"),
            owner: Role::Data,
            goal: Goal::PersonCheck { task: "t".into() },
        })
        .collect();
    assert_eq!(Choices::picked(1, many).unwrap_err(), ChoiceError::TooMany);
}

/// T27-006: a check with an outside effect never runs on the move's approval; it needs its own
/// approval, for that very check and plan, and even then it is not built yet.
#[test]
fn t27_006_outside_effects_need_their_own_approval_and_are_not_run() {
    for effect in [
        ExternalEffect::SendMessage,
        ExternalEffect::Print,
        ExternalEffect::CallOnlineService,
        ExternalEffect::StartApp,
        ExternalEffect::RunMacro,
        ExternalEffect::UseCameraOrMicrophone,
        ExternalEffect::ConnectToNetworkService,
    ] {
        let mut c = pick(vec![
            Goal::External {
                effect,
                task: "Send a test mail".into(),
            },
            Goal::PersonCheck {
                task: "Open the budget".into(),
            },
        ]);
        let first = &run(&c, &own())[0];
        assert_eq!(
            first.outcome,
            Outcome::NotRun {
                why: NotRun::NeedsItsOwnApproval
            }
        );
        assert_eq!(first.outcome.label(), Label::StillNeedsAttention);
        // Nothing to approve on a plain check; an approval for an older plan; one made for
        // another person's different list of checks.
        assert_eq!(
            EffectApproval::from_person(&c, 1, 1).unwrap_err(),
            ChoiceError::NothingToApprove
        );
        let old = EffectApproval::from_person(&c, 0, 0).unwrap();
        assert_eq!(c.approve_effect(old), Err(ChoiceError::StalePlan));
        let other = pick(vec![Goal::External {
            effect,
            task: "Print a page".into(),
        }]);
        let foreign = EffectApproval::from_person(&other, 0, 1).unwrap();
        assert_eq!(c.approve_effect(foreign), Err(ChoiceError::WrongKind));
        assert_eq!(
            run(&c, &own())[0].outcome,
            Outcome::NotRun {
                why: NotRun::NeedsItsOwnApproval
            }
        );
        let ok = EffectApproval::from_person(&c, 0, 1).unwrap();
        c.approve_effect(ok).unwrap();
        assert_eq!(
            run(&c, &own())[0].outcome,
            Outcome::NotRun {
                why: NotRun::NotBuiltYet
            }
        );
        // The other check is untouched by that approval.
        assert_eq!(
            run(&c, &own())[1].outcome,
            Outcome::NotRun {
                why: NotRun::WaitingForPerson
            }
        );
    }
}

#[test]
fn linked_files_are_looked_up_inside_the_project_only() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.txt", b"x");
    let links = |l: Vec<&str>| Goal::LinkedFiles {
        root: dir.path().to_path_buf(),
        links: l.into_iter().map(PathBuf::from).collect(),
    };
    assert_eq!(
        only(&pick(vec![links(vec!["a.txt"])])),
        Outcome::MachineChecked {
            coverage: Coverage::Whole
        }
    );
    assert_eq!(
        only(&pick(vec![links(vec!["a.txt", "media/clip.mov"])])),
        Outcome::NeedsAttention {
            missing: vec![Missing::LinkedFile {
                name: PathBuf::from("media/clip.mov")
                    .to_string_lossy()
                    .into_owned()
            }],
            coverage: Coverage::Whole
        }
    );
    let abs = dir.path().join("a.txt");
    for bad in [vec!["../a.txt"], vec![abs.to_str().unwrap()], vec![""]] {
        assert_eq!(
            only(&pick(vec![links(bad)])),
            Outcome::NotRun {
                why: NotRun::OutsideTheProject
            }
        );
    }
    // More links than one check looks up: the coverage says so.
    let many = vec!["a.txt"; 10_001];
    assert_eq!(
        only(&pick(vec![links(many)])),
        Outcome::MachineChecked {
            coverage: Coverage::Sample {
                checked: 10_000,
                of: 10_001
            }
        }
    );
    assert!(any_sample(&run(
        &pick(vec![links(vec!["a.txt"; 10_001])]),
        &own()
    )));
    assert!(!any_sample(&run(&pick(vec![links(vec!["a.txt"])]), &own())));
    // Fresh-context review F3: a machine check of only part is never labelled machine-checked.
    assert_eq!(
        only(&pick(vec![links(vec!["a.txt"; 10_001])])).label(),
        Label::StillNeedsAttention
    );
    assert_eq!(
        only(&pick(vec![links(vec!["a.txt"])])).label(),
        Label::MachineChecked
    );
}

/// Fresh-context review F3: a link (or junction) whose target is gone is a missing file; one
/// whose target is there is found.
#[test]
fn a_link_whose_target_is_gone_is_missing() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("real")).unwrap();
    std::fs::create_dir(dir.path().join("gone")).unwrap();
    // Windows: junctions, which every account can make (file links need Developer Mode).
    #[cfg(windows)]
    for (link, to) in [("good.txt", "real"), ("dangling.txt", "gone")] {
        let made = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(dir.path().join(link))
            .arg(dir.path().join(to))
            .output()
            .unwrap();
        assert!(made.status.success(), "{made:?}");
    }
    #[cfg(unix)]
    for (link, to) in [("good.txt", "real"), ("dangling.txt", "gone")] {
        std::os::unix::fs::symlink(dir.path().join(to), dir.path().join(link)).unwrap();
    }
    // Its target goes: the link is left pointing at nothing.
    std::fs::remove_dir(dir.path().join("gone")).unwrap();
    assert!(std::fs::symlink_metadata(dir.path().join("dangling.txt")).is_ok());
    let links = Goal::LinkedFiles {
        root: dir.path().to_path_buf(),
        links: vec![PathBuf::from("good.txt"), PathBuf::from("dangling.txt")],
    };
    assert_eq!(
        only(&pick(vec![links])),
        Outcome::NeedsAttention {
            missing: vec![Missing::LinkedFile {
                name: "dangling.txt".into()
            }],
            coverage: Coverage::Whole
        }
    );
}

/// A font whose family name comes after other names (a copyright line first) is still read.
#[test]
fn a_family_name_after_other_names_is_still_read() {
    let copyright: Vec<u8> = "(c) Ada"
        .encode_utf16()
        .flat_map(u16::to_be_bytes)
        .collect();
    let family: Vec<u8> = "Later Sans"
        .encode_utf16()
        .flat_map(u16::to_be_bytes)
        .collect();
    let mut table = Vec::new();
    table.extend(0u16.to_be_bytes()); // format
    table.extend(2u16.to_be_bytes()); // count
    table.extend(30u16.to_be_bytes()); // string offset: 6 + 2 * 12
    for v in [3u16, 1, 0x409, 0, copyright.len() as u16, 0] {
        table.extend(v.to_be_bytes());
    }
    for v in [
        3u16,
        1,
        0x409,
        1,
        family.len() as u16,
        copyright.len() as u16,
    ] {
        table.extend(v.to_be_bytes());
    }
    table.extend(&copyright);
    table.extend(&family);
    let mut f = Vec::new();
    f.extend(0x0001_0000u32.to_be_bytes());
    f.extend(1u16.to_be_bytes());
    f.extend([0u8; 6]);
    f.extend(*b"name");
    f.extend(0u32.to_be_bytes());
    f.extend(28u32.to_be_bytes());
    f.extend((table.len() as u32).to_be_bytes());
    f.extend(table);
    let dir = tempfile::tempdir().unwrap();
    let path = write(dir.path(), "Later.ttf", &f);
    assert_eq!(families(&path), ["Later Sans"]);
}

/// A font is found by the family name inside the font file, never guessed from the file name.
#[test]
fn a_font_is_found_by_its_own_name_table() {
    let dir = tempfile::tempdir().unwrap();
    let f = write(dir.path(), "MyFont-Regular.ttf", &font_bytes("Plain Sans"));
    write(dir.path(), "notes.txt", b"Plain Sans");
    assert_eq!(families(&f), ["Plain Sans"]);
    let font = |family: &str| Goal::Font {
        family: family.into(),
        folders: vec![dir.path().to_path_buf()],
    };
    assert_eq!(
        only(&pick(vec![font("plain-sans")])),
        Outcome::MachineChecked {
            coverage: Coverage::Whole
        }
    );
    assert_eq!(
        only(&pick(vec![font("MyFont")])),
        Outcome::NeedsAttention {
            missing: vec![Missing::Font {
                family: "MyFont".into(),
                not_checked: vec![]
            }],
            coverage: Coverage::Whole
        }
    );
    // A file with a font's name that is not a font, and a cut-short font, name nothing.
    let junk = write(dir.path(), "junk.otf", b"not a font at all");
    assert!(families(&junk).is_empty());
    let mut cut = font_bytes("Plain Sans");
    cut.truncate(40);
    let cut = write(dir.path(), "cut.ttf", &cut);
    assert!(families(&cut).is_empty());
}

#[test]
fn an_archive_index_is_checked_without_unpacking() {
    let dir = tempfile::tempdir().unwrap();
    let check = |bytes: &[u8]| {
        let p = write(dir.path(), "a.zip", bytes);
        only(&pick(vec![Goal::ArchiveOpens { file: p }]))
    };
    let ok = Outcome::MachineChecked {
        coverage: Coverage::Whole,
    };
    let damaged = |what| Outcome::NeedsAttention {
        missing: vec![Missing::Damaged { what }],
        coverage: Coverage::Whole,
    };
    assert_eq!(check(&zip_bytes(b"")), ok);
    assert_eq!(check(&zip_bytes(b"a comment")), ok);
    // Cut short, or something added after the end record.
    let z = zip_bytes(b"");
    assert_eq!(check(&z[..z.len() - 1]), damaged(Damage::NoArchiveEnd));
    let mut extra = zip_bytes(b"");
    extra.push(0);
    assert_eq!(check(&extra), damaged(Damage::NoArchiveEnd));
    assert_eq!(check(b"PK"), damaged(Damage::NoArchiveEnd));
    // The index points outside the file, or not at the list of files.
    let mut z = zip_bytes(b"");
    let n = z.len();
    z[n - 6..n - 2].copy_from_slice(&1000u32.to_le_bytes());
    assert_eq!(check(&z), damaged(Damage::ArchiveIndexOutside));
    let mut z = zip_bytes(b"");
    let n = z.len();
    z[n - 6..n - 2].copy_from_slice(&1u32.to_le_bytes());
    assert_eq!(check(&z), damaged(Damage::ArchiveIndexOutside));
    // One part of several.
    let mut z = zip_bytes(b"");
    let n = z.len();
    z[n - 18] = 1;
    assert_eq!(check(&z), damaged(Damage::ArchiveInParts));
    let mut z = zip_bytes(b"");
    let n = z.len();
    z[n - 14] = 2;
    assert_eq!(check(&z), damaged(Damage::ArchiveInParts));
    // An empty archive is whole.
    let mut empty = vec![0x50, 0x4b, 0x05, 0x06];
    empty.extend([0u8; 18]);
    assert_eq!(check(&empty), ok);
    // A ZIP64 archive needs its locator just before the end record.
    let mut z64 = vec![0u8; 4];
    z64.extend([0x50, 0x4b, 0x06, 0x07]);
    z64.extend([0u8; 16]);
    z64.extend([0x50, 0x4b, 0x05, 0x06, 0, 0, 0, 0, 0xff, 0xff, 0xff, 0xff]);
    z64.extend([0xff; 8]);
    z64.extend([0, 0]);
    assert_eq!(check(&z64), ok);
    let mut no_loc = z64.clone();
    no_loc[4] = 0;
    assert_eq!(check(&no_loc), damaged(Damage::NoArchiveEnd));
    // Not there at all.
    assert_eq!(
        only(&pick(vec![Goal::ArchiveOpens {
            file: dir.path().join("gone.zip")
        }])),
        Outcome::NeedsAttention {
            missing: vec![Missing::NotThere],
            coverage: Coverage::Whole
        }
    );
}

#[test]
fn a_database_header_is_checked_without_opening_it() {
    let dir = tempfile::tempdir().unwrap();
    let check = |bytes: &[u8]| {
        let p = write(dir.path(), "db.sqlite", bytes);
        only(&pick(vec![Goal::DatabaseOpens { file: p }]))
    };
    let ok = Outcome::MachineChecked {
        coverage: Coverage::Whole,
    };
    let bad = Outcome::NeedsAttention {
        missing: vec![Missing::Damaged {
            what: Damage::DatabaseHeader,
        }],
        coverage: Coverage::Whole,
    };
    assert_eq!(check(&sqlite_bytes(4096, 1, 4096)), ok);
    assert_eq!(check(&sqlite_bytes(4096, 2, 8192)), ok);
    assert_eq!(check(&sqlite_bytes(512, 1, 512)), ok);
    assert_eq!(check(&sqlite_bytes(32_768, 1, 32_768)), ok);
    assert_eq!(check(&sqlite_bytes(1, 1, 65_536)), ok);
    assert_eq!(check(&sqlite_bytes(1000, 1, 1000)), bad);
    assert_eq!(check(&sqlite_bytes(256, 1, 512)), bad);
    assert_eq!(check(&sqlite_bytes(4096, 1, 4000)), bad);
    assert_eq!(check(&sqlite_bytes(4096, 3, 8192)), bad);
    // The page count is not in force when its "valid for" number does not match.
    let mut stale = sqlite_bytes(4096, 3, 8192);
    stale[92..96].copy_from_slice(&4u32.to_be_bytes());
    assert_eq!(check(&stale), ok);
    for (at, v) in [(18, 3u8), (19, 0), (21, 63), (22, 31), (23, 33)] {
        let mut b = sqlite_bytes(4096, 1, 4096);
        b[at] = v;
        assert_eq!(check(&b), bad, "byte {at}");
    }
    let mut not = sqlite_bytes(4096, 1, 4096);
    not[0] = b's';
    assert_eq!(
        check(&not),
        Outcome::NeedsAttention {
            missing: vec![Missing::Damaged {
                what: Damage::NotADatabase
            }],
            coverage: Coverage::Whole
        }
    );
    assert_eq!(
        check(b"SQLite format 3\0"),
        Outcome::NeedsAttention {
            missing: vec![Missing::Damaged {
                what: Damage::NotADatabase
            }],
            coverage: Coverage::Whole
        }
    );
}

/// The checks only read: nothing they look at changes.
#[test]
fn checks_change_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let z = write(dir.path(), "a.zip", &zip_bytes(b""));
    let d = write(dir.path(), "db", &sqlite_bytes(4096, 1, 4096));
    let f = write(dir.path(), "f.ttf", &font_bytes("Plain Sans"));
    let before: Vec<_> = [&z, &d, &f]
        .iter()
        .map(|p| {
            (
                std::fs::read(p).unwrap(),
                std::fs::metadata(p).unwrap().modified().unwrap(),
            )
        })
        .collect();
    let names_before = std::fs::read_dir(dir.path()).unwrap().count();
    run(
        &pick(vec![
            Goal::ArchiveOpens { file: z.clone() },
            Goal::DatabaseOpens { file: d.clone() },
            Goal::Font {
                family: "Plain Sans".into(),
                folders: vec![dir.path().to_path_buf()],
            },
            Goal::LinkedFiles {
                root: dir.path().to_path_buf(),
                links: vec!["a.zip".into(), "missing".into()],
            },
        ]),
        &own(),
    );
    let after: Vec<_> = [&z, &d, &f]
        .iter()
        .map(|p| {
            (
                std::fs::read(p).unwrap(),
                std::fs::metadata(p).unwrap().modified().unwrap(),
            )
        })
        .collect();
    assert_eq!(before, after);
    let names_after = std::fs::read_dir(dir.path()).unwrap().count();
    assert_eq!(names_before, names_after);
}

#[test]
fn the_person_s_own_check_is_labelled_person_confirmed() {
    let c = pick(vec![
        Goal::PersonCheck {
            task: "Open three photos".into(),
        },
        Goal::ArchiveOpens {
            file: PathBuf::from("x"),
        },
    ]);
    assert_eq!(
        run(&c, &own())[0].outcome,
        Outcome::NotRun {
            why: NotRun::WaitingForPerson
        }
    );
    let sample = Coverage::sample(3, 900).unwrap();
    let yes = person_answer(&c, 0, 1, true, sample).unwrap();
    assert_eq!(yes.outcome, Outcome::PersonConfirmed { coverage: sample });
    assert_eq!(yes.outcome.label(), Label::PersonConfirmed);
    assert!(any_sample(std::slice::from_ref(&yes)));
    let no = person_answer(&c, 0, 1, false, Coverage::Whole).unwrap();
    assert_eq!(no.outcome.label(), Label::StillNeedsAttention);
    assert_eq!(
        person_answer(&c, 0, 2, true, Coverage::Whole).unwrap_err(),
        ChoiceError::StalePlan
    );
    assert_eq!(
        person_answer(&c, 1, 1, true, Coverage::Whole).unwrap_err(),
        ChoiceError::WrongKind
    );
    assert_eq!(
        person_answer(&c, 2, 1, true, Coverage::Whole).unwrap_err(),
        ChoiceError::NoSuchCheck
    );
    // "It did not work" goes back to the part that owns the item.
    let req = repair_requests(&c, &[no]);
    assert_eq!(req.len(), 1);
    assert_eq!(
        (req[0].to, &req[0].need),
        (Role::Install, &Missing::PersonSaidNo)
    );
}

/// T27-007: what a check finds missing becomes a focused repair request to its owner.
#[test]
fn t27_007_missing_dependencies_go_to_their_owner() {
    let dir = tempfile::tempdir().unwrap();
    let c = Choices::picked(
        1,
        vec![
            Chosen {
                item: id("project"),
                owner: Role::Install,
                goal: Goal::LinkedFiles {
                    root: dir.path().to_path_buf(),
                    links: vec!["clip.mov".into()],
                },
            },
            Chosen {
                item: id("poster"),
                owner: Role::Install,
                goal: Goal::Font {
                    family: "Plain Sans".into(),
                    folders: vec![dir.path().to_path_buf()],
                },
            },
            Chosen {
                item: id("db"),
                owner: Role::Settings,
                goal: Goal::DatabaseOpens {
                    file: write(dir.path(), "db", b"nope"),
                },
            },
        ],
    )
    .unwrap();
    let results = run(&c, &own());
    let req = repair_requests(&c, &results);
    let got: Vec<(ItemId, Role)> = req.iter().map(|r| (r.item, r.to)).collect();
    assert_eq!(
        got,
        [
            (id("project"), Role::Data),
            (id("poster"), Role::Data),
            (id("db"), Role::Settings)
        ]
    );
    // Results of another plan, or naming another item, ask for nothing.
    let mut stale = results.clone();
    stale[0].generation = 0;
    stale[1].item = id("other");
    assert_eq!(repair_requests(&c, &stale).len(), 1);
}

/// T27-005: an app that launched does not make the work ready while a picked check fails.
#[test]
fn t27_005_ready_only_when_every_picked_check_passed() {
    let ed = app("Editor");
    let doc = file("budget.xlsx");
    let (_, mut l) = ledger(vec![ed.clone(), doc.clone()]);
    let ok = Sees(Observed::Matches);
    let mut r = receipt(
        &l,
        Role::Install,
        ed.id,
        Some(ItemResult::done(ItemStatus::Reinstalled).unwrap()),
    );
    r.evidence = Some(pctwin_readiness::ledger::Evidence::Installed {
        package: "Editor".into(),
        version: "5.1".into(),
    });
    l.accept(r, &ok, T0 + 20).unwrap();
    assert_eq!(summarize(&l, &[]), Summary::FinishedWithTasks);
    l.accept(
        receipt(
            &l,
            Role::Transfer,
            doc.id,
            Some(ItemResult::done(ItemStatus::Copied).unwrap()),
        ),
        &ok,
        T0 + 20,
    )
    .unwrap();
    assert_eq!(summarize(&l, &[]), Summary::DataPreserved);
    let c = Choices::picked(
        l.generation(),
        vec![Chosen {
            item: doc.id,
            owner: Role::Data,
            goal: Goal::PersonCheck {
                task: "Open the budget with its links".into(),
            },
        }],
    )
    .unwrap();
    let failed = person_answer(&c, 0, l.generation(), false, Coverage::Whole).unwrap();
    assert_eq!(
        summarize(&l, std::slice::from_ref(&failed)),
        Summary::FinishedWithTasks
    );
    let waiting = &run(&c, &own())[0];
    assert_eq!(
        summarize(&l, std::slice::from_ref(waiting)),
        Summary::FinishedWithTasks
    );
    let passed = person_answer(&c, 0, l.generation(), true, Coverage::Whole).unwrap();
    assert_eq!(
        summarize(&l, std::slice::from_ref(&passed)),
        Summary::ReadyForSelectedWork
    );
    // A failed check of an older plan does not count for this one.
    let mut old = failed.clone();
    old.generation -= 1;
    assert_eq!(
        summarize(&l, &[old, passed.clone()]),
        Summary::ReadyForSelectedWork
    );
    // A key that is not checked keeps it from ready.
    l.needs_key(doc.id).unwrap();
    assert_eq!(summarize(&l, &[passed]), Summary::FinishedWithTasks);
}

/// ZIP64: any one of the three fields at its largest value calls for the ZIP64 locator.
#[test]
fn any_zip64_field_needs_the_locator() {
    let dir = tempfile::tempdir().unwrap();
    for field in 0..3 {
        for locator in [true, false] {
            let mut z = vec![0u8; 4];
            z.extend(if locator {
                [0x50, 0x4b, 0x06, 0x07]
            } else {
                [0, 0, 0, 0]
            });
            z.extend([0u8; 16]);
            let mut end = vec![0x50, 0x4b, 0x05, 0x06, 0, 0, 0, 0, 1, 0, 1, 0];
            end.extend(0u32.to_le_bytes());
            end.extend(0u32.to_le_bytes());
            end.extend([0, 0]);
            match field {
                0 => end[10..12].copy_from_slice(&[0xff, 0xff]),
                1 => end[12..16].copy_from_slice(&[0xff; 4]),
                _ => end[16..20].copy_from_slice(&[0xff; 4]),
            }
            if field == 0 {
                end[8..10].copy_from_slice(&[0xff, 0xff]);
            }
            z.extend(end);
            let p = write(dir.path(), "z64.zip", &z);
            let got = only(&pick(vec![Goal::ArchiveOpens { file: p }]));
            let want = if locator {
                Outcome::MachineChecked {
                    coverage: Coverage::Whole,
                }
            } else {
                Outcome::NeedsAttention {
                    missing: vec![Missing::Damaged {
                        what: Damage::NoArchiveEnd,
                    }],
                    coverage: Coverage::Whole,
                }
            };
            assert_eq!(got, want, "field {field} locator {locator}");
        }
    }
}

/// Phase 2 adoption, changes 7 and 8: a font is told by what it holds, never its name; a font
/// file of a kind PCTwin cannot read is found, not checked, and said so (never "not installed"
/// for certain); a kind added only as a signed row is named with no code change.
#[test]
fn font_files_are_found_by_what_they_hold_and_unread_kinds_are_said() {
    use pctwin_catalogue::{Catalogue, test_key};
    use pctwin_readiness::checks::{FontFileRow, font_with, shipped_font_files};
    assert!(shipped_font_files().report().is_clean());
    let dir = tempfile::tempdir().unwrap();
    // A real font under a name that says nothing.
    let odd = write(dir.path(), "font-cache.dat", &font_bytes("Odd Name Sans"));
    assert!(is_font(&odd));
    let folders = vec![dir.path().to_path_buf()];
    assert_eq!(
        font_with(shipped_font_files(), "Odd Name Sans", &folders, &own()),
        Outcome::MachineChecked {
            coverage: Coverage::Whole
        }
    );
    // A web font and a notes file beside it: the web font is found, not checked.
    write(dir.path(), "Brand.woff2", b"wOF2....");
    write(dir.path(), "Brand2.WOFF2", b"wOF2....");
    write(dir.path(), "readme.txt", b"hello");
    write(dir.path(), "x.xfont", b"XF");
    // A folder named like a font is not a font file.
    std::fs::create_dir(dir.path().join("Folder.woff2")).unwrap();
    assert_eq!(
        font_with(shipped_font_files(), "Brand", &folders, &own()),
        Outcome::NeedsAttention {
            missing: vec![Missing::Font {
                family: "Brand".into(),
                not_checked: vec![pctwin_readiness::checks::FontNotChecked {
                    ext: "woff2".into(),
                    name: Some("Web font".into()),
                    why: pctwin_readiness::checks::FontWhy::KindNotRead,
                }]
            }],
            coverage: Coverage::Sample { checked: 1, of: 3 }
        }
    );
    // Only fonts PCTwin cannot read: not run, never "not installed".
    let only_web = tempfile::tempdir().unwrap();
    write(only_web.path(), "a.woff", b"wOFF");
    assert_eq!(
        font_with(
            shipped_font_files(),
            "Brand",
            &[only_web.path().to_path_buf()],
            &own()
        ),
        Outcome::NotRun {
            why: NotRun::CouldNotRead
        }
    );
    // A new kind, added only as a signed row, is named.
    let bytes = br#"{"format":1,"catalogue":"font-files","version":2,"rows":[
        {"extension":"xfont","name":"X font"},
        {"extension":"X.Y","name":"bad"},
        {"extension":"ok","name":" "},
        {"extension":"","name":"Empty"},
        {"extension":"abcdefghijklmnopq","name":"Long"}
    ]}"#;
    let sig = test_key::sign(bytes, "font-files", 2);
    let rows = Catalogue::<FontFileRow>::signed(
        bytes,
        &sig,
        &test_key::keys(),
        &pctwin_catalogue::MemoryMarks::at(1),
    )
    .unwrap();
    assert_eq!(rows.rows().len(), 1);
    assert_eq!(rows.report().skipped.len(), 4);
    assert_eq!(
        font_with(&rows, "Brand", &folders, &own()),
        Outcome::NeedsAttention {
            missing: vec![Missing::Font {
                family: "Brand".into(),
                not_checked: vec![pctwin_readiness::checks::FontNotChecked {
                    ext: "xfont".into(),
                    name: Some("X font".into()),
                    why: pctwin_readiness::checks::FontWhy::KindNotRead,
                }]
            }],
            coverage: Coverage::Sample { checked: 1, of: 2 }
        }
    );
    // Nothing at all in the folders: not installed there.
    let empty = tempfile::tempdir().unwrap();
    assert!(matches!(
        font_with(
            shipped_font_files(),
            "Brand",
            &[empty.path().to_path_buf()],
            &own()
        ),
        Outcome::NeedsAttention {
            coverage: Coverage::Whole,
            ..
        }
    ));
}

/// Fourth review, finding F5: a damaged `.ttf` alone in the folder made the check say it had
/// looked at everything and the family was not installed. A file of a kind PCTwin reads that is
/// not whole, and a font whose names cannot be read, are found, not checked, and said so.
#[test]
fn f5_a_font_file_that_cannot_be_read_is_never_counted_as_checked() {
    use pctwin_readiness::checks::{FontNotChecked, FontWhy, font_with, shipped_font_files};
    let only = tempfile::tempdir().unwrap();
    write(only.path(), "Family.ttf", b"\0\0\0\0garbage");
    write(only.path(), "Family.xyzfont", b"zz");
    assert_eq!(
        font_with(
            shipped_font_files(),
            "Family",
            &[only.path().to_path_buf()],
            &own()
        ),
        Outcome::NotRun {
            why: NotRun::CouldNotRead
        }
    );
    // A font by its first bytes whose name table is missing, beside one PCTwin reads.
    let two = tempfile::tempdir().unwrap();
    write(two.path(), "Other.ttf", &font_bytes("Other Sans"));
    write(two.path(), "Broken.otf", b"OTTO\0\0\0\0\0\0\0\0");
    write(two.path(), "Bad.ttf", b"not a font");
    assert_eq!(
        font_with(
            shipped_font_files(),
            "Family",
            &[two.path().to_path_buf()],
            &own()
        ),
        Outcome::NeedsAttention {
            missing: vec![Missing::Font {
                family: "Family".into(),
                not_checked: vec![
                    FontNotChecked {
                        ext: "otf".into(),
                        name: None,
                        why: FontWhy::NamesUnreadable,
                    },
                    FontNotChecked {
                        ext: "ttf".into(),
                        name: Some("TrueType font".into()),
                        why: FontWhy::Damaged,
                    },
                ]
            }],
            coverage: Coverage::Sample { checked: 1, of: 3 }
        }
    );
}

/// PCTwin's own readers, run in the test's own process (only a parser crate's tests may).
fn own() -> pctwin_readiness::checks::InThisProcess {
    pctwin_readiness::checks::InThisProcess::new(pctwin_parse_seal::in_child())
}

fn names(path: &std::path::Path) -> pctwin_readiness::checks::FontNames {
    let mut f = std::fs::File::open(path).unwrap();
    let size = f.metadata().unwrap().len();
    pctwin_readiness::checks::font_names(pctwin_parse_seal::in_child(), &mut f, size)
}

fn families(path: &std::path::Path) -> Vec<String> {
    names(path).families
}

fn is_font(path: &std::path::Path) -> bool {
    names(path).is_font
}
