use serde::{Serialize,Deserialize};
use std::collections::BTreeMap;
use std::fmt;
use std::io::Write;
use crate::rels::{YamlSynsetRelType,SynsetRelType};
use crate::wordnet::*;
use crate::wordnet::util::{escape_yaml_string, string_or_vec};
use std::borrow::Cow;

pub trait Synsets : Sized {
    fn get<'a>(&'a self, id : &SynsetId) -> Option<Cow<'a, Synset>>;
    fn insert(&mut self, id : SynsetId, sysnet : Synset) -> Option<Synset>;
    fn update<X>(&mut self, id : &SynsetId, f : impl FnOnce(&mut Synset) -> X) -> Result<X>;
    fn iter<'a>(&'a self) -> impl Iterator<Item=(SynsetId, Cow<'a, Synset>)> + 'a;
    fn into_iter(self) -> impl Iterator<Item=(SynsetId, Synset)>;
    fn len(&self) -> usize;
    fn remove_entry(&mut self, id : &SynsetId) -> Option<(SynsetId, Synset)>;
    fn save<W : Write>(&self, w : &mut W) -> std::io::Result<()> {
        for (key, ss) in self.iter() {
            write!(w, "{}:", key.as_str())?;
            ss.save(w)?;
            write!(w, "\n")?;
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct BTSynsets(pub(crate) BTreeMap<SynsetId, Synset>);

impl BTSynsets {
    pub(crate) fn new() -> BTSynsets { BTSynsets(BTreeMap::new()) }
}
    
impl Synsets for BTSynsets {
    fn get<'a>(&'a self, id : &SynsetId) -> Option<Cow<'a, Synset>> {
        self.0.get(id).map(|x| Cow::Borrowed(x))
    }
    fn insert(&mut self, id : SynsetId, synset : Synset) -> Option<Synset> {
        self.0.insert(id, synset)
    }
    fn update<X>(&mut self, id : &SynsetId, f : impl FnOnce(&mut Synset) -> X) -> Result<X> {
        if let Some(x) = self.0.get_mut(id) {
            Ok(f(x))
        } else {
            Err(LexiconError::SynsetIdNotFound(id.clone()))
        }
    }
    fn iter<'a>(&'a self) -> impl Iterator<Item=(SynsetId, Cow<'a, Synset>)> {
        self.0.iter().map(|(k, v)| (k.clone(), Cow::Borrowed(v)) )
    }
    fn into_iter(self) -> impl Iterator<Item=(SynsetId, Synset)> {
        self.0.into_iter()
    }
    fn len(&self) -> usize {
        self.0.len()
    }
    fn remove_entry(&mut self, id : &SynsetId) -> Option<(SynsetId, Synset)> {
        self.0.remove_entry(id)
    }
}
 

#[derive(Debug, PartialEq, Serialize, Deserialize,Clone)]
pub struct Synset {
    pub definition : Vec<String>,
    #[serde(default)]
    pub example : Vec<Example>,
    pub ili : Option<ILIID>,
    #[serde(default, deserialize_with = "string_or_vec")]
    pub wikidata : Vec<String>,
    pub source : Option<String>,
    pub members : Vec<String>,
    #[serde(rename="partOfSpeech")]
    pub part_of_speech : PartOfSpeech,
    #[serde(default)]
    pub also : Vec<SynsetId>,
    #[serde(default)]
    pub attribute : Vec<SynsetId>,
    #[serde(default)]
    pub causes : Vec<SynsetId>,
    #[serde(default)]
    pub domain_region : Vec<SynsetId>,
    #[serde(default)]
    pub domain_topic : Vec<SynsetId>,
    #[serde(default)]
    pub exemplifies : Vec<SynsetId>,
    #[serde(default)]
    pub entails : Vec<SynsetId>,
    #[serde(default)]
    pub hypernym : Vec<SynsetId>,
    #[serde(default)]
    pub instance_hypernym : Vec<SynsetId>,
    #[serde(default)]
    pub mero_location : Vec<SynsetId>,
    #[serde(default)]
    pub mero_member : Vec<SynsetId>,
    #[serde(default)]
    pub mero_part : Vec<SynsetId>,
    #[serde(default)]
    pub mero_portion : Vec<SynsetId>,
    #[serde(default)]
    pub mero_substance : Vec<SynsetId>,
    #[serde(default)]
    pub meronym : Vec<SynsetId>,
    #[serde(default)]
    pub similar : Vec<SynsetId>,
    #[serde(default)]
    pub feminine : Vec<SynsetId>,
    #[serde(default)]
    pub masculine : Vec<SynsetId>,
    #[serde(default)]
    pub other : Vec<SynsetId>
}

impl Synset {
    pub fn new(part_of_speech : PartOfSpeech) -> Synset {
        Synset {
            definition : Vec::new(),
            example : Vec::new(),
            ili : None,
            wikidata : Vec::new(),
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
            feminine : Vec::new(),
            masculine : Vec::new(),
            other : Vec::new()
        }
    }

    pub(crate) fn remove_rel(&mut self, target : &SynsetId) {
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
        self.feminine.retain(|x| x != target);
        self.masculine.retain(|x| x != target);
        self.other.retain(|x| x != target);
    }

    pub(crate) fn insert_rel(&mut self, rel_type : &YamlSynsetRelType,
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
            YamlSynsetRelType::Feminine => {
                if !self.feminine.iter().any(|id| id == target_id) {
                    self.feminine.push(target_id.clone());
                }
            },
            YamlSynsetRelType::Masculine => {
                if !self.masculine.iter().any(|id| id == target_id) {
                    self.masculine.push(target_id.clone());
                }
            },
            YamlSynsetRelType::Other => {
                if !self.other.iter().any(|id| id == target_id) {
                    self.other.push(target_id.clone());
                }
            }
        }
    }

    pub(crate) fn save<W : Write>(&self, w : &mut W) -> std::io::Result<()> {
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
        write_prop_synset(w, &self.feminine, "feminine")?;
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
        write_prop_synset(w, &self.masculine, "masculine")?;
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
        if self.wikidata.len() == 1 {
            write!(w, "\n  wikidata: {}", self.wikidata[0])?;
        } else if self.wikidata.len() > 1 {
            write!(w, "\n  wikidata:")?;
            for wd in self.wikidata.iter() {
                write!(w, "\n  - {}", wd)?;
            }
        }

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
        for s in self.feminine.iter() {
            links_from.push((SynsetRelType::Feminine, s.clone()));
        }
        for s in self.masculine.iter() {
            links_from.push((SynsetRelType::Masculine, s.clone()));
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


#[derive(Debug, PartialEq, Serialize, Deserialize,Clone)]
pub struct ILIID(String);

impl ILIID {
    #[allow(dead_code)]
    pub fn new(s : &str) -> ILIID { ILIID(s.to_string()) }
    pub fn as_str(&self) -> &str { &self.0 }
}
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone,Eq,Hash,PartialOrd,Ord)]
pub struct SynsetId(String);

impl SynsetId {
    pub fn new(s : &str) -> SynsetId { SynsetId(s.to_string()) }
    pub fn new_owned(s : String) -> SynsetId { SynsetId(s) }
    pub fn as_str(&self) -> &str { &self.0 }
}

impl fmt::Display for SynsetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}


