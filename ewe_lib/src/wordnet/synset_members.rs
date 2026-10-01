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
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub members : Vec<Member>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub definition : ScoredVec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub example : Vec<Example>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ili : Option<ILIID>,
    #[serde(default, deserialize_with = "string_or_vec")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub wikidata : Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source : Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence : Option<f64>,
    #[serde(rename="partOfSpeech")]
    pub part_of_speech : PartOfSpeech,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub also : ScoredVec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub attribute : ScoredVec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub causes : ScoredVec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub domain_region : ScoredVec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub domain_topic : ScoredVec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exemplifies : ScoredVec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub entails : ScoredVec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub hypernym : ScoredVec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub instance_hypernym : ScoredVec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mero_location : ScoredVec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mero_member : ScoredVec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mero_part : ScoredVec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mero_portion : ScoredVec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mero_substance : ScoredVec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub meronym : ScoredVec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub similar : ScoredVec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub feminine : ScoredVec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub masculine : ScoredVec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub other : ScoredVec<SynsetId>,

    // Inverse fields
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub hyponym : ScoredVec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_caused_by: ScoredVec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub has_domain_region: ScoredVec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub has_domain_topic: ScoredVec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_exemplified_by: ScoredVec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_entailed_by: ScoredVec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub instance_hyponym: ScoredVec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub holo_location: ScoredVec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub holo_member: ScoredVec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub holo_part: ScoredVec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub holo_portion: ScoredVec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub holo_substance: ScoredVec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub holonym: ScoredVec<SynsetId>,

    // Sense Relations
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub antonym: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub also_sense: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub similar_sense: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub participle: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_participle_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pertainym: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub derivation: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub domain_topic_sense: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub has_domain_topic_sense: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub domain_region_sense: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub has_domain_region_sense: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exemplifies_sense: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_exemplified_by_sense: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub agent: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_agent_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub material: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_material_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub event: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_event_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub instrument: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_instrument_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub location: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_location_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub by_means_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_by_means_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub undergoer: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_undergoer_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub property: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_property_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub result: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_result_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub state: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_state_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub uses: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_used_by: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub destination: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_destination_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub body_part: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_body_part_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub vehicle: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_vehicle_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub other_sense: Vec<SenseRelation>
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
    pub poskey : PosKey,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_no : Option<u32>,
    /// The confidence of the whole lexical entry (as opposed to `sense.confidence`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_confidence : Option<f64>
}

#[derive(Debug, PartialEq, Serialize, Deserialize,Clone)]
#[cfg_attr(feature="redb", derive(speedy::Readable, speedy::Writable))]
pub struct MemberSense {
    pub id : SenseId,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub subcat: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adjposition: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
}
 
