/// Member Synset is a enriched version of the `Synset` with information
/// including reverse links, ids and the relevant members.
/// It is used to render the interface for the editor, but contains
/// redundant information not found in the serialized form.
use serde::{Serialize, Deserialize};
use crate::wordnet::*;
use std::collections::HashMap;
use crate::rels::{SynsetRelType,SenseRelType};

#[derive(Debug, PartialEq, Serialize, Deserialize,Clone)]
#[cfg_attr(feature="redb", derive(speedy::Readable, speedy::Writable))]
pub struct MemberSynset {
    pub id : SynsetId,
    pub lexname : String,
    pub members : Vec<Member>,
    pub definition : Vec<String>,
    #[serde(default)]
    pub example : Vec<Example>,
    pub ili : Option<ILIID>,
    #[serde(default, deserialize_with = "string_or_vec")]
    pub wikidata : Vec<String>,
    pub source : Option<String>,
    #[serde(rename="partOfSpeech")]
    pub part_of_speech : PartOfSpeech,
    #[serde(default)]
    also : Vec<SynsetId>,
    #[serde(default)]
    attribute : Vec<SynsetId>,
    #[serde(default)]
    causes : Vec<SynsetId>,
    #[serde(default)]
    pub domain_region : Vec<SynsetId>,
    #[serde(default)]
    pub domain_topic : Vec<SynsetId>,
    #[serde(default)]
    pub exemplifies : Vec<SynsetId>,
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
    pub feminine : Vec<SynsetId>,
    #[serde(default)]
    pub masculine : Vec<SynsetId>,
    #[serde(default)]
    other : Vec<SynsetId>,

    // Inverse fields
    #[serde(default)]
    hyponym : Vec<SynsetId>,
    #[serde(default)]
    is_caused_by: Vec<SynsetId>,
    #[serde(default)]
    has_domain_region: Vec<SynsetId>,
    #[serde(default)]
    has_domain_topic: Vec<SynsetId>,
    #[serde(default)]
    is_exemplified_by: Vec<SynsetId>,
    #[serde(default)]
    is_entailed_by: Vec<SynsetId>,
    #[serde(default)]
    instance_hyponym: Vec<SynsetId>,
    #[serde(default)]
    holo_member: Vec<SynsetId>,
    #[serde(default)]
    holo_part: Vec<SynsetId>,
    #[serde(default)]
    holo_substance: Vec<SynsetId>,

    // Sense Relations
    #[serde(default)]
    antonym: Vec<SenseRelation>,
    #[serde(default)]
    participle: Vec<SenseRelation>,
    #[serde(default)]
    is_participle_of: Vec<SenseRelation>,
    #[serde(default)]
    pertainym: Vec<SenseRelation>,
    #[serde(default)]
    is_pertainym_of: Vec<SenseRelation>,
    #[serde(default)]
    derivation: Vec<SenseRelation>,
    #[serde(default)]
    exemplifies_sense: Vec<SenseRelation>,
    #[serde(default)]
    is_exemplified_by_sense: Vec<SenseRelation>,
    #[serde(default)]
    agent: Vec<SenseRelation>,
    #[serde(default)]
    is_agent_of: Vec<SenseRelation>,
    #[serde(default)]
    material: Vec<SenseRelation>,
    #[serde(default)]
    is_material_of: Vec<SenseRelation>,
    #[serde(default)]
    event: Vec<SenseRelation>,
    #[serde(default)]
    is_event_of: Vec<SenseRelation>,
    #[serde(default)]
    instrument: Vec<SenseRelation>,
    #[serde(default)]
    is_instrument_of: Vec<SenseRelation>,
    #[serde(default)]
    location: Vec<SenseRelation>,
    #[serde(default)]
    is_location_of: Vec<SenseRelation>,
    #[serde(default)]
    by_means_of: Vec<SenseRelation>,
    #[serde(default)]
    is_by_means_of: Vec<SenseRelation>,
    #[serde(default)]
    undergoer: Vec<SenseRelation>,
    #[serde(default)]
    is_undergoer_of: Vec<SenseRelation>,
    #[serde(default)]
    property: Vec<SenseRelation>,
    #[serde(default)]
    is_property_of: Vec<SenseRelation>,
    #[serde(default)]
    result: Vec<SenseRelation>,
    #[serde(default)]
    is_result_of: Vec<SenseRelation>,
    #[serde(default)]
    state: Vec<SenseRelation>,
    #[serde(default)]
    is_state_of: Vec<SenseRelation>,
    #[serde(default)]
    uses: Vec<SenseRelation>,
    #[serde(default)]
    is_used_by: Vec<SenseRelation>,
    #[serde(default)]
    destination: Vec<SenseRelation>,
    #[serde(default)]
    is_destination_of: Vec<SenseRelation>,
    #[serde(default)]
    body_part: Vec<SenseRelation>,
    #[serde(default)]
    is_body_part_of: Vec<SenseRelation>,
    #[serde(default)]
    vehicle: Vec<SenseRelation>,
    #[serde(default)]
    is_vehicle_of: Vec<SenseRelation>
}

