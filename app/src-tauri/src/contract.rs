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
    Range, ReportItem, ReportNote, RunState, Size, Stage,
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
        MoveProblem::TooManyFiles,
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
        Stage::OthersNotMoved {
            why: OthersWhy::AllowInSettings,
        },
        Stage::Rebuilding,
        Stage::AppData,
    ]
}

/// The full move's plan section: an app installed, one needing the person, a group of settings,
/// a special category's step, and PCTwin's own "not moved yet" line.
fn rebuild_plan() -> crate::fullmove::RebuildPlanView {
    use crate::fullmove::ChangeView;
    use crate::guard_views::{ChangeAction, UndoPromise};
    use pctwin_fullmove::extras::{NotMovedCount, NotMovedGroup, SpecialStep};
    use pctwin_fullmove::plan::{Kind, Title};
    let change = |title, kind, action, undo, count, needs, special, files| ChangeView {
        title,
        kind,
        action,
        undo,
        count,
        needs,
        special,
        files,
        secret: None,
        source: None,
        note: None,
    };
    crate::fullmove::RebuildPlanView {
        changes: vec![
            ChangeView {
                source: Some("Example.Writer".into()),
                ..change(
                    Title::Name("Writer".into()),
                    Kind::App,
                    ChangeAction::Install,
                    UndoPromise::RemoveYourself,
                    0,
                    None,
                    None,
                    0,
                )
            },
            change(
                Title::Name("Sketch Tool".into()),
                Kind::App,
                ChangeAction::LeaveAlone,
                UndoPromise::MustKeep,
                0,
                Some(Reason::ManualInstall),
                None,
                0,
            ),
            change(
                Title::Name("Writer settings".into()),
                Kind::Settings,
                ChangeAction::Replace,
                UndoPromise::CanReverse,
                12,
                None,
                None,
                0,
            ),
            ChangeView {
                note: Some(pctwin_fullmove::plan::ChangeNote::OlderHere {
                    version: "2.0".into(),
                }),
                ..change(
                    Title::Name("Notes".into()),
                    Kind::App,
                    ChangeAction::LeaveAlone,
                    UndoPromise::MustKeep,
                    0,
                    None,
                    None,
                    0,
                )
            },
            ChangeView {
                note: Some(pctwin_fullmove::plan::ChangeNote::Sites),
                ..change(
                    Title::Name("Browser".into()),
                    Kind::Special,
                    ChangeAction::LeaveAlone,
                    UndoPromise::MustKeep,
                    0,
                    Some(Reason::SignIn),
                    Some(SpecialStep::Relink),
                    1,
                )
            },
            change(
                Title::Name("Mail".into()),
                Kind::Special,
                ChangeAction::LeaveAlone,
                UndoPromise::MustKeep,
                0,
                Some(Reason::SignIn),
                Some(SpecialStep::SignIn),
                3,
            ),
            change(
                Title::Name("Browser".into()),
                Kind::Special,
                ChangeAction::Merge,
                UndoPromise::CanReverse,
                0,
                None,
                Some(SpecialStep::Copy),
                12,
            ),
            ChangeView {
                secret: Some(2),
                ..change(
                    Title::Name("Browser".into()),
                    Kind::Special,
                    ChangeAction::Merge,
                    UndoPromise::CanReverse,
                    0,
                    None,
                    Some(SpecialStep::Copy),
                    2,
                )
            },
            change(
                Title::Key(pctwin_fullmove::plan::SPECIAL_NOT_YET),
                Kind::Special,
                ChangeAction::LeaveAlone,
                UndoPromise::MustKeep,
                0,
                Some(Reason::FinishByHand),
                Some(SpecialStep::NotYet),
                0,
            ),
        ],
        more: 0,
        not_moved: vec![
            NotMovedCount {
                group: NotMovedGroup::Private,
                count: 40,
            },
            NotMovedCount {
                group: NotMovedGroup::CouldRun,
                count: 3,
            },
        ],
        special_not_moved: 7,
        links: 4,
        prompt: false,
        restore_point: false,
    }
}

