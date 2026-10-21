//! One move from start to end, once the two laptops are connected and paired, for every program
//! that runs a move: the test lab (`pctwin-labmove`) and, later, the app. Pairing itself stays
//! with each of them; everything after it is here, once.
//!
//! - [`send_plan`] turns the old laptop's record into the files to send, in the order the app
//!   moves things, with every file not sent and why ([`NotSentWhy`]). Quick Send uses it too.
//! - [`worth_compressing`] is the one list of formats that are already compressed.
//! - [`send`] is the old laptop's side: it agrees the version, sends the record, sends the files,
//!   and gives one typed outcome per file.
//! - [`receive_plan`] and [`ApprovedPlan::receive`] (or [`receive`], both in one) are the new
//!   laptop's side: it agrees the version, receives the record, asks the person through [`Steps`]
//!   where each person goes and whether they approve the plan, then receives only what that plan
//!   allows, through the safety gate, with every write in the change journal.
//!
//! Only the new laptop approves a plan (decided 16 October 2026); the old laptop never does. A
//! step the person refuses ([`Stop`]) ends the move before anything is written.
//!
//! Nothing here writes a file itself: every write is the receiver's, through the gate.

// The type-resolved fence: clippy.toml bans every way to change a file or folder, as in
// pctwin-transfer. Tests make files to test on, so they are exempt.
#![cfg_attr(not(test), forbid(clippy::disallowed_methods))]

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::path::PathBuf;

use pctwin_gate::Destinations;
use pctwin_journal::Ledger;
use pctwin_record::{
    Inclusion, Item, ItemId, ItemKind, LeftOutReason, Owner, PersonTarget, Record, RecordError,
};
use pctwin_transfer::{
    Allowance, Channel, LandingPlace, MOVE_PROTOCOL, ReceiveOutcome, ReceiverSession, SendJob,
    SendOutcome, SenderSession, Tier, TransferError, agree_version, plan_order, receive_inventory,
    send_inventory,
};

/// Files a move sends at once.
const IN_FLIGHT: usize = 4;

/// Formats that are already compressed, so compressing them again only costs time.
const COMPRESSED: [&str; 12] = [
    "jpg", "jpeg", "png", "heic", "mp3", "mp4", "mov", "zip", "docx", "xlsx", "gz", "7z",
];

/// Whether a file at `path` (`/` between folders) is worth trying to compress: false for formats
/// that are already compressed, judged by the last extension of its own name, in any case. A name
/// with no dot has no extension, so it is tried.
pub fn worth_compressing(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    match name.rsplit_once('.') {
        Some((_, ext)) => !COMPRESSED.iter().any(|c| c.eq_ignore_ascii_case(ext)),
        None => true,
    }
}

/// Why a file in the record is not sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotSentWhy {
    /// The record leaves it out, for this reason.
    LeftOut { reason: LeftOutReason },
    /// The old laptop has no folder for its role, so there is nowhere to read it from.
    NoFolderForRole,
}

/// A file in the record that is not sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotSent {
    pub item: ItemId,
    pub why: NotSentWhy,
}

/// What the old laptop sends: the jobs in the order planned, and the files it does not send.
#[derive(Debug, Clone)]
pub struct SendPlan {
    pub jobs: Vec<SendJob>,
    pub not_sent: Vec<NotSent>,
}

/// The files to send from `record`. `source` says where each file is read from (`None`: the old
/// laptop has no folder for its role) and `landing` where it lands, which must be the very
/// function the new laptop checks it with. Only files are sent; a file the record leaves out is
/// listed as not sent with the record's reason. Jobs come in the order the app moves things, each
/// with its tier.
pub fn send_plan(
    record: &Record,
    source: impl Fn(&Item) -> Option<PathBuf>,
    landing: impl Fn(&Item) -> LandingPlace,
) -> SendPlan {
    let mut not_sent = Vec::new();
    let mut by_item = BTreeMap::new();
    for item in &record.items {
        if item.kind != ItemKind::File {
            continue;
        }
        if let Inclusion::LeftOut { reason } = item.inclusion {
            not_sent.push(NotSent {
                item: item.id,
                why: NotSentWhy::LeftOut { reason },
            });
            continue;
        }
        let Some(source) = source(item) else {
            not_sent.push(NotSent {
                item: item.id,
                why: NotSentWhy::NoFolderForRole,
            });
            continue;
        };
        let place = landing(item);
        by_item.insert(
            item.id,
            SendJob {
                item: item.id,
                source,
                compressible: worth_compressing(&place.path),
                destination: place.destination,
                path: place.path,
                tier: Tier::Rest,
            },
        );
    }
    // The order the app moves things in; anything it does not place goes last.
    let mut jobs = Vec::new();
    for (id, tier) in plan_order(&record.items, &[], &BTreeMap::new(), 0) {
        if let Some(mut job) = by_item.remove(&id) {
            job.tier = tier;
            jobs.push(job);
        }
    }
    jobs.extend(by_item.into_values());
    SendPlan { jobs, not_sent }
}

