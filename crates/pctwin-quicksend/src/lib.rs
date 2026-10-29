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

pub mod note;

pub use note::{Note, NoteError, NoteKind};

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use pctwin_gate::Destinations;
use pctwin_journal::{DownloadMark, Ledger};
use pctwin_record::{
    FolderRole, Inclusion, Item, ItemId, ItemKind, ItemName, ItemPath, LaptopId, LeftOutReason,
    ManagedBy, Owner, Place, Portability, Record, RecordError, Storage,
};
use pctwin_transfer::{
    Allowance, Cause, Channel, KEEP_FREE, LandingPlace, MOVE_PROTOCOL, ReceiveOutcome,
    ReceiverSession, SendJob, SendOutcome, SenderSession, SizeRoom, TransferError, agree_version,
    receive_inventory, send_inventory, shown_path,
};

/// The destination label everything of a Quick Send lands under: the folder the receiver chose.
const DESTINATION: &str = "quick-send";

/// Files sent at once.
const LANES: usize = 4;

/// The receiver's answer: which offered files to send.
const ACCEPTED: u8 = 36;
/// An item id in the answer: 32 hex characters.
const ID_LEN: usize = 32;

/// What this exchange carries, said first: `[40][what]`.
const WHAT: u8 = 40;
const WHAT_FILES: u8 = 1;
const WHAT_NOTE: u8 = 2;
/// A note: `[41][its text, UTF-8]`.
const NOTE: u8 = 41;
/// The receiver took the note (it arrived and passed the checks; not that anyone read it).
const NOTE_TAKEN: u8 = 42;

/// How long a sender waits for a note to be taken: no person answers it, so not long.
pub const NOTE_WAIT: std::time::Duration = std::time::Duration::from_secs(60);

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
    #[error("the other laptop did not answer the offer in time")]
    NotAnswered,
    /// The note does not follow the rules (on the sender, before anything is sent; on the
    /// receiver, refused on arrival).
    #[error(transparent)]
    Note(#[from] NoteError),
    #[error("there are more files ({files} so far) than one send can carry")]
    TooMany { files: usize },
    /// The receiving folder has no room for what was offered (keeping [`KEEP_FREE`] spare): the
    /// offer was declined and nothing was written. `room` is what could be used there.
    #[error("not enough room: {needed} bytes needed, {room} bytes can be used")]
    NoRoom { needed: u64, room: u64 },
}

/// Past this many items an offer stops counting (one send carries fewer still: see
/// [`QuickError::TooMany`]), so choosing a vast folder never fills this laptop's memory.
const MOST_OFFERED_ITEMS: usize = 1_000_000;