/// The full move's outcome section.
fn rebuilt() -> crate::fullmove::RebuiltView {
    use crate::fullmove::{HereLine, LineNote, RebuiltLine, RestoreWord};
    use pctwin_fullmove::data::DataWhy;
    use pctwin_fullmove::plan::{Kind, Title};
    let line = |title, kind, count, status, reason, notes| RebuiltLine {
        title,
        kind,
        count,
        status,
        reason,
        notes,
    };
    let data = |why| {
        line(
            Title::Name("Mail".into()),
            Kind::Special,
            0,
            ItemStatus::NeedsAction,
            Some(Reason::FinishByHand),
            vec![LineNote::Data(why)],
        )
    };
    crate::fullmove::RebuiltView {
        lines: vec![
            line(
                Title::Name("Sketch Tool".into()),
                Kind::App,
                0,
                ItemStatus::NeedsAction,
                Some(Reason::ManualInstall),
                vec![
                    LineNote::LicenceKey,
                    LineNote::Registration {
                        owner: Some("Ada".into()),
                        company: None,
                        product_id: Some("12345-OEM".into()),
                    },
                ],
            ),
            line(
                Title::Name("Writer".into()),
                Kind::App,
                0,
                ItemStatus::NeedsAction,
                Some(Reason::SignIn),
                vec![
                    LineNote::SignIn,
                    LineNote::Extras {
                        apps: vec!["Writer Helper".into()],
                    },
                ],
            ),
            line(
                Title::Name("Writer settings".into()),
                Kind::Settings,
                12,
                ItemStatus::Converted,
                None,
                vec![],
            ),
            line(
                Title::Name("Browser".into()),
                Kind::Special,
                0,
                ItemStatus::NeedsAction,
                Some(Reason::FinishByHand),
                vec![LineNote::Sites],
            ),
            data(DataWhy::Lost),
            data(DataWhy::DidNotArrive),
            data(DataWhy::AppOpen),
            data(DataWhy::Changing),
            data(DataWhy::Unreadable),
            data(DataWhy::NotRead),
            data(DataWhy::NotWhole),
            data(DataWhy::NotLanded),
            data(DataWhy::Beside {
                folder: "Mail (from old laptop)".into(),
            }),
            data(DataWhy::Placed {
                folder: "Mail".into(),
            }),
            line(
                Title::Key(pctwin_fullmove::plan::SPECIAL_NOT_YET),
                Kind::Special,
                0,
                ItemStatus::NeedsAction,
                Some(Reason::FinishByHand),
                vec![],
            ),
        ],
        more: 0,
        here: vec![
            HereLine {
                name: "Paint".into(),
                older: None,
            },
            HereLine {
                name: "Notes".into(),
                older: Some("2.0".into()),
            },
        ],
        here_more: 0,
        fresh: true,
        restore_point: RestoreWord::NotAsked,
        stopped: false,
        can_undo: true,
        undo_id: Some("01010101010101010101010101010101".into()),
    }
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

/// The connection during a move (C09, C10): connected again, waiting (trying, and with nothing
/// new), and ended with and without anything moved.
fn links() -> Vec<crate::moving::ConnectionView> {
    use crate::moving::ConnectionView;
    vec![
        ConnectionView::Connected,
        ConnectionView::Waiting {
            nothing_new: false,
            last_seen_seconds: 12,
            saved_bytes: 81_600_000_000,
            total_bytes: 120_000_000_000,
        },
        ConnectionView::Waiting {
            nothing_new: true,
            last_seen_seconds: 95,
            saved_bytes: 81_600_000_000,
            total_bytes: 120_000_000_000,
        },
        ConnectionView::Ended {
            moved_something: true,
            saved_bytes: 81_600_000_000,
            total_bytes: 120_000_000_000,
        },
        ConnectionView::Ended {
            moved_something: false,
            saved_bytes: 0,
            total_bytes: 120_000_000_000,
        },
    ]
}

fn peers() -> Vec<crate::moving::Peer> {
    use crate::moving::{LinkView, Peer, Versions};
    use pctwin_move::greeting::{About, Power, System};
    let about = |system, chip: &str, app: &str, power, free_bytes| About {
        system,
        chip: chip.into(),
        app: app.into(),
        power,
        free_bytes,
        label: None,
    };
    vec![
        Peer {
            mine: about(
                System::Mac,
                "aarch64",
                "0.1.0",
                Power::Plugged,
                Some(410_000_000_000),
            ),
            theirs: about(
                System::Windows,
                "x86_64",
                "0.1.0",
                Power::Battery { percent: 34 },
                Some(52_000_000_000),
            ),
            other: Some(picked()),
            mine_name: Some(LabelView::Named {
                name: "Ada's new laptop".into(),
            }),
            theirs_name: Some(picked()),
            versions: Versions::Same,
            link: Some(LinkView {
                bytes_per_second: 62_000_000,
                slow: false,
            }),
        },
        Peer {
            mine: about(System::Linux, "x86_64", "0.1.0", Power::Unknown, None),
            theirs: about(System::Other, "riscv64", "0.1.0", Power::Plugged, None),
            other: None,
            mine_name: None,
            theirs_name: None,
            versions: Versions::Same,
            link: Some(LinkView {
                bytes_per_second: 1_200_000,
                slow: true,
            }),
        },
        Peer {
            mine: about(System::Windows, "x86_64", "0.1.0", Power::Plugged, None),
            theirs: about(System::Mac, "aarch64", "0.2.0", Power::Plugged, None),
            other: None,
            mine_name: None,
            theirs_name: None,
            versions: Versions::UpdateHere,
            link: None,
        },
    ]
}

fn plans() -> Vec<PlanView> {
    let files_only = PlanView {
        first: vec![FolderRole::Documents],
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
            estimated_bytes: 0,
            synced: vec![crate::moving::SyncedPlace {
                folder: "C:\\Users\\ada\\OneDrive\\Documents".into(),
                service: "OneDrive".into(),
            }],
            minutes: Some(Range { low: 4, high: 8 }),
            renamed: 1,
            too_big: 0,
            refused: 0,
            runs_at_sign_in: 1,
        },
        rebuild: None,
    };
    let full = PlanView {
        rebuild: Some(rebuild_plan()),
        ..files_only.clone()
    };
    vec![files_only, full]
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
        MoveProgress {
            notice: Some(Notice::MaySleep),
            ..quiet(RunState::Running, None)
        },
        quiet(RunState::Paused, Some(named())),
    ]
}

