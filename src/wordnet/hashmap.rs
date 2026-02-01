use std::collections::HashMap;
use crate::rels::{SenseRelType,SynsetRelType};
use crate::wordnet::*;
use crate::wordnet::entry::BTEntries;
use std::borrow::Cow;

pub struct LexiconHashMapBackend {
    entries : HashMap<char, BTEntries>,
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
    fn entries_get<'a>(&'a self, key : char) -> Result<Option<Cow<'a, BTEntries>>> {
        Ok(self.entries.get(&key).map(|x| Cow::Borrowed(x)))
    }
    fn entries_insert(&mut self, key : char, entries : BTEntries) -> Result<()> {
        self.entries.insert(key, entries);
        Ok(())
    }
    fn entries_iter<'a>(&'a self) -> Result<impl Iterator<Item=Result<(char, Cow<'a, BTEntries>)>>> {
        Ok(self.entries.iter().map(|(k, v)| Ok((*k, Cow::Borrowed(v)))))
    }
    fn entries_update(&mut self, key : char, f : impl FnOnce(&mut BTEntries)) -> Result<()> {
        if let Some(e) = self.entries.get_mut(&key) {
            f(e);
        } else {
            let mut e = BTEntries::new();
            f(&mut e);
            self.entries.insert(key, e);
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
    fn synsets_insert_synset(&mut self, lexname : &str, synset_id : SynsetId, synset : Synset) -> Result<()> {
        self.synsets.entry(lexname.to_owned()).or_insert_with(BTSynsets::new)
            .insert(synset_id.clone(), synset.clone())?;
        Ok(())
    }
    fn synsets_remove_synset(&mut self, lexname : &str,  synset_id : &SynsetId) -> Result<Option<(SynsetId, Synset)>> {
        if let Some(synsets) = self.synsets.get_mut(lexname) {
            if let Some(synset) = synsets.remove_entry(synset_id)? {
                return Ok(Some(synset))
            }
        }
        Ok(None)
    }
    //fn remove_synset(&mut self, synset_id : &SynsetId) -> Result<()> {
    //    let mut removed = Vec::new();
    //    for synsets in self.synsets.values_mut() {
    //        synsets.remove_entry(synset_id)?;
    //    }
    //    Ok(())
    //}
    //fn insert_synset(&mut self, lexname : String, synset_id : SynsetId,
    //                     synset : Synset) -> Result<()> {
    //    add_link_to(self, &synset_id, &synset)?;
    //    self.synset_id_to_lexfile_insert(synset_id.clone(), lexname.clone())?;
    //    self.synsets.entry(lexname).or_insert_with(BTSynsets::new)
    //        .insert(synset_id, synset)?;
    //    Ok(())
    //}

    fn update_synset(&mut self, synset_id : &SynsetId, f : impl FnOnce(&mut Synset)) -> Result<()> {
        for synsets in self.synsets.values_mut() {
            if let Some(synset) = synsets.get_mut(synset_id) {
                f(synset);
                return Ok(());
            }
        }
        Err(LexiconError::SynsetIdNotFound(synset_id.clone()))
    }

    fn synset_id_to_lexfile_get<'a>(&'a self, synset_id : &SynsetId) -> Result<Option<Cow<'a, String>>> {
        Ok(self.synset_id_to_lexfile.get(synset_id).map(Cow::Borrowed))
    }
    fn synset_id_to_lexfile_insert(&mut self, synset_id : SynsetId, lexfile : String) -> Result<()> {
        self.synset_id_to_lexfile.insert(synset_id, lexfile);
        Ok(())
    }
    fn sense_links_to_get<'a>(&'a self, sense_id : &SenseId) -> Result<Option<Cow<'a, Vec<(SenseRelType, SenseId)>>>> {
        Ok(self.sense_links_to.get(sense_id).map(Cow::Borrowed))
    }
    fn sense_links_to_get_or(&mut self, sense_id : SenseId, f : impl FnOnce() -> Vec<(SenseRelType, SenseId)>) 
        -> Result<Vec<(SenseRelType, SenseId)>> {
        Ok(self.sense_links_to.entry(sense_id).or_insert_with(f).clone())
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
    fn links_to_get<'a>(&'a self, synset_id : &SynsetId) -> Result<Option<Cow<'a, Vec<(SynsetRelType, SynsetId)>>>> {
        Ok(self.links_to.get(synset_id).map(Cow::Borrowed))
    }
    fn links_to_get_or(&mut self, synset_id : SynsetId, f : impl FnOnce() -> Vec<(SynsetRelType, SynsetId)>) -> Result<Vec<(SynsetRelType, SynsetId)>> {
        Ok(self.links_to.entry(synset_id).or_insert_with(f).clone())
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
    fn sense_id_to_lemma_pos_get(&self, sense_id : &SenseId) -> Result<Option<(String, PosKey)>> {
        Ok(self.sense_id_to_lemma_pos.get(sense_id).cloned())
    }
    fn sense_id_to_lemma_pos_insert(&mut self, sense_id : SenseId, lemma_pos : (String, PosKey)) -> Result<()> {
        self.sense_id_to_lemma_pos.insert(sense_id, lemma_pos);
        Ok(())
    }
    fn deprecations_get<'a>(&'a self) -> Result<Cow<'a, Vec<DeprecationRecord>>> {
        Ok(Cow::Borrowed(&self.deprecations))
    }
    fn deprecations_push(&mut self, record : DeprecationRecord) -> Result<()> {
        self.deprecations.push(record);
        Ok(())
    }
}


