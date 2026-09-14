//! `ewe word` - searches for a word by lemma.

use crate::common::{locate_wordnet, print_synset};
use ewe_lib::wordnet::Lexicon;
use std::path::PathBuf;
use std::process::exit;

pub(crate) fn run(word: &str, ignore_case: bool, sense_ids: bool, wordnet: Option<PathBuf>) {
    let (_, wn) = locate_wordnet(wordnet).unwrap_or_else(|e| {
        eprintln!("{}", e);
        exit(-1);
    });
    let entries = if ignore_case {
        wn.entry_by_lemma_ignore_case(word)
            .expect("Cannot read wordnet")
    } else {
        wn.entry_by_lemma(word).expect("Cannot read wordnet")
    };
    if entries.is_empty() {
        println!("No entries found for '{}'", word);
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
