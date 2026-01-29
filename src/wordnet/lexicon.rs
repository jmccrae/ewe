use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::fs::File;
use crate::rels::{SenseRelType,SynsetRelType};
use indicatif::ProgressBar;
use crate::wordnet::*;
use crate::wordnet::entry::BTEntries;

pub trait Lexicon : Sized {
    type E : Entries;
    type S : Synsets;
    // Data access methods
    fn entries_get(&self, lemma : &str) -> Option<&Self::E>;
    fn entries_insert(&mut self, key : String, entries : BTEntries);
    fn entries_iter(&self) -> impl Iterator<Item=(&String, &Self::E)>;
    fn entries_update(&mut self, lemma : &str, f : impl FnOnce(&mut Self::E));
    fn synsets_get(&self, lexname : &str) -> Option<&Self::S>;
    fn synsets_insert(&mut self, lexname : String, synsets : BTSynsets);
    fn synsets_iter(&self) -> impl Iterator<Item=(&String, &Self::S)>;
    fn synsets_update<X>(&mut self, lexname : &str, f : impl FnOnce(&mut Self::S) -> X) -> X;
    fn synsets_contains_key(&self, lexname : &str) -> bool {
        self.synsets_get(lexname).is_some()
    }
    fn synset_id_to_lexfile_get(&self, synset_id : &SynsetId) -> Option<&String>;
    fn synset_id_to_lexfile_insert(&mut self, synset_id : SynsetId, lexfile : String);
    fn sense_links_to_get(&self, sense_id : &SenseId) -> Option<&Vec<(SenseRelType, SenseId)>>;
    fn sense_links_to_get_or(&mut self, sense_id : SenseId, f : impl FnOnce() -> Vec<(SenseRelType, SenseId)>) 
        -> &mut Vec<(SenseRelType, SenseId)>;
    fn sense_links_to_update(&mut self, sense_id : &SenseId, f : impl FnOnce(&mut Vec<(SenseRelType, SenseId)>));
    fn sense_links_to_push(&mut self, sense_id : SenseId, rel : SenseRelType, target : SenseId);
    fn set_sense_links_to(&mut self, links_to : HashMap<SenseId, Vec<(SenseRelType, SenseId)>>);
    fn links_to_get(&self, synset_id : &SynsetId) -> Option<&Vec<(SynsetRelType, SynsetId)>>;
    fn links_to_get_or(&mut self, synset_id : SynsetId, f : impl FnOnce() -> Vec<(SynsetRelType, SynsetId)>) 
        -> &mut Vec<(SynsetRelType, SynsetId)>;
    fn links_to_update(&mut self, synset_id : &SynsetId, f : impl FnOnce(&mut Vec<(SynsetRelType, SynsetId)>));
    fn links_to_push(&mut self, synset_id : SynsetId, rel : SynsetRelType, target : SynsetId);
    fn set_links_to(&mut self, links_to : HashMap<SynsetId, Vec<(SynsetRelType, SynsetId)>>);
    fn sense_id_to_lemma_pos_get(&self, sense_id : &SenseId) -> Option<&(String, PosKey)>;
    fn sense_id_to_lemma_pos_insert(&mut self, sense_id : SenseId, lemma_pos : (String, PosKey));
    fn deprecations_get(&self) -> &Vec<DeprecationRecord>;
    fn deprecations_push(&mut self, record : DeprecationRecord);

