//! The example files the screens are tested against (app/src/contract/*.json): one value of every
//! shape that crosses the bridge, as the engine really serialises it. The screens' own test
//! (contract.test.ts) parses each one with the same functions bridge.ts uses, so a change on
//! either side that the other does not follow fails a test.
//!
//! Each enum that crosses the bridge is declared with the names of all its variants
//! (`pctwin_types::listed_enum!`), and a test below requires every one of them in the examples, each
//! with its own tag on the wire. A new variant is in that list the moment it exists, so it cannot be
//! left out of the files.
//!
//! The files must match: `cargo test -p pctwin-app contract` fails when they differ, and
//! `PCTWIN_WRITE_CONTRACT=1 cargo test -p pctwin-app contract` writes them.

use std::collections::BTreeSet;
use std::path::PathBuf;

use pctwin_discovery::Label;
use pctwin_gate::{CheckedFrom, WhyMemory};
use pctwin_journey::Problem;
use serde_json::Value;

use pctwin_record::{FolderRole, LeftOutReason};
use pctwin_types::{ItemStatus, Reason};

use crate::moving::{
    ByStatus, Bytes, Counts, Current, EndedEarly, Evidence, GroupName, Laptops, LeftOut, MoveGroup,
    MoveOutcome, MoveProblem, MoveProgress, Notice, OthersWhy, PlanGroup, PlanView, PreviewView,
    Range, ReportItem, RunState, Size, Stage,
};
use crate::pairing::{HeadsUp, LabelView, NearbyView, NewView, OldView, ProblemView};

fn picked() -> LabelView {
    // Colour 4 and animal 0 are "blue" and "ant".
    LabelView::from(&Label::Picked {
        colour: 4,
        animal: 0,
    })
}

fn named() -> LabelView {
    LabelView::from(&Label::named("Kitchen laptop").unwrap())
}

fn labels() -> Vec<LabelView> {
    vec![picked(), named()]
}

fn choose_network() -> ProblemView {
    Problem::ChooseNetwork(vec!["Ethernet".into(), "Wi-Fi".into()]).into()
}

fn old_views() -> Vec<OldView> {
    vec![
        OldView::Label { label: picked() },
        OldView::Code {
            code: "123456".into(),
            seconds_left: 42,
        },
        OldView::Wait { seconds_left: 30 },
        OldView::Locked,
        OldView::Choose {
            round: 3,
            numbers: [11, 42, 87],
        },
        OldView::Paired,
        OldView::TimedOut,
        OldView::Left,
        OldView::Lost,
        OldView::Label { label: named() },
    ]
}

fn new_views() -> Vec<NewView> {
    vec![
        NewView::Paired,
        NewView::Problem {
            problem: Problem::WrongCode.into(),
        },
        NewView::Lost,
        NewView::Problem {
            problem: choose_network(),
        },
    ]
}

/// Every problem the journey has (`Problem::all()`, which its test checks against the variants
/// made with the enum), as a rejected command carries it; choosing a network comes with networks
/// to choose from.
fn problems() -> Vec<ProblemView> {
    Problem::all()
        .into_iter()
        .map(|p| match p {
            Problem::ChooseNetwork(_) => choose_network(),
            p => p.into(),
        })
        .collect()
}

fn nearby() -> Vec<NearbyView> {
    vec![
        NearbyView {
            index: 0,
            label: picked(),
            same_label_nearby: false,
            needs_update: false,
        },
        NearbyView {
            index: 1,
            label: named(),
            same_label_nearby: true,
            needs_update: false,
        },
        NearbyView {
            index: 2,
            label: picked(),
            same_label_nearby: false,
            needs_update: true,
        },
    ]
}

fn heads_ups() -> Vec<HeadsUp> {
    vec![HeadsUp::LocalNetwork, HeadsUp::Firewall]
}

fn move_problems() -> Vec<MoveProblem> {
    vec![
        MoveProblem::NotPaired,
        MoveProblem::Busy,
        MoveProblem::NotMoving,
        MoveProblem::PlanChanged,
        MoveProblem::Failed,
    ]
}