fn outcomes() -> Vec<MoveOutcome> {
    let files_only = MoveOutcome {
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
        broke_off: false,
        checked_from_memory: 3,
        after_move: crate::moving::AfterMove {
            photos: crate::moving::KindCount {
                planned: 12,
                copied: 11,
            },
            documents: crate::moving::KindCount {
                planned: 20,
                copied: 20,
            },
            ..Default::default()
        },
        rebuilt: None,
        landed: Vec::new(),
    };
    let full = MoveOutcome {
        rebuilt: Some(rebuilt()),
        landed: vec![crate::moving::LandedPlace {
            name: crate::moving::group_name("documents"),
            folder: r"C:\Users\Ada\Documents".into(),
        }],
        ..files_only.clone()
    };
    vec![files_only, full]
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
        note: None,
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
        // Sent as it is now, and gone before it could be sent ("changed since the scan").
        ReportItem {
            note: Some(ReportNote::ChangedSinceScan),
            ..line("9", ItemStatus::Copied, None, Some(Evidence::Drive))
        },
        ReportItem {
            note: Some(ReportNote::NoLongerThere),
            ..line("10", ItemStatus::Failed, Some(Reason::Unreadable), None)
        },
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
        new_dates: 1,
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
                    word: None,
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
        minutes_left: Some(Range { low: 4, high: 4 }),
    }]
}