    /// Load a lexicon from a folder of YAML files
    fn load<P: AsRef<Path>>(mut self, folder : P) -> Result<Self, WordNetYAMLIOError> {
        let dep_file = folder.as_ref().join("../deprecations.csv");
        if dep_file.exists() {
            let mut reader = csv::Reader::from_path(dep_file)
                .map_err(|e| WordNetYAMLIOError::Csv(format!("Error reading deprecations due to {}", e)))?;
            for r in reader.deserialize::<DeprecationRecord>() {
                self.deprecations_push(r.map_err(|e| {
                    WordNetYAMLIOError::Csv(format!("Error reading deprecations due to {}", e))
                })?);
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
                let key = file_name[8..9].to_string();
                let entries2 : BTEntries =
                    serde_yaml::from_reader(File::open(file.path())
                        .map_err(|e| WordNetYAMLIOError::Io(format!("Error reading {} due to {}", file_name, e)))?)
                        .map_err(|e| WordNetYAMLIOError::Serde(format!("Error reading {} due to {}", file_name, e)))?;
                for (lemma, map) in entries2.0.iter() {
                    for (pos, entry) in map.iter() {
                        for sense in entry.sense.iter() {
                            self.sense_id_to_lemma_pos_insert(sense.id.clone(),
                                (lemma.to_string(), pos.clone()));
                        }
                    }
                }

                self.entries_insert(key, entries2);
            } else if file_name.ends_with(".yaml") && file_name != "frames.yaml" {
                let synsets2 : BTSynsets = serde_yaml::from_reader(
                    File::open(file.path())
                        .map_err(|e| WordNetYAMLIOError::Io(format!("Error reading {} due to {}", file_name, e)))?)
                        .map_err(|e| WordNetYAMLIOError::Serde(format!("Error reading {} due to {}", file_name, e)))?;
                let lexname = file_name[0..file_name.len()-5].to_string();
                for id in synsets2.0.keys() {
                    self.synset_id_to_lexfile_insert(id.clone(), lexname.clone());
                }
                self.synsets_insert(lexname, synsets2);
            }
            bar.inc(1);
        }
        // Potentially ineffecient and we should try to reimplement it at some point
        let mut sense_links_to = HashMap::new();
        for (_, es) in self.entries_iter() {
            for (_, _, e) in es.entries() {
                for sense in e.sense.iter() {
                    for (rel_type, target) in sense.sense_links_from() {
                        sense_links_to.entry(target.clone())
                            .or_insert_with(Vec::new)
                            .push((rel_type, sense.id.clone()));
                    }
                }
            }
        }
        self.set_sense_links_to(sense_links_to);
        let mut links_to = HashMap::new();
        for (_, ss) in self.synsets_iter() {
            for (ssid, s) in ss.iter() {
                for (rel_type, target) in s.links_from() {
                    links_to.entry(target.clone())
                        .or_insert_with(Vec::new)
                        .push((rel_type, ssid.clone()));
                }
            }
        }
        self.set_links_to(links_to);
        bar.finish();
        Ok(self)
    }

    /// Save a lexicon to a set of files
    fn save<P: AsRef<Path>>(&self, folder : P) -> std::io::Result<()> {
        println!("Saving WordNet");
        let bar = ProgressBar::new(73);
        for (ekey, entries) in self.entries_iter() {
            let mut w = File::create(folder.as_ref().join(
                format!("entries-{}.yaml", ekey)))?;
            entries.save(&mut w)?;
            bar.inc(1);
        }
        for (skey, synsets) in self.synsets_iter() {
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
    fn lex_name_for(&self, synset_id : &SynsetId) -> Option<String> {
        self.synset_id_to_lexfile_get(synset_id).map(|x| x.clone())
    }

    /// Get the entry data for a lemma
    fn entry_by_lemma(&self, lemma : &str) -> Vec<&Entry> {
        if lemma.is_empty() {
            return Vec::new();
        }
        match self.entries_get(&entry_key(lemma)) {
            Some(v) => v.entry_by_lemma(lemma),
            None => Vec::new()
        }
    }

    /// Get the entry data for a lemma, ignoring case 
    fn entry_by_lemma_ignore_case(&self, lemma : &str) -> Vec<&Entry> {
        self.entries_iter()
            .flat_map(|(_,v)| v.entry_by_lemma_ignore_case(lemma))
            .collect()
    }

    /// Get the entry data (with the part of speech key) for a lemma
    fn entry_by_lemma_with_pos(&self, lemma : &str) -> Vec<(&PosKey, &Entry)> {
        match lemma.chars().nth(0) {
            Some(c) if c.to_ascii_lowercase() >= 'a' && c.to_ascii_lowercase() <= 'z' => {
                let key = format!("{}", c.to_lowercase());
                match self.entries_get(&key) {
                    Some(v) => v.entry_by_lemma_with_pos(lemma),
                    None => {
                        eprintln!("No entries for {}", key);
                        Vec::new()
                    }
                }
            },
            Some(_) => {
                match self.entries_get("0") {
                    Some(v) => v.entry_by_lemma_with_pos(lemma),
                    None => Vec::new()
                }
            },
            None => {
                eprintln!("Query with empty string");
                Vec::new()
            }
        }
    }

    /// Get the sense by lemma and synset id
    fn get_sense<'a>(&'a self, lemma : &str, synset_id : &SynsetId) -> Vec<&'a Sense> {
        match self.entries_get(&entry_key(&lemma)) {
            Some(entries) => entries.get_sense(lemma, synset_id),
            None => Vec::new()
        }
    }

