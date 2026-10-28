//! Why a file failed: one typed cause from a fixed list, with the plain words the person reads.
//!
//! Every way a file can fail is matched to its cause here, by exhaustive matches with no catch-all
//! arm: a new error added anywhere below does not build until it is given its cause, so the
//! report can never invent a cause or hide one under "something went wrong".

use std::io;
use std::ops::Deref;

use pctwin_gate::{GateError, PathError};

use crate::TransferError;
use crate::allowance::Refusal;

/// Why a file was not copied, as the report groups it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Cause {
    /// There is not enough free space where it lands.
    NoSpace,
    /// It is bigger than the drive it lands on holds in one file (FAT32: 4 GB).
    TooBigForDrive,
    /// It changed while it was being moved, or since the plan was approved.
    ChangedDuringMove,
    /// The old laptop could not read it.
    Unreadable,
    /// It was no longer on the old laptop when the move reached it (removed or moved since the
    /// scan).
    NoLongerThere,
    /// Another program had it open for writing, so it was not read (never a torn copy); it can
    /// be moved again once that program closes it.
    InUse,
    /// It is stored online only on the old laptop, and is never downloaded to be moved.
    OnlineOnly,
    /// It could not be checked as a true copy (damaged on the way, or stored wrong).
    NotVerified,
    /// The new laptop's safety check refused it (not in the approved plan, or not where planned).
    RefusedBySafetyCheck,
    /// The drive it lands on does not allow its name, or no safe name could be given.
    NameNotAllowed,
    /// The new laptop could not write it.
    CouldNotWrite,
    /// The drive it lands on is locked (BitLocker): unlock it, then move it again.
    DriveLocked,
    /// It was copied, then removed or changed at once, often by security software.
    RemovedAfterCopy,
    /// The person stopped the move before it finished.
    Stopped,
    /// The other laptop did not accept it, and did not say why (the old laptop's view).
    NotAccepted,
    /// The move itself ended before this file finished.
    MoveEnded,
}

/// Which laptop a failure happened on: the same I/O error means reading on the old laptop and
/// writing on the new one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The old laptop, reading.
    Old,
    /// The new laptop, writing.
    New,
}

/// A failed file: its cause, and the plain words that say what happened and what to do next.
/// Reads as its words (`Display`, and as a `str`), so the report shows exactly what is recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    cause: Cause,
    why: String,
}

impl Failure {
    pub fn new(cause: Cause, why: impl Into<String>) -> Self {
        Self {
            cause,
            why: why.into(),
        }
    }

    /// The failure `e` caused on `side`, in its own words.
    pub fn of(e: &TransferError, side: Side) -> Self {
        Self::new(Cause::of(e, side), e.to_string())
    }

    pub fn cause(&self) -> Cause {
        self.cause
    }

    pub fn why(&self) -> &str {
        &self.why
    }
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.why)
    }
}

impl Deref for Failure {
    type Target = str;

    fn deref(&self) -> &str {
        &self.why
    }
}

impl From<Refusal> for Failure {
    fn from(r: Refusal) -> Self {
        Self::new(Cause::of_refusal(r), r.to_string())
    }
}

/// The old laptop's word for why it did not send a planned file ([`Message::NotSent`]): only
/// what the old laptop can know, each with the words the new laptop shows.
const SAID_BY_THE_OLD_LAPTOP: [(Cause, u8, &str); 6] = [
    (
        Cause::NoLongerThere,
        0,
        "it was no longer on the old laptop when the move reached it",
    ),
    (Cause::Unreadable, 1, "the old laptop could not read it"),
    (
        Cause::OnlineOnly,
        2,
        "it is stored online only on the old laptop, so it was not downloaded to be moved",
    ),
    (
        Cause::ChangedDuringMove,
        3,
        "it kept changing on the old laptop, so it was not sent; close the program using it and move it again",
    ),
    (
        Cause::NotVerified,
        4,
        "the old laptop could not read it the same way twice, so it was not sent",
    ),
    (
        Cause::InUse,
        5,
        "another program had it open on the old laptop, so it was not sent; move it again once it is closed",
    ),
];

