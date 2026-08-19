//! WN-LMF XML import/export - see <https://globalwordnet.github.io/schemas/>.
//!
//! Targets strict `WN-LMF-1.4.dtd`: the newest published GWA schema version, and the first to
//! define relation types (e.g. `undergoer`) that the internal model already carries. A
//! whole-lexicon export is always self-contained (every relation target is included in the same
//! document), so strict `IDREF` typing is achievable.

use thiserror::Error;

pub mod ids;
pub mod reader;
pub mod writer;
pub use reader::read_lexicon_xml;
pub use writer::{write_lexicon_xml, write_lexicon_xml_subset};

pub const WN_LMF_DOCTYPE: &str =
    "LexicalResource SYSTEM \"http://globalwordnet.github.io/schemas/WN-LMF-1.4.dtd\"";

/// Lexicon-level metadata written to/read from the `Lexicon` element's attributes. Standalone
/// from any particular host application's settings (unlike the `EweSettings` this replaces in
/// `ewe_dioxus`'s exporter), since `ewe_lib` has no notion of a running app's configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexiconMetadata {
    pub id_prefix: String,
    pub label: String,
    pub language: String,
    pub email: Option<String>,
    pub license: String,
    pub version: String,
    pub url: Option<String>,
}

#[derive(Error, Debug)]
pub enum XmlExportError {
    #[error("Could not write WN-LMF XML: {0}")]
    Xml(#[from] quick_xml::Error),
    #[error("Could not write WN-LMF XML: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not write WN-LMF XML: {0}")]
    Lexicon(#[from] crate::wordnet::LexiconError),
}

#[derive(Error, Debug)]
pub enum XmlImportError {
    #[error("Could not read WN-LMF XML: {0}")]
    Xml(#[from] quick_xml::Error),
    #[error("Could not read WN-LMF XML: {0}")]
    Attr(#[from] quick_xml::events::attributes::AttrError),
    #[error("Could not read WN-LMF XML: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not read WN-LMF XML: {0}")]
    Lexicon(#[from] crate::wordnet::LexiconError),
    #[error("Could not read WN-LMF XML: {0}")]
    Load(#[from] crate::wordnet::WordNetYAMLIOError),
    #[error("Malformed WN-LMF document: {0}")]
    Malformed(String),
}