    /// Get the sense by its sense identifier
    fn get_sense_by_id(&self, sense_id : &SenseId) -> Option<(&String, &PosKey, &Sense)> {
        if let Some((lemma, pos)) = self.sense_id_to_lemma_pos_get(sense_id) {
            for (pos2, e) in self.entry_by_lemma_with_pos(lemma) {
                if pos == pos2 {
                    for sense in e.sense.iter() {
                        if &sense.id == sense_id {
                            return Some((lemma, pos, sense))
                        }
                    }
                }
            }
        }
        None
    }

    /// Get the part of speech key for an entry referring to a specific synset
    fn pos_for_entry_synset(&self, lemma : &str, synset_id : &SynsetId) -> Option<PosKey> {
        for (pos, entry) in self.entry_by_lemma_with_pos(lemma) {
            for sense in entry.sense.iter() {
                if sense.synset == *synset_id {
                    return Some(pos.clone());
                }
            }
        }
        return None;
    }

    /// Get synset data by ID
    fn synset_by_id(&self, synset_id : &SynsetId) -> Option<&Synset> {
        match self.lex_name_for(synset_id) {
            Some(lex_name) => {
                match self.synsets_get(&lex_name) {
                    Some(sss) => {
                        sss.get(synset_id)
                    },
                    None => None
                }
            },
            None => None
        }
    }

    /// Update synset data by ID 
    fn update_synset(&mut self, synset_id : &SynsetId, f : impl FnOnce(&mut Synset)) -> Result<(), String> {
        match self.lex_name_for(synset_id) {
            Some(lex_name) => {
                self.synsets_update(&lex_name, |sss| {
                    sss.update(synset_id, |ss| {
                        f(ss)
                    })
                })?;
                Ok(())
            },
            None => Err(format!("Synset ID {} not found", synset_id))
        }
    }

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
    fn has_sense(&self, sense_id : &SenseId) -> bool {
        self.sense_id_to_lemma_pos_get(sense_id).is_some()
    }

    /// Get the list of lemmas associated with a synset
    fn members_by_id(&self, synset_id : &SynsetId) -> Vec<String> {
        self.synset_by_id(synset_id).iter().flat_map(|synset|
            synset.members.iter().map(|x| x.clone())).collect()
    }

    /// Add an entry to WordNet
    fn insert_entry(&mut self, lemma : String, pos : PosKey, entry : Entry) {
        add_sense_link_to(self, &entry);
        for sense in entry.sense.iter() {
            self.sense_id_to_lemma_pos_insert(sense.id.clone(), (lemma.clone(), pos.clone()));
        }
        self.entries_update(&entry_key(&lemma), |e : &mut Self::E| {
            e.insert_entry(lemma, pos, entry);
        });
    }

    /// Add a synset to WordNet
    fn insert_synset(&mut self, lexname : String, synset_id : SynsetId,
                         synset : Synset) {
        add_link_to(self, &synset_id, &synset);
        self.synset_id_to_lexfile_insert(synset_id.clone(), lexname.clone());
        self.synsets_update(&lexname, |s| {
            s.insert(synset_id, synset.clone());
        });
    }

