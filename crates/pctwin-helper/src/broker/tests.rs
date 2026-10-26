//! The broker's acceptance cases (T22-0xx in the Phase 2.2 acceptance matrix), with a fake
//! system approval, journal and adapters.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use sha2::{Digest, Sha256};

use super::actions::*;
use super::receipt::{Outcome, receipt_key};
use super::*;

const TOKEN: &str = "session-token-for-tests";
const PACKAGE: &[u8] = b"an approved package";

struct FakeAuthority {
    answer: Consent,
    asked: usize,
    /// Runs while the system's window is open.
    during: Option<Box<dyn FnMut()>>,
}

impl NativeAuthority for FakeAuthority {
    fn obtain(&mut self, _: &Action) -> Consent {
        self.asked += 1;
        if let Some(f) = self.during.as_mut() {
            f();
        }
        self.answer
    }
}

#[derive(Default)]
struct FakeJournal {
    down: bool,
    outcomes_down: bool,
    /// Stops the move while the intent is being written.
    revoke_on_intent: Rc<RefCell<Option<Revoker>>>,
    entries: Vec<(Intent, Option<Outcome>)>,
}

impl IntentJournal for FakeJournal {
    fn lookup(&mut self, key: &Hex32) -> Result<Recorded, JournalUnavailable> {
        if self.down {
            return Err(JournalUnavailable);
        }
        Ok(
            match self
                .entries
                .iter()
                .enumerate()
                .find(|(_, (i, _))| &i.idempotency == key)
            {
                None => Recorded::Nothing,
                Some((n, (_, None))) => Recorded::Unfinished(JournalRef(n as u64)),
                Some((n, (_, Some(o)))) => Recorded::Finished(JournalRef(n as u64), o.clone()),
            },
        )
    }

    fn record_intent(&mut self, intent: &Intent) -> Result<JournalRef, JournalUnavailable> {
        if self.down {
            return Err(JournalUnavailable);
        }
        self.entries.push((intent.clone(), None));
        if let Some(r) = self.revoke_on_intent.borrow().as_ref() {
            r.revoke();
        }
        Ok(JournalRef(self.entries.len() as u64 - 1))
    }

    fn record_outcome(&mut self, at: JournalRef, o: &Outcome) -> Result<(), JournalUnavailable> {
        if self.down || self.outcomes_down {
            return Err(JournalUnavailable);
        }
        self.entries[at.0 as usize].1 = Some(o.clone());
        Ok(())
    }
}

struct FakeAdapters {
    installs: usize,
    settings: usize,
    restore_points: usize,
    result: Effect,
    observed: Observed,
    /// Revokes the move while the installer runs (it cannot be stopped half-way).
    revoke_during: Rc<RefCell<Option<Revoker>>>,
    /// The bytes the installer was given, read from the held file.
    seen: Vec<Vec<u8>>,
}

impl Default for FakeAdapters {
    fn default() -> Self {
        Self {
            installs: 0,
            settings: 0,
            restore_points: 0,
            result: Effect::Done {
                restart_needed: false,
            },
            observed: Observed::Unknown,
            revoke_during: Rc::default(),
            seen: Vec::new(),
        }
    }
}

impl Adapters for FakeAdapters {
    fn install(&mut self, p: &VerifiedArtifact, _: &InstallVerifiedPackage, _: &Revoker) -> Effect {
        self.installs += 1;
        self.seen.push(std::fs::read(p.path()).unwrap());
        if let Some(r) = self.revoke_during.borrow().as_ref() {
            r.revoke();
        }
        self.result.clone()
    }
    fn apply_setting(&mut self, _: &NamedSetting, _: &Revoker) -> Effect {
        self.settings += 1;
        self.result.clone()
    }
    fn restore_point(&mut self, _: &Revoker) -> Effect {
        self.restore_points += 1;
        Effect::RestorePoint(restore::RestorePointResult::EarlierExists {
            at: "20261026080000.000000+000".into(),
        })
    }
    fn observe(&mut self, _: &Action) -> Observed {
        self.observed
    }
}

