use serde::{Serialize,Deserialize};
use std::collections::BTreeMap;
use std::fmt;
use std::io::Write;
use crate::rels::{YamlSynsetRelType,SynsetRelType};
use crate::wordnet::*;
use crate::wordnet::util::{escape_yaml_string, format_confidence, string_or_vec};
use std::borrow::Cow;
use std::result;


pub trait Synsets : Sized {
    fn get<'a>(&'a self, id : &SynsetId) -> Result<Option<Cow<'a, Synset>>>;
    //fn insert<L : Lexicon>(&mut self, id : SynsetId, synset : Synset, lexicon : &L) -> Result<Option<Synset>>;
    //fn update<X>(&mut self, id : &SynsetId, f : impl FnOnce(&mut Synset) -> X) -> Result<X>;
    fn iter<'a>(&'a self) -> Result<impl Iterator<Item=Result<(SynsetId, Cow<'a, Synset>)>> + 'a>;
    fn into_iter(self) -> Result<impl Iterator<Item=Result<(SynsetId, Synset)>> + 'static>;
    fn len(&self) -> Result<usize>;
    fn remove_entry(&mut self, id : &SynsetId) -> Result<Option<(SynsetId, Synset)>>;
    fn save<W : Write>(&self, w : &mut W) -> result::Result<(), LexiconSaveError> {
        for ss in self.iter()? {
            let (key, ss) = ss?;
            write!(w, "{}:", key.as_str())?;
            ss.save(w)?;
            write!(w, "\n")?;
        }
        Ok(())
    }
    fn ssid_by_prefix(&self, prefix : &str, max_results : usize) -> Result<Vec<String>>;
}


#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct BTSynsets(pub(crate) BTreeMap<SynsetId, Synset>);

impl BTSynsets {
    pub(crate) fn new() -> BTSynsets { BTSynsets(BTreeMap::new()) }
    pub(crate) fn insert(&mut self, id : SynsetId, synset : Synset) -> Result<Option<Synset>> {
        Ok(self.0.insert(id, synset))
    }
    pub(crate) fn get_mut(&mut self, id : &SynsetId) -> Option<&mut Synset> {
        self.0.get_mut(id)
    }
}
    
   
impl Synsets for BTSynsets {
    fn get<'a>(&'a self, id : &SynsetId) -> Result<Option<Cow<'a, Synset>>> {
        Ok(self.0.get(id).map(|x| Cow::Borrowed(x)))
    }
    //fn insert(&mut self, id : SynsetId, synset : Synset) -> Result<Option<Synset>> {
    //    Ok(self.0.insert(id, synset))
    //}
    //fn update<X>(&mut self, id : &SynsetId, f : impl FnOnce(&mut Synset) -> X) -> Result<X> {
    //    if let Some(x) = self.0.get_mut(id) {
    //        Ok(f(x))
    //    } else {
    //        Err(LexiconError::SynsetIdNotFound(id.clone()))
    //    }
    //}
    fn iter<'a>(&'a self) -> Result<impl Iterator<Item=Result<(SynsetId, Cow<'a, Synset>)>> + 'a> {
        Ok(self.0.iter().map(|(k, v)| Ok((k.clone(), Cow::Borrowed(v)))))
    }
    fn into_iter(self) -> Result<impl Iterator<Item=Result<(SynsetId, Synset)>> + 'static> {
        Ok(self.0.into_iter().map(|(k, v)| Ok((k, v))))
    }
    fn len(&self) -> Result<usize> {
        Ok(self.0.len())
    }
    fn remove_entry(&mut self, id : &SynsetId) -> Result<Option<(SynsetId, Synset)>> {
        Ok(self.0.remove_entry(id))
    }

    fn ssid_by_prefix(&self, prefix : &str, limit : usize) -> Result<Vec<String>> {
        Ok(self.0.keys().filter(|id| id.as_str().starts_with(prefix))
            .take(limit)
            .map(|id| id.as_str().to_string()).collect())
    }
}
 