    /// Add a sense to an existing entry. This will not create an entry if it does not exist
    fn insert_sense(&mut self, lemma : String, pos : PosKey, sense : Sense) {
        add_sense_link_to_sense(self, &sense);
        self.sense_id_to_lemma_pos_insert(sense.id.clone(), (lemma.clone(), pos.clone()));
        self.entries_update(&entry_key(&lemma), |e : &mut Self::E| {
            e.insert_sense(lemma, pos, sense)
                .unwrap_or_else(|_| {
                    eprintln!("Failed to insert sense as the entry does not exist");
                });
        });
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
                        synset_id : &SynsetId) -> Vec<SenseId> {
        let v = self.sense_links_from(lemma, pos, synset_id);
        let mut keys = Vec::new();
        self.entries_update(&entry_key(lemma), |e : &mut Self::E| {
                keys.extend(e.remove_sense(lemma, pos, synset_id)) });
        for source in keys.iter() {
            for (rel, target) in v.iter() {
                self.sense_links_to_update(target, |key| {
                    key.retain(|x| x.0 != *rel && 
                        x.1 != *source);
                });
            }
        }
        keys

    }

    /// Remove a synset. This does not remove any senses or incoming links!
    fn remove_synset(&mut self, synset_id : &SynsetId) {
        let mut removed = Vec::new();
        match self.lex_name_for(synset_id) {
            Some(lexname) => {
                self.synsets_update(&lexname, |m| {
                    removed.extend(m.remove_entry(synset_id));
                });
                for (_, ss) in removed.iter() {
                    remove_link_to(self, synset_id, &ss);
                }
            },
            None => {}
        }
    }

    /// For a given sense, get all links from this sense
    fn sense_links_from(&self, lemma : &str, pos : &PosKey, 
                            synset_id : &SynsetId) -> Vec<(SenseRelType, SenseId)> {
        match self.entries_get(&entry_key(lemma)) {
            Some(e) => e.sense_links_from(lemma, pos, synset_id),
            None => Vec::new()
        }
    }

    /// For a given sense, find all backlinks referring to this sense
    fn sense_links_to(&self, lemma : &str, pos : &PosKey,
                          synset_id : &SynsetId) -> Vec<(SenseRelType, SenseId)> {
        match self.get_sense_id(lemma, pos, synset_id) {
            Some(sense_id) => {
                match self.sense_links_to_get(sense_id) {
                    Some(v) => v.clone(),
                    None => Vec::new()
                }
            },
            None => Vec::new()
        }
    }
    
    /// For a given sense, get all links from this sense
    fn sense_links_from_id(&self, sense_id : &SenseId) 
                        -> Vec<(SenseRelType, SenseId)> {
        match self.sense_id_to_lemma_pos_get(sense_id) {
            Some((lemma, pos)) => {
                match self.entries_get(&entry_key(lemma)) {
                    Some(e) => e.sense_links_from_id(lemma, pos, sense_id),
                    None => Vec::new()
                }
            },
            None => Vec::new()
        }
    }

