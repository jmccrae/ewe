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
use crate::rels::YamlSynsetRelType;
use indicatif::ProgressBar;
use lazy_static::lazy_static;
use regex::Regex;

pub struct Lexicon {
    entries : HashMap<String, Entries>,
    synsets : HashMap<String, Synsets>,
    synset_id_to_lexfile : HashMap<SynsetId, String>
}

impl Lexicon {
    /// Create a new empty lexicon
    pub fn new() -> Lexicon {
        Lexicon {
            entries: HashMap::new(),
            synsets: HashMap::new(),
            synset_id_to_lexfile: HashMap::new()
        }
    }

    pub fn load<P: AsRef<Path>>(folder : P) -> Result<Lexicon, WordNetYAMLIOError> {
        let mut entries = HashMap::new();
        let mut synsets = HashMap::new();
        let mut synset_id_to_lexfile = HashMap::new();
        let folder_files = fs::read_dir(folder)
            .map_err(|e| WordNetYAMLIOError::Io(format!("Could not list directory: {}", e)))?;
        println!("Loading WordNet");
        let bar = ProgressBar::new(72);
        for file in folder_files {
            let file = file.map_err(|e|
                WordNetYAMLIOError::Io(format!("Could not list directory: {}", e)))?;
            let file_name = file.path().file_name().
                and_then(|x| x.to_str()).
                map(|x| x.to_string()).
                unwrap_or_else(|| "".to_string());
            if file_name.starts_with("entries-") && file_name.ends_with(".yaml") {
                let key = file_name[8..9].to_string();
                entries.insert(key,
                    serde_yaml::from_reader(File::open(file.path())
                        .map_err(|e| WordNetYAMLIOError::Io(format!("Error reading {} due to {}", file_name, e)))?)
                        .map_err(|e| WordNetYAMLIOError::Serde(format!("Error reading {} due to {}", file_name, e)))?);
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
        bar.finish();
        Ok(Lexicon { entries, synsets, synset_id_to_lexfile })
    }

    pub fn save<P: AsRef<Path>>(&self, folder : P) -> std::io::Result<()> {
        println!("Saving WordNet");
        let bar = ProgressBar::new(72);
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
        bar.finish();
        Ok(())
    }

    pub fn lex_name_for(&self, synset_id : &SynsetId) -> Option<String> {
        self.synset_id_to_lexfile.get(synset_id).map(|x| x.clone())
    }

    pub fn entry_by_lemma(&self, lemma : &str) -> Vec<&Entry> {
        match lemma.chars().nth(0) {
            Some(c) if c.to_ascii_lowercase() > 'a' && c.to_ascii_lowercase() < 'z' => {
                let key = format!("{}", c.to_lowercase());
                match self.entries.get(&key) {
                    Some(v) => v.entry_by_lemma(lemma),
                    None => {
                        eprintln!("No entries for {}", key);
                        Vec::new()
                    }
                }
            },
            Some(c) => {
                match self.entries.get("0") {
                    Some(v) => v.entry_by_lemma(lemma),
                    None => Vec::new()
                }
            },
            None => {
                eprintln!("Query with empty string");
                Vec::new()
            }
        }
    }

    pub fn entry_by_lemma_with_pos(&self, lemma : &str) -> Vec<(&String, &Entry)> {
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
            Some(c) => {
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


    pub fn has_synset(&self, synset_id : &SynsetId) -> bool {
        self.synset_by_id(synset_id).is_some()
    }

    pub fn members_by_id(&self, synset_id : &SynsetId) -> Vec<String> {
        self.synset_by_id(synset_id).iter().flat_map(|synset|
            synset.members.iter().map(|x| x.clone())).collect()
    }

    pub fn insert_entry(&mut self, lemma : String, pos : String, entry : Entry) {
        let key = lemma.to_lowercase().chars().next().expect("Empty lemma!");
        let key = if key < 'a' || key > 'z' {
            '0'
        } else {
            key
        };
        self.entries.entry(key.to_string()).
            or_insert_with(|| Entries::new()).insert_entry(lemma, pos, entry);
    }

    pub fn insert_synset(&mut self, lexname : String, synset_id : SynsetId,
                         synset : Synset) {
        self.synset_id_to_lexfile.insert(synset_id.clone(), lexname.clone());
        self.synsets.entry(lexname).
            or_insert_with(|| Synsets::new()).0.insert(synset_id, synset);
    }
}

static YAML_LINE_LENGTH : usize = 80;
lazy_static! {
    static ref NUMBERS: Regex = Regex::new("^(\\.)?\\d+$").unwrap();
}

fn escape_yaml_string(s : &str, indent : usize, initial_indent : usize) -> String {

    let s2 : String = if !s.starts_with("'") && s.chars().any(|c| c > '~') {
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
    } else if s.starts_with("\"") || s.ends_with(":") 
        || s.starts_with("'") || s == "true" || s == "false" 
        || s == "yes" || s == "no" || s == "null" || NUMBERS.is_match(s) 
        || s.ends_with(" ") || s.contains(": ")
        || s == "No" || s == "off" || s == "on" 
        || s.starts_with("`") {
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

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Entries(BTreeMap<String, BTreeMap<String, Entry>>);

impl Entries {
    fn new() -> Entries {
        Entries(BTreeMap::new())
    }

    fn entry_by_lemma(&self, lemma : &str) -> Vec<&Entry> {
        self.0.get(lemma).iter().flat_map(|x| x.values()).collect()
    }

    fn entry_by_lemma_with_pos(&self, lemma : &str) -> Vec<(&String, &Entry)> {
        self.0.get(lemma).iter().flat_map(|x| x.iter()).collect()
    }

    fn save<W : Write>(&self, w : &mut W) -> std::io::Result<()> {
        for (lemma, by_pos) in self.0.iter() {
            write!(w, "{}:\n", escape_yaml_string(lemma,0,0))?;
            for (pos, entry) in by_pos.iter() {
                write!(w, "  {}:\n", pos)?;
                entry.save(w)?;
            }
        }
        Ok(())
    }
    pub fn insert_entry(&mut self, lemma : String, pos : String, entry : Entry) {
        self.0.entry(lemma).
            or_insert_with(|| BTreeMap::new()).insert(pos, entry);
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize,Clone)]
pub struct Entry {
    pub sense : Vec<Sense>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub form : Vec<String>
}

impl Entry {
    pub fn new() -> Entry {
        Entry {
            sense: Vec::new(),
            form: Vec::new()
        }
    }

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

    pub fn remove_all_relations(&mut self, target : &SenseId) {
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
    agent : Vec<SynsetId>,
    #[serde(default)]
    also : Vec<SynsetId>,
    #[serde(default)]
    attribute : Vec<SynsetId>,
    #[serde(default)]
    be_in_state : Vec<SynsetId>,
    #[serde(default)]
    causes : Vec<SynsetId>,
    #[serde(default)]
    classifies : Vec<SynsetId>,
    #[serde(default)]
    co_agent_instrument : Vec<SynsetId>,
    #[serde(default)]
    co_agent_patient : Vec<SynsetId>,
    #[serde(default)]
    co_agent_result : Vec<SynsetId>,
    #[serde(default)]
    co_patient_instrument : Vec<SynsetId>,
    #[serde(default)]
    co_result_instrument : Vec<SynsetId>,
    #[serde(default)]
    co_role : Vec<SynsetId>,
    #[serde(default)]
    direction : Vec<SynsetId>,
    #[serde(default)]
    domain_region : Vec<SynsetId>,
    #[serde(default)]
    domain_topic : Vec<SynsetId>,
    #[serde(default)]
    exemplifies : Vec<SynsetId>,
    #[serde(default)]
    entails : Vec<SynsetId>,
    #[serde(default)]
    eq_synonym : Vec<SynsetId>,
    #[serde(default)]
    hypernym : Vec<SynsetId>,
    #[serde(default)]
    instance_hypernym : Vec<SynsetId>,
    #[serde(default)]
    instrument : Vec<SynsetId>,
    #[serde(default)]
    location : Vec<SynsetId>,
    #[serde(default)]
    manner_of : Vec<SynsetId>,
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
    other : Vec<SynsetId>,
    #[serde(default)]
    patient : Vec<SynsetId>,
    #[serde(default)]
    restricts : Vec<SynsetId>,
    #[serde(default)]
    result : Vec<SynsetId>,
    #[serde(default)]
    role : Vec<SynsetId>,
    #[serde(default)]
    source_direction : Vec<SynsetId>,
    #[serde(default)]
    target_direction : Vec<SynsetId>,
    #[serde(default)]
    subevent : Vec<SynsetId>,
    #[serde(default)]
    antonym : Vec<SynsetId>
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
            agent : Vec::new(),
            also : Vec::new(),
            attribute : Vec::new(),
            be_in_state : Vec::new(),
            causes : Vec::new(),
            classifies : Vec::new(),
            co_agent_instrument : Vec::new(),
            co_agent_patient : Vec::new(),
            co_agent_result : Vec::new(),
            co_patient_instrument : Vec::new(),
            co_result_instrument : Vec::new(),
            co_role : Vec::new(),
            direction : Vec::new(),
            domain_region : Vec::new(),
            domain_topic : Vec::new(),
            exemplifies : Vec::new(),
            entails : Vec::new(),
            eq_synonym : Vec::new(),
            hypernym : Vec::new(),
            instance_hypernym : Vec::new(),
            instrument : Vec::new(),
            location : Vec::new(),
            manner_of : Vec::new(),
            mero_location : Vec::new(),
            mero_member : Vec::new(),
            mero_part : Vec::new(),
            mero_portion : Vec::new(),
            mero_substance : Vec::new(),
            meronym : Vec::new(),
            similar : Vec::new(),
            other : Vec::new(),
            patient : Vec::new(),
            restricts : Vec::new(),
            result : Vec::new(),
            role : Vec::new(),
            source_direction : Vec::new(),
            target_direction : Vec::new(),
            subevent : Vec::new(),
            antonym : Vec::new()
        }
    }

    pub fn remove_all_relations(&mut self, target : &SynsetId) {
        self.agent.retain(|x| x != target);
        self.also.retain(|x| x != target);
        self.attribute.retain(|x| x != target);
        self.be_in_state.retain(|x| x != target);
        self.causes.retain(|x| x != target);
        self.classifies.retain(|x| x != target);
        self.co_agent_instrument.retain(|x| x != target);
        self.co_agent_patient.retain(|x| x != target);
        self.co_agent_result.retain(|x| x != target);
        self.co_patient_instrument.retain(|x| x != target);
        self.co_result_instrument.retain(|x| x != target);
        self.co_role.retain(|x| x != target);
        self.direction.retain(|x| x != target);
        self.domain_region.retain(|x| x != target);
        self.domain_topic.retain(|x| x != target);
        self.exemplifies.retain(|x| x != target);
        self.entails.retain(|x| x != target);
        self.eq_synonym.retain(|x| x != target);
        self.hypernym.retain(|x| x != target);
        self.instance_hypernym.retain(|x| x != target);
        self.instrument.retain(|x| x != target);
        self.location.retain(|x| x != target);
        self.manner_of.retain(|x| x != target);
        self.mero_location.retain(|x| x != target);
        self.mero_member.retain(|x| x != target);
        self.mero_part.retain(|x| x != target);
        self.mero_portion.retain(|x| x != target);
        self.mero_substance.retain(|x| x != target);
        self.meronym.retain(|x| x != target);
        self.similar.retain(|x| x != target);
        self.other.retain(|x| x != target);
        self.patient.retain(|x| x != target);
        self.restricts.retain(|x| x != target);
        self.result.retain(|x| x != target);
        self.role.retain(|x| x != target);
        self.source_direction.retain(|x| x != target);
        self.target_direction.retain(|x| x != target);
        self.subevent.retain(|x| x != target);
        self.antonym.retain(|x| x != target);
    }

    pub fn insert_rel(&mut self, rel_type : &YamlSynsetRelType,
                      target_id : &SynsetId) {
        match rel_type {
            YamlSynsetRelType::Agent => {
                if !self.agent.iter().any(|id| id == target_id) {
                   self.agent.push(target_id.clone());
                }
            },
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
            YamlSynsetRelType::BeInState => {
                if !self.be_in_state.iter().any(|id| id == target_id) {
                    self.be_in_state.push(target_id.clone());
                }
            },
            YamlSynsetRelType::Causes => {
                if !self.causes.iter().any(|id| id == target_id) {
                    self.causes.push(target_id.clone());
                }
            },
            YamlSynsetRelType::Classifies => {
                if !self.classifies.iter().any(|id| id == target_id) {
                    self.classifies.push(target_id.clone());
                }
            },
            YamlSynsetRelType::CoAgentInstrument => {
                if !self.co_agent_instrument.iter().any(|id| id == target_id) {
                    self.co_agent_instrument.push(target_id.clone());
                }
            },
            YamlSynsetRelType::CoAgentPatient => {
                if !self.co_agent_patient.iter().any(|id| id == target_id) {
                    self.co_agent_patient.push(target_id.clone());
                }
            },
            YamlSynsetRelType::CoAgentResult => {
                if !self.co_agent_result.iter().any(|id| id == target_id) {
                    self.co_agent_result.push(target_id.clone());
                }
            },
            YamlSynsetRelType::CoPatientInstrument => {
                if !self.co_patient_instrument.iter().any(|id| id == target_id) {
                    self.co_patient_instrument.push(target_id.clone());
                }
            },
            YamlSynsetRelType::CoResultInstrument => {
                if !self.co_result_instrument.iter().any(|id| id == target_id) {
                    self.co_result_instrument.push(target_id.clone());
                }
            },
            YamlSynsetRelType::CoRole => {
                if !self.co_role.iter().any(|id| id == target_id) {
                    self.co_role.push(target_id.clone());
                }
            },
            YamlSynsetRelType::Direction => {
                if !self.direction.iter().any(|id| id == target_id) {
                    self.direction.push(target_id.clone());
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
            YamlSynsetRelType::EqSynonym => {
                if !self.eq_synonym.iter().any(|id| id == target_id) {
                    self.eq_synonym.push(target_id.clone());
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
            YamlSynsetRelType::Instrument => {
                if !self.instrument.iter().any(|id| id == target_id) {
                    self.instrument.push(target_id.clone());
                }
            },
            YamlSynsetRelType::Location => {
                if !self.location.iter().any(|id| id == target_id) {
                    self.location.push(target_id.clone());
                }
            },
            YamlSynsetRelType::MannerOf => {
                if !self.manner_of.iter().any(|id| id == target_id) {
                    self.manner_of.push(target_id.clone());
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
            },
            YamlSynsetRelType::Patient => {
                if !self.patient.iter().any(|id| id == target_id) {
                    self.patient.push(target_id.clone());
                }
            },
            YamlSynsetRelType::Restricts => {
                if !self.restricts.iter().any(|id| id == target_id) {
                    self.restricts.push(target_id.clone());
                }
            },
            YamlSynsetRelType::Result => {
                if !self.result.iter().any(|id| id == target_id) {
                    self.result.push(target_id.clone());
                }
            },
            YamlSynsetRelType::Role => {
                if !self.role.iter().any(|id| id == target_id) {
                    self.role.push(target_id.clone());
                }
            },
            YamlSynsetRelType::SourceDirection => {
                if !self.source_direction.iter().any(|id| id == target_id) {
                    self.source_direction.push(target_id.clone());
                }
            },
            YamlSynsetRelType::TargetDirection => {
                if !self.target_direction.iter().any(|id| id == target_id) {
                    self.target_direction.push(target_id.clone());
                }
            },
            YamlSynsetRelType::Subevent => {
                if !self.subevent.iter().any(|id| id == target_id) {
                    self.subevent.push(target_id.clone());
                }
            },
            YamlSynsetRelType::Antonym => {
                if !self.antonym.iter().any(|id| id == target_id) {
                    self.antonym.push(target_id.clone());
                }
            }
        }
    }

    fn save<W : Write>(&self, w : &mut W) -> std::io::Result<()> {
        write_prop_synset(w, &self.agent, "agent")?;
        write_prop_synset(w, &self.also, "also")?;
        write_prop_synset(w, &self.antonym, "antonym")?;
        write_prop_synset(w, &self.attribute, "attribute")?;
        write_prop_synset(w, &self.be_in_state, "be_in_state")?;
        write_prop_synset(w, &self.causes, "causes")?;
        write_prop_synset(w, &self.classifies, "classifies")?;
        write_prop_synset(w, &self.co_agent_instrument, "co_agent_instrument")?;
        write_prop_synset(w, &self.co_agent_patient, "co_agent_patient")?;
        write_prop_synset(w, &self.co_agent_result, "co_agent_result")?;
        write_prop_synset(w, &self.co_patient_instrument, "co_patient_instrument")?;
        write_prop_synset(w, &self.co_result_instrument, "co_result_instrument")?;
        write_prop_synset(w, &self.co_role, "co_role")?;
        if !self.definition.is_empty() {
            write!(w, "\n  definition:")?;
            for defn in self.definition.iter() {
                write!(w, "\n  - {}", escape_yaml_string(defn,4,4))?;
            }
        }
        write_prop_synset(w, &self.direction, "direction")?;
        write_prop_synset(w, &self.domain_region, "domain_region")?;
        write_prop_synset(w, &self.domain_topic, "domain_topic")?;
        write_prop_synset(w, &self.entails, "entails")?;
        write_prop_synset(w, &self.eq_synonym, "eq_synonym")?;
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
        write_prop_synset(w, &self.instrument, "instrument")?;
        write_prop_synset(w, &self.location, "location")?;
        write_prop_synset(w, &self.manner_of, "manner_of")?;
        write!(w, "\n  members:")?;
        for m in self.members.iter() {
            write!(w, "\n  - {}", escape_yaml_string(m, 4,4))?;
        }
        write_prop_synset(w, &self.mero_location, "mero_location")?;
        write_prop_synset(w, &self.mero_member, "mero_member")?;
        write_prop_synset(w, &self.mero_part, "mero_part")?;
        write_prop_synset(w, &self.mero_portion, "mero_portion")?;
        write_prop_synset(w, &self.mero_substance, "mero_substance")?;
        write_prop_synset(w, &self.meronym, "meronym")?;
        write_prop_synset(w, &self.other, "other")?;
        write!(w, "\n  partOfSpeech: {}", self.part_of_speech.value())?;
        write_prop_synset(w, &self.patient, "patient")?;
        write_prop_synset(w, &self.restricts, "restricts")?;
        write_prop_synset(w, &self.result, "result")?;
        write_prop_synset(w, &self.role, "role")?;
        write_prop_synset(w, &self.similar, "similar")?;
        match &self.source {
            Some(s) => { 
                write!(w, "\n  source: {}", escape_yaml_string(s, 4, 4))?;
            },
            None => {}
        };
        write_prop_synset(w, &self.source_direction, "source_direction")?;
        write_prop_synset(w, &self.subevent, "subevent")?;
        write_prop_synset(w, &self.target_direction, "target_direction")?;
        Ok(())
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
    fn new(text : &str, source : Option<String>) -> Example {
        Example {
            text: text.to_string(), source 
        }
    }

    fn save<W : Write>(&self, w : &mut W) -> std::io::Result<()> {
        write!(w, "\n  - ")?;
        match &self.source {
            Some(s) => {
                write!(w, "source: {}\n    text: {}", s,
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

    pub fn equals_pos(&self, s : &str) -> bool {
        match self {
            PartOfSpeech::n => s.starts_with("n"),
            PartOfSpeech::v => s.starts_with("v"),
            PartOfSpeech::a => s.starts_with("a") || s.starts_with("s"),
            PartOfSpeech::r => s.starts_with("r"),
            PartOfSpeech::s => s.starts_with("a") || s.starts_with("s")
        }
    }

    pub fn equals_str(&self, s : &str) -> bool {
        match self {
            PartOfSpeech::n => s.starts_with("n"),
            PartOfSpeech::v => s.starts_with("v"),
            PartOfSpeech::a => s.starts_with("a"),
            PartOfSpeech::r => s.starts_with("r"),
            PartOfSpeech::s => s.starts_with("s")
        }
    }

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
    pub fn new(s : &str) -> SenseId { SenseId(s.to_string()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone,Eq,Hash,PartialOrd,Ord)]
pub struct SynsetId(String);

impl SynsetId {
    pub fn new(s : &str) -> SynsetId { SynsetId(s.to_string()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

#[derive(Error,Debug)]
pub enum WordNetYAMLIOError {
    #[error("Could not load WordNet: {0}")]
    Io(String),
    #[error("Could not load WordNet: {0}")]
    Serde(String)
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
            "\"she prepared a three course meal\"", None));
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
        sense.derivation.push(SenseId::new("foo%1:01:00::"));

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
