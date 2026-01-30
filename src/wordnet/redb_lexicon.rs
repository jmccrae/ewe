use redb::{Database, TableDefinition};
use crate::wordnet::*;
use crate::wordnet::lexicon::entry_key;
use std::rc::Rc;
use crate::wordnet::entry::BTEntries;
use crate::wordnet::synset::BTSynsets;
use std::collections::HashMap;
use crate::rels::{SenseRelType, SynsetRelType};

const ENTRIES_TABLE: TableDefinition<(&str, &str), String> = TableDefinition::new("entries");


pub struct ReDBLexicon {
    db: Rc<Database>,
    entries: HashMap<String, ReDBEntries>,
    synsets: HashMap<String, ReDBSynsets>,
}

impl Lexicon for ReDBLexicon {
    type E = ReDBEntries;
    type S = ReDBSynsets;
    // Data access methods
    fn entries_get(&self, lemma : &str) -> Option<&Self::E> {
        self.entries.get(&entry_key(lemma))
    }
    fn entries_insert(&mut self, key : String, entries : BTEntries) {
        panic!("TODO");
    }
    fn entries_iter(&self) -> impl Iterator<Item=(&String, &Self::E)> {
        self.entries.iter()
    }
    fn entries_update(&mut self, lemma : &str, f : impl FnOnce(&mut Self::E)) {
        f(self.entries.entry(entry_key(lemma).to_string())
            .or_insert_with(|| ReDBEntries {
                db: Rc::clone(&self.db),
                key: entry_key(lemma).chars().next().unwrap()
            })
        );
    }
    fn synsets_get(&self, lexname : &str) -> Option<&Self::S> {
        panic!("TODO");
    }
    fn synsets_insert(&mut self, lexname : String, synsets : BTSynsets) {
        panic!("TODO");
    }
    fn synsets_iter(&self) -> impl Iterator<Item=(&String, &Self::S)> {
        self.synsets.iter()
    }
    fn synsets_update<X>(&mut self, lexname : &str, f : impl FnOnce(&mut Self::S) -> X) -> X {
        panic!("TODO");
    }
    fn synsets_contains_key(&self, lexname : &str) -> bool {
        self.synsets_get(lexname).is_some()
    }
    fn synset_id_to_lexfile_get(&self, synset_id : &SynsetId) -> Option<&String> {
        panic!("TODO");
    }
    fn synset_id_to_lexfile_insert(&mut self, synset_id : SynsetId, lexfile : String) {
        panic!("TODO");
    }
    fn sense_links_to_get(&self, sense_id : &SenseId) -> Option<&Vec<(SenseRelType, SenseId)>> {
        panic!("TODO");
    }
    fn sense_links_to_get_or(&mut self, sense_id : SenseId, f : impl FnOnce() -> Vec<(SenseRelType, SenseId)>) 
        -> &mut Vec<(SenseRelType, SenseId)> {
            panic!("TODO");
    }
    fn sense_links_to_update(&mut self, sense_id : &SenseId, f : impl FnOnce(&mut Vec<(SenseRelType, SenseId)>)) {
        panic!("TODO");
    }
    fn sense_links_to_push(&mut self, sense_id : SenseId, rel : SenseRelType, target : SenseId) {
        panic!("TODO");
    }
    fn set_sense_links_to(&mut self, links_to : HashMap<SenseId, Vec<(SenseRelType, SenseId)>>) {
        panic!("TODO");
    }
    fn links_to_get(&self, synset_id : &SynsetId) -> Option<&Vec<(SynsetRelType, SynsetId)>> {
        panic!("TODO");
    }
    fn links_to_get_or(&mut self, synset_id : SynsetId, f : impl FnOnce() -> Vec<(SynsetRelType, SynsetId)>) 
        -> &mut Vec<(SynsetRelType, SynsetId)> {
            panic!("TODO");
    }
    fn links_to_update(&mut self, synset_id : &SynsetId, f : impl FnOnce(&mut Vec<(SynsetRelType, SynsetId)>)) {
        panic!("TODO");
    }
    fn links_to_push(&mut self, synset_id : SynsetId, rel : SynsetRelType, target : SynsetId) {
        panic!("TODO");
    }
    fn set_links_to(&mut self, links_to : HashMap<SynsetId, Vec<(SynsetRelType, SynsetId)>>) {
        panic!("TODO");
    }
    fn sense_id_to_lemma_pos_get(&self, sense_id : &SenseId) -> Option<&(String, PosKey)> {
        panic!("TODO");
    }
    fn sense_id_to_lemma_pos_insert(&mut self, sense_id : SenseId, lemma_pos : (String, PosKey)) {
        panic!("TODO");
    }
    fn deprecations_get(&self) -> &Vec<DeprecationRecord> {
        panic!("TODO");
    }
    fn deprecations_push(&mut self, record : DeprecationRecord) {
        panic!("TODO");
    }
}

struct ReDBEntries {
    db: Rc<Database>,
    key : char
}

impl Entries for ReDBEntries {
    fn entry(&self, lemma : &str, pos_key : &PosKey) -> Option<&Entry> {
        panic!("TODO");
    }
    fn insert_entry(&mut self, lemma : String, pos : PosKey, entry : Entry) {
        panic!("TODO");
    }
    fn update_entry<X>(&mut self, lemma : &str, pos_key : &PosKey,
        f : impl FnOnce(&mut Entry) -> X) -> Result<X, String> {
        panic!("TODO");
        }
    fn remove_entry(&mut self, lemma : &str, pos_key : &PosKey) -> Option<Entry> {
        panic!("TODO");
    }

    fn entry_by_lemma(&self, lemma : &str) -> Vec<&Entry> {
        panic!("TODO");
    }
    fn entry_by_lemma_with_pos(&self, lemma : &str) -> Vec<(&PosKey, &Entry)> {
        panic!("TODO");
    }
    fn entry_by_lemma_ignore_case(&self, lemma : &str) -> Vec<&Entry> {
        panic!("TODO");
    }

    fn iter(&self) -> impl Iterator<Item=(&String,Vec<(&PosKey, &Entry)>)> {
        panic!("TODO");
        Vec::new().iter()
    }

    fn n_entries(&self) -> usize {
        panic!("TODO");
    }
 
}

struct ReDBSynsets {
    db: Rc<Database>,
    lexname : String
}

impl Synsets for ReDBSynsets {
    fn get(&self, id : &SynsetId) -> Option<&Synset> {
        panic!("TODO");
    }
    fn insert(&mut self, id : SynsetId, sysnet : Synset) -> Option<Synset> {
        panic!("TODO");
    }
    fn update<X>(&mut self, id : &SynsetId, f : impl FnOnce(&mut Synset) -> X) -> Result<X, String> {
        panic!("TODO");
    }
    fn iter(&self) -> impl Iterator<Item=(&SynsetId, &Synset)> {
        panic!("TODO")
    }
    fn len(&self) -> usize {
        panic!("TODO");
    }
    fn remove_entry(&mut self, id : &SynsetId) -> Option<(SynsetId, Synset)> {
        panic!("TODO");
    }
}
