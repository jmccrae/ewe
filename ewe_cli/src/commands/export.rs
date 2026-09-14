//! `ewe export` - writes the wordnet out as XML, WNDB, or RDF.

use crate::common::locate_wordnet;
use clap::Subcommand;
use ewe_lib::wordnet::rdf::{write_lexicon_rdf, RdfExportOptions, RdfFormat};
use ewe_lib::wordnet::xml::write_lexicon_xml;
use ewe_lib::wordnet::{write_wndb, LexiconMetadata, WndbExportOptions};
use std::path::{Path, PathBuf};
use std::process::exit;

#[derive(Subcommand, Debug)]
pub(crate) enum ExportFormat {
    /// Export as a whole-lexicon, self-contained WN-LMF XML document
    /// (https://globalwordnet.github.io/schemas/)
    Xml {
        /// Path to write the XML document to
        path: PathBuf,

        /// The `Lexicon/@id` prefix used to build every element id in the document
        #[arg(long, default_value = "oewn")]
        id_prefix: String,
        /// The `Lexicon/@label`
        #[arg(long, default_value = "Open English Wordnet")]
        label: String,
        /// The `Lexicon/@language` (BCP 47 code)
        #[arg(long, default_value = "en")]
        language: String,
        /// The `Lexicon/@email` contact address
        #[arg(long)]
        email: Option<String>,
        /// The `Lexicon/@license` URL
        #[arg(long, default_value = "https://creativecommons.org/licenses/by/4.0")]
        license: String,
        /// The `Lexicon/@version`
        #[arg(long, default_value = "1")]
        version: String,
        /// The `Lexicon/@url` project homepage
        #[arg(long)]
        url: Option<String>,
    },
    /// Export as the classic WNDB (Princeton WordNet database) file set
    /// (data.*/index.*/index.sense/*.exc)
    Wndb {
        /// Directory to write the WNDB files to (created if it doesn't exist)
        path: PathBuf,

        /// A license/header file, prepended verbatim to every `data.*`/`index.*` file. Defaults
        /// to `WNDB_License.txt` at the WordNet's root if that file exists (see
        /// https://github.com/globalwordnet/english-wordnet's layout); pass this to override it,
        /// or point at a file explicitly when there's no such default to find.
        #[arg(long)]
        license_file: Option<PathBuf>,
    },
    /// Export as a whole-lexicon RDF document (Turtle or RDF/XML), suitable for an
    /// `en-word.net`-style RDF release - every `LexicalEntry` is declared exactly once,
    /// regardless of how many senses/synsets it appears in, so no external dedup pass
    /// (the old release process's `rapper -i turtle -o turtle`) is needed
    Rdf {
        /// Path to write the RDF document to
        path: PathBuf,

        /// The RDF serialization to write
        #[arg(long, value_enum, default_value_t = RdfSyntax::Turtle)]
        format: RdfSyntax,
        /// Base URI resources (`{site}synset/...`, `{site}lemma/...`) are built under, and the
        /// subject of the export's `lime:Lexicon` header
        #[arg(long, default_value = "https://en-word.net/")]
        site: String,
        /// The `lime:Lexicon` header's `rdfs:label`
        #[arg(long, default_value = "Open English Wordnet")]
        label: String,
        /// BCP 47 language tag applied to lemma/definition/example literals and the header's
        /// `dc:language`
        #[arg(long, default_value = "en")]
        language: String,
        /// The header's `schema:email` contact address
        #[arg(long)]
        email: Option<String>,
        /// License URL asserted as the header's `cc:license`
        #[arg(long, default_value = "https://creativecommons.org/licenses/by/4.0/")]
        license: String,
        /// The header's `owl:versionInfo`
        #[arg(long, default_value = "1")]
        version: String,
        /// The header's `schema:url` project homepage (defaults to `--site` if unset)
        #[arg(long)]
        url: Option<String>,
    },
}

/// The RDF serialization to write - maps directly onto `ewe_lib::wordnet::rdf::RdfFormat`
/// (`oxrdfio::RdfFormat`), just with kebab-case CLI-friendly variant names.
#[derive(clap::ValueEnum, Clone, Debug)]
pub(crate) enum RdfSyntax {
    Turtle,
    #[value(name = "rdf-xml")]
    RdfXml,
}

