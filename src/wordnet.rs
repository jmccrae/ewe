use thiserror::Error;
use serde::{Serialize,Deserialize,Serializer,Deserializer};
use std::collections::{HashMap, BTreeMap};
use std::fs;
use std::path::Path;
use std::fs::File;
use std::fmt;
use std::io::Write;
use serde::de::{self, Visitor, MapAccess};
use crate::serde::ser::SerializeMap;
use crate::rels::{YamlSynsetRelType,SenseRelType,SynsetRelType};
use indicatif::ProgressBar;
use lazy_static::lazy_static;
use regex::Regex;

/// The Lexicon contains the whole WordNet graph
pub struct Lexicon {
    entries : HashMap<String, Entries>,
    synsets : HashMap<String, Synsets>,
    synset_id_to_lexfile : HashMap<SynsetId, String>,
    sense_links_to : HashMap<SenseId, Vec<(SenseRelType, SenseId)>>,
    links_to : HashMap<SynsetId, Vec<(SynsetRelType, SynsetId)>>,
    sense_id_to_lemma_pos : HashMap<SenseId, (String, PosKey)>,
    deprecations : Vec<DeprecationRecord>
}

impl Lexicon {
    /// Create a new empty lexicon
    #[allow(dead_code)]
    pub fn new() -> Lexicon {
        Lexicon {
            entries: HashMap::new(),
            synsets: HashMap::new(),
            synset_id_to_lexfile: HashMap::new(),
            sense_links_to: HashMap::new(),
            links_to : HashMap::new(),
            sense_id_to_lemma_pos: HashMap::new(),
            deprecations: Vec::new()
        }
    }

