extern crate indicatif;
extern crate lazy_static;
extern crate regex;
extern crate serde_yaml;

mod commands;
mod common;
mod indicatif_progress;

use clap::{Parser, Subcommand};
use commands::export::ExportFormat;
use commands::import::ImportFormat;
use ewe_lib::wordnet::rdf::RdfExportOptions;
use ewe_lib::wordnet::LexiconMetadata;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name="ewe",
    version="0.2.0",
    about="EWE Wordnet Editor",
    long_about=None)]
struct EweCli {
    #[command(subcommand)]
    command: Option<Command>,
    /// Optional path to the WordNet database, available in all modes
    #[arg(long, global = true, value_name = "PATH")]
    wordnet: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run an automaton script
    Automaton {
        /// The path to the script file to execute
        script: String,
    },
    /// Search for a specific word
    Word {
        /// The word you want to search for
        word: String,

        /// Perform a case-insensitive search
        #[arg(short, long)]
        ignore_case: bool,

        /// Show the sense ID for each sense
        #[arg(short, long)]
        sense_ids: bool,
    },
    /// Search for an item by its unique ID
    Id {
        /// The numeric or textual ID to look up
        id: String,
    },
    /// Export the wordnet to another format
    Export {
        #[command(subcommand)]
        format: ExportFormat,
    },
    /// Import a wordnet from another format
    Import {
        #[command(subcommand)]
        format: ImportFormat,
    },
    /// Create a new, empty wordnet project, prompting for its key metadata
    Init {
        /// Directory to create the new project in (created if it doesn't exist)
        #[arg(default_value = "./")]
        path: PathBuf,
    },
}

fn main() {
    let cli = EweCli::parse();
    match &cli.command {
        Some(Command::Automaton { ref script }) => {
            commands::automaton::run(script, cli.wordnet);
        }
        Some(Command::Id { ref id }) => {
            commands::id::run(id, cli.wordnet);
        }
        Some(Command::Word {
            ref word,
            ignore_case,
            sense_ids,
        }) => {
            commands::word::run(word, *ignore_case, *sense_ids, cli.wordnet);
        }
        Some(Command::Export {
            format:
                ExportFormat::Xml {
                    ref path,
                    id_prefix,
                    label,
                    language,
                    email,
                    license,
                    version,
                    url,
                },
        }) => {
            let metadata = LexiconMetadata {
                id_prefix: id_prefix.clone(),
                label: label.clone(),
                language: language.clone(),
                email: email.clone(),
                license: license.clone(),
                version: version.clone(),
                url: url.clone(),
            };
            commands::export::run_xml(path, metadata, cli.wordnet);
        }
        Some(Command::Export {
            format:
                ExportFormat::Wndb {
                    ref path,
                    license_file,
                },
        }) => {
            commands::export::run_wndb(path, license_file.clone(), cli.wordnet);
        }
        Some(Command::Export {
            format:
                ExportFormat::Rdf {
                    ref path,
                    format,
                    site,
                    label,
                    language,
                    email,
                    license,
                    version,
                    url,
                },
        }) => {
            let options = RdfExportOptions {
                format: format.clone().into(),
                site: site.clone(),
                metadata: LexiconMetadata {
                    // Unused by the RDF export (its URIs are built from bare lemma/synset
                    // ids, not a `Lexicon/@id` prefix) - not exposed as a CLI flag.
                    id_prefix: String::new(),
                    label: label.clone(),
                    language: language.clone(),
                    email: email.clone(),
                    license: license.clone(),
                    version: version.clone(),
                    url: url.clone().or_else(|| Some(site.clone())),
                },
            };
            commands::export::run_rdf(path, options, cli.wordnet);
        }
        Some(Command::Import {
            format: ImportFormat::Xml { ref path },
        }) => {
            commands::import::run_xml(path, cli.wordnet);
        }
        Some(Command::Init { ref path }) => {
            commands::init::run(path);
        }
        None => {
            commands::tui::run();
        }
    }
}