fn sha(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn id(s: &str) -> Id {
    Id::parse(s).unwrap()
}

fn install_action(handle: u64, scope: InstallScope) -> Action {
    Action::InstallVerifiedPackage(InstallVerifiedPackage {
        artifact: handle,
        sha256: Hex32(sha(PACKAGE)),
        kind: PackageKind::Msi,
        scope,
    })
}

fn hibernate() -> Action {
    Action::ApplyNamedSetting(ApplyNamedSetting {
        setting: NamedSetting::Hibernate(false),
    })
}

fn time_zone() -> Action {
    Action::ApplyNamedSetting(ApplyNamedSetting {
        setting: NamedSetting::TimeZone(TimeZoneName::parse("UTC").unwrap()),
    })
}

fn restore_point() -> Action {
    Action::CreateRestorePoint(CreateRestorePoint {})
}

/// A test bench: the plan, the fakes and a clock the test moves.
struct Rig {
    _dir: tempfile::TempDir,
    staging: Option<Staging>,
    package_path: std::path::PathBuf,
    plan: ApprovedPlan,
    authority: FakeAuthority,
    journal: FakeJournal,
    adapters: FakeAdapters,
    now: Rc<Cell<Now>>,
    clock: Box<dyn Fn() -> Now>,
    base: (std::time::Instant, std::time::SystemTime),
    nonce: u8,
}

impl Rig {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let mut staging = Staging::new(dir.path().to_path_buf());
        let handle = staging.admit(&mut &PACKAGE[..], sha(PACKAGE)).unwrap();
        let package_path = staging.path_of(handle).unwrap().to_path_buf();
        let op = |name: &str, action: Action| PlannedOperation {
            action_id: id(name),
            action,
        };
        let plan = ApprovedPlan {
            generation: 7,
            operations: vec![
                op("install.app", install_action(handle, InstallScope::Machine)),
                op("install.mine", install_action(handle, InstallScope::User)),
                op("setting.hibernate", hibernate()),
                op("setting.time-zone", time_zone()),
                op("restore-point", restore_point()),
            ],
        };
        let base = (std::time::Instant::now(), std::time::SystemTime::now());
        let now = Rc::new(Cell::new(Now::at(base.0, base.1)));
        let reading = Rc::clone(&now);
        Self {
            _dir: dir,
            staging: Some(staging),
            package_path,
            plan,
            authority: FakeAuthority {
                answer: Consent::Granted,
                asked: 0,
                during: None,
            },
            journal: FakeJournal::default(),
            adapters: FakeAdapters::default(),
            now,
            clock: Box::new(move || reading.get()),
            base,
            nonce: 0,
        }
    }

    fn launch(&self) -> LaunchSeal {
        LaunchSeal {
            generation: self.plan.generation,
            digest: self.plan.digest(),
        }
    }

    /// A broker with the plan sealed.
    fn broker(&mut self) -> Broker<'_> {
        let launch = self.launch();
        let plan = self.plan.clone();
        let staging = self.staging.take().expect("one broker per rig");
        let mut b = Broker::new(
            launch,
            TOKEN,
            staging,
            &mut self.authority,
            &mut self.journal,
            &mut self.adapters,
            &*self.clock,
        );
        b.seal(plan).unwrap();
        b
    }

    fn request(&mut self, action_id: &str) -> BrokerRequest {
        self.nonce += 1;
        let op = self.plan.operation(&id(action_id)).unwrap().clone();
        BrokerRequest {
            protocol: PROTOCOL,
            audience: op.action.audience(),
            request_id: id(&format!("r{}", self.nonce)),
            plan_generation: self.plan.generation,
            plan_digest: self.plan.digest(),
            action_id: op.action_id,
            action: op.action,
            nonce: Hex16([self.nonce; 16]),
        }
    }

    /// The rig's start moved on by `mono` on the forward-only clock and by `wall` seconds
    /// (negative: set back) on the calendar.
    fn at(&self, mono: Duration, wall: i64) -> Now {
        let calendar = if wall >= 0 {
            self.base.1 + Duration::from_secs(wall as u64)
        } else {
            self.base.1 - Duration::from_secs(wall.unsigned_abs())
        };
        Now::at(self.base.0 + mono, calendar)
    }
}

fn denied(a: Answer) -> Denial {
    match a {
        Answer::Denied(d) => d,
        Answer::Done(r) => panic!("not denied: {r:?}"),
    }
}

fn done(a: Answer) -> Receipt {
    match a {
        Answer::Done(r) => *r,
        Answer::Denied(d) => panic!("denied: {d:?}"),
    }
}

#[test]
fn an_approved_install_is_done_once_and_receipted() {
    let mut rig = Rig::new();
    let req = rig.request("install.app");
    let mut b = rig.broker();
    let r = done(b.handle(&req));
    drop(b);
    assert_eq!(
        r.outcome,
        Outcome::Done {
            restart_needed: false
        }
    );
    assert!(r.verify(&receipt_key(TOKEN)));
    assert!(!r.verify(&receipt_key("another session")));
    assert!(r.recorded && !r.replayed);
    assert_eq!((r.plan_generation, r.plan_digest), (7, rig.plan.digest()));
    assert_eq!(r.action_id, id("install.app"));
    assert_eq!(r.request_id, req.request_id);
    assert_eq!(rig.adapters.installs, 1);
    assert_eq!(rig.adapters.seen, [PACKAGE.to_vec()]);
    assert_eq!(rig.authority.asked, 1);
    assert_eq!(rig.journal.entries.len(), 1);
    let (intent, outcome) = &rig.journal.entries[0];
    assert_eq!(intent.action_id, id("install.app"));
    assert_eq!(intent.idempotency, r.idempotency);
    assert_eq!(intent.plan_generation, 7);
    assert_eq!(intent.plan_digest, rig.plan.digest());
    assert_eq!(outcome, &Some(r.outcome.clone()));
    assert_eq!(r.journal, 0);
}

