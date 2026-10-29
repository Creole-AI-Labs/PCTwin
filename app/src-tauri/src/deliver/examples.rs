//! The engine's own examples of every 2.9 shape that crosses the bridge, for contract.rs (which
//! writes them into app/src/contract/ for the screens' tests). Every word of every list appears.

use pctwin_fullmove::plan::{Kind, Title};
use pctwin_types::{ItemStatus, Reason};

use super::changes::{AfterWhy, ChangedAfter, ChangesView, Recheck};
use super::kit::{KitPart, KitSaved};
use super::repair::{Prerequisite, RepairDone, RepairLine, RepairOffer, RepairProblem};
use super::safe_stop::{BusyWith, SafeStopView};
use super::work::{WorkCheck, WorkChecks, WorkLabel, WorkProblem, WorkState, WorkWhat};

fn check(index: u32, what: WorkWhat, name: &str, label: WorkLabel, state: WorkState) -> WorkCheck {
    WorkCheck {
        index,
        what,
        name: name.into(),
        label,
        state,
        missing: Vec::new(),
        missing_more: 0,
        not_checked: Vec::new(),
        checked: 0,
        of: 0,
    }
}

pub fn work_checks() -> Vec<WorkChecks> {
    use WorkLabel::*;
    use WorkState::*;
    use WorkWhat::*;
    vec![
        WorkChecks {
            generation: 0,
            checks: Vec::new(),
            more: 0,
        },
        WorkChecks {
            generation: 7,
            checks: vec![
                check(0, LinkedFiles, "Thesis.svg", StillNeedsAttention, NotRunYet),
                WorkCheck {
                    checked: 12,
                    of: 12,
                    ..check(1, LinkedFiles, "Garden plan.svg", MachineChecked, AllThere)
                },
                WorkCheck {
                    missing: vec!["images/front.jpg".into(), "Inter".into()],
                    missing_more: 3,
                    checked: 40,
                    of: 45,
                    ..check(2, LinkedFiles, "Poster.svg", StillNeedsAttention, Missing)
                },
                WorkCheck {
                    missing: vec!["Inter".into()],
                    not_checked: vec!["Web font (.woff2)".into()],
                    checked: 1,
                    of: 1,
                    ..check(3, Font, "Inter", StillNeedsAttention, Missing)
                },
                check(4, Archive, "Photos 2024.zip", StillNeedsAttention, Damaged),
                check(5, Database, "Recipes.sqlite", StillNeedsAttention, NotThere),
                check(
                    6,
                    Task,
                    "Print my boarding pass",
                    StillNeedsAttention,
                    WaitingForYou,
                ),
                WorkCheck {
                    checked: 1,
                    of: 1,
                    ..check(7, Task, "Open my accounts", PersonConfirmed, AllThere)
                },
                check(8, Task, "Open my invoices", StillNeedsAttention, YouSaidNo),
                check(
                    9,
                    LinkedFiles,
                    "Album.m3u",
                    StillNeedsAttention,
                    OutsideProject,
                ),
                check(10, Database, "Locked.db", StillNeedsAttention, CouldNotRead),
                check(
                    11,
                    LinkedFiles,
                    "Drawing.xyz",
                    StillNeedsAttention,
                    NotSupported,
                ),
                WorkCheck {
                    checked: 10_000,
                    of: 10_250,
                    ..check(
                        12,
                        LinkedFiles,
                        "Big map.svg",
                        StillNeedsAttention,
                        PartlyChecked,
                    )
                },
            ],
            more: 0,
        },
    ]
}

pub fn work_problems() -> Vec<WorkProblem> {
    use WorkProblem::*;
    vec![
        NotChosen,
        TooMany,
        NoSuchCheck,
        Stale,
        BadName,
        WrongKind,
        Failed,
    ]
}

fn repair_line(line: u32, title: &str, kind: Kind, waits_for: Vec<Prerequisite>) -> RepairLine {
    RepairLine {
        line,
        title: Title::Name(title.into()),
        kind,
        status: ItemStatus::Failed,
        reason: Some(Reason::Unreadable),
        waits_for,
    }
}

pub fn repair_offers() -> Vec<RepairOffer> {
    use Prerequisite::*;
    vec![
        RepairOffer {
            generation: 3,
            lines: vec![
                repair_line(0, "Writer", Kind::App, Vec::new()),
                RepairLine {
                    status: ItemStatus::NeedsAction,
                    reason: Some(Reason::ManualInstall),
                    ..repair_line(
                        1,
                        "Sketch Tool",
                        Kind::App,
                        vec![Internet, Administrator, InstallerTool],
                    )
                },
                repair_line(2, "Writer settings", Kind::Settings, vec![CloseApp]),
                repair_line(3, "Mail", Kind::Special, vec![OldLaptop, Space]),
            ],
            more: 0,
            kept: 41,
        },
        RepairOffer {
            generation: 4,
            lines: Vec::new(),
            more: 0,
            kept: 45,
        },
    ]
}

pub fn repair_done() -> Vec<RepairDone> {
    vec![RepairDone {
        generation: 4,
        fixed: 2,
        lines: vec![repair_line(
            0,
            "Mail",
            Kind::Special,
            vec![Prerequisite::OldLaptop],
        )],
        more: 0,
        kept: 43,
        undo_id: Some("0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f".into()),
    }]
}

pub fn repair_problems() -> Vec<RepairProblem> {
    use RepairProblem::*;
    vec![NothingToRepair, Stale, Busy, Failed]
}

pub fn changes() -> Vec<ChangesView> {
    use Recheck::*;
    vec![ChangesView {
        generation: 2,
        lines: vec![
            ChangedAfter {
                name: "Writer".into(),
                why: AfterWhy::AppUpdated,
                steps: vec![Install, Settings],
            },
            ChangedAfter {
                name: "Sketch Tool".into(),
                why: AfterWhy::AppRemoved,
                steps: vec![Settings, AppData],
            },
            ChangedAfter {
                name: "Pictures".into(),
                why: AfterWhy::DestinationChanged,
                steps: vec![Files],
            },
            ChangedAfter {
                name: "Thesis.svg".into(),
                why: AfterWhy::ProjectEdited,
                steps: vec![Files],
            },
        ],
        more: 0,
        held: 6,
    }]
}

pub fn safe_stops() -> Vec<SafeStopView> {
    use BusyWith::*;
    vec![
        SafeStopView {
            safe: true,
            busy: Vec::new(),
        },
        SafeStopView {
            safe: false,
            busy: vec![Moving, Rebuilding, AppData, Undoing],
        },
    ]
}

pub fn kits() -> Vec<KitSaved> {
    use KitPart::*;
    vec![KitSaved {
        folder: r"E:\Keep".into(),
        files: vec![
            "PCTwin recovery kit.json".into(),
            "PCTwin recovery kit - read me.txt".into(),
        ],
        holds: vec![MoveRecord, Results, BeforeWipeList],
        lacks: vec![FileData, SetupSteps, RecoveryTool],
    }]
}
