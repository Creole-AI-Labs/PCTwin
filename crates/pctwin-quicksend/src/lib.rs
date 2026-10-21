//! Quick Send (expansion plan EX01): chosen files and folders to a laptop nearby, at any time, over
//! the same sealed connection, blocks and safety gate as a move, with no apps or settings.
//!
//! - The sender makes an [`Offer`] of what the person chose and [`send`]s it: first only names and
//!   sizes (the offer is a small move record, sent as in a move), then only what the receiver
//!   accepted.
//! - The receiver gets the offer ([`receive_offer`]), and the person accepts everything, some
//!   files, or nothing, and chooses the folder they go to ([`Incoming::accept`],
//!   [`Incoming::decline`]). The sender cannot name a place: every path lands inside the chosen
//!   folder through the safety gate, which turns any name it cannot store into a safe one and
//!   never replaces a file already there. Nothing received is opened or run.
//! - Both sides get a [`Receipt`]: what arrived and under which name, what was declined, and what
//!   failed and why. As in a move, an interrupted send continues later with only what is missing.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use pctwin_gate::{Approved, Destinations};
use pctwin_journal::{DownloadMark, Ledger};
use pctwin_record::{
    FolderRole, Inclusion, Item, ItemId, ItemKind, ItemName, ItemPath, LaptopId, LeftOutReason,
    ManagedBy, Owner, Place, Portability, Record, RecordError, Storage,
};
use pctwin_transfer::{
    Allowance, Cause, Channel, LandingPlace, MOVE_PROTOCOL, ReceiveOutcome, ReceiverSession,
    SendJob, SendOutcome, SenderSession, SizeRoom, TransferError, agree_version, receive_inventory,
    send_inventory, shown_path,
};

/// The destination label everything of a Quick Send lands under: the folder the receiver chose.
const DESTINATION: &str = "quick-send";

/// Files sent at once.
const LANES: usize = 4;

/// The receiver's answer: which offered files to send.
const ACCEPTED: u8 = 36;
/// An item id in the answer: 32 hex characters.
const ID_LEN: usize = 32;

/// Why a Quick Send did not go ahead.
#[derive(Debug, thiserror::Error)]
pub enum QuickError {
    #[error("nothing was chosen to send")]
    NothingChosen,
    #[error("something chosen is not there any more: {0}")]
    NotThere(PathBuf),
    #[error("what the other laptop offered is not a Quick Send")]
    NotAQuickSend,
    #[error("the other laptop's answer was not in the expected form")]
    BadAnswer,
    #[error(transparent)]
    Transfer(#[from] TransferError),
    #[error(transparent)]
    Record(#[from] RecordError),
    #[error("the chosen folder cannot be used: {0}")]
    Destination(String),
}

/// One offered file, as both screens list it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfferedFile {
    pub id: ItemId,
    /// Its path inside the send, `/` between folders ("Holiday/beach.jpg").
    pub path: String,
    pub size: u64,
}

/// What a sender offers: the chosen files and folders, as a small move record, and where each
/// file is read from.
pub struct Offer {
    record: Record,
    jobs: Vec<SendJob>,
}

fn place() -> Place {
    Place {
        role: FolderRole::Other,
        storage: Storage::SystemDrive,
    }
}

/// Where an offered file lands: inside the folder the receiver chose, at its path in the send.
/// Both sides use this one function (the sender to say where it sends each file, the receiver to
/// check it), so the sender never chooses a place.
fn landing(item: &Item) -> LandingPlace {
    LandingPlace::new(DESTINATION, shown_path(&item.path))
}

/// `name`, or `stem (n).ext` for the first n from 2 not in `taken` (compared ignoring case).
fn unique_name(name: &str, taken: &mut BTreeSet<String>) -> String {
    let mut candidate = name.to_string();
    let (stem, ext) = match name.rfind('.') {
        Some(at) if at > 0 => (&name[..at], &name[at..]),
        _ => (name, ""),
    };
    let mut n = 2;
    while taken.contains(&candidate.to_lowercase()) {
        candidate = format!("{stem} ({n}){ext}");
        n += 1;
    }
    taken.insert(candidate.to_lowercase());
    candidate
}

impl Offer {
    /// An offer of `chosen` files and folders. Folders are offered with everything inside them
    /// that a move would take (links are not followed; what cannot be read is listed as left
    /// out). Two chosen things with the same name are kept apart ("notes (2).txt").
    pub fn of(chosen: &[PathBuf]) -> Result<Self, QuickError> {
        if chosen.is_empty() {
            return Err(QuickError::NothingChosen);
        }
        let laptop = LaptopId::generate()?;
        let owner = Owner::Shared;
        let place = place();
        let mut record = Record::new(laptop);
        // Where each offered file is read from.
        let mut sources = BTreeMap::new();
        let mut taken = BTreeSet::new();
        for path in chosen {
            let meta =
                std::fs::symlink_metadata(path).map_err(|_| QuickError::NotThere(path.clone()))?;
            let name = path
                .file_name()
                .ok_or_else(|| QuickError::NotThere(path.clone()))?
                .to_string_lossy()
                .into_owned();
            let top = ItemName::from_text(&unique_name(&name, &mut taken));
            let top_path = ItemPath::new(vec![top.clone()])?;
            let item = |kind, path: ItemPath, size_bytes, inclusion| Item {
                id: ItemId::derive(&laptop, &owner, &place, &path),
                kind,
                owner: owner.clone(),
                place: place.clone(),
                path,
                size_bytes,
                modified_ns: None,
                portability: Portability::Portable,
                managed_by: ManagedBy::Personal,
                download_bytes: None,
                inclusion,
            };
            if meta.is_file() {
                let file = item(ItemKind::File, top_path, meta.len(), Inclusion::Included);
                sources.insert(file.id, path.clone());
                record.items.push(file);
            } else if meta.is_dir() {
                record
                    .items
                    .push(item(ItemKind::Folder, top_path, 0, Inclusion::Included));
                let scan = pctwin_scan::scan_folder(&laptop, &owner, &place, path);
                for inner in scan.items {
                    let mut parts = vec![top.clone()];
                    parts.extend(inner.path.parts().iter().cloned());
                    let mut source = path.clone();
                    for part in inner.path.parts() {
                        source.push(part.on_disk());
                    }
                    let found = item(
                        inner.kind,
                        ItemPath::new(parts)?,
                        inner.size_bytes,
                        inner.inclusion,
                    );
                    if found.kind == ItemKind::File && found.inclusion == Inclusion::Included {
                        sources.insert(found.id, source);
                    }
                    record.items.push(found);
                }
            } else {
                return Err(QuickError::NotThere(path.clone()));
            }
        }
        record.validate()?;
        // Sent as a move sends: in the order a move plans, each landing inside the chosen folder.
        // What a folder holds that cannot be read is left out by the record and not sent.
        let jobs = pctwin_move::send_plan(&record, |i| sources.get(&i.id).cloned(), landing).jobs;
        Ok(Self { record, jobs })
    }