#[test]
fn a_setting_and_a_restore_point_reach_their_own_adapters() {
    let mut rig = Rig::new();
    let s = rig.request("setting.hibernate");
    let p = rig.request("restore-point");
    let mut b = rig.broker();
    assert_eq!(
        done(b.handle(&s)).outcome,
        Outcome::Done {
            restart_needed: false
        }
    );
    let r = done(b.handle(&p));
    drop(b);
    assert!(matches!(r.outcome, Outcome::RestorePoint(_)));
    assert_eq!(
        (
            rig.adapters.installs,
            rig.adapters.settings,
            rig.adapters.restore_points
        ),
        (0, 1, 1)
    );
}

/// T22-004: the same request said again is refused; the same action asked again, with a new
/// request, gets the first result back without a second change.
#[test]
fn t22_004_a_replayed_request_makes_no_second_change() {
    let mut rig = Rig::new();
    let req = rig.request("install.app");
    let again = rig.request("install.app");
    let mut b = rig.broker();
    let first = done(b.handle(&req));
    assert_eq!(denied(b.handle(&req)), Denial::Replayed);
    let second = done(b.handle(&again));
    drop(b);
    assert!(second.replayed);
    assert_eq!(second.outcome, first.outcome);
    assert_eq!(second.idempotency, first.idempotency);
    assert_eq!(rig.adapters.installs, 1);
    assert_eq!(rig.authority.asked, 1);
}

/// T22-024: two requests for the same operation: one change, the same result for both.
#[test]
fn t22_024_two_requests_for_one_operation_make_one_change() {
    let mut rig = Rig::new();
    let a = rig.request("restore-point");
    let b2 = rig.request("restore-point");
    let mut b = rig.broker();
    let one = done(b.handle(&a));
    let two = done(b.handle(&b2));
    drop(b);
    assert_eq!(one.outcome, two.outcome);
    assert_eq!(rig.adapters.restore_points, 1);
    assert_eq!(rig.journal.entries.len(), 1);
}

/// T22-006: a request with a field the schema does not name (a claimed approval, a process ID,
/// a command) is refused when read.
#[test]
fn t22_006_unknown_fields_and_claimed_approval_are_refused() {
    let mut rig = Rig::new();
    let req = rig.request("install.app");
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(
        serde_json::from_value::<BrokerRequest>(json.clone()).unwrap(),
        req
    );
    for (field, value) in [
        ("approved", serde_json::json!(true)),
        ("pid", serde_json::json!(4)),
        ("command", serde_json::json!("cmd /c whoami")),
        ("env", serde_json::json!({"PATH": "C:/rogue"})),
        ("idempotency", serde_json::json!("x")),
    ] {
        let mut bad = json.clone();
        bad[field] = value;
        assert!(
            serde_json::from_value::<BrokerRequest>(bad).is_err(),
            "{field}"
        );
    }
}

/// T22-007: an install request from the private-data role, the planner or the screens is
/// refused, and so is any role asking for another role's action.
#[test]
fn t22_007_the_wrong_role_is_refused() {
    let mut rig = Rig::new();
    let mut reqs = Vec::new();
    for audience in [
        Audience::Data,
        Audience::Orchestrator,
        Audience::Interface,
        Audience::Broker,
        Audience::Settings,
        Audience::Recovery,
    ] {
        let mut r = rig.request("install.app");
        r.audience = audience;
        reqs.push(r);
    }
    let mut setting = rig.request("setting.hibernate");
    setting.audience = Audience::Install;
    reqs.push(setting);
    let mut b = rig.broker();
    for r in &reqs {
        assert_eq!(
            denied(b.handle(r)),
            Denial::WrongAudience,
            "{:?}",
            r.audience
        );
    }
    drop(b);
    assert_eq!(rig.adapters.installs + rig.adapters.settings, 0);
}

/// T22-008: a different plan generation or digest, an action changed from the plan's, or one not
/// in it, is refused; a plan other than the one on the command line is never sealed.
#[test]
fn t22_008_a_changed_plan_is_refused() {
    let mut rig = Rig::new();
    let mut generation = rig.request("install.app");
    generation.plan_generation += 1;
    let mut digest = rig.request("install.app");
    digest.plan_digest.0[0] ^= 1;
    let mut changed = rig.request("install.app");
    if let Action::InstallVerifiedPackage(p) = &mut changed.action {
        p.artifact += 1;
    }
    let mut unknown = rig.request("install.app");
    unknown.action_id = id("install.other");
    let mut swapped = rig.request("setting.hibernate");
    swapped.action = Action::ApplyNamedSetting(ApplyNamedSetting {
        setting: NamedSetting::Hibernate(true),
    });
    let mut b = rig.broker();
    assert_eq!(denied(b.handle(&generation)), Denial::StalePlan);
    assert_eq!(denied(b.handle(&digest)), Denial::StalePlan);
    assert_eq!(denied(b.handle(&changed)), Denial::NotInPlan);
    assert_eq!(denied(b.handle(&unknown)), Denial::NotInPlan);
    assert_eq!(denied(b.handle(&swapped)), Denial::NotInPlan);
    drop(b);
    assert_eq!(rig.adapters.installs + rig.adapters.settings, 0);
}

