/// Opening (and, if necessary, rebuilding) the ReDB lexicon database.
use ewe_lib::progress::{LoggingProgress, Progress};
use ewe_lib::wordnet::{Lexicon, ReDBLexicon};
use std::ops::{Deref, DerefMut};
use std::path::Path;
use std::sync::{RwLockReadGuard, RwLockWriteGuard};
use std::time::SystemTime;
use teanga::disk_corpus::RedbDb;
use teanga::{Corpus, DiskCorpus};
use thiserror::Error;

use crate::backend::senses::key_layer_name;
use crate::settings::EweSettings;

#[derive(Error, Debug)]
#[error("Lexicon not available")]
pub struct LexiconUnavailable;

/// A read guard that derefs straight to `ReDBLexicon`, hiding the `Option` that
/// `crate::LEXICON` wraps it in (so a lexicon that failed to open at startup can later be
/// hot-swapped in - see the doc comment on `crate::LEXICON`). Only ever constructed once the
/// `Option` has already been confirmed `Some`, so the `unwrap` here can't fail in practice.
pub struct LexiconGuard<'a>(RwLockReadGuard<'a, Option<ReDBLexicon>>);

impl<'a> Deref for LexiconGuard<'a> {
    type Target = ReDBLexicon;
    fn deref(&self) -> &ReDBLexicon {
        self.0.as_ref().unwrap()
    }
}

/// The write-lock counterpart of [`LexiconGuard`].
pub struct LexiconGuardMut<'a>(RwLockWriteGuard<'a, Option<ReDBLexicon>>);

impl<'a> Deref for LexiconGuardMut<'a> {
    type Target = ReDBLexicon;
    fn deref(&self) -> &ReDBLexicon {
        self.0.as_ref().unwrap()
    }
}

impl<'a> DerefMut for LexiconGuardMut<'a> {
    fn deref_mut(&mut self) -> &mut ReDBLexicon {
        self.0.as_mut().unwrap()
    }
}

/// Takes a read lock on the shared lexicon, or an error if it failed to load at startup (and
/// hasn't since been configured via `backend::setup::configure_wordnet_source`).
pub fn read_lexicon() -> Result<LexiconGuard<'static>, LexiconUnavailable> {
    let guard = crate::LEXICON.get().read().unwrap();
    if guard.is_none() {
        return Err(LexiconUnavailable);
    }
    Ok(LexiconGuard(guard))
}

/// Takes a write lock on the shared lexicon, or an error if it failed to load at startup (and
/// hasn't since been configured via `backend::setup::configure_wordnet_source`).
pub fn write_lexicon() -> Result<LexiconGuardMut<'static>, LexiconUnavailable> {
    let guard = crate::LEXICON.get().write().unwrap();
    if guard.is_none() {
        return Err(LexiconUnavailable);
    }
    Ok(LexiconGuardMut(guard))
}

/// Takes a read lock on the shared settings, reflecting the most recently configured project
/// (see `backend::setup::configure_wordnet_source`) rather than necessarily what was loaded at
/// startup.
pub fn read_settings() -> RwLockReadGuard<'static, EweSettings> {
    crate::SETTINGS.get().read().unwrap()
}

/// Takes a write lock on the shared settings.
pub fn write_settings() -> RwLockWriteGuard<'static, EweSettings> {
    crate::SETTINGS.get().write().unwrap()
}

/// Takes a read lock on the shared corpus. `None` means no corpus is loaded (it's supplementary
/// - used only to show usage examples - so callers should treat that as "nothing to show", not
/// an error).
pub fn read_corpus() -> RwLockReadGuard<'static, Option<DiskCorpus<RedbDb>>> {
    crate::CORPUS.get().read().unwrap()
}

/// True if `settings` clearly has no Wordnet configured yet - no `wordnet_source` to build from,
/// and no `database` file to open either. This is the normal starting state for a freshly
/// launched, not-yet-configured desktop app (see `backend::setup`), not an unexpected failure -
/// callers use this to skip logging a scary "failed to open" error for what's actually routine.
pub fn is_unconfigured_lexicon(settings: &EweSettings) -> bool {
    settings.wordnet_source.is_none() && !Path::new(&settings.database).exists()
}

/// True if `settings` clearly has no corpus configured yet - see [`is_unconfigured_lexicon`],
/// which this mirrors (the corpus is separate from, and optional independently of, the lexicon).
pub fn is_unconfigured_corpus(settings: &EweSettings) -> bool {
    settings.corpus_source.is_none() && !Path::new(&settings.corpus_database).exists()
}

