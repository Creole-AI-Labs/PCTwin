//! A journal put back from an older copy is refused (second security review, finding S2,
//! 27 October 2026).
//!
//! Every record that undo trusts is sealed ([`crate::keystore`]), but a whole older copy of the
//! journal file (a backup put back, a copy saved before an undo) is sealed right too: it would
//! make undo forget that it had already removed an app, and remove the person's own reinstall.
//!
//! So the journal counts its sealed writes. Each transaction that writes a sealed record (an
//! effect, a step of undoing one, a broker action) also raises a counter kept in the journal,
//! sealed itself, in that same transaction; once it is on disk, the same number is written to the
//! system's key store beside the record key ([`crate::keystore::KeyStore::put_mark`]). When the
//! key is used, the journal's own counter must be at least the one in the key store: an older
//! journal reads as [`crate::JournalError::RolledBack`], is reported, and drives nothing.
//!
//! The order (journal first, key store second) means a crash between the two leaves the journal
//! ahead, which is accepted and catches the key store up; never a journal wrongly refused. A key
//! store that holds the record key but no counter, beside a journal that has counted, is refused
//! too (the counter is made with the key, so it was removed).
//!
//! Closing undo is counted in every journal, also one that holds only Phase 1 file records (third
//! security review, finding 9): such a journal gets its record key at the close. Before any undo,
//! a journal with no counter at all is refused once the key store holds a count for it.
//!
//! What this is not: a wall against a program running as the person, which can write the key
//! store as PCTwin does; and if writing the key store fails after a commit, the key store lags
//! until the next write or open, so a copy that much older passes.

use redb::{
    ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition, WriteTransaction,
};

use crate::keystore::{KeyError, TagKey};
use crate::{Journal, JournalError, damaged, storage};

pub(crate) const SEAL_MARK: TableDefinition<&str, &[u8]> = TableDefinition::new("seal-mark");
const ROW: &str = "mark";
const KIND: &str = "seal-mark";

fn read_in(
    table: &impl ReadableTable<&'static str, &'static [u8]>,
    key: &TagKey,
) -> Result<u64, JournalError> {
    let Some(v) = table.get(ROW).map_err(storage)? else {
        return Ok(0);
    };
    let bytes = v.value();
    let (tag, body) = bytes
        .split_first_chunk::<32>()
        .ok_or(damaged("the record's counter is too short"))?;
    let n = <[u8; 8]>::try_from(body).map_err(|_| damaged("the record's counter is damaged"))?;
    if !key.verify(KIND, 0, &n, tag) {
        return Err(damaged("the record's counter was changed outside PCTwin"));
    }
    Ok(u64::from_be_bytes(n))
}

/// Raises the journal's counter inside `tx`; returns the new value.
pub(crate) fn bump(tx: &WriteTransaction, key: &TagKey) -> Result<u64, JournalError> {
    let mut table = tx.open_table(SEAL_MARK).map_err(storage)?;
    let next = read_in(&table, key)?
        .checked_add(1)
        .ok_or(damaged("the record's counter is full"))?;
    let n = next.to_be_bytes();
    let bytes = [key.tag(KIND, 0, &n).as_slice(), &n].concat();
    table.insert(ROW, bytes.as_slice()).map_err(storage)?;
    Ok(next)
}

impl Journal {
    /// The journal's own counter, as on disk now.
    fn journal_mark(&self, key: &TagKey) -> Result<u64, JournalError> {
        let tx = self.db.begin_read().map_err(storage)?;
        match tx.open_table(SEAL_MARK) {
            Ok(table) => read_in(&table, key),
            Err(redb::TableError::TableDoesNotExist(_)) => Ok(0),
            Err(e) => Err(storage(e)),
        }
    }