#[test]
fn only_the_plan_named_at_launch_is_sealed_and_only_once() {
    let mut rig = Rig::new();
    let req = rig.request("restore-point");
    let launch = rig.launch();
    let plan = rig.plan.clone();
    let mut other = plan.clone();
    other.operations.pop();
    let mut later = plan.clone();
    later.generation += 1;
    let staging = rig.staging.take().unwrap();
    let mut b = Broker::new(
        launch,
        TOKEN,
        staging,
        &mut rig.authority,
        &mut rig.journal,
        &mut rig.adapters,
        &*rig.clock,
    );
    assert_eq!(denied(b.handle(&req)), Denial::NotSealed);
    assert_eq!(b.seal(other), Err(Denial::WrongPlan));
    assert_eq!(b.seal(later), Err(Denial::WrongPlan));
    let empty = ApprovedPlan {
        generation: plan.generation,
        operations: vec![],
    };
    assert_eq!(b.seal(empty), Err(Denial::WrongPlan));
    assert_eq!(b.seal(plan.clone()), Ok(()));
    assert_eq!(b.seal(plan.clone()), Err(Denial::Replayed));
    b.close();
    assert_eq!(b.seal(plan), Err(Denial::Closed));
}

#[test]
fn launch_seals_are_read_exactly() {
    let d = "ab".repeat(32);
    assert_eq!(
        LaunchSeal::parse(&format!("12:{d}")),
        Some(LaunchSeal {
            generation: 12,
            digest: Hex32([0xab; 32])
        })
    );
    for bad in [
        format!(":{d}"),
        format!("-1:{d}"),
        format!("+1:{d}"),
        format!("1 :{d}"),
        format!("1:{}", "ab".repeat(31)),
        d.clone(),
        format!("99999999999999999999:{d}"),
    ] {
        assert_eq!(LaunchSeal::parse(&bad), None, "{bad}");
    }
}

/// T22-009: the lease ends on the forward-only clock however the calendar is set, and ends the
/// session.
#[test]
fn t22_009_an_expired_lease_gives_no_authority() {
    let mut rig = Rig::new();
    let req = rig.request("restore-point");
    let req2 = rig.request("restore-point");
    // The forward-only clock past the lease, the calendar set back a day.
    let late = rig.at(MAX_LIFETIME, -86_400);
    let back = rig.at(Duration::ZERO, 0);
    let now = Rc::clone(&rig.now);
    let mut b = rig.broker();
    now.set(late);
    assert_eq!(denied(b.handle(&req)), Denial::Expired);
    assert!(b.is_closed());
    // Closed for good, even with the clocks back where they were.
    now.set(back);
    assert_eq!(denied(b.handle(&req2)), Denial::Closed);
    drop(b);
    assert_eq!(rig.adapters.restore_points, 0);
}

#[test]
fn each_action_renews_the_lease_but_an_idle_session_ends() {
    let mut rig = Rig::new();
    let a = rig.request("restore-point");
    let c = rig.request("setting.hibernate");
    let d = rig.request("install.app");
    let second = Duration::from_secs(1);
    let nearly = rig.at(
        MAX_LIFETIME - second,
        (MAX_LIFETIME - second).as_secs() as i64,
    );
    let later = rig.at(
        MAX_LIFETIME + 60 * second,
        (MAX_LIFETIME + 60 * second).as_secs() as i64,
    );
    let idle = rig.at(
        2 * MAX_LIFETIME + 60 * second,
        (2 * MAX_LIFETIME + 60 * second).as_secs() as i64,
    );
    let now = Rc::clone(&rig.now);
    let mut b = rig.broker();
    // Most of the lease gone, then an action: the lease starts again from it.
    now.set(nearly);
    assert!(matches!(b.handle(&a), Answer::Done(_)));
    now.set(later);
    assert!(matches!(b.handle(&c), Answer::Done(_)));
    // Then a whole lease with nothing done: over.
    now.set(idle);
    assert_eq!(denied(b.handle(&d)), Denial::Expired);
}

/// T22-010: the administrator says no, or does not answer: nothing is recorded or changed.
#[test]
fn t22_010_a_refused_or_unanswered_prompt_changes_nothing() {
    for (answer, why) in [
        (Consent::Denied, Denial::NativeDenied),
        (Consent::TimedOut, Denial::NativeTimedOut),
    ] {
        let mut rig = Rig::new();
        rig.authority.answer = answer;
        let req = rig.request("install.app");
        let mut b = rig.broker();
        assert_eq!(denied(b.handle(&req)), why);
        drop(b);
        assert_eq!(rig.adapters.installs, 0);
        assert!(rig.journal.entries.is_empty());
    }
}

/// T22-011: the package changed while the system's window was open: checked again after the
/// approval, and refused.
#[test]
fn t22_011_a_package_changed_during_the_prompt_is_refused() {
    let mut rig = Rig::new();
    let path = rig.package_path.clone();
    rig.authority.during = Some(Box::new(move || {
        std::fs::write(&path, b"a different package").unwrap();
    }));
    let req = rig.request("install.app");
    let mut b = rig.broker();
    assert_eq!(denied(b.handle(&req)), Denial::Unsafe);
    drop(b);
    assert_eq!(rig.authority.asked, 1);
    assert_eq!(rig.adapters.installs, 0);
    assert!(rig.journal.entries.is_empty());
}

