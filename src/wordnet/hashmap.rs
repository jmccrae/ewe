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
    #[cfg(test)]
    pub fn add_lexfile(&mut self, lexfile : &str) -> Result<()> {
        self.synsets_insert(lexfile.to_owned(), BTSynsets::new())?;
        Ok(())
    }

}

impl Lexicon for LexiconHashMapBackend {
    type E = BTEntries;
    type S = BTSynsets;
    fn entries_get<'a>(&'a self, lemma : &str) -> Result<Option<Cow<'a, BTEntries>>> {
        Ok(self.entries.get(lemma).map(|x| Cow::Borrowed(x)))
    }
    fn entries_insert(&mut self, key : String, entries : BTEntries) -> Result<()> {
        self.entries.insert(key, entries);
        Ok(())
    }
    fn entries_iter<'a>(&'a self) -> Result<impl Iterator<Item=Result<(&'a String, Cow<'a, BTEntries>)>>> {
        Ok(self.entries.iter().map(|(k, v)| Ok((k, Cow::Borrowed(v)))))
    }
    fn entries_update(&mut self, lemma : &str, f : impl FnOnce(&mut BTEntries)) -> Result<()> {
        if let Some(e) = self.entries.get_mut(lemma) {
            f(e);
        } else {
            let mut e = BTEntries::new();
            f(&mut e);
            self.entries.insert(lemma.to_string(), e);
        }
        Ok(())
    }
    fn synsets_get<'a>(&'a self, lexname : &str) -> Result<Option<Cow<'a, BTSynsets>>> {
        Ok(self.synsets.get(lexname).map(|x| Cow::Borrowed(x)))
    }
    fn synsets_insert(&mut self, lexname : String, synsets : BTSynsets) -> Result<()> {
        self.synsets.insert(lexname, synsets);
        Ok(())
    }
    fn synsets_iter<'a>(&'a self) -> Result<impl Iterator<Item=Result<(&'a String, Cow<'a, BTSynsets>)>>> {
        Ok(self.synsets.iter().map(|(k, v)| Ok((k, Cow::Borrowed(v)))))
    }
    fn synsets_update<X>(&mut self, lexname : &str, f : impl FnOnce(&mut BTSynsets) -> X) -> Result<X> {
        if let Some(s) = self.synsets.get_mut(lexname) {
            Ok(f(s))
        } else {
            let mut s = BTSynsets::new();
            let result = f(&mut s);
            self.synsets.insert(lexname.to_string(), s);
            Ok(result)
        }
    }
    fn synset_id_to_lexfile_get(&self, synset_id : &SynsetId) -> Result<Option<&String>> {
        Ok(self.synset_id_to_lexfile.get(synset_id))
    }
    fn synset_id_to_lexfile_insert(&mut self, synset_id : SynsetId, lexfile : String) -> Result<()> {
        self.synset_id_to_lexfile.insert(synset_id, lexfile);
        Ok(())
    }
    fn sense_links_to_get(&self, sense_id : &SenseId) -> Result<Option<&Vec<(SenseRelType, SenseId)>>> {
        Ok(self.sense_links_to.get(sense_id))
    }
    fn sense_links_to_get_or(&mut self, sense_id : SenseId, f : impl FnOnce() -> Vec<(SenseRelType, SenseId)>) 
        -> Result<&mut Vec<(SenseRelType, SenseId)>> {
        Ok(self.sense_links_to.entry(sense_id).or_insert_with(f))
    }
    fn sense_links_to_update(&mut self, sense_id : &SenseId, f : impl FnOnce(&mut Vec<(SenseRelType, SenseId)>)) -> Result<()> {
        if let Some(v) = self.sense_links_to.get_mut(sense_id) {
            f(v);
        } else {
            let mut v = Vec::new();
            f(&mut v);
            self.sense_links_to.insert(sense_id.clone(), v);
        }
        Ok(())
    }
    fn sense_links_to_push(&mut self, sense_id : SenseId, rel : SenseRelType, target : SenseId) -> Result<()> {
        self.sense_links_to.entry(sense_id).or_insert_with(Vec::new)
            .push((rel, target));
        Ok(())
    }
    fn set_sense_links_to(&mut self, links_to : HashMap<SenseId, Vec<(SenseRelType, SenseId)>>) -> Result<()> {
        self.sense_links_to = links_to;
        Ok(())
    }
    fn links_to_get(&self, synset_id : &SynsetId) -> Result<Option<&Vec<(SynsetRelType, SynsetId)>>> {
        Ok(self.links_to.get(synset_id))
    }
    fn links_to_get_or(&mut self, synset_id : SynsetId, f : impl FnOnce() -> Vec<(SynsetRelType, SynsetId)>) 
        -> Result<&mut Vec<(SynsetRelType, SynsetId)>> {
        Ok(self.links_to.entry(synset_id).or_insert_with(f))
    }
    fn links_to_update(&mut self, synset_id : &SynsetId, f : impl FnOnce(&mut Vec<(SynsetRelType, SynsetId)>)) -> Result<()> {
        if let Some(v) = self.links_to.get_mut(synset_id) {
            f(v);
        } else {
            let mut v = Vec::new();
            f(&mut v);
            self.links_to.insert(synset_id.clone(), v);
        }
        Ok(())
    }
    fn links_to_push(&mut self, synset_id : SynsetId, rel : SynsetRelType, target : SynsetId) -> Result<()> {
        self.links_to.entry(synset_id).or_insert_with(Vec::new)
            .push((rel, target));
        Ok(())
    }
    fn set_links_to(&mut self, links_to : HashMap<SynsetId, Vec<(SynsetRelType, SynsetId)>>) -> Result<()> {
        self.links_to = links_to;
        Ok(())
    }
    fn sense_id_to_lemma_pos_get(&self, sense_id : &SenseId) -> Result<Option<&(String, PosKey)>> {
        Ok(self.sense_id_to_lemma_pos.get(sense_id))
    }
    fn sense_id_to_lemma_pos_insert(&mut self, sense_id : SenseId, lemma_pos : (String, PosKey)) -> Result<()> {
        self.sense_id_to_lemma_pos.insert(sense_id, lemma_pos);
        Ok(())
    }
    fn deprecations_get(&self) -> Result<&Vec<DeprecationRecord>> {
        Ok(&self.deprecations)
    }
    fn deprecations_push(&mut self, record : DeprecationRecord) -> Result<()> {
        self.deprecations.push(record);
        Ok(())
    }
}