    /// Load a lexicon from a folder of YAML files
    pub fn load<P: AsRef<Path>>(folder : P) -> Result<Lexicon, WordNetYAMLIOError> {
        let dep_file = folder.as_ref().join("../deprecations.csv");
        let mut deprecations = Vec::new();
        if dep_file.exists() {
            let mut reader = csv::Reader::from_path(dep_file)
                .map_err(|e| WordNetYAMLIOError::Csv(format!("Error reading deprecations due to {}", e)))?;
            for r in reader.deserialize() {
                deprecations.push(r.map_err(|e| {
                    WordNetYAMLIOError::Csv(format!("Error reading deprecations due to {}", e))
                })?);
            }
        } 
        let mut entries : HashMap<String, Entries> = HashMap::new();
        let mut synsets = HashMap::new();
        let mut synset_id_to_lexfile = HashMap::new();
        let mut sense_id_to_lemma_pos = HashMap::new();
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
                let entries2 : Entries =
                    serde_yaml::from_reader(File::open(file.path())
                        .map_err(|e| WordNetYAMLIOError::Io(format!("Error reading {} due to {}", file_name, e)))?)
                        .map_err(|e| WordNetYAMLIOError::Serde(format!("Error reading {} due to {}", file_name, e)))?;
                for (lemma, map) in entries2.0.iter() {
                    for (pos, entry) in map.iter() {
                        for sense in entry.sense.iter() {
                            sense_id_to_lemma_pos.insert(sense.id.clone(),
                                (lemma.to_string(), pos.clone()));
                        }
                    }
                }

                entries.insert(key, entries2);
            } else if file_name.ends_with(".yaml") && file_name != "frames.yaml" {
                let synsets2 : Synsets = serde_yaml::from_reader(
                    File::open(file.path())
                        .map_err(|e| WordNetYAMLIOError::Io(format!("Error reading {} due to {}", file_name, e)))?)
                        .map_err(|e| WordNetYAMLIOError::Serde(format!("Error reading {} due to {}", file_name, e)))?;
                let lexname = file_name[0..file_name.len()-5].to_string();
                for id in synsets2.0.keys() {
                    synset_id_to_lexfile.insert(id.clone(), lexname.clone());
                }
                synsets.insert(lexname, synsets2);
            }
            bar.inc(1);
        }
        let mut sense_links_to = HashMap::new();
        for es in entries.values() {
            for e2 in es.0.values() {
                for e in e2.values() {
                    add_sense_link_to(&mut sense_links_to, e);
                }
            }
        }
        let mut links_to = HashMap::new();
        for ss in synsets.values() {
            for (ssid, s) in ss.0.iter() {
                add_link_to(&mut links_to, ssid, s);
            }
        }
        bar.finish();
        Ok(Lexicon { entries, synsets, synset_id_to_lexfile, 
            sense_links_to, links_to,
            sense_id_to_lemma_pos, deprecations
        })
    }

    /// Save a lexicon to a set of files
    pub fn save<P: AsRef<Path>>(&self, folder : P) -> std::io::Result<()> {
        println!("Saving WordNet");
        let bar = ProgressBar::new(73);
        for (ekey, entries) in self.entries.iter() {
            let mut w = File::create(folder.as_ref().join(
                format!("entries-{}.yaml", ekey)))?;
            entries.save(&mut w)?;
            bar.inc(1);
        }
        for (skey, synsets) in self.synsets.iter() {
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
                for dep in self.deprecations.iter() {
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
    pub fn lex_name_for(&self, synset_id : &SynsetId) -> Option<String> {
        self.synset_id_to_lexfile.get(synset_id).map(|x| x.clone())
    }

    /// Get the entry data for a lemma
    pub fn entry_by_lemma(&self, lemma : &str) -> Vec<&Entry> {
        match self.entries.get(&entry_key(lemma)) {
            Some(v) => v.entry_by_lemma(lemma),
            None => Vec::new()
        }
    }

    /// Get the entry data (with the part of speech key) for a lemma
    pub fn entry_by_lemma_with_pos(&self, lemma : &str) -> Vec<(&PosKey, &Entry)> {
        match lemma.chars().nth(0) {
            Some(c) if c.to_ascii_lowercase() > 'a' && c.to_ascii_lowercase() < 'z' => {
                let key = format!("{}", c.to_lowercase());
                match self.entries.get(&key) {
                    Some(v) => v.entry_by_lemma_with_pos(lemma),
                    None => {
                        eprintln!("No entries for {}", key);
                        Vec::new()
                    }
                }
            },
            Some(_) => {
                match self.entries.get("0") {
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

    /// Get the mutable sense by lemma and synset id
    pub fn get_sense<'a>(&'a self, lemma : &str, synset_id : &SynsetId) -> Vec<&'a Sense> {
        match self.entries.get(&entry_key(&lemma)) {
            Some(entries) => entries.get_sense(lemma, synset_id),
            None => Vec::new()
        }
    }

    /// Get the part of speech key for an entry referring to a specific synset
    pub fn pos_for_entry_synset(&self, lemma : &str, synset_id : &SynsetId) -> Option<PosKey> {
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
    pub fn synset_by_id(&self, synset_id : &SynsetId) -> Option<&Synset> {
        match self.lex_name_for(synset_id) {
            Some(lex_name) => {
                match self.synsets.get(&lex_name) {
                    Some(sss) => {
                        sss.0.get(synset_id)
                    },
                    None => None
                }
            },
            None => None
        }
    }

    /// Get synset data by ID (mutable)
    pub fn synset_by_id_mut(&mut self, synset_id : &SynsetId) -> Option<&mut Synset> {
        match self.lex_name_for(synset_id) {
            Some(lex_name) => {
                match self.synsets.get_mut(&lex_name) {
                    Some(sss) => {
                        sss.0.get_mut(synset_id)
                    },
                    None => None
                }
            },
            None => None
        }
    }

    ///// Verifies if a synset is in the graph
    //pub fn has_synset(&self, synset_id : &SynsetId) -> bool {
    //    self.synset_by_id(synset_id).is_some()
    //}

    /// Verifies if a sense is in the graph
    pub fn has_sense(&self, sense_id : &SenseId) -> bool {
        self.sense_id_to_lemma_pos.get(sense_id).is_some()
    }

    /// Get the list of lemmas associated with a synset
    pub fn members_by_id(&self, synset_id : &SynsetId) -> Vec<String> {
        self.synset_by_id(synset_id).iter().flat_map(|synset|
            synset.members.iter().map(|x| x.clone())).collect()
    }

    /// Add an entry to WordNet
    pub fn insert_entry(&mut self, lemma : String, pos : PosKey, entry : Entry) {
        add_sense_link_to(&mut self.sense_links_to, &entry);
        for sense in entry.sense.iter() {
            self.sense_id_to_lemma_pos.insert(sense.id.clone(), (lemma.clone(), pos.clone()));
        }
        self.entries.entry(entry_key(&lemma)).
            or_insert_with(|| Entries::new()).insert_entry(lemma, pos, entry);
    }

    /// Add a synset to WordNet
    pub fn insert_synset(&mut self, lexname : String, synset_id : SynsetId,
                         synset : Synset) {
        add_link_to(&mut self.links_to, &synset_id, &synset);
        self.synset_id_to_lexfile.insert(synset_id.clone(), lexname.clone());
        self.synsets.entry(lexname).
            or_insert_with(|| Synsets::new()).0.insert(synset_id, synset);
    }

    /// Add a sense to an existing entry. This will not create an entry if it does not exist
    pub fn insert_sense(&mut self, lemma : String, pos : PosKey, sense : Sense) {
        add_sense_link_to_sense(&mut self.sense_links_to, &sense);
        self.sense_id_to_lemma_pos.insert(sense.id.clone(), (lemma.clone(), pos.clone()));
        self.entries.entry(entry_key(&lemma)).
            or_insert_with(|| Entries::new()).insert_sense(lemma, pos, sense);
    }

    ///// Remove an entry from WordNet
    //pub fn remove_entry(&mut self, lemma : &str, pos : &PosKey) {
    //    match self.entries.get_mut(&entry_key(lemma)) {
    //        Some(e) => e.remove_entry(&mut self.sense_links_to, lemma, pos),
    //        None => {}
    //    }
    //}

    /// Remove the sense of an existing entry. This does not remove incoming sense links!
    pub fn remove_sense(&mut self, lemma : &str, pos : &PosKey, 
                        synset_id : &SynsetId) -> Vec<SenseId> {
        let v = self.sense_links_from(lemma, pos, synset_id);
        match self.entries.get_mut(&entry_key(lemma)) {
            Some(e) => {
                let keys = e.remove_sense(lemma, pos, synset_id);
                for source in keys.iter() {
                    for (rel, target) in v.iter() {
                        match self.sense_links_to.get_mut(target) {
                            Some(key) => key.retain(|x| x.0 != *rel && 
                                                    x.1 != *source),
                            None => {}
                        }
                    }
                }
                keys
            }
            None => Vec::new()
        }

    }

    /// Remove a synset. This does not remove any senses or incoming links!
    pub fn remove_synset(&mut self, synset_id : &SynsetId) {
        match self.lex_name_for(synset_id) {
            Some(lexname) => {
                match self.synsets.get_mut(&lexname) {
                    Some(m) => {
                        match m.0.remove_entry(synset_id) {
                            Some((_, ss)) => {
                                remove_link_to(&mut self.links_to, synset_id, &ss);
                            },
                            None => {}
                        }
                    },
                    None => {}
                }
            },
            None => {}
        }
    }

    /// For a given sense, get all links from this sense
    pub fn sense_links_from(&self, lemma : &str, pos : &PosKey, 
                            synset_id : &SynsetId) -> Vec<(SenseRelType, SenseId)> {
        match self.entries.get(&entry_key(lemma)) {
            Some(e) => e.sense_links_from(lemma, pos, synset_id),
            None => Vec::new()
        }
    }

    /// For a given sense, find all backlinks referring to this sense
    pub fn sense_links_to(&self, lemma : &str, pos : &PosKey,
                          synset_id : &SynsetId) -> Vec<(SenseRelType, SenseId)> {
        match self.get_sense_id(lemma, pos, synset_id) {
            Some(sense_id) => {
                match self.sense_links_to.get(sense_id) {
                    Some(v) => v.clone(),
                    None => Vec::new()
                }
            },
            None => Vec::new()
        }
    }
    
    /// For a given sense, get all links from this sense
    pub fn sense_links_from_id(&self, sense_id : &SenseId) 
                        -> Vec<(SenseRelType, SenseId)> {
        match self.sense_id_to_lemma_pos.get(sense_id) {
            Some((lemma, pos)) => {
                match self.entries.get(&entry_key(lemma)) {
                    Some(e) => e.sense_links_from_id(lemma, pos, sense_id),
                    None => Vec::new()
                }
            },
            None => Vec::new()
        }
    }

    ///// For a given sense, find all backlinks referring to this sense
    //pub fn sense_links_to_id(&self, sense_id : &SenseId) -> 
    //    Vec<(SenseRelType, SenseId)> {
    //        match self.sense_links_to.get(sense_id) {
    //            Some(v) => v.clone(),
    //            None => Vec::new()
    //    }
    //}


    /// For a synset, find all backlinks referring to this synset
    pub fn links_to(&self, synset_id : &SynsetId) -> Vec<(SynsetRelType, SynsetId)> {
        match self.links_to.get(synset_id) {
            Some(s) => s.clone(),
            None => Vec::new()
        }
    }

    /// For a synset, find all links from this synset
    pub fn links_from(&self, synset_id : &SynsetId) -> Vec<(SynsetRelType, SynsetId)> {
        match self.synset_by_id(synset_id) {
            Some(ss) => ss.links_from(),
            None => Vec::new()
        }
    }

    /// Get a sense ID for a lemma, POS key and synset
    fn get_sense_id<'a>(&'a self, lemma : &str, pos : &PosKey, synset_id : &SynsetId) -> 
        Option<&'a SenseId> {
        match self.entries.get(&entry_key(lemma)) {
            Some(e) => e.get_sense_id(lemma, pos, synset_id),
            None => None
        }
    }

    /// Add a relation between two senses
    pub fn add_sense_rel(&mut self, source : &SenseId, rel : SenseRelType,
                   target : &SenseId) {
        self.sense_links_to.entry(target.clone()).or_insert_with(|| Vec::new()).
            push((rel.clone(), source.clone()));
        match self.sense_id_to_lemma_pos.get(source) {
            Some((lemma, pos)) => {
                match self.entries.get_mut(&entry_key(lemma)) {
                    Some(e) => e.add_rel(lemma, pos, source, rel, target),
                    None => {
                    }
                }
            },
            None => {
                eprintln!("Could not map sense id to lemma, pos")
            }
        }
    }

    /// Remove all links between two senses
    pub fn remove_sense_rel(&mut self, source : &SenseId, 
                      target : &SenseId) {
        match self.sense_links_to.get_mut(target) {
            Some(v) => v.retain(|x| x.1 != *source),
            None => {}
        }
        match self.sense_id_to_lemma_pos.get(source) {
            Some((lemma, pos)) => {
                match self.entries.get_mut(&entry_key(lemma)) {
                    Some(e) => e.remove_rel(lemma, pos, source, target),
                    None => {
                    }
                }
            },
            None => {
                eprintln!("Could not map sense id to lemma, pos")
            }
        }
    }

    /// Add a synset relation to WordNet
    pub fn add_rel(&mut self, source : &SynsetId, rel : SynsetRelType,
                   target : &SynsetId) {
        self.links_to.entry(target.clone()).or_insert_with(|| Vec::new()).
            push((rel.clone(), source.clone()));
        let (s2t, rel) = rel.to_yaml();
        if s2t {
            match self.synset_by_id_mut(source) {
                Some(ss) => ss.insert_rel(&rel, target),
                None => {}
            }
        } else {
            match self.synset_by_id_mut(source) {
                Some(ss) => ss.insert_rel(&rel, target),
                None => {}
            }
        }
    }

    /// Remove all links between two synsets
    pub fn remove_rel(&mut self, source : &SynsetId, target : &SynsetId) {
        match self.links_to.get_mut(target) {
            Some(v) => v.retain(|x| x.1 != *source),
            None => {}
        }
        match self.synset_by_id_mut(source) {
            Some(ss) => ss.remove_rel(target),
            None => {}
        }
    }

    /// Get the list of variant forms of an entry
    pub fn get_forms(&self, lemma : &str, pos : &PosKey) -> Vec<String> {
        match self.entries.get(&entry_key(&lemma)) {
            Some(e) => e.get_forms(lemma, pos),
            None => Vec::new()
        }
    }

    /// Add a variant form to an entry
    pub fn add_form(&mut self, lemma : &str, pos : &PosKey, form : String) {
        match self.entries.get_mut(&entry_key(&lemma)) {
            Some(e) => e.add_form(lemma, pos, form),
            None => {}
        }
    }

    /// Add a deprecation note
    pub fn deprecate(&mut self, synset : &SynsetId, supersede : &SynsetId, 
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
        self.deprecations.push(DeprecationRecord(
            format!("ewn-{}", synset.as_str()),
            ili,
            format!("ewn-{}", supersede.as_str()),
            supersede_ili,
            reason));
    }

    pub fn update_sense_key(&mut self, old_key : &SenseId, new_key : &SenseId) {
        match self.sense_id_to_lemma_pos.get(old_key) {
            Some((lemma, pos)) => {
                match self.entries.get_mut(&entry_key(lemma)) {
                    Some(e) => {
                        e.update_sense_key(lemma, pos, old_key, new_key);
                    },
                    None => {}
                }
            },
            None => {}
        }
        match self.sense_links_to.get(old_key).map(|x| x.clone()) {
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
    pub fn entries(&self) -> impl Iterator<Item=(&String, &PosKey, &Entry)> {
        self.entries.iter().flat_map(|(_,e)| {
            e.entries()
        })
    }

    /// Get all synsets
    pub fn synsets(&self) -> impl Iterator<Item=(&SynsetId, &Synset)> {
        self.synsets.iter().flat_map(|(_,e)| {
            e.0.iter()
        })
    }
                
    /// Get the part of speech from a lexicographer file
    pub fn pos_for_lexfile(&self, lexfile : &str) -> Vec<PartOfSpeech> {
        if self.synsets.contains_key(lexfile) {
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
    pub fn n_entries(&self) -> usize {
        self.entries.values().map(|v| v.n_entries()).sum()
    }

    /// Number of synsets in the dictionary
    pub fn n_synsets(&self) -> usize {
        self.synsets.values().map(|v| v.0.len()).sum()
    }
}

fn add_sense_link_to(map : &mut HashMap<SenseId, Vec<(SenseRelType, SenseId)>>,
                     entry : &Entry) {
    for sense in entry.sense.iter() {
        for (rel_type, target) in sense.sense_links_from() {
            map.entry(target).or_insert_with(|| Vec::new())
                .push((rel_type, sense.id.clone()))
        }
    }
}

fn add_sense_link_to_sense(map : &mut HashMap<SenseId, Vec<(SenseRelType, SenseId)>>,
                           sense : &Sense) {
    for (rel_type, target) in sense.sense_links_from() {
        map.entry(target).or_insert_with(|| Vec::new())
            .push((rel_type, sense.id.clone()))
    }
}

fn add_link_to(map : &mut HashMap<SynsetId, Vec<(SynsetRelType, SynsetId)>>,
               synset_id : &SynsetId, synset : &Synset) {
    for (rel_type, target) in synset.links_from() {
        map.entry(target).or_insert_with(|| Vec::new()).
            push((rel_type, synset_id.clone()));
    }
}

fn remove_link_to(map : &mut HashMap<SynsetId, Vec<(SynsetRelType, SynsetId)>>,
               synset_id : &SynsetId, synset : &Synset) {
    for (_, target) in synset.links_from() {
        match map.get_mut(&target) {
            Some(m) => m.retain(|sr| sr.1 != *synset_id),
            None => {}
        }
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


static YAML_LINE_LENGTH : usize = 80;
lazy_static! {
    static ref NUMBERS: Regex = Regex::new("^(\\.)?\\d+$").unwrap();
}

fn escape_yaml_string(s : &str, indent : usize, initial_indent : usize) -> String {

    let s2 : String = if !s.starts_with("'") && s.chars().any(|c| c > '~') ||
        s.starts_with("Seen in this light, it is the spectrality of the figure") {
        format!("\"{}\"", s.chars().map(|c| {
            if c == '"' {
                "\\\"".to_string()
            } else if c <= '~' {
                c.to_string()
            } else if (c as u32) < 256 {
                format!("\\x{:02X}", c as u32)
            } else {
                format!("\\u{:04X}", c as u32)
            }
        }).collect::<Vec<String>>().join(""))
    } else if s.starts_with("\"") || s.ends_with(":")  || s.contains(": ")
        || s.starts_with("'") || s == "true" || s == "false" 
        || s == "yes" || s == "no" || s == "null" || NUMBERS.is_match(s) 
        || s.ends_with(" ") || s.contains(": ")
        || s == "No" || s == "off" || s == "on" 
        || s.starts_with("`") || s.starts_with("...") {
        format!("'{}'", str::replace(s, "'", "''"))
    } else {
        s.to_owned()
    }; 
    if s2.len() + indent > YAML_LINE_LENGTH {
        let mut s3 = String::new();
        let mut size = initial_indent;
        for s4 in s2.split(" ") {
            if size > indent && s3.len() > 0 {
                s3.push_str(" ");
                size += 1;
            }
            // Super hacks for Python compat
            if s4 == "B\\xE1n\\xE1thy," {
                s3.push_str("B\\xE1n\\xE1\\\n");
                for _ in 0..indent {
                    s3.push_str(" ");
                }
                s3.push_str("thy,");
                size = indent + 4;
            } else if s4 == "Djarkatch\\xE9" {
                s3.push_str("Djarkatch\\xE9\\\n");
                for _ in 0..indent {
                    s3.push_str(" ");
                }
                s3.push_str("\\");
                size = indent + 2;
            } else if s4 == "b\\xE9chamel\\\".\"" {
                s3.push_str("b\\xE9chamel\\\"\\\n");
                for _ in 0..indent {
                    s3.push_str(" ");
                }
                s3.push_str(".\"");
                size = indent +2;
            }
            // Very odd rule in the Python line splitting algorithm
            else if s2.starts_with("\"") &&
                s4.len() + size > YAML_LINE_LENGTH &&
                (s4.contains("\\x") || s4.contains("\\u")
                 || s4.contains("\\\"")) {
                let mut indices : Vec<usize> =s4.find("\\x").iter().chain(
                    s4.find("\\u").iter().chain(
                        s4.find("\\\"").iter())).map(|x| *x).collect();
                indices.sort();
                let mut s5 = s4;
                for i in indices {
                    let (s6, s7) = s5.split_at(i);
                    let n = if s7.starts_with("\\u") {
                        6
                    } else if s7.starts_with("\\x") {
                        4
                    } else /*s7.starts_with("\\\"")*/ {
                        2
                    };
                    if s6.len() + n + size > YAML_LINE_LENGTH {
                        s3.push_str(s6);
                        if n == 2 {
                            s3.push_str("\n\\");
                            s3.push_str(&s7[0..n]);
                            for _ in 0..indent {
                                s3.push_str(" ");
                            }
                            size = indent;
                            s5 = &s7[n..];
                        } else {
                            s3.push_str(&s7[0..n]);
                            s3.push_str("\\\n");
                            for _ in 0..indent {
                                s3.push_str(" ");
                            }
                            size = indent;
                            s5 = &s7[n..];
                        }
                    } else {
                        s3.push_str(s6);
                        size += s6.len();
                        s5 = s7;
                    }
                }
                s3.push_str(s5);
                size += s5.len();
            } else {
                s3.push_str(&s4);
                size += s4.len();
                if size > YAML_LINE_LENGTH {
                    if s3.starts_with("\"") {
                        s3.push_str("\\\n");
                        for _ in 0..indent {
                            s3.push_str(" ");
                        }
                        s3.push_str("\\");
                        size = indent + 1;
                    } else {
                        s3.push_str("\n");
                        for _ in 0..indent {
                            s3.push_str(" ");
                        }
                        size = indent;
                    }
                }
            }
        } 
        if size == indent {
            s3.truncate(s3.len() - indent - 1);
        }
        if size == indent + 1 && s3.starts_with("\"") {
            s3.truncate(s3.len() - indent - 3);
        }
        s3
    } else {
        s2
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone,Eq,Hash,PartialOrd,Ord)]
pub struct PosKey(String);

impl PosKey {
    pub fn new(s : String) -> PosKey { PosKey(s) }
    pub fn as_str(&self) -> &str { &self.0 }
    pub fn to_part_of_speech(&self) -> Option<PartOfSpeech> {
        if self.0.starts_with("n") {
            Some(PartOfSpeech::n)
        } else if self.0.starts_with("v") {
            Some(PartOfSpeech::v)
        } else if self.0.starts_with("a") {
            Some(PartOfSpeech::a)
        } else if self.0.starts_with("r") {
            Some(PartOfSpeech::r)
        } else if self.0.starts_with("s") {
            Some(PartOfSpeech::s)
        } else {
            None
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Entries(BTreeMap<String, BTreeMap<PosKey, Entry>>);

impl Entries {
    fn new() -> Entries {
        Entries(BTreeMap::new())
    }

    fn entry_by_lemma(&self, lemma : &str) -> Vec<&Entry> {
        self.0.get(lemma).iter().flat_map(|x| x.values()).collect()
    }

    fn entry_by_lemma_with_pos(&self, lemma : &str) -> Vec<(&PosKey, &Entry)> {
        self.0.get(lemma).iter().flat_map(|x| x.iter()).collect()
    }

    fn save<W : Write>(&self, w : &mut W) -> std::io::Result<()> {
        for (lemma, by_pos) in self.0.iter() {
            write!(w, "{}:\n", escape_yaml_string(lemma,0,0))?;
            for (pos, entry) in by_pos.iter() {
                write!(w, "  {}:\n", pos.as_str())?;
                entry.save(w)?;
            }
        }
        Ok(())
    }
    fn insert_entry(&mut self, lemma : String, pos : PosKey, entry : Entry) {
        self.0.entry(lemma).
            or_insert_with(|| BTreeMap::new()).insert(pos, entry);
    }

    fn insert_sense(&mut self, lemma : String, pos : PosKey, sense : Sense) {
        match self.0.entry(lemma).
            or_insert_with(|| BTreeMap::new()).get_mut(&pos) {
                Some(entry) => entry.sense.push(sense),
                None => eprintln!("Failed to insert sense to non-existant entry")
            };
    }

    //fn remove_entry(&mut self, 
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

    fn remove_sense(&mut self, lemma : &str, pos : &PosKey, synset : &SynsetId) -> Vec<SenseId> {
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

    fn get_sense_id<'a>(&'a self, lemma : &str, pos : &PosKey, synset_id : &SynsetId) -> 
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

    fn add_rel(&mut self, lemma : &str, pos : &PosKey,
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

    fn remove_rel(&mut self, lemma : &str, pos : &PosKey,
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
                    e.form.push(form);
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


    fn update_sense_key(&mut self, lemma : &str, key : &PosKey,
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

    fn entries(&self) -> impl Iterator<Item=(&String, &PosKey, &Entry)> {
        self.0.iter().flat_map(|(lemma, e)| {
            let mut v = Vec::new();
            for (pos, entry) in e.iter() {
                v.push((lemma, pos, entry));
            }
            v
        })
    }

    fn n_entries(&self) -> usize {
        self.0.values().map(|v| v.len()).sum()
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize,Clone,Default)]
pub struct Entry {
    pub sense : Vec<Sense>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub form : Vec<String>
}

impl Entry {
    pub fn new() -> Entry { Entry::default() }

    fn save<W : Write>(&self, w : &mut W) -> std::io::Result<()> {
        if !self.form.is_empty() {
            write!(w,"    form:")?;
            for f in self.form.iter() {
                write!(w, "\n    - {}", f)?;
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



#[derive(Debug, PartialEq, Serialize, Deserialize,Clone)]
pub struct Sense {
    pub id : SenseId,
    pub synset : SynsetId,
    #[serde(default)]
    pub adjposition : Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub subcat: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub antonym: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub also: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub participle: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pertainym: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub derivation: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub domain_topic: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub has_domain_topic: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub domain_region: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub has_domain_region: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exemplifies: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_exemplified_by: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub similar: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub other: Vec<SenseId>,
}

impl Sense {
    pub fn new(id : SenseId, synset : SynsetId) -> Sense {
        Sense {
            id, synset, 
            subcat: Vec::new(),
            antonym: Vec::new(),
            also: Vec::new(),
            participle: Vec::new(),
            pertainym: Vec::new(),
            derivation: Vec::new(),
            domain_topic: Vec::new(),
            has_domain_topic: Vec::new(),
            domain_region: Vec::new(),
            has_domain_region: Vec::new(),
            exemplifies: Vec::new(),
            is_exemplified_by: Vec::new(),
            similar: Vec::new(),
            other: Vec::new(),
            adjposition: None
        }
    }

    pub fn remove_rel(&mut self, target : &SenseId) {
        self.antonym.retain(|x| x != target);
        self.also.retain(|x| x != target);
        self.participle.retain(|x| x != target);
        self.pertainym.retain(|x| x != target);
        self.derivation.retain(|x| x != target);
        self.domain_topic.retain(|x| x != target);
        self.has_domain_topic.retain(|x| x != target);
        self.domain_region.retain(|x| x != target);
        self.has_domain_region.retain(|x| x != target);
        self.exemplifies.retain(|x| x != target);
        self.is_exemplified_by.retain(|x| x != target);
        self.similar.retain(|x| x != target);
        self.other.retain(|x| x != target);
    }

    fn save<W : Write>(&self, w : &mut W) -> std::io::Result<()> {
        write!(w, "\n    - ")?;
        let mut first = true;
        match self.adjposition {
            Some(ref adjposition) => { 
                write!(w, "adjposition: {}", adjposition)?;
                first = false
            },
            None => {}
        };
        first = write_prop_sense(w, &self.also, "also", first)?;
        first = write_prop_sense(w, &self.antonym, "antonym", first)?;
        first = write_prop_sense(w, &self.derivation, "derivation", first)?;
        first = write_prop_sense(w, &self.domain_region, "domain_region", first)?;
        first = write_prop_sense(w, &self.domain_topic, "domain_topic", first)?;
        first = write_prop_sense(w, &self.exemplifies, "exemplifies", first)?;
        first = write_prop_sense(w, &self.has_domain_region, "has_domain_region", first)?;
        first = write_prop_sense(w, &self.has_domain_topic, "has_domain_topic", first)?;
        if first {
            write!(w, "id: {}", escape_yaml_string(self.id.as_str(), 8, 8))?;
            first = false;
        } else {
            write!(w, "\n      id: {}", escape_yaml_string(self.id.as_str(), 8, 8))?;
        }
        write_prop_sense(w, &self.is_exemplified_by, "is_exemplified_by", first)?;
        write_prop_sense(w, &self.other, "other", first)?;
        write_prop_sense(w, &self.participle, "participle", first)?;
        write_prop_sense(w, &self.pertainym, "pertainym", first)?;
        write_prop_sense(w, &self.similar, "similar", first)?;
        if !self.subcat.is_empty() {
            write!(w, "\n      subcat:")?;
            for subcat_id in self.subcat.iter() {
                write!(w, "\n      - {}", subcat_id)?;
            }
        }
        write!(w, "\n      synset: {}", self.synset.as_str())?;
     
        Ok(())
    }

    pub fn sense_links_from(&self) -> Vec<(SenseRelType, SenseId)> {
        self.antonym.iter().map(|id| (SenseRelType::Antonym, id.clone())).chain(
        self.also.iter().map(|id| (SenseRelType::Also, id.clone())).chain(
        self.participle.iter().map(|id| (SenseRelType::Participle, id.clone())).chain(
        self.pertainym.iter().map(|id| (SenseRelType::Pertainym, id.clone())).chain(
        self.derivation.iter().map(|id| (SenseRelType::Derivation, id.clone())).chain(
        self.domain_topic.iter().map(|id| (SenseRelType::DomainTopic, id.clone())).chain(
        self.has_domain_topic.iter().map(|id| (SenseRelType::HasDomainTopic, id.clone())).chain(
        self.domain_region.iter().map(|id| (SenseRelType::DomainRegion, id.clone())).chain(
        self.has_domain_region.iter().map(|id| (SenseRelType::HasDomainRegion, id.clone())).chain(
        self.exemplifies.iter().map(|id| (SenseRelType::Exemplifies, id.clone())).chain(
        self.is_exemplified_by.iter().map(|id| (SenseRelType::IsExemplifiedBy, id.clone())).chain(
        self.similar.iter().map(|id| (SenseRelType::Similar, id.clone())).chain(
        self.other.iter().map(|id| (SenseRelType::Antonym, id.clone())))))))))))))).collect()
    }
 
    
    fn add_rel(&mut self, rel : SenseRelType, target : SenseId) {
        match rel {
            SenseRelType::Antonym => self.antonym.push(target),
            SenseRelType::Also => self.also.push(target),
            SenseRelType::Participle => self.participle.push(target),
            SenseRelType::Pertainym => self.pertainym.push(target),
            SenseRelType::Derivation => self.derivation.push(target),
            SenseRelType::DomainTopic => self.domain_topic.push(target),
            SenseRelType::HasDomainTopic => self.has_domain_topic.push(target),
            SenseRelType::DomainRegion => self.domain_region.push(target),
            SenseRelType::HasDomainRegion => self.has_domain_region.push(target),
            SenseRelType::Exemplifies => self.exemplifies.push(target),
            SenseRelType::IsExemplifiedBy => self.is_exemplified_by.push(target),
            SenseRelType::Similar => self.similar.push(target),
            SenseRelType::Other => self.other.push(target)
        };
    }
}

fn write_prop_sense<W : Write>(w : &mut W, senses : &Vec<SenseId>, name : &str, first : bool) -> std::io::Result<bool> {
    if senses.is_empty() {
        Ok(first)
    } else if !first {
        write!(w, "\n      {}:", name)?; 
        for sense_id in senses.iter() {
            write!(w, "\n      - {}", escape_yaml_string(sense_id.as_str(), 8, 8))?;
        }
        Ok(false)
    } else {
        write!(w, "{}:", name)?; 
        for sense_id in senses.iter() {
            write!(w, "\n      - {}", escape_yaml_string(sense_id.as_str(), 8, 8))?;
        }
        Ok(false)
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Synsets(BTreeMap<SynsetId, Synset>);

impl Synsets {
    fn new() -> Synsets { Synsets(BTreeMap::new()) }

    fn save<W : Write>(&self, w : &mut W) -> std::io::Result<()> {
        for (key, ss) in self.0.iter() {
            write!(w, "{}:", key.as_str())?;
            ss.save(w)?;
            write!(w, "\n")?;
        }
        Ok(())
    }
}
    

#[derive(Debug, PartialEq, Serialize, Deserialize,Clone)]
pub struct Synset {
    pub definition : Vec<String>,
    #[serde(default)]
    pub example : Vec<Example>,
    pub ili : Option<ILIID>,
    pub source : Option<String>,
    pub members : Vec<String>,
    #[serde(rename="partOfSpeech")]
    pub part_of_speech : PartOfSpeech,
    #[serde(default)]
    also : Vec<SynsetId>,
    #[serde(default)]
    attribute : Vec<SynsetId>,
    #[serde(default)]
    causes : Vec<SynsetId>,
    #[serde(default)]
    domain_region : Vec<SynsetId>,
    #[serde(default)]
    domain_topic : Vec<SynsetId>,
    #[serde(default)]
    exemplifies : Vec<SynsetId>,
    #[serde(default)]
    entails : Vec<SynsetId>,
    #[serde(default)]
    pub hypernym : Vec<SynsetId>,
    #[serde(default)]
    pub instance_hypernym : Vec<SynsetId>,
    #[serde(default)]
    mero_location : Vec<SynsetId>,
    #[serde(default)]
    mero_member : Vec<SynsetId>,
    #[serde(default)]
    mero_part : Vec<SynsetId>,
    #[serde(default)]
    mero_portion : Vec<SynsetId>,
    #[serde(default)]
    mero_substance : Vec<SynsetId>,
    #[serde(default)]
    meronym : Vec<SynsetId>,
    #[serde(default)]
    pub similar : Vec<SynsetId>,
    #[serde(default)]
    other : Vec<SynsetId>
}

impl Synset {
    pub fn new(part_of_speech : PartOfSpeech) -> Synset {
        Synset {
            definition : Vec::new(),
            example : Vec::new(),
            ili : None,
            source : None,
            members : Vec::new(),
            part_of_speech,
            also : Vec::new(),
            attribute : Vec::new(),
            causes : Vec::new(),
            domain_region : Vec::new(),
            domain_topic : Vec::new(),
            exemplifies : Vec::new(),
            entails : Vec::new(),
            hypernym : Vec::new(),
            instance_hypernym : Vec::new(),
            mero_location : Vec::new(),
            mero_member : Vec::new(),
            mero_part : Vec::new(),
            mero_portion : Vec::new(),
            mero_substance : Vec::new(),
            meronym : Vec::new(),
            similar : Vec::new(),
            other : Vec::new()
        }
    }

    fn remove_rel(&mut self, target : &SynsetId) {
        self.also.retain(|x| x != target);
        self.attribute.retain(|x| x != target);
        self.causes.retain(|x| x != target);
        self.domain_region.retain(|x| x != target);
        self.domain_topic.retain(|x| x != target);
        self.exemplifies.retain(|x| x != target);
        self.entails.retain(|x| x != target);
        self.hypernym.retain(|x| x != target);
        self.instance_hypernym.retain(|x| x != target);
        self.mero_location.retain(|x| x != target);
        self.mero_member.retain(|x| x != target);
        self.mero_part.retain(|x| x != target);
        self.mero_portion.retain(|x| x != target);
        self.mero_substance.retain(|x| x != target);
        self.meronym.retain(|x| x != target);
        self.similar.retain(|x| x != target);
        self.other.retain(|x| x != target);
    }

    fn insert_rel(&mut self, rel_type : &YamlSynsetRelType,
                      target_id : &SynsetId) {
        match rel_type {
            YamlSynsetRelType::Also => {
                if !self.also.iter().any(|id| id == target_id) {
                    self.also.push(target_id.clone());
                }
            },
            YamlSynsetRelType::Attribute => {
                if !self.attribute.iter().any(|id| id == target_id) {
                    self.attribute.push(target_id.clone());
                }
            },
            YamlSynsetRelType::Causes => {
                if !self.causes.iter().any(|id| id == target_id) {
                    self.causes.push(target_id.clone());
                }
            },
            YamlSynsetRelType::DomainRegion => {
                if !self.domain_region.iter().any(|id| id == target_id) {
                    self.domain_region.push(target_id.clone());
                }
            },
            YamlSynsetRelType::DomainTopic => {
                if !self.domain_topic.iter().any(|id| id == target_id) {
                    self.domain_topic.push(target_id.clone());
                }
            },
            YamlSynsetRelType::Exemplifies => {
                if !self.exemplifies.iter().any(|id| id == target_id) {
                    self.exemplifies.push(target_id.clone());
                }
            },
            YamlSynsetRelType::Entails => {
                if !self.entails.iter().any(|id| id == target_id) {
                    self.entails.push(target_id.clone());
                }
            },
            YamlSynsetRelType::Hypernym => {
                if !self.hypernym.iter().any(|id| id == target_id) {
                    self.hypernym.push(target_id.clone());
                }
            },
            YamlSynsetRelType::InstanceHypernym => {
                if !self.instance_hypernym.iter().any(|id| id == target_id) {
                    self.instance_hypernym.push(target_id.clone());
                }
            },
            YamlSynsetRelType::MeroLocation => {
                if !self.mero_location.iter().any(|id| id == target_id) {
                    self.mero_location.push(target_id.clone());
                }
            },
            YamlSynsetRelType::MeroMember => {
                if !self.mero_member.iter().any(|id| id == target_id) {
                    self.mero_member.push(target_id.clone());
                }
            },
            YamlSynsetRelType::MeroPart => {
                if !self.mero_part.iter().any(|id| id == target_id) {
                    self.mero_part.push(target_id.clone());
                }
            },
            YamlSynsetRelType::MeroPortion => {
                if !self.mero_portion.iter().any(|id| id == target_id) {
                    self.mero_portion.push(target_id.clone());
                }
            },
            YamlSynsetRelType::MeroSubstance => {
                if !self.mero_substance.iter().any(|id| id == target_id) {
                    self.mero_substance.push(target_id.clone());
                }
            },
            YamlSynsetRelType::Meronym => {
                if !self.meronym.iter().any(|id| id == target_id) {
                    self.meronym.push(target_id.clone());
                }
            },
            YamlSynsetRelType::Similar => {
                if !self.similar.iter().any(|id| id == target_id) {
                    self.similar.push(target_id.clone());
                }
            },
            YamlSynsetRelType::Other => {
                if !self.other.iter().any(|id| id == target_id) {
                    self.other.push(target_id.clone());
                }
            }
        }
    }

    fn save<W : Write>(&self, w : &mut W) -> std::io::Result<()> {
        write_prop_synset(w, &self.also, "also")?;
        write_prop_synset(w, &self.attribute, "attribute")?;
        write_prop_synset(w, &self.causes, "causes")?;
        if !self.definition.is_empty() {
            write!(w, "\n  definition:")?;
            for defn in self.definition.iter() {
                write!(w, "\n  - {}", escape_yaml_string(defn,4,4))?;
            }
        }
        write_prop_synset(w, &self.domain_region, "domain_region")?;
        write_prop_synset(w, &self.domain_topic, "domain_topic")?;
        write_prop_synset(w, &self.entails, "entails")?;
        if !self.example.is_empty() {
            write!(w, "\n  example:")?;
            for example in self.example.iter() {
                example.save(w)?;
            }
        }
        write_prop_synset(w, &self.exemplifies, "exemplifies")?;
        write_prop_synset(w, &self.hypernym, "hypernym")?;
        match &self.ili {
            Some(s) => { 
                write!(w, "\n  ili: {}", s.as_str())?;
            },
            None => {}
        }
        write_prop_synset(w, &self.instance_hypernym, "instance_hypernym")?;
        write!(w, "\n  members:")?;
        for m in self.members.iter() {
            write!(w, "\n  - {}", escape_yaml_string(m, 4,4))?;
        }
        if self.members.is_empty() {
            write!(w, " []")?;
        }
        write_prop_synset(w, &self.mero_location, "mero_location")?;
        write_prop_synset(w, &self.mero_member, "mero_member")?;
        write_prop_synset(w, &self.mero_part, "mero_part")?;
        write_prop_synset(w, &self.mero_portion, "mero_portion")?;
        write_prop_synset(w, &self.mero_substance, "mero_substance")?;
        write_prop_synset(w, &self.meronym, "meronym")?;
        write_prop_synset(w, &self.other, "other")?;
        write!(w, "\n  partOfSpeech: {}", self.part_of_speech.value())?;
        write_prop_synset(w, &self.similar, "similar")?;
        match &self.source {
            Some(s) => { 
                write!(w, "\n  source: {}", escape_yaml_string(s, 4, 4))?;
            },
            None => {}
        };
        Ok(())
    }

    pub fn links_from(&self) -> Vec<(SynsetRelType, SynsetId)> {
        let mut links_from = Vec::new();
        for s in self.also.iter() {
            links_from.push((SynsetRelType::Also, s.clone()));
        }
        for s in self.attribute.iter() {
            links_from.push((SynsetRelType::Attribute, s.clone()));
        }
        for s in self.causes.iter() {
            links_from.push((SynsetRelType::Causes, s.clone()));
        }
        for s in self.domain_region.iter() {
            links_from.push((SynsetRelType::DomainRegion, s.clone()));
        }
        for s in self.domain_topic.iter() {
            links_from.push((SynsetRelType::DomainTopic, s.clone()));
        }
        for s in self.exemplifies.iter() {
            links_from.push((SynsetRelType::Exemplifies, s.clone()));
        }
        for s in self.entails.iter() {
            links_from.push((SynsetRelType::Entails, s.clone()));
        }
        for s in self.hypernym.iter() {
            links_from.push((SynsetRelType::Hypernym, s.clone()));
        }
        for s in self.instance_hypernym.iter() {
            links_from.push((SynsetRelType::InstanceHypernym, s.clone()));
        }
        for s in self.mero_location.iter() {
            links_from.push((SynsetRelType::MeroLocation, s.clone()));
        }
        for s in self.mero_member.iter() {
            links_from.push((SynsetRelType::MeroMember, s.clone()));
        }
        for s in self.mero_part.iter() {
            links_from.push((SynsetRelType::MeroPart, s.clone()));
        }
        for s in self.mero_portion.iter() {
            links_from.push((SynsetRelType::MeroPortion, s.clone()));
        }
        for s in self.mero_substance.iter() {
            links_from.push((SynsetRelType::MeroSubstance, s.clone()));
        }
        for s in self.meronym.iter() {
            links_from.push((SynsetRelType::Meronym, s.clone()));
        }
        for s in self.similar.iter() {
            links_from.push((SynsetRelType::Similar, s.clone()));
        }
        for s in self.other.iter() {
            links_from.push((SynsetRelType::Other, s.clone()));
        }
        links_from
    }

}

fn write_prop_synset<W : Write>(w : &mut W, synsets : &Vec<SynsetId>, name : &str) -> std::io::Result<()> {
    if synsets.is_empty() {
        Ok(())
    } else {
        write!(w, "\n  {}:", name)?;
        for sense_id in synsets.iter() {
            write!(w, "\n  - {}", sense_id.as_str())?;
        }
        Ok(())
    }
}



#[derive(Debug, PartialEq,Clone)]
pub struct Example {
    pub text : String,
    pub source : Option<String>
}

impl Example {
    pub fn new(text : String, source : Option<String>) -> Example {
        Example {
            text: text, source 
        }
    }

    fn save<W : Write>(&self, w : &mut W) -> std::io::Result<()> {
        write!(w, "\n  - ")?;
        match &self.source {
            Some(s) => {
                write!(w, "source: {}\n    text: {}", 
                       escape_yaml_string(s, 6, 10),
                       escape_yaml_string(&self.text, 6, 10))?;
            },
            None => {
                write!(w, "{}", escape_yaml_string(&self.text, 4, 4))?;
            }
        }
        Ok(())
    }
}

impl Serialize for Example {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.source {
            Some(ref s) => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("source", s)?;
                map.serialize_entry("text", &self.text)?;
                map.end()
            },
            None => {
                serializer.serialize_str(&self.text)
            }
        }
    }
}


impl<'de> Deserialize<'de> for Example {
    fn deserialize<D>(deserializer: D) -> Result<Example, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ExampleVisitor)
    }
}

pub struct ExampleVisitor;

impl<'de> Visitor<'de> for ExampleVisitor
{
    type Value = Example;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("string or map")
    }

    fn visit_str<E>(self, value: &str) -> Result<Example, E>
    where
        E: de::Error,
    {
        Ok(Example { text: value.to_string(), source: None })
    }

    fn visit_map<M>(self, mut map: M) -> Result<Example, M::Error>
    where
        M: MapAccess<'de>,
    {
        let key1 = map.next_key::<String>()?;
        let val1 = map.next_value::<String>()?;
        let key2 = map.next_key::<String>()?;
        let val2 = map.next_value::<String>()?;
        if key1 == Some("text".to_string()) && key2 == Some("source".to_string()) {
            Ok(Example { text: val1, source: Some(val2) })
        } else if key2 == Some("text".to_string()) && key1 == Some("source".to_string()) {
            Ok(Example { text: val2, source: Some(val1) })
        } else {
            panic!("Unexpected keys in example")
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize,Clone)]
pub struct ILIID(String);

impl ILIID {
    #[allow(dead_code)]
    pub fn new(s : &str) -> ILIID { ILIID(s.to_string()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, Serialize, Deserialize,Clone)]
pub enum PartOfSpeech { n, v, a, r, s }

impl PartOfSpeech {
    pub fn value(&self) -> &'static str {
        match self {
            PartOfSpeech::n => "n",
            PartOfSpeech::v => "v",
            PartOfSpeech::a => "a",
            PartOfSpeech::r => "r",
            PartOfSpeech::s => "s"
        }
    }

    pub fn to_pos_key(&self) -> PosKey {
        PosKey::new(self.value().to_string())
    }

    //pub fn equals_pos(&self, s : &str) -> bool {
    //    match self {
    //        PartOfSpeech::n => s.starts_with("n"),
    //        PartOfSpeech::v => s.starts_with("v"),
    //        PartOfSpeech::a => s.starts_with("a") || s.starts_with("s"),
    //        PartOfSpeech::r => s.starts_with("r"),
    //        PartOfSpeech::s => s.starts_with("a") || s.starts_with("s")
    //    }
    //}

    //pub fn equals_str(&self, s : &str) -> bool {
    //    match self {
    //        PartOfSpeech::n => s.starts_with("n"),
    //        PartOfSpeech::v => s.starts_with("v"),
    //        PartOfSpeech::a => s.starts_with("a"),
    //        PartOfSpeech::r => s.starts_with("r"),
    //        PartOfSpeech::s => s.starts_with("s")
    //    }
    //}

    pub fn ss_type(&self) -> u32 {
        match self {
            PartOfSpeech::n => 1,
            PartOfSpeech::v => 2,
            PartOfSpeech::a => 3,
            PartOfSpeech::r => 4,
            PartOfSpeech::s => 5
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone,Eq,Hash)]
pub struct SenseId(String);

impl SenseId {
    pub fn new(s : String) -> SenseId { SenseId(s) }
    pub fn as_str(&self) -> &str { &self.0 }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone,Eq,Hash,PartialOrd,Ord)]
pub struct SynsetId(String);

impl SynsetId {
    pub fn new(s : &str) -> SynsetId { SynsetId(s.to_string()) }
    pub fn new_owned(s : String) -> SynsetId { SynsetId(s) }
    pub fn as_str(&self) -> &str { &self.0 }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone,Eq,Hash,PartialOrd,Ord)]
pub struct DeprecationRecord(String,String,String,String,String);

#[derive(Error,Debug)]
pub enum WordNetYAMLIOError {
    #[error("Could not load WordNet: {0}")]
    Io(String),
    #[error("Could not load WordNet: {0}")]
    Serde(String),
    #[error("Could not load WordNet: {0}")]
    Csv(String)
}

#[cfg(test)]
#[allow(unused_variables)]
mod tests {
    use super::*;

    #[test]
    fn test_entry() {
        let entry_str = "sense:
- id: 'foo%1:01:00::'
  synset: 00001740-n
";
        assert_eq!(serde_yaml::from_str::<Entry>(&entry_str).unwrap(),
            Entry {
                sense: vec![Sense::new(
                    SenseId("foo%1:01:00::".to_string()),
                    SynsetId("00001740-n".to_string())
                )],
                form: Vec::new()
            });
    }

    #[test]
    fn test_save_entry() {
        let entry_str = "    sense:
    - id: 'foo%1:01:00::'
      synset: 00001740-n
";
        let mut gen_str : Vec<u8> = Vec::new();

        Entry {
            sense: vec![Sense::new(
                SenseId("foo%1:01:00::".to_string()),
                SynsetId("00001740-n".to_string())
            )],
            form: Vec::new()
        }.save(&mut gen_str).unwrap();
        assert_eq!(entry_str, String::from_utf8(gen_str).unwrap());
    }
 

    #[test]
    fn test_entries() {
        let entry_str = "abate:
  v:
    sense:
    - derivation:
      - abatable%5:00:00:stoppable:00
      - 'abator%1:18:00::'
      id: 'abate%2:30:01::'
      subcat:
      - vtai
      - vtii
      synset: 00246175-v
    - derivation:
      - 'abatement%1:11:01::'
      id: 'abate%2:30:00::'
      subcat:
      - vii
      synset: 00245945-v
abatement:
  n:
    sense:
    - derivation:
      - 'abate%2:30:00::'
      id: 'abatement%1:11:01::'
      synset: 07382856-n
    - id: 'abatement%1:04:00::'
      synset: 00362159-n";
        let e : Entries = serde_yaml::from_str(&entry_str).unwrap();
    }

    #[test]
    fn test_synset() {
        let synset_str = "definition:
- part of a meal served at one time
example:
- '\"she prepared a three course meal\"'
hypernym:
- 07586285-n
ili: i76474
members:
- course
partOfSpeech: n";
        let s : Synset = serde_yaml::from_str(&synset_str).unwrap();
    }

    #[test]
    fn test_save_synset() {
        let synset_str = "
  definition:
  - part of a meal served at one time
  example:
  - '\"she prepared a three course meal\"'
  hypernym:
  - 07586285-n
  ili: i76474
  members:
  - course
  partOfSpeech: n";
        let mut ss = Synset::new(PartOfSpeech::n);
        ss.definition.push("part of a meal served at one time".to_owned());
        ss.example.push(Example::new(
            "\"she prepared a three course meal\"".to_owned(), None));
        ss.hypernym.push(SynsetId::new("07586285-n"));
        ss.ili = Some(ILIID::new("i76474"));
        ss.members.push("course".to_owned());
        let mut gen_str : Vec<u8> = Vec::new();
        ss.save(&mut gen_str).unwrap();
        assert_eq!(synset_str, String::from_utf8(gen_str).unwrap());
    }

    #[test]
    fn test_split_line() {
        let string = "especially of muscles; drawing away from the midline of the body or from an adjacent part";
        assert_eq!("especially of muscles; drawing away from the midline of the body or from an adjacent\n    part", escape_yaml_string(string, 4, 4));
    }


    #[test]
    fn test_split_line2() {
        let string = "(usually followed by `to') having the necessary means or skill or know-how or authority to do something";
        assert_eq!("(usually followed by `to') having the necessary means or skill or know-how or\n    authority to do something", escape_yaml_string(string, 4, 4));
    }

    #[test]
    fn test_split_line3() {
        let string = "\"the abaxial surface of a leaf is the underside or side facing away from the stem\"";
        assert_eq!("'\"the abaxial surface of a leaf is the underside or side facing away from the\n    stem\"'", escape_yaml_string(string, 4, 4));
    }

    #[test]
    fn test_split_line4() {
        let string = "Canned cream of mushroom soup has been described as \"America's béchamel\"";
        assert_eq!("\"Canned cream of mushroom soup has been described as \\\"America's b\\xE9chamel\\\n\\\"", escape_yaml_string(string, 6, 6));
    }

    #[test]
    fn test_split_line5() {
        let string = "If you consider a point on a radius of the rolling curve in generating a cardioid that is not on its circumference, the result is a conchoid called the limaçon of Pascal.";
        assert_eq!("\"If you consider a point on a radius of the rolling curve in generating a cardioid\\\n    \\ that is not on its circumference, the result is a conchoid called the lima\\xE7\\\n    on of Pascal.\"", escape_yaml_string(string, 4, 4));
    }

    #[test]
    fn test_entry_deriv() {
        let entry_str = "    sense:
    - derivation:
      - 'foo%1:01:00::'
      id: 'foo%1:01:00::'
      synset: 00001740-n
";
        let mut gen_str : Vec<u8> = Vec::new();
        let mut sense = Sense::new(
                SenseId("foo%1:01:00::".to_string()),
                SynsetId("00001740-n".to_string())
            );
        sense.derivation.push(SenseId::new("foo%1:01:00::".to_owned()));

        Entry {
            sense: vec![sense],
            form: Vec::new()
        }.save(&mut gen_str).unwrap();
        assert_eq!(entry_str, String::from_utf8(gen_str).unwrap());
    }

    #[test]
    fn test_unicode_convert() {
        assert_eq!("\"f\\xF6o\"",escape_yaml_string("föo", 0, 0));
        assert_eq!("\"\\\"f\\xF6o\\\"\"",escape_yaml_string("\"föo\"", 0, 0));
    }

//    #[test]
//    fn test_load() {
//        Lexicon::load("/home/jmccrae/projects/globalwordnet/english-wordnet/src/yaml/").unwrap();
//    }
}
