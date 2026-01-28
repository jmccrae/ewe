use serde::{Serialize,Deserialize};
use std::collections::BTreeMap;
use std::io::Write;
use crate::rels::SenseRelType;
use crate::wordnet::*;
use crate::wordnet::util::escape_yaml_string;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Entries(pub(crate) BTreeMap<String, BTreeMap<PosKey, Entry>>);

impl Entries {
    pub(crate) fn new() -> Entries {
        Entries(BTreeMap::new())
    }

    pub(crate) fn entry_by_lemma(&self, lemma : &str) -> Vec<&Entry> {
        self.0.get(lemma).iter().flat_map(|x| x.values()).collect()
    }

    pub(crate) fn entry_by_lemma_with_pos(&self, lemma : &str) -> Vec<(&PosKey, &Entry)> {
        self.0.get(lemma).iter().flat_map(|x| x.iter()).collect()
    }

    pub(crate) fn entry_by_lemma_ignore_case(&self, lemma : &str) -> Vec<&Entry> {
        self.0.iter().filter(|(k,_)| k.to_lowercase() == lemma.to_lowercase()).
            flat_map(|(_,v)| v.values()).collect()
    }

    pub(crate) fn save<W : Write>(&self, w : &mut W) -> std::io::Result<()> {
        for (lemma, by_pos) in self.0.iter() {
            write!(w, "{}:\n", escape_yaml_string(lemma,0,0))?;
            for (pos, entry) in by_pos.iter() {
                write!(w, "  {}:\n", pos.as_str())?;
                entry.save(w)?;
            }
        }
        Ok(())
    }
    pub(crate) fn insert_entry(&mut self, lemma : String, pos : PosKey, entry : Entry) {
        self.0.entry(lemma).
            or_insert_with(|| BTreeMap::new()).insert(pos, entry);
    }

    pub(crate) fn insert_sense(&mut self, lemma : String, pos : PosKey, sense : Sense) {
        match self.0.entry(lemma).
            or_insert_with(|| BTreeMap::new()).get_mut(&pos) {
                Some(entry) => entry.sense.push(sense),
                None => eprintln!("Failed to insert sense to non-existant entry")
            };
    }

    //pub(crate) fn remove_entry(&mut self, 
    //                sense_links_to : &mut HashMap<SenseId, Vec<(SenseRelType, SenseId)>>,
    //                lemma : &str, pos : &PosKey) {
    //    match self.0.get_mut(lemma) {
    //        Some(m) => { 
    //            match m.get(pos) {
    //                Some(e) => remove_sense_link_to(sense_links_to, e),
    //                None => {}
    //            };
    //            m.remove(pos); 
    //        },
    //        None => {}
    //    };
    //    if self.0.contains_key(lemma) && self.0.get(lemma).unwrap().is_empty() {
    //        self.0.remove(lemma);
    //    }
    //}

    pub(crate) fn remove_sense(&mut self, lemma : &str, pos : &PosKey, synset : &SynsetId) -> Vec<SenseId> {
        let removed_ids= match self.0.get_mut(lemma) {
            Some(m) => {
                let sense_id = match m.get_mut(pos) {
                    Some(e) => {
                        let sense_id = e.sense.iter().
                            filter(|s| s.synset == *synset).
                            map(|s| s.id.clone()).collect();
                        e.sense.retain(|s| s.synset != *synset);
                        sense_id
                    },
                    None => Vec::new()
                };
                if m.contains_key(pos) && m.get(pos).unwrap().sense.is_empty() {
                    m.remove(pos);
                }
                sense_id
            },
            None => Vec::new()
        };
        if self.0.contains_key(lemma) && self.0.get(lemma).unwrap().is_empty() {
            self.0.remove(lemma);
        }
        removed_ids
    }

    pub fn sense_links_from(&self, lemma : &str, pos : &PosKey, 
                            synset_id : &SynsetId) -> Vec<(SenseRelType, SenseId)> {
        match self.0.get(lemma) {
            Some(ref mut m) => {
                match m.get(pos) {
                    Some(ref mut e) => {
                        e.sense.iter().filter(|sense| sense.synset == *synset_id)
                            .flat_map(|sense| sense.sense_links_from()).collect()
                    },
                    None => Vec::new()
                }
            },
            None => Vec::new()
        }
    }

    pub fn sense_links_from_id(&self, lemma : &str, pos : &PosKey, 
                               sense_id : &SenseId) -> Vec<(SenseRelType, SenseId)> {
        match self.0.get(lemma) {
            Some(ref mut m) => {
                match m.get(pos) {
                    Some(ref mut e) => {
                        e.sense.iter().filter(|sense| sense.id == *sense_id)
                            .flat_map(|sense| sense.sense_links_from()).collect()
                    },
                    None => Vec::new()
                }
            },
            None => Vec::new()
        }
    }