fn qs_folders() -> Vec<crate::quicksend::FolderView> {
    vec![crate::quicksend::FolderView {
        name: "Downloads".into(),
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
        QsProblem::NoRoom {
            needed: 238_000_000,
            room: 120_000_000,
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
        (
            "move-appdata",
            json([
                pctwin_fullmove::data::DataProgress {
                    done: 1,
                    total: 3,
                    waiting_for: vec![],
                },
                pctwin_fullmove::data::DataProgress {
                    done: 2,
                    total: 3,
                    waiting_for: vec!["Chromey".into()],
                },
            ]),
        ),
        (
            "move-appdata-open",
            json([crate::fullmove::AppsOpen {
                apps: vec!["Chromey".into(), "Mail".into()],
            }]),
        ),
        (
            "move-choose",
            json([
                crate::moving::ChoiceView {
                    groups: vec![
                        crate::moving::ChoiceGroup {
                            role: FolderRole::Documents,
                            files: 1_240,
                            bytes: 3_200_000_000,
                        },
                        crate::moving::ChoiceGroup {
                            role: FolderRole::Pictures,
                            files: 8_400,
                            bytes: 41_000_000_000,
                        },
                    ],
                    apps: 12,
                    left_behind: vec![crate::moving::LeftBehind {
                        why: pctwin_record::LeftOutReason::Rebuildable,
                        items: 3,
                        bytes: 900_000_000,
                    }],
                },
                crate::moving::ChoiceView {
                    groups: Vec::new(),
                    apps: 0,
                    left_behind: Vec::new(),
                },
            ]),
        ),
        (
            "move-rebuild",
            json([
                pctwin_fullmove::run::Progress::Permission,
                pctwin_fullmove::run::Progress::Apps { done: 1, total: 3 },
                pctwin_fullmove::run::Progress::Settings {
                    done: 40,
                    total: 120,
                },
            ]),
        ),
        ("report-item", json(report_items())),
        ("privacy", json([crate::moving::Privacy { places: 3 }])),
        ("found", json([crate::moving::Found { bytes: 41_200_000 }])),
        ("peer", json(peers())),
        ("move-link", json(links())),
        (
            "history-entry",
            json([
                crate::history::Entry {
                    id: "1792627200000-0".into(),
                    kind: "move".into(),
                    when: "2026-10-22T00:00:00Z".into(),
                    other: Some(json(picked())),
                    new_name: None,
                    state: "steps-left".into(),
                    steps_left: 2,
                },
                crate::history::Entry {
                    id: "1792627200000-1".into(),
                    kind: "move".into(),
                    when: "2026-10-22T00:05:00Z".into(),
                    other: None,
                    new_name: None,
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
                crate::moving::PlaceAnswer::Locked,
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
        ("guard-snapshot", json(crate::guard_examples::snapshots())),
        ("guard-refused", json(crate::guard_examples::refusals())),
        ("guard-answer", json(crate::guard_examples::answers())),
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
        "move-link",
        &links(),
        crate::moving::ConnectionView::variant,
        crate::moving::ConnectionView::VARIANTS,
    );
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
            { "name": "window_drag", "args": [] },
            { "name": "set_tray_labels", "args": [["show", "Show"], ["quit", "Quit"]] },
            { "name": "pairing_old_start", "args": [["network", "Wi-Fi"]] },
            { "name": "pairing_old_pick", "args": [["round", 1], ["number", 42]] },
            { "name": "pairing_old_decline", "args": [["round", 1]] },
            { "name": "pairing_old_shuffle", "args": [] },
            { "name": "pairing_old_rename", "args": [["name", "Kitchen laptop"]] },
            { "name": "pairing_old_start_again", "args": [] },
            { "name": "pairing_old_nearby", "args": [] },
            { "name": "pairing_heads_up", "args": [["role", "old"]] },
            { "name": "pairing_stop", "args": [] },
            { "name": "pairing_new_search", "args": [] },
            { "name": "pairing_new_connect", "args": [["index", 0], ["code", "123456"]] },
            { "name": "move_begin", "args": [["fallback", { "music": "From your old laptop/Music" }], ["others", ["1001"]], ["approve", "Approve reading the files of {names}."]] },
            { "name": "move_approve", "args": [["plan", { "revision": 2, "fingerprint": "9f" }], ["approve", true], ["leftOut", ["music"]], ["secrets", [2]]] },
            { "name": "move_place_people", "args": [["choices", [{ "id": "S-2", "to": { "kind": "new" } }]], ["words", { "title": "New account", "body": "{name}: {account} / {password}", "approve": "Approve reading the files of {names}." }]] },
            { "name": "move_choose_place", "args": [["role", "pictures"]] },
            { "name": "move_usual_place", "args": [["role", "pictures"]] },
            { "name": "move_privacy_answer", "args": [["again", true]] },
            { "name": "open_privacy_settings", "args": [] },
            { "name": "move_pause", "args": [] },
            { "name": "move_resume", "args": [] },
            { "name": "move_stop", "args": [] },
            { "name": "move_retry", "args": [] },
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
            { "name": "quicksend_offer_missing", "args": [] },
            { "name": "quicksend_send", "args": [] },
            { "name": "quicksend_receive", "args": [] },
            { "name": "quicksend_folder", "args": [] },
            { "name": "quicksend_choose_folder", "args": [] },
            { "name": "quicksend_open_folder", "args": [] },
            { "name": "quicksend_keep_original_dates", "args": [] },
            { "name": "quicksend_pause", "args": [] },
            { "name": "quicksend_resume", "args": [] },
            { "name": "quicksend_stop", "args": [] },
            { "name": "update_check", "args": [] },
            { "name": "update_install", "args": [] },
            { "name": "laptop_name", "args": [] },
            { "name": "laptop_name_shuffle", "args": [] },
            { "name": "laptop_name_use", "args": [] },
            { "name": "laptop_name_type", "args": [["name", "Ezzy Laptop"]] },
            { "name": "guard_view", "args": [] },
            { "name": "guard_intent", "args": [["intent", { "job": "job-1", "epoch": 2, "generation": 3, "press": "press-1", "intent": { "kind": "try-again", "line": 0 } }]] },
            { "name": "fullmove_undo", "args": [["id", "01010101010101010101010101010101"]] },
            { "name": "move_appdata_answer", "args": [["retry", true]] },
            { "name": "move_choose_what", "args": [["choice", { "groups": ["documents"], "apps": true }]] },
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
            "moveLink": crate::moving::LINK_EVENT,
            "moveRebuild": crate::fullmove::REBUILD_EVENT,
            "moveUndoClosed": crate::fullmove::UNDO_CLOSED_EVENT,
            "moveAppdata": crate::fullmove::APPDATA_EVENT,
            "moveAppdataOpen": crate::fullmove::APPDATA_OPEN_EVENT,
            "moveChoose": crate::moving::CHOOSE_EVENT,
            "qsProgress": crate::quicksend::PROGRESS_EVENT,
            "qsIncoming": crate::quicksend::INCOMING_EVENT,
            "qsReceipt": crate::quicksend::RECEIPT_EVENT,
            "qsProblem": crate::quicksend::PROBLEM_EVENT,
            "qsPicked": crate::quicksend::PICKED_EVENT,
            "qsDragging": crate::quicksend::DRAGGING_EVENT,
            "guard": crate::guard::EVENT,
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
