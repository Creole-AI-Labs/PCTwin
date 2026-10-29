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
    /// Can be read, but cannot write a new intent durably.
    intents_down: bool,
    /// Cannot write a new intent once it holds this many entries.
    intents_down_after: Option<usize>,
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
        if self.down
            || self.intents_down
            || self
                .intents_down_after
                .is_some_and(|n| self.entries.len() >= n)
        {
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
    /// The products the removal adapter was asked to remove, and what it answers.
    uninstalled: Vec<InstalledProduct>,
    uninstall_result: Effect,
    /// What the helper's own maker check says, and the bytes it was asked about.
    maker: super::maker::MakerCheck,
    maker_checked: Vec<Vec<u8>>,
}

fn test_maker() -> pctwin_maker::facts::WindowsSigner {
    pctwin_maker::facts::WindowsSigner {
        subject: pctwin_maker::facts::Subject {
            common_name: Some("Example Ltd".into()),
            organization: Some("Example Ltd".into()),
            country: Some("NG".into()),
            ..Default::default()
        },
        certificate: pctwin_maker::facts::Sha256([1; 32]),
        issuer: "CN=Some CA".into(),
        issuer_organization: Some("Some CA".into()),
        public_key: None,
    }
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
            uninstalled: Vec::new(),
            uninstall_result: Effect::Uninstall(uninstall::UninstallResult::Removed {
                restart_needed: false,
            }),
            maker: super::maker::MakerCheck::Approved,
            maker_checked: Vec::new(),
        }
    }
}

impl Adapters for FakeAdapters {
    fn check_maker(
        &mut self,
        p: &VerifiedArtifact,
        _: &InstallVerifiedPackage,
    ) -> super::maker::MakerCheck {
        self.maker_checked.push(std::fs::read(p.path()).unwrap());
        self.maker
    }
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
            number: 5,
        })
    }
    fn uninstall(
        &mut self,
        _: &UninstallWindowsInstallerProduct,
        installed: &InstalledProduct,
        _: &Revoker,
    ) -> Effect {
        self.uninstalled.push(installed.clone());
        self.uninstall_result.clone()
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
        product_code: None,
        for_move: None,
        artifact: handle,
        sha256: Hex32(sha(PACKAGE)),
        kind: PackageKind::Msi,
        scope,
        maker: Box::new(test_maker()),
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
        Self::with(Vec::new())
    }

    /// The usual plan, and `extra` operations after it.
    fn with(extra: Vec<PlannedOperation>) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let mut staging = Staging::new(crate::broker::staging::HelperFolder::for_tests(
            dir.path().to_path_buf(),
        ));
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
            ]
            .into_iter()
            .chain(extra)
            .collect(),
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

