/// Opening (and, if necessary, rebuilding) the ReDB lexicon database.
use ewe_lib::progress::LoggingProgress;
use ewe_lib::wordnet::{Lexicon, ReDBLexicon};
use std::path::Path;
use std::time::SystemTime;
use teanga::disk_corpus::RedbDb;
use teanga::{Corpus, DiskCorpus};

use crate::backend::senses::key_layer_name;
use crate::settings::EweSettings;

/// Open the lexicon database at `settings.database`. If it doesn't exist yet, or (unless
/// `settings.disable_auto_reload` is set) any file in `settings.wordnet_source` has been
/// modified more recently than the database, the database is rebuilt from source first.
pub fn open_lexicon(settings: &EweSettings) -> Result<ReDBLexicon, Box<dyn std::error::Error>> {
    let cache_size_bytes = settings.lexicon_cache_mb * 1024 * 1024;
    if let Some(source) = &settings.wordnet_source {
        if is_stale(&settings.database, source, settings.disable_auto_reload)? {
            eprintln!(
                "Wordnet source at {} is newer than {}, rebuilding database",
                source, settings.database
            );
            let lexicon = ReDBLexicon::create(&settings.database, cache_size_bytes)?;
            return Ok(lexicon.load(source, &mut LoggingProgress::new())?);
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
    let mut latest = SystemTime::UNIX_EPOCH;
    latest_mtime_recursive(Path::new(source), &mut latest)?;
    let dep_file = Path::new(source).join("../deprecations.csv");
    if let Ok(mtime) = dep_file.metadata().and_then(|m| m.modified()) {
        latest = latest.max(mtime);
    }
    Ok(latest)
}

/// Recurse into `dir` and its subdirectories, updating `latest` with the newest
/// modification time found among all files.
fn latest_mtime_recursive(dir: &Path, latest: &mut SystemTime) -> Result<(), Box<dyn std::error::Error>> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            latest_mtime_recursive(&path, latest)?;
        } else {
            let mtime = entry.metadata()?.modified()?;
            *latest = (*latest).max(mtime);
        }
    }
    Ok(())
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