/// T22-016 through the broker: changed before the request, refused before the prompt.
#[test]
fn t22_016_a_package_changed_before_the_request_is_refused_before_asking() {
    let mut rig = Rig::new();
    std::fs::write(&rig.package_path, b"swapped").unwrap();
    let req = rig.request("install.app");
    let mut b = rig.broker();
    assert_eq!(denied(b.handle(&req)), Denial::Unsafe);
    drop(b);
    assert_eq!(rig.authority.asked, 0);
}

/// T22-012: stopped after checking (here, while the system's window was open): no change.
#[test]
fn t22_012_revoked_before_the_action_changes_nothing() {
    let mut rig = Rig::new();
    let req = rig.request("install.app");
    let req2 = rig.request("restore-point");
    let revoker: Rc<RefCell<Option<Revoker>>> = Rc::new(RefCell::new(None));
    let inside = Rc::clone(&revoker);
    rig.authority.during = Some(Box::new(move || {
        if let Some(r) = inside.borrow().as_ref() {
            r.revoke();
        }
    }));
    let mut b = rig.broker();
    *revoker.borrow_mut() = Some(b.revoker());
    assert_eq!(denied(b.handle(&req)), Denial::Revoked);
    assert!(b.is_closed());
    assert_eq!(denied(b.handle(&req2)), Denial::Closed);
    drop(b);
    assert_eq!(rig.adapters.installs + rig.adapters.restore_points, 0);
    assert!(rig.journal.entries.is_empty());
}

/// Stopped in the last moment, while the intent was written: still no change (the unfinished
/// intent is checked against the laptop by the next helper, as after a crash).
#[test]
fn revoked_while_recording_the_intent_changes_nothing() {
    let mut rig = Rig::new();
    let req = rig.request("restore-point");
    let slot = Rc::clone(&rig.journal.revoke_on_intent);
    let mut b = rig.broker();
    *slot.borrow_mut() = Some(b.revoker());
    assert_eq!(denied(b.handle(&req)), Denial::Revoked);
    assert!(b.is_closed());
    drop(b);
    assert_eq!(rig.adapters.restore_points, 0);
    assert_eq!(rig.journal.entries.len(), 1);
    assert_eq!(rig.journal.entries[0].1, None);
}

#[test]
fn revoked_before_asking_is_refused_at_once() {
    let mut rig = Rig::new();
    let req = rig.request("restore-point");
    let mut b = rig.broker();
    b.revoker().revoke();
    assert_eq!(denied(b.handle(&req)), Denial::Revoked);
    drop(b);
    assert_eq!(rig.authority.asked, 0);
}

/// T22-013: stopped while an installer that cannot be stopped is running: it finishes, its real
/// (partial) result is recorded and reported, and nothing new starts.
#[test]
fn t22_013_revoked_during_a_system_action_reports_the_real_effect() {
    let mut rig = Rig::new();
    let req = rig.request("install.app");
    let next = rig.request("restore-point");
    rig.adapters.result = Effect::Partial {
        code: 1603,
        native: NativeText::new("stopped half-way"),
    };
    let slot = Rc::clone(&rig.adapters.revoke_during);
    let mut b = rig.broker();
    *slot.borrow_mut() = Some(b.revoker());
    let r = done(b.handle(&req));
    assert_eq!(r.outcome, Outcome::Partial { code: 1603 });
    assert!(b.is_closed());
    assert_eq!(denied(b.handle(&next)), Denial::Closed);
    drop(b);
    assert_eq!(
        rig.journal.entries[0].1,
        Some(Outcome::Partial { code: 1603 })
    );
    assert_eq!(rig.adapters.restore_points, 0);
}

/// T22-018: the journal cannot record the intent: no change.
#[test]
fn t22_018_no_journal_no_change() {
    let mut rig = Rig::new();
    rig.journal.down = true;
    let req = rig.request("install.app");
    let mut b = rig.broker();
    assert_eq!(denied(b.handle(&req)), Denial::JournalUnavailable);
    drop(b);
    assert_eq!(rig.adapters.installs, 0);
    // The built-in "no journal yet" refuses everything too.
    let mut none = NoJournal;
    assert!(none.lookup(&Hex32::default()).is_err());
    assert!(
        none.record_intent(&Intent {
            idempotency: Hex32::default(),
            plan_generation: 0,
            plan_digest: Hex32::default(),
            action_id: id("a"),
            action: restore_point(),
        })
        .is_err()
    );
    assert!(
        none.record_outcome(JournalRef(0), &Outcome::AlreadyInPlace)
            .is_err()
    );
}

#[test]
fn an_outcome_that_could_not_be_written_is_said_so() {
    let mut rig = Rig::new();
    rig.journal.outcomes_down = true;
    let req = rig.request("restore-point");
    let mut b = rig.broker();
    let r = done(b.handle(&req));
    drop(b);
    assert!(!r.recorded);
    assert_eq!(rig.journal.entries.len(), 1);
    assert_eq!(rig.journal.entries[0].1, None);
}

