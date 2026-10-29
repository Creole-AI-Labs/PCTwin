//! One example of every shape the flow guard sends (src/guard.rs, src/guard_views.rs), for the
//! screens' contract files (contract.rs writes them to app/src/contract/guard-*.json). The test
//! below requires every variant of every enum the screens read, so a new one cannot be left out.

use std::collections::BTreeSet;

use crate::guard::{
    Answer, DisplayState, IntentName, Measured, Projection, Refused, Snapshot, Unit,
};
use crate::guard_views::*;

fn projection(display: DisplayState, screen: ScreenView, progress: Option<Measured>) -> Projection {
    Projection {
        job: "job-1".into(),
        epoch: 2,
        generation: 3,
        version: 9,
        display,
        allowed: vec![IntentName::Approve, IntentName::Back],
        progress,
        screen,
    }
}

fn budget(fits: Option<bool>) -> DriveBudget {
    DriveBudget {
        drive: "C:".into(),
        needed: 300,
        free: fits.map(|_| 200),
        fits,
        short_by: if fits == Some(false) { 100 } else { 0 },
        parts: SpaceUse::VARIANTS
            .iter()
            .enumerate()
            .map(|(i, _)| SpacePart {
                used_for: [
                    SpaceUse::Files,
                    SpaceUse::AppDownloads,
                    SpaceUse::Unpacking,
                    SpaceUse::UndoRecord,
                    SpaceUse::PartlyCopied,
                ][i],
                bytes: 60,
            })
            .collect(),
    }
}

const PROMISES: [UndoPromise; 6] = [
    UndoPromise::CanReverse,
    UndoPromise::CanRemoveApp,
    UndoPromise::MustKeep,
    UndoPromise::NeedsYouService,
    UndoPromise::NeedsYouMaker,
    UndoPromise::RemoveYourself,
];
const ACTIONS: [ChangeAction; 4] = [
    ChangeAction::Install,
    ChangeAction::Merge,
    ChangeAction::Replace,
    ChangeAction::LeaveAlone,
];
const REASONS: [PermissionReason; 5] = [
    PermissionReason::ReadOtherPeople,
    PermissionReason::MakeAccounts,
    PermissionReason::SharedFolders,
    PermissionReason::InstallForEveryone,
    PermissionReason::RestorePoint,
];
const STAGES: [PermissionStage; 7] = [
    PermissionStage::Explain,
    PermissionStage::WaitingForSystem,
    PermissionStage::Denied,
    PermissionStage::ClosedWithoutAnswer,
    PermissionStage::Given,
    PermissionStage::EndedBySleep,
    PermissionStage::EndedOtherSession,
];
const DISPLAYS: [DisplayState; 18] = [
    DisplayState::NotStarted,
    DisplayState::Preparing,
    DisplayState::AwaitingPerson,
    DisplayState::AwaitingSystem,
    DisplayState::AwaitingMaker,
    DisplayState::Working,
    DisplayState::Checking,
    DisplayState::Applying,
    DisplayState::PauseRequested,
    DisplayState::Paused,
    DisplayState::SourceUnavailable,
    DisplayState::Blocked,
    DisplayState::Partial,
    DisplayState::Retrying,
    DisplayState::Reconciling,
    DisplayState::Failed,
    DisplayState::Complete,
    DisplayState::Cancelled,
];