    /// An error unless this journal is at least as new as the last sealed write PCTwin made to it
    /// (the counter in the key store).
    pub(crate) fn check_mark(&self, key: &TagKey) -> Result<(), JournalError> {
        let mine = self.journal_mark(key)?;
        let mut known = self
            .record_key
            .mark
            .lock()
            .map_err(|_| damaged("the record counter's lock was broken"))?;
        let kept = match *known {
            Some(m) => Some(m),
            None => self
                .record_key
                .store
                .get_mark(self.journal_id)
                .map_err(|e| match e {
                    KeyError::Missing => JournalError::RolledBack,
                    KeyError::Unavailable => JournalError::RecordKeyUnavailable,
                })?,
        };
        match kept {
            // The key store's counter was removed while the journal has counted.
            None if mine > 0 => return Err(JournalError::RolledBack),
            Some(m) if mine < m => return Err(JournalError::RolledBack),
            _ => {}
        }
        let kept = kept.unwrap_or(0);
        if mine > kept {
            // A crash between the journal's commit and the key store's write: caught up.
            if self
                .record_key
                .store
                .put_mark(self.journal_id, mine)
                .is_ok()
            {
                *known = Some(mine);
            }
        } else {
            *known = Some(kept);
        }
        Ok(())
    }

    /// [`JournalError::RolledBack`] if this journal holds any effect and is older than the last
    /// sealed write; otherwise nothing (a key that cannot be had is dealt with where it is used).
    pub(crate) fn refuse_if_rolled_back(&self) -> Result<(), JournalError> {
        let tx = self.db.begin_read().map_err(storage)?;
        let effects = tx.open_table(crate::effects::EFFECTS).map_err(storage)?;
        if effects.is_empty().map_err(storage)? {
            return Ok(());
        }
        match self.record_key() {
            Err(JournalError::RolledBack) => Err(JournalError::RolledBack),
            _ => Ok(()),
        }
    }

    /// Whether the journal has a counter at all (one was ever raised in it).
    fn has_counter(&self) -> Result<bool, JournalError> {
        let tx = self.db.begin_read().map_err(storage)?;
        match tx.open_table(SEAL_MARK) {
            Ok(table) => Ok(table.get(ROW).map_err(storage)?.is_some()),
            Err(redb::TableError::TableDoesNotExist(_)) => Ok(false),
            Err(e) => Err(storage(e)),
        }
    }

    /// Before any undo (third review, finding 9): a journal that has a counter must be at least
    /// as new as the key store says; one with no counter at all is refused once the key store
    /// holds a count for it (a close, or a sealed write, was recorded somewhere: this is an older
    /// copy). A journal that never counted, with nothing counted for it, undoes as before.
    pub(crate) fn refuse_stale_for_undo(&self) -> Result<(), JournalError> {
        if self.has_counter()? {
            return self.record_key().map(|_| ());
        }
        let known = *self
            .record_key
            .mark
            .lock()
            .map_err(|_| damaged("the record counter's lock was broken"))?;
        let kept = match known {
            Some(m) => Some(m),
            None => self
                .record_key
                .store
                .get_mark(self.journal_id)
                .map_err(|e| match e {
                    KeyError::Missing => JournalError::RolledBack,
                    KeyError::Unavailable => JournalError::RecordKeyUnavailable,
                })?,
        };
        match kept {
            Some(_) => Err(JournalError::RolledBack),
            None => Ok(()),
        }
    }

    /// Raises the counter in `tx`, commits it, then writes the new counter to the key store.
    pub(crate) fn commit_sealed(
        &self,
        tx: WriteTransaction,
        key: &TagKey,
    ) -> Result<(), JournalError> {
        let n = bump(&tx, key)?;
        tx.commit().map_err(storage)?;
        self.note_mark(n);
        Ok(())
    }

    /// Writes counter `n`, just committed, to the key store (if that fails, it lags; see the
    /// module notes).
    pub(crate) fn note_mark(&self, n: u64) {
        if self.record_key.store.put_mark(self.journal_id, n).is_ok()
            && let Ok(mut known) = self.record_key.mark.lock()
        {
            *known = Some(known.map_or(n, |k| k.max(n)));
        }
    }
}
