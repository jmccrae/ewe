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
    /// Print summary statistics about the wordnet (synsets, entries, senses, relations), plus
    /// counts for the hypernym-hierarchy "test patterns" from Lohk, Fellbaum & Võhandu, "Tuning
    /// Hierarchies in Princeton WordNet" (GWC 2016) - self-hypernymy, shortcut, dense
    /// component and the compound pattern. Each `--<pattern>-instances` flag lists every
    /// instance of that pattern instead of just its total count.
    Stats {
        /// List every self-hypernymy instance found (a word that appears twice along a
        /// hypernym path - a synset member that is also a member of one of its own hypernym
        /// ancestors), not just the total count that's reported by default.
        #[arg(long)]
        self_hypernymy_instances: bool,

        /// List every shortcut instance found (a redundant direct hypernym edge whose target
        /// is also reachable via another of the synset's hypernym parents).
        #[arg(long)]
        shortcut_instances: bool,

        /// List every dense component instance found (synsets sharing the same two-or-more
        /// hypernym parents via multiple inheritance).
        #[arg(long)]
        dense_component_instances: bool,

        /// List every "compound" pattern instance found (a hypernym whose member word is a
        /// suffix of two or more hyponyms' members, where at least one of those hyponyms also
        /// has an unrelated extra hypernym).
        #[arg(long)]
        compound_pattern_instances: bool,
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
        Some(Command::Stats {
            self_hypernymy_instances,
            shortcut_instances,
            dense_component_instances,
            compound_pattern_instances,
        }) => {
            commands::stats::run(
                cli.wordnet,
                *self_hypernymy_instances,
                *shortcut_instances,
                *dense_component_instances,
                *compound_pattern_instances,
            );
        }
        None => {
            commands::tui::run();
        }
    }
}
