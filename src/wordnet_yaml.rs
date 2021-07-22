use thiserror::Error;
use serde::{Serialize,Deserialize,Serializer,Deserializer};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::fs::File;
use std::fmt;
use serde::de::{self, Visitor, MapAccess};
use crate::serde::ser::SerializeMap;

pub struct Lexicon {
    entries : HashMap<String, Entries>,
    synsets : HashMap<String, Synsets>
}

impl Lexicon {
    pub fn load<P: AsRef<Path>>(folder : P) -> Result<Lexicon, WordNetYAMLIOError> {
        let mut entries = HashMap::new();
        let mut synsets = HashMap::new();
        for file in fs::read_dir(folder)? {
            let file = file?;
            let file_name = file.path().file_name().
                and_then(|x| x.to_str()).
                map(|x| x.to_string()).
                unwrap_or_else(|| "".to_string());
            if file_name.starts_with("entries-") && file_name.ends_with(".yaml") {
                entries.insert(file_name[9..10].to_string(),
                    serde_yaml::from_reader(File::open(file.path())?)?);
            } else if file_name.ends_with(".yaml") && file_name != "frames.yaml" {
                eprintln!("{}", file_name);
                synsets.insert(file_name[0..file_name.len()-5].to_string(),
                    serde_yaml::from_reader(File::open(file.path())?)?);
            }
        }
        Ok(Lexicon { entries, synsets })
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Entries(HashMap<String, HashMap<String, Entry>>);

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub sense : Vec<Sense>,
    #[serde(default)]
    pub form : Vec<String>
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Sense {
    pub id : SenseId,
    pub synset : SynsetId,
    #[serde(default)]
    pub subcat: Vec<String>,
    #[serde(default)]
    pub antonym: Vec<SenseId>,
    #[serde(default)]
    pub also: Vec<SenseId>,
    #[serde(default)]
    pub participle: Vec<SenseId>,
    #[serde(default)]
    pub pertainym: Vec<SenseId>,
    #[serde(default)]
    pub derivation: Vec<SenseId>,
    #[serde(default)]
    pub domain_topic: Vec<SenseId>,
    #[serde(default)]
    pub has_domain_topic: Vec<SenseId>,
    #[serde(default)]
    pub domain_region: Vec<SenseId>,
    #[serde(default)]
    pub has_domain_region: Vec<SenseId>,
    #[serde(default)]
    pub exemplifies: Vec<SenseId>,
    #[serde(default)]
    pub is_exemplified_by: Vec<SenseId>,
    #[serde(default)]
    pub similar: Vec<SenseId>,
    #[serde(default)]
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
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Synsets(HashMap<String, Synset>);

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Synset {
    definition : Vec<String>,
    #[serde(default)]
    example : Vec<Example>,
    ili : Option<ILIID>,
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

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct SenseId(String);

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct SynsetId(String);

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
