//! `ewe id` - looks up a synset or sense by its unique ID.

use crate::common::{locate_wordnet, print_synset};
use ewe_lib::wordnet::{Lexicon, SenseId, SynsetId};
use std::path::PathBuf;
use std::process::exit;

pub(crate) fn run(id: &str, wordnet: Option<PathBuf>) {
    let (_, wn) = locate_wordnet(wordnet).unwrap_or_else(|e| {
        eprintln!("{}", e);
        exit(-1);
    });
    if let Some(synset) = wn
        .synset_by_id(&SynsetId::new(&id))
        .expect("Cannot read wordnet")
    {
        print_synset(&SynsetId::new(&id), &synset, &wn, None);
    } else if let Some((_, _, sense)) = wn
        .get_sense_by_id(&SenseId::new(id))
        .expect("Cannot read wordnet")
    {
        let synset = wn
            .synset_by_id(&sense.synset)
            .expect("Cannot read wordnet")
            .unwrap();
        print_synset(&sense.synset, &synset, &wn, Some(&sense.id));
    } else {
        println!("No synset or sense found for '{}'", id);
    }
}