fn stages() -> Vec<Stage> {
    vec![
        Stage::Scanning,
        Stage::Waiting,
        Stage::Moving,
        Stage::Administrator,
        Stage::OthersNotMoved {
            why: OthersWhy::NotApproved,
        },
        Stage::OthersNotMoved {
            why: OthersWhy::Failed,
        },
        Stage::OthersNotMoved {
            why: OthersWhy::NotHere,
        },
    ]
}

fn endings() -> Vec<EndedEarly> {
    vec![
        EndedEarly::LinkLost {
            detail: "the connection was lost".into(),
        },
        EndedEarly::NotApproved,
        EndedEarly::Failed {
            detail: "the journal could not be opened".into(),
        },
        EndedEarly::UpdateHere,
        EndedEarly::UpdateThere,
    ]
}

fn peers() -> Vec<crate::moving::Peer> {
    use pctwin_move::greeting::{About, Power, System};
    let about = |system, chip: &str, power| About {
        system,
        chip: chip.into(),
        app: "0.1.0".into(),
        power,
    };
    vec![
        crate::moving::Peer {
            mine: about(System::Mac, "aarch64", Power::Plugged),
            theirs: about(System::Windows, "x86_64", Power::Battery { percent: 34 }),
        },
        crate::moving::Peer {
            mine: about(System::Linux, "x86_64", Power::Unknown),
            theirs: about(System::Other, "riscv64", Power::Plugged),
        },
    ]
}

fn plans() -> Vec<PlanView> {
    vec![PlanView {
        revision: 2,
        fingerprint: "9f".repeat(32),
        groups: vec![PlanGroup {
            id: "documents".into(),
            role: FolderRole::Documents,
            path: String::new(),
            size: Size::Known {
                bytes: 1_200_000,
                files: 34,
            },
            to_drive: String::new(),
            to_folder: "documents".into(),
        }],
        left_out: vec![LeftOut {
            reason: LeftOutReason::CloudOnly,
            count: 3,
        }],
        preview: PreviewView {
            fits: Some(true),
            short_bytes: 0,
            minutes: Some(Range { low: 4, high: 8 }),
            renamed: 1,
            too_big: 0,
            refused: 0,
            runs_at_sign_in: 1,
        },
    }]
}

fn quiet(state: RunState, old: Option<LabelView>) -> MoveProgress {
    MoveProgress {
        state,
        laptops: Laptops { old },
        groups: Vec::new(),
        counts: Counts::default(),
        bytes: Bytes {
            checked: 0,
            total: 0,
        },
        current: None,
        speed: None,
        minutes_left: None,
        notice: None,
    }
}

fn progress() -> Vec<MoveProgress> {
    let counts = Counts {
        total: 34,
        waiting: 20,
        copying: 1,
        checking: 1,
        done: 10,
        needs_action: 1,
        failed: 1,
        skipped_by_user: 0,
    };
    vec![
        MoveProgress {
            state: RunState::Running,
            laptops: Laptops {
                old: Some(picked()),
            },
            groups: vec![
                MoveGroup {
                    id: "documents".into(),
                    name: GroupName::Role {
                        role: FolderRole::Documents,
                    },
                    counts,
                },
                MoveGroup {
                    id: "for-1".into(),
                    name: GroupName::Folder {
                        name: "for-1".into(),
                    },
                    counts: Counts::default(),
                },
            ],
            counts,
            bytes: Bytes {
                checked: 400_000,
                total: 1_200_000,
            },
            current: Some(Current {
                name: "letter.txt".into(),
                step: "checking",
            }),
            speed: Some(Range {
                low: 40_000_000,
                high: 40_000_000,
            }),
            minutes_left: Some(Range { low: 3, high: 3 }),
            notice: Some(Notice::DriveProblem {
                place: GroupName::Role {
                    role: FolderRole::Pictures,
                },
            }),
        },
        MoveProgress {
            notice: Some(Notice::LowBattery { paused: true }),
            ..quiet(RunState::Pausing, None)
        },
        quiet(RunState::Paused, Some(named())),
    ]
}