/// T22-019: an install for the person only, or (on Windows) a time zone, is not done by the
/// helper: the app does it with the person's own rights.
#[test]
fn t22_019_no_administrator_needed_means_not_the_helper() {
    let mut rig = Rig::new();
    let mine = rig.request("install.mine");
    let tz = rig.request("setting.time-zone");
    let mut b = rig.broker();
    assert_eq!(denied(b.handle(&mine)), Denial::NotNeeded);
    let tz_answer = b.handle(&tz);
    drop(b);
    if cfg!(windows) {
        assert_eq!(denied(tz_answer), Denial::NotNeeded);
    } else {
        assert!(matches!(tz_answer, Answer::Done(_)));
    }
    assert_eq!(rig.adapters.installs, 0);
    assert_eq!(rig.authority.asked, if cfg!(windows) { 0 } else { 1 });
}

/// T22-021: the helper stopped after the change but before writing its outcome. The next helper
/// checks what is really there before anything is retried.
#[test]
fn t22_021_after_a_crash_the_real_state_is_checked_before_a_retry() {
    for (observed, expect) in [
        (Observed::Present, Some(Outcome::AlreadyInPlace)),
        (
            Observed::Absent,
            Some(Outcome::Done {
                restart_needed: false,
            }),
        ),
        (Observed::Unknown, None),
    ] {
        let mut rig = Rig::new();
        let first = rig.request("install.app");
        let retry = rig.request("install.app");
        // First helper: the change is made, the outcome is not written (it stopped).
        rig.journal.outcomes_down = true;
        let mut b = rig.broker();
        let _ = done(b.handle(&first));
        drop(b);
        assert_eq!(rig.adapters.installs, 1);
        assert_eq!(rig.journal.entries[0].1, None);
        // The next helper, with the same journal.
        rig.journal.outcomes_down = false;
        rig.adapters.observed = observed;
        // The package is staged again by the new helper, under the same handle.
        std::fs::remove_file(&rig.package_path).unwrap();
        let mut staging = Staging::new(rig._dir.path().to_path_buf());
        assert_eq!(staging.admit(&mut &PACKAGE[..], sha(PACKAGE)).unwrap(), 1);
        rig.staging = Some(staging);
        let mut b = rig.broker();
        let answer = b.handle(&retry);
        drop(b);
        match expect {
            Some(outcome) => {
                let r = done(answer);
                assert_eq!(r.outcome, outcome, "{observed:?}");
                assert_eq!(rig.journal.entries.len(), 1, "no second intent");
                assert_eq!(rig.journal.entries[0].1, Some(outcome));
            }
            None => assert_eq!(denied(answer), Denial::RecoveryRequired),
        }
        let installs = if observed == Observed::Absent { 2 } else { 1 };
        assert_eq!(rig.adapters.installs, installs, "{observed:?}");
    }
}

/// T22-025: an app that speaks another protocol is refused.
#[test]
fn t22_025_another_protocol_is_refused() {
    let mut rig = Rig::new();
    let mut req = rig.request("restore-point");
    req.protocol = PROTOCOL + 1;
    let mut b = rig.broker();
    assert_eq!(denied(b.handle(&req)), Denial::Incompatible);
    req.protocol = 0;
    assert_eq!(denied(b.handle(&req)), Denial::Incompatible);
}

/// T22-026: once the session ends nothing more is done, and its packages are gone.
#[test]
fn t22_026_a_closed_session_does_nothing() {
    let mut rig = Rig::new();
    let req = rig.request("install.app");
    let mut b = rig.broker();
    b.close();
    assert!(b.is_closed());
    assert_eq!(denied(b.handle(&req)), Denial::Closed);
    drop(b);
    assert!(!rig.package_path.exists());
    assert_eq!(rig.adapters.installs, 0);
}

/// T22-027: the system's words about a failure (which may hold a path or a password) never
/// leave the helper.
#[test]
fn t22_027_the_systems_words_never_leave_the_helper() {
    let mut rig = Rig::new();
    let secret = r"password=hunter2 in C:\Users\Ada\Secret Plans";
    rig.adapters.result = Effect::Failed {
        code: 1603,
        native: NativeText::new(secret),
    };
    let req = rig.request("install.app");
    let mut b = rig.broker();
    let answer = b.handle(&req);
    drop(b);
    let r = done(answer.clone());
    assert_eq!(r.outcome, Outcome::Failed { code: 1603 });
    let wire = serde_json::to_string(&r).unwrap();
    let shown = format!("{answer:?} {:?}", rig.adapters.result);
    for text in [&wire, &shown] {
        assert!(!text.contains("hunter2"), "{text}");
        assert!(!text.contains("Ada"), "{text}");
    }
    assert!(shown.contains("redacted"));
    // Every refusal is a fixed sentence.
    for d in [
        Denial::Incompatible,
        Denial::Closed,
        Denial::NotSealed,
        Denial::WrongPlan,
        Denial::Expired,
        Denial::Revoked,
        Denial::Replayed,
        Denial::TooMany,
        Denial::WrongAudience,
        Denial::StalePlan,
        Denial::NotInPlan,
        Denial::NotNeeded,
        Denial::Unsafe,
        Denial::InUse,
        Denial::NativeDenied,
        Denial::NativeTimedOut,
        Denial::JournalUnavailable,
        Denial::RecoveryRequired,
        Denial::NotAvailable,
    ] {
        let s = d.to_string();
        assert!(!s.is_empty() && !s.contains('{'), "{d:?}");
    }
}

