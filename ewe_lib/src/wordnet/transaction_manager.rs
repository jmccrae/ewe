use std::collections::HashMap;
use std::sync::Arc;

use redb::{Database, Durability, ReadTransaction, WriteTransaction, ReadableDatabase};

use crate::wordnet::{Entry, PosKey, Result, Synset};

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
    /// Recently read entries, synsets and lexfile names, if enabled (see [`ReadCache`])
    cache: Option<ReadCache>,
}

/// A bounded cache of what was read from the database, for a read-only pass over a big one (an
/// export) where the same hub synsets and heavy lemmas are read again and again by relations.
/// It is emptied whenever a write transaction starts, so it can never be stale. When it is
/// full it is emptied too, which keeps it simple and still catches the repeated reads.
pub struct ReadCache {
    entries: HashMap<(char, String), Arc<Vec<(PosKey, Entry)>>>,
    entries_weight: usize,
    synsets: HashMap<(String, String), Synset>,
    lexfiles: HashMap<String, String>,
}

impl ReadCache {
    const MAX_ENTRIES_WEIGHT: usize = 300_000;
    const MAX_SYNSETS: usize = 100_000;
    const MAX_LEXFILES: usize = 500_000;

    fn new() -> Self {
        ReadCache {
            entries: HashMap::new(),
            entries_weight: 0,
            synsets: HashMap::new(),
            lexfiles: HashMap::new(),
        }
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.entries_weight = 0;
        self.synsets.clear();
        self.lexfiles.clear();
    }
}

impl TransactionManager {
    /// Create a new manager for the given database.
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            write_txn: None,
            durability: Durability::Immediate,
            cache: None,
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
            if let Some(cache) = self.cache.as_mut() {
                cache.clear();
            }
            let mut txn = self.db.begin_write()?;
            txn.set_durability(self.durability)
                .map_err(|e| crate::wordnet::LexiconError::GenericError(e.to_string()))?;
            self.write_txn = Some(txn);
        }
        Ok(self.write_txn.as_mut().unwrap())
    }
}

impl TransactionManager {
    /// Turn the read cache on (see [`ReadCache`]). Meant for a pass that only reads.
    pub fn enable_read_cache(&mut self) {
        self.cache = Some(ReadCache::new());
    }

    pub fn cached_entry(&self, key: char, lemma: &str) -> Option<Arc<Vec<(PosKey, Entry)>>> {
        self.cache.as_ref()?.entries.get(&(key, lemma.to_string())).cloned()
    }

    pub fn cache_entry(&mut self, key: char, lemma: &str, value: &Arc<Vec<(PosKey, Entry)>>) {
        let Some(cache) = self.cache.as_mut() else { return };
        let weight = 1 + value.iter().map(|(_, e)| e.sense.len() + 1).sum::<usize>();
        if cache.entries_weight + weight > ReadCache::MAX_ENTRIES_WEIGHT {
            cache.entries.clear();
            cache.entries_weight = 0;
        }
        cache.entries_weight += weight;
        cache.entries.insert((key, lemma.to_string()), Arc::clone(value));
    }

    pub fn cached_synset(&self, lexname: &str, id: &str) -> Option<Synset> {
        self.cache.as_ref()?.synsets.get(&(lexname.to_string(), id.to_string())).cloned()
    }

    pub fn cache_synset(&mut self, lexname: &str, id: &str, synset: &Synset) {
        let Some(cache) = self.cache.as_mut() else { return };
        if cache.synsets.len() >= ReadCache::MAX_SYNSETS {
            cache.synsets.clear();
        }
        cache.synsets.insert((lexname.to_string(), id.to_string()), synset.clone());
    }

    pub fn cached_lexfile(&self, id: &str) -> Option<String> {
        self.cache.as_ref()?.lexfiles.get(id).cloned()
    }

    pub fn cache_lexfile(&mut self, id: &str, lexfile: &str) {
        let Some(cache) = self.cache.as_mut() else { return };
        if cache.lexfiles.len() >= ReadCache::MAX_LEXFILES {
            cache.lexfiles.clear();
        }
        cache.lexfiles.insert(id.to_string(), lexfile.to_string());
    }

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

