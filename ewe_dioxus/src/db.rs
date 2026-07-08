/// Opening (and, if necessary, rebuilding) the ReDB lexicon database.
use oewn_lib::progress::NullProgress;
use oewn_lib::wordnet::{Lexicon, ReDBLexicon};
use std::path::Path;
use std::time::SystemTime;

use crate::settings::EweSettings;

/// Open the lexicon database at `settings.database`. If it doesn't exist yet, or any
/// file in `settings.wordnet_source` has been modified more recently than the database,
/// the database is rebuilt from source first.
pub fn open_lexicon(settings: &EweSettings) -> Result<ReDBLexicon, Box<dyn std::error::Error>> {
    if let Some(source) = &settings.wordnet_source {
        if is_stale(&settings.database, source)? {
            eprintln!(
                "Wordnet source at {} is newer than {}, rebuilding database",
                source, settings.database
            );
            let lexicon = ReDBLexicon::create(&settings.database)?;
            return Ok(lexicon.load(source, &mut NullProgress)?);
        }
    }
    Ok(ReDBLexicon::open(&settings.database)?)
}

/// True if the database at `database` doesn't exist, or if any file under `source`
/// (including the sibling `deprecations.csv`) is newer than it.
fn is_stale(database: &str, source: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let db_mtime = match Path::new(database).metadata().and_then(|m| m.modified()) {
        Ok(mtime) => mtime,
        Err(_) => return Ok(true),
    };
    Ok(latest_source_mtime(source)? > db_mtime)
}

/// The most recent modification time among the source YAML files and the deprecations file.
fn latest_source_mtime(source: &str) -> Result<SystemTime, Box<dyn std::error::Error>> {
    let mut latest = SystemTime::UNIX_EPOCH;
    for entry in std::fs::read_dir(source)? {
        let mtime = entry?.metadata()?.modified()?;
        latest = latest.max(mtime);
    }
    let dep_file = Path::new(source).join("../deprecations.csv");
    if let Ok(mtime) = dep_file.metadata().and_then(|m| m.modified()) {
        latest = latest.max(mtime);
    }
    Ok(latest)
}
