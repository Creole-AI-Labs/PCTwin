//! Undo of the apps PCTwin installed (decided 28 October 2026), proven up to the administrator
//! helper's boundary with nothing removed and nothing run elevated: a Windows Installer app the
//! helper recorded installing is removed through the helper only while it is still the version
//! installed, and only when Windows' list shows it gone afterwards; another version is kept and
//! said; one already gone is said; an app installed by an installer program is listed for the
//! person. Nothing is ever said removed unless the list shows it gone.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use pctwin_apps::found::windows::{RawEntry, RegValue};
use pctwin_fullmove::install::{Helper, HelperDown, Machine};
use pctwin_fullmove::run::{Removable, RemovableApp};
use pctwin_fullmove::undo::{AppUndo, HelperUndo, ListOnly, Result, UndoPlan};
use pctwin_helper::Acted;
use pctwin_helper::broker::actions::{
    Action, ApprovedPlan, Hex32, InstalledProduct, ProductCode, ProductVersion,
};
use pctwin_helper::broker::receipt::{Outcome, Receipt};
use pctwin_helper::broker::uninstall::UninstallResult;
use pctwin_helper::broker::{BrokerRequest, Denial};
use pctwin_record::RegistryView;

const WRITER: &str = "{3B0346D3-155D-43E5-9561-B9BE51CF2EB6}";
const VIEWER: &str = "{11111111-2222-3333-4444-555555555555}";

fn entry(code: &str, version: &str) -> RawEntry {
    let mut values = BTreeMap::new();
    values.insert("displayname".into(), RegValue::Text("An app".into()));
    values.insert("displayversion".into(), RegValue::Text(version.into()));
    values.insert("windowsinstaller".into(), RegValue::Number(1));
    RawEntry {
        view: RegistryView::Machine,
        key: code.into(),
        values,
    }
}

fn app(name: &str, code: &str, version: &str, key: u8) -> RemovableApp {
    RemovableApp {
        name: name.into(),
        removable: Removable {
            product: InstalledProduct {
                code: ProductCode::parse(code).unwrap(),
                version: ProductVersion::parse(version).unwrap(),
            },
            installed_by: Hex32([key; 32]),
        },
    }
}

/// Windows' list, shared by the fake machine and the fake helper (which removes from it).
type List = Rc<RefCell<Vec<RawEntry>>>;

struct FakeMachine(List);

impl Machine for FakeMachine {
    fn entries(&self) -> Vec<RawEntry> {
        self.0.borrow().clone()
    }
}

/// What the helper does for a removal.
#[derive(Clone)]
enum Does {
    /// Removes it from the list and says so.
    Remove {
        restart_needed: bool,
    },
    /// Says removed, but the list still shows it.
    SaysRemoved,
    Answers(Outcome),
    Refuses(Denial),
    Unreachable,
}

struct FakeHelper {
    list: List,
    declines: bool,
    does: Does,
    started: Vec<ApprovedPlan>,
    asked: Vec<BrokerRequest>,
    bye: bool,
}

impl FakeHelper {
    fn new(list: &List, does: Does) -> Self {
        Self {
            list: Rc::clone(list),
            declines: false,
            does,
            started: Vec::new(),
            asked: Vec::new(),
            bye: false,
        }
    }
}

fn receipt(r: &BrokerRequest, outcome: Outcome) -> Receipt {
    Receipt {
        request_id: r.request_id.clone(),
        action_id: r.action_id.clone(),
        plan_generation: r.plan_generation,
        plan_digest: r.plan_digest,
        idempotency: Hex32([9; 32]),
        outcome,
        journal: 1,
        recorded: true,
        replayed: false,
        mac: Hex32([0; 32]),
    }
}

impl Helper for FakeHelper {
    fn start(&mut self, plan: &ApprovedPlan) -> std::result::Result<(), HelperDown> {
        self.started.push(plan.clone());
        if self.declines {
            Err(HelperDown::Declined)
        } else {
            Ok(())
        }
    }
    fn stage(
        &mut self,
        _: u64,
        _: Hex32,
        _: u64,
        _: &mut dyn std::io::Read,
    ) -> std::result::Result<(), String> {
        Err("a removal stages nothing".into())
    }
    fn act(&mut self, request: &BrokerRequest) -> std::result::Result<Acted, String> {
        self.asked.push(request.clone());
        let plan = self.started.last().unwrap();
        if request.plan_digest != plan.digest()
            || plan.operation(&request.action_id).map(|o| &o.action) != Some(&request.action)
        {
            return Ok(Acted::Denied(Denial::NotInPlan));
        }
        let Action::UninstallWindowsInstallerProduct(u) = &request.action else {
            return Ok(Acted::Denied(Denial::NotInPlan));
        };
        let outcome = match self.does.clone() {
            Does::Remove { restart_needed } => {
                self.list
                    .borrow_mut()
                    .retain(|e| e.key != u.product_code.as_str());
                Outcome::Uninstall(UninstallResult::Removed { restart_needed })
            }
            Does::SaysRemoved => Outcome::Uninstall(UninstallResult::Removed {
                restart_needed: false,
            }),
            Does::Answers(o) => o,
            Does::Refuses(d) => return Ok(Acted::Denied(d)),
            Does::Unreachable => return Err("the helper went away".into()),
        };
        Ok(Acted::Done(Box::new(receipt(request, outcome))))
    }
    fn bye(&mut self) {
        self.bye = true;
    }
}

