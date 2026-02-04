use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::fs::File;
use crate::rels::{SenseRelType,SynsetRelType};
use crate::wordnet::util::LexiconSaveError;
use indicatif::ProgressBar;
use crate::wordnet::*;
use crate::wordnet::entry::BTEntries;
use std::borrow::Cow;
use std::result;

pub type Result<T> = result::Result<T, LexiconError>;

pub trait Lexicon : Sized {
    type E : Entries + Clone;
    type S : Synsets + Clone;
    // Data access methods
    fn entries_get<'a>(&'a self, key : char) -> Result<Option<Cow<'a, Self::E>>>;
    fn entries_insert(&mut self, key : char, entries : BTEntries) -> Result<()>;
    fn entries_iter<'a>(&'a self) -> Result<impl Iterator<Item=Result<(char, Cow<'a, Self::E>)>>>;
    fn entries_update<X>(&mut self, key : char, f : impl FnOnce(&mut Self::E) -> X) -> Result<X>;
    fn synsets_get<'a>(&'a self, lexname : &str) -> Result<Option<Cow<'a, Self::S>>>;
    fn synsets_insert(&mut self, lexname : String, synsets : BTSynsets) -> Result<()>;
    fn synsets_iter<'a>(&'a self) -> Result<impl Iterator<Item=Result<(&'a String, Cow<'a, Self::S>)>>>;
    fn synsets_contains_key(&self, lexname : &str) -> Result<bool> {
        Ok(self.synsets_get(lexname)?.is_some())
    }
    fn synsets_insert_synset(&mut self, lexname : &str, synset_id : SynsetId, synset : Synset) -> Result<()>;
    fn synsets_remove_synset(&mut self, lexname : &str,  synset_id : &SynsetId) -> Result<Option<(SynsetId, Synset)>>;
    fn synset_id_to_lexfile_get<'a>(&'a self, synset_id : &SynsetId) -> Result<Option<Cow<'a, String>>>;
    fn synset_id_to_lexfile_insert(&mut self, synset_id : SynsetId, lexfile : String) -> Result<()>;
    fn sense_links_to_get<'a>(&'a self, sense_id : &SenseId) -> Result<Option<Cow<'a, Vec<(SenseRelType, SenseId)>>>>;
    fn sense_links_to_get_or(&mut self, sense_id : SenseId, f : impl FnOnce() -> Vec<(SenseRelType, SenseId)>) 
        -> Result<Vec<(SenseRelType, SenseId)>>;
    fn sense_links_to_update(&mut self, sense_id : &SenseId, f : impl FnOnce(&mut Vec<(SenseRelType, SenseId)>)) -> Result<()>;
    fn sense_links_to_push(&mut self, sense_id : SenseId, rel : SenseRelType, target : SenseId) -> Result<()>;
    fn set_sense_links_to(&mut self, links_to : HashMap<SenseId, Vec<(SenseRelType, SenseId)>>) -> Result<()>;
    fn links_to_get<'a>(&'a self, synset_id : &SynsetId) -> Result<Option<Cow<'a, Vec<(SynsetRelType, SynsetId)>>>>;
    fn links_to_get_or(&mut self, synset_id : SynsetId, f : impl FnOnce() -> Vec<(SynsetRelType, SynsetId)>) -> Result<Vec<(SynsetRelType, SynsetId)>>;
    fn links_to_update(&mut self, synset_id : &SynsetId, f : impl FnOnce(&mut Vec<(SynsetRelType, SynsetId)>)) -> Result<()>;
    fn links_to_push(&mut self, synset_id : SynsetId, rel : SynsetRelType, target : SynsetId) -> Result<()>;
    fn set_links_to(&mut self, links_to : HashMap<SynsetId, Vec<(SynsetRelType, SynsetId)>>) -> Result<()>;
    fn sense_id_to_lemma_pos_get(&self, sense_id : &SenseId) -> Result<Option<(String, PosKey)>>;
    fn sense_id_to_lemma_pos_insert(&mut self, sense_id : SenseId, lemma_pos : (String, PosKey)) -> Result<()>;
    fn deprecations_get<'a>(&'a self) -> Result<Cow<'a, Vec<DeprecationRecord>>>;
    fn deprecations_push(&mut self, record : DeprecationRecord) -> Result<()>;

    /// Load a lexicon from a folder of YAML files
    fn load<P: AsRef<Path>>(mut self, folder : P) -> result::Result<Self, WordNetYAMLIOError> {
        let dep_file = folder.as_ref().join("../deprecations.csv");
        if dep_file.exists() {
            let mut reader = csv::Reader::from_path(dep_file)
                .map_err(|e| WordNetYAMLIOError::Csv(format!("Error reading deprecations due to {}", e)))?;
            for r in reader.deserialize::<DeprecationRecord>() {
                self.deprecations_push(r.map_err(|e| {
                    WordNetYAMLIOError::Csv(format!("Error reading deprecations due to {}", e))
                })?)?;
            }
        } 
        let folder_files = fs::read_dir(folder)
            .map_err(|e| WordNetYAMLIOError::Io(format!("Could not list directory: {}", e)))?;
        println!("Loading WordNet");
        let bar = ProgressBar::new(73);
        for file in folder_files {
            let file = file.map_err(|e|
                WordNetYAMLIOError::Io(format!("Could not list directory: {}", e)))?;
            let file_name = file.path().file_name().
                and_then(|x| x.to_str()).
                map(|x| x.to_string()).
                unwrap_or_else(|| "".to_string());
            if file_name.starts_with("entries-") && file_name.ends_with(".yaml") {
                let key = file_name[8..9].chars().into_iter().next().expect("Unreachable as file_name must be at least 10 chars long");
                let entries2 : BTEntries =
                    serde_yaml::from_reader(File::open(file.path())
                        .map_err(|e| WordNetYAMLIOError::Io(format!("Error reading {} due to {}", file_name, e)))?)
                        .map_err(|e| WordNetYAMLIOError::Serde(format!("Error reading {} due to {}", file_name, e)))?;
                for (lemma, map) in entries2.0.iter() {
                    for (pos, entry) in map.iter() {
                        for sense in entry.sense.iter() {
                            self.sense_id_to_lemma_pos_insert(sense.id.clone(),
                                (lemma.to_string(), pos.clone()))?;
                        }
                    }
                }

                self.entries_insert(key, entries2)?;
            } else if file_name.ends_with(".yaml") && file_name != "frames.yaml" {
                let synsets2 : BTSynsets = serde_yaml::from_reader(
                    File::open(file.path())
                        .map_err(|e| WordNetYAMLIOError::Io(format!("Error reading {} due to {}", file_name, e)))?)
                        .map_err(|e| WordNetYAMLIOError::Serde(format!("Error reading {} due to {}", file_name, e)))?;
                let lexname = file_name[0..file_name.len()-5].to_string();
                for id in synsets2.0.keys() {
                    self.synset_id_to_lexfile_insert(id.clone(), lexname.clone())?;
                }
                self.synsets_insert(lexname, synsets2)?;
            }
            bar.inc(1);
        }
        // Potentially ineffecient and we should try to reimplement it at some point
        let mut sense_links_to = HashMap::new();
        for es in self.entries_iter()? {
            let (_, es) = es?;
            for e in es.entries()? {
                let (_, _, e) = e?;
                for sense in e.sense.iter() {
                    for (rel_type, target) in sense.sense_links_from() {
                        sense_links_to.entry(target.clone())
                            .or_insert_with(Vec::new)
                            .push((rel_type, sense.id.clone()));
                    }
                }
            }
        }
        self.set_sense_links_to(sense_links_to)?;
        let mut links_to = HashMap::new();
        for ss in self.synsets_iter()? {
            let (_, ss) = ss?;
            for s in ss.iter()? {
                let (ssid, s) = s?;
                for (rel_type, target) in s.links_from() {
                    links_to.entry(target.clone())
                        .or_insert_with(Vec::new)
                        .push((rel_type, ssid.clone()));
                }
            }
        }
        self.set_links_to(links_to)?;
        bar.finish();
        Ok(self)
    }

    /// Save a lexicon to a set of files
    fn save<P: AsRef<Path>>(&self, folder : P) -> result::Result<(), LexiconSaveError> {
        println!("Saving WordNet");
        let bar = ProgressBar::new(73);
        for entries in self.entries_iter()? {
            let (ekey, entries) = entries?;
            let mut w = File::create(folder.as_ref().join(
                format!("entries-{}.yaml", ekey)))?;
            entries.save(&mut w)?;
            bar.inc(1);
        }
        for synsets in self.synsets_iter()? {
            let (skey, synsets) = synsets?;
            let mut w = File::create(folder.as_ref().join(
                format!("{}.yaml", skey)))?;
            synsets.save(&mut w)?;
            bar.inc(1);
        }
        match csv::WriterBuilder::new()
            .quote_style(csv::QuoteStyle::Always)
            .from_path(folder.as_ref().join("../deprecations.csv")) {
            Ok(mut csv_writer) => {
                csv_writer.serialize(DeprecationRecord("ID".to_string(),
                    "ILI".to_string(), "SUPERSEDED_BY".to_string(),
                    "SUPERSEDING_ILI".to_string(), "REASON".to_string()))
                        .unwrap_or_else(|_| eprintln!("Cannot write CSV"));
                for dep in self.deprecations_get().iter() {
                    csv_writer.serialize(dep).unwrap_or_else(|_| eprintln!("Cannot write CSV file"));
                }
            },
            Err(_) => {
                eprintln!("Cannot write CSV file");
            }
        }
        bar.finish();
        Ok(())
    }

    /// Get the lexicographer file name for a synset
    fn lex_name_for(&self, synset_id : &SynsetId) -> Result<Option<String>> {
        Ok(self.synset_id_to_lexfile_get(synset_id)?.map(|x| x.into_owned()))
    }

    /// Get the entry data for a lemma
    fn entry_by_lemma<'a>(&'a self, lemma : &str) -> Result<Vec<Cow<'a, Entry>>> {
        if lemma.is_empty() {
            return Ok(Vec::new());
        }
        Ok(match self.entries_get(entry_key(lemma))? {
            Some(Cow::Borrowed(v)) => v.entry_by_lemma(lemma)?,
            Some(Cow::Owned(v)) => v.entry_by_lemma(lemma)?.into_iter().map(|e| Cow::Owned(e.into_owned())).collect(),
            _ => Vec::new()
        })
    }

    /// Get the entry data for a lemma, ignoring case 
    fn entry_by_lemma_ignore_case<'a>(&'a self, lemma : &str) -> Result<Vec<Cow<'a, Entry>>> {
        Ok(self.entries_iter()?.map(|v| match v {
                Ok((_, Cow::Borrowed(v))) => Ok(v.entry_by_lemma_ignore_case(lemma)?),
                Ok((_, Cow::Owned(v))) => Ok(v.entry_by_lemma_ignore_case(lemma)?.
                    into_iter().map(|e| Cow::Owned(e.into_owned())).collect()),
                _ => Ok(Vec::new())
            })
            .collect::<Result<Vec<Vec<Cow<'a, Entry>>>>>()?
            .into_iter()
            .flatten()
            .collect())
    }

    /// Get the entry data (with the part of speech key) for a lemma
    fn entry_by_lemma_with_pos<'a>(&'a self, lemma : &str) -> Result<Vec<(PosKey, Cow<'a, Entry>)>> {
        match lemma.chars().nth(0) {
            Some(c) if c.to_ascii_lowercase() >= 'a' && c.to_ascii_lowercase() <= 'z' => {
                let key = c.to_ascii_lowercase();
                Ok(match self.entries_get(key)? {
                    Some(Cow::Borrowed(v)) => v.entry_by_lemma_with_pos(lemma)?,
                    Some(Cow::Owned(v)) => v.entry_by_lemma_with_pos(lemma)?
                        .into_iter().map(|(p,e)| (p, Cow::Owned(e.into_owned()))).collect(),
                    _ => {
                        Vec::new()
                    }
                })
            },
            Some(_) => {
                Ok(match self.entries_get('0')? {
                    Some(Cow::Borrowed(v)) => v.entry_by_lemma_with_pos(lemma)?,
                    Some(Cow::Owned(v)) => v.entry_by_lemma_with_pos(lemma)?
                        .into_iter().map(|(p,e)| (p, Cow::Owned(e.into_owned()))).collect(),
                    _ => Vec::new()
                })
            },
            None => {
                Ok(Vec::new())
            }
        }
    }

    /// Get the sense by lemma and synset id
    fn get_sense<'a>(&'a self, lemma : &str, synset_id : &SynsetId) -> Result<Vec<Cow<'a, Sense>>> {
        Ok(match self.entries_get(entry_key(&lemma))? {
            Some(Cow::Borrowed(entries)) => entries.get_sense(lemma, synset_id)?,
            Some(Cow::Owned(entries)) => entries.get_sense(lemma, synset_id)?
                .into_iter().map(|s| Cow::Owned(s.into_owned())).collect(),
            None => Vec::new()
        })
    }

    /// Get the sense by its sense identifier
    #[allow(unused)]
    fn get_sense_by_id<'a>(&'a self, sense_id : &SenseId) -> Result<Option<(String, PosKey, Cow<'a, Sense>)>> {
        if let Some((lemma, pos)) = self.sense_id_to_lemma_pos_get(sense_id)? {
            for (pos2, e) in self.entry_by_lemma_with_pos(&lemma)? {
                if pos == pos2 {
                    match e {
                        Cow::Borrowed(e) => {
                            for sense in e.sense.iter() {
                                if &sense.id == sense_id {
                                    return Ok(Some((lemma.clone(), pos.clone(), Cow::Borrowed(sense))))
                                }
                            }
                        },
                        Cow::Owned(e) => {
                            for sense in e.into_senses() {
                                if &sense.id == sense_id {
                                    return Ok(Some((lemma.clone(), pos.clone(), Cow::Owned(sense))))
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(None)
    }

    /// Get the part of speech key for an entry referring to a specific synset
    fn pos_for_entry_synset(&self, lemma : &str, synset_id : &SynsetId) -> Result<Option<PosKey>> {
        for (pos, entry) in self.entry_by_lemma_with_pos(lemma)? {
            for sense in entry.sense.iter() {
                if sense.synset == *synset_id {
                    return Ok(Some(pos.clone()));
                }
            }
        }
        return Ok(None);
    }

    /// Get synset data by ID
    fn synset_by_id<'a>(&'a self, synset_id : &SynsetId) -> Result<Option<Cow<'a, Synset>>> {
        Ok(match self.lex_name_for(synset_id)? {
            Some(lex_name) => {
                match self.synsets_get(&lex_name)? {
                    Some(Cow::Borrowed(sss)) => {
                        sss.get(synset_id)?
                    },
                    Some(Cow::Owned(sss)) => {
                        sss.get(synset_id)?.map(|s| Cow::Owned(s.into_owned()))
                    },
                    None => None
                }
            },
            None => None
        })
    }

    /// Update synset data by ID 
    fn update_synset(&mut self, synset_id : &SynsetId, f : impl FnOnce(&mut Synset)) -> Result<()>;

    /// Get synset data by ID (mutable)
    //fn synset_by_id_mut(&mut self, synset_id : &SynsetId) -> Option<&mut Synset> {
    //    panic!("TODO")
        //match self.lex_name_for(synset_id) {
        //    Some(lex_name) => {
        //        match self.synsets_get_mut(&lex_name) {
        //            Some(sss) => {
        //                sss.0.get_mut(synset_id)
        //            },
        //            None => None
        //        }
        //    },
        //    None => None
        //}
    //}

    ///// Verifies if a synset is in the graph
    //fn has_synset(&self, synset_id : &SynsetId) -> bool {
    //    self.synset_by_id(synset_id).is_some()
    //}

    /// Verifies if a sense is in the graph
    fn has_sense(&self, sense_id : &SenseId) -> Result<bool> {
        Ok(self.sense_id_to_lemma_pos_get(sense_id)?.is_some())
    }

    /// Get the list of lemmas associated with a synset
    fn members_by_id(&self, synset_id : &SynsetId) -> Result<Vec<String>> {
        Ok(self.synset_by_id(synset_id)?.iter().flat_map(|synset|
            synset.members.iter().map(|x| x.clone())).collect())
    }

    /// Add an entry to WordNet
    fn insert_entry(&mut self, lemma : String, pos : PosKey, entry : Entry) -> Result<()> {
        add_sense_link_to(self, &entry)?;
        for sense in entry.sense.iter() {
            self.sense_id_to_lemma_pos_insert(sense.id.clone(), (lemma.clone(), pos.clone()))?;
        }
        self.entries_update(entry_key(&lemma), |e| {
            e.insert_entry(lemma, pos, entry)
        })??;
        Ok(())
    }

    /// Add a synset to WordNet
    fn insert_synset(&mut self, lexname : String, synset_id : SynsetId,
                         synset : Synset) -> Result<()> {
        add_link_to(self, &synset_id, &synset)?;
        self.synset_id_to_lexfile_insert(synset_id.clone(), lexname.clone())?;
        self.synsets_insert_synset(&lexname, synset_id, synset)?;
        Ok(())
    }

    /// Add a sense to an existing entry. This will not create an entry if it does not exist
    fn insert_sense(&mut self, lemma : String, pos : PosKey, sense : Sense) -> Result<()> {
        add_sense_link_to_sense(self, &sense)?;
        self.sense_id_to_lemma_pos_insert(sense.id.clone(), (lemma.clone(), pos.clone()))?;
        self.entries_update(entry_key(&lemma), |e| {
            e.insert_sense(lemma, pos, sense)
        })??;
        Ok(())
    }

    ///// Remove an entry from WordNet
    //fn remove_entry(&mut self, lemma : &str, pos : &PosKey) {
    //    match self.entries.get_mut(&entry_key(lemma)) {
    //        Some(e) => e.remove_entry(&mut self.sense_links_to, lemma, pos),
    //        None => {}
    //    }
    //}

    /// Remove the sense of an existing entry. This does not remove incoming sense links!
    fn remove_sense(&mut self, lemma : &str, pos : &PosKey, 
                        synset_id : &SynsetId) -> Result<Vec<SenseId>> {
        let v = self.sense_links_from(lemma, pos, synset_id)?;
        let mut keys  : Vec<SenseId> = Vec::new();
        self.entries_update(entry_key(lemma), |e| {
                Ok::<(),LexiconError>(keys.extend(e.remove_sense(lemma, pos, synset_id)?)) })??;
        for source in keys.iter() {
            for (rel, target) in v.iter() {
                self.sense_links_to_update(target, |key| {
                    key.retain(|x| x.0 != *rel && 
                        x.1 != *source);
                })?;
            }
        }
        Ok(keys)
    }

    /// Remove a synset. This does not remove any senses or incoming links!
    fn remove_synset(&mut self, synset_id : &SynsetId) -> Result<()> {
        match self.lex_name_for(synset_id)? {
            Some(lexname) => {
                if let Some((_, ss)) = self.synsets_remove_synset(&lexname, synset_id)? {
                    remove_link_to(self, synset_id, &ss)?;
                }
            },
            None => {}
        }
        Ok(())
    }

    /// For a given sense, get all links from this sense
    fn sense_links_from(&self, lemma : &str, pos : &PosKey, 
                            synset_id : &SynsetId) -> Result<Vec<(SenseRelType, SenseId)>> {
        Ok(match self.entries_get(entry_key(lemma))? {
            Some(e) => e.sense_links_from(lemma, pos, synset_id)?,
            None => Vec::new()
        })
    }

    /// For a given sense, find all backlinks referring to this sense
    fn sense_links_to(&self, lemma : &str, pos : &PosKey,
                          synset_id : &SynsetId) -> Result<Vec<(SenseRelType, SenseId)>> {
        Ok(match self.get_sense_id(lemma, pos, synset_id)? {
            Some(sense_id) => {
                match self.sense_links_to_get(&sense_id)? {
                    Some(v) => v.into_owned(),
                    None => Vec::new()
                }
            },
            None => Vec::new()
        })
    }
    
    /// For a given sense, get all links from this sense
    fn sense_links_from_id(&self, sense_id : &SenseId) 
                        -> Result<Vec<(SenseRelType, SenseId)>> {
        Ok(match self.sense_id_to_lemma_pos_get(sense_id)? {
            Some((lemma, pos)) => {
                match self.entries_get(entry_key(&lemma))? {
                    Some(e) => e.sense_links_from_id(&lemma, &pos, sense_id)?,
                    None => Vec::new()
                }
            },
            None => Vec::new()
        })
    }

    /// For a given synset, find all sense links to and from all senses of this synset
    fn all_sense_links(&self, synset_id : &SynsetId) -> Result<Vec<(SenseId, SenseRelType, SenseId)>> {
        let mut links = Vec::new();
        for member in self.members_by_id(synset_id)? {
            for sense in self.get_sense(&member, synset_id)? {
                for (sense_rel_type, target) in sense.sense_links_from() {
                    links.push((sense.id.clone(), sense_rel_type, target));
                }
                if let Some(links_to) = self.sense_links_to_get(&sense.id)? {
                    for (sense_rel_type, source) in links_to.iter() {
                        links.push((source.clone(), sense_rel_type.clone(), sense.id.clone()));
                    }
                }
            }
        }
        Ok(links)
    }


    ///// For a given sense, find all backlinks referring to this sense
    //fn sense_links_to_id(&self, sense_id : &SenseId) -> 
    //    Vec<(SenseRelType, SenseId)> {
    //        match self.sense_links_to.get(sense_id) {
    //            Some(v) => v.clone(),
    //            None => Vec::new()
    //    }
    //}


    /// For a synset, find all backlinks referring to this synset
    fn links_to(&self, synset_id : &SynsetId) -> Result<Vec<(SynsetRelType, SynsetId)>> {
        Ok(match self.links_to_get(synset_id)? {
            Some(s) => s.into_owned(),
            None => Vec::new()
        })
    }

    /// For a synset, find all links from this synset
    fn links_from(&self, synset_id : &SynsetId) -> Result<Vec<(SynsetRelType, SynsetId)>> {
        Ok(match self.synset_by_id(synset_id)? {
            Some(ss) => ss.links_from(),
            None => Vec::new()
        })
    }

    /// Get a sense ID for a lemma, POS key and synset
    fn get_sense_id<'a>(&'a self, lemma : &str, pos : &PosKey, synset_id : &SynsetId) -> 
        Result<Option<SenseId>> {
        Ok(match self.entries_get(entry_key(lemma))? {
            Some(e) => e.get_sense_id(lemma, pos, synset_id)?,
            None => None
        })
    }

    // Get a sense ID for a lemma and synset
    fn get_sense_id2<'a>(&'a self, lemma : &str, synset_id : &SynsetId) -> 
        Result<Option<SenseId>> {
        Ok(match self.entries_get(entry_key(lemma))? {
            Some(e) => e.get_sense_id2(lemma, synset_id)?,
            None => None
        })
    }

    /// Add a relation between two senses
    fn add_sense_rel(&mut self, source : &SenseId, rel : SenseRelType,
                   target : &SenseId) -> Result<()> {
        if let Some(inv) = rel.inverse() {
            self.add_sense_rel(target, inv, source)?;
        } else {
            self.sense_links_to_get_or(target.clone(), || Vec::new())?.
                push((rel.clone(), source.clone()));
            let mut lemma_pos = None;
            match self.sense_id_to_lemma_pos_get(source)? {
                Some((lemma, pos)) => {
                    lemma_pos = Some((lemma.clone(), pos.clone()));
                }
                None => {
                    eprintln!("Could not map sense id to lemma, pos")
                }
                
            }
            match lemma_pos {
                Some((lemma, pos)) => {
                    self.entries_update(entry_key(&lemma), |e| {
                        e.add_rel(&lemma, &pos, source, rel, target)
                    })??;
                },
                None => {}
            }
        }
        Ok(())
    }

    /// Remove all links between two senses
    fn remove_sense_rel(&mut self, source : &SenseId, 
                      target : &SenseId) -> Result<()> {
        self.sense_links_to_update(target, |v| {
            v.retain(|x| x.1 != *source);
        })?;
        let mut lemma_pos = None;
        match self.sense_id_to_lemma_pos_get(source)? {
            Some((lemma, pos)) => {
                lemma_pos = Some((lemma.clone(), pos.clone()));
            },
            None => {
                eprintln!("Could not map sense id to lemma, pos")
            }
        }
        match lemma_pos {
            Some ((lemma, pos)) => {
                self.entries_update(entry_key(&lemma), |e| {
                    e.remove_rel(&lemma, &pos, source, target)
                })??;
            },
            None => {}
        }
        Ok(())
    }

    /// Add a synset relation to WordNet
    fn add_rel(&mut self, source : &SynsetId, rel : SynsetRelType,
                   target : &SynsetId) -> Result<()> {
        self.links_to_get_or(target.clone(), || Vec::new())?.
            push((rel.clone(), source.clone()));
        let (s2t, rel) = rel.to_yaml();
        if s2t {
            self.update_synset(source, |ss| {
                ss.insert_rel(&rel, target);
            })?;
        } else {
            self.update_synset(source, |ss| {
                ss.insert_rel(&rel, source);
            })?;
        }
        Ok(())
    }

    /// Remove all links between two synsets
    fn remove_rel(&mut self, source : &SynsetId, target : &SynsetId) -> Result<()> {
        self.links_to_update(target, |v| {
            v.retain(|x| x.1 != *source);
        })?;
        self.update_synset(source, |ss| {
            ss.remove_rel(target);
        })?;
        Ok(())
    }

    /// Get the list of variant forms of an entry
    fn get_forms(&self, lemma : &str, pos : &PosKey) -> Result<Vec<String>> {
        Ok(match self.entries_get(entry_key(&lemma))? {
            Some(e) => e.get_forms(lemma, pos)?,
            None => Vec::new()
        })
    }

    /// Add a variant form to an entry
    fn add_form(&mut self, lemma : &str, pos : &PosKey, form : String) -> Result<()> {
        self.entries_update(entry_key(&lemma), |e| {
            e.add_form(lemma, pos, form)
        })??;
        Ok(())
    }

    /// Get the list of pronunications of an entry
    fn get_pronunciations(&self, lemma : &str, pos : &PosKey) -> Result<Vec<Pronunciation>> {
        Ok(match self.entries_get(entry_key(&lemma))? {
            Some(e) => e.get_pronunciations(lemma, pos)?,
            None => Vec::new()
        })
    }

    /// Add a pronunciation to an entry
    fn add_pronunciation(&mut self, lemma : &str, pos : &PosKey, pronunciation : Pronunciation) -> Result<()> {
        self.entries_update(entry_key(&lemma), |e| {
            e.add_pronunciation(lemma, pos, pronunciation)
        })??;
        Ok(())
    }

    /// Add a deprecation note
    fn deprecate(&mut self, synset : &SynsetId, supersede : &SynsetId, 
                     reason : String) -> Result<()> {
        let ili = match self.synset_by_id(synset)? {
            Some(ss) => match ss.ili {
                Some(ref ili) => ili.as_str().to_string(),
                None => String::new()
            },
            None => String::new()
        };
        let supersede_ili = match self.synset_by_id(supersede)? {
            Some(ss) => match ss.ili {
                Some(ref ili) => ili.as_str().to_string(),
                None => String::new()
            },
            None => String::new()
        };
        self.deprecations_push(DeprecationRecord(
            format!("ewn-{}", synset.as_str()),
            ili,
            format!("ewn-{}", supersede.as_str()),
            supersede_ili,
            reason))?;
        Ok(())
    }

    fn update_sense_key(&mut self, old_key : &SenseId, new_key : &SenseId) -> Result<()> {
        let mut lemma_pos = None;
        match self.sense_id_to_lemma_pos_get(old_key)? {
            Some((lemma, pos)) => {
                lemma_pos = Some((lemma.clone(), pos.clone()));
            },
            None => {}
        };
        if let Some((lemma, pos)) = lemma_pos {
            self.entries_update(entry_key(&lemma), |e| {
                e.update_sense_key(&lemma, &pos, old_key, new_key)
            })??;
        }
        match self.sense_links_to_get(old_key)?.map(|x| x.clone()) {
            Some(links_to) => {
                for (rel, source) in links_to.into_owned() {
                    self.remove_sense_rel(&source, old_key)?;
                    self.add_sense_rel(&source, rel.clone(), old_key)?;
                }
            },
            None => {}
        }
        Ok(())
    }

    /// Get all the entries
    fn entries<'a>(&'a self) -> Result<impl Iterator<Item=Result<(String, PosKey, Cow<'a, Entry>)>>> {
        Ok(self.entries_iter()?.flat_map(|e| {
            match e {
                Ok((_, e)) => {
                    let it = match e {
                        Cow::Borrowed(v) => {
                            match v.entries() {
                                Ok(e) => Box::new(e),
                                Err(e) => Box::new(vec![Err(e)].into_iter()) as Box<dyn Iterator<Item = _>>
                            }
                        },
                        Cow::Owned(v) => {
                            match v.into_entries() {
                                Ok(e) => Box::new(e.into_iter()
                                .map(|r| {
                                    match r {
                                        Ok((s, p, e)) => Ok((s, p, Cow::Owned(e))),
                                        Err(e) => Err(e)
                                    }
                                })) as Box<dyn Iterator<Item = _>>,
                                Err(e) => {
                                    Box::new(vec![Err(e)].into_iter()) as Box<dyn Iterator<Item = _>>
                                }
                            }
                        }
                    };
                    it
                },
                Err(e) => { Box::new(vec![Err(e)].into_iter()) as Box<dyn Iterator<Item = _>> }
            }
        }))
    }

    /// Get all synsets
    fn synsets<'a>(&'a self) -> Result<impl Iterator<Item=Result<(SynsetId, Cow<'a, Synset>)>>> {
        Ok(self.synsets_iter()?.flat_map(|e| {
            match e {
                Ok((_, e)) => {
                    let it = match e {
                        Cow::Borrowed(v) => {
                            match v.iter() {
                                Ok(it) => Box::new(it),
                                Err(e) => Box::new(vec![Err(e)].into_iter()) as Box<dyn Iterator<Item = _>>
                            }
                        },
                        Cow::Owned(v) => {
                            match v.into_iter() {
                                Ok(it) => Box::new(it
                                .map(|s| {
                                    match s {
                                        Ok((sid, ss)) => Ok((sid, Cow::Owned(ss))),
                                        Err(e) => Err(e)
                                    }
                                })) as Box<dyn Iterator<Item = _>>,
                                Err(e) => {
                                    Box::new(vec![Err(e)].into_iter()) as Box<dyn Iterator<Item = _>>
                                }
                            }
                        }
                    };
                    it
                },
                Err(e) => { Box::new(vec![Err(e)].into_iter()) as Box<dyn Iterator<Item = _>> }
            }
        }))
    }
                
    /// Get the part of speech from a lexicographer file
    fn pos_for_lexfile(&self, lexfile : &str) -> Result<Vec<PartOfSpeech>> {
        Ok(if self.synsets_contains_key(lexfile)? {
            if lexfile.starts_with("noun") {
                vec![PartOfSpeech::n]
            } else if lexfile.starts_with("verb") {
                vec![PartOfSpeech::v]
            } else if lexfile.starts_with("adv") {
                vec![PartOfSpeech::r]
            } else if lexfile.starts_with("adj") {
                vec![PartOfSpeech::a, PartOfSpeech::s]
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        })
    }
   /// Number of entries in the dictionary
    fn n_entries(&self) -> Result<usize> {
        let counts: Vec<usize> = self.entries_iter()?
            .map(|v| {
                let (_, entry) = v?; 
                entry.n_entries() // This returns Result<usize>
            })
        .collect::<Result<Vec<usize>>>()?; 

        Ok(counts.into_iter().sum())
    } 

    /// Number of synsets in the dictionary
    fn n_synsets(&self) -> Result<usize> {
        let counts: Vec<usize> = self.synsets_iter()?
            .map(|v| {
                let (_, synsets) = v?; 
                synsets.len() 
            })
        .collect::<Result<Vec<usize>>>()?;

        Ok(counts.into_iter().sum())
    }

    /// Get the synset augmented with the member data
    #[cfg(feature = "redb")]
    fn get_member_synset(&self, id : &SynsetId) -> Result<MemberSynset> {
        if let Some(synset) = self.synset_by_id(id)? {
            let synset = synset.into_owned();
            MemberSynset::from_synset(id, synset, self)
        } else {
           Err(LexiconError::SynsetIdNotFound(id.clone())) 
        }
    }


    //#[cfg(test)]
    //fn add_lexfile(&mut self, lexfile : &str) -> Result<()> {
    //    self.synsets_insert(lexfile.to_owned(), Synsets::new());
    //    Ok(())
    //}

}

fn add_sense_link_to<L : Lexicon>(backend : &mut L,
                     entry : &Entry) -> Result<()> {
    for sense in entry.sense.iter() {
        for (rel_type, target) in sense.sense_links_from() {
            backend.sense_links_to_push(target.clone(), rel_type, sense.id.clone())?;
        }
    }
    Ok(())
}

fn add_sense_link_to_sense<L : Lexicon>(backend : &mut L,
                           sense : &Sense) -> Result<()> {
    for (rel_type, target) in sense.sense_links_from() {
        backend.sense_links_to_push(target.clone(), rel_type, sense.id.clone())?;
    }
    Ok(())
}

pub(crate) fn add_link_to<L : Lexicon>(backend : &mut L,
               synset_id : &SynsetId, synset : &Synset) -> Result<()> {
    for (rel_type, target) in synset.links_from() {
        backend.links_to_push(target.clone(), rel_type, synset_id.clone())?;
    }
    Ok(())
}

fn remove_link_to<L : Lexicon>(backend : &mut L,
               synset_id : &SynsetId, synset : &Synset) -> Result<()> {
    for (_, target) in synset.links_from() {
        backend.links_to_update(&target, |m| {
            m.retain(|sr| sr.1 != *synset_id);
        })?;
    }
    Ok(())
}

pub(crate) fn entry_key(lemma : &str) -> char {
    let key = lemma.to_lowercase().chars().next().expect("Empty lemma!");
    if key < 'a' || key > 'z' {
        '0'
    } else {
        key
    }
}