    /// For a given synset, find all sense links to and from all senses of this synset
    fn all_sense_links(&self, synset_id : &SynsetId) -> Vec<(SenseId, SenseRelType, SenseId)> {
        let mut links = Vec::new();
        for member in self.members_by_id(synset_id) {
            for sense in self.get_sense(&member, synset_id) {
                for (sense_rel_type, target) in sense.sense_links_from() {
                    links.push((sense.id.clone(), sense_rel_type, target));
                }
                if let Some(links_to) = self.sense_links_to_get(&sense.id) {
                    for (sense_rel_type, source) in links_to.iter() {
                        links.push((source.clone(), sense_rel_type.clone(), sense.id.clone()));
                    }
                }
            }
        }
        links
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
    fn links_to(&self, synset_id : &SynsetId) -> Vec<(SynsetRelType, SynsetId)> {
        match self.links_to_get(synset_id) {
            Some(s) => s.clone(),
            None => Vec::new()
        }
    }

    /// For a synset, find all links from this synset
    fn links_from(&self, synset_id : &SynsetId) -> Vec<(SynsetRelType, SynsetId)> {
        match self.synset_by_id(synset_id) {
            Some(ss) => ss.links_from(),
            None => Vec::new()
        }
    }

    /// Get a sense ID for a lemma, POS key and synset
    fn get_sense_id<'a>(&'a self, lemma : &str, pos : &PosKey, synset_id : &SynsetId) -> 
        Option<&'a SenseId> {
        match self.entries_get(&entry_key(lemma)) {
            Some(e) => e.get_sense_id(lemma, pos, synset_id),
            None => None
        }
    }

    // Get a sense ID for a lemma and synset
    fn get_sense_id2<'a>(&'a self, lemma : &str, synset_id : &SynsetId) -> 
        Option<&'a SenseId> {
        match self.entries_get(&entry_key(lemma)) {
            Some(e) => e.get_sense_id2(lemma, synset_id),
            None => None
        }
    }

    /// Add a relation between two senses
    fn add_sense_rel(&mut self, source : &SenseId, rel : SenseRelType,
                   target : &SenseId) {
        if let Some(inv) = rel.inverse() {
            self.add_sense_rel(target, inv, source);
        } else {
            self.sense_links_to_get_or(target.clone(), || Vec::new()).
                push((rel.clone(), source.clone()));
            let mut lemma_pos = None;
            match self.sense_id_to_lemma_pos_get(source) {
                Some((lemma, pos)) => {
                    lemma_pos = Some((lemma.clone(), pos.clone()));
                }
                None => {
                    eprintln!("Could not map sense id to lemma, pos")
                }
                
            }
            match lemma_pos {
                Some((lemma, pos)) => {
                    self.entries_update(&entry_key(&lemma), |e : &mut Self::E| {
                        e.add_rel(&lemma, &pos, source, rel, target).
                            unwrap_or_else(|_| {
                                eprintln!("Failed to update entry");
                            });
                    });
                },
                None => {}
            }
        }
    }

    /// Remove all links between two senses
    fn remove_sense_rel(&mut self, source : &SenseId, 
                      target : &SenseId) {
        self.sense_links_to_update(target, |v| {
            v.retain(|x| x.1 != *source);
        });
        let mut lemma_pos = None;
        match self.sense_id_to_lemma_pos_get(source) {
            Some((lemma, pos)) => {
                lemma_pos = Some((lemma.clone(), pos.clone()));
            },
            None => {
                eprintln!("Could not map sense id to lemma, pos")
            }
        }
        match lemma_pos {
            Some ((lemma, pos)) => {
                self.entries_update(&entry_key(&lemma), |e : &mut Self::E| {
                    e.remove_rel(&lemma, &pos, source, target).
                        unwrap_or_else(|_| {
                            eprintln!("Could not remove sense rel");
                        });
                });
            },
            None => {}
        }

    }

    /// Add a synset relation to WordNet
    fn add_rel(&mut self, source : &SynsetId, rel : SynsetRelType,
                   target : &SynsetId) -> Result<(), String> {
        self.links_to_get_or(target.clone(), || Vec::new()).
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
    fn remove_rel(&mut self, source : &SynsetId, target : &SynsetId) -> Result<(), String> {
        self.links_to_update(target, |v| {
            v.retain(|x| x.1 != *source);
        });
        self.update_synset(source, |ss| {
            ss.remove_rel(target);
        })?;
        Ok(())
    }

    /// Get the list of variant forms of an entry
    fn get_forms(&self, lemma : &str, pos : &PosKey) -> Vec<String> {
        match self.entries_get(&entry_key(&lemma)) {
            Some(e) => e.get_forms(lemma, pos),
            None => Vec::new()
        }
    }

    /// Add a variant form to an entry
    fn add_form(&mut self, lemma : &str, pos : &PosKey, form : String) {
        self.entries_update(&entry_key(&lemma), |e : &mut Self::E| {
            e.add_form(lemma, pos, form)
                .unwrap_or_else(|_| {
                    eprintln!("Could not find form");
                });
        });
    }

    /// Get the list of pronunications of an entry
    fn get_pronunciations(&self, lemma : &str, pos : &PosKey) -> Vec<Pronunciation> {
        match self.entries_get(&entry_key(&lemma)) {
            Some(e) => e.get_pronunciations(lemma, pos),
            None => Vec::new()
        }
    }

    /// Add a pronunciation to an entry
    fn add_pronunciation(&mut self, lemma : &str, pos : &PosKey, pronunciation : Pronunciation) {
        self.entries_update(&entry_key(&lemma), |e : &mut Self::E| {
            e.add_pronunciation(lemma, pos, pronunciation).
                unwrap_or_else(|_| {
                    eprintln!("Could not remove pronunciation");
                });

        });
    }

    /// Add a deprecation note
    fn deprecate(&mut self, synset : &SynsetId, supersede : &SynsetId, 
                     reason : String) {
        let ili = match self.synset_by_id(synset) {
            Some(ss) => match ss.ili {
                Some(ref ili) => ili.as_str().to_string(),
                None => String::new()
            },
            None => String::new()
        };
        let supersede_ili = match self.synset_by_id(supersede) {
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
            reason));
    }

    fn update_sense_key(&mut self, old_key : &SenseId, new_key : &SenseId) {
        let mut lemma_pos = None;
        match self.sense_id_to_lemma_pos_get(old_key) {
            Some((lemma, pos)) => {
                lemma_pos = Some((lemma.clone(), pos.clone()));
            },
            None => {}
        };
        if let Some((lemma, pos)) = lemma_pos {
            self.entries_update(&entry_key(&lemma), |e : &mut Self::E| {
                e.update_sense_key(&lemma, &pos, old_key, new_key)
                    .unwrap_or_else(|_| {
                        eprintln!("Could not remove sense key");
                    });
            });
        }
        match self.sense_links_to_get(old_key).map(|x| x.clone()) {
            Some(links_to) => {
                for (rel, source) in links_to {
                    self.remove_sense_rel(&source, old_key);
                    self.add_sense_rel(&source, rel.clone(), old_key);
                }
            },
            None => {}
        }
    }

    /// Get all the entries
    fn entries(&self) -> impl Iterator<Item=(&String, &PosKey, &Entry)> {
        self.entries_iter().flat_map(|(_,e)| {
            e.entries()
        })
    }

    /// Get all synsets
    fn synsets(&self) -> impl Iterator<Item=(&SynsetId, &Synset)> {
        self.synsets_iter().flat_map(|(_,e)| {
            e.iter()
        })
    }
                
    /// Get the part of speech from a lexicographer file
    fn pos_for_lexfile(&self, lexfile : &str) -> Vec<PartOfSpeech> {
        if self.synsets_contains_key(lexfile) {
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
        }
    }

    /// Number of entries in the dictionary
    fn n_entries(&self) -> usize {
        self.entries_iter().map(|v| v.1.n_entries()).sum()
    }

    /// Number of synsets in the dictionary
    fn n_synsets(&self) -> usize {
        self.synsets_iter().map(|v| v.1.len()).sum()
    }

    #[cfg(test)]
    fn add_lexfile(&mut self, lexfile : &str) {
        self.synsets_insert(lexfile.to_owned(), Synsets::new());
    }

}

