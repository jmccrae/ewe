//! `ewe stats` - summary statistics about the wordnet, plus counts for the hypernym-hierarchy
//! "test patterns" from Lohk, Fellbaum & Võhandu, "Tuning Hierarchies in Princeton WordNet"
//! (GWC 2016).

use crate::common::locate_wordnet;
use ewe_lib::wordnet::{Lexicon, SynsetId};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::exit;

fn print_count_table(counts: &BTreeMap<String, usize>) {
    for (key, count) in counts {
        println!("    {:<20} {}", key, count);
    }
}

pub(crate) fn run(
    wordnet: Option<PathBuf>,
    self_hypernymy_instances: bool,
    shortcut_instances: bool,
    dense_component_instances: bool,
    compound_pattern_instances: bool,
) {
    let (_, wn) = locate_wordnet(wordnet).unwrap_or_else(|e| {
        eprintln!("{}", e);
        exit(-1);
    });

    let mut synsets_by_pos: BTreeMap<String, usize> = BTreeMap::new();
    let mut synset_rel_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut roots_by_pos: BTreeMap<String, usize> = BTreeMap::new();
    let mut n_synsets = 0usize;
    let mut n_multiple_inheritance = 0usize;
    for s in wn.synsets().expect("Cannot read wordnet") {
        let (_, synset) = s.expect("Cannot read wordnet");
        n_synsets += 1;
        *synsets_by_pos
            .entry(synset.part_of_speech.value().to_string())
            .or_insert(0) += 1;
        let n_parents = synset.hypernym.len() + synset.instance_hypernym.len();
        if n_parents == 0 {
            *roots_by_pos
                .entry(synset.part_of_speech.value().to_string())
                .or_insert(0) += 1;
        } else if n_parents > 1 {
            n_multiple_inheritance += 1;
        }
        for (rel, _) in synset.links_from() {
            *synset_rel_counts.entry(rel.value().to_string()).or_insert(0) += 1;
        }
    }

    let mut entries_by_pos: BTreeMap<String, usize> = BTreeMap::new();
    let mut sense_rel_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut n_entries = 0usize;
    let mut n_senses = 0usize;
    for e in wn.entries().expect("Cannot read wordnet") {
        let (_, pos, entry) = e.expect("Cannot read wordnet");
        n_entries += 1;
        *entries_by_pos.entry(pos.as_str().to_string()).or_insert(0) += 1;
        n_senses += entry.sense.len();
        for sense in entry.sense.iter() {
            for (rel, _) in sense.sense_links_from() {
                *sense_rel_counts.entry(rel.value().to_string()).or_insert(0) += 1;
            }
        }
    }

    let n_synset_relations: usize = synset_rel_counts.values().sum();
    let n_sense_relations: usize = sense_rel_counts.values().sum();

    println!("Synsets: {}", n_synsets);
    print_count_table(&synsets_by_pos);
    println!("Entries: {}", n_entries);
    print_count_table(&entries_by_pos);
    println!("Senses:  {}", n_senses);
    println!();
    println!(
        "Relations: {}",
        n_synset_relations + n_sense_relations
    );
    println!("  Synset relations: {}", n_synset_relations);
    print_count_table(&synset_rel_counts);
    println!("  Sense relations: {}", n_sense_relations);
    print_count_table(&sense_rel_counts);
    println!();
    println!("Roots (no hypernym): {}", roots_by_pos.values().sum::<usize>());
    print_count_table(&roots_by_pos);
    println!("Multiple inheritance: {}", n_multiple_inheritance);

    println!();
    let definition = |id: &SynsetId| {
        wn.synset_by_id(id)
            .ok()
            .flatten()
            .and_then(|s| s.definition.get(0).cloned())
            .unwrap_or_default()
    };
    let graph = ewe_lib::stats::HypernymGraph::build(&wn).expect("Cannot read wordnet");

    let self_hyper = ewe_lib::stats::find_self_hypernymy(&graph);
    let distinct_words: std::collections::HashSet<&str> =
        self_hyper.iter().map(|i| i.word.as_str()).collect();
    let distinct_synsets: std::collections::HashSet<&SynsetId> =
        self_hyper.iter().map(|i| &i.synset).collect();
    println!(
        "Self-hypernymy: {} instance(s) across {} word(s) and {} synset(s)",
        self_hyper.len(),
        distinct_words.len(),
        distinct_synsets.len()
    );
    if self_hypernymy_instances {
        for inst in &self_hyper {
            println!(
                "  {}: {} ({}) -> {} ({})",
                inst.word,
                inst.synset.as_str(),
                definition(&inst.synset),
                inst.ancestor.as_str(),
                definition(&inst.ancestor)
            );
        }
    }

    let shortcuts = ewe_lib::stats::find_shortcuts(&graph);
    println!("Shortcut: {} instance(s)", shortcuts.len());
    if shortcut_instances {
        for inst in &shortcuts {
            println!(
                "  {} ({}) -> {} ({}) [also reachable via {} ({})]",
                inst.synset.as_str(),
                definition(&inst.synset),
                inst.redundant_hypernym.as_str(),
                definition(&inst.redundant_hypernym),
                inst.via.as_str(),
                definition(&inst.via)
            );
        }
    }

    let dense_components = ewe_lib::stats::find_dense_components(&graph);
    println!("Dense component: {} instance(s)", dense_components.len());
    if dense_component_instances {
        for inst in &dense_components {
            let children: Vec<String> = inst
                .children
                .iter()
                .map(|c| format!("{} ({})", c.as_str(), definition(c)))
                .collect();
            println!(
                "  {} ({}) & {} ({}): {}",
                inst.parents.0.as_str(),
                definition(&inst.parents.0),
                inst.parents.1.as_str(),
                definition(&inst.parents.1),
                children.join(", ")
            );
        }
    }

    let compound_pattern = ewe_lib::stats::find_compound_pattern(&graph);
    println!("Compound pattern: {} instance(s)", compound_pattern.len());
    if compound_pattern_instances {
        for inst in &compound_pattern {
            let children: Vec<String> = inst
                .children
                .iter()
                .map(|c| format!("{} ({})", c.as_str(), definition(c)))
                .collect();
            let extra: Vec<String> = inst
                .children_with_extra_hypernym
                .iter()
                .map(|c| format!("{} ({})", c.as_str(), definition(c)))
                .collect();
            println!(
                "  {}: {} ({}) -> {} [extra hypernym: {}]",
                inst.word,
                inst.parent.as_str(),
                definition(&inst.parent),
                children.join(", "),
                extra.join(", ")
            );
        }
    }
}