impl std::fmt::Display for RdfSyntax {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RdfSyntax::Turtle => write!(f, "turtle"),
            RdfSyntax::RdfXml => write!(f, "rdf-xml"),
        }
    }
}

impl From<RdfSyntax> for RdfFormat {
    fn from(syntax: RdfSyntax) -> RdfFormat {
        match syntax {
            RdfSyntax::Turtle => RdfFormat::Turtle,
            RdfSyntax::RdfXml => RdfFormat::RdfXml,
        }
    }
}

/// The WordNet root - `WNDB_License.txt` and `src/sense-orders.csv` (see
/// [`default_license_file`]/[`default_sense_orders`]) both live relative to this, not to
/// [`locate_wordnet`]'s resolved yaml directory (`{root}/src/yaml/`). `locate_wordnet` accepts
/// either the root or the yaml directory itself, so this is only exact for the (overwhelmingly
/// common) former case - pointing `--wordnet` straight at a bare yaml directory just means
/// neither file is found, same as if they didn't exist.
fn wordnet_root(wordnet: &Option<PathBuf>) -> PathBuf {
    wordnet.clone().unwrap_or_else(|| PathBuf::from("."))
}

/// Auto-detects `WNDB_License.txt` at the WordNet's root (issue #41) - the layout
/// https://github.com/globalwordnet/english-wordnet ships, and the real OEWN release's source of
/// this file. Only consulted when `--license-file` isn't given explicitly.
fn default_license_file(wordnet: &Option<PathBuf>) -> Option<PathBuf> {
    let candidate = wordnet_root(wordnet).join("WNDB_License.txt");
    candidate.is_file().then_some(candidate)
}

/// Auto-detects `src/sense-orders.csv` at the WordNet's root (issue #41), if present - see
/// `WndbExportOptions::sense_orders`'s doc comment for what it's for. Not a CLI flag at all
/// (unlike `license_file`): it's derived data checked into the source layout alongside `src/yaml`,
/// not something callers should need to point at by hand.
fn default_sense_orders(wordnet: &Option<PathBuf>) -> Option<PathBuf> {
    let candidate = wordnet_root(wordnet).join("src").join("sense-orders.csv");
    candidate.is_file().then_some(candidate)
}

pub(crate) fn run_wndb(path: &Path, license_file: Option<PathBuf>, wordnet: Option<PathBuf>) {
    let license_file = license_file.or_else(|| default_license_file(&wordnet));
    let sense_orders = default_sense_orders(&wordnet);
    let (_, wn) = locate_wordnet(wordnet).unwrap_or_else(|e| {
        eprintln!("{}", e);
        exit(-1);
    });
    let options = WndbExportOptions {
        license_file,
        sense_orders,
    };
    write_wndb(&wn, path, &options).unwrap_or_else(|e| {
        eprintln!("Could not generate WNDB: {}", e);
        exit(-1);
    });
    println!("Wrote {}", path.display());
}

pub(crate) fn run_xml(path: &Path, metadata: LexiconMetadata, wordnet: Option<PathBuf>) {
    let (_, wn) = locate_wordnet(wordnet).unwrap_or_else(|e| {
        eprintln!("{}", e);
        exit(-1);
    });
    let xml = write_lexicon_xml(&wn, &metadata).unwrap_or_else(|e| {
        eprintln!("Could not generate XML: {}", e);
        exit(-1);
    });
    std::fs::write(path, xml).unwrap_or_else(|e| {
        eprintln!("Could not write {}: {}", path.display(), e);
        exit(-1);
    });
    println!("Wrote {}", path.display());
}

pub(crate) fn run_rdf(path: &Path, options: RdfExportOptions, wordnet: Option<PathBuf>) {
    let (_, wn) = locate_wordnet(wordnet).unwrap_or_else(|e| {
        eprintln!("{}", e);
        exit(-1);
    });
    let rdf = write_lexicon_rdf(&wn, &options).unwrap_or_else(|e| {
        eprintln!("Could not generate RDF: {}", e);
        exit(-1);
    });
    std::fs::write(path, rdf).unwrap_or_else(|e| {
        eprintln!("Could not write {}: {}", path.display(), e);
        exit(-1);
    });
    println!("Wrote {}", path.display());
}
