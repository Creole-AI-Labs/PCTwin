//! The app's side of the broker's conversation (join of 2.2 and 2.3, (c)): the plan sealed, a
//! restore point asked for, its honest answer and its record in the helper's journal, then the
//! move stopped.

use std::collections::BTreeSet;
use std::net::{TcpListener, TcpStream};

use pctwin_helper::broker::actions::{
    Action, ApprovedPlan, CreateRestorePoint, Hex16, Id, NamedSetting, PlannedOperation,
};
use pctwin_helper::broker::restore::{NotMadeReason, RestorePointResult};
use pctwin_helper::broker::staging::{Staging, VerifiedArtifact};
use pctwin_helper::broker::{
    Adapters, Broker, BrokerRequest, Consent, Denial, Effect, LaunchSeal, NativeAuthority,
    Observed, PROTOCOL, Revoker,
};
use pctwin_helper::{Acted, Client, Refusal, Session};

struct Approves;
impl NativeAuthority for Approves {
    fn obtain(&mut self, _: &Action) -> Consent {
        Consent::Granted
    }
}

/// A laptop whose System Protection is off.
struct ProtectionOff;
impl Adapters for ProtectionOff {
    fn install(
        &mut self,
        _: &VerifiedArtifact,
        _: &pctwin_helper::broker::actions::InstallVerifiedPackage,
        _: &Revoker,
    ) -> Effect {
        unreachable!("no install in this plan")
    }
    fn apply_setting(&mut self, _: &NamedSetting, _: &Revoker) -> Effect {
        unreachable!("no setting in this plan")
    }
    fn restore_point(&mut self, _: &Revoker) -> Effect {
        Effect::RestorePoint(RestorePointResult::NotMade {
            reason: NotMadeReason::ProtectionOff,
        })
    }
    fn observe(&mut self, _: &Action) -> Observed {
        Observed::Absent
    }
}

fn plan() -> ApprovedPlan {
    ApprovedPlan {
        generation: 3,
        operations: vec![PlannedOperation {
            action_id: Id::parse("restore-point").unwrap(),
            action: Action::CreateRestorePoint(CreateRestorePoint {}),
        }],
    }
}

fn request(nonce: u8) -> BrokerRequest {
    let plan = plan();
    BrokerRequest {
        protocol: PROTOCOL,
        audience: pctwin_grant::Audience::Recovery,
        request_id: Id::parse(&format!("r{nonce}")).unwrap(),
        plan_generation: plan.generation,
        plan_digest: plan.digest(),
        action_id: Id::parse("restore-point").unwrap(),
        action: Action::CreateRestorePoint(CreateRestorePoint {}),
        nonce: Hex16([nonce; 16]),
    }
}

#[test]
fn a_restore_point_through_the_client_is_answered_and_recorded() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let journal_dir = tempfile::tempdir().unwrap();
    let journal_path = journal_dir.path().join("helper.redb");
    let helper_path = journal_path.clone();
    let helper = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let packages = tempfile::tempdir().unwrap();
        let keys = pctwin_grant::RoleKeys::mint().unwrap();
        let j = pctwin_journal::Journal::open_with_record_key(
            &helper_path,
            pctwin_journal::keystore::TagKey::from_bytes([8; 32]),
        )
        .unwrap();
        let mut journal =
            pctwin_helper::broker::journal::JournalIntents::new(j.broker(&keys.broker));
        let (mut authority, mut adapters) = (Approves, ProtectionOff);
        let clock = pctwin_grant::Now::real;
        let mut scanner = |_: &str| -> Result<pctwin_helper::person::PersonScan, Refusal> {
            Err(Refusal::NotApproved)
        };
        let plan = plan();
        let broker = Broker::new(
            LaunchSeal {
                generation: plan.generation,
                digest: plan.digest(),
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
    // Not the plan the helper was started for: refused.
    let mut other = plan();
    other.generation = 4;
    assert_eq!(client.seal_plan(&other).unwrap(), Err(Denial::WrongPlan));
    assert_eq!(client.seal_plan(&plan()).unwrap(), Ok(()));
    let receipt = match client.act(&request(1)).unwrap() {
        Acted::Done(r) => r,
        Acted::Denied(d) => panic!("denied: {d:?}"),
    };
    assert_eq!(
        receipt.outcome,
        pctwin_helper::broker::receipt::Outcome::RestorePoint(RestorePointResult::NotMade {
            reason: NotMadeReason::ProtectionOff
        })
    );
    assert!(receipt.recorded, "its outcome is in the helper's journal");
    assert!(receipt.verify(&pctwin_helper::broker::receipt::receipt_key("tok")));
    // The same request again is a replay: refused.
    assert_eq!(
        client.act(&request(1)).unwrap(),
        Acted::Denied(Denial::Replayed)
    );
    // The move stopped: nothing more.
    client.revoke().unwrap();
    assert_eq!(
        client.act(&request(2)).unwrap(),
        Acted::Denied(Denial::Closed)
    );
    client.bye().unwrap();
    helper.join().unwrap();
}