impl Cause {
    /// How the old laptop says this cause on the wire, if it is one it can say.
    pub(crate) fn said(self) -> Option<u8> {
        SAID_BY_THE_OLD_LAPTOP
            .iter()
            .find(|(c, _, _)| *c == self)
            .map(|(_, code, _)| *code)
    }

    /// The failure the old laptop said with `code` (`None`: not a code it may say).
    pub(crate) fn heard(code: u8) -> Option<Failure> {
        SAID_BY_THE_OLD_LAPTOP
            .iter()
            .find(|(_, c, _)| *c == code)
            .map(|(cause, _, why)| Failure::new(*cause, *why))
    }

    /// The cause of `e` on `side`.
    pub fn of(e: &TransferError, side: Side) -> Self {
        match e {
            TransferError::Io(io) => Self::of_io(io, side),
            TransferError::OnlineOnly => Self::OnlineOnly,
            TransferError::InUse => Self::InUse,
            TransferError::Damaged(_)
            | TransferError::OutOfOrder { .. }
            | TransferError::Incomplete { .. }
            | TransferError::NotStored { .. } => Self::NotVerified,
            TransferError::ChangedSince | TransferError::ChangedWhileRead => {
                Self::ChangedDuringMove
            }
            TransferError::Gate(g) => Self::of_gate(g),
            TransferError::Stopped | TransferError::StoppedThere => Self::Stopped,
            TransferError::ConnectionDropped
            | TransferError::Protocol(_)
            | TransferError::Record(_)
            | TransferError::Inventory(_)
            | TransferError::VersionGap(_)
            | TransferError::NoAnswer(_) => Self::MoveEnded,
        }
    }

    /// The cause of the safety gate's refusal `e` (always on the new laptop).
    pub fn of_gate(e: &GateError) -> Self {
        match e {
            GateError::Path(p) => match p {
                PathError::ComponentTooLong | PathError::TooDeep | PathError::TooLong => {
                    Self::NameNotAllowed
                }
                PathError::Empty
                | PathError::Absolute
                | PathError::Traversal
                | PathError::EmptyComponent
                | PathError::DotComponent
                | PathError::NulCharacter => Self::RefusedBySafetyCheck,
            },
            GateError::Conflict
            | GateError::WholeTaken
            | GateError::TooManyClashes
            | GateError::NoSafeName
            | GateError::NameRejected => Self::NameNotAllowed,
            GateError::SizeMismatch { .. } | GateError::NotReadBack => Self::NotVerified,
            GateError::UnknownDestination
            | GateError::DuplicateDestination
            | GateError::BadDestinationId
            | GateError::NeedsAdminHelper
            | GateError::OutsideFile
            | GateError::Overlap
            | GateError::NotArrived
            | GateError::TooScattered
            | GateError::BadTag
            | GateError::BadBlockList => Self::RefusedBySafetyCheck,
            GateError::NoSpace => Self::NoSpace,
            GateError::TooBigForDrive { .. } => Self::TooBigForDrive,
            GateError::ReplacedWhileFilling => Self::CouldNotWrite,
            GateError::Io(io) => Self::of_io(io, Side::New),
        }
    }

    /// The cause of the new laptop's refusal to start a file.
    pub fn of_refusal(r: Refusal) -> Self {
        match r {
            Refusal::NotInPlan
            | Refusal::NotWherePlanned
            | Refusal::OverTotal
            | Refusal::AlreadyMoved
            | Refusal::AlreadyStarted
            | Refusal::StartedTooOften => Self::RefusedBySafetyCheck,
            Refusal::GrewTooMuch { .. } | Refusal::Grew { .. } | Refusal::KeptChanging => {
                Self::ChangedDuringMove
            }
        }
    }