fn add_sense_link_to<L : Lexicon>(backend : &mut L,
    //map : &mut HashMap<SenseId, Vec<(SenseRelType, SenseId)>>,
                     entry : &Entry) {
    for sense in entry.sense.iter() {
        for (rel_type, target) in sense.sense_links_from() {
            backend.sense_links_to_push(target.clone(), rel_type, sense.id.clone());
        }
    }
}

fn add_sense_link_to_sense<L : Lexicon>(backend : &mut L,
                           sense : &Sense) {
    for (rel_type, target) in sense.sense_links_from() {
        backend.sense_links_to_push(target.clone(), rel_type, sense.id.clone());
    }
}

fn add_link_to<L : Lexicon>(backend : &mut L,
               synset_id : &SynsetId, synset : &Synset) {
    for (rel_type, target) in synset.links_from() {
        backend.links_to_push(target.clone(), rel_type, synset_id.clone());
    }
}

fn remove_link_to<L : Lexicon>(backend : &mut L,
               synset_id : &SynsetId, synset : &Synset) {
    for (_, target) in synset.links_from() {
        backend.links_to_update(&target, |m| {
            m.retain(|sr| sr.1 != *synset_id);
        });
    }
}

fn entry_key(lemma : &str) -> String {
    let key = lemma.to_lowercase().chars().next().expect("Empty lemma!");
    if key < 'a' || key > 'z' {
        '0'.to_string()
    } else {
        key.to_string()
    }
}