#[derive(Debug, PartialEq, Serialize, Deserialize,Clone)]
#[cfg_attr(feature="redb", derive(speedy::Readable, speedy::Writable))]
pub struct Member {
    pub lemma : String,
    pub sense : MemberSense,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub form : Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pronunciation : Vec<Pronunciation>,
    #[serde(default)]
    pub poskey : Option<PosKey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_no : Option<u32>
}

#[derive(Debug, PartialEq, Serialize, Deserialize,Clone)]
#[cfg_attr(feature="redb", derive(speedy::Readable, speedy::Writable))]
pub struct MemberSense {
    pub id : SenseId,
    pub subcat: Vec<String>,
}
 
#[derive(Debug, PartialEq, Serialize, Deserialize,Clone)]
#[cfg_attr(feature="redb", derive(speedy::Readable, speedy::Writable))]
pub struct SenseRelation {
    pub target_synset: SynsetId,
    pub source_lemma: String,
    pub target_lemma: String
}

impl MemberSynset {
    pub fn from_synset<L : Lexicon>(synset_id : &SynsetId, 
        synset : Synset, lexicon : &L) -> Result<MemberSynset> {
        let mut members = Vec::new();
        let mut sense_links = HashMap::new();
        let mut inv_sense_links = HashMap::new();
        for m in synset.members.iter() {
            for (poskey, entry) in lexicon.entry_by_lemma_with_pos(m)? {
                for sense in entry.sense.iter() {
                    if &sense.synset != synset_id {
                        continue;
                    }
                    members.push(Member {
                        lemma: m.clone(),
                        sense: MemberSense {
                            id: sense.id.clone(),
                            subcat: sense.subcat.clone()
                        },
                        form: entry.form.clone(),
                        pronunciation: entry.pronunciation.clone(),
                        poskey: Some(poskey.clone()),
                        entry_no: poskey.entry_no()
                    });
                    macro_rules! extract_sense_rel {
                        ($rel:ident,$name:ident) => {
                            for target in sense.$rel.iter() {
                                if let Some((target_lemma, _, target_sense)) = lexicon.get_sense_by_id(target)? {
                                    sense_links.entry(SenseRelType::$name)
                                        .or_insert_with(|| Vec::new())
                                        .push(SenseRelation {
                                            target_synset: target_sense.synset.clone(),
                                            source_lemma: m.clone(),
                                            target_lemma: target_lemma.clone()
                                        });
                                }
                            }
                        }
                    }
                    extract_sense_rel!(antonym,Antonym);
                    extract_sense_rel!(participle,Participle);
                    extract_sense_rel!(pertainym,Pertainym);
                    extract_sense_rel!(derivation,Derivation);
                    extract_sense_rel!(agent,Agent);
                    extract_sense_rel!(exemplifies,Exemplifies);
                    extract_sense_rel!(material,Material);
                    extract_sense_rel!(event,Event);
                    extract_sense_rel!(instrument,Instrument);
                    extract_sense_rel!(location,Location);
                    extract_sense_rel!(by_means_of,ByMeansOf);
                    extract_sense_rel!(undergoer,Undergoer);
                    extract_sense_rel!(property,Property);
                    extract_sense_rel!(result,Result);
                    extract_sense_rel!(state,State);
                    extract_sense_rel!(uses,Uses);
                    extract_sense_rel!(destination,Destination);
                    extract_sense_rel!(body_part,BodyPart);
                    extract_sense_rel!(vehicle,Vehicle);
                    if let Some(sense_links_to) = lexicon.sense_links_to_get(&sense.id)? {
                        for (rel, target_sense_id) in sense_links_to.iter() {
                            if let Some((target_lemma, _, target_sense)) = lexicon.get_sense_by_id(&target_sense_id)? {
                                inv_sense_links.
                                    entry(rel.clone()).
                                    or_insert_with(|| Vec::new()).
                                    push(SenseRelation {
                                        target_synset: target_sense.synset.clone(),
                                        source_lemma: m.clone(),
                                        target_lemma: target_lemma.clone()
                                    });
                            }
                        }
                    }
                }
            }
        }
        let links_to = lexicon.links_to_get(synset_id)?;
        let mut links = HashMap::new();
        if let Some(links_to) = links_to {
            for (rel, target) in links_to.into_owned().into_iter() {
                links.entry(rel).or_insert_with(|| Vec::new()).push(target);
            }
        }

        Ok(MemberSynset {
            members,
            id: synset_id.clone(),
            lexname: lexicon.lex_name_for(synset_id)?.unwrap_or("".to_string()),
            definition: synset.definition,
            example: synset.example,
            ili: synset.ili,
            wikidata: synset.wikidata,
            source: synset.source,
            part_of_speech: synset.part_of_speech,
            also: synset.also,
            attribute: synset.attribute,
            causes: synset.causes,
            domain_region: synset.domain_region,
            domain_topic: synset.domain_topic,
            exemplifies: synset.exemplifies,
            entails: synset.entails,
            hypernym: synset.hypernym,
            instance_hypernym: synset.instance_hypernym,
            mero_member: synset.mero_member,
            mero_part: synset.mero_part,
            mero_substance: synset.mero_substance,
            mero_location: synset.mero_location,
            mero_portion: synset.mero_portion,
            meronym: synset.meronym,
            similar: synset.similar,
            feminine: synset.feminine,
            masculine: synset.masculine,
            other: synset.other,
            hyponym: links.remove(&SynsetRelType::Hyponym).unwrap_or_else(|| Vec::new()),
            is_caused_by: links.remove(&SynsetRelType::IsCausedBy).unwrap_or_else(|| Vec::new()),
            has_domain_region: links.remove(&SynsetRelType::HasDomainRegion).unwrap_or_else(|| Vec::new()),
            has_domain_topic: links.remove(&SynsetRelType::HasDomainTopic).unwrap_or_else(|| Vec::new()),
            is_exemplified_by: links.remove(&SynsetRelType::IsExemplifiedBy).unwrap_or_else(|| Vec::new()),
            is_entailed_by: links.remove(&SynsetRelType::IsEntailedBy).unwrap_or_else(|| Vec::new()),
            instance_hyponym: links.remove(&SynsetRelType::InstanceHyponym).unwrap_or_else(|| Vec::new()),
            holo_member: links.remove(&SynsetRelType::HoloMember).unwrap_or_else(|| Vec::new()),
            holo_part: links.remove(&SynsetRelType::HoloPart).unwrap_or_else(|| Vec::new()),
            holo_substance: links.remove(&SynsetRelType::HoloSubstance).unwrap_or_else(|| Vec::new()),
            antonym: sense_links.remove(&SenseRelType::Antonym).unwrap_or_else(|| Vec::new()),
            participle: sense_links.remove(&SenseRelType::Participle).unwrap_or_else(|| Vec::new()),
            is_participle_of: inv_sense_links.remove(&SenseRelType::Participle).unwrap_or_else(|| Vec::new()),
            pertainym: sense_links.remove(&SenseRelType::Pertainym).unwrap_or_else(|| Vec::new()),
            is_pertainym_of: inv_sense_links.remove(&SenseRelType::Pertainym).unwrap_or_else(|| Vec::new()),
            derivation: sense_links.remove(&SenseRelType::Derivation).unwrap_or_else(|| Vec::new()),
            exemplifies_sense: sense_links.remove(&SenseRelType::Exemplifies).unwrap_or_else(|| Vec::new()),
            is_exemplified_by_sense: inv_sense_links.remove(&SenseRelType::Exemplifies).unwrap_or_else(|| Vec::new()),
            agent: sense_links.remove(&SenseRelType::Agent).unwrap_or_else(|| Vec::new()),
            is_agent_of: inv_sense_links.remove(&SenseRelType::Agent).unwrap_or_else(|| Vec::new()),
            material: sense_links.remove(&SenseRelType::Material).unwrap_or_else(|| Vec::new()),
            is_material_of: inv_sense_links.remove(&SenseRelType::Material).unwrap_or_else(|| Vec::new()),
            event: sense_links.remove(&SenseRelType::Event).unwrap_or_else(|| Vec::new()),
            is_event_of: inv_sense_links.remove(&SenseRelType::Event).unwrap_or_else(|| Vec::new()),
            instrument: sense_links.remove(&SenseRelType::Instrument).unwrap_or_else(|| Vec::new()),
            is_instrument_of: inv_sense_links.remove(&SenseRelType::Instrument).unwrap_or_else(|| Vec::new()),
            location: sense_links.remove(&SenseRelType::Location).unwrap_or_else(|| Vec::new()),
            is_location_of: inv_sense_links.remove(&SenseRelType::Location).unwrap_or_else(|| Vec::new()),
            by_means_of: sense_links.remove(&SenseRelType::ByMeansOf).unwrap_or_else(|| Vec::new()),
            is_by_means_of: inv_sense_links.remove(&SenseRelType::ByMeansOf).unwrap_or_else(|| Vec::new()),
            undergoer: sense_links.remove(&SenseRelType::Undergoer).unwrap_or_else(|| Vec::new()),
            is_undergoer_of: inv_sense_links.remove(&SenseRelType::Undergoer).unwrap_or_else(|| Vec::new()),
            property: sense_links.remove(&SenseRelType::Property).unwrap_or_else(|| Vec::new()),
            is_property_of: inv_sense_links.remove(&SenseRelType::Property).unwrap_or_else(|| Vec::new()),
            result: sense_links.remove(&SenseRelType::Result).unwrap_or_else(|| Vec::new()),
            is_result_of: inv_sense_links.remove(&SenseRelType::Result).unwrap_or_else(|| Vec::new()),
            state: sense_links.remove(&SenseRelType::State).unwrap_or_else(|| Vec::new()),
            is_state_of: inv_sense_links.remove(&SenseRelType::State).unwrap_or_else(|| Vec::new()),
            uses: sense_links.remove(&SenseRelType::Uses).unwrap_or_else(|| Vec::new()),
            is_used_by: inv_sense_links.remove(&SenseRelType::Uses).unwrap_or_else(|| Vec::new()),
            destination: sense_links.remove(&SenseRelType::Destination).unwrap_or_else(|| Vec::new()),
            is_destination_of: inv_sense_links.remove(&SenseRelType::Destination).unwrap_or_else(|| Vec::new()),
            body_part: sense_links.remove(&SenseRelType::BodyPart).unwrap_or_else(|| Vec::new()),
            is_body_part_of: inv_sense_links.remove(&SenseRelType::BodyPart).unwrap_or_else(|| Vec::new()),
            vehicle: sense_links.remove(&SenseRelType::Vehicle).unwrap_or_else(|| Vec::new()),
            is_vehicle_of: inv_sense_links.remove(&SenseRelType::Vehicle).unwrap_or_else(|| Vec::new())
        })
    }

    pub fn into_synset(self) -> Synset {
        Synset {
            members: self.members.into_iter().map(|m| {
                m.lemma
            }).collect(),
            definition: self.definition,
            example: self.example,
            ili: self.ili,
            wikidata: self.wikidata,
            source: self.source,
            part_of_speech: self.part_of_speech,
            also: self.also,
            attribute: self.attribute,
            causes: self.causes,
            domain_region: self.domain_region,
            domain_topic: self.domain_topic,
            exemplifies: self.exemplifies,
            entails: self.entails,
            hypernym: self.hypernym,
            instance_hypernym: self.instance_hypernym,
            mero_member: self.mero_member,
            mero_part: self.mero_part,
            mero_substance: self.mero_substance,
            mero_location: self.mero_location,
            mero_portion: self.mero_portion,
            meronym: self.meronym,
            similar: self.similar,
            feminine: self.feminine,
            masculine: self.masculine,
            other: self.other
        }
    }
}


