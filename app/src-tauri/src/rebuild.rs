//! The move's restore point, asked of the administrator helper (the join of 2.2 and 2.3, (c),
//! 27 October 2026). The executor asks its adapter for one before a move's first install or
//! setting (`pctwin_executor::Adapter::restore_point`); on this laptop the adapter's answer comes
//! from here.
//!
//! Decided: a move that installs apps or changes settings asks once for an administrator, with
//! the system's own prompt (the helper is started for the approved plan). If the person says no,
//! the honest answer "not made: you chose not to" is recorded and the move goes on: the journal
//! is the first safety net, the restore point a second. Whatever the helper answers is turned
//! into the journal's words as it is; nothing is claimed that Windows did not confirm.

use std::io::{Read, Write};

use pctwin_helper::broker::actions::{Action, ApprovedPlan, CreateRestorePoint, Hex16, Id};
use pctwin_helper::broker::receipt::Outcome;
use pctwin_helper::broker::restore::{NotMadeReason, RestorePointResult, wmi_time_secs};
use pctwin_helper::broker::{BrokerRequest, Denial, PROTOCOL};
use pctwin_helper::{Acted, Client, ClientError};
use pctwin_journal::RestorePoint;

/// The person said no at the system's prompt.
pub const DECLINED: &str = "not made: you chose not to";
/// The helper could not be started or reached.
pub const NO_HELPER: &str = "not made: PCTwin could not reach its administrator helper";
/// The helper did not do it (the move was stopped, the plan did not hold it).
pub const NOT_DONE: &str = "not made: PCTwin's administrator helper did not make it";

/// How starting the helper for the move went.
pub enum Started<C> {
    /// The person said no at the system's prompt.
    Declined,
    /// It could not be started or reached.
    Failed,
    /// It is running for this move, the plan not yet given.
    Ready(C),
}

/// The helper's broker, as the app speaks to it.
pub trait PlanChannel {
    fn seal_plan(&mut self, plan: &ApprovedPlan) -> Result<Result<(), Denial>, ClientError>;
    fn act(&mut self, request: &BrokerRequest) -> Result<Acted, ClientError>;
}

impl<S: Read + Write> PlanChannel for Client<S> {
    fn seal_plan(&mut self, plan: &ApprovedPlan) -> Result<Result<(), Denial>, ClientError> {
        Client::seal_plan(self, plan)
    }
    fn act(&mut self, request: &BrokerRequest) -> Result<Acted, ClientError> {
        Client::act(self, request)
    }
}

/// The plan's restore point operation: its ID in the plan.
pub fn restore_point_id(plan: &ApprovedPlan) -> Option<&Id> {
    plan.operations
        .iter()
        .find(|o| matches!(o.action, Action::CreateRestorePoint(_)))
        .map(|o| &o.action_id)
}

/// Asks the started helper for the move's restore point, once, and gives the honest answer in
/// the journal's words. The helper (when it started) stays in `started` for the move's other
/// privileged steps. `nonce`: the request's one-use number.
pub fn restore_point_via<C: PlanChannel>(
    started: &mut Started<C>,
    plan: &ApprovedPlan,
    nonce: Hex16,
) -> RestorePoint {
    let not_made = |why: &str| RestorePoint::NotMade { why: why.into() };
    let channel = match started {
        Started::Declined => return not_made(DECLINED),
        Started::Failed => return not_made(NO_HELPER),
        Started::Ready(c) => c,
    };
    let Some(action_id) = restore_point_id(plan) else {
        return not_made(NOT_DONE);
    };
    match channel.seal_plan(plan) {
        Ok(Ok(())) | Ok(Err(Denial::Replayed)) => {}
        Ok(Err(_)) => return not_made(NOT_DONE),
        Err(_) => return not_made(NO_HELPER),
    }
    let request = BrokerRequest {
        protocol: PROTOCOL,
        audience: pctwin_grant::Audience::Recovery,
        request_id: action_id.clone(),
        plan_generation: plan.generation,
        plan_digest: plan.digest(),
        action_id: action_id.clone(),
        action: Action::CreateRestorePoint(CreateRestorePoint {}),
        nonce,
    };
    match channel.act(&request) {
        Ok(Acted::Done(receipt)) => match &receipt.outcome {
            Outcome::RestorePoint(r) => journal_point(r),
            _ => not_made(NOT_DONE),
        },
        Ok(Acted::Denied(_)) => not_made(NOT_DONE),
        Err(_) => not_made(NO_HELPER),
    }
}

