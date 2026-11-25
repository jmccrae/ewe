//! The on-disk database behind `--low-memory`: the YAML source is streamed into a `ReDBLexicon`
//! once, reused for as long as it is newer than every source file (the rule `ewe_dioxus` uses),
//! and read through a page cache of bounded size, so memory use does not grow with the wordnet.

use crate::common::LowMemory;
use crate::indicatif_progress::IndicatifProgress;
use ewe_lib::wordnet::{Lexicon, ReDBLexicon};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

/// The directory databases are kept in unless `--db` says otherwise
fn cache_dir() -> PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(std::env::temp_dir);
    base.join("ewe")
}

/// The default database for a YAML source: one per source directory
fn default_db(source: &str) -> PathBuf {
    let canonical = std::fs::canonicalize(source).unwrap_or_else(|_| PathBuf::from(source));
    let mut hasher = DefaultHasher::new();
    canonical.hash(&mut hasher);
    cache_dir().join(format!("lexicon-{:016x}.redb", hasher.finish()))
}

/// Whether `database` is missing or older than any file of `source`
fn is_stale(database: &Path, source: &str) -> Result<bool, String> {
    let db_mtime = match database.metadata().and_then(|m| m.modified()) {
        Ok(mtime) => mtime,
        Err(_) => return Ok(true),
    };
    let src_mtime = ewe_lib::source_mtime::latest_source_mtime(source)
        .map_err(|e| format!("Could not read {}: {}", source, e))?;
    Ok(src_mtime > db_mtime)
}

/// Open the database for `source`, (re)building it from the YAML first if it is missing, stale
/// or unusable (e.g. from an older version).
pub(crate) fn open(source: &str, options: &LowMemory) -> Result<ReDBLexicon, String> {
    let database = options.db.clone().unwrap_or_else(|| default_db(source));
    if let Some(dir) = database.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("Could not create {}: {}", dir.display(), e))?;
    }
    let cache = options.cache_mb * 1024 * 1024;
    if !is_stale(&database, source)? {
        match ReDBLexicon::open(&database, cache) {
            Ok(wn) if wn.n_entries().map(|n| n > 0).unwrap_or(false) => return Ok(wn),
            Ok(_) => eprintln!("Database {} is empty, rebuilding", database.display()),
            Err(e) if e.is_database_locked() => {
                return Err(format!("The database {} is in use by another process", database.display()))
            }
            Err(e) => eprintln!("Could not use database {} ({}), rebuilding", database.display(), e),
        }
    } else {
        eprintln!("Building database {} from {}", database.display(), source);
    }
    // Build under a temporary name and move it into place only when complete: the load runs
    // without syncing, so an interrupted one must not be mistaken for a finished database.
    let building = PathBuf::from(format!("{}.building", database.display()));
    let mut progress = IndicatifProgress::new();
    let wn = ReDBLexicon::create(&building, cache)
        .map_err(|e| format!("Could not create database {}: {}", building.display(), e))?;
    wn.set_bulk_load(true).map_err(|e| format!("Could not create database: {}", e))?;
    let wn = wn
        .load(source, &mut progress)
        .map_err(|e| format!("Could not load WordNet from {}: {}", source, e))?;
    wn.set_bulk_load(false).map_err(|e| format!("Could not write database: {}", e))?;
    std::fs::rename(&building, &database)
        .map_err(|e| format!("Could not move {} to {}: {}", building.display(), database.display(), e))?;
    Ok(wn)
}

/// A scratch database that is deleted when dropped
pub(crate) struct TempDb(PathBuf);

impl TempDb {
    pub(crate) fn new(options: &LowMemory) -> std::io::Result<TempDb> {
        let dir = options
            .db
            .as_ref()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .unwrap_or_else(cache_dir);
        std::fs::create_dir_all(&dir)?;
        Ok(TempDb(dir.join(format!("scratch-{}.redb", std::process::id()))))
    }

    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDb {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
