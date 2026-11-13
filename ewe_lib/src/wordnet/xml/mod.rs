//! WN-LMF XML import/export - see <https://globalwordnet.github.io/schemas/>.
//!
//! Targets strict `WN-LMF-1.4.dtd` by default: the newest published GWA schema version, and the
//! first to define relation types (e.g. `undergoer`) that the internal model already carries. A
//! whole-lexicon export is always self-contained (every relation target is included in the same
//! document), so strict `IDREF` typing is achievable.

use thiserror::Error;

pub mod ids;
pub mod reader;
pub mod writer;
pub use reader::read_lexicon_xml;
pub use writer::{
    write_lexicon_xml, write_lexicon_xml_report, write_lexicon_xml_subset,
    write_lexicon_xml_subset_report,
};

/// The WN-LMF schema version an export targets. Data the chosen version cannot represent is
/// dropped and reported in [`XmlOutput::warnings`] rather than written as invalid XML.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LmfVersion {
    /// The newest published version.
    #[default]
    V1_4,
    /// Unreleased at the time of writing; adds `Lemma/@reconstructed`.
    V1_5,
}

impl LmfVersion {
    pub fn doctype(&self) -> String {
        format!(
            "LexicalResource SYSTEM \"http://globalwordnet.github.io/schemas/WN-LMF-{}.dtd\"",
            self.as_str()
        )
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            LmfVersion::V1_4 => "1.4",
            LmfVersion::V1_5 => "1.5",
        }
    }

    /// The `partOfSpeech` code to write in this version: WN-LMF 1.4 has no code for the parts
    /// of speech added in 1.5, so those fall back to the closest older one.
    pub fn pos_code(&self, pos: crate::wordnet::PartOfSpeech) -> &'static str {
        if *self >= LmfVersion::V1_5 { pos.value() } else { pos.lmf_1_4_code() }
    }

    /// Whether `Lemma/@reconstructed` exists in this version.
    pub fn supports_reconstructed(&self) -> bool {
        *self >= LmfVersion::V1_5
    }
}

impl PartialOrd for LmfVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> { Some(self.cmp(other)) }
}
impl Ord for LmfVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering { (*self as u8).cmp(&(*other as u8)) }
}

impl std::str::FromStr for LmfVersion {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, String> {
        match s.trim() {
            "1.4" => Ok(LmfVersion::V1_4),
            "1.5" => Ok(LmfVersion::V1_5),
            other => Err(format!("unknown WN-LMF version {other:?} (expected 1.4 or 1.5)")),
        }
    }
}

/// An exported document plus a description of any data it could not represent.
#[derive(Debug, Clone, PartialEq)]
pub struct XmlOutput {
    pub bytes: Vec<u8>,
    /// Human-readable data-loss warnings; empty when the export was lossless.
    pub warnings: Vec<String>,
}

/// Lexicon-level metadata written to/read from the `Lexicon` element's attributes. Standalone
/// from any particular host application's settings (unlike the `EweSettings` this replaces in
/// `ewe_dioxus`'s exporter), since `ewe_lib` has no notion of a running app's configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct LexiconMetadata {
    pub id_prefix: String,
    pub label: String,
    pub language: String,
    pub email: Option<String>,
    pub license: String,
    pub version: String,
    pub url: Option<String>,
    /// `Lexicon/@confidenceScore`. `None` leaves the attribute out (the DTD defaults it to 1.0).
    pub confidence: Option<f64>,
    /// The schema version to write. See [`LmfVersion`].
    pub lmf_version: LmfVersion,
}

#[derive(Error, Debug)]
pub enum XmlExportError {
    #[error("Could not write WN-LMF XML: {0}")]
    Xml(#[from] quick_xml::Error),
    #[error("Could not write WN-LMF XML: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not write WN-LMF XML: {0}")]
    Lexicon(#[from] crate::wordnet::LexiconError),
    #[error(
        "Could not write WN-LMF XML: synset {synset} has no ILI and no ili_definition, which a \
         lexicon in language {language:?} needs (set `ili_definition`, or set `ili` to an \
         empty string if it deliberately has neither)"
    )]
    MissingIliDefinition { synset: String, language: String },
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
