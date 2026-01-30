use serde::{Serialize,Deserialize};
use std::collections::BTreeMap;
use std::io::Write;
use crate::rels::SenseRelType;
use crate::wordnet::*;
use std::borrow::Cow;
use crate::wordnet::util::escape_yaml_string;
use std::result;

pub trait Entries : Sized {
    fn entry<'a>(&'a self, lemma : &str, pos_key : &PosKey) -> Option<Cow<'a, Entry>>;
    fn insert_entry(&mut self, lemma : String, pos : PosKey, entry : Entry);
    fn update_entry<X>(&mut self, lemma : &str, pos_key : &PosKey,
        f : impl FnOnce(&mut Entry) -> X) -> Result<X>;
    fn remove_entry(&mut self, lemma : &str, pos_key : &PosKey) -> Option<Entry>;

    fn entry_by_lemma<'a>(&'a self, lemma : &str) -> Vec<Cow<'a, Entry>>;
    fn entry_by_lemma_with_pos<'a>(&'a self, lemma : &str) -> Vec<(PosKey, Cow<'a, Entry>)>;
    fn entry_by_lemma_ignore_case<'a>(&'a self, lemma : &str) -> Vec<Cow<'a, Entry>>;

    //fn iter(&self) -> impl Iterator<Item=(&String,Vec<(&PosKey, &Entry)>)>;

    fn n_entries(&self) -> usize;
    
    fn save<W : Write>(&self, w : &mut W) -> result::Result<(), LexiconSaveError> {
        let mut last_lemma = None;
        for (lemma, pos, entry) in self.entries() {
            if last_lemma.is_none() || last_lemma.as_ref().unwrap() != &lemma {
                write!(w, "{}:\n", escape_yaml_string(&lemma,0,0))?;
            }
            write!(w, "  {}:\n", pos.as_str())?;
            entry.save(w)?;
            last_lemma = Some(lemma);
        }
        Ok(())
    }


    fn insert_sense(&mut self, lemma : String, pos : PosKey, sense : Sense) -> Result<()> {
        self.update_entry(&lemma, &pos, |entry| {
            entry.sense.push(sense);
        })?;
        Ok(())
    }
    fn remove_sense(&mut self, lemma : &str, pos : &PosKey, synset : &SynsetId) -> Vec<SenseId> {
        if let Ok(removed_ids) = self.update_entry(lemma, pos, |e| {
            let sense_id = e.sense.iter().
                filter(|s| s.synset == *synset).
                map(|s| s.id.clone()).collect();
            e.sense.retain(|s| s.synset != *synset);
            sense_id
        }) {
            if let Some(e) = self.entry(lemma, pos) {
                if e.sense.is_empty() {
                    self.remove_entry(lemma, pos);
                }
            }
            removed_ids
        } else {
            vec![]
        }
    }

    fn sense_links_from(&self, lemma : &str, pos : &PosKey, 
                            synset_id : &SynsetId) -> Vec<(SenseRelType, SenseId)> {
        if let Some(e) = self.entry(lemma, pos) {
            e.sense.iter().filter(|sense| sense.synset == *synset_id)
                .flat_map(|sense| sense.sense_links_from()).collect()
        } else {
            vec![]
        }
    }

    fn sense_links_from_id(&self, lemma : &str, pos : &PosKey, 
                               sense_id : &SenseId) -> Vec<(SenseRelType, SenseId)> {
        if let Some(e) = self.entry(lemma, pos) {
            e.sense.iter().filter(|sense| sense.id == *sense_id)
                .flat_map(|sense| sense.sense_links_from()).collect()
        } else {
            vec![]
        }
    }

    fn get_sense_id<'a>(&'a self, lemma : &str, pos : &PosKey, synset_id : &SynsetId) -> Option<SenseId> {
        if let Some(e) = self.entry(lemma, pos) {
            e.sense.iter().filter(|sense| sense.synset == *synset_id)
                .map(|sense| sense.id.clone()).nth(0)
        } else {
            None
        }
    }

    fn get_sense_id2<'a>(&'a self, lemma : &str, synset_id : &SynsetId) -> Option<SenseId> {
        for e in self.entry_by_lemma(lemma) {
            for sense in e.sense.iter() {
                if sense.synset == *synset_id {
                    return Some(sense.id.clone());
                }
            }
        }
        None
    }

    fn add_rel(&mut self, lemma : &str, pos : &PosKey,
               source : &SenseId, rel : SenseRelType,
               target : &SenseId) -> Result<()> {
        self.update_entry(lemma, pos, |e| {
            for sense in e.sense.iter_mut() {
                if sense.id == *source {
                    sense.add_rel(rel.clone(), target.clone());
                }
            }
        })
    }

    fn remove_rel(&mut self, lemma : &str, pos : &PosKey,
               source : &SenseId, 
               target : &SenseId) -> Result<()> {
        self.update_entry(lemma, pos, |e| {
            for sense in e.sense.iter_mut() {
                if sense.id == *source {
                    sense.remove_rel(target);
                }
            }
        })
    }

    fn get_forms(&self, lemma : &str, pos : &PosKey) -> Vec<String> {
        if let Some(e) = self.entry(lemma, pos) {
            e.form.clone()
        } else {
            vec![]
        }
    }

    fn add_form(&mut self, lemma : &str, pos : &PosKey, form : String) -> Result<()> {
        self.update_entry(lemma, pos, |e| {
            if !e.form.contains(&form) {
                e.form.push(form);
            }
        })
    }

    fn get_pronunciations(&self, lemma : &str, pos : &PosKey) -> Vec<Pronunciation> {
        if let Some(e) = self.entry(lemma, pos) {
            e.pronunciation.iter().map(|x| x.clone()).collect()
        } else {
            vec![]
        }
    }

    fn add_pronunciation(&mut self, lemma : &str, pos : &PosKey, pronunciation : Pronunciation) -> Result<()> {
        self.update_entry(lemma, pos, |e| {
            if !e.pronunciation.iter().any(|x| *x == pronunciation) {
                e.pronunciation.push(pronunciation);
            }
        })
    }

    fn get_sense<'a>(&'a self, lemma : &str, 
                         synset_id : &SynsetId) -> Vec<Cow<'a, Sense>> {
        
        let mut senses = Vec::new();
        for e in self.entry_by_lemma(lemma) {
            match e {
                Cow::Borrowed(e) => {
                    for s in e.sense.iter() {
                        if s.synset == *synset_id {
                            senses.push(Cow::Borrowed(s));
                        }
                    }
                },
                Cow::Owned(e) => {
                    for s in e.sense.iter() {
                        if s.synset == *synset_id {
                            senses.push(Cow::Owned(s.clone()));
                        }
                    }
                }
            }
        }
        senses
    }


    fn update_sense_key(&mut self, lemma : &str, key : &PosKey,
                        old_key : &SenseId, new_key : &SenseId) -> Result<()> {
        self.update_entry(lemma, key, |entry| {
            for sense in entry.sense.iter_mut() {
                if sense.id == *old_key {
                    sense.id = new_key.clone();
                }
            }
        })
    }

    fn entries<'a>(&'a self) -> impl Iterator<Item=(String, PosKey, Cow<'a, Entry>)>;

    fn into_entries(self) -> impl Iterator<Item=(String, PosKey, Entry)>;
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct BTEntries(pub(crate) BTreeMap<String, BTreeMap<PosKey, Entry>>);

impl BTEntries {
    pub(crate) fn new() -> BTEntries {
        BTEntries(BTreeMap::new())
    }
}

impl Entries for BTEntries {
    fn entry<'a>(&'a self, lemma : &str, pos_key : &PosKey) -> Option<Cow<'a, Entry>> {
        if let Some(by_pos) = self.0.get(lemma) {
            by_pos.get(pos_key).map(Cow::Borrowed)
        } else {
            None
        }
    }

    fn update_entry<X>(&mut self, lemma : &str, pos_key : &PosKey,
        f : impl FnOnce(&mut Entry) -> X) -> Result<X> {
        if let Some(by_pos) = self.0.get_mut(lemma) {
            if let Some(entry) = by_pos.get_mut(pos_key) {
                Ok(f(entry))
            } else {
                Err(LexiconError::EntryNotFound(lemma.to_string(), pos_key.clone()))
            }
        } else {
            Err(LexiconError::EntryNotFound(lemma.to_string(), pos_key.clone()))
        }
    }

    //fn iter(&self) -> impl Iterator<Item=(&String, Vec<(&PosKey, &Entry)>)> {
    //    self.0.iter().map(|(k, v)| {
    //        (k, v.iter().collect())
    //    })
    //}


    fn entry_by_lemma<'a>(&'a self, lemma : &str) -> Vec<Cow<'a, Entry>> {
        self.0.get(lemma).iter().flat_map(|x| x.values().map(|y| Cow::Borrowed(y))).collect()
    }

    fn entry_by_lemma_with_pos<'a>(&'a self, lemma : &str) -> Vec<(PosKey, Cow<'a, Entry>)> {
        self.0.get(lemma).iter().flat_map(|x| x.iter()
            .map(|(k,v)| (k.clone(), Cow::Borrowed(v)))).collect()
    }

    fn entry_by_lemma_ignore_case<'a>(&'a self, lemma : &str) -> Vec<Cow<'a, Entry>> {
        self.0.iter().filter(|(k,_)| k.to_lowercase() == lemma.to_lowercase()).
            flat_map(|(_,v)| v.values().map(Cow::Borrowed)).collect()
    }

    fn insert_entry(&mut self, lemma : String, pos : PosKey, entry : Entry) {
        self.0.entry(lemma).
            or_insert_with(|| BTreeMap::new()).insert(pos, entry);
    }

    fn remove_entry(&mut self, lemma : &str, pos : &PosKey) -> Option<Entry> {
        if let Some(by_pos) = self.0.get_mut(lemma) {
            let result = by_pos.remove(pos);
            if by_pos.is_empty() {
               self.0.remove(lemma);
            }
            result
        } else {
            None
        }
    }

    fn n_entries(&self) -> usize {
        self.0.values().map(|v| v.len()).sum()
    }
    fn entries<'a>(&'a self) -> impl Iterator<Item=(String, PosKey, Cow<'a, Entry>)> {
        self.0.iter().flat_map(|(s, inner_map)| {
            // Move 's' (String) into the inner closure so it can be paired with each entry
            inner_map.into_iter().map(move |(p, e)| {
                (s.clone(), p.clone(), Cow::Borrowed(e))
            })
        })
    } 

    #[allow(unused)]
    fn into_entries(self) -> impl Iterator<Item=(String, PosKey, Entry)> {
        // self.0 accesses the BTreeMap inside the tuple struct
        self.0.into_iter().flat_map(|(s, inner_map)| {
            // Move 's' (String) into the inner closure so it can be paired with each entry
            inner_map.into_iter().map(move |(p, e)| {
                (s.clone(), p, e)
            })
        })
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
    pub pronunciation : Vec<Pronunciation>
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

    #[allow(unused)]
    pub(crate) fn into_senses(self) -> Vec<Sense> {
        self.sense
    }
}