#[derive(Debug, PartialEq, Serialize, Deserialize,Clone)]
#[cfg_attr(feature="redb", derive(speedy::Readable, speedy::Writable))]
pub struct SenseRelation {
    pub target_synset: SynsetId,
    pub source_lemma: String,
    /// `None` when the target is a bare synset rather than a specific sense
    /// (only possible for the sense-synset relations: domain_topic,
    /// domain_region, exemplifies, other).
    pub target_lemma: Option<String>,
    pub target_poskey: Option<PosKey>,
    /// The relation's confidence. For an inverse relation (e.g. `is_agent_of`) this is the
    /// score stored on the forward relation it was derived from.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>
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
                            subcat: sense.subcat.clone(),
                            adjposition: sense.adjposition.clone(),
                            confidence: sense.confidence
                        },
                        form: entry.form.clone(),
                        pronunciation: entry.pronunciation.clone(),
                        poskey: poskey.clone(),
                        entry_no: poskey.entry_no(),
                        entry_confidence: entry.confidence
                    });
                    macro_rules! extract_sense_rel {
                        ($rel:ident,$name:ident) => {
                            for (target, confidence) in sense.$rel.iter_scored() {
                                if let Some((target_lemma, target_poskey, target_sense)) = lexicon.get_sense_by_id(target)? {
                                    sense_links.entry(SenseRelType::$name)
                                        .or_insert_with(|| Vec::new())
                                        .push(SenseRelation {
                                            target_synset: target_sense.synset.clone(),
                                            source_lemma: m.clone(),
                                            target_lemma: Some(target_lemma.clone()),
                                            target_poskey: Some(target_poskey.clone()),
                                            confidence
                                        });
                                }
                            }
                        }
                    }
                    // domain_topic/domain_region/exemplifies/other can target either a
                    // sense or a bare synset - resolve each entry against the lexicon
                    // and branch; an entry that fails to resolve (a dangling reference)
                    // is skipped here, since validate() is what reports it.
                    macro_rules! extract_sense_or_synset_rel {
                        ($rel:ident,$name:ident) => {
                            for (target, confidence) in sense.$rel.iter_scored() {
                                match target.resolve(lexicon) {
                                    Ok(SenseOrSynsetId::Sense(target_sense_id)) => {
                                        if let Some((target_lemma, target_poskey, target_sense)) = lexicon.get_sense_by_id(&target_sense_id)? {
                                            sense_links.entry(SenseRelType::$name)
                                                .or_insert_with(|| Vec::new())
                                                .push(SenseRelation {
                                                    target_synset: target_sense.synset.clone(),
                                                    source_lemma: m.clone(),
                                                    target_lemma: Some(target_lemma.clone()),
                                                    target_poskey: Some(target_poskey.clone()),
                                                    confidence
                                                });
                                        }
                                    }
                                    Ok(SenseOrSynsetId::Synset(target_synset_id)) => {
                                        sense_links.entry(SenseRelType::$name)
                                            .or_insert_with(|| Vec::new())
                                            .push(SenseRelation {
                                                target_synset: target_synset_id.clone(),
                                                source_lemma: m.clone(),
                                                target_lemma: None,
                                                target_poskey: None,
                                                confidence
                                            });
                                    }
                                    Err(_) => {}
                                }
                            }
                        }
                    }
                    extract_sense_rel!(antonym,Antonym);
                    extract_sense_rel!(also,Also);
                    extract_sense_rel!(similar,Similar);
                    extract_sense_rel!(participle,Participle);
                    extract_sense_rel!(pertainym,Pertainym);
                    extract_sense_rel!(derivation,Derivation);
                    extract_sense_or_synset_rel!(domain_topic,DomainTopic);
                    extract_sense_or_synset_rel!(domain_region,DomainRegion);
                    extract_sense_rel!(agent,Agent);
                    extract_sense_or_synset_rel!(exemplifies,Exemplifies);
                    extract_sense_or_synset_rel!(other,Other);
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
                            if let Some((target_lemma, target_poskey, target_sense)) = lexicon.get_sense_by_id(&target_sense_id)? {
                                if let Some(inv_rel) = rel.inverse() {
                                    inv_sense_links.
                                        entry(inv_rel).
                                        or_insert_with(|| Vec::new()).
                                        push(SenseRelation {
                                            target_synset: target_sense.synset.clone(),
                                            source_lemma: m.clone(),
                                            target_lemma: Some(target_lemma.clone()),
                                            target_poskey: Some(target_poskey.clone()),
                                            confidence: target_sense.rel_confidence(rel, sense.id.as_str())
                                        });
                                }
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
                if let Some(inv_rel) = rel.inverse() {
                    // The score lives on the forward relation, i.e. on the source synset.
                    let confidence = match lexicon.synset_by_id(&target)? {
                        Some(source) => source.rel_confidence(&rel, synset_id),
                        None => None
                    };
                    links.entry(inv_rel).or_insert_with(|| ScoredVec::new())
                        .push_scored(target.clone(), confidence);
                }
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
            confidence: synset.confidence,
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
            hyponym: links.remove(&SynsetRelType::Hyponym).unwrap_or_else(|| ScoredVec::new()),
            is_caused_by: links.remove(&SynsetRelType::IsCausedBy).unwrap_or_else(|| ScoredVec::new()),
            has_domain_region: links.remove(&SynsetRelType::HasDomainRegion).unwrap_or_else(|| ScoredVec::new()),
            has_domain_topic: links.remove(&SynsetRelType::HasDomainTopic).unwrap_or_else(|| ScoredVec::new()),
            is_exemplified_by: links.remove(&SynsetRelType::IsExemplifiedBy).unwrap_or_else(|| ScoredVec::new()),
            is_entailed_by: links.remove(&SynsetRelType::IsEntailedBy).unwrap_or_else(|| ScoredVec::new()),
            instance_hyponym: links.remove(&SynsetRelType::InstanceHyponym).unwrap_or_else(|| ScoredVec::new()),
            holo_location: links.remove(&SynsetRelType::HoloLocation).unwrap_or_else(|| ScoredVec::new()),
            holo_member: links.remove(&SynsetRelType::HoloMember).unwrap_or_else(|| ScoredVec::new()),
            holo_part: links.remove(&SynsetRelType::HoloPart).unwrap_or_else(|| ScoredVec::new()),
            holo_portion: links.remove(&SynsetRelType::HoloPortion).unwrap_or_else(|| ScoredVec::new()),
            holo_substance: links.remove(&SynsetRelType::HoloSubstance).unwrap_or_else(|| ScoredVec::new()),
            holonym: links.remove(&SynsetRelType::Holonym).unwrap_or_else(|| ScoredVec::new()),
            antonym: sense_links.remove(&SenseRelType::Antonym).unwrap_or_else(|| Vec::new()),
            also_sense: sense_links.remove(&SenseRelType::Also).unwrap_or_else(|| Vec::new()),
            similar_sense: sense_links.remove(&SenseRelType::Similar).unwrap_or_else(|| Vec::new()),
            participle: sense_links.remove(&SenseRelType::Participle).unwrap_or_else(|| Vec::new()),
            is_participle_of: inv_sense_links.remove(&SenseRelType::Participle).unwrap_or_else(|| Vec::new()),
            pertainym: sense_links.remove(&SenseRelType::Pertainym).unwrap_or_else(|| Vec::new()),
            derivation: sense_links.remove(&SenseRelType::Derivation).unwrap_or_else(|| Vec::new()),
            domain_topic_sense: sense_links.remove(&SenseRelType::DomainTopic).unwrap_or_else(|| Vec::new()),
            has_domain_topic_sense: inv_sense_links.remove(&SenseRelType::HasDomainTopic).unwrap_or_else(|| Vec::new()),
            domain_region_sense: sense_links.remove(&SenseRelType::DomainRegion).unwrap_or_else(|| Vec::new()),
            has_domain_region_sense: inv_sense_links.remove(&SenseRelType::HasDomainRegion).unwrap_or_else(|| Vec::new()),
            exemplifies_sense: sense_links.remove(&SenseRelType::Exemplifies).unwrap_or_else(|| Vec::new()),
            is_exemplified_by_sense: inv_sense_links.remove(&SenseRelType::IsExemplifiedBy).unwrap_or_else(|| Vec::new()),
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
            is_vehicle_of: inv_sense_links.remove(&SenseRelType::Vehicle).unwrap_or_else(|| Vec::new()),
            other_sense: sense_links.remove(&SenseRelType::Other).unwrap_or_else(|| Vec::new())
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
            confidence: self.confidence,
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


