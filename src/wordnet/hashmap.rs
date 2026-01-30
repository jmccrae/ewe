use std::collections::HashMap;
use crate::rels::{SenseRelType,SynsetRelType};
use crate::wordnet::*;
use crate::wordnet::entry::BTEntries;
use std::borrow::Cow;

pub struct LexiconHashMapBackend {
    entries : HashMap<String, BTEntries>,
    synsets : HashMap<String, BTSynsets>,
    synset_id_to_lexfile : HashMap<SynsetId, String>,
    sense_links_to : HashMap<SenseId, Vec<(SenseRelType, SenseId)>>,
    links_to : HashMap<SynsetId, Vec<(SynsetRelType, SynsetId)>>,
    sense_id_to_lemma_pos : HashMap<SenseId, (String, PosKey)>,
    deprecations : Vec<DeprecationRecord>
}

impl LexiconHashMapBackend {
    pub fn new() -> LexiconHashMapBackend {
        LexiconHashMapBackend {
            entries : HashMap::new(),
            synsets : HashMap::new(),
            synset_id_to_lexfile : HashMap::new(),
            sense_links_to : HashMap::new(),
            links_to : HashMap::new(),
            sense_id_to_lemma_pos : HashMap::new(),
            deprecations : Vec::new()
        }
    }
}

impl Lexicon for LexiconHashMapBackend {
    type E = BTEntries;
    type S = BTSynsets;
    fn entries_get<'a>(&'a self, lemma : &str) -> Option<Cow<'a, BTEntries>> {
        self.entries.get(lemma).map(|x| Cow::Borrowed(x))
    }
    fn entries_insert(&mut self, key : String, entries : BTEntries) {
        self.entries.insert(key, entries);
    }
    fn entries_iter<'a>(&'a self) -> impl Iterator<Item=(&'a String, Cow<'a, BTEntries>)> {
        self.entries.iter().map(|(k, v)| (k, Cow::Borrowed(v)))
    }
    fn entries_update(&mut self, lemma : &str, f : impl FnOnce(&mut BTEntries)) {
        if let Some(e) = self.entries.get_mut(lemma) {
            f(e);
        } else {
            let mut e = BTEntries::new();
            f(&mut e);
            self.entries.insert(lemma.to_string(), e);
        }
    }
    fn synsets_get<'a>(&'a self, lexname : &str) -> Option<Cow<'a, BTSynsets>> {
        self.synsets.get(lexname).map(|x| Cow::Borrowed(x))
    }
    fn synsets_insert(&mut self, lexname : String, synsets : BTSynsets) {
        self.synsets.insert(lexname, synsets);
    }
    fn synsets_iter<'a>(&'a self) -> impl Iterator<Item=(&'a String, Cow<'a, BTSynsets>)> {
        self.synsets.iter().map(|(k, v)| (k, Cow::Borrowed(v)))
    }
    fn synsets_update<X>(&mut self, lexname : &str, f : impl FnOnce(&mut BTSynsets) -> X) -> X {
        if let Some(s) = self.synsets.get_mut(lexname) {
            f(s)
        } else {
            let mut s = BTSynsets::new();
            let result = f(&mut s);
            self.synsets.insert(lexname.to_string(), s);
            result
        }
    }
    fn synset_id_to_lexfile_get(&self, synset_id : &SynsetId) -> Option<&String> {
        self.synset_id_to_lexfile.get(synset_id)
    }
    fn synset_id_to_lexfile_insert(&mut self, synset_id : SynsetId, lexfile : String) {
        self.synset_id_to_lexfile.insert(synset_id, lexfile);
    }
    fn sense_links_to_get(&self, sense_id : &SenseId) -> Option<&Vec<(SenseRelType, SenseId)>> {
        self.sense_links_to.get(sense_id)
    }
    fn sense_links_to_get_or(&mut self, sense_id : SenseId, f : impl FnOnce() -> Vec<(SenseRelType, SenseId)>) 
        -> &mut Vec<(SenseRelType, SenseId)> {
        self.sense_links_to.entry(sense_id).or_insert_with(f)
    }
    fn sense_links_to_update(&mut self, sense_id : &SenseId, f : impl FnOnce(&mut Vec<(SenseRelType, SenseId)>)) {
        if let Some(v) = self.sense_links_to.get_mut(sense_id) {
            f(v);
        } else {
            let mut v = Vec::new();
            f(&mut v);
            self.sense_links_to.insert(sense_id.clone(), v);
        }
    }
    fn sense_links_to_push(&mut self, sense_id : SenseId, rel : SenseRelType, target : SenseId) {
        self.sense_links_to.entry(sense_id).or_insert_with(Vec::new)
            .push((rel, target));
    }
    fn set_sense_links_to(&mut self, links_to : HashMap<SenseId, Vec<(SenseRelType, SenseId)>>) {
        self.sense_links_to = links_to;
    }
    fn links_to_get(&self, synset_id : &SynsetId) -> Option<&Vec<(SynsetRelType, SynsetId)>> {
        self.links_to.get(synset_id)
    }
    fn links_to_get_or(&mut self, synset_id : SynsetId, f : impl FnOnce() -> Vec<(SynsetRelType, SynsetId)>) 
        -> &mut Vec<(SynsetRelType, SynsetId)> {
        self.links_to.entry(synset_id).or_insert_with(f)
    }
    fn links_to_update(&mut self, synset_id : &SynsetId, f : impl FnOnce(&mut Vec<(SynsetRelType, SynsetId)>)) {
        if let Some(v) = self.links_to.get_mut(synset_id) {
            f(v);
        } else {
            let mut v = Vec::new();
            f(&mut v);
            self.links_to.insert(synset_id.clone(), v);
        }
    }
    fn links_to_push(&mut self, synset_id : SynsetId, rel : SynsetRelType, target : SynsetId) {
        self.links_to.entry(synset_id).or_insert_with(Vec::new)
            .push((rel, target));
    }
    fn set_links_to(&mut self, links_to : HashMap<SynsetId, Vec<(SynsetRelType, SynsetId)>>) {
        self.links_to = links_to;
    }
    fn sense_id_to_lemma_pos_get(&self, sense_id : &SenseId) -> Option<&(String, PosKey)> {
        self.sense_id_to_lemma_pos.get(sense_id)
    }
    fn sense_id_to_lemma_pos_insert(&mut self, sense_id : SenseId, lemma_pos : (String, PosKey)) {
        self.sense_id_to_lemma_pos.insert(sense_id, lemma_pos);
    }
    fn deprecations_get(&self) -> &Vec<DeprecationRecord> {
        &self.deprecations
    }
    fn deprecations_push(&mut self, record : DeprecationRecord) {
        self.deprecations.push(record);
    }
}