/// The command line's generation must be the plan's own, even when the digest matches; and a
/// plan that is not well formed is never sealed, even when the command line names it.
#[test]
fn a_seal_must_match_in_every_part() {
    let mut rig = Rig::new();
    let plan = rig.plan.clone();
    let mut twice = plan.clone();
    let first = twice.operations[0].clone();
    twice.operations.push(first);
    let seals = [
        (
            LaunchSeal {
                generation: plan.generation + 1,
                digest: plan.digest(),
            },
            plan.clone(),
        ),
        (
            LaunchSeal {
                generation: twice.generation,
                digest: twice.digest(),
            },
            twice,
        ),
    ];
    for (launch, offered) in seals {
        let staging = Staging::new(crate::broker::staging::HelperFolder::for_tests(
            rig._dir.path().to_path_buf(),
        ));
        let mut b = Broker::new(
            launch,
            TOKEN,
            staging,
            &mut rig.authority,
            &mut rig.journal,
            &mut rig.adapters,
            &*rig.clock,
        );
        assert_eq!(b.seal(offered), Err(Denial::WrongPlan));
    }
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

/// T22-018, the other way round: the journal can be read but the intent cannot be written.
#[test]
fn t22_018_an_intent_that_cannot_be_written_means_no_change() {
    let mut rig = Rig::new();
    rig.journal.intents_down = true;
    let req = rig.request("restore-point");
    let mut b = rig.broker();
    assert_eq!(denied(b.handle(&req)), Denial::JournalUnavailable);
    drop(b);
    assert_eq!(rig.adapters.restore_points, 0);
    assert!(rig.journal.entries.is_empty());
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
        let mut staging = Staging::new(crate::broker::staging::HelperFolder::for_tests(
            rig._dir.path().to_path_buf(),
        ));
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
    let mut s = Staging::new(crate::broker::staging::HelperFolder::for_tests(
        dir.path().to_path_buf(),
    ));
    let h = s.admit(&mut &b"x"[..], sha(b"x")).unwrap();
    let v = s.open_verified(h, sha(b"x")).unwrap();
    let mut a = BuiltInAdapters;
    let stop = Revoker::default();
    let p = InstallVerifiedPackage {
        product_code: None,
        for_move: None,
        artifact: h,
        sha256: Hex32(sha(b"x")),
        kind: PackageKind::Msi,
        // For the person alone: the helper never runs an installer for that, so nothing is
        // started here (the real install is proven only by the administrator CI jobs).
        scope: InstallScope::User,
        maker: Box::new(test_maker()),
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
    assert_eq!(a.observe(&restore_point()), Observed::Absent);
    assert_eq!(a.observe(&hibernate()), Observed::Unknown);
    // Windows' own list holds no product from these bytes; elsewhere it cannot be looked at.
    assert_eq!(
        a.observe(&install_action(1, InstallScope::Machine)),
        if cfg!(windows) {
            Observed::Absent
        } else {
            Observed::Unknown
        }
    );
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

/// Review finding 6: however busy, a session ends at a fixed deadline set when the plan was
/// sealed: acting renews the lease, never the deadline.
#[test]
fn a_busy_session_still_ends_at_its_fixed_deadline() {
    let mut rig = Rig::new();
    rig.plan.operations = (0..24)
        .map(|n| PlannedOperation {
            action_id: id(&format!("rp.{n}")),
            action: restore_point(),
        })
        .collect();
    let requests: Vec<BrokerRequest> = (0..24).map(|n| rig.request(&format!("rp.{n}"))).collect();
    let step = Duration::from_secs(14 * 60);
    let times: Vec<Now> = (1..=24u32)
        .map(|n| rig.at(step * n, i64::from(n) * 14 * 60))
        .collect();
    let now = Rc::clone(&rig.now);
    let mut b = rig.broker();
    let mut ended = None;
    for (n, (r, t)) in requests.iter().zip(&times).enumerate() {
        now.set(*t);
        match b.handle(r) {
            Answer::Done(_) => {}
            Answer::Denied(d) => {
                ended = Some((n, d));
                break;
            }
        }
    }
    let (n, why) = ended.expect("the session ended");
    assert_eq!(why, Denial::Expired);
    // Every action inside the deadline was done; the first after it was not.
    let ended_at = step * (n as u32 + 1);
    assert!(ended_at > pctwin_grant::MAX_SESSION, "{ended_at:?}");
    assert!(ended_at <= pctwin_grant::MAX_SESSION + step, "{ended_at:?}");
    assert!(b.is_closed());
}

/// This test program's role keys, made once and lent to each test.
fn role_keys() -> &'static pctwin_grant::RoleKeys {
    static KEYS: std::sync::OnceLock<pctwin_grant::RoleKeys> = std::sync::OnceLock::new();
    KEYS.get_or_init(|| pctwin_grant::RoleKeys::mint().expect("the keys are made once"))
}

/// A journal whose outcome writes fail: the helper stopping between the change and its outcome.
struct StopsBeforeOutcome<'a>(&'a mut dyn IntentJournal);

impl IntentJournal for StopsBeforeOutcome<'_> {
    fn lookup(&mut self, k: &Hex32) -> Result<Recorded, JournalUnavailable> {
        self.0.lookup(k)
    }
    fn record_intent(&mut self, i: &Intent) -> Result<JournalRef, JournalUnavailable> {
        self.0.record_intent(i)
    }
    fn record_outcome(&mut self, _: JournalRef, _: &Outcome) -> Result<(), JournalUnavailable> {
        Err(JournalUnavailable)
    }
}

/// Join of 2.2 and 2.3 (b), with the real journal on disk: the intent is durable before the
/// change; the helper stops before the outcome; the next helper, on the same journal, checks
/// what is really there before anything is retried, and never installs twice; asked once more,
/// the finished result is handed back from the journal.
#[test]
fn the_real_journal_keeps_the_intent_first_and_a_crash_is_reconciled() {
    let journal_dir = tempfile::tempdir().unwrap();
    let path = journal_dir.path().join("helper.redb");
    let open = || {
        pctwin_journal::Journal::open_with_record_key(
            &path,
            pctwin_journal::keystore::TagKey::from_bytes([3; 32]),
        )
        .unwrap()
    };
    let mut rig = Rig::new();
    let first = rig.request("install.app");
    let retry = rig.request("install.app");
    let again = rig.request("install.app");
    let launch = rig.launch();
    // The first helper: the change is made; it stops before writing the outcome.
    {
        let j = open();
        let mut inner = journal::JournalIntents::new(j.broker(&role_keys().broker));
        let mut stopping = StopsBeforeOutcome(&mut inner);
        let mut b = Broker::new(
            launch,
            TOKEN,
            rig.staging.take().unwrap(),
            &mut rig.authority,
            &mut stopping,
            &mut rig.adapters,
            &*rig.clock,
        );
        b.seal(rig.plan.clone()).unwrap();
        let r = done(b.handle(&first));
        assert!(!r.recorded);
    }
    assert_eq!(rig.adapters.installs, 1);
    {
        let j = open();
        let key = super::Hex32(*first_key(&rig, &first).as_bytes());
        assert!(matches!(
            j.broker(&role_keys().broker).lookup(&key.0).unwrap(),
            pctwin_journal::BrokerState::Unfinished(_)
        ));
    }
    // The next helper: the app is there now.
    rig.adapters.observed = Observed::Present;
    for (request, replayed_from_record) in [(&retry, false), (&again, true)] {
        let j = open();
        let mut intents = journal::JournalIntents::new(j.broker(&role_keys().broker));
        let mut staging = Staging::new(crate::broker::staging::HelperFolder::for_tests(
            rig._dir.path().to_path_buf(),
        ));
        assert_eq!(staging.admit(&mut &PACKAGE[..], sha(PACKAGE)).unwrap(), 1);
        let mut b = Broker::new(
            launch,
            TOKEN,
            staging,
            &mut rig.authority,
            &mut intents,
            &mut rig.adapters,
            &*rig.clock,
        );
        b.seal(rig.plan.clone()).unwrap();
        let r = done(b.handle(request));
        assert_eq!(r.outcome, Outcome::AlreadyInPlace);
        assert!(r.replayed && r.recorded, "{replayed_from_record}");
    }
    assert_eq!(rig.adapters.installs, 1, "never installed twice");
    // The outcome is on disk now.
    let j = open();
    assert!(matches!(
        j.broker(&role_keys().broker)
            .lookup(first_key(&rig, &first).as_bytes())
            .unwrap(),
        pctwin_journal::BrokerState::Finished(_, _)
    ));
}