    /// The files offered, in path order.
    pub fn files(&self) -> Vec<OfferedFile> {
        files_of(&self.record)
    }

    /// For tests that play a sender breaking the rules: the record and the send jobs.
    #[doc(hidden)]
    pub fn into_parts(self) -> (Record, Vec<SendJob>) {
        (self.record, self.jobs)
    }
}

fn files_of(record: &Record) -> Vec<OfferedFile> {
    let mut files: Vec<OfferedFile> = record
        .items
        .iter()
        .filter(|i| i.kind == ItemKind::File && i.inclusion == Inclusion::Included)
        .map(|i| OfferedFile {
            id: i.id,
            path: shown_path(&i.path),
            size: i.size_bytes,
        })
        .collect();
    files.sort_by(|a, b| a.path.cmp(&b.path));
    files
}

/// How one offered file ended, on either side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// It is in the chosen folder, under `landed_as` (another name if one was taken; on the
    /// sender, the name it was sent as).
    Arrived { landed_as: String },
    /// An identical file was already there, so it was not sent again.
    AlreadyThere { landed_as: String },
    /// The receiver did not accept it.
    Declined,
    /// It is on the receiver under `landed_as` but not finished as a visible, proven copy: `why`
    /// says what happened and what to do next.
    NeedsAction { landed_as: String, why: String },
    /// It did not arrive: the typed reason (for "more details" and "Send missing pieces"), and
    /// the words.
    Failed { cause: Cause, why: String },
}

/// One line of a receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptLine {
    /// The file's path in the send.
    pub path: String,
    pub outcome: Outcome,
}

/// What a Quick Send came to, file by file, on one side.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Receipt {
    pub lines: Vec<ReceiptLine>,
}

/// Sender: offers `offer` over `ch` (already paired), then sends what the receiver accepts.
pub async fn send(ch: &mut impl Channel, offer: Offer) -> Result<Receipt, QuickError> {
    let version = agree_version(ch, MOVE_PROTOCOL).await?;
    send_inventory(ch, &offer.record, version).await?;
    let offered = files_of(&offer.record);
    let accepted = read_accepted(ch, &offered).await?;
    let jobs: Vec<SendJob> = offer
        .jobs
        .into_iter()
        .filter(|j| accepted.contains(&j.item))
        .collect();
    let mut sender = SenderSession::new(jobs, LANES).with_protocol(version);
    if !accepted.is_empty() {
        sender.run(ch).await?;
    }
    Ok(Receipt {
        lines: offered
            .into_iter()
            .map(|f| {
                let outcome = if !accepted.contains(&f.id) {
                    Outcome::Declined
                } else {
                    match sender.outcome(f.id) {
                        Some(SendOutcome::Arrived) => Outcome::Arrived {
                            landed_as: f.path.clone(),
                        },
                        Some(SendOutcome::AlreadyThere) => Outcome::AlreadyThere {
                            landed_as: f.path.clone(),
                        },
                        Some(SendOutcome::Failed(why)) => Outcome::Failed {
                            cause: why.cause(),
                            why: why.to_string(),
                        },
                        None => Outcome::Failed {
                            cause: Cause::NotAccepted,
                            why: "it was not sent".into(),
                        },
                    }
                };
                ReceiptLine {
                    path: f.path,
                    outcome,
                }
            })
            .collect(),
    })
}