#[test]
fn a_session_answers_a_bounded_number_of_requests() {
    let mut rig = Rig::new();
    let template = rig.request("install.mine");
    let mut b = rig.broker();
    for n in 0..MAX_REQUESTS {
        let mut r = template.clone();
        r.nonce = Hex16([0; 16]);
        r.nonce.0[..8].copy_from_slice(&(n as u64).to_be_bytes());
        assert_eq!(denied(b.handle(&r)), Denial::NotNeeded);
    }
    let mut r = template;
    r.nonce = Hex16([0xff; 16]);
    assert_eq!(denied(b.handle(&r)), Denial::TooMany);
}

#[test]
fn the_built_in_adapters_claim_nothing_they_cannot_do() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Staging::new(dir.path().to_path_buf());
    let h = s.admit(&mut &b"x"[..], sha(b"x")).unwrap();
    let v = s.open_verified(h, sha(b"x")).unwrap();
    let mut a = BuiltInAdapters;
    let stop = Revoker::default();
    let p = InstallVerifiedPackage {
        artifact: h,
        sha256: Hex32(sha(b"x")),
        kind: PackageKind::Msi,
        scope: InstallScope::Machine,
    };
    assert!(matches!(
        a.install(&v, &p, &stop),
        Effect::Failed {
            code: NOT_AVAILABLE,
            ..
        }
    ));
    assert!(matches!(
        a.apply_setting(&NamedSetting::Hibernate(true), &stop),
        Effect::Failed {
            code: NOT_AVAILABLE,
            ..
        }
    ));
    assert_eq!(a.observe(&restore_point()), Observed::Unknown);
    #[cfg(not(windows))]
    assert_eq!(
        a.restore_point(&stop),
        Effect::RestorePoint(restore::RestorePointResult::NotMade {
            reason: restore::NotMadeReason::NotWindows
        })
    );
    assert_eq!(SessionElevation.obtain(&restore_point()), Consent::Granted);
}

/// The channel to the app: requests in, answers out; it can break after some answers.
struct Duplex {
    input: std::io::Cursor<Vec<u8>>,
    output: Vec<u8>,
    answers_before_break: Option<usize>,
}

impl Duplex {
    fn new(requests: &[crate::Request]) -> Self {
        let mut input = Vec::new();
        for r in requests {
            crate::protocol::send_request(&mut input, r).unwrap();
        }
        Self {
            input: std::io::Cursor::new(input),
            output: Vec::new(),
            answers_before_break: None,
        }
    }

    fn answers(&self) -> Vec<crate::Answer> {
        let mut read = self.output.as_slice();
        let mut all = Vec::new();
        while !read.is_empty() {
            match crate::protocol::read_from_helper(&mut read).unwrap() {
                crate::FromHelper::Answer(a) => all.push(a),
                other => panic!("not an answer: {other:?}"),
            }
        }
        all
    }
}

impl std::io::Read for Duplex {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.input.read(buf)
    }
}

impl std::io::Write for Duplex {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.answers_before_break == Some(0) {
            return Err(std::io::ErrorKind::BrokenPipe.into());
        }
        self.output.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        if let Some(n) = self.answers_before_break.as_mut() {
            *n = n.saturating_sub(1);
        }
        Ok(())
    }
}

fn no_scan(_: &str) -> Result<crate::person::PersonScan, crate::Refusal> {
    Err(crate::Refusal::NotApproved)
}

/// The whole conversation over the channel: hello, the plan, an action, goodbye; then the
/// broker is closed.
#[test]
fn a_session_seals_acts_and_closes() {
    use crate::{Answer as A, Request as R};
    let mut rig = Rig::new();
    let act = rig.request("install.app");
    let plan = rig.plan.clone();
    let mut scan = no_scan;
    let mut channel = Duplex::new(&[
        R::Hello {
            session: TOKEN.into(),
        },
        R::SealPlan { plan: plan.clone() },
        R::SealPlan { plan },
        R::Act(Box::new(act.clone())),
        R::Act(Box::new(act)),
        R::Bye,
    ]);
    let launch = rig.launch();
    let staging = rig.staging.take().unwrap();
    let broker = Broker::new(
        launch,
        TOKEN,
        staging,
        &mut rig.authority,
        &mut rig.journal,
        &mut rig.adapters,
        &*rig.clock,
    );
    let mut session =
        crate::Session::new(TOKEN.into(), Default::default(), &mut scan).with_broker(broker);
    session.serve(&mut channel).unwrap();
    assert!(session.broker().unwrap().is_closed());
    drop(session);
    let answers = channel.answers();
    assert_eq!(answers[0], A::Ready);
    assert_eq!(answers[1], A::Sealed);
    assert_eq!(answers[2], A::Denied(Denial::Replayed));
    let A::Receipt(r) = &answers[3] else {
        panic!("{:?}", answers[3]);
    };
    assert!(r.verify(&receipt_key(TOKEN)));
    assert_eq!(answers[4], A::Denied(Denial::Replayed));
    assert_eq!(answers[5], A::Goodbye);
    assert_eq!(rig.adapters.installs, 1);
    // Its packages went with it.
    assert!(!rig.package_path.exists());
}

