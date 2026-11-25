//! Helpers shared across more than one subcommand.

use crate::indicatif_progress::IndicatifProgress;
use ewe_lib::progress::NullProgress;
use ewe_lib::wordnet::{Lexicon, LexiconHashMapBackend, ReDBLexicon, SenseId, Synset, SynsetId};
use ewe_lib::validate::{validate_with, ValidationOptions};
use std::collections::HashMap;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::result;

/// Whether `path` looks like a YAML wordnet source directory. `entries-a.yaml` is the
/// traditional check, but a brand new project (e.g. fresh out of `ewe init`) has no entries at
/// all yet, so also accept `frames.yaml` - unlike any particular `entries-*.yaml`, `Lexicon::save`
/// always writes that one, empty or not.
pub(crate) fn looks_like_wordnet_dir(path: &Path) -> bool {
    path.join("frames.yaml").exists() || has_entries_file(path)
}

/// Whether the directory contains any `entries-*.yaml` file (the sections depend on the language)
fn has_entries_file(path: &Path) -> bool {
    std::fs::read_dir(path)
        .map(|d| {
            d.flatten().any(|f| {
                let n = f.file_name().to_string_lossy().to_string();
                n.starts_with("entries-") && n.ends_with(".yaml")
            })
        })
        .unwrap_or(false)
}

/// Which names (proper nouns) an export or the statistics cover, see
/// `ewe_lib::names::NamesMode`
#[derive(clap::ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum NamesArg {
    /// Everything: names keep their `-e` ids and part of speech `e`
    #[default]
    Include,
    /// The core edition, without names
    Exclude,
    /// Only the names
    Only,
    /// The names as ordinary nouns (`-n` ids, `noun.*` lexfiles), as in releases before names
    /// had their own part of speech
    Merged,
}

impl From<NamesArg> for ewe_lib::names::NamesMode {
    fn from(arg: NamesArg) -> Self {
        match arg {
            NamesArg::Include => ewe_lib::names::NamesMode::Include,
            NamesArg::Exclude => ewe_lib::names::NamesMode::Exclude,
            NamesArg::Only => ewe_lib::names::NamesMode::Only,
            NamesArg::Merged => ewe_lib::names::NamesMode::Merged,
        }
    }
}

/// The wordnet restricted to the chosen names, exiting with an error message if that is not
/// possible. `Include` returns the wordnet itself.
pub(crate) fn apply_names(wn: LexiconHashMapBackend, names: NamesArg) -> LexiconHashMapBackend {
    if names == NamesArg::Include {
        return wn;
    }
    ewe_lib::names::restrict_names(&wn, names.into()).unwrap_or_else(|e| {
        eprintln!("Could not select names: {}", e);
        exit(-1);
    })
}

/// The wordnet loaded either wholly into memory or, with `--low-memory`, into an on-disk
/// database whose page cache is bounded (see [`LowMemory`]).
pub(crate) enum Loaded {
    Memory(LexiconHashMapBackend),
    Disk(ReDBLexicon, LowMemory),
}

/// Settings for the on-disk (`--low-memory`) backend
#[derive(Clone, Debug)]
pub(crate) struct LowMemory {
    /// Where the database is kept; `None` for the default location in the user cache directory
    pub db: Option<PathBuf>,
    /// Bound on the database page cache, in MiB
    pub cache_mb: usize,
}

/// Resolve the directory holding the YAML files from the `--wordnet` option
pub(crate) fn resolve_wordnet_path(path: Option<PathBuf>) -> Result<String, String> {
    if let Some(path) = path {
        if looks_like_wordnet_dir(&path) {
            Ok(path.to_string_lossy().to_string())
        } else if looks_like_wordnet_dir(&path.join("src/yaml")) {
            Ok(path.join("src/yaml/").to_string_lossy().to_string())
        } else {
            Err(format!("Could not find WordNet at {}", path.to_string_lossy()))
        }
    } else if looks_like_wordnet_dir(Path::new("./src/yaml")) {
        Ok("./src/yaml/".to_owned())
    } else if looks_like_wordnet_dir(Path::new("./")) {
        Ok("./".to_owned())
    } else {
        Err("Please specify WordNet home".to_owned())
    }
}

pub(crate) fn locate_wordnet(
    path: Option<PathBuf>,
) -> Result<(String, LexiconHashMapBackend), String> {
    let path = resolve_wordnet_path(path)?;
    let mut progress = NullProgress;
    let wn = LexiconHashMapBackend::new()
        .load(&path, &mut progress)
        .map_err(|e| format!("Could not load WordNet from {}: {}", path, e))?;
    Ok((path, wn))
}

/// As [`locate_wordnet`], but with `low_memory` the wordnet is read from a database on disk
/// instead of being loaded into RAM (see [`crate::db`]).
pub(crate) fn locate_wordnet_with(
    path: Option<PathBuf>,
    low_memory: Option<LowMemory>,
) -> Result<(String, Loaded), String> {
    match low_memory {
        None => locate_wordnet(path).map(|(p, wn)| (p, Loaded::Memory(wn))),
        Some(options) => {
            let path = resolve_wordnet_path(path)?;
            let wn = crate::db::open(&path, &options)?;
            Ok((path, Loaded::Disk(wn, options)))
        }
    }
}