/// Open the lexicon database at `settings.database`, reporting rebuild progress to stderr via
/// [`LoggingProgress`]. See [`open_lexicon_with_progress`] for the general form (used by
/// `backend::setup::configure_wordnet_source` to report progress back to the client instead).
pub fn open_lexicon(settings: &EweSettings) -> Result<ReDBLexicon, Box<dyn std::error::Error>> {
    open_lexicon_with_progress(settings, &mut LoggingProgress::new())
}

/// Open the lexicon database at `settings.database`. If it doesn't exist yet, or (unless
/// `settings.disable_auto_reload` is set) any file in `settings.wordnet_source` has been
/// modified more recently than the database, the database is rebuilt from source first,
/// reporting progress through `progress` as it goes.
pub fn open_lexicon_with_progress<Pr: Progress>(
    settings: &EweSettings,
    progress: &mut Pr,
) -> Result<ReDBLexicon, Box<dyn std::error::Error>> {
    let cache_size_bytes = settings.lexicon_cache_mb * 1024 * 1024;
    if let Some(source) = &settings.wordnet_source {
        if is_stale(&settings.database, source, settings.disable_auto_reload)? {
            eprintln!(
                "Wordnet source at {} is newer than {}, rebuilding database",
                source, settings.database
            );
            let lexicon = ReDBLexicon::create(&settings.database, cache_size_bytes)?;
            return Ok(lexicon.load(source, progress)?);
        }
    }
    Ok(ReDBLexicon::open(&settings.database, cache_size_bytes)?)
}

/// True if the database at `database` doesn't exist, or if `disable_auto_reload` is unset
/// and any file under `source` (including the sibling `deprecations.csv`) is newer than it.
fn is_stale(database: &str, source: &str, disable_auto_reload: bool) -> Result<bool, Box<dyn std::error::Error>> {
    let db_mtime = match Path::new(database).metadata().and_then(|m| m.modified()) {
        Ok(mtime) => mtime,
        Err(_) => return Ok(true),
    };
    if disable_auto_reload {
        return Ok(false);
    }
    Ok(latest_source_mtime(source)? > db_mtime)
}

/// The most recent modification time among the source YAML files (searched recursively,
/// since large sources like NameNet split files across subdirectories) and the
/// deprecations file.
fn latest_source_mtime(source: &str) -> Result<SystemTime, Box<dyn std::error::Error>> {
    Ok(ewe_lib::source_mtime::latest_source_mtime(source)?)
}

/// Open the corpus database at `settings.corpus_database`. If it doesn't exist yet,
/// or (unless `settings.disable_auto_reload` is set) `settings.corpus_source` has been
/// modified more recently than the database, the database is rebuilt from source first.
/// Either way, a search index on the configured key layer (see [`key_layer_name`]) is
/// guaranteed to exist by the time this returns, so sense lookups don't have to scan
/// every document.
pub fn open_corpus(settings: &EweSettings) -> Result<DiskCorpus<RedbDb>, Box<dyn std::error::Error>> {
    let mut corpus = if let Some(source) = &settings.corpus_source {
        if is_file_stale(&settings.corpus_database, source, settings.disable_auto_reload)? {
            eprintln!(
                "Corpus source at {} is newer than {}, rebuilding database",
                source, settings.corpus_database
            );
            if Path::new(&settings.corpus_database).exists() {
                std::fs::remove_file(&settings.corpus_database)?;
            }
            let mut corpus = DiskCorpus::<RedbDb>::new(&settings.corpus_database)?;
            let file = std::fs::File::open(source)?;
            teanga::read_yaml(file, &mut corpus)?;
            corpus.commit()?;
            corpus
        } else {
            DiskCorpus::<RedbDb>::new(&settings.corpus_database)?
        }
    } else {
        DiskCorpus::<RedbDb>::new(&settings.corpus_database)?
    };

    // The index is persisted in the database file, so this is a no-op (just an
    // index-file lookup) on every startup after the first.
    let layer = key_layer_name(&settings.id_prefix);
    if !corpus.has_index(&layer) {
        eprintln!("Building search index on '{}' layer", layer);
        corpus.create_index(&layer)?;
        corpus.commit()?;
    }

    Ok(corpus)
}

/// True if the database at `database` doesn't exist, or if `disable_auto_reload` is unset
/// and `source` is newer than it.
fn is_file_stale(database: &str, source: &str, disable_auto_reload: bool) -> Result<bool, Box<dyn std::error::Error>> {
    let db_mtime = match Path::new(database).metadata().and_then(|m| m.modified()) {
        Ok(mtime) => mtime,
        Err(_) => return Ok(true),
    };
    if disable_auto_reload {
        return Ok(false);
    }
    Ok(Path::new(source).metadata()?.modified()? > db_mtime)
}