/// The broker's one key for a request, as the broker makes it.
fn first_key(rig: &Rig, request: &BrokerRequest) -> blake3::Hash {
    let _ = rig;
    let mut h = blake3::Hasher::new_derive_key("PCTwin broker idempotency 2026-10-26 v1");
    h.update(&request.plan_digest.0);
    h.update(&(request.action_id.as_str().len() as u64).to_be_bytes());
    h.update(request.action_id.as_str().as_bytes());
    h.update(&request.action.identity().0);
    h.finalize()
}

/// Join (b): an action that finishes is recorded under its own number, outcome and all, and the
/// receipt names that number.
#[test]
fn the_real_journal_records_a_finished_action_under_its_number() {
    let journal_dir = tempfile::tempdir().unwrap();
    let path = journal_dir.path().join("helper.redb");
    let j = pctwin_journal::Journal::open_with_record_key(
        &path,
        pctwin_journal::keystore::TagKey::from_bytes([4; 32]),
    )
    .unwrap();
    let mut rig = Rig::new();
    let first = rig.request("restore-point");
    let other = rig.request("setting.hibernate");
    let launch = rig.launch();
    let mut intents = journal::JournalIntents::new(j.broker(&role_keys().broker));
    let mut b = Broker::new(
        launch,
        TOKEN,
        rig.staging.take().unwrap(),
        &mut rig.authority,
        &mut intents,
        &mut rig.adapters,
        &*rig.clock,
    );
    b.seal(rig.plan.clone()).unwrap();
    let r = done(b.handle(&first));
    let s = done(b.handle(&other));
    drop(b);
    assert!(r.recorded && s.recorded);
    assert_eq!((r.journal, s.journal), (1, 2));
    assert!(matches!(
        j.broker(&role_keys().broker)
            .lookup(first_key(&rig, &first).as_bytes())
            .unwrap(),
        pctwin_journal::BrokerState::Finished(1, _)
    ));
}

/// T22-023 through the broker: once the session watcher sees the laptop sleep (or another
/// person's session come to the screen), the broker's grants are over: nothing more is done.
#[test]
fn t22_023_a_sleep_seen_by_the_watcher_ends_the_grants() {
    let mut rig = Rig::new();
    let first = rig.request("restore-point");
    let then = rig.request("setting.hibernate");
    let mut b = rig.broker();
    assert!(matches!(b.handle(&first), Answer::Done(_)));
    let base = (std::time::Instant::now(), std::time::SystemTime::now());
    let looks = [
        watch::Look {
            mono: base.0,
            wall: base.1,
            on_screen: Some(true),
        },
        watch::Look {
            mono: base.0 + Duration::from_secs(2),
            wall: base.1 + Duration::from_secs(7_200),
            on_screen: Some(true),
        },
    ];
    let mut n = 0;
    let why = watch::watch(
        &b.revoker(),
        watch::TICK,
        || {
            n += 1;
            looks[n - 1]
        },
        |_| {},
    );
    assert_eq!(why, Some(watch::Ended::Slept));
    assert_eq!(denied(b.handle(&then)), Denial::Revoked);
    drop(b);
    assert_eq!(rig.adapters.settings, 0);
}