#[derive(Debug, PartialEq, Serialize, Deserialize,Clone)]
#[cfg_attr(feature="redb", derive(speedy::Readable, speedy::Writable))]
pub struct Synset {
    pub definition : ScoredVec<String>,
    #[serde(default)]
    pub example : Vec<Example>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ili : Option<ILIID>,
    #[serde(default, deserialize_with = "string_or_vec")]
    pub wikidata : Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source : Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence : Option<f64>,
    pub members : Vec<String>,
    #[serde(rename="partOfSpeech")]
    pub part_of_speech : PartOfSpeech,
    #[serde(default)]
    pub also : ScoredVec<SynsetId>,
    #[serde(default)]
    pub attribute : ScoredVec<SynsetId>,
    #[serde(default)]
    pub causes : ScoredVec<SynsetId>,
    #[serde(default)]
    pub domain_region : ScoredVec<SynsetId>,
    #[serde(default)]
    pub domain_topic : ScoredVec<SynsetId>,
    #[serde(default)]
    pub exemplifies : ScoredVec<SynsetId>,
    #[serde(default)]
    pub entails : ScoredVec<SynsetId>,
    #[serde(default)]
    pub hypernym : ScoredVec<SynsetId>,
    #[serde(default)]
    pub instance_hypernym : ScoredVec<SynsetId>,
    #[serde(default)]
    pub mero_location : ScoredVec<SynsetId>,
    #[serde(default)]
    pub mero_member : ScoredVec<SynsetId>,
    #[serde(default)]
    pub mero_part : ScoredVec<SynsetId>,
    #[serde(default)]
    pub mero_portion : ScoredVec<SynsetId>,
    #[serde(default)]
    pub mero_substance : ScoredVec<SynsetId>,
    #[serde(default)]
    pub meronym : ScoredVec<SynsetId>,
    #[serde(default)]
    pub similar : ScoredVec<SynsetId>,
    #[serde(default)]
    pub feminine : ScoredVec<SynsetId>,
    #[serde(default)]
    pub masculine : ScoredVec<SynsetId>,
    #[serde(default)]
    pub other : ScoredVec<SynsetId>
}

