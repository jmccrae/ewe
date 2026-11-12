//! `ewe word` - searches for a word by lemma or alternate form.

use crate::common::{locate_wordnet, print_synset};
use ewe_lib::wordnet::Lexicon;
use std::path::PathBuf;
use std::process::exit;

pub(crate) fn run(word: &str, ignore_case: bool, sense_ids: bool, wordnet: Option<PathBuf>) {
    let (_, wn) = locate_wordnet(wordnet).unwrap_or_else(|e| {
        eprintln!("{}", e);
        exit(-1);
    });
    // Falls back to alternate forms (e.g. "geese" -> "goose") if no lemma matches
    let entries = wn.lookup_word(word, ignore_case).expect("Cannot read wordnet");
    if entries.is_empty() {
        println!("No entries found for '{}'", word);
        let suggestions = wn.lemma_search(word, Some(10)).unwrap_or_default();
        if !suggestions.is_empty() {
            println!("Did you mean: {}", suggestions.join(", "));
        }
    } else {
        for entry in entries {
            for sense in &entry.sense {
                let synset = wn
                    .synset_by_id(&sense.synset)
                    .expect("Cannot read wordnet")
                    .unwrap();
                let sid = if sense_ids { Some(&sense.id) } else { None };
                print_synset(&sense.synset, &synset, &wn, sid);
            }
        }
    }
}