#[test]
fn nothing_reaches_the_broker_before_hello_or_without_one() {
    use crate::{Answer as A, Request as R};
    let mut rig = Rig::new();
    let act = rig.request("restore-point");
    let mut scan = no_scan;
    let mut channel = Duplex::new(&[R::Act(Box::new(act.clone()))]);
    crate::Session::new(TOKEN.into(), Default::default(), &mut scan)
        .serve(&mut channel)
        .unwrap();
    assert_eq!(channel.answers(), [A::Refused(crate::Refusal::NoSession)]);
    // A helper started without a plan has no broker.
    let mut scan = no_scan;
    let mut channel = Duplex::new(&[
        R::Hello {
            session: TOKEN.into(),
        },
        R::SealPlan {
            plan: rig.plan.clone(),
        },
        R::Act(Box::new(act)),
        R::Revoke,
    ]);
    crate::Session::new(TOKEN.into(), Default::default(), &mut scan)
        .serve(&mut channel)
        .unwrap();
    assert_eq!(
        channel.answers()[1..],
        [
            A::Denied(Denial::NotAvailable),
            A::Denied(Denial::NotAvailable),
            A::Denied(Denial::NotAvailable)
        ]
    );
}

/// The app stops the move: nothing more is done in this session.
#[test]
fn the_apps_stop_ends_the_brokers_work() {
    use crate::{Answer as A, Request as R};
    let mut rig = Rig::new();
    let act = rig.request("restore-point");
    let plan = rig.plan.clone();
    let mut scan = no_scan;
    let mut channel = Duplex::new(&[
        R::Hello {
            session: TOKEN.into(),
        },
        R::SealPlan { plan },
        R::Revoke,
        R::Act(Box::new(act)),
    ]);
    let launch = rig.launch();
    let staging = rig.staging.take().unwrap();
    let broker = Broker::new(
        launch,
        TOKEN,
        staging,
        &mut rig.authority,
        &mut rig.journal,
        &mut rig.adapters,
        &*rig.clock,
    );
    crate::Session::new(TOKEN.into(), Default::default(), &mut scan)
        .with_broker(broker)
        .serve(&mut channel)
        .unwrap();
    assert_eq!(
        channel.answers()[2..],
        [A::Denied(Denial::Revoked), A::Denied(Denial::Closed)]
    );
    assert_eq!(rig.adapters.restore_points, 0);
}

/// T22-022: the app goes away while an action runs. The action reaches its end, its outcome is
/// in the journal before the answer is sent, and the broker is closed.
#[test]
fn t22_022_the_app_gone_mid_action_leaves_a_recorded_outcome_and_a_closed_broker() {
    use crate::Request as R;
    let mut rig = Rig::new();
    let act = rig.request("install.app");
    let plan = rig.plan.clone();
    let mut scan = no_scan;
    let mut channel = Duplex::new(&[
        R::Hello {
            session: TOKEN.into(),
        },
        R::SealPlan { plan },
        R::Act(Box::new(act)),
    ]);
    // Ready and Sealed get through; the receipt does not.
    channel.answers_before_break = Some(2);
    let launch = rig.launch();
    let staging = rig.staging.take().unwrap();
    let broker = Broker::new(
        launch,
        TOKEN,
        staging,
        &mut rig.authority,
        &mut rig.journal,
        &mut rig.adapters,
        &*rig.clock,
    );
    let mut session =
        crate::Session::new(TOKEN.into(), Default::default(), &mut scan).with_broker(broker);
    assert!(session.serve(&mut channel).is_err());
    assert!(session.broker().unwrap().is_closed());
    drop(session);
    assert_eq!(rig.adapters.installs, 1);
    assert_eq!(
        rig.journal.entries[0].1,
        Some(Outcome::Done {
            restart_needed: false
        })
    );
}

/// T22-005: a request frame longer than the limit is refused before its body is read.
#[test]
fn t22_005_an_oversized_request_is_refused_unread() {
    let mut wire = ((crate::protocol::MAX_REQUEST + 1) as u32)
        .to_be_bytes()
        .to_vec();
    wire.extend(std::iter::repeat_n(b'{', 16));
    assert!(matches!(
        crate::protocol::read_request(&mut wire.as_slice()),
        Err(crate::protocol::FrameError::TooLong)
    ));
    // A plan too big for one request cannot even be sent.
    let rig = Rig::new();
    let mut huge = rig.plan.clone();
    let op = huge.operations[0].clone();
    huge.operations = (0..2000)
        .map(|i| PlannedOperation {
            action_id: id(&format!("op-{i:0>50}")),
            action: op.action.clone(),
        })
        .collect();
    assert!(matches!(
        crate::protocol::send_request(&mut Vec::new(), &crate::Request::SealPlan { plan: huge }),
        Err(crate::protocol::FrameError::TooLong)
    ));
}