    /// The cause of the I/O error `e` on `side`: a full drive or a refused name on the new
    /// laptop is said so; anything else is a read on the old laptop or a write on the new one.
    pub fn of_io(e: &io::Error, side: Side) -> Self {
        match side {
            Side::Old if e.kind() == io::ErrorKind::NotFound => Self::NoLongerThere,
            Side::Old => Self::Unreadable,
            Side::New if pctwin_gate::is_drive_locked(e) => Self::DriveLocked,
            Side::New => match e.kind() {
                io::ErrorKind::StorageFull | io::ErrorKind::QuotaExceeded => Self::NoSpace,
                io::ErrorKind::FileTooLarge => Self::TooBigForDrive,
                io::ErrorKind::InvalidFilename => Self::NameNotAllowed,
                _ => Self::CouldNotWrite,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Gap 9 (28 October 2026): a locked drive on the new laptop is said as locked (unlock it
    /// first), never as a generic write failure.
    #[test]
    fn a_locked_drive_on_the_new_laptop_is_said_as_locked() {
        let locked = io::Error::from_raw_os_error(0x8031_0000_u32 as i32);
        let expected = if cfg!(windows) {
            Cause::DriveLocked
        } else {
            Cause::CouldNotWrite
        };
        assert_eq!(Cause::of_io(&locked, Side::New), expected);
        assert_eq!(
            Cause::of_gate(&GateError::Io(io::Error::from_raw_os_error(
                0x8031_0000_u32 as i32
            ))),
            expected
        );
        assert_eq!(Cause::of_io(&locked, Side::Old), Cause::Unreadable);
    }

    #[test]
    fn a_full_drive_on_the_new_laptop_is_no_space_and_on_the_old_one_a_read() {
        let full = io::Error::from(io::ErrorKind::StorageFull);
        assert_eq!(Cause::of_io(&full, Side::New), Cause::NoSpace);
        assert_eq!(Cause::of_io(&full, Side::Old), Cause::Unreadable);
        let denied = io::Error::from(io::ErrorKind::PermissionDenied);
        assert_eq!(Cause::of_io(&denied, Side::New), Cause::CouldNotWrite);
        let name = io::Error::from(io::ErrorKind::InvalidFilename);
        assert_eq!(Cause::of_io(&name, Side::New), Cause::NameNotAllowed);
    }

    #[test]
    fn the_gate_s_refusals_are_named() {
        assert_eq!(Cause::of_gate(&GateError::NoSpace), Cause::NoSpace);
        assert_eq!(
            Cause::of_gate(&GateError::NameRejected),
            Cause::NameNotAllowed
        );
        assert_eq!(
            Cause::of_gate(&GateError::Path(PathError::Traversal)),
            Cause::RefusedBySafetyCheck
        );
        assert_eq!(
            Cause::of_gate(&GateError::Path(PathError::ComponentTooLong)),
            Cause::NameNotAllowed
        );
        assert_eq!(
            Cause::of_gate(&GateError::TooBigForDrive {
                largest: u32::MAX as u64
            }),
            Cause::TooBigForDrive
        );
        assert_eq!(Cause::of_gate(&GateError::NotReadBack), Cause::NotVerified);
    }

    #[test]
    fn a_failure_reads_as_its_words_and_keeps_its_cause() {
        let f = Failure::from(Refusal::NotInPlan);
        assert_eq!(f.cause(), Cause::RefusedBySafetyCheck);
        assert!(f.contains("plan"));
        assert_eq!(f.to_string(), Refusal::NotInPlan.to_string());
        let changed = Failure::from(Refusal::Grew {
            approved: 1,
            announced: 2,
        });
        assert_eq!(changed.cause(), Cause::ChangedDuringMove);
        let read = Failure::of(&TransferError::ChangedWhileRead, Side::Old);
        assert_eq!(read.cause(), Cause::ChangedDuringMove);
        let stopped = Failure::of(&TransferError::Stopped, Side::New);
        assert_eq!(stopped.cause(), Cause::Stopped);
    }
}
