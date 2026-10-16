//! Giving a checked file its real name through the journal, the same way when a file lands and
//! when recovery finishes one: the step is recorded first, then the file is named the way its
//! drive's test found safe (a hard link or a no-replace move; or, on a drive with no safe rename,
//! filled in under the name, each step recorded as it happens).

use std::io;

use pctwin_gate::{Claim, FillSteps, Naming, Sealed};
use pctwin_journal::{FileId, JournalError, Ledger};

use crate::fingerprint_reader;

/// What a file's whole fingerprint is worked out over, to read a filled-in copy back against it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Whole {
    pub(crate) size: u64,
    pub(crate) block_size: u64,
    pub(crate) fingerprint: [u8; 32],
}

/// The journal's steps of a fill. A step the journal cannot record is kept, so the move stops
/// as it does whenever the journal cannot be written.
struct JournalFill<'a> {
    journal: &'a dyn Ledger,
    id: u64,
    whole: Whole,
    broke: Option<JournalError>,
}

impl JournalFill<'_> {
    fn keep(&mut self, r: Result<(), JournalError>) -> io::Result<()> {
        r.map_err(|e| {
            let words = e.to_string();
            self.broke.get_or_insert(e);
            io::Error::other(words)
        })
    }
}

impl FillSteps for JournalFill<'_> {
    fn made(&mut self, made: FileId) -> io::Result<()> {
        let r = self.journal.fill_made(self.id, made);
        self.keep(r)
    }
    fn read_back(&mut self, mut file: &mut dyn io::Read) -> io::Result<bool> {
        let found = fingerprint_reader(&mut file, self.whole.size, self.whole.block_size)?;
        Ok(found == Some(self.whole.fingerprint))
    }
    fn filled(&mut self) -> io::Result<()> {
        let r = self.journal.filled(self.id);
        self.keep(r)
    }
    fn cleared(&mut self) -> io::Result<()> {
        let r = self.journal.fill_cleared(self.id, None);
        self.keep(r)
    }
}

/// Whether a file read back now is exactly the whole file.
pub(crate) fn reads_back_whole(mut file: &mut dyn io::Read, whole: &Whole) -> io::Result<bool> {
    Ok(fingerprint_reader(&mut file, whole.size, whole.block_size)? == Some(whole.fingerprint))
}

/// How giving one name ended, and whether the file was filled in under it (then the journal is
/// at `Filled`, and the file at the name is a new file, not the checked one).
pub(crate) struct Named<'d> {
    pub(crate) claim: Claim<'d>,
    pub(crate) filled_in: bool,
}

/// Records the real name `name` for write `id`, then gives `sealed` that name the way its drive
/// allows. A journal that cannot record a step gives the file back with the error (before the
/// fill, untouched; during it, with PCTwin's own partial copy removed where the name still held
/// it), and the caller stops.
pub(crate) fn name_as<'d>(
    journal: &dyn Ledger,
    id: u64,
    sealed: Sealed<'d>,
    name: &str,
    whole: Whole,
) -> Result<Named<'d>, Box<(JournalError, Sealed<'d>)>> {
    let naming = match sealed.naming() {
        Ok(naming) => naming,
        Err(e) => {
            return Ok(Named {
                claim: Claim::Failed(e, sealed),
                filled_in: false,
            });
        }
    };
    if naming != Naming::FillIn {
        if let Err(e) = journal.applied(id, name) {
            return Err(Box::new((e, sealed)));
        }
        return Ok(Named {
            claim: sealed.claim_as(name),
            filled_in: false,
        });
    }
    let plan = match sealed.fill_plan() {
        Ok(plan) => plan,
        Err(e) => {
            return Ok(Named {
                claim: Claim::Failed(e, sealed),
                filled_in: false,
            });
        }
    };
    if let Err(e) = journal.filling(id, name, plan.tag, plan.proof) {
        return Err(Box::new((e, sealed)));
    }
    let mut steps = JournalFill {
        journal,
        id,
        whole,
        broke: None,
    };
    let claim = sealed.fill_as(name, &plan, &mut steps);
    if let Some(e) = steps.broke {
        return match claim {
            Claim::Failed(_, back) | Claim::Taken(back) => Err(Box::new((e, back))),
            // Named though a step was not recorded (only clearing, which follows a failure, can
            // be): recovery decides from what the journal has.
            Claim::Named(_) => Ok(Named {
                claim,
                filled_in: true,
            }),
        };
    }
    let filled_in = matches!(claim, Claim::Named(_));
    Ok(Named { claim, filled_in })
}
