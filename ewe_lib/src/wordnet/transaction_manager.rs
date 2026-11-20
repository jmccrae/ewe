use std::sync::Arc;

use redb::{Database, Durability, ReadTransaction, WriteTransaction, ReadableDatabase};

use crate::wordnet::Result;

/// Manages a long-lived write transaction for a `redb::Database`.
///
/// - Keeps a single `WriteTransaction` open as long as possible.
/// - Whenever a read is requested, any open write transaction is committed first,
///   then a fresh `ReadTransaction` is created.
/// - When the manager is dropped, any in-flight write transaction is committed.
pub struct TransactionManager {
    db: Arc<Database>,
    write_txn: Option<WriteTransaction>,
    /// Durability of new write transactions (see [`TransactionManager::set_bulk_load`])
    durability: Durability,
}

impl TransactionManager {
    /// Create a new manager for the given database.
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            write_txn: None,
            durability: Durability::Immediate,
        }
    }

    /// Start a read transaction.
    ///
    /// If there is an open write transaction, it is committed first to ensure
    /// that the read sees all pending changes.
    pub fn begin_read(&mut self) -> Result<ReadTransaction> {
        if let Some(txn) = self.write_txn.take() {
            txn.commit()?;
        }
        Ok(self.db.begin_read()?)
    }

    /// Get a mutable reference to the long-lived write transaction.
    ///
    /// The write transaction is created lazily on first use and then reused
    /// for subsequent writes until a read is requested or this manager is
    /// dropped.
    pub fn begin_write(&mut self) -> Result<&mut WriteTransaction> {
        if self.write_txn.is_none() {
            let mut txn = self.db.begin_write()?;
            txn.set_durability(self.durability)
                .map_err(|e| crate::wordnet::LexiconError::GenericError(e.to_string()))?;
            self.write_txn = Some(txn);
        }
        Ok(self.write_txn.as_mut().unwrap())
    }
}

impl TransactionManager {
    /// While on, commits are not flushed to disk. A bulk load that reads between its writes
    /// commits (and would otherwise sync) once per read, which is far too slow on a real disk.
    /// Turning it off again makes everything written so far durable with one synced commit. A
    /// crash in between can leave the database corrupt, so only use this on a file that is
    /// rebuilt from scratch if the load does not complete.
    pub fn set_bulk_load(&mut self, on: bool) -> Result<()> {
        if let Some(txn) = self.write_txn.take() {
            txn.commit()?;
        }
        if on {
            self.durability = Durability::None;
        } else {
            self.durability = Durability::Immediate;
            // A commit with `Immediate` durability makes the earlier non-durable ones durable
            self.db.begin_write()?.commit()?;
        }
        Ok(())
    }
}

impl Drop for TransactionManager {
    fn drop(&mut self) {
        if let Some(txn) = self.write_txn.take() {
            if let Err(e) = txn.commit() {
                eprintln!("Error committing write transaction on drop: {e}");
            }
        }
    }
}