fn outcomes() -> Vec<MoveOutcome> {
    vec![MoveOutcome {
        people: vec![crate::moving::PersonOutcome {
            name: "Bea".into(),
            copied: 120,
            needs_action: 0,
            failed: 1,
            at_sign_in: true,
        }],
        by_status: ByStatus {
            copied: 30,
            reinstalled: 0,
            converted: 0,
            needs_action: 1,
            skipped_by_user: 1,
            failed: 2,
        },
        not_copied: 0,
        bytes_checked: 1_100_000,
        stopped: false,
        checked_from_memory: 3,
    }]
}

fn report_items() -> Vec<ReportItem> {
    let line = |n: &str, status, reason, evidence| ReportItem {
        id: format!("{n:0>32}"),
        name: format!("{n}.txt"),
        source: format!("documents/{n}.txt"),
        destination: (status != ItemStatus::Failed).then(|| format!("{n}.txt")),
        status,
        reason,
        evidence,
    };
    vec![
        line("1", ItemStatus::Copied, None, Some(Evidence::Drive)),
        line(
            "2",
            ItemStatus::Copied,
            None,
            Some(Evidence::Memory {
                why: WhyMemory::TooSmall,
            }),
        ),
        line("6", ItemStatus::Copied, None, Some(Evidence::AlreadyThere)),
        line("7", ItemStatus::Copied, None, Some(Evidence::Recovered)),
        line("8", ItemStatus::Copied, None, Some(Evidence::NewLaptop)),
        line("3", ItemStatus::Failed, Some(Reason::Unreadable), None),
        line("4", ItemStatus::NeedsAction, Some(Reason::OnlineOnly), None),
        line("5", ItemStatus::SkippedByUser, None, None),
    ]
}

fn qs_picked() -> Vec<crate::quicksend::PickedView> {
    use crate::quicksend::PickedView;
    vec![
        PickedView {
            id: "p1".into(),
            name: "letter.txt".into(),
            kind: "file",
            included: true,
            bytes: Some(1200),
            files: Some(1),
            problem: None,
        },
        PickedView {
            id: "p2".into(),
            name: "Holiday".into(),
            kind: "folder",
            included: true,
            bytes: None,
            files: None,
            problem: None,
        },
        PickedView {
            id: "p3".into(),
            name: "gone.txt".into(),
            kind: "file",
            included: true,
            bytes: None,
            files: None,
            problem: Some("missing"),
        },
    ]
}

fn qs_offered() -> Vec<crate::quicksend::OfferedView> {
    vec![crate::quicksend::OfferedView {
        id: "00112233445566778899aabbccddeeff".into(),
        path: "Holiday/beach.jpg".into(),
        size: 2_400_000,
    }]
}

fn qs_receipts() -> Vec<crate::quicksend::ReceiptView> {
    use crate::quicksend::{LineView, OutcomeView, ReceiptView};
    let line = |path: &str, outcome| LineView {
        path: path.into(),
        outcome,
    };
    vec![ReceiptView {
        side: "receiver",
        stopped: false,
        lines: vec![
            line(
                "a.jpg",
                OutcomeView::Arrived {
                    landed_as: "a.jpg".into(),
                },
            ),
            line(
                "b.jpg",
                OutcomeView::AlreadyThere {
                    landed_as: "b.jpg".into(),
                },
            ),
            line("c.jpg", OutcomeView::Declined),
            line(
                "d.jpg",
                OutcomeView::NeedsAction {
                    landed_as: "d.jpg".into(),
                    why: "open it once to check it".into(),
                },
            ),
            line(
                "e.jpg",
                OutcomeView::Failed {
                    why: "it could not be read".into(),
                },
            ),
        ],
    }]
}

