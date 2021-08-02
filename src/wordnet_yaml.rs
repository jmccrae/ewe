use thiserror::Error;
use serde::{Serialize,Deserialize,Serializer,Deserializer};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::fs::File;
use std::fmt;
use std::io::Write;
use serde::de::{self, Visitor, MapAccess};
use crate::serde::ser::SerializeMap;
use crate::rels::YamlSynsetRelType;
use indicatif::ProgressBar;

pub struct Lexicon {
    entries : HashMap<String, Entries>,
    synsets : HashMap<String, Synsets>,
    synset_id_to_lexfile : HashMap<SynsetId, String>
}

impl Lexicon {
    pub fn load<P: AsRef<Path>>(folder : P) -> Result<Lexicon, WordNetYAMLIOError> {
        let mut entries = HashMap::new();
        let mut synsets = HashMap::new();
        let mut synset_id_to_lexfile = HashMap::new();
        let folder_files = fs::read_dir(folder)?;
        println!("Loading WordNet");
        let bar = ProgressBar::new(72);
        for file in folder_files {
            let file = file?;
            let file_name = file.path().file_name().
                and_then(|x| x.to_str()).
                map(|x| x.to_string()).
                unwrap_or_else(|| "".to_string());
            if file_name.starts_with("entries-") && file_name.ends_with(".yaml") {
                let key = file_name[8..9].to_string();
                entries.insert(key,
                    serde_yaml::from_reader(File::open(file.path())?)?);
            } else if file_name.ends_with(".yaml") && file_name != "frames.yaml" {
                let synsets2 : Synsets = serde_yaml::from_reader(File::open(file.path())?)?;
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
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Entries(HashMap<String, HashMap<String, Entry>>);

impl Entries {
    fn entry_by_lemma(&self, lemma : &str) -> Vec<&Entry> {
        self.0.get(lemma).iter().flat_map(|x| x.values()).collect()
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub sense : Vec<Sense>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub form : Vec<String>
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Sense {
    pub id : SenseId,
    pub synset : SynsetId,
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
            other: Vec::new()
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

    pub fn save<W : Write>(&self, w : &mut W) -> std::io::Result<()> {
        write!(w, "\n    - ")?;
        let mut first = true;
        first = write_prop_sense(w, &self.also, "also", first)?;
        first = write_prop_sense(w, &self.antonym, "antonym", first)?;
        first = write_prop_sense(w, &self.derivation, "derivation", first)?;
        first = write_prop_sense(w, &self.domain_region, "domain_region", first)?;
        first = write_prop_sense(w, &self.domain_topic, "domain_topic", first)?;
        first = write_prop_sense(w, &self.exemplifies, "exemplifies", first)?;
        first = write_prop_sense(w, &self.has_domain_region, "has_domain_region", first)?;
        first = write_prop_sense(w, &self.has_domain_topic, "has_domain_topic", first)?;
        if first {
            write!(w, "id:\n      - '{}'", self.id.as_str())?;
            first = false;
        } else {
            write!(w, "\n      id:\n      - '{}'", self.id.as_str())?;
        }
        write_prop_sense(w, &self.is_exemplified_by, "is_exemplified_by", first)?;
        write_prop_sense(w, &self.other, "other", first)?;
        write_prop_sense(w, &self.participle, "participle", first)?;
        write_prop_sense(w, &self.pertainym, "pertainym", first)?;
        write_prop_sense(w, &self.similar, "similar", first)?;
        if !self.subcat.is_empty() {
            write!(w, "\n      subcat:\n      - ")?;
            let mut f = true;
            for subcat_id in self.subcat.iter() {
                if !f {
                    write!(w, "\n        ")?;
                }
                write!(w, "{}", subcat_id);
            }
            f = false;
        }
        write!(w, "\n      synset: {}\n", self.synset.as_str())?;
     
        Ok(())
    }

}

fn write_prop_sense<W : Write>(w : &mut W, senses : &Vec<SenseId>, name : &str, first : bool) -> std::io::Result<bool> {
    if senses.is_empty() {
        Ok(first)
    } else if !first {
        write!(w, "\n      {}:\n      - ", name)?;
        let mut f = true;
        for sense_id in senses.iter() {
            if !f {
                write!(w, "\n        ")?;
            }
            write!(w, "'{}'", sense_id.as_str());
        }
        f = false;
        Ok(false)
    } else {
        Ok(first)
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Synsets(HashMap<SynsetId, Synset>);

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Synset {
    pub definition : Vec<String>,
    #[serde(default)]
    pub example : Vec<Example>,
    pub ili : Option<ILIID>,
    members : Vec<String>,
    #[serde(rename="partOfSpeech")]
    part_of_speech : PartOfSpeech,
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
    similar : Vec<SynsetId>,
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
}



#[derive(Debug, PartialEq)]
pub struct Example {
    pub text : String,
    pub source : Option<String>
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

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ILIID(String);

#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, Serialize, Deserialize)]
enum PartOfSpeech { n, v, a, r, s }

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone,Eq,Hash)]
pub struct SenseId(String);

impl SenseId {
    pub fn as_str(&self) -> &str { &self.0 }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone,Eq,Hash)]
pub struct SynsetId(String);

impl SynsetId {
    pub fn new(s : &str) -> SynsetId { SynsetId(s.to_string()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

#[derive(Error,Debug)]
pub enum WordNetYAMLIOError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Serde(#[from] serde_yaml::Error)
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

//    #[test]
//    fn test_load() {
//        Lexicon::load("/home/jmccrae/projects/globalwordnet/english-wordnet/src/yaml/").unwrap();
//    }
}