    pub(crate) fn get_sense_id<'a>(&'a self, lemma : &str, pos : &PosKey, synset_id : &SynsetId) -> 
        Option<&'a SenseId> {
     match self.0.get(lemma) {
            Some(m) => {
                match m.get(pos) {
                    Some(e) => {
                        e.sense.iter().filter(|sense| sense.synset == *synset_id)
                            .map(|sense| &sense.id).nth(0)
                    },
                    None => None
                }
            },
            None => None
        }
    }

    pub(crate) fn get_sense_id2<'a>(&'a self, lemma : &str, synset_id : &SynsetId) -> 
        Option<&'a SenseId> {
     match self.0.get(lemma) {
            Some(m) => {
                for (_, e) in m.iter() {
                    for sense in e.sense.iter() {
                        if sense.synset == *synset_id {
                            return Some(&sense.id);
                        }
                    }
                }
                None
            },
            None => None
        }
    }

    pub(crate) fn add_rel(&mut self, lemma : &str, pos : &PosKey,
               source : &SenseId, rel : SenseRelType,
               target : &SenseId) {
        match self.0.get_mut(lemma) {
            Some(m) => match m.get_mut(pos) {
                Some(e) => {
                    for sense in e.sense.iter_mut() {
                        if sense.id == *source {
                            sense.add_rel(rel.clone(), target.clone());
                        }
                    }
                },
                None => {}
            },
            None => {}
        }
    }

    pub(crate) fn remove_rel(&mut self, lemma : &str, pos : &PosKey,
               source : &SenseId, 
               target : &SenseId) {
        match self.0.get_mut(lemma) {
            Some(m) => match m.get_mut(pos) {
                Some(e) => {
                    for sense in e.sense.iter_mut() {
                        if sense.id == *source {
                            sense.remove_rel(target);
                        }
                    }
                },
                None => {}
            },
            None => {}
        }
    }

    pub fn get_forms(&self, lemma : &str, pos : &PosKey) -> Vec<String> {
        match self.0.get(lemma) {
            Some(m) => match m.get(pos) {
                Some(e) => e.form.clone(),
                None => Vec::new()
            },
            None => Vec::new()
        }
    }

    pub fn add_form(&mut self, lemma : &str, pos : &PosKey, form : String) {
        match self.0.get_mut(lemma) {
            Some(m) => match m.get_mut(pos) {
                Some(e) => {
                    if !e.form.contains(&form) {
                        e.form.push(form);
                    }
                },
                None => {}
            },
            None => {}
        }
    }

    pub fn get_pronunciations(&self, lemma : &str, pos : &PosKey) -> Vec<Pronunciation> {
        match self.0.get(lemma) {
            Some(m) => match m.get(pos) {
                Some(e) => e.pronunciation.iter().map(|x| x.clone()).collect(),
                None => Vec::new()
            },
            None => Vec::new()
        }
    }

    pub fn add_pronunciation(&mut self, lemma : &str, pos : &PosKey, pronunciation : Pronunciation) {
        match self.0.get_mut(lemma) {
            Some(m) => match m.get_mut(pos) {
                Some(e) => {
                    if !e.pronunciation.iter().any(|x| *x == pronunciation) {
                        e.pronunciation.push(pronunciation);
                    }
                },
                None => {}
            },
            None => {}
        }
    }

    pub fn get_sense<'a>(&'a self, lemma : &str, 
                         synset_id : &SynsetId) -> Vec<&'a Sense> {
        match self.0.get(lemma) {
            Some(m) => {
                let mut senses = Vec::new();
                for (_, ss) in m.iter() {
                    for s in ss.sense.iter() {
                        if s.synset == *synset_id {
                            senses.push(s);
                        }
                    }
                }
                senses
            },
            None => Vec::new()
        }
    }


    pub(crate) fn update_sense_key(&mut self, lemma : &str, key : &PosKey,
                        old_key : &SenseId, new_key : &SenseId) {
        match self.0.get_mut(lemma) {
            Some(x) => {
                match x.get_mut(key) {
                    Some(entry) => {
                        for sense in entry.sense.iter_mut() {
                            if sense.id == *old_key {
                                sense.id = new_key.clone();
                            }
                        }
                    },
                    None => {}
                }
            },
            None => {}
        }
    }

    pub(crate) fn entries(&self) -> impl Iterator<Item=(&String, &PosKey, &Entry)> {
        self.0.iter().flat_map(|(lemma, e)| {
            let mut v = Vec::new();
            for (pos, entry) in e.iter() {
                v.push((lemma, pos, entry));
            }
            v
        })
    }

    pub(crate) fn n_entries(&self) -> usize {
        self.0.values().map(|v| v.len()).sum()
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize,Clone,Default)]
pub struct Entry {
    pub sense : Vec<Sense>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub form : Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pronunciation : Vec<Pronunciation>
}

impl Entry {
    pub fn new() -> Entry { Entry::default() }

    pub(crate) fn save<W : Write>(&self, w : &mut W) -> std::io::Result<()> {
        if !self.form.is_empty() {
            write!(w,"    form:")?;
            for f in self.form.iter() {
                write!(w, "\n    - {}", f)?;
            }
            write!(w,"\n")?;
        }
        if !self.pronunciation.is_empty() {
            write!(w,"    pronunciation:")?;
            for p in self.pronunciation.iter() {
                p.save(w)?;
            }
            write!(w,"\n")?;
        }
        write!(w,"    sense:")?;
        for s in self.sense.iter() {
            s.save(w)?;
        }
        write!(w, "\n")?;
        Ok(())
    }
}