/// Second review, finding B2: an install with an administrator's rights goes on only when the
/// helper's own maker check of its held copy says so; it is asked about exactly the bytes the
/// installer would be given, after the approval, and its "no" changes nothing.
#[test]
fn an_install_needs_the_helpers_own_maker_check() {
    let mut rig = Rig::new();
    rig.adapters.maker = super::maker::MakerCheck::Refused;
    let req = rig.request("install.app");
    let mut b = rig.broker();
    assert_eq!(denied(b.handle(&req)), Denial::NotTheMaker);
    drop(b);
    assert_eq!(rig.adapters.installs, 0);
    assert_eq!(rig.adapters.maker_checked, [PACKAGE.to_vec()]);
    assert!(
        rig.journal.entries.is_empty(),
        "nothing recorded as intended"
    );
    // Asked after the approval, never instead of it.
    assert_eq!(rig.authority.asked, 1);

    let mut rig = Rig::new();
    let req = rig.request("install.app");
    let mut b = rig.broker();
    done(b.handle(&req));
    drop(b);
    assert_eq!(rig.adapters.maker_checked, [PACKAGE.to_vec()]);
    assert_eq!(rig.adapters.installs, 1);
    // Settings and restore points are not packages: no maker check.
    let mut rig = Rig::new();
    rig.adapters.maker = super::maker::MakerCheck::Refused;
    let req = rig.request("restore-point");
    let mut b = rig.broker();
    done(b.handle(&req));
    drop(b);
    assert!(rig.adapters.maker_checked.is_empty());
}

/// Second review, findings B2 and B3, for real: the helper's own maker check of a real signed
/// Windows Installer package on this laptop, staged in the helper's own folder and held. It
/// approves the package only for its own maker and exact bytes.
#[cfg(windows)]
#[test]
fn the_helpers_own_maker_check_of_a_real_package() {
    use super::maker::{MakerCheck, check_package, laptop_arch};
    // A package built for this laptop's own chip: the check approves only those (an x64 package
    // on an Arm laptop, as on GitHub's Windows Arm runner, is refused for its architecture).
    let chip = laptop_arch().expect("this laptop's chip");
    let Some(msi) = std::fs::read_dir(r"C:\Windows\Installer")
        .ok()
        .and_then(|list| {
            list.flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("msi")))
                .filter(|p| std::fs::metadata(p).is_ok_and(|m| m.len() < 64 * 1024 * 1024))
                .filter(|p| {
                    pctwin_maker::windows_old_copy(p)
                        .arch
                        .is_some_and(|a| a.runs_on(chip))
                })
                .find(|p| pctwin_maker::windows_maker_of(p).is_ok())
        })
    else {
        eprintln!("no signed Windows Installer package for this chip here");
        return;
    };
    let ready = matches!(
        pctwin_maker::windows::judge_status(pctwin_maker::windows::defender_status()),
        pctwin_maker::windows::DefenderState::Ready { signature_age_days, .. }
            if signature_age_days <= pctwin_maker::windows::MAX_SIGNATURE_AGE_DAYS
    );
    let bytes = std::fs::read(&msi).unwrap();
    let digest = sha(&bytes);
    let dir = tempfile::tempdir().unwrap();
    let mut staging = Staging::new(crate::broker::staging::HelperFolder::for_tests(
        dir.path().to_path_buf(),
    ));
    let handle = staging.admit(&mut bytes.as_slice(), digest).unwrap();
    let held = staging.open_verified(handle, digest).unwrap();
    let maker = pctwin_maker::windows_maker_of(&msi).unwrap();
    let action =
        |maker: pctwin_maker::facts::WindowsSigner, sha256: [u8; 32]| InstallVerifiedPackage {
            product_code: None,
            for_move: None,
            artifact: handle,
            sha256: Hex32(sha256),
            kind: PackageKind::Msi,
            scope: InstallScope::Machine,
            maker: Box::new(maker),
        };
    let limits = pctwin_maker::Limits::default();
    let got = check_package(&held, &action(maker.clone(), digest), &limits);
    eprintln!("{}: {got:?} (Defender ready: {ready})", msi.display());
    if ready {
        assert_eq!(got, MakerCheck::Approved);
    } else {
        assert_eq!(got, MakerCheck::Refused);
    }
    // Another maker: refused.
    let mut other = maker.clone();
    other.certificate.0[0] ^= 1;
    other.public_key = None;
    other.issuer_organization = Some("Another CA".into());
    other.subject.organization = Some("Another Ltd".into());
    assert_eq!(
        check_package(&held, &action(other, digest), &limits),
        MakerCheck::Refused
    );
    // Other approved bytes than the ones held: refused.
    let mut wrong = digest;
    wrong[0] ^= 1;
    assert_eq!(
        check_package(&held, &action(maker, wrong), &limits),
        MakerCheck::Refused
    );
}

