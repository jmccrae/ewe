//! `ewe import` - imports a wordnet from another format, saving it as a YAML project.

use crate::indicatif_progress::IndicatifProgress;
use clap::Subcommand;
use ewe_lib::wordnet::xml::read_lexicon_xml;
use ewe_lib::wordnet::{Lexicon, LexiconHashMapBackend, LexiconMetadata};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::exit;

#[derive(Subcommand, Debug)]
pub(crate) enum ImportFormat {
    /// Import from a WN-LMF XML document (https://globalwordnet.github.io/schemas/), saving the
    /// result as a YAML source tree
    Xml {
        /// Path to the XML document to import
        path: PathBuf,
    },
}

/// Escapes a value for a TOML basic string (`"..."`).
fn toml_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// A minimal `settings.toml` for the freshly-imported project, in the same format
/// `ewe_dioxus/english-wordnet-settings.toml` documents (see that crate's README) - populated
/// from the imported `Lexicon` element's own metadata rather than OEWN's specific branding,
/// which wouldn't be correct for an arbitrary imported GWA wordnet.
fn generate_settings_toml(metadata: &LexiconMetadata) -> String {
    let mut out = String::new();
    out.push_str("database = \"wordnet.db\"\n");
    out.push_str("wordnet_source = \"src/yaml/\"\n");
    out.push_str(&format!("id_prefix = \"{}\"\n", toml_escape(&metadata.id_prefix)));
    if !metadata.label.is_empty() {
        out.push_str(&format!("project_name = \"{}\"\n", toml_escape(&metadata.label)));
    }
    if let Some(email) = &metadata.email {
        out.push_str(&format!("contact_email = \"{}\"\n", toml_escape(email)));
    }
    if let Some(url) = &metadata.url {
        out.push_str(&format!("source_url = \"{}\"\n", toml_escape(url)));
    }
    out
}

/// Writes `wn`/`metadata` out as a full project directory, mirroring the real OEWN layout:
/// entries-*.yaml/lexfile.yaml/frames.yaml under `src/yaml/`, `deprecations.csv` as its sibling
/// (`Lexicon::save` already writes that one via `src/yaml/../deprecations.csv`), and a
/// `settings.toml` an `ewe_dioxus` deployment can point straight at. Shared by `ewe import xml`
/// and `ewe init`, so both produce the same project shape.
pub(crate) fn write_project_structure<L: Lexicon>(
    wn: &L,
    metadata: &LexiconMetadata,
    out_dir: &Path,
) {
    let yaml_dir = out_dir.join("src").join("yaml");
    std::fs::create_dir_all(&yaml_dir).unwrap_or_else(|e| {
        eprintln!("Could not create {}: {}", yaml_dir.display(), e);
        exit(-1);
    });
    let mut progress = IndicatifProgress::new();
    wn.save(&yaml_dir, &mut progress).unwrap_or_else(|e| {
        eprintln!("Could not save to {}: {}", yaml_dir.display(), e);
        exit(-1);
    });
    let settings_path = out_dir.join("settings.toml");
    std::fs::write(&settings_path, generate_settings_toml(metadata)).unwrap_or_else(|e| {
        eprintln!("Could not write {}: {}", settings_path.display(), e);
        exit(-1);
    });
}

pub(crate) fn run_xml(path: &Path, out_dir: Option<PathBuf>) {
    let out_dir = out_dir.unwrap_or_else(|| PathBuf::from("./"));
    let file = File::open(path).unwrap_or_else(|e| {
        eprintln!("Could not open {}: {}", path.display(), e);
        exit(-1);
    });
    let (wn, metadata) = read_lexicon_xml(LexiconHashMapBackend::new(), file).unwrap_or_else(|e| {
        eprintln!("Could not import {}: {}", path.display(), e);
        exit(-1);
    });
    println!(
        "Imported {} entries, {} synsets from lexicon {:?} ({})",
        wn.n_entries().expect("Cannot read imported lexicon"),
        wn.n_synsets().expect("Cannot read imported lexicon"),
        metadata.label,
        metadata.id_prefix
    );

    write_project_structure(&wn, &metadata, &out_dir);
    println!("Saved project to {}", out_dir.display());
}