/// What a sender offered, for the person to accept or decline.
pub struct Incoming {
    record: Record,
    /// The move protocol version both laptops agreed.
    version: u16,
}

/// Receiver: the offer from the other laptop over `ch` (already paired): names and sizes only.
/// Refused unless it is a Quick Send (everything in it shared, nothing about people).
pub async fn receive_offer(ch: &mut impl Channel) -> Result<Incoming, QuickError> {
    let version = agree_version(ch, MOVE_PROTOCOL).await?;
    let record = receive_inventory(ch, version).await?;
    if record.items.iter().any(|i| i.owner != Owner::Shared) {
        return Err(QuickError::NotAQuickSend);
    }
    Ok(Incoming { record, version })
}

impl Incoming {
    /// The files offered, in path order.
    pub fn files(&self) -> Vec<OfferedFile> {
        files_of(&self.record)
    }

    /// The size of everything offered, in bytes.
    pub fn total_bytes(&self) -> u64 {
        self.files().iter().map(|f| f.size).sum()
    }

    /// Accepts the files `chosen` (all of them with `None`) into the folder `dest` the person
    /// chose, receives them through the safety gate (recording each step in `journal`, for
    /// `account`), and returns the receipt.
    pub async fn accept(
        self,
        ch: &mut impl Channel,
        dest: &Path,
        chosen: Option<&[ItemId]>,
        journal: &dyn Ledger,
        account: &str,
    ) -> Result<Receipt, QuickError> {
        let offered = self.files();
        let wanted: BTreeSet<ItemId> = match chosen {
            Some(ids) => ids
                .iter()
                .copied()
                .filter(|id| offered.iter().any(|f| f.id == *id))
                .collect(),
            None => offered.iter().map(|f| f.id).collect(),
        };
        // The offered record is not needed afterwards: the next revision is made from it in place,
        // never as a second copy.
        let mut record = self.record.into_next_revision();
        for item in &mut record.items {
            if item.kind == ItemKind::File
                && item.inclusion == Inclusion::Included
                && !wanted.contains(&item.id)
            {
                item.inclusion = Inclusion::LeftOut {
                    reason: LeftOutReason::ChosenByUser,
                };
            }
        }
        let approval = record.approve()?;
        // Where each file lands comes from the offer, never from the sender's start; the files
        // were chosen a moment ago, so none may grow; and they come from someone else's laptop,
        // so each is marked as from the internet at least.
        let allowance = Allowance::from_record(&record, &approval, |i| Some(landing(i)))?
            .with_size_room(SizeRoom::NoGrowth)
            .with_mark_floor(DownloadMark::Internet);
        let mut table = Destinations::new();
        table
            .approve(DESTINATION, Approved::MyFolders, dest)
            .map_err(|e| QuickError::Destination(e.to_string()))?;
        write_accepted(ch, &wanted).await?;
        let mut receiver =
            ReceiverSession::new(&table, allowance, journal, account).with_protocol(self.version);
        if !wanted.is_empty() {
            receiver.run(ch).await?;
        }
        Ok(Receipt {
            lines: offered
                .into_iter()
                .map(|f| {
                    let outcome = if !wanted.contains(&f.id) {
                        Outcome::Declined
                    } else {
                        match receiver.outcome(f.id) {
                            Some(ReceiveOutcome::Finished(done)) => Outcome::Arrived {
                                landed_as: done.final_path.clone(),
                            },
                            // Not finished as a visible, proven copy: never "arrived".
                            Some(ReceiveOutcome::NeedsAction { finished, why }) => {
                                Outcome::NeedsAction {
                                    landed_as: finished.final_path.clone(),
                                    why: why.clone(),
                                }
                            }
                            Some(ReceiveOutcome::AlreadyThere(stored)) => Outcome::AlreadyThere {
                                landed_as: stored.clone(),
                            },
                            Some(ReceiveOutcome::Failed(why)) => Outcome::Failed {
                                cause: why.cause(),
                                why: why.to_string(),
                            },
                            None => Outcome::Failed {
                                cause: Cause::NotAccepted,
                                why: "it did not arrive".into(),
                            },
                        }
                    };
                    ReceiptLine {
                        path: f.path,
                        outcome,
                    }
                })
                .collect(),
        })
    }

