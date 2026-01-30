use serde::{Serialize,Deserialize};
use std::io::Write;
use crate::rels::SenseRelType;
use crate::wordnet::*;
use crate::wordnet::util::{write_prop_sense, escape_yaml_string};
use std::fmt;

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
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub agent: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub material: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub event: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub instrument: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub location: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub by_means_of: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub undergoer: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub property: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub result: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub state: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub uses: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub destination: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub body_part: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub vehicle: Vec<SenseId>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    sent : Vec<String>
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
            agent : Vec::new(),
            material : Vec::new(),
            event : Vec::new(),
            instrument : Vec::new(),
            location : Vec::new(),
            by_means_of : Vec::new(),
            undergoer : Vec::new(),
            property : Vec::new(),
            result : Vec::new(),
            state : Vec::new(),
            uses : Vec::new(),
            destination : Vec::new(),
            body_part : Vec::new(),
            vehicle : Vec::new(),
            adjposition: None,
            sent : Vec::new()
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
        self.agent.retain(|x| x != target);
        self.material.retain(|x| x != target);
        self.event.retain(|x| x != target);
        self.instrument.retain(|x| x != target);
        self.location.retain(|x| x != target);
        self.by_means_of.retain(|x| x != target);
        self.undergoer.retain(|x| x != target);
        self.property.retain(|x| x != target);
        self.result.retain(|x| x != target);
        self.state.retain(|x| x != target);
        self.uses.retain(|x| x != target);
        self.destination.retain(|x| x != target);
        self.body_part.retain(|x| x != target);
        self.vehicle.retain(|x| x != target);
        self.other.retain(|x| x != target);
    }

    pub(crate) fn save<W : Write>(&self, w : &mut W) -> std::io::Result<()> {
        write!(w, "\n    - ")?;
        let mut first = true;
        match self.adjposition {
            Some(ref adjposition) => { 
                write!(w, "adjposition: {}", adjposition)?;
                first = false
            },
            None => {}
        };
        first = write_prop_sense(w, &self.agent, "agent", first)?;
        first = write_prop_sense(w, &self.also, "also", first)?;
        first = write_prop_sense(w, &self.antonym, "antonym", first)?;
        first = write_prop_sense(w, &self.body_part, "body_part", first)?;
        first = write_prop_sense(w, &self.by_means_of, "by_means_of", first)?;
        first = write_prop_sense(w, &self.derivation, "derivation", first)?;
        first = write_prop_sense(w, &self.destination, "destination", first)?;
        first = write_prop_sense(w, &self.domain_region, "domain_region", first)?;
        first = write_prop_sense(w, &self.domain_topic, "domain_topic", first)?;
        first = write_prop_sense(w, &self.event, "event", first)?;
        first = write_prop_sense(w, &self.exemplifies, "exemplifies", first)?;
        first = write_prop_sense(w, &self.has_domain_region, "has_domain_region", first)?;
        first = write_prop_sense(w, &self.has_domain_topic, "has_domain_topic", first)?;
        if first {
            write!(w, "id: {}", escape_yaml_string(self.id.as_str(), 8, 8))?;
            first = false;
        } else {
            write!(w, "\n      id: {}", escape_yaml_string(self.id.as_str(), 8, 8))?;
        }
        write_prop_sense(w, &self.instrument, "instrument", first)?;
        write_prop_sense(w, &self.is_exemplified_by, "is_exemplified_by", first)?;
        write_prop_sense(w, &self.location, "location", first)?;
        write_prop_sense(w, &self.material, "material", first)?;
        write_prop_sense(w, &self.other, "other", first)?;
        write_prop_sense(w, &self.participle, "participle", first)?;
        write_prop_sense(w, &self.pertainym, "pertainym", first)?;
        write_prop_sense(w, &self.property, "property", first)?;
        write_prop_sense(w, &self.result, "result", first)?;
        if !self.sent.is_empty() {
            write!(w, "\n      sent:")?;
            for subcat_id in self.sent.iter() {
                write!(w, "\n      - {}", subcat_id)?;
            }
        }
        write_prop_sense(w, &self.similar, "similar", first)?;
        write_prop_sense(w, &self.state, "state", first)?;
        if !self.subcat.is_empty() {
            write!(w, "\n      subcat:")?;
            for subcat_id in self.subcat.iter() {
                write!(w, "\n      - {}", subcat_id)?;
            }
        }
        write!(w, "\n      synset: {}", self.synset.as_str())?;
        write_prop_sense(w, &self.undergoer, "undergoer", first)?;
        write_prop_sense(w, &self.uses, "uses", first)?;
        write_prop_sense(w, &self.vehicle, "vehicle", first)?;
     
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
        self.other.iter().map(|id| (SenseRelType::Antonym, id.clone())).chain(
        self.agent.iter().map(|id| (SenseRelType::Agent, id.clone())).chain(
        self.material.iter().map(|id| (SenseRelType::Material, id.clone())).chain(
        self.event.iter().map(|id| (SenseRelType::Event, id.clone())).chain(
        self.instrument.iter().map(|id| (SenseRelType::Instrument, id.clone())).chain(
        self.location.iter().map(|id| (SenseRelType::Location, id.clone())).chain(
        self.by_means_of.iter().map(|id| (SenseRelType::ByMeansOf, id.clone())).chain(
        self.undergoer.iter().map(|id| (SenseRelType::Undergoer, id.clone())).chain(
        self.property.iter().map(|id| (SenseRelType::Property, id.clone())).chain(
        self.result.iter().map(|id| (SenseRelType::Result, id.clone())).chain(
        self.state.iter().map(|id| (SenseRelType::State, id.clone())).chain(
        self.uses.iter().map(|id| (SenseRelType::Uses, id.clone())).chain(
        self.destination.iter().map(|id| (SenseRelType::Destination, id.clone())).chain(
        self.body_part.iter().map(|id| (SenseRelType::BodyPart, id.clone())).chain(
        self.vehicle.iter().map(|id| (SenseRelType::Vehicle, id.clone()))
                                                    )))))))))))))))))))))))))).collect()
    }
 
    
    pub(crate) fn add_rel(&mut self, rel : SenseRelType, target : SenseId) {
        match rel {
            SenseRelType::Antonym => if !self.antonym.iter().any(|x| *x == target) { self.antonym.push(target) },
            SenseRelType::Also => if !self.also.iter().any(|x| *x == target) { self.also.push(target) },
            SenseRelType::Participle => if !self.participle.iter().any(|x| *x == target) { self.participle.push(target) },
            SenseRelType::Pertainym => if !self.pertainym.iter().any(|x| *x == target) { self.pertainym.push(target) },
            SenseRelType::Derivation => if !self.derivation.iter().any(|x| *x == target) { self.derivation.push(target) },
            SenseRelType::DomainTopic => if !self.domain_topic.iter().any(|x| *x == target) { self.domain_topic.push(target) },
            SenseRelType::HasDomainTopic => if !self.has_domain_topic.iter().any(|x| *x == target) { self.has_domain_topic.push(target) },
            SenseRelType::DomainRegion => if !self.domain_region.iter().any(|x| *x == target) { self.domain_region.push(target) },
            SenseRelType::HasDomainRegion => if !self.has_domain_region.iter().any(|x| *x == target) { self.has_domain_region.push(target) },
            SenseRelType::Exemplifies => if !self.exemplifies.iter().any(|x| *x == target) { self.exemplifies.push(target) },
            SenseRelType::IsExemplifiedBy => if !self.is_exemplified_by.iter().any(|x| *x == target) { self.is_exemplified_by.push(target) },
            SenseRelType::IsPertainymOf => {},
            SenseRelType::Similar => if !self.similar.iter().any(|x| *x == target) { self.similar.push(target) },
            SenseRelType::Agent => if !self.agent.iter().any(|x| *x == target) { self.agent.push(target) },
            SenseRelType::Material => if !self.material.iter().any(|x| *x == target) { self.material.push(target) },
            SenseRelType::Event => if !self.event.iter().any(|x| *x == target) { self.event.push(target) },
            SenseRelType::Instrument => if !self.instrument.iter().any(|x| *x == target) { self.instrument.push(target) },
            SenseRelType::Location => if !self.location.iter().any(|x| *x == target) { self.location.push(target) },
            SenseRelType::ByMeansOf => if !self.by_means_of.iter().any(|x| *x == target) { self.by_means_of.push(target) },
            SenseRelType::Undergoer => if !self.undergoer.iter().any(|x| *x == target) { self.undergoer.push(target) },
            SenseRelType::Property => if !self.property.iter().any(|x| *x == target) { self.property.push(target) },
            SenseRelType::Result => if !self.result.iter().any(|x| *x == target) { self.result.push(target) },
            SenseRelType::State => if !self.state.iter().any(|x| *x == target) { self.state.push(target) },
            SenseRelType::Uses => if !self.uses.iter().any(|x| *x == target) { self.uses.push(target) },
            SenseRelType::Destination => if !self.destination.iter().any(|x| *x == target) { self.destination.push(target) },
            SenseRelType::BodyPart => if !self.body_part.iter().any(|x| *x == target) { self.body_part.push(target) },
            SenseRelType::Vehicle => if !self.vehicle.iter().any(|x| *x == target) { self.vehicle.push(target) },

            SenseRelType::Other => self.other.push(target)
        };
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone,Eq,Hash)]
pub struct SenseId(String);

impl SenseId {
    pub fn new(s : String) -> SenseId { SenseId(s) }
    pub fn as_str(&self) -> &str { &self.0 }
}


impl fmt::Display for SenseId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