fn screens() -> Vec<ScreenView> {
    let drive = |is_target| RescueDrive {
        drive: "E:".into(),
        size: 512,
        file_system: "NTFS".into(),
        is_target,
    };
    let mut out = vec![
        ScreenView::Reconciling,
        ScreenView::Changes {
            changes: ACTIONS
                .iter()
                .zip(PROMISES.iter().cycle())
                .map(|(a, p)| ChangeLine {
                    name: "<b>VLC</b>".into(),
                    action: *a,
                    undo: *p,
                })
                .chain(std::iter::once(ChangeLine {
                    name: "Dark mode".into(),
                    action: ChangeAction::Replace,
                    undo: UndoPromise::NeedsYouMaker,
                }))
                .chain(std::iter::once(ChangeLine {
                    name: "Writer".into(),
                    action: ChangeAction::Install,
                    undo: UndoPromise::RemoveYourself,
                }))
                .collect(),
            more: 2,
            download_bytes: Some(1_000),
            drives: vec![budget(Some(true)), budget(Some(false)), budget(None)],
            permissions: REASONS.to_vec(),
            restore_point: true,
        },
        ScreenView::Changes {
            changes: vec![],
            more: 0,
            download_bytes: None,
            drives: vec![],
            permissions: vec![],
            restore_point: false,
        },
        ScreenView::Changed {
            lines: [
                ChangedWhy::ChangedOnOldLaptop,
                ChangedWhy::AppUpdated,
                ChangedWhy::DestinationChanged,
                ChangedWhy::NewScope,
                ChangedWhy::PermissionEnded,
            ]
            .iter()
            .map(|why| ChangedLine {
                name: "Thesis.docx".into(),
                why: *why,
            })
            .collect(),
            more: 0,
        },
        ScreenView::Space {
            drives: vec![budget(Some(false))],
            proposal: [
                KeepOff::LeaveInCloud,
                KeepOff::OffloadDrive,
                KeepOff::LeaveOnOldLaptop,
            ]
            .iter()
            .map(|to| ProposalLine {
                to: *to,
                items: 3,
                bytes: 30,
            })
            .collect(),
            offload: Some(OffloadView {
                drive: "Backup".into(),
                free: None,
                approved: false,
                connected: true,
            }),
            safe_in: vec![
                SafeInLine {
                    provider: provider(pctwin_record::CloudProvider::OneDrive),
                    files: 4,
                },
                SafeInLine {
                    provider: provider(pctwin_record::CloudProvider::Other),
                    files: 1,
                },
            ],
        },
        ScreenView::Space {
            drives: vec![],
            proposal: vec![],
            offload: None,
            safe_in: vec![],
        },
        ScreenView::Owners {
            rows: vec![
                OwnerRow {
                    laptop: "Kitchen".into(),
                    person: "Ada".into(),
                    account: Some("ada".into()),
                    own_folder: false,
                },
                OwnerRow {
                    laptop: "Work".into(),
                    person: "Ada".into(),
                    account: None,
                    own_folder: true,
                },
            ],
            more: 0,
        },
        ScreenView::Report {
            lines: vec![
                ReportLine {
                    line: 0,
                    what: LineWhat::Items { count: 3 },
                    laptop: "Kitchen".into(),
                    why: LineWhy::Failed,
                },
                ReportLine {
                    line: 1,
                    what: LineWhat::App {
                        name: "Zoom".into(),
                    },
                    laptop: "Kitchen".into(),
                    why: LineWhy::LaptopNotConnected,
                },
                ReportLine {
                    line: 2,
                    what: LineWhat::Settings { count: 1 },
                    laptop: "Kitchen".into(),
                    why: LineWhy::WaitsForAnother,
                },
                ReportLine {
                    line: 3,
                    what: LineWhat::Items { count: 1 },
                    laptop: "Kitchen".into(),
                    why: LineWhy::NotStarted,
                },
            ],
            more: 1,
            safe_in: vec![],
        },
        ScreenView::UndoPreview {
            lines: [
                UndoPlan::PutBack,
                UndoPlan::RemoveApp,
                UndoPlan::KeepChanged,
                UndoPlan::Elsewhere,
                UndoPlan::LeaveAlone,
                UndoPlan::RemoveYourself,
                UndoPlan::KeepOtherVersion,
                UndoPlan::AlreadyGone,
            ]
            .iter()
            .map(|plan| UndoPlanLine {
                name: "a.txt".into(),
                plan: *plan,
            })
            .collect(),
            more: 0,
            when: Some(1_793_000_000),
        },
        ScreenView::UndoDone {
            lines: vec![
                UndoLine {
                    name: "a".into(),
                    result: UndoResult::Restored,
                    detail: None,
                },
                UndoLine {
                    name: "b".into(),
                    result: UndoResult::Removed,
                    detail: None,
                },
                UndoLine {
                    name: "c".into(),
                    result: UndoResult::AsBefore,
                    detail: None,
                },
                UndoLine {
                    name: "d".into(),
                    result: UndoResult::KeptChanged,
                    detail: Some("it changed since the move".into()),
                },
                UndoLine {
                    name: "e".into(),
                    result: UndoResult::KeptOther,
                    detail: Some("in use".into()),
                },
                UndoLine {
                    name: "f".into(),
                    result: UndoResult::KeptAt {
                        at: "f (kept).txt".into(),
                    },
                    detail: Some("changed".into()),
                },
                UndoLine {
                    name: "g".into(),
                    result: UndoResult::DoneElsewhere,
                    detail: None,
                },
                UndoLine {
                    name: "h".into(),
                    result: UndoResult::NotDoneYet,
                    detail: Some("locked".into()),
                },
                UndoLine {
                    name: "i".into(),
                    result: UndoResult::RemoveYourself,
                    detail: None,
                },
                UndoLine {
                    name: "j".into(),
                    result: UndoResult::KeptOtherVersion,
                    detail: None,
                },
                UndoLine {
                    name: "k".into(),
                    result: UndoResult::RemovedRestart,
                    detail: None,
                },
                UndoLine {
                    name: "l".into(),
                    result: UndoResult::NotRemoved,
                    detail: None,
                },
            ],
            more: 0,
        },
        ScreenView::Rescue {
            stage: RescueStage::Intro,
        },
        ScreenView::Rescue {
            stage: RescueStage::PickDrive {
                drives: vec![drive(false), drive(true)],
                refused: Some(1),
            },
        },
        ScreenView::Rescue {
            stage: RescueStage::PickDrive {
                drives: vec![drive(false)],
                refused: None,
            },
        },
        ScreenView::Rescue {
            stage: RescueStage::NoSystem,
        },
        ScreenView::Rescue {
            stage: RescueStage::Salvage {
                readable: 10,
                read_only: ReadOnlyView::SystemReadOnly,
                unreadable: vec!["x".into()],
                more: 0,
            },
        },
        ScreenView::Rescue {
            stage: RescueStage::Salvage {
                readable: 10,
                read_only: ReadOnlyView::PctwinReadsOnly,
                unreadable: vec![],
                more: 0,
            },
        },
    ];
    for lock in [
        LockView::BitLocker,
        LockView::FileVault,
        LockView::AppleChip,
    ] {
        out.push(ScreenView::Rescue {
            stage: RescueStage::Locked { lock },
        });
    }
    for state in [
        LocalCopyState::Choosing,
        LocalCopyState::Working,
        LocalCopyState::PausedMetered,
        LocalCopyState::PausedNoSpace,
        LocalCopyState::Done,
    ] {
        out.push(ScreenView::LocalCopy {
            provider: Some("OneDrive".into()),
            files: 3,
            download_bytes: 300,
            old_free_bytes: if state == LocalCopyState::Done {
                None
            } else {
                Some(900)
            },
            state,
        });
    }
    let points = [
        Some(RestorePointView::Made { at: "10:04".into() }),
        Some(RestorePointView::EarlierExists { at: "08:15".into() }),
        Some(RestorePointView::NotChosen),
        Some(RestorePointView::NotMade {
            why: NotMadeWhy::NotWindows,
        }),
        Some(RestorePointView::NotMade {
            why: NotMadeWhy::ProtectionOff,
        }),
        Some(RestorePointView::NotMade {
            why: NotMadeWhy::NeedsAdmin,
        }),
        Some(RestorePointView::NotMade {
            why: NotMadeWhy::Unconfirmed,
        }),
        Some(RestorePointView::NotMade {
            why: NotMadeWhy::Failed,
        }),
    ];
    for (i, stage) in STAGES.iter().enumerate() {
        out.push(ScreenView::Permission {
            stage: *stage,
            reasons: vec![REASONS[i % REASONS.len()]],
            restore_point: points.get(i).cloned().flatten(),
        });
    }
    out.push(ScreenView::Permission {
        stage: PermissionStage::Given,
        reasons: vec![],
        restore_point: points[7].clone(),
    });
    out
}