fn qs_progress() -> Vec<crate::quicksend::ProgressView> {
    vec![crate::quicksend::ProgressView {
        phase: "copying",
        files_done: 3,
        files_total: 10,
        bytes_done: 1_000_000,
        bytes_total: 4_000_000,
        current: None,
        failed: 0,
        speed: Some(Range {
            low: 2_000_000,
            high: 2_000_000,
        }),
    }]
}

fn qs_folders() -> Vec<crate::quicksend::FolderView> {
    vec![crate::quicksend::FolderView {
        path: r"C:\Users\Ada\Downloads".into(),
        free_bytes: Some(80_000_000_000),
    }]
}

fn qs_problems() -> Vec<crate::quicksend::QsProblem> {
    use crate::quicksend::QsProblem;
    vec![
        QsProblem::LinkLost {
            detail: "the connection dropped".into(),
        },
        QsProblem::Failed {
            detail: "the folder cannot be used".into(),
        },
    ]
}

fn json(examples: impl serde::Serialize) -> Value {
    serde_json::to_value(examples).expect("examples serialise")
}

/// Each file's name (without .json) and its examples. bridge.ts has one parse function per name.
fn families() -> Vec<(&'static str, Value)> {
    vec![
        ("label-view", json(labels())),
        ("old-view", json(old_views())),
        ("new-view", json(new_views())),
        ("problem-view", json(problems())),
        ("nearby-view", json(nearby())),
        ("evidence", json(CheckedFrom::ALL)),
        ("heads-up", json(heads_ups())),
        ("move-problem", json(move_problems())),
        ("move-stage", json(stages())),
        ("ended-early", json(endings())),
        ("plan-view", json(plans())),
        ("move-progress", json(progress())),
        ("move-outcome", json(outcomes())),
        ("report-item", json(report_items())),
        ("privacy", json([crate::moving::Privacy { places: 3 }])),
        ("found", json([crate::moving::Found { bytes: 41_200_000 }])),
        ("peer", json(peers())),
        (
            "history-entry",
            json([
                crate::history::Entry {
                    id: "1792627200000-0".into(),
                    kind: "move".into(),
                    when: "2026-10-22T00:00:00Z".into(),
                    other: Some(json(picked())),
                    state: "steps-left".into(),
                    steps_left: 2,
                },
                crate::history::Entry {
                    id: "1792627200000-1".into(),
                    kind: "send".into(),
                    when: "2026-10-22T00:05:00Z".into(),
                    other: None,
                    state: "finished".into(),
                    steps_left: 0,
                },
            ]),
        ),
        (
            "ready-note",
            json([pctwin_move::handover::ReadyNote {
                landed: 120,
                kept: 2,
                failed: 1,
            }]),
        ),
        (
            "place-answer",
            json([
                crate::moving::PlaceAnswer::Chosen {
                    path: r"E:\Photos".into(),
                    free_bytes: Some(512_000_000_000),
                },
                crate::moving::PlaceAnswer::Chosen {
                    path: "/Volumes/Backup/Photos".into(),
                    free_bytes: None,
                },
                crate::moving::PlaceAnswer::NotChosen,
                crate::moving::PlaceAnswer::NoSafeName,
                crate::moving::PlaceAnswer::CannotOpen,
            ]),
        ),
        ("qs-picked", json(qs_picked())),
        ("qs-offered", json(qs_offered())),
        (
            "qs-incoming",
            json([crate::quicksend::IncomingView {
                files: qs_offered(),
            }]),
        ),
        ("qs-receipt", json(qs_receipts())),
        ("qs-progress", json(qs_progress())),
        ("qs-folder", json(qs_folders())),
        ("qs-problem", json(qs_problems())),
        (
            "people-to-place",
            json([crate::moving::PeopleToPlace {
                people: vec![crate::moving::PersonToPlace {
                    id: "S-1-5-21-1-2-3-1002".into(),
                    name: "Bea".into(),
                    files: 120,
                    bytes: 3_400_000,
                }],
                accounts: vec![crate::others::AccountHere {
                    id: "S-1-5-21-9-9-9-1001".into(),
                    name: "bea".into(),
                }],
                can_place: true,
            }]),
        ),
        (
            "people-here",
            json([crate::others::PeopleHere {
                people: vec![
                    crate::others::PersonHere {
                        id: "S-1-5-21-1-2-3-1001".into(),
                        name: "Ada".into(),
                        is_you: true,
                    },
                    crate::others::PersonHere {
                        id: "S-1-5-21-1-2-3-1002".into(),
                        name: "Bea".into(),
                        is_you: false,
                    },
                ],
                can_move_others: true,
            }]),
        ),
        (
            "interrupted",
            json([crate::recovery::Interrupted { unfinished: 3 }]),
        ),
        (
            "tidied",
            json([crate::recovery::Tidied {
                put_in_place: 2,
                kept_to_continue: 1,
                not_finished: 0,
                left_for_later: 1,
            }]),
        ),
    ]
}

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src/contract")
}

