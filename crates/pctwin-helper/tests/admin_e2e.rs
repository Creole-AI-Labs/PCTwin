//! End to end, as an administrator on Windows (the join of 2.2 and 2.3; run by the CI job
//! "Admin end to end", never by an ordinary test run): a tiny Windows Installer package built here
//! with Windows' own Windows Installer (fixtures/msi/make-test-msi.ps1, no third-party tool) is
//! installed through the broker, with its intent in the helper's journal on disk first; the
//! helper stops before writing the outcome; the next helper, on the same journal, finds the
//! intent, checks Windows' own list of installed products, and does not install again; then a
//! restore point is asked for and its honest answer recorded. The package is removed at the end.
//!
//! Run: `cargo test -p pctwin-helper --test admin_e2e -- --ignored --exact
//! an_elevated_install_stopped_before_its_outcome_is_never_done_twice` (in an elevated shell).

#![cfg(windows)]

use std::path::Path;
use std::process::Command;

use sha2::{Digest, Sha256};

use pctwin_helper::broker::actions::{
    Action, ApprovedPlan, CreateRestorePoint, Hex16, Hex32, Id, InstallScope,
    InstallVerifiedPackage, NamedSetting, PackageKind, PlannedOperation,
};
use pctwin_helper::broker::receipt::Outcome;
use pctwin_helper::broker::staging::{Staging, VerifiedArtifact};
use pctwin_helper::broker::{
    Adapters, Answer, Broker, BrokerRequest, BuiltInAdapters, Consent, Effect, Intent,
    IntentJournal, JournalRef, JournalUnavailable, LaunchSeal, NativeAuthority, NativeText,
    Observed, PROTOCOL, Recorded, Revoker,
};

/// The test package's product code (fixed in the script that builds it).
const PRODUCT: &str = "{6F1E0A55-3C2B-4D8E-9C1A-50C7EA5E7E01}";

struct Approves;
impl NativeAuthority for Approves {
    fn obtain(&mut self, _: &Action) -> Consent {
        Consent::Granted
    }
}