struct Bench {
    list: List,
    helper: FakeHelper,
}

impl Bench {
    fn new(entries: Vec<RawEntry>, does: Does) -> Self {
        let list = Rc::new(RefCell::new(entries));
        let helper = FakeHelper::new(&list, does);
        Self { list, helper }
    }

    fn plan(&mut self, apps: &[RemovableApp]) -> Vec<UndoPlan> {
        let machine = FakeMachine(Rc::clone(&self.list));
        let mut n = 0u8;
        let mut nonce = || {
            n += 1;
            Some([n; 16])
        };
        let mut undo = HelperUndo {
            helper: &mut self.helper,
            machine: &machine,
            nonce: &mut nonce,
        };
        apps.iter().map(|a| undo.plan(a)).collect()
    }

    fn remove(&mut self, apps: &[RemovableApp]) -> Vec<Result> {
        let machine = FakeMachine(Rc::clone(&self.list));
        let mut n = 0u8;
        let mut nonce = || {
            n += 1;
            Some([n; 16])
        };
        let mut undo = HelperUndo {
            helper: &mut self.helper,
            machine: &machine,
            nonce: &mut nonce,
        };
        undo.remove(apps)
    }
}

/// The preview looks at Windows' list now: the version installed is removed; another version is
/// kept; one already gone is said. Nothing is asked of the helper.
#[test]
fn the_preview_says_what_undo_will_do_with_each_app() {
    let apps = [
        app("Writer", WRITER, "2.1.0", 1),
        app("Viewer", VIEWER, "5.0", 2),
        app("Gone", "{22222222-2222-3333-4444-555555555555}", "1.0", 3),
    ];
    let mut b = Bench::new(
        vec![entry(WRITER, "2.1.0"), entry(VIEWER, "5.1")],
        Does::Remove {
            restart_needed: false,
        },
    );
    assert_eq!(
        b.plan(&apps),
        [
            UndoPlan::RemoveApp,
            UndoPlan::KeepOtherVersion,
            UndoPlan::AlreadyGone
        ]
    );
    assert!(b.helper.started.is_empty() && b.helper.asked.is_empty());
}

/// Step 3's acceptance: one prompt, for a plan of exactly the removals of the versions PCTwin
/// installed, each naming the product code and the install the helper recorded it by; removed
/// only when the list shows it gone.
#[test]
fn the_version_installed_is_removed_through_the_helper_and_checked_gone() {
    let apps = [
        app("Writer", WRITER, "2.1.0", 1),
        app("Viewer", VIEWER, "5.0", 2),
        app("Gone", "{22222222-2222-3333-4444-555555555555}", "1.0", 3),
    ];
    let mut b = Bench::new(
        vec![entry(WRITER, "2.1.0"), entry(VIEWER, "5.1")],
        Does::Remove {
            restart_needed: false,
        },
    );
    let got = b.remove(&apps);
    assert_eq!(
        got,
        [
            Result::Removed {
                restart_needed: false
            },
            Result::KeptOtherVersion,
            Result::AsBefore
        ]
    );
    // One prompt, for the one app to remove; the other version and the gone app were never sent.
    assert_eq!(b.helper.started.len(), 1);
    let ops = &b.helper.started[0].operations;
    assert_eq!(ops.len(), 1);
    let Action::UninstallWindowsInstallerProduct(u) = &ops[0].action else {
        panic!("{ops:?}")
    };
    assert_eq!(u.product_code, ProductCode::parse(WRITER).unwrap());
    assert_eq!(u.installed_by, Hex32([1; 32]));
    assert_eq!(b.helper.asked.len(), 1);
    assert!(b.helper.bye);
    // The other version is still there.
    assert_eq!(b.list.borrow().len(), 1);
}

#[test]
fn a_restart_that_finishes_it_is_said() {
    let mut b = Bench::new(
        vec![entry(WRITER, "2.1.0"), entry(VIEWER, "5.1")],
        Does::Remove {
            restart_needed: true,
        },
    );
    assert_eq!(
        b.remove(&[app("Writer", WRITER, "2.1.0", 1)]),
        [Result::Removed {
            restart_needed: true
        }]
    );
}