/// How long the sender waits for the receiver's person to answer the offer before it ends,
/// honestly, as not answered.
pub const ANSWER_WAIT: std::time::Duration = std::time::Duration::from_secs(10 * 60);

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
        Self::of_again(chosen, LaptopId::generate()?)
    }

    /// The offer [`of`](Self::of) made again for the same `chosen` things under the `laptop` ID
    /// of an earlier offer ([`laptop`](Self::laptop)), so every file has the ID it had then: the
    /// receiver knows which it already holds, whole or partway. With
    /// [`only_missing`](Self::only_missing), "Send the missing pieces".
    pub fn of_again(chosen: &[PathBuf], laptop: LaptopId) -> Result<Self, QuickError> {
        Self::of_within(chosen, laptop, pctwin_record::MAX_HELD)
    }

    /// As [`of_again`](Self::of_again), refusing an offer whose list the receiver could not hold
    /// within `held` bytes (the receiver's limit for one list: [`pctwin_record::MAX_HELD`]), said
    /// plainly before anything is sent ([`QuickError::TooMany`]).
    fn of_within(chosen: &[PathBuf], laptop: LaptopId, held: u64) -> Result<Self, QuickError> {
        if chosen.is_empty() {
            return Err(QuickError::NothingChosen);
        }
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
                if record.items.len() > MOST_OFFERED_ITEMS {
                    return Err(QuickError::TooMany {
                        files: record.items.len(),
                    });
                }
            } else {
                return Err(QuickError::NotThere(path.clone()));
            }
        }
        record.validate()?;
        if !pctwin_record::fits_one_move(&record.items, held)? {
            return Err(QuickError::TooMany {
                files: record
                    .items
                    .iter()
                    .filter(|i| i.kind == ItemKind::File)
                    .count(),
            });
        }
        // Sent as a move sends: in the order a move plans, each landing inside the chosen folder.
        // What a folder holds that cannot be read is left out by the record and not sent.
        let jobs = pctwin_move::send_plan(&record, |i| sources.get(&i.id).cloned(), landing).jobs;
        Ok(Self { record, jobs })
    }

    /// The files offered, in path order.
    pub fn files(&self) -> Vec<OfferedFile> {
        files_of(&self.record)
    }

    /// The ID this offer's files are made under, to offer the missing pieces later with
    /// [`of_again`](Self::of_again).
    pub fn laptop(&self) -> LaptopId {
        self.record.source_laptop
    }

    /// Only what `receipt` (of the earlier send of this very offer) says did not arrive: files
    /// that failed or were not accepted (as "Send the missing pieces" lists them). What arrived,
    /// or was already there, or is there needing action, is kept and not offered again. A file
    /// the receiver holds partway is continued there from the blocks it has.
    pub fn only_missing(mut self, receipt: &Receipt) -> Self {
        // By the file's ID, never by how its name shows (two names can show alike).
        let missing: BTreeSet<ItemId> = receipt
            .lines
            .iter()
            .filter(|l| matches!(l.outcome, Outcome::Failed { .. } | Outcome::Declined))
            .map(|l| l.id)
            .collect();
        // A file the receipt does not list at all (it is new since) is offered too.
        let listed: BTreeSet<ItemId> = receipt.lines.iter().map(|l| l.id).collect();
        let keep = |i: &Item| {
            i.kind != ItemKind::File || missing.contains(&i.id) || !listed.contains(&i.id)
        };
        self.record.items.retain(|i| keep(i));
        let ids: BTreeSet<ItemId> = self.record.items.iter().map(|i| i.id).collect();
        self.jobs.retain(|j| ids.contains(&j.item));
        self
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
    /// The file, by its ID in the send (what "Send the missing pieces" goes by: two names that
    /// show alike are never taken for one another).
    pub id: ItemId,
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
    send_live(ch, offer, &pctwin_move::Live::default()).await
}