/// Why a move did not go ahead or did not finish.
#[derive(Debug, thiserror::Error)]
pub enum MoveError {
    #[error(transparent)]
    Transfer(#[from] TransferError),
    #[error(transparent)]
    Record(#[from] RecordError),
    #[error(transparent)]
    Stopped(#[from] Stop),
}

/// How one sent file ended on the old laptop (`None`: it has no outcome).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SentFile {
    pub item: ItemId,
    pub outcome: Option<SendOutcome>,
}

/// What the old laptop's side of a move came to: one line per job, in the order sent, and the
/// files it did not send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sent {
    pub files: Vec<SentFile>,
    pub not_sent: Vec<NotSent>,
}

/// The old laptop, over `link` (already paired): agrees the version, sends `record`, then sends
/// the files `plan` holds.
pub async fn send<C: Channel>(
    link: &mut C,
    record: &Record,
    plan: SendPlan,
) -> Result<Sent, MoveError> {
    let version = agree_version(link, MOVE_PROTOCOL).await?;
    send_inventory(link, record).await?;
    let items: Vec<ItemId> = plan.jobs.iter().map(|j| j.item).collect();
    let mut sender = SenderSession::new(plan.jobs, IN_FLIGHT).with_protocol(version);
    sender.run(link).await?;
    Ok(Sent {
        files: items
            .into_iter()
            .map(|item| SentFile {
                item,
                outcome: sender.outcome(item).cloned(),
            })
            .collect(),
        not_sent: plan.not_sent,
    })
}

/// Why the person stopped a move on the new laptop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Stop {
    #[error("the plan was not approved")]
    NotApproved,
    #[error("the move was cancelled")]
    Cancelled,
}

/// The choices a person makes on the new laptop during a move. The test lab answers them by
/// itself; the app answers them from its screens. Each answer may be [`Stop`], which ends the
/// move before anything is written.
///
/// Written as `async fn` in an implementation. There is no `Send` bound: a move runs on one task.
pub trait Steps {
    /// Where the things of the person `account_id` on the old laptop go on this one. Asked once
    /// for each person who owns something in `record`.
    fn person(
        &mut self,
        record: &Record,
        account_id: &str,
    ) -> impl Future<Output = Result<PersonTarget, Stop>>;

    /// The person approves `record`, the plan as it will run, with every person placed. Only the
    /// new laptop ever approves.
    fn approve(&mut self, record: &Record) -> impl Future<Output = Result<(), Stop>>;

    /// Where `item` lands on this laptop (`None`: it is not received). It must be the very
    /// function the old laptop sends with, so the old laptop never chooses a place.
    fn landing(&self, item: &Item) -> Option<LandingPlace>;
}

/// A plan the person approved on the new laptop, ready to receive.
pub struct ApprovedPlan {
    record: Record,
    allowance: Allowance,
    /// The move protocol version both laptops agreed.
    version: u16,
}

/// How one received file ended on the new laptop (`None`: it has no outcome).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceivedFile {
    pub item: ItemId,
    pub outcome: Option<ReceiveOutcome>,
}

/// What the new laptop's side of a move came to: the plan as approved, and one line per included
/// file, in the record's order.
#[derive(Debug, Clone)]
pub struct Received {
    pub record: Record,
    pub files: Vec<ReceivedFile>,
}

/// The new laptop, over `link` (already paired): agrees the version, receives the old laptop's
/// record, asks `steps` where each person goes and for the person's approval, and holds the
/// approved plan. Nothing is written.
pub async fn receive_plan<C: Channel, S: Steps>(
    link: &mut C,
    steps: &mut S,
) -> Result<ApprovedPlan, MoveError> {
    let version = agree_version(link, MOVE_PROTOCOL).await?;
    let mut record = receive_inventory(link).await?;
    let mut people = BTreeSet::new();
    let owners: Vec<String> = record
        .items
        .iter()
        .filter_map(|i| match &i.owner {
            Owner::Person { account_id } if people.insert(account_id.as_str()) => {
                Some(account_id.clone())
            }
            _ => None,
        })
        .collect();
    for account_id in owners {
        let target = steps.person(&record, &account_id).await?;
        record.mapping.people.insert(account_id, target);
    }
    steps.approve(&record).await?;
    let approval = record.approve()?;
    let allowance = Allowance::from_record(&record, &approval, |i| steps.landing(i))?;
    Ok(ApprovedPlan {
        record,
        allowance,
        version,
    })
}

impl ApprovedPlan {
    /// The plan as the person approved it.
    pub fn record(&self) -> &Record {
        &self.record
    }

    /// Receives over `link` into the places in `table`, only what the plan allows, recording every
    /// write in `journal`; `account` is the signed-in person's account on this laptop.
    pub async fn receive<C: Channel>(
        self,
        link: &mut C,
        table: &Destinations,
        journal: &dyn Ledger,
        account: &str,
    ) -> Result<Received, MoveError> {
        let mut receiver = ReceiverSession::new(table, self.allowance, journal, account)
            .with_protocol(self.version);
        receiver.run(link).await?;
        let files = self
            .record
            .items
            .iter()
            .filter(|i| i.kind == ItemKind::File && i.inclusion == Inclusion::Included)
            .map(|i| ReceivedFile {
                item: i.id,
                outcome: receiver.outcome(i.id),
            })
            .collect();
        Ok(Received {
            record: self.record,
            files,
        })
    }
}

/// The new laptop's whole side: [`receive_plan`], then [`ApprovedPlan::receive`].
pub async fn receive<C: Channel, S: Steps>(
    link: &mut C,
    steps: &mut S,
    table: &Destinations,
    journal: &dyn Ledger,
    account: &str,
) -> Result<Received, MoveError> {
    receive_plan(link, steps)
        .await?
        .receive(link, table, journal, account)
        .await
}
