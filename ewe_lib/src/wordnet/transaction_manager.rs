use std::sync::Arc;

use redb::{Database, ReadTransaction, WriteTransaction, ReadableDatabase};

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
}

impl TransactionManager {
    /// Create a new manager for the given database.
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            write_txn: None,
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
            self.write_txn = Some(self.db.begin_write()?);
        }
        Ok(self.write_txn.as_mut().unwrap())
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