/// Phase 2.1: a package reaches the helper's own folder only for an install the sealed plan
/// names by exactly that handle and those bytes, all of them, once; anything else keeps nothing.
#[test]
fn a_package_is_kept_only_for_the_sealed_plan_and_only_whole() {
    let dir = tempfile::tempdir().unwrap();
    let digest = sha(PACKAGE);
    let plan = ApprovedPlan {
        generation: 3,
        operations: vec![PlannedOperation {
            action_id: id("install.app"),
            action: install_action(5, InstallScope::Machine),
        }],
    };
    let launch = LaunchSeal {
        generation: plan.generation,
        digest: plan.digest(),
    };
    let mut authority = FakeAuthority {
        answer: Consent::Granted,
        asked: 0,
        during: None,
    };
    let mut journal = FakeJournal::default();
    let mut adapters = FakeAdapters::default();
    let clock = Now::real;
    let mut b = Broker::new(
        launch,
        TOKEN,
        Staging::new(crate::broker::staging::HelperFolder::for_tests(
            dir.path().to_path_buf(),
        )),
        &mut authority,
        &mut journal,
        &mut adapters,
        &clock,
    );
    let size = PACKAGE.len() as u64;
    // Not before the plan is sealed.
    assert_eq!(
        b.stage_begin(5, Hex32(digest), size),
        Err(Denial::NotSealed)
    );
    b.seal(plan).unwrap();
    // Not a handle or bytes the plan names.
    assert_eq!(
        b.stage_begin(6, Hex32(digest), size),
        Err(Denial::NotInPlan)
    );
    assert_eq!(
        b.stage_begin(5, Hex32([0; 32]), size),
        Err(Denial::NotInPlan)
    );
    // Bytes without a start are refused.
    assert_eq!(b.stage_bytes(PACKAGE), Err(Denial::NotInPlan));
    // Other bytes of the right length: nothing kept, nothing left in the folder.
    b.stage_begin(5, Hex32(digest), size).unwrap();
    assert_eq!(b.stage_begin(5, Hex32(digest), size), Err(Denial::Replayed));
    let other = vec![0x5a; PACKAGE.len()];
    assert_eq!(b.stage_bytes(&other), Err(Denial::Unsafe));
    assert!(!b.staging().holds(5));
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    // More than announced: refused, nothing kept.
    b.stage_begin(5, Hex32(digest), size - 1).unwrap();
    assert_eq!(b.stage_bytes(PACKAGE), Err(Denial::Unsafe));
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    // In two pieces, exactly: kept as 5, its name beginning with half its SHA-256.
    b.stage_begin(5, Hex32(digest), size).unwrap();
    let (a, rest) = PACKAGE.split_at(PACKAGE.len() / 2);
    assert_eq!(b.stage_bytes(a), Ok(None));
    assert_eq!(b.stage_bytes(rest), Ok(Some(5)));
    assert!(b.staging().holds(5));
    let names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names.len(), 1);
    assert!(names[0].starts_with(&staging::staged_prefix(&digest)));
    // Once only.
    assert_eq!(b.stage_begin(5, Hex32(digest), size), Err(Denial::Replayed));
    // An abandoned package leaves nothing; a closed broker takes nothing.
    b.close();
    assert_eq!(b.stage_begin(5, Hex32(digest), size), Err(Denial::Closed));
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

/// An installer program's kind is part of the approved action: the helper checks it is a
/// Windows program, never an installer package, and the plan cannot name switches.
#[test]
fn an_installer_program_is_checked_as_a_program() {
    let mut a = match install_action(1, InstallScope::Machine) {
        Action::InstallVerifiedPackage(p) => p,
        _ => unreachable!(),
    };
    a.kind = PackageKind::Exe(ExeKind::Inno);
    let e = maker::expected_of(&a, pctwin_maker::facts::Arch::X64);
    assert_eq!(e.formats, vec![pctwin_maker::facts::Format::WindowsProgram]);
    let json = serde_json::to_string(&Action::InstallVerifiedPackage(a.clone())).unwrap();
    assert!(json.contains("\"inno\""));
    // A field for switches does not exist.
    let with_switches = json.replace("\"scope\"", "\"switches\":\"/S\",\"scope\"");
    assert!(serde_json::from_str::<Action>(&with_switches).is_err());
    // Nor a kind PCTwin does not know (Nullsoft has no restart switch).
    let nsis = json.replace("\"inno\"", "\"nullsoft\"");
    assert!(serde_json::from_str::<Action>(&nsis).is_err());
}

/// The Mac App Store action takes a store number and nothing else, asks for an administrator only
/// on a Mac, and is the installs role's.
#[test]
fn the_mac_app_store_action_is_a_number_only() {
    let a = Action::InstallFromMacAppStore(InstallFromMacAppStore {
        adam_id: 497_799_835,
    });
    assert_eq!(a.audience(), pctwin_grant::Audience::Install);
    assert_eq!(a.needs_admin(), cfg!(target_os = "macos"));
    let json = serde_json::to_string(&a).unwrap();
    assert!(
        serde_json::from_str::<Action>(&json.replace("497799835", "\"497799835; rm\"")).is_err()
    );
    assert!(
        serde_json::from_str::<Action>(&json.replace("}}", ",\"path\":\"/tmp/mas\"}}")).is_err()
    );
}

/// The staging folder's own rules, below the broker: a short package is never kept, a handle is
/// kept once, and a package part way through when the session closes leaves nothing.
#[test]
fn staging_keeps_only_whole_packages_once() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Staging::new(crate::broker::staging::HelperFolder::for_tests(
        dir.path().to_path_buf(),
    ));
    let digest = sha(PACKAGE);
    let mut a = s.begin(4, digest, PACKAGE.len() as u64 + 1).unwrap();
    a.write(PACKAGE).unwrap();
    assert!(s.finish(a).is_err(), "one byte short");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    let mut a = s.begin(4, digest, PACKAGE.len() as u64).unwrap();
    a.write(PACKAGE).unwrap();
    assert_eq!(s.finish(a), Ok(4));
    assert!(
        s.begin(4, digest, PACKAGE.len() as u64).is_err(),
        "kept once"
    );
    assert!(s.begin(0, digest, 1).is_err());
}

