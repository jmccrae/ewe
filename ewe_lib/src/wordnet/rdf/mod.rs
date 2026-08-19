//! RDF export - see <https://globalwordnet.github.io/schemas/> for the `wn:` vocabulary this
//! targets, on top of `ontolex`/`lime`/`skos`/`dc`.
//!
//! [`write_lexicon_rdf`] produces a whole-lexicon export: every `LexicalEntry` is declared
//! exactly once regardless of how many senses/synsets it participates in - the old OEWN release
//! process (`wordnet-rdf-dump`, dumping one synset at a time and re-declaring every member's
//! entry triples each time) needed an external `rapper -i turtle -o turtle` pass afterwards
//! purely to dedupe those repeats; this export makes that pass unnecessary by construction.
//! [`write_lexicon_rdf_subset`] is the lower-level building block it's implemented on top of,
//! also exposed directly for callers (e.g. `ewe_dioxus`'s per-synset/per-lemma web routes) that
//! intentionally export less than the whole lexicon - it applies the same entry-dedup within
//! whatever synsets it's given, so a multi-sense lemma still gets one `LexicalEntry` even when
//! all its senses are passed in together (e.g. the per-lemma route).

use thiserror::Error;

pub mod writer;
pub use writer::{write_lexicon_rdf, write_lexicon_rdf_subset};

pub use oxrdfio::RdfFormat;

use super::xml::LexiconMetadata;

/// Options controlling a whole-lexicon or subset RDF export. Reuses
/// `crate::wordnet::xml::LexiconMetadata` for the `lime:Lexicon` header's general
/// metadata (`label`/`language`/`email`/`license`/`version`/`url` - see
/// <https://globalwordnet.github.io/schemas/#rdf> for the exact properties each maps to)
/// rather than duplicating those fields - `id_prefix` is carried but unused here, since RDF
/// URIs are built from bare lemma/synset ids, not a `Lexicon/@id` prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RdfExportOptions {
    pub format: RdfFormat,
    /// Base URI resources (`{site}synset/...`, `{site}lemma/...`) are built under, and the
    /// subject of the whole-lexicon export's `lime:Lexicon` header.
    pub site: String,
    pub metadata: LexiconMetadata,
}

#[derive(Error, Debug)]
pub enum RdfExportError {
    #[error("Could not write RDF: {0}")]
    Iri(#[from] oxrdf::IriParseError),
    #[error("Could not write RDF: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not write RDF: {0}")]
    Lexicon(#[from] crate::wordnet::LexiconError),
}
