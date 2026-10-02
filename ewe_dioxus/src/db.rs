/// Opening (and, if necessary, rebuilding) the ReDB lexicon database.
use ewe_lib::progress::{LoggingProgress, Progress};
use ewe_lib::wordnet::{Lexicon, ReDBLexicon};
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
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
            for path in corpus_source_files(source)? {
                let file = std::fs::File::open(&path)?;
                teanga::read_yaml(file, &mut corpus)?;
            }
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

/// The corpus YAML files making up `corpus_source`: the file itself, or, if it is a
/// directory, every `.yaml` file directly inside it, in name order. A directory lets a large
/// corpus be split into several files (e.g. one per source), all sharing the same `_meta`.
fn corpus_source_files(source: &str) -> std::io::Result<Vec<PathBuf>> {
    let path = Path::new(source);
    if !path.is_dir() {
        return Ok(vec![path.to_path_buf()]);
    }
    let mut files = std::fs::read_dir(path)?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?
        .into_iter()
        .filter(|p| p.is_file() && p.extension().is_some_and(|ext| ext == "yaml"))
        .collect::<Vec<_>>();
    files.sort();
    Ok(files)
}

/// True if the database at `database` doesn't exist, or if `disable_auto_reload` is unset
/// and `source` (or, for a directory, the directory itself or any corpus file in it) is
/// newer than it.
fn is_file_stale(database: &str, source: &str, disable_auto_reload: bool) -> Result<bool, Box<dyn std::error::Error>> {
    let db_mtime = match Path::new(database).metadata().and_then(|m| m.modified()) {
        Ok(mtime) => mtime,
        Err(_) => return Ok(true),
    };
    if disable_auto_reload {
        return Ok(false);
    }
    // The directory's own mtime changes when a file is added or removed.
    let mut latest = Path::new(source).metadata()?.modified()?;
    for path in corpus_source_files(source)? {
        latest = latest.max(path.metadata()?.modified()?);
    }
    Ok(latest > db_mtime)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A directory under the system temp dir, unique per test run, cleaned up on drop.
    struct ScratchDir(PathBuf);

    impl ScratchDir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "ewe-db-test-{name}-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(&dir).unwrap();
            ScratchDir(dir)
        }
    }

    impl Drop for ScratchDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    const META: &str = "_meta:
    text:
        type: characters
    tokens:
        type: span
        base: text
    test_key:
        type: element
        base: tokens
        data: string
";

    #[test]
    fn test_corpus_source_directory() {
        let scratch = ScratchDir::new("corpus-dir");
        let source = scratch.0.join("corpus");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("a.yaml"), format!("{META}ecWc:
    text: This is an example
    tokens: [[0, 4], [5, 7], [8, 10], [11, 18]]
    test_key: [[3, \"test-00001740-n\"]]
")).unwrap();
        std::fs::write(source.join("b.yaml"), format!("{META}Kjco:
    text: This is a document.
    tokens: [[0, 4], [5, 7], [8, 9], [10, 18], [18, 19]]
    test_key: [[3, \"test-00001740-n\"]]
")).unwrap();
        std::fs::write(source.join("notes.txt"), "not a corpus file").unwrap();

        let source = source.to_string_lossy().to_string();
        assert_eq!(corpus_source_files(&source).unwrap().len(), 2);

        let mut settings = EweSettings::default();
        settings.id_prefix = "test".to_string();
        settings.corpus_source = Some(source.clone());
        settings.corpus_database = scratch.0.join("corpus.db").to_string_lossy().to_string();
        let corpus = open_corpus(&settings).unwrap();
        let mut ids = corpus.get_order().to_vec();
        ids.sort();
        assert_eq!(ids, vec!["Kjco".to_string(), "ecWc".to_string()]);
        drop(corpus);

        assert!(!is_file_stale(&settings.corpus_database, &source, false).unwrap());
    }
}