/// Pretty JSON with a final line break, the same on every system.
fn text(value: &Value) -> String {
    let mut s = serde_json::to_string_pretty(value).unwrap();
    s.push('\n');
    s
}

#[test]
fn the_example_files_match_the_engine() {
    let write = std::env::var_os("PCTWIN_WRITE_CONTRACT").is_some_and(|v| v == "1");
    let dir = dir();
    let families = families();
    let names: Vec<String> = families.iter().map(|(n, _)| format!("{n}.json")).collect();
    let mut stale = Vec::new();
    for (name, value) in &families {
        let path = dir.join(format!("{name}.json"));
        let want = text(value);
        if write {
            std::fs::write(&path, &want).unwrap();
            continue;
        }
        // A checkout that turned line breaks into CRLF is not a difference.
        let have = std::fs::read_to_string(&path).map(|s| s.replace("\r\n", "\n"));
        if have.ok().as_deref() != Some(want.as_str()) {
            stale.push(format!("{name}.json"));
        }
    }
    // A file for a shape that no longer crosses the bridge is stale too.
    for entry in std::fs::read_dir(&dir).unwrap() {
        let file = entry.unwrap().file_name().to_string_lossy().into_owned();
        if file.ends_with(".json") && !names.contains(&file) {
            if write {
                std::fs::remove_file(dir.join(&file)).unwrap();
            } else {
                stale.push(format!("{file} (no longer made)"));
            }
        }
    }
    assert!(
        stale.is_empty(),
        "the screens' example files in app/src/contract differ from what the engine sends: {stale:?}. \
         If the change is intended, run with PCTWIN_WRITE_CONTRACT=1 to update them, then update \
         bridge.ts so contract.test.ts passes."
    );
}

/// The `kind` tags in a family's examples.
fn tags(examples: Value) -> BTreeSet<String> {
    examples
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["kind"].as_str().expect("a kind tag").to_owned())
        .collect()
}