/// Windows' own list of installed products, read with Windows' `reg.exe`.
fn installed() -> Observed {
    let key = format!(r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{PRODUCT}");
    match Command::new(system32("reg.exe"))
        .args(["query", &key])
        .output()
    {
        Ok(o) if o.status.success() => Observed::Present,
        Ok(_) => Observed::Absent,
        Err(_) => Observed::Unknown,
    }
}

fn system32(program: &str) -> std::path::PathBuf {
    Path::new(&std::env::var_os("SystemRoot").expect("SystemRoot"))
        .join("System32")
        .join(program)
}

/// The installs the broker would get from 2.1, for the test package only: Windows Installer
/// started by the broker's own fixed command, and Windows' list looked at for what is there.
struct TestMsi {
    installs: usize,
}

impl Adapters for TestMsi {
    /// The test package is PCTwin's own unsigned one; the helper's real maker check is proven on
    /// real signed packages in the broker's own tests.
    fn check_maker(
        &mut self,
        _: &VerifiedArtifact,
        _: &InstallVerifiedPackage,
    ) -> pctwin_helper::broker::maker::MakerCheck {
        pctwin_helper::broker::maker::MakerCheck::Approved
    }
    fn install(&mut self, p: &VerifiedArtifact, a: &InstallVerifiedPackage, _: &Revoker) -> Effect {
        self.installs += 1;
        let mut command = pctwin_helper::broker::tool::msiexec_install(p, a.scope).unwrap();
        // Windows Installer's own log, shown when the install fails, so a failure says why.
        let log = std::env::temp_dir().join(format!("pctwin-test-msi-{}.log", self.installs));
        command.arg("/l*v").arg(&log);
        let status = command.status();
        if !matches!(status.as_ref().map(|s| s.code()), Ok(Some(0 | 3010))) {
            if let Ok(bytes) = std::fs::read(&log) {
                let text: String = char::decode_utf16(
                    bytes
                        .chunks_exact(2)
                        .map(|c| u16::from_le_bytes([c[0], c[1]])),
                )
                .map(|c| c.unwrap_or('?'))
                .collect();
                let lines: Vec<&str> = text.lines().collect();
                eprintln!(
                    "Windows Installer log ({}):
{}",
                    p.path().display(),
                    lines[lines.len().saturating_sub(80)..].join(
                        "
"
                    )
                );
            }
        }
        match status.map(|s| s.code()) {
            Ok(Some(0)) => Effect::Done {
                restart_needed: false,
            },
            Ok(Some(3010)) => Effect::Done {
                restart_needed: true,
            },
            Ok(code) => Effect::Failed {
                code: i64::from(code.unwrap_or(-1)),
                native: NativeText::new(""),
            },
            Err(_) => Effect::Failed {
                code: -1,
                native: NativeText::new(""),
            },
        }
    }
    fn apply_setting(&mut self, _: &NamedSetting, _: &Revoker) -> Effect {
        unreachable!("no setting in this plan")
    }
    fn restore_point(&mut self, stop: &Revoker) -> Effect {
        BuiltInAdapters.restore_point(stop)
    }
    fn observe(&mut self, action: &Action) -> Observed {
        match action {
            Action::InstallVerifiedPackage(_) => installed(),
            other => BuiltInAdapters.observe(other),
        }
    }
}

/// The helper stopping after the change and before its outcome is written.
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

fn request(plan: &ApprovedPlan, id: &str, nonce: u8) -> BrokerRequest {
    let op = plan.operation(&Id::parse(id).unwrap()).unwrap();
    BrokerRequest {
        protocol: PROTOCOL,
        audience: op.action.audience(),
        request_id: Id::parse(&format!("r{nonce}")).unwrap(),
        plan_generation: plan.generation,
        plan_digest: plan.digest(),
        action_id: op.action_id.clone(),
        action: op.action.clone(),
        nonce: Hex16([nonce; 16]),
    }
}

#[test]
#[ignore = "needs an administrator on Windows: run by the CI job \"Admin end to end\""]
fn an_elevated_install_stopped_before_its_outcome_is_never_done_twice() {
    assert!(
        pctwin_helper_os::this_process_elevated().unwrap(),
        "run this test as an administrator"
    );
    assert_eq!(
        installed(),
        Observed::Absent,
        "the test package is not installed yet"
    );
    let work = tempfile::tempdir().unwrap();
    // The package, built with Windows' own Windows Installer.
    let msi = work.path().join("pctwin-test.msi");
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/msi/make-test-msi.ps1");
    let built = Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(&script)
        .arg("-Out")
        .arg(&msi)
        .output()
        .unwrap();
    assert!(built.status.success(), "{built:?}");
    let bytes = std::fs::read(&msi).unwrap();
    let sha: [u8; 32] = Sha256::digest(&bytes).into();
    let packages = work.path().join("packages");
    std::fs::create_dir(&packages).unwrap();
    let journal_path = work.path().join("helper.redb");
    let key = || pctwin_journal::keystore::TagKey::from_bytes([0x2a; 32]);
    let keys = pctwin_grant::RoleKeys::mint().unwrap();

    let stage = || {
        let mut s = Staging::new(pctwin_helper::broker::staging::HelperFolder::for_tests(
            packages.clone(),
        ));
        let handle = s.admit(&mut bytes.as_slice(), sha).unwrap();
        (s, handle)
    };
    let (staging, handle) = stage();
    let plan = ApprovedPlan {
        generation: 1,
        operations: vec![
            PlannedOperation {
                action_id: Id::parse("install.test").unwrap(),
                action: Action::InstallVerifiedPackage(InstallVerifiedPackage {
                    artifact: handle,
                    sha256: Hex32(sha),
                    kind: PackageKind::Msi,
                    scope: InstallScope::Machine,
                    maker: Box::new(pctwin_maker::facts::WindowsSigner {
                        subject: Default::default(),
                        certificate: pctwin_maker::facts::Sha256([0; 32]),
                        issuer: String::new(),
                        issuer_organization: None,
                        public_key: None,
                    }),
                }),
            },
            PlannedOperation {
                action_id: Id::parse("restore-point").unwrap(),
                action: Action::CreateRestorePoint(CreateRestorePoint {}),
            },
        ],
    };
    let launch = LaunchSeal {
        generation: plan.generation,
        digest: plan.digest(),
    };
    let clock = pctwin_grant::Now::real;
    let mut adapters = TestMsi { installs: 0 };

    // The first helper: intent on disk, the install, then it stops before the outcome.
    {
        let j = pctwin_journal::Journal::open_with_record_key(&journal_path, key()).unwrap();
        let mut intents =
            pctwin_helper::broker::journal::JournalIntents::new(j.broker(&keys.broker));
        let mut stopping = StopsBeforeOutcome(&mut intents);
        let mut authority = Approves;
        let mut b = Broker::new(
            launch,
            "tok",
            staging,
            &mut authority,
            &mut stopping,
            &mut adapters,
            &clock,
        );
        b.seal(plan.clone()).unwrap();
        match b.handle(&request(&plan, "install.test", 1)) {
            Answer::Done(r) => {
                assert_eq!(
                    r.outcome,
                    Outcome::Done {
                        restart_needed: false
                    }
                );
                assert!(!r.recorded, "the helper stopped before the outcome");
            }
            Answer::Denied(d) => panic!("denied: {d:?}"),
        }
    }
    assert_eq!(adapters.installs, 1);
    assert_eq!(installed(), Observed::Present);

    // The next helper, on the same journal: it finds the intent with no outcome, looks at
    // Windows' list, and does not install again.
    let (staging, again) = stage();
    assert_eq!(again, handle);
    let j = pctwin_journal::Journal::open_with_record_key(&journal_path, key()).unwrap();
    let mut intents = pctwin_helper::broker::journal::JournalIntents::new(j.broker(&keys.broker));
    let mut authority = Approves;
    let mut b = Broker::new(
        launch,
        "tok",
        staging,
        &mut authority,
        &mut intents,
        &mut adapters,
        &clock,
    );
    b.seal(plan.clone()).unwrap();
    match b.handle(&request(&plan, "install.test", 2)) {
        Answer::Done(r) => {
            assert_eq!(r.outcome, Outcome::AlreadyInPlace);
            assert!(r.replayed && r.recorded);
        }
        Answer::Denied(d) => panic!("denied: {d:?}"),
    }
    // A restore point, asked once: whatever Windows answers is recorded as it was given.
    match b.handle(&request(&plan, "restore-point", 3)) {
        Answer::Done(r) => {
            eprintln!("restore point: {:?}", r.outcome);
            assert!(matches!(r.outcome, Outcome::RestorePoint(_)));
            assert!(r.recorded);
        }
        Answer::Denied(d) => panic!("denied: {d:?}"),
    }
    drop(b);
    assert_eq!(adapters.installs, 1, "never installed twice");

    // Leave the runner as it was.
    let removed = Command::new(system32("msiexec.exe"))
        .args(["/x", PRODUCT, "/qn", "/norestart"])
        .status()
        .unwrap();
    assert!(removed.success(), "{removed:?}");
    assert_eq!(installed(), Observed::Absent);
}
