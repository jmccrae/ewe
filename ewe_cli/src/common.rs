//! Helpers shared across more than one subcommand.

use crate::indicatif_progress::IndicatifProgress;
use ewe_lib::progress::NullProgress;
use ewe_lib::wordnet::{Lexicon, LexiconHashMapBackend, SenseId, Synset, SynsetId};
use ewe_lib::validate::validate;
use std::collections::HashMap;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::result;

/// Whether `path` looks like a YAML wordnet source directory. `entries-a.yaml` is the
/// traditional check, but a brand new project (e.g. fresh out of `ewe init`) has no entries at
/// all yet, so also accept `frames.yaml` - unlike any particular `entries-*.yaml`, `Lexicon::save`
/// always writes that one, empty or not.
pub(crate) fn looks_like_wordnet_dir(path: &Path) -> bool {
    path.join("entries-a.yaml").exists() || path.join("frames.yaml").exists()
}

pub(crate) fn locate_wordnet(
    path: Option<PathBuf>,
) -> Result<(String, LexiconHashMapBackend), String> {
    let path = if let Some(path) = path {
        if looks_like_wordnet_dir(&path) {
            path.to_string_lossy().to_string()
        } else if looks_like_wordnet_dir(&path.join("src/yaml")) {
            path.join("src/yaml/").to_string_lossy().to_string()
        } else {
            return Err(format!(
                "Could not find WordNet at {}",
                path.to_string_lossy()
            ));
        }
    } else if looks_like_wordnet_dir(Path::new("./src/yaml")) {
        "./src/yaml/".to_owned()
    } else if looks_like_wordnet_dir(Path::new("./")) {
        "./".to_owned()
    } else {
        return Err(format!("Please specify WordNet home"));
    };
    let mut progress = NullProgress;
    let wn = LexiconHashMapBackend::new()
        .load(&path, &mut progress)
        .map_err(|e| format!("Could not load WordNet from {}: {}", path, e))?;
    Ok((path, wn))
}

pub(crate) fn save<L: Lexicon>(
    wn: &L,
    path: &str,
) -> result::Result<bool, ewe_lib::wordnet::LexiconSaveError> {
    let mut progress = IndicatifProgress::new();
    let errors = validate(wn, &mut progress)?;
    if !errors.is_empty() {
        println!("There were validation errors");
        for error in errors {
            println!("{}", error);
        }
        let really_save = input("Do you really want to save [y/N]? ").to_lowercase();
        if really_save == "y" {
            let mut progress = IndicatifProgress::new();
            wn.save(path, &mut progress)?;
            Ok(true)
        } else {
            Ok(false)
        }
    } else {
        let mut progress = IndicatifProgress::new();
        wn.save(path, &mut progress)?;
        Ok(true)
    }
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