    /// Declines the whole offer: nothing is sent, nothing is written.
    pub async fn decline(self, ch: &mut impl Channel) -> Result<(), QuickError> {
        write_accepted(ch, &BTreeSet::new()).await
    }
}

/// `[36][count, 4 bytes][each item id as 32 hex characters]`.
async fn write_accepted(ch: &mut impl Channel, ids: &BTreeSet<ItemId>) -> Result<(), QuickError> {
    let count = u32::try_from(ids.len()).map_err(|_| QuickError::BadAnswer)?;
    let mut msg = Vec::with_capacity(5 + ids.len() * ID_LEN);
    msg.push(ACCEPTED);
    msg.extend_from_slice(&count.to_be_bytes());
    for id in ids {
        msg.extend_from_slice(id.to_hex().as_bytes());
    }
    ch.send(&msg)
        .await
        .map_err(|_| TransferError::ConnectionDropped)?;
    Ok(())
}

/// The receiver's answer: only ids it was offered, each once, and nothing else in the message.
async fn read_accepted(
    ch: &mut impl Channel,
    offered: &[OfferedFile],
) -> Result<BTreeSet<ItemId>, QuickError> {
    let msg = ch
        .recv()
        .await
        .map_err(|_| TransferError::ConnectionDropped)?;
    let (head, rest) = msg.split_at_checked(5).ok_or(QuickError::BadAnswer)?;
    if head[0] != ACCEPTED {
        return Err(QuickError::BadAnswer);
    }
    let count = u32::from_be_bytes([head[1], head[2], head[3], head[4]]) as usize;
    if count > offered.len() || rest.len() != count * ID_LEN {
        return Err(QuickError::BadAnswer);
    }
    let mut accepted = BTreeSet::new();
    let (chunks, _) = rest.as_chunks::<ID_LEN>();
    for chunk in chunks {
        let hex = std::str::from_utf8(chunk).map_err(|_| QuickError::BadAnswer)?;
        let id = ItemId::from_hex(hex).map_err(|_| QuickError::BadAnswer)?;
        if !offered.iter().any(|f| f.id == id) || !accepted.insert(id) {
            return Err(QuickError::BadAnswer);
        }
    }
    Ok(accepted)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A receiver's channel answering with `answer`, for the sender to read.
    struct Answer(Option<Vec<u8>>);

    impl Channel for Answer {
        async fn send(&mut self, _: &[u8]) -> Result<(), pctwin_transfer::ChannelError> {
            Ok(())
        }
        async fn recv(&mut self) -> Result<Vec<u8>, pctwin_transfer::ChannelError> {
            self.0.take().ok_or(pctwin_transfer::ChannelError)
        }
    }

    fn offered(n: u8) -> Vec<OfferedFile> {
        (0..n)
            .map(|i| OfferedFile {
                id: ItemId::from_hex(&format!("{i:032x}")).unwrap(),
                path: format!("f{i}"),
                size: 1,
            })
            .collect()
    }

    fn answer(ids: &[ItemId], count: u32) -> Vec<u8> {
        let mut m = vec![ACCEPTED];
        m.extend_from_slice(&count.to_be_bytes());
        for id in ids {
            m.extend_from_slice(id.to_hex().as_bytes());
        }
        m
    }

    async fn read(msg: Vec<u8>, offer: &[OfferedFile]) -> Result<BTreeSet<ItemId>, QuickError> {
        read_accepted(&mut Answer(Some(msg)), offer).await
    }

    /// The answer names only offered files, each once, with the right count and nothing more.
    #[tokio::test]
    async fn an_answer_is_read_only_in_its_exact_form() {
        let offer = offered(3);
        let ids: Vec<ItemId> = offer.iter().map(|f| f.id).collect();
        assert_eq!(read(answer(&ids[..2], 2), &offer).await.unwrap().len(), 2);
        assert!(read(answer(&[], 0), &offer).await.unwrap().is_empty());
        let stranger = ItemId::from_hex(&format!("{:032x}", 99)).unwrap();
        for bad in [
            answer(&[stranger], 1),
            answer(&[ids[0], ids[0]], 2),
            answer(&ids[..2], 3),
            answer(&ids[..2], 1),
            answer(&ids, 4),
            vec![ACCEPTED, 0, 0],
            {
                let mut m = answer(&ids[..1], 1);
                m[0] = 7;
                m
            },
            {
                let mut m = answer(&ids[..1], 1);
                m.push(b'0');
                m
            },
        ] {
            assert!(
                matches!(read(bad.clone(), &offer).await, Err(QuickError::BadAnswer)),
                "{bad:?}"
            );
        }
    }
}