/// Every snapshot shape: unavailable, and a view of every screen in turn, cycling the display
/// states, with progress known, unknown and absent.
pub fn snapshots() -> Vec<Snapshot> {
    let mut out = vec![Snapshot::Unavailable];
    for (i, screen) in screens().into_iter().enumerate() {
        let progress = match i % 3 {
            0 => Some(Measured {
                done: 5,
                total: Some(10),
                unit: Unit::Bytes,
            }),
            1 => Some(Measured {
                done: 5,
                total: None,
                unit: Unit::Items,
            }),
            _ => None,
        };
        out.push(Snapshot::Ready {
            view: Box::new(projection(DISPLAYS[i % DISPLAYS.len()], screen, progress)),
        });
    }
    // Any display state not reached above.
    for d in DISPLAYS {
        out.push(Snapshot::Ready {
            view: Box::new(projection(d, ScreenView::Reconciling, None)),
        });
    }
    out
}

pub fn refusals() -> Vec<Refused> {
    vec![
        Refused::Unavailable,
        Refused::WrongJob,
        Refused::OldSession,
        Refused::PlanChanged,
        Refused::NotNow,
        Refused::NotShown,
    ]
}

pub fn answers() -> Vec<Answer> {
    vec![Answer::Taken, Answer::AlreadyTaken]
}

