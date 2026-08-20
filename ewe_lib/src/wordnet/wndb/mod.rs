//! WNDB export - the classic Princeton WordNet database file set (`data.*`/`index.*`/
//! `index.sense`/`*.exc`), a faithful port of `gwn-scala-api`'s `WNDB.write` (see [`writer`]'s
//! doc comment). Write-only: WNDB is a legacy release target `ewe` produces, never a source of
//! truth it reads back in.

use crate::wordnet::SynsetId;
use std::path::PathBuf;
use thiserror::Error;

mod sense_orders;
mod tables;
mod writer;

pub use writer::write_wndb;

/// `WNDB.write`'s options that actually matter for a from-scratch export - no
/// `id`/`label`/`email`/etc: none of that lexicon metadata is ever read by the write path (only
/// by `WNDB.read`, which `ewe` doesn't implement).
#[derive(Debug, Clone, Default)]
pub struct WndbExportOptions {
    /// Prepended verbatim to every `data.*`/`index.*` file (never to `.exc`/`index.sense`) -
    /// matches `usePrincetonHeader=false, licenseFile=Some(f)` when given. `None` means no header
    /// at all - `ewe` never hardcodes Princeton's own fallback header text (see the module doc
    /// comment on why: OEWN's real release always supplies its own license file).
    pub license_file: Option<PathBuf>,
    /// `lemma,pos,synset_id1 synset_id2 ...` CSV recording sense order for lemmas that fold
    /// together case-insensitively in `index.{pos}` (see `sense_orders`'s doc comment).
    pub sense_orders: Option<PathBuf>,
}

#[derive(Error, Debug)]
pub enum WndbExportError {
    #[error("Could not write WNDB: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not write WNDB: {0}")]
    Lexicon(#[from] crate::wordnet::LexiconError),
    #[error("Could not write WNDB: {0}")]
    Csv(#[from] csv::Error),
    #[error("Could not write WNDB: synset {0} referenced but not found in the lexicon")]
    MissingSynset(SynsetId),
    #[error("Could not write WNDB: irregular form {0:?} contains a space, which WNDB's .exc format cannot represent")]
    FormWithSpace(String),
}