/// Each enum's examples hold every one of its variants (named by the enum's own list, made with
/// it), and the wire tags are as many as the variants, so no two variants look alike to the screens.
#[test]
fn every_variant_is_in_the_examples_with_its_own_tag() {
    fn check<T: serde::Serialize>(
        family: &str,
        examples: &[T],
        variant: fn(&T) -> &'static str,
        all: &[&str],
    ) {
        let have: BTreeSet<&str> = examples.iter().map(variant).collect();
        let want: BTreeSet<&str> = all.iter().copied().collect();
        let missing: Vec<_> = want.difference(&have).collect();
        assert!(
            missing.is_empty(),
            "{family}.json has no example of {missing:?}"
        );
        assert_eq!(
            tags(json(examples)).len(),
            all.len(),
            "{family}.json: two variants share a tag"
        );
    }
    check(
        "label-view",
        &labels(),
        LabelView::variant,
        LabelView::VARIANTS,
    );
    check(
        "old-view",
        &old_views(),
        OldView::variant,
        OldView::VARIANTS,
    );
    check(
        "new-view",
        &new_views(),
        NewView::variant,
        NewView::VARIANTS,
    );
    check(
        "heads-up",
        &heads_ups(),
        HeadsUp::variant,
        HeadsUp::VARIANTS,
    );
    check(
        "move-problem",
        &move_problems(),
        MoveProblem::variant,
        MoveProblem::VARIANTS,
    );
    check("move-stage", &stages(), Stage::variant, Stage::VARIANTS);
    check(
        "ended-early",
        &endings(),
        EndedEarly::variant,
        EndedEarly::VARIANTS,
    );

    // Problems: one example per problem, each with its own key.
    let keys: BTreeSet<&str> = problems().iter().map(|p| p.key).collect();
    assert_eq!(problems().len(), Problem::VARIANTS.len());
    assert_eq!(
        keys.len(),
        Problem::VARIANTS.len(),
        "two problems share a key"
    );

    // Evidence: every value (the gate's test checks `ALL` against its variants), each distinct.
    let evidence: BTreeSet<String> = CheckedFrom::ALL
        .iter()
        .map(|e| json(e).to_string())
        .collect();
    assert_eq!(evidence.len(), CheckedFrom::ALL.len());
    let reasons: BTreeSet<&str> = CheckedFrom::ALL
        .iter()
        .filter_map(|e| match e {
            CheckedFrom::Memory(why) => Some(why.variant()),
            CheckedFrom::Drive => None,
        })
        .collect();
    assert_eq!(reasons.len(), WhyMemory::VARIANTS.len());
}

/// What the screens send with each of the app's commands: every argument's name, in the
/// command's own order, with one value of the right type; and the names of the app's events.
/// Written to app/src/contract/shell/calls.json. tests/permissions.rs proves every name on the real
/// commands, and the screens' bridge test proves bridge.ts calls and listens with exactly these
/// names, so a renamed command, argument or event on either side fails a test.
fn calls() -> Value {
    use crate::moving::{
        FINISHED_EVENT, PEOPLE_EVENT, PLAN_EVENT, PRIVACY_EVENT, PROBLEM_EVENT, PROGRESS_EVENT,
        STAGE_EVENT,
    };
    use crate::pairing::{NEW_EVENT, OLD_EVENT};
    use serde_json::json;
    json!({
        "commands": [
            { "name": "set_window_mode", "args": [["mode", "mini"]] },
            { "name": "set_tray_labels", "args": [["show", "Show"], ["quit", "Quit"]] },
            { "name": "pairing_old_start", "args": [["network", "Wi-Fi"]] },
            { "name": "pairing_old_pick", "args": [["round", 1], ["number", 42]] },
            { "name": "pairing_old_decline", "args": [["round", 1]] },
            { "name": "pairing_old_shuffle", "args": [] },
            { "name": "pairing_old_rename", "args": [["name", "Kitchen laptop"]] },
            { "name": "pairing_old_start_again", "args": [] },
            { "name": "pairing_heads_up", "args": [["role", "old"]] },
            { "name": "pairing_stop", "args": [] },
            { "name": "pairing_new_search", "args": [] },
            { "name": "pairing_new_connect", "args": [["index", 0], ["code", "123456"]] },
            { "name": "move_begin", "args": [["fallback", { "music": "From your old laptop/Music" }], ["others", ["1001"]]] },
            { "name": "move_approve", "args": [["plan", { "revision": 2, "fingerprint": "9f" }], ["approve", true], ["leftOut", ["music"]]] },
            { "name": "move_place_people", "args": [["choices", [{ "id": "S-2", "to": { "kind": "new" } }]], ["words", { "title": "New account", "body": "{name}: {account} / {password}" }]] },
            { "name": "move_choose_place", "args": [["role", "pictures"]] },
            { "name": "move_usual_place", "args": [["role", "pictures"]] },
            { "name": "move_privacy_answer", "args": [["again", true]] },
            { "name": "open_privacy_settings", "args": [] },
            { "name": "move_pause", "args": [] },
            { "name": "move_resume", "args": [] },
            { "name": "move_stop", "args": [] },
            { "name": "move_report", "args": [] },
            { "name": "move_people_here", "args": [] },
            { "name": "help_code", "args": [["topic", "connection-lost"], ["step", "moving"]] },
            { "name": "recovery_check", "args": [] },
            { "name": "recovery_run", "args": [] },
            { "name": "things_ready", "args": [] },
            { "name": "history_list", "args": [] },
            { "name": "history_open", "args": [["id", "1792627200000-0"]] },
            { "name": "history_export", "args": [] },
            { "name": "quicksend_pick", "args": [["folders", false]] },
            { "name": "quicksend_accept_drops", "args": [["on", true]] },
            { "name": "quicksend_clear", "args": [] },
            { "name": "quicksend_offer", "args": [["ids", ["p1"]]] },
            { "name": "quicksend_send", "args": [] },
            { "name": "quicksend_receive", "args": [] },
            { "name": "quicksend_choose_folder", "args": [] },
            { "name": "quicksend_answer", "args": [["answer", { "kind": "decline" }]] },
            { "name": "quicksend_pause", "args": [] },
            { "name": "quicksend_resume", "args": [] },
            { "name": "quicksend_stop", "args": [] },
        ],
        "events": {
            "old": OLD_EVENT,
            "new": NEW_EVENT,
            "moveStage": STAGE_EVENT,
            "movePlan": PLAN_EVENT,
            "moveProgress": PROGRESS_EVENT,
            "moveFinished": FINISHED_EVENT,
            "moveProblem": PROBLEM_EVENT,
            "movePeople": PEOPLE_EVENT,
            "movePrivacy": PRIVACY_EVENT,
            "moveFound": crate::moving::FOUND_EVENT,
            "movePeer": crate::moving::PEER_EVENT,
            "qsProgress": crate::quicksend::PROGRESS_EVENT,
            "qsIncoming": crate::quicksend::INCOMING_EVENT,
            "qsReceipt": crate::quicksend::RECEIPT_EVENT,
            "qsProblem": crate::quicksend::PROBLEM_EVENT,
            "qsPicked": crate::quicksend::PICKED_EVENT,
            "qsDragging": crate::quicksend::DRAGGING_EVENT,
        },
        // What the screens may name when they ask for a help code: the engine's closed lists.
        "helpTopics": crate::help::HelpTopic::ALL,
        "helpSteps": crate::help::HelpStep::ALL,
    })
}