/// Nothing to remove: no prompt at all.
#[test]
fn nothing_to_remove_asks_for_no_administrator() {
    let mut b = Bench::new(
        vec![entry(VIEWER, "5.1")],
        Does::Remove {
            restart_needed: false,
        },
    );
    let got = b.remove(&[
        app("Viewer", VIEWER, "5.0", 2),
        app("Writer", WRITER, "2.1.0", 1),
    ]);
    assert_eq!(got, [Result::KeptOtherVersion, Result::AsBefore]);
    assert!(b.helper.started.is_empty());
    assert!(!b.helper.bye);
}

/// Never shown removed unless the list shows it gone: the helper's word alone is not enough,
/// and a declined prompt, a refusal, a failure or a helper that went away are each said.
#[test]
fn never_removed_unless_the_list_shows_it_gone() {
    let one = [app("Writer", WRITER, "2.1.0", 1)];
    for (why, does) in [
        ("says removed, still listed", Does::SaysRemoved),
        (
            "msiexec failed",
            Does::Answers(Outcome::Failed { code: 1603 }),
        ),
        ("part way", Does::Answers(Outcome::Partial { code: -4 })),
        (
            "says already gone, still listed",
            Does::Answers(Outcome::Uninstall(UninstallResult::AlreadyGone)),
        ),
        (
            "refused by the helper",
            Does::Refuses(Denial::NotInstalledByPctwin),
        ),
        ("helper went away", Does::Unreachable),
        (
            "an answer for something else",
            Does::Answers(Outcome::Done {
                restart_needed: false,
            }),
        ),
    ] {
        let mut b = Bench::new(vec![entry(WRITER, "2.1.0")], does);
        assert_eq!(b.remove(&one), [Result::NotRemoved], "{why}");
    }
    // The helper found another version there by the time it looked: kept, said.
    let mut b = Bench::new(
        vec![entry(WRITER, "2.1.0")],
        Does::Answers(Outcome::Uninstall(UninstallResult::KeptOtherVersion)),
    );
    assert_eq!(b.remove(&one), [Result::KeptOtherVersion]);
    // The person said no at the prompt.
    let mut b = Bench::new(
        vec![entry(WRITER, "2.1.0")],
        Does::Remove {
            restart_needed: false,
        },
    );
    b.helper.declines = true;
    assert_eq!(b.remove(&one), [Result::NotRemoved]);
    assert!(b.helper.asked.is_empty());
    assert_eq!(b.list.borrow().len(), 1);
}

/// Where no helper removes apps (not Windows), each is listed for the person.
#[test]
fn without_the_helper_each_app_is_left_for_the_person() {
    let apps = [app("Writer", WRITER, "2.1.0", 1)];
    let mut list = ListOnly;
    assert_eq!(list.plan(&apps[0]), UndoPlan::RemoveYourself);
    assert_eq!(list.remove(&apps), [Result::RemoveYourself]);
}

/// Each undo attempt is its own set of actions (a failed removal is tried again next time, not
/// answered from the helper's record of the last attempt), and every ID is one the helper reads.
#[test]
fn each_attempt_names_its_own_actions() {
    let one = [app("Writer", WRITER, "2.1.0", 1)];
    let mut first = Bench::new(
        vec![entry(WRITER, "2.1.0")],
        Does::Answers(Outcome::Failed { code: 1618 }),
    );
    first.remove(&one);
    let mut second = Bench::new(
        vec![entry(WRITER, "2.1.0")],
        Does::Answers(Outcome::Failed { code: 1618 }),
    );
    // A different one-use number makes a different attempt.
    second.helper.started.clear();
    let machine = FakeMachine(Rc::clone(&second.list));
    let mut n = 100u8;
    let mut nonce = || {
        n += 1;
        Some([n; 16])
    };
    HelperUndo {
        helper: &mut second.helper,
        machine: &machine,
        nonce: &mut nonce,
    }
    .remove(&one);
    let a = &first.helper.started[0];
    let b = &second.helper.started[0];
    assert_ne!(a.operations[0].action_id, b.operations[0].action_id);
    assert_ne!(a.digest(), b.digest());
    assert!(
        a.operations[0].action_id.as_str().starts_with("uninstall."),
        "{:?}",
        a.operations[0].action_id
    );
}

/// A list that cannot be read (Windows' list is never empty) is never taken as "gone": the
/// helper looks itself, and nothing is said removed unless the list shows it gone.
#[test]
fn an_unreadable_list_is_never_taken_as_gone() {
    let one = [app("Writer", WRITER, "2.1.0", 1)];
    let mut b = Bench::new(
        vec![],
        Does::Answers(Outcome::Uninstall(UninstallResult::Removed {
            restart_needed: false,
        })),
    );
    assert_eq!(b.plan(&one), [UndoPlan::RemoveApp]);
    assert_eq!(b.remove(&one), [Result::NotRemoved]);
    assert_eq!(b.helper.asked.len(), 1);
}