#[test]
fn a_package_part_way_leaves_nothing_when_the_session_closes() {
    let dir = tempfile::tempdir().unwrap();
    let digest = sha(PACKAGE);
    let plan = ApprovedPlan {
        generation: 3,
        operations: vec![PlannedOperation {
            action_id: id("install.app"),
            action: install_action(5, InstallScope::Machine),
        }],
    };
    let launch = LaunchSeal {
        generation: plan.generation,
        digest: plan.digest(),
    };
    let mut authority = FakeAuthority {
        answer: Consent::Granted,
        asked: 0,
        during: None,
    };
    let mut journal = FakeJournal::default();
    let mut adapters = FakeAdapters::default();
    let clock = Now::real;
    let mut b = Broker::new(
        launch,
        TOKEN,
        Staging::new(crate::broker::staging::HelperFolder::for_tests(
            dir.path().to_path_buf(),
        )),
        &mut authority,
        &mut journal,
        &mut adapters,
        &clock,
    );
    b.seal(plan).unwrap();
    b.stage_begin(5, Hex32(digest), PACKAGE.len() as u64)
        .unwrap();
    assert_eq!(b.stage_bytes(&PACKAGE[..10]), Ok(None));
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    b.close();
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

/* ---------- Removing an app PCTwin installed (undo, decided 28 October 2026) ---------- */

const CODE: &str = "{A1B2C3D4-E5F6-4789-9ABC-DEF012345678}";
const OTHER: &str = "{11111111-2222-3333-4444-555555555555}";
const INSTALL_KEY: Hex32 = Hex32([0x5a; 32]);

fn code(s: &str) -> ProductCode {
    ProductCode::parse(s).unwrap()
}

const MOVE: Hex16 = Hex16([0x3c; 16]);

fn product(c: &str) -> InstalledProduct {
    InstalledProduct {
        code: code(c),
        version: ProductVersion::parse("2.0").unwrap(),
        installed_on: InstallDate::parse("20261028").unwrap(),
    }
}

fn removal(c: &str) -> Action {
    Action::UninstallWindowsInstallerProduct(UninstallWindowsInstallerProduct {
        product_code: code(c),
        installed_by: INSTALL_KEY,
        for_move: MOVE,
    })
}

/// A rig whose plan removes `CODE` (and `OTHER`), and whose journal holds, under the install's
/// key, `recorded` (None: nothing).
fn removal_rig(recorded: Option<Option<Outcome>>) -> Rig {
    let mut rig = Rig::with(vec![
        PlannedOperation {
            action_id: id("uninstall.0"),
            action: removal(CODE),
        },
        PlannedOperation {
            action_id: id("uninstall.1"),
            action: removal(OTHER),
        },
    ]);
    if let Some(outcome) = recorded {
        rig.journal.entries.push((
            Intent {
                idempotency: INSTALL_KEY,
                plan_generation: 3,
                plan_digest: Hex32([1; 32]),
                action_id: id("install.0"),
                action: install_action(1, InstallScope::Machine),
            },
            outcome,
        ));
    }
    rig
}

fn installed_outcome(c: &str) -> Outcome {
    Outcome::Installed {
        restart_needed: false,
        product: product(c),
        for_move: MOVE,
    }
}

/// Step 2's acceptance: a product the helper's own journal recorded installing is removed,
/// with the version it recorded, journaled and receipted.
#[test]
fn a_product_this_helper_recorded_installing_is_removed() {
    let mut rig = removal_rig(Some(Some(installed_outcome(CODE))));
    let req = rig.request("uninstall.0");
    let again = rig.request("uninstall.0");
    let mut b = rig.broker();
    let r = done(b.handle(&req));
    let second = done(b.handle(&again));
    drop(b);
    assert_eq!(
        r.outcome,
        Outcome::Uninstall(uninstall::UninstallResult::Removed {
            restart_needed: false
        })
    );
    assert!(r.recorded && !r.replayed);
    assert!(second.replayed);
    assert_eq!(rig.adapters.uninstalled, [product(CODE)]);
    assert_eq!(rig.authority.asked, 1);
    // The install's record, this removal's intent, the install's used mark, and the outcome.
    assert_eq!(rig.journal.entries.len(), 3);
    assert_eq!(rig.journal.entries[1].0.action, removal(CODE));
    assert_eq!(rig.journal.entries[1].1, Some(r.outcome.clone()));
    assert_eq!(
        rig.journal.entries[2].0.idempotency,
        uninstall::consumed_key(&INSTALL_KEY)
    );
    assert_eq!(rig.journal.entries[2].1, Some(r.outcome));
}

/// Step 2's acceptance: the broker refuses a removal for any product code the journal did not
/// record this helper installing, before asking anyone and before anything starts.
#[test]
fn a_removal_the_journal_did_not_record_is_refused_before_anything_starts() {
    let cases: Vec<(&str, Option<Option<Outcome>>, &str)> = vec![
        ("nothing recorded", None, "uninstall.0"),
        ("install unfinished", Some(None), "uninstall.0"),
        (
            "an install that recorded no product (an installer program)",
            Some(Some(Outcome::Done {
                restart_needed: false,
            })),
            "uninstall.0",
        ),
        (
            "an install that failed",
            Some(Some(Outcome::Failed { code: 1603 })),
            "uninstall.0",
        ),
        (
            "an install found already in place",
            Some(Some(Outcome::AlreadyInPlace)),
            "uninstall.0",
        ),
        (
            "an install that recorded another product",
            Some(Some(installed_outcome(CODE))),
            "uninstall.1",
        ),
        (
            "an install recorded for another move",
            Some(Some(Outcome::Installed {
                restart_needed: false,
                product: product(CODE),
                for_move: Hex16([0x99; 16]),
            })),
            "uninstall.0",
        ),
        (
            "a record that is a removal, not an install",
            Some(Some(Outcome::Uninstall(
                uninstall::UninstallResult::AlreadyGone,
            ))),
            "uninstall.0",
        ),
    ];
    for (why, recorded, op) in cases {
        let mut rig = removal_rig(recorded);
        let before = rig.journal.entries.len();
        let req = rig.request(op);
        let mut b = rig.broker();
        assert_eq!(
            denied(b.handle(&req)),
            Denial::NotInstalledByPctwin,
            "{why}"
        );
        drop(b);
        assert!(rig.adapters.uninstalled.is_empty(), "{why}");
        assert_eq!(rig.authority.asked, 0, "{why}");
        assert_eq!(rig.journal.entries.len(), before, "{why}");
    }
    // A journal that cannot be read: nothing is done.
    let mut rig = removal_rig(Some(Some(installed_outcome(CODE))));
    rig.journal.down = true;
    let req = rig.request("uninstall.0");
    let mut b = rig.broker();
    assert_eq!(denied(b.handle(&req)), Denial::JournalUnavailable);
    drop(b);
    assert!(rig.adapters.uninstalled.is_empty());
}

/// A removal not in the approved plan, or with its code swapped, is refused like any action.
#[test]
fn a_removal_outside_the_plan_is_refused() {
    let mut rig = removal_rig(Some(Some(installed_outcome(CODE))));
    let mut req = rig.request("uninstall.0");
    req.action = Action::UninstallWindowsInstallerProduct(UninstallWindowsInstallerProduct {
        product_code: code(OTHER),
        installed_by: INSTALL_KEY,
        for_move: MOVE,
    });
    let mut b = rig.broker();
    assert_eq!(denied(b.handle(&req)), Denial::NotInPlan);
    drop(b);
    assert!(rig.adapters.uninstalled.is_empty());
}

/// The journal, as the rig's, but unreadable once `down` is set.
struct Flippy<'a> {
    inner: &'a mut FakeJournal,
    down: Rc<Cell<bool>>,
}