/// The helper's restore point answer in the journal's words. A time Windows gave that cannot be
/// read is kept as 0 (the answer itself is not changed).
pub fn journal_point(r: &RestorePointResult) -> RestorePoint {
    let secs = |at: &str| wmi_time_secs(at).unwrap_or(0);
    match r {
        RestorePointResult::Made { at, number } => RestorePoint::Made {
            at: secs(at),
            number: *number,
        },
        RestorePointResult::EarlierExists { at, number } => RestorePoint::EarlierExists {
            at: secs(at),
            number: *number,
        },
        RestorePointResult::NotMade { reason } => match reason {
            NotMadeReason::NotWindows => RestorePoint::NotWindows,
            NotMadeReason::ProtectionOff => RestorePoint::NotMade {
                why: "not made: System Protection is turned off on this laptop".into(),
            },
            NotMadeReason::NeedsAdmin => RestorePoint::NotMade {
                why: "not made: it needs an administrator".into(),
            },
            NotMadeReason::Unconfirmed => RestorePoint::NotMade {
                why: "not made: Windows said yes, but no new restore point can be found".into(),
            },
            NotMadeReason::Failed => RestorePoint::NotMade {
                why: "not made: Windows could not make one".into(),
            },
        },
    }
}

/// The move's restore point on this laptop: on Windows, starts the helper for the approved plan
/// (the system's prompt, once), asks, and returns the answer and the helper (kept for the move's
/// other privileged steps); elsewhere restore points do not exist.
#[cfg(windows)]
pub fn restore_point_for_move(
    mover: &str,
    laptop: pctwin_record::LaptopId,
    plan: &ApprovedPlan,
    stopped: &dyn Fn() -> bool,
) -> (
    RestorePoint,
    Started<Client<pctwin_helper::client::Channel>>,
) {
    let seal = format!("{}:{}", plan.generation, plan.digest());
    let mut started = match crate::others::launch_elevated(mover, &[], laptop, Some(&seal), stopped)
    {
        Ok(Some(client)) => Started::Ready(client),
        Ok(None) => Started::Declined,
        Err(_) => Started::Failed,
    };
    let mut bytes = [0u8; 16];
    if getrandom::fill(&mut bytes).is_err() {
        return (
            RestorePoint::NotMade {
                why: NO_HELPER.into(),
            },
            started,
        );
    }
    let point = restore_point_via(&mut started, plan, Hex16(bytes));
    (point, started)
}