/// As [`send`], counted in `live`'s meter and held or stopped by its control (the screens'
/// progress, pause and stop). A stop, on either laptop, ends it with the receipt so far: a file
/// it cut off failed, never arrived.
pub async fn send_live(
    ch: &mut impl Channel,
    offer: Offer,
    live: &pctwin_move::Live,
) -> Result<Receipt, QuickError> {
    let version = agree_version(ch, MOVE_PROTOCOL).await?;
    write_what(ch, WHAT_FILES).await?;
    send_inventory(ch, &offer.record, version).await?;
    let offered = files_of(&offer.record);
    let Some(accepted) = wait_for_answer(ch, &offered, live).await? else {
        // Stopped here while waiting: nothing was sent, and the receipt says so.
        return Ok(Receipt {
            lines: offered
                .into_iter()
                .map(|f| ReceiptLine {
                    id: f.id,
                    path: f.path,
                    outcome: Outcome::Failed {
                        cause: Cause::Stopped,
                        why: "the send was stopped before it was sent".into(),
                    },
                })
                .collect(),
        });
    };
    let jobs: Vec<SendJob> = offer
        .jobs
        .into_iter()
        .filter(|j| accepted.contains(&j.item))
        .collect();
    let mut sender = SenderSession::new(jobs, LANES)
        .with_protocol(version)
        .with_meter(live.meter.clone())
        .with_control(live.control.clone());
    if !accepted.is_empty() {
        stopped_is_fine(sender.run(ch).await)?;
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
                    id: f.id,
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

/// What arrived over a Quick Send link.
pub enum Arrival {
    /// Files offered, for the person to say where they go.
    Files(Incoming),
    /// A note or link, checked, to show with Copy (and Open for a link, only when the person
    /// chooses it). Nothing was written.
    Note(Note),
}

/// Receiver: what the other laptop sends over `ch` (already paired): an offer of files (names
/// and sizes only, refused unless it is a Quick Send: everything in it shared, nothing about
/// people), or a note, checked by the same rules the sender used and taken.
pub async fn receive(ch: &mut impl Channel) -> Result<Arrival, QuickError> {
    let version = agree_version(ch, MOVE_PROTOCOL).await?;
    match read_what(ch).await? {
        WHAT_NOTE => {
            let note = read_note(ch).await?;
            ch.send(&[NOTE_TAKEN])
                .await
                .map_err(|_| TransferError::ConnectionDropped)?;
            Ok(Arrival::Note(note))
        }
        _ => {
            let record = receive_inventory(ch, version).await?;
            if record.items.iter().any(|i| i.owner != Owner::Shared) {
                return Err(QuickError::NotAQuickSend);
            }
            Ok(Arrival::Files(Incoming { record, version }))
        }
    }
}

/// Receiver: as [`receive`], for an offer of files only (a note is refused as not one).
pub async fn receive_offer(ch: &mut impl Channel) -> Result<Incoming, QuickError> {
    match receive(ch).await? {
        Arrival::Files(incoming) => Ok(incoming),
        Arrival::Note(_) => Err(QuickError::NotAQuickSend),
    }
}

/// Sender: sends `note` over `ch` (already paired) and waits, at most [`NOTE_WAIT`], until the
/// other laptop says it took it (it arrived and passed its checks; never that it was read).
pub async fn send_note(ch: &mut impl Channel, note: &Note) -> Result<(), QuickError> {
    agree_version(ch, MOVE_PROTOCOL).await?;
    write_what(ch, WHAT_NOTE).await?;
    let mut msg = Vec::with_capacity(1 + note.text().len());
    msg.push(NOTE);
    msg.extend_from_slice(note.text().as_bytes());
    ch.send(&msg)
        .await
        .map_err(|_| TransferError::ConnectionDropped)?;
    let taken = tokio::time::timeout(NOTE_WAIT, ch.recv())
        .await
        .map_err(|_| QuickError::NotAnswered)?
        .map_err(|_| TransferError::ConnectionDropped)?;
    if taken != [NOTE_TAKEN] {
        return Err(QuickError::BadAnswer);
    }
    Ok(())
}

async fn write_what(ch: &mut impl Channel, what: u8) -> Result<(), QuickError> {
    ch.send(&[WHAT, what])
        .await
        .map_err(|_| TransferError::ConnectionDropped)?;
    Ok(())
}

/// What the exchange carries: files or a note, and nothing else.
async fn read_what(ch: &mut impl Channel) -> Result<u8, QuickError> {
    let msg = ch
        .recv()
        .await
        .map_err(|_| TransferError::ConnectionDropped)?;
    match msg.as_slice() {
        [WHAT, what @ (WHAT_FILES | WHAT_NOTE)] => Ok(*what),
        _ => Err(QuickError::NotAQuickSend),
    }
}

/// A note as it came: its exact form, its size limit before anything is decoded, then the rules.
async fn read_note(ch: &mut impl Channel) -> Result<Note, QuickError> {
    let msg = ch
        .recv()
        .await
        .map_err(|_| TransferError::ConnectionDropped)?;
    let (kind, body) = msg.split_first().ok_or(QuickError::BadAnswer)?;
    if *kind != NOTE || body.len() > note::MAX_NOTE_BYTES {
        return Err(QuickError::BadAnswer);
    }
    let text = std::str::from_utf8(body).map_err(|_| QuickError::BadAnswer)?;
    Ok(Note::arrived(text)?)
}

impl Incoming {
    /// The files offered, in path order.
    pub fn files(&self) -> Vec<OfferedFile> {
        files_of(&self.record)
    }

    /// The size of everything offered, in bytes.
    pub fn total_bytes(&self) -> u64 {
        // Sizes come from someone else's laptop: added without ever wrapping.
        self.files()
            .iter()
            .fold(0u64, |t, f| t.saturating_add(f.size))
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
        self.accept_live(
            ch,
            dest,
            chosen,
            journal,
            account,
            &pctwin_move::Live::default(),
        )
        .await
    }

    /// As [`accept`](Self::accept), counted in `live`'s meter and held or stopped by its
    /// control. A stop ends it with the receipt so far.
    pub async fn accept_live(
        self,
        ch: &mut impl Channel,
        dest: &Path,
        chosen: Option<&[ItemId]>,
        journal: &dyn Ledger,
        account: &str,
        live: &pctwin_move::Live,
    ) -> Result<Receipt, QuickError> {
        let dest = pctwin_gate::Destination::open(dest)
            .map_err(|e| QuickError::Destination(e.to_string()))?;
        self.accept_into(ch, dest, chosen, journal, account, live)
            .await
    }

    /// As [`accept_live`](Self::accept_live), into the very folder `dest` opened when the person
    /// chose it in the system's picker: a folder put at that path since (another folder, a link
    /// somewhere else) is never the one written to.
    pub async fn accept_into(
        self,
        ch: &mut impl Channel,
        dest: pctwin_gate::Destination,
        chosen: Option<&[ItemId]>,
        journal: &dyn Ledger,
        account: &str,
        live: &pctwin_move::Live,
    ) -> Result<Receipt, QuickError> {
        let free = dest.available_bytes();
        self.accept_into_with_free(ch, dest, chosen, journal, account, live, free)
            .await
    }

    /// As [`accept_into`](Self::accept_into), with the folder's free space as measured (`None`:
    /// the drive cannot say). Before anything is accepted, an offer the folder has no room for
    /// (with [`KEEP_FREE`] spare) is declined over the wire, so the sender is told and nothing is
    /// sent or written. Free space that cannot be measured does not refuse: the gate still never
    /// overwrites, and a file that cannot be written ends failed, said as that.
    #[allow(clippy::too_many_arguments)]
    pub async fn accept_into_with_free(
        self,
        ch: &mut impl Channel,
        dest: pctwin_gate::Destination,
        chosen: Option<&[ItemId]>,
        journal: &dyn Ledger,
        account: &str,
        live: &pctwin_move::Live,
        free: Option<u64>,
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
        let needed = offered
            .iter()
            .filter(|f| wanted.contains(&f.id))
            .fold(0u64, |t, f| t.saturating_add(f.size));
        if let Err(no_room) = room_for(needed, free) {
            write_accepted(ch, &BTreeSet::new()).await?;
            return Err(no_room);
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
            .approve_chosen(DESTINATION, dest)
            .map_err(|e| QuickError::Destination(e.to_string()))?;
        write_accepted(ch, &wanted).await?;
        let mut receiver = ReceiverSession::new(&table, allowance, journal, account)
            .with_scope(pctwin_journal::Scope::QUICK_SEND)
            .with_protocol(self.version)
            .with_meter(live.meter.clone())
            .with_control(live.control.clone());
        // A file of this offer received partway before (a send stopped or cut off, then "Send
        // the missing pieces") continues from the blocks this laptop's journal says it has.
        receiver.restore()?;
        if !wanted.is_empty() {
            stopped_is_fine(receiver.run(ch).await)?;
        }
        Ok(Receipt {
            lines: offered
                .into_iter()
                .map(|f| {
                    let outcome = if !wanted.contains(&f.id) {
                        Outcome::Declined
                    } else {
                        received(receiver.outcome(f.id).as_ref())
                    };
                    ReceiptLine {
                        id: f.id,
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

/// Whether `needed` bytes fit where `free` bytes are free, keeping [`KEEP_FREE`] spare. Unknown
/// free space fits (see [`Incoming::accept_into_with_free`]).
fn room_for(needed: u64, free: Option<u64>) -> Result<(), QuickError> {
    match free {
        Some(free) if needed.saturating_add(KEEP_FREE) > free => Err(QuickError::NoRoom {
            needed,
            room: free.saturating_sub(KEEP_FREE),
        }),
        _ => Ok(()),
    }
}

/// The words for a file from someone else's laptop that landed on a drive that could not mark it
/// as downloaded (FAT and exFAT sticks, some network drives): the system will not warn when it is
/// opened, so the person is told to take care.
pub const MARK_NOT_KEPT: &str =
    "This drive could not mark the file as downloaded from another laptop. Open it with care.";

/// How a file the receiver accepted came to be, from the receiving session's outcome. A file from
/// someone else's laptop whose "downloaded" mark the drive could not keep needs the person's care:
/// never simply "arrived".
fn received(outcome: Option<&ReceiveOutcome>) -> Outcome {
    match outcome {
        Some(ReceiveOutcome::Finished(done)) if done.mark_not_kept.is_some() => {
            Outcome::NeedsAction {
                landed_as: done.final_path.clone(),
                why: MARK_NOT_KEPT.into(),
            }
        }
        Some(ReceiveOutcome::Finished(done)) => Outcome::Arrived {
            landed_as: done.final_path.clone(),
        },
        // Not finished as a visible, proven copy: never "arrived".
        Some(ReceiveOutcome::NeedsAction { finished, why }) => Outcome::NeedsAction {
            landed_as: finished.final_path.clone(),
            why: why.clone(),
        },
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
}

/// A run that a person stopped (on either laptop) is not an error: the receipt says what each
/// file came to.
fn stopped_is_fine(ran: Result<(), pctwin_transfer::TransferError>) -> Result<(), QuickError> {
    match ran {
        Err(
            pctwin_transfer::TransferError::Stopped | pctwin_transfer::TransferError::StoppedThere,
        ) => Ok(()),
        other => Ok(other?),
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

/// The receiver's answer, waited for at most [`ANSWER_WAIT`]; `None` when the person here stopped
/// the send while it waited (it ends at once: nothing was sent).
async fn wait_for_answer(
    ch: &mut impl Channel,
    offered: &[OfferedFile],
    live: &pctwin_move::Live,
) -> Result<Option<BTreeSet<ItemId>>, QuickError> {
    tokio::select! {
        answer = read_accepted(ch, offered) => answer.map(Some),
        () = live.control.wait_stopped() => Ok(None),
        () = tokio::time::sleep(ANSWER_WAIT) => Err(QuickError::NotAnswered),
    }
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

    /// Exit-gate review (24 October 2026): "Send the missing pieces" goes by each file's ID, so
    /// two files whose names show alike (names that are not text, on Linux) are never taken for
    /// one another: only the one that did not arrive is offered again.
    #[test]
    fn the_missing_pieces_are_found_by_id_never_by_a_name_that_shows_alike() {
        let laptop = LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap();
        let mut record = Record::new(laptop);
        let file = |raw: &[u8]| {
            let path = ItemPath::new(vec![ItemName::from_unix_bytes(raw)]).unwrap();
            Item {
                id: ItemId::derive(&laptop, &Owner::Shared, &place(), &path),
                kind: ItemKind::File,
                owner: Owner::Shared,
                place: place(),
                path,
                size_bytes: 1,
                modified_ns: None,
                portability: Portability::Portable,
                managed_by: ManagedBy::Personal,
                download_bytes: None,
                inclusion: Inclusion::Included,
            }
        };
        // Two names that are not text, shown alike.
        let (a, b) = (file(&[b'n', 0xFF]), file(&[b'n', 0xFE]));
        assert_eq!(shown_path(&a.path), shown_path(&b.path));
        assert_ne!(a.id, b.id);
        record.items = vec![a.clone(), b.clone()];
        let offer = Offer {
            record,
            jobs: Vec::new(),
        };
        let receipt = Receipt {
            lines: vec![
                ReceiptLine {
                    id: a.id,
                    path: shown_path(&a.path),
                    outcome: Outcome::Arrived {
                        landed_as: "n".into(),
                    },
                },
                ReceiptLine {
                    id: b.id,
                    path: shown_path(&b.path),
                    outcome: Outcome::Failed {
                        cause: Cause::NotVerified,
                        why: "no".into(),
                    },
                },
            ],
        };
        let again = offer.only_missing(&receipt);
        let ids: Vec<ItemId> = again.files().iter().map(|f| f.id).collect();
        assert_eq!(ids, [b.id]);
    }

    /// Exit-gate review (24 October 2026): a stranger's file on a drive that cannot keep the
    /// "downloaded" mark (no marker drive: FAT, exFAT, some network drives) is never just "arrived":
    /// it needs the person's care, in plain words. One with its mark kept arrived.
    #[test]
    fn a_strangers_file_whose_mark_was_not_kept_needs_care() {
        let done = |mark| {
            ReceiveOutcome::Finished(pctwin_gate::Finished {
                final_path: "Holiday/beach.jpg".into(),
                sent_path: "Holiday/beach.jpg".into(),
                changes: Vec::new(),
                mark_not_kept: mark,
                time_not_kept: false,
                checked_from: None,
            })
        };
        assert_eq!(
            received(Some(&done(Some(DownloadMark::Internet)))),
            Outcome::NeedsAction {
                landed_as: "Holiday/beach.jpg".into(),
                why: MARK_NOT_KEPT.into()
            }
        );
        assert_eq!(
            received(Some(&done(None))),
            Outcome::Arrived {
                landed_as: "Holiday/beach.jpg".into()
            }
        );
    }

    /// Exit-gate review (24 October 2026): a folder holding more files than the receiver can take
    /// in one list is refused when the offer is made, before anything is sent, saying how many.
    #[test]
    fn an_offer_of_more_files_than_one_send_carries_is_refused_before_sending() {
        let root = tempfile::tempdir().unwrap();
        let folder = root.path().join("Big");
        std::fs::create_dir_all(&folder).unwrap();
        for n in 0..40 {
            std::fs::write(folder.join(format!("{n}.txt")), b"x").unwrap();
        }
        let laptop = LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap();
        let fits = Offer::of_within(
            std::slice::from_ref(&folder),
            laptop,
            pctwin_record::MAX_HELD,
        );
        assert_eq!(fits.unwrap().files().len(), 40);
        // A receiver that could hold far less: refused, saying how many.
        let too_many = Offer::of_within(&[folder], laptop, 1);
        assert!(
            matches!(too_many, Err(QuickError::TooMany { files: 40 })),
            "{:?}",
            too_many.err()
        );
    }

    /// A receiver that never answers.
    struct Silent;

    impl Channel for Silent {
        async fn send(&mut self, _: &[u8]) -> Result<(), pctwin_transfer::ChannelError> {
            Ok(())
        }
        async fn recv(&mut self) -> Result<Vec<u8>, pctwin_transfer::ChannelError> {
            std::future::pending().await
        }
    }

    /// Exit-gate review (24 October 2026): a receiver that never answers the offer is waited for
    /// at most [`ANSWER_WAIT`], then the send ends as not answered; the person here stopping ends
    /// the wait at once.
    #[tokio::test(start_paused = true)]
    async fn the_wait_for_an_answer_ends_in_time_or_at_a_stop() {
        let offer = offered(2);
        let live = pctwin_move::Live::default();
        let start = tokio::time::Instant::now();
        let got = wait_for_answer(&mut Silent, &offer, &live).await;
        assert!(matches!(got, Err(QuickError::NotAnswered)), "{got:?}");
        assert!(start.elapsed() >= ANSWER_WAIT);

        let live = pctwin_move::Live::default();
        let stop = live.control.clone();
        let start = tokio::time::Instant::now();
        let mut silent = Silent;
        let (got, ()) = tokio::join!(wait_for_answer(&mut silent, &offer, &live), async {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            stop.stop();
        });
        assert!(matches!(got, Ok(None)), "{got:?}");
        assert!(start.elapsed() < ANSWER_WAIT);
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

#[cfg(test)]
mod totals {
    use super::*;
    use pctwin_record::{
        FolderRole, Item, ItemId, ItemKind, ItemName, ItemPath, LaptopId, ManagedBy, Place,
        Portability, Storage,
    };

    #[test]
    fn an_offer_s_total_never_wraps_however_big_its_sizes_say_they_are() {
        let laptop = LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap();
        let mut record = Record::new(laptop);
        for name in ["a.bin", "b.bin"] {
            let place = Place {
                role: FolderRole::Other,
                storage: Storage::SystemDrive,
            };
            let path = ItemPath::new(vec![ItemName::from_text(name)]).unwrap();
            record.items.push(Item {
                id: ItemId::derive(&laptop, &Owner::Shared, &place, &path),
                kind: ItemKind::File,
                owner: Owner::Shared,
                place,
                path,
                size_bytes: u64::MAX,
                modified_ns: None,
                portability: Portability::Portable,
                managed_by: ManagedBy::Personal,
                download_bytes: None,
                inclusion: Inclusion::Included,
            });
        }
        let incoming = Incoming { record, version: 4 };
        assert_eq!(incoming.total_bytes(), u64::MAX);
    }
}