/// Restrict a wordnet to the chosen names. In low-memory mode the restricted copy is written to
/// a second on-disk database (deleted when the returned guard is dropped) rather than into RAM.
pub(crate) fn apply_names_disk(
    wn: ReDBLexicon,
    names: NamesArg,
    options: &LowMemory,
) -> (ReDBLexicon, Option<crate::db::TempDb>) {
    if names == NamesArg::Include {
        return (wn, None);
    }
    let temp = crate::db::TempDb::new(options).unwrap_or_else(|e| {
        eprintln!("Could not create a temporary database: {}", e);
        exit(-1);
    });
    let out = ReDBLexicon::create(temp.path(), options.cache_mb * 1024 * 1024).unwrap_or_else(|e| {
        eprintln!("Could not create a temporary database: {}", e);
        exit(-1);
    });
    let restricted = ewe_lib::names::restrict_names_into(&wn, names.into(), out).unwrap_or_else(|e| {
        eprintln!("Could not select names: {}", e);
        exit(-1);
    });
    drop(wn);
    (restricted, Some(temp))
}

/// The `[validation]` options from the project `settings.toml` for the YAML folder `path` (see
/// `ValidationOptions::for_wordnet`), exiting with code 2 if that file can't be read or parsed.
pub(crate) fn load_validation_options(path: &str) -> ValidationOptions {
    ValidationOptions::for_wordnet(Path::new(path)).unwrap_or_else(|e| {
        eprintln!("{}", e);
        std::process::exit(2);
    })
}

pub(crate) fn save<L: Lexicon>(
    wn: &L,
    path: &str,
    options: &ValidationOptions,
) -> result::Result<bool, ewe_lib::wordnet::LexiconSaveError> {
    let mut progress = IndicatifProgress::new();
    let errors = validate_with(wn, &mut progress, options)?;
    if !errors.is_empty() {
        println!("There were validation errors");
        for error in errors {
            println!("{}", error);
        }
        let really_save = input("Do you really want to save [y/N]? ").to_lowercase();
        if really_save == "y" {
            let mut progress = IndicatifProgress::new();
            wn.save_with(path, &mut progress, &storage_options(path))?;
            Ok(true)
        } else {
            Ok(false)
        }
    } else {
        let mut progress = IndicatifProgress::new();
        wn.save_with(path, &mut progress, &storage_options(path))?;
        Ok(true)
    }
}

/// How the project's `settings.toml` says to lay out the YAML files (`names_layout`). A
/// settings file that cannot be read gives the default; loading has already reported it.
fn storage_options(path: &str) -> ewe_lib::wordnet::StorageOptions {
    ewe_lib::wordnet::StorageOptions::for_wordnet(Path::new(path)).unwrap_or_default()
}

pub(crate) fn input(prompt: &str) -> String {
    io::stdout()
        .lock()
        .write_all(prompt.as_bytes())
        .expect("Cannot write to STDOUT");
    io::stdout().flush().expect("Cannot flush STDOUT");
    let mut buffer = String::new();
    io::stdin()
        .read_line(&mut buffer)
        .expect("Cannot read from STDIN");
    buffer.trim().to_string()
}

fn group_by_key<A, B>(pairs: Vec<(A, B)>) -> HashMap<A, Vec<B>>
where
    A: Eq + std::hash::Hash,
{
    let mut map = HashMap::new();

    for (a, b) in pairs {
        map.entry(a).or_insert_with(Vec::new).push(b);
    }

    map
}

pub(crate) fn print_synset(
    synset_id: &SynsetId,
    synset: &Synset,
    lexicon: &impl Lexicon,
    sense_id: Option<&SenseId>,
) {
    println!("{}: {}", synset_id, synset.members.join(", "));
    if let Some(sid) = sense_id {
        println!("    Sense: {}", sid);
    }
    println!("    {}", synset.definition[0]);
    if !synset.example.is_empty() {
        println!(
            "    ({})",
            synset
                .example
                .iter()
                .map(|ex| ex.text.clone())
                .collect::<Vec<String>>()
                .join("; ")
        );
    }
    let links_from = lexicon.links_from(synset_id).expect("Cannot read lexicon");
    for (rel, targets) in group_by_key(links_from) {
        let target_strs: Vec<String> = targets
            .into_iter()
            .map(|t| {
                let s: String = format!(
                    "{} ({})",
                    t,
                    lexicon
                        .synset_by_id(&t)
                        .expect("Cannot read lexicon")
                        .expect("ID not in lexicon")
                        .members
                        .join(", ")
                );
                s
            })
            .collect();
        let target_str = target_strs.join("; ");
        println!("    {}: {}", rel.value(), target_str);
    }
    let links_to = lexicon.links_to(synset_id).expect("Cannot read lexicon");
    for (rel, targets) in
        group_by_key(links_to.into_iter().filter(|(r, _)| !r.is_symmetric()).collect())
    {
        let target_strs: Vec<String> = targets
            .into_iter()
            .map(|t| {
                let s: String = format!(
                    "{} ({})",
                    t,
                    lexicon
                        .synset_by_id(&t)
                        .expect("Cannot read lexicon")
                        .expect("ID not in lexicon")
                        .members
                        .join(", ")
                );
                s
            })
            .collect();
        let target_str = target_strs.join("; ");
        if let Some(inv_rel) = rel.inverse() {
            println!("    {}: {}", inv_rel.value(), target_str);
        } else {
            println!("    Inverse {}: {}", rel.value(), target_str);
        }
    }

    println!("");
}