#[test]
fn the_calls_file_matches_the_engine() {
    let path = dir().join("shell").join("calls.json");
    let want = text(&calls());
    if std::env::var_os("PCTWIN_WRITE_CONTRACT").is_some_and(|v| v == "1") {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &want).unwrap();
        return;
    }
    let have = std::fs::read_to_string(&path).map(|s| s.replace("\r\n", "\n"));
    assert_eq!(
        have.ok().as_deref(),
        Some(want.as_str()),
        "app/src/contract/shell/calls.json differs from the engine's commands and events. If the \
         change is intended, run with PCTWIN_WRITE_CONTRACT=1 to update it, then update bridge.ts \
         so bridge.test.ts passes."
    );
}

/// Every colour and animal reaches the screens as a word key they accept (lower-case words joined
/// by hyphens, bridge.ts `isKey`), not only the one in the examples.
#[test]
fn every_label_word_is_a_key_the_screens_accept() {
    fn is_key(s: &str) -> bool {
        !s.is_empty()
            && s.split('-').all(|part| {
                !part.is_empty()
                    && part
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
            })
    }
    let colours = u8::try_from(pctwin_discovery::COLOURS.len()).unwrap();
    let animals = u8::try_from(pctwin_discovery::ANIMALS.len()).unwrap();
    for colour in 0..colours {
        for animal in 0..animals {
            match LabelView::from(&Label::Picked { colour, animal }) {
                LabelView::Picked {
                    colour: c,
                    animal: a,
                } => {
                    assert!(is_key(c) && is_key(a), "{c} / {a}");
                }
                other => panic!("colour {colour}, animal {animal} became {other:?}"),
            }
        }
    }
}