/// Elsewhere restore points do not exist; nothing is started.
#[cfg(not(windows))]
pub fn restore_point_for_move(
    _mover: &str,
    _laptop: pctwin_record::LaptopId,
    _plan: &ApprovedPlan,
    _stopped: &dyn Fn() -> bool,
) -> (
    RestorePoint,
    Started<Client<pctwin_helper::client::Channel>>,
) {
    (RestorePoint::NotWindows, Started::Failed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pctwin_helper::broker::actions::PlannedOperation;
    use pctwin_helper::broker::receipt::Receipt;

    fn plan() -> ApprovedPlan {
        ApprovedPlan {
            generation: 2,
            operations: vec![PlannedOperation {
                action_id: Id::parse("restore-point").unwrap(),
                action: Action::CreateRestorePoint(CreateRestorePoint {}),
            }],
        }
    }

    /// A helper that answers as told.
    struct Fake {
        sealed: Result<Result<(), Denial>, ()>,
        answer: Result<Acted, ()>,
        asked: Vec<BrokerRequest>,
    }

    impl PlanChannel for Fake {
        fn seal_plan(&mut self, _: &ApprovedPlan) -> Result<Result<(), Denial>, ClientError> {
            self.sealed.map_err(|()| ClientError::OutOfTurn)
        }
        fn act(&mut self, r: &BrokerRequest) -> Result<Acted, ClientError> {
            self.asked.push(r.clone());
            self.answer.clone().map_err(|()| ClientError::OutOfTurn)
        }
    }

    fn receipt(outcome: Outcome) -> Acted {
        Acted::Done(Box::new(Receipt {
            request_id: Id::parse("restore-point").unwrap(),
            action_id: Id::parse("restore-point").unwrap(),
            plan_generation: 2,
            plan_digest: plan().digest(),
            idempotency: Default::default(),
            outcome,
            journal: 1,
            recorded: true,
            replayed: false,
            mac: Default::default(),
        }))
    }

    fn ready(answer: Result<Acted, ()>) -> Started<Fake> {
        Started::Ready(Fake {
            sealed: Ok(Ok(())),
            answer,
            asked: Vec::new(),
        })
    }

    /// The person says no at the system's prompt: recorded as not made, because they chose not
    /// to; nothing is asked of any helper.
    #[test]
    fn a_declined_prompt_is_recorded_as_the_persons_choice() {
        let mut s: Started<Fake> = Started::Declined;
        assert_eq!(
            restore_point_via(&mut s, &plan(), Hex16([1; 16])),
            RestorePoint::NotMade {
                why: DECLINED.into()
            }
        );
        let mut f: Started<Fake> = Started::Failed;
        assert_eq!(
            restore_point_via(&mut f, &plan(), Hex16([1; 16])),
            RestorePoint::NotMade {
                why: NO_HELPER.into()
            }
        );
    }

    /// The helper's honest answers become the journal's, as they were given.
    #[test]
    fn the_helpers_answer_is_kept_as_it_was_given() {
        let made = RestorePointResult::Made {
            at: "20261027090000.000000+000".into(),
            number: 12,
        };
        let mut s = ready(Ok(receipt(Outcome::RestorePoint(made))));
        let got = restore_point_via(&mut s, &plan(), Hex16([2; 16]));
        assert_eq!(
            got,
            RestorePoint::Made {
                at: wmi_time_secs("20261027090000.000000+000").unwrap(),
                number: 12
            }
        );
        let Started::Ready(f) = &s else { panic!() };
        assert_eq!(f.asked.len(), 1);
        assert_eq!(f.asked[0].audience, pctwin_grant::Audience::Recovery);
        assert_eq!(f.asked[0].plan_digest, plan().digest());
        assert_eq!(f.asked[0].nonce, Hex16([2; 16]));
        let earlier = RestorePointResult::EarlierExists {
            at: "20261027080000.000000+000".into(),
            number: 11,
        };
        assert!(matches!(
            journal_point(&earlier),
            RestorePoint::EarlierExists { number: 11, .. }
        ));
        for reason in [
            NotMadeReason::ProtectionOff,
            NotMadeReason::NeedsAdmin,
            NotMadeReason::Unconfirmed,
            NotMadeReason::Failed,
        ] {
            assert!(matches!(
                journal_point(&RestorePointResult::NotMade { reason }),
                RestorePoint::NotMade { why } if why.starts_with("not made: ")
            ));
        }
        assert_eq!(
            journal_point(&RestorePointResult::NotMade {
                reason: NotMadeReason::NotWindows
            }),
            RestorePoint::NotWindows
        );
    }

    /// Anything but a restore point answer is never taken for one.
    #[test]
    fn a_refusal_or_a_broken_channel_is_not_made() {
        for (answer, why) in [
            (Ok(Acted::Denied(Denial::Revoked)), NOT_DONE),
            (
                Ok(receipt(Outcome::Done {
                    restart_needed: false,
                })),
                NOT_DONE,
            ),
            (Err(()), NO_HELPER),
        ] {
            let mut s = ready(answer);
            assert_eq!(
                restore_point_via(&mut s, &plan(), Hex16([3; 16])),
                RestorePoint::NotMade { why: why.into() }
            );
        }
        let mut refused = Started::Ready(Fake {
            sealed: Ok(Err(Denial::WrongPlan)),
            answer: Err(()),
            asked: Vec::new(),
        });
        assert_eq!(
            restore_point_via(&mut refused, &plan(), Hex16([3; 16])),
            RestorePoint::NotMade {
                why: NOT_DONE.into()
            }
        );
        let mut no_point = ready(Err(()));
        let empty = ApprovedPlan {
            generation: 2,
            operations: vec![],
        };
        assert_eq!(
            restore_point_via(&mut no_point, &empty, Hex16([3; 16])),
            RestorePoint::NotMade {
                why: NOT_DONE.into()
            }
        );
    }
}
