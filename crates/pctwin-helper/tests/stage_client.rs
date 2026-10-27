//! Phase 2.1: a package crosses the helper's channel into its own folder (its own test program:
//! the role keys are made once per program).

use std::collections::BTreeSet;
use std::net::{TcpListener, TcpStream};

use pctwin_helper::broker::actions::{
    Action, ApprovedPlan, Hex16, Hex32, Id, InstallScope, InstallVerifiedPackage, NamedSetting,
    PackageKind, PlannedOperation,
};
use pctwin_helper::broker::staging::{Staging, VerifiedArtifact};
use pctwin_helper::broker::{
    Adapters, Broker, BrokerRequest, Consent, Denial, Effect, LaunchSeal, NativeAuthority,
    Observed, PROTOCOL, Revoker,
};
use pctwin_helper::{Acted, Client, Refusal, Session};
use sha2::Digest;

struct Approves;
impl NativeAuthority for Approves {
    fn obtain(&mut self, _: &Action) -> Consent {
        Consent::Granted
    }
}

/// Records the bytes the maker check was given; installs nothing.
struct Records(std::sync::Arc<std::sync::Mutex<Vec<Vec<u8>>>>);
impl Adapters for Records {
    fn check_maker(
        &mut self,
        p: &VerifiedArtifact,
        _: &InstallVerifiedPackage,
    ) -> pctwin_helper::broker::maker::MakerCheck {
        self.0
            .lock()
            .unwrap()
            .push(std::fs::read(p.path()).unwrap());
        pctwin_helper::broker::maker::MakerCheck::Approved
    }
    fn install(&mut self, _: &VerifiedArtifact, _: &InstallVerifiedPackage, _: &Revoker) -> Effect {
        Effect::Done {
            restart_needed: false,
        }
    }
    fn apply_setting(&mut self, _: &NamedSetting, _: &Revoker) -> Effect {
        unreachable!()
    }
    fn restore_point(&mut self, _: &Revoker) -> Effect {
        unreachable!()
    }
    fn observe(&mut self, _: &Action) -> Observed {
        Observed::Absent
    }
}

/// A package larger than one piece crosses the channel into the helper's own folder, only for
/// the sealed plan, and the install then uses exactly those bytes, as the helper's maker check
/// (here a fake that records what it was given) sees them.
#[test]
fn a_package_crosses_the_channel_whole_and_is_installed_from_the_helpers_copy() {
    let package: Vec<u8> = (0..100_000u32).map(|i| (i % 251) as u8).collect();
    let digest: [u8; 32] = sha2::Sha256::digest(&package).into();
    let install = Action::InstallVerifiedPackage(InstallVerifiedPackage {
        artifact: 1,
        sha256: Hex32(digest),
        kind: PackageKind::Msi,
        scope: InstallScope::Machine,
        maker: Box::new(pctwin_maker::facts::WindowsSigner {
            subject: Default::default(),
            certificate: pctwin_maker::facts::Sha256([1; 32]),
            issuer: String::new(),
            issuer_organization: None,
            public_key: None,
        }),
    });
    let plan = ApprovedPlan {
        generation: 9,
        operations: vec![PlannedOperation {
            action_id: Id::parse("install.app").unwrap(),
            action: install.clone(),
        }],
    };
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let helper_plan = plan.clone();
    let helper_seen = std::sync::Arc::clone(&seen);
    let helper = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let packages = tempfile::tempdir().unwrap();
        let journal_dir = tempfile::tempdir().unwrap();
        let keys = pctwin_grant::RoleKeys::mint().unwrap();
        let j = pctwin_journal::Journal::open_with_record_key(
            &journal_dir.path().join("helper.redb"),
            pctwin_journal::keystore::TagKey::from_bytes([8; 32]),
        )
        .unwrap();
        let mut journal =
            pctwin_helper::broker::journal::JournalIntents::new(j.broker(&keys.broker));
        let (mut authority, mut adapters) = (Approves, Records(helper_seen));
        let clock = pctwin_grant::Now::real;
        let mut scanner = |_: &str| -> Result<pctwin_helper::person::PersonScan, Refusal> {
            Err(Refusal::NotApproved)
        };
        let broker = Broker::new(
            LaunchSeal {
                generation: helper_plan.generation,
                digest: helper_plan.digest(),
            },
            "tok",
            Staging::new(packages.path().to_path_buf()),
            &mut authority,
            &mut journal,
            &mut adapters,
            &clock,
        );
        Session::new("tok".into(), BTreeSet::new(), &mut scanner)
            .with_broker(broker)
            .serve(&mut stream)
            .unwrap();
    });
    let mut client = Client::open(TcpStream::connect(addr).unwrap(), "tok").unwrap();
    let size = package.len() as u64;
    // Before the plan: nothing is taken.
    assert_eq!(
        client
            .stage(1, Hex32(digest), size, &mut package.as_slice())
            .unwrap(),
        Err(Denial::NotSealed)
    );
    assert_eq!(client.seal_plan(&plan).unwrap(), Ok(()));
    // Bytes other than the plan's: refused, nothing kept.
    let mut other = package.clone();
    other[50_000] ^= 1;
    assert_eq!(
        client
            .stage(1, Hex32(digest), size, &mut other.as_slice())
            .unwrap(),
        Err(Denial::Unsafe)
    );
    // The plan's bytes, in four pieces.
    assert_eq!(
        client
            .stage(1, Hex32(digest), size, &mut package.as_slice())
            .unwrap(),
        Ok(())
    );
    let request = BrokerRequest {
        protocol: PROTOCOL,
        audience: pctwin_grant::Audience::Install,
        request_id: Id::parse("r1").unwrap(),
        plan_generation: plan.generation,
        plan_digest: plan.digest(),
        action_id: Id::parse("install.app").unwrap(),
        action: install,
        nonce: Hex16([1; 16]),
    };
    assert!(matches!(client.act(&request).unwrap(), Acted::Done(_)));
    assert_eq!(seen.lock().unwrap().as_slice(), &[package]);
    client.bye().unwrap();
    helper.join().unwrap();
}