#[test]
fn the_examples_hold_every_variant_the_screens_read() {
    fn seen(values: &[serde_json::Value], key: &str) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        fn walk(v: &serde_json::Value, key: &str, out: &mut BTreeSet<String>) {
            match v {
                serde_json::Value::Object(m) => {
                    for (k, x) in m {
                        if k == key {
                            if let Some(t) = x.get("kind").and_then(|t| t.as_str()) {
                                out.insert(t.to_owned());
                            } else if let Some(t) = x.as_str() {
                                out.insert(t.to_owned());
                            }
                        }
                        walk(x, key, out);
                    }
                }
                serde_json::Value::Array(a) => a.iter().for_each(|x| walk(x, key, out)),
                _ => {}
            }
        }
        values.iter().for_each(|v| walk(v, key, &mut out));
        out
    }
    let all: Vec<serde_json::Value> = snapshots()
        .iter()
        .map(|s| serde_json::to_value(s).unwrap())
        .collect();
    let count = |key: &str| seen(&all, key).len();
    assert_eq!(count("display"), DisplayState::VARIANTS.len());
    assert_eq!(count("screen"), ScreenView::VARIANTS.len());
    assert_eq!(count("action"), ChangeAction::VARIANTS.len());
    assert_eq!(count("undo"), UndoPromise::VARIANTS.len());
    assert_eq!(count("usedFor"), SpaceUse::VARIANTS.len());
    assert_eq!(count("to"), KeepOff::VARIANTS.len());
    assert_eq!(count("state"), LocalCopyState::VARIANTS.len());
    // "failed" is both a report line's why and a restore point's: one tag, two enums.
    assert_eq!(
        count("why"),
        ChangedWhy::VARIANTS.len() + LineWhy::VARIANTS.len() + NotMadeWhy::VARIANTS.len() - 1
    );
    assert_eq!(count("lock"), LockView::VARIANTS.len());
    assert_eq!(count("readOnly"), ReadOnlyView::VARIANTS.len());
    assert_eq!(count("what"), LineWhat::VARIANTS.len());
    assert_eq!(count("plan"), UndoPlan::VARIANTS.len());
    assert_eq!(count("result"), UndoResult::VARIANTS.len());
    assert_eq!(count("restorePoint"), RestorePointView::VARIANTS.len());
    let stages = seen(&all, "stage");
    assert_eq!(
        stages.len(),
        PermissionStage::VARIANTS.len() + RescueStage::VARIANTS.len()
    );
    let reasons: BTreeSet<String> = all
        .iter()
        .flat_map(|v| {
            let mut s = BTreeSet::new();
            fn walk(v: &serde_json::Value, s: &mut BTreeSet<String>) {
                match v {
                    serde_json::Value::Object(m) => {
                        if let Some(serde_json::Value::Array(a)) =
                            m.get("reasons").or(m.get("permissions"))
                        {
                            for r in a {
                                s.insert(r.as_str().unwrap().to_owned());
                            }
                        }
                        m.values().for_each(|x| walk(x, s));
                    }
                    serde_json::Value::Array(a) => a.iter().for_each(|x| walk(x, s)),
                    _ => {}
                }
            }
            walk(v, &mut s);
            s
        })
        .collect();
    assert_eq!(reasons.len(), PermissionReason::VARIANTS.len());
    assert_eq!(refusals().len(), Refused::VARIANTS.len());
    assert_eq!(answers().len(), Answer::VARIANTS.len());
}