impl IntentJournal for Flippy<'_> {
    fn lookup(&mut self, key: &Hex32) -> Result<Recorded, JournalUnavailable> {
        if self.down.get() {
            return Err(JournalUnavailable);
        }
        self.inner.lookup(key)
    }
    fn record_intent(&mut self, i: &Intent) -> Result<JournalRef, JournalUnavailable> {
        self.inner.record_intent(i)
    }
    fn record_outcome(&mut self, at: JournalRef, o: &Outcome) -> Result<(), JournalUnavailable> {
        self.inner.record_outcome(at, o)
    }
}

/// The record is checked again after the approval: one no longer readable then stops it.
#[test]
fn the_record_is_checked_again_after_the_approval() {
    let mut rig = removal_rig(Some(Some(installed_outcome(CODE))));
    let req = rig.request("uninstall.0");
    let down = Rc::new(Cell::new(false));
    let flip = Rc::clone(&down);
    rig.authority.during = Some(Box::new(move || flip.set(true)));
    let launch = rig.launch();
    let plan = rig.plan.clone();
    let staging = rig.staging.take().unwrap();
    let mut journal = Flippy {
        inner: &mut rig.journal,
        down,
    };
    let mut b = Broker::new(
        launch,
        TOKEN,
        staging,
        &mut rig.authority,
        &mut journal,
        &mut rig.adapters,
        &*rig.clock,
    );
    b.seal(plan).unwrap();
    assert_eq!(denied(b.handle(&req)), Denial::JournalUnavailable);
    drop(b);
    assert_eq!(rig.authority.asked, 1);
    assert!(rig.adapters.uninstalled.is_empty());
}