impl Synset {
    pub fn new(part_of_speech : PartOfSpeech) -> Synset {
        Synset {
            definition : Vec::new().into(),
            example : Vec::new(),
            ili : None,
            wikidata : Vec::new(),
            source : None,
            confidence : None,
            members : Vec::new(),
            part_of_speech,
            also : Vec::new().into(),
            attribute : Vec::new().into(),
            causes : Vec::new().into(),
            domain_region : Vec::new().into(),
            domain_topic : Vec::new().into(),
            exemplifies : Vec::new().into(),
            entails : Vec::new().into(),
            hypernym : Vec::new().into(),
            instance_hypernym : Vec::new().into(),
            mero_location : Vec::new().into(),
            mero_member : Vec::new().into(),
            mero_part : Vec::new().into(),
            mero_portion : Vec::new().into(),
            mero_substance : Vec::new().into(),
            meronym : Vec::new().into(),
            similar : Vec::new().into(),
            feminine : Vec::new().into(),
            masculine : Vec::new().into(),
            other : Vec::new().into()
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
                    self.also.push_scored(target_id.clone(), None);
                }
            },
            YamlSynsetRelType::Attribute => {
                if !self.attribute.iter().any(|id| id == target_id) {
                    self.attribute.push_scored(target_id.clone(), None);
                }
            },
            YamlSynsetRelType::Causes => {
                if !self.causes.iter().any(|id| id == target_id) {
                    self.causes.push_scored(target_id.clone(), None);
                }
            },
            YamlSynsetRelType::DomainRegion => {
                if !self.domain_region.iter().any(|id| id == target_id) {
                    self.domain_region.push_scored(target_id.clone(), None);
                }
            },
            YamlSynsetRelType::DomainTopic => {
                if !self.domain_topic.iter().any(|id| id == target_id) {
                    self.domain_topic.push_scored(target_id.clone(), None);
                }
            },
            YamlSynsetRelType::Exemplifies => {
                if !self.exemplifies.iter().any(|id| id == target_id) {
                    self.exemplifies.push_scored(target_id.clone(), None);
                }
            },
            YamlSynsetRelType::Entails => {
                if !self.entails.iter().any(|id| id == target_id) {
                    self.entails.push_scored(target_id.clone(), None);
                }
            },
            YamlSynsetRelType::Hypernym => {
                if !self.hypernym.iter().any(|id| id == target_id) {
                    self.hypernym.push_scored(target_id.clone(), None);
                }
            },
            YamlSynsetRelType::InstanceHypernym => {
                if !self.instance_hypernym.iter().any(|id| id == target_id) {
                    self.instance_hypernym.push_scored(target_id.clone(), None);
                }
            },
            YamlSynsetRelType::MeroLocation => {
                if !self.mero_location.iter().any(|id| id == target_id) {
                    self.mero_location.push_scored(target_id.clone(), None);
                }
            },
            YamlSynsetRelType::MeroMember => {
                if !self.mero_member.iter().any(|id| id == target_id) {
                    self.mero_member.push_scored(target_id.clone(), None);
                }
            },
            YamlSynsetRelType::MeroPart => {
                if !self.mero_part.iter().any(|id| id == target_id) {
                    self.mero_part.push_scored(target_id.clone(), None);
                }
            },
            YamlSynsetRelType::MeroPortion => {
                if !self.mero_portion.iter().any(|id| id == target_id) {
                    self.mero_portion.push_scored(target_id.clone(), None);
                }
            },
            YamlSynsetRelType::MeroSubstance => {
                if !self.mero_substance.iter().any(|id| id == target_id) {
                    self.mero_substance.push_scored(target_id.clone(), None);
                }
            },
            YamlSynsetRelType::Meronym => {
                if !self.meronym.iter().any(|id| id == target_id) {
                    self.meronym.push_scored(target_id.clone(), None);
                }
            },
            YamlSynsetRelType::Similar => {
                if !self.similar.iter().any(|id| id == target_id) {
                    self.similar.push_scored(target_id.clone(), None);
                }
            },
            YamlSynsetRelType::Feminine => {
                if !self.feminine.iter().any(|id| id == target_id) {
                    self.feminine.push_scored(target_id.clone(), None);
                }
            },
            YamlSynsetRelType::Masculine => {
                if !self.masculine.iter().any(|id| id == target_id) {
                    self.masculine.push_scored(target_id.clone(), None);
                }
            },
            YamlSynsetRelType::Other => {
                if !self.other.iter().any(|id| id == target_id) {
                    self.other.push_scored(target_id.clone(), None);
                }
            }
        }
    }

    /// `insert_rel`, then set the new (or existing) relation's confidence.
    pub(crate) fn insert_rel_scored(&mut self, rel_type : &YamlSynsetRelType,
                      target_id : &SynsetId, confidence : Option<f64>) {
        self.insert_rel(rel_type, target_id);
        self.set_rel_confidence(&rel_type.to_synset_rel(), target_id, confidence);
    }

    /// The stored target list for `rel`, or `None` for an inverse-only type (e.g. `Hyponym`)
    /// that is never stored on this side.
    pub fn rel_targets(&self, rel : &SynsetRelType) -> Option<&ScoredVec<SynsetId>> {
        match rel {
            SynsetRelType::Also => Some(&self.also),
            SynsetRelType::Attribute => Some(&self.attribute),
            SynsetRelType::Causes => Some(&self.causes),
            SynsetRelType::DomainRegion => Some(&self.domain_region),
            SynsetRelType::DomainTopic => Some(&self.domain_topic),
            SynsetRelType::Exemplifies => Some(&self.exemplifies),
            SynsetRelType::Entails => Some(&self.entails),
            SynsetRelType::Hypernym => Some(&self.hypernym),
            SynsetRelType::InstanceHypernym => Some(&self.instance_hypernym),
            SynsetRelType::MeroLocation => Some(&self.mero_location),
            SynsetRelType::MeroMember => Some(&self.mero_member),
            SynsetRelType::MeroPart => Some(&self.mero_part),
            SynsetRelType::MeroPortion => Some(&self.mero_portion),
            SynsetRelType::MeroSubstance => Some(&self.mero_substance),
            SynsetRelType::Meronym => Some(&self.meronym),
            SynsetRelType::Similar => Some(&self.similar),
            SynsetRelType::Feminine => Some(&self.feminine),
            SynsetRelType::Masculine => Some(&self.masculine),
            SynsetRelType::Other => Some(&self.other),
            _ => None
        }
    }

    pub fn rel_targets_mut(&mut self, rel : &SynsetRelType) -> Option<&mut ScoredVec<SynsetId>> {
        match rel {
            SynsetRelType::Also => Some(&mut self.also),
            SynsetRelType::Attribute => Some(&mut self.attribute),
            SynsetRelType::Causes => Some(&mut self.causes),
            SynsetRelType::DomainRegion => Some(&mut self.domain_region),
            SynsetRelType::DomainTopic => Some(&mut self.domain_topic),
            SynsetRelType::Exemplifies => Some(&mut self.exemplifies),
            SynsetRelType::Entails => Some(&mut self.entails),
            SynsetRelType::Hypernym => Some(&mut self.hypernym),
            SynsetRelType::InstanceHypernym => Some(&mut self.instance_hypernym),
            SynsetRelType::MeroLocation => Some(&mut self.mero_location),
            SynsetRelType::MeroMember => Some(&mut self.mero_member),
            SynsetRelType::MeroPart => Some(&mut self.mero_part),
            SynsetRelType::MeroPortion => Some(&mut self.mero_portion),
            SynsetRelType::MeroSubstance => Some(&mut self.mero_substance),
            SynsetRelType::Meronym => Some(&mut self.meronym),
            SynsetRelType::Similar => Some(&mut self.similar),
            SynsetRelType::Feminine => Some(&mut self.feminine),
            SynsetRelType::Masculine => Some(&mut self.masculine),
            SynsetRelType::Other => Some(&mut self.other),
            _ => None
        }
    }

    /// The confidence of the stored `rel` relation to `target`, if it has one.
    pub fn rel_confidence(&self, rel : &SynsetRelType, target : &SynsetId) -> Option<f64> {
        self.rel_targets(rel).and_then(|ts| ts.confidence(target.as_str()))
    }

    /// Set (or clear) the confidence of the stored `rel` relation to `target`. Returns false
    /// if there is no such stored relation.
    pub fn set_rel_confidence(&mut self, rel : &SynsetRelType, target : &SynsetId,
                              confidence : Option<f64>) -> bool {
        match self.rel_targets_mut(rel) {
            Some(ts) => ts.set_confidence(target.as_str(), confidence),
            None => false
        }
    }

    pub(crate) fn save<W : Write>(&self, w : &mut W) -> std::io::Result<()> {
        write_prop_synset(w, &self.also, "also")?;
        write_prop_synset(w, &self.attribute, "attribute")?;
        write_prop_synset(w, &self.causes, "causes")?;
        if let Some(c) = self.confidence {
            write!(w, "\n  confidence: {}", format_confidence(c))?;
        }
        if !self.definition.is_empty() {
            write!(w, "\n  definition:")?;
            for (defn, confidence) in self.definition.iter_scored() {
                match confidence {
                    None => write!(w, "\n  - {}", escape_yaml_string(defn,4,4))?,
                    Some(c) => write!(w, "\n  - confidence: {}\n    text: {}",
                        format_confidence(c), escape_yaml_string(defn, 6, 10))?,
                }
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

fn write_prop_synset<W : Write>(w : &mut W, synsets : &ScoredVec<SynsetId>, name : &str) -> std::io::Result<()> {
    if synsets.is_empty() {
        Ok(())
    } else {
        write!(w, "\n  {}:", name)?;
        for (synset_id, confidence) in synsets.iter_scored() {
            match confidence {
                None => write!(w, "\n  - {}", synset_id.as_str())?,
                Some(c) => write!(w, "\n  - confidence: {}\n    target: {}",
                    format_confidence(c), synset_id.as_str())?,
            }
        }
        Ok(())
    }
}


#[derive(Debug, PartialEq, Serialize, Deserialize,Clone)]
#[cfg_attr(feature="redb", derive(speedy::Readable, speedy::Writable))]
pub struct ILIID(String);

impl ILIID {
    #[allow(dead_code)]
    pub fn new(s : &str) -> ILIID { ILIID(s.to_string()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

impl fmt::Display for ILIID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone,Eq,Hash,PartialOrd,Ord)]
#[cfg_attr(feature="redb", derive(speedy::Readable, speedy::Writable))]
pub struct SynsetId(String);

impl SynsetId {
    pub fn new(s : &str) -> SynsetId { SynsetId(s.to_string()) }
    pub fn new_owned(s : String) -> SynsetId { SynsetId(s) }
    pub fn as_str(&self) -> &str { &self.0 }
}

impl AsRef<str> for SynsetId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl ScoredItem for SynsetId {
    const MAP_KEY: &'static str = "target";
    fn from_string(s: String) -> Self {
        SynsetId(s)
    }
}

impl fmt::Display for SynsetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}