/// An install that recorded its product hands it back in its receipt, for the app's undo book.
#[test]
fn an_install_that_recorded_its_product_says_so_in_its_receipt() {
    let mut rig = Rig::new();
    rig.adapters.result = Effect::Installed {
        restart_needed: true,
        product: product(CODE),
        for_move: MOVE,
    };
    let req = rig.request("install.app");
    let mut b = rig.broker();
    let r = done(b.handle(&req));
    drop(b);
    assert_eq!(
        r.outcome,
        Outcome::Installed {
            restart_needed: true,
            product: product(CODE),
            for_move: MOVE,
        }
    );
    assert_eq!(rig.journal.entries[0].1, Some(r.outcome.clone()));
    // The receipt's key is the one a removal names.
    assert_eq!(rig.journal.entries[0].0.idempotency, r.idempotency);
    let json = serde_json::to_string(&r.outcome).unwrap();
    assert_eq!(serde_json::from_str::<Outcome>(&json).unwrap(), r.outcome);
}

/// A rig whose plan removes `CODE` twice, as two attempts (two undo runs), after its install.
fn two_attempts() -> Rig {
    let mut rig = Rig::with(vec![
        PlannedOperation {
            action_id: id("uninstall.0.first"),
            action: removal(CODE),
        },
        PlannedOperation {
            action_id: id("uninstall.0.second"),
            action: removal(CODE),
        },
    ]);
    rig.journal.entries.push((
        Intent {
            idempotency: INSTALL_KEY,
            plan_generation: 3,
            plan_digest: Hex32([1; 32]),
            action_id: id("install.0"),
            action: install_action(1, InstallScope::Machine),
        },
        Some(installed_outcome(CODE)),
    ));
    rig
}

/// F1: a removal that ended uses the install up: a second undo (the app installed again by the
/// person since, at the same version) is refused, whatever the first one found.
#[test]
fn an_install_is_used_once_by_a_removal_that_ended() {
    for ended in [
        uninstall::UninstallResult::Removed {
            restart_needed: false,
        },
        uninstall::UninstallResult::AlreadyGone,
        uninstall::UninstallResult::KeptOtherVersion,
    ] {
        let mut rig = two_attempts();
        rig.adapters.uninstall_result = Effect::Uninstall(ended.clone());
        let first = rig.request("uninstall.0.first");
        let second = rig.request("uninstall.0.second");
        let mut b = rig.broker();
        assert_eq!(
            done(b.handle(&first)).outcome,
            Outcome::Uninstall(ended.clone())
        );
        assert_eq!(
            denied(b.handle(&second)),
            Denial::AlreadyUndone,
            "{ended:?}"
        );
        drop(b);
        assert_eq!(rig.adapters.uninstalled.len(), 1, "{ended:?}");
    }
}

/// F1: a removal that failed does not use the install up: the next undo tries again.
#[test]
fn a_failed_removal_leaves_the_install_for_the_next_try() {
    let mut rig = two_attempts();
    rig.adapters.uninstall_result = Effect::Failed {
        code: 1618,
        native: NativeText::new(""),
    };
    let first = rig.request("uninstall.0.first");
    let second = rig.request("uninstall.0.second");
    let mut b = rig.broker();
    assert_eq!(
        done(b.handle(&first)).outcome,
        Outcome::Failed { code: 1618 }
    );
    assert_eq!(
        done(b.handle(&second)).outcome,
        Outcome::Failed { code: 1618 }
    );
    drop(b);
    assert_eq!(rig.adapters.uninstalled.len(), 2);
    assert!(
        rig.journal
            .entries
            .iter()
            .all(|(i, _)| i.idempotency != uninstall::consumed_key(&INSTALL_KEY))
    );
}

/// F1 and F6: a removal found in place after a crash also uses the install up.
#[test]
fn a_removal_found_in_place_after_a_crash_uses_the_install_up() {
    let mut rig = two_attempts();
    let first = rig.request("uninstall.0.first");
    let second = rig.request("uninstall.0.second");
    // The first attempt's intent was written, then the helper stopped.
    let key = {
        let b = rig.broker();
        b.idempotency(&first)
    };
    rig.journal.entries.push((
        Intent {
            idempotency: key,
            plan_generation: first.plan_generation,
            plan_digest: first.plan_digest,
            action_id: first.action_id.clone(),
            action: first.action.clone(),
        },
        None,
    ));
    rig.adapters.observed = Observed::Present;
    rig.staging = Some(Staging::new(
        crate::broker::staging::HelperFolder::for_tests(rig._dir.path().to_path_buf()),
    ));
    let mut b = rig.broker();
    let r = done(b.handle(&first));
    assert_eq!(r.outcome, Outcome::AlreadyInPlace);
    assert!(r.recorded);
    assert_eq!(denied(b.handle(&second)), Denial::AlreadyUndone);
    drop(b);
    assert!(rig.adapters.uninstalled.is_empty());
}

/// When the used mark cannot be written, the receipt says it was not recorded.
#[test]
fn a_used_mark_that_cannot_be_written_is_said() {
    let mut rig = two_attempts();
    rig.journal.intents_down_after = Some(2);
    let first = rig.request("uninstall.0.first");
    let mut b = rig.broker();
    let r = done(b.handle(&first));
    drop(b);
    assert!(!r.recorded);
}
