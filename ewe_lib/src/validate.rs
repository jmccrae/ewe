use crate::wordnet::*;
use crate::rels::*;
use crate::sense_keys::get_sense_key2;
use crate::progress::Progress;
use std::fmt;
use std::collections::{HashSet,HashMap};
use lazy_static::lazy_static;
use regex::Regex;
use crate::change_manager;

pub fn validate<L : Lexicon, Bar : Progress>(wn : &L, bar : &mut Bar) -> Result<Vec<ValidationError>> {
    let mut errors = Vec::new();
    bar.start((wn.n_entries()? + 2 * wn.n_synsets()?) as u64);
    bar.set_percent_mode(true);
    let mut sense_keys = HashSet::new();
    let mut definition_index : HashMap<String, SynsetId> = HashMap::new();
    let mut ili_index : HashMap<String, SynsetId> = HashMap::new();
    let mut wikidata_index : HashMap<String, SynsetId> = HashMap::new();
    for entry in wn.entries()? {
        let (lemma, poskey, entry) = entry?;
        bar.inc(1);
        for sense in entry.sense.iter() {
           match get_sense_key2(wn, &lemma, Some(&sense.id), &sense.synset)? {
               Some(sense_key) => {
                   if sense_key != sense.id {
                       errors.push(ValidationError::InvalidSenseId {
                           id: sense.id.clone(),
                           expected: sense_key.clone()
                       });
                   }
               },
               None => {} // No synset error will be checked next.
           }
           match wn.synset_by_id(&sense.synset)? {
               Some(synset) => {
                   if poskey.to_part_of_speech() == None ||
                       synset.part_of_speech != poskey.to_part_of_speech().unwrap() && 
                       !(synset.part_of_speech == PartOfSpeech::s &&
                         poskey.to_part_of_speech().unwrap() == PartOfSpeech::a) {
                        errors.push(ValidationError::EntryPartOfSpeech {
                            id: sense.id.clone(),
                            pos: poskey.clone(),
                            synset_pos: synset.part_of_speech.clone()
                        });
                   }

                   if !synset.members.iter().any(|member| *member == lemma) {
                       errors.push(ValidationError::SenseNotInSynsetMembers {
                           id: sense.synset.clone(),
                           member: lemma.clone()
                       });
                   }
               }, None => {
                   errors.push(ValidationError::SenseSynsetNotExists {
                       id: sense.id.clone(),
                       synset: sense.synset.clone()
                   });
               }
           }
           let mut sr_items = HashSet::new();
           for (rel, raw_target) in sense.sense_links_from() {
               let target = match raw_target.resolve(wn) {
                   Ok(target) => target,
                   Err(_) => {
                       errors.push(ValidationError::SenseRelTargetMissing {
                           id: sense.id.clone(),
                           rel: rel.clone(),
                           target: raw_target.clone()
                       });
                       continue;
                   }
               };

               match poskey.to_part_of_speech() {
                   Some(pos) => {
                       if !rel.pos().iter().any(|p| **p == pos) {
                           errors.push(ValidationError::SenseRelationPOS {
                               id: sense.id.clone(),
                               pos: pos,
                               rel: rel.clone()
                           });
                      }
                   },
                   None => {}
               }
               // No relation type marked `is_symmetric` is sense-synset, so a
               // `Synset` target here would mean the data is already broken
               // in a way `SenseRelationPOS` above will have caught.
               if rel.is_symmetric() {
                   if let SenseOrSynsetId::Sense(target_sense) = &target {
                       if !wn.sense_links_from_id(target_sense)?.iter().any(|(r2, t2)| {
                           *r2 == rel && *t2 == UnresolvedSenseOrSynsetId::Sense(sense.id.clone()) }) {
                           errors.push(ValidationError::SenseRelationSymmetry {
                               source: sense.id.clone(),
                               rel: rel.clone(),
                               target: target.clone()
                           });
                       }
                   }
               }
               if let SenseOrSynsetId::Sense(target_sense) = &target {
                   if sense.id == *target_sense {
                       errors.push(ValidationError::SelfReferencingSenseRelation {
                           source: sense.id.clone(),
                           rel: rel.clone(),
                           target: target.clone() });
                   }
               }
               if sr_items.contains(&(rel.clone(), target.clone())) {
                   errors.push(ValidationError::DuplicateSenseRelation {
                       source: sense.id.clone(),
                       rel, target });
               } else {
                   sr_items.insert((rel, target));
               }
           }
           if sense_keys.contains(&sense.id) {
               errors.push(ValidationError::DuplicateSenseKey {
                   id: sense.id.clone()
               });
           } else {
               sense_keys.insert(sense.id.clone());
           }
           let mut subcat = sense.subcat.clone();
           subcat.sort_unstable();
           subcat.dedup();
           if subcat.len() != sense.subcat.len() {
                errors.push(ValidationError::DuplicateSyntacticBehaviour {
                    id: sense.id.clone()
                });
           }

           for sense2 in entry.sense.iter() {
               if sense.id != sense2.id && sense.synset == sense2.synset {
                   errors.push(ValidationError::DuplicateSense { 
                       id1: sense.id.clone(), id2: sense2.id.clone(), 
                       synset: sense.synset.clone() 
                   });
               }
           }
        }
        if entry.sense.is_empty() {
            errors.push(ValidationError::NoSenses {
                lemma: lemma.clone(),
                poskey: poskey.clone()
            });
        }
    }
    for synset in wn.synsets()? {
        let (synset_id, synset) = synset?;
        bar.inc(1);
        let ssid = synset_id.as_str();
        if ssid[(ssid.len() - 1)..ssid.len()] != *synset.part_of_speech.value() {
            errors.push(ValidationError::SynsetIdPos {
                id: synset_id.clone(),
                pos: synset.part_of_speech.clone()
            });
        }
        if !is_valid_synset_id(&synset_id) {
            errors.push(ValidationError::InvalidSynsetId {
                id: synset_id.clone()
            });
        }

        if synset.members.is_empty() {
            errors.push(ValidationError::EmptySynset {
                id: synset_id.clone()
            });
        }

        match synset.ili {
            Some(ref ili) => {
                if !is_valid_ili(&ili) {
                    errors.push(ValidationError::InvalidILIId {
                        id: synset_id.clone(),
                        ili: ili.clone()
                    });
                }
                if ili.as_str() != "in" {
                    match ili_index.get(ili.as_str()) {
                        Some(prev) => {
                            errors.push(ValidationError::DuplicateILI {
                                id1: prev.clone(),
                                id2: synset_id.clone(),
                                ili: ili.clone()
                            });
                        },
                        None => {
                            ili_index.insert(ili.as_str().to_string(), synset_id.clone());
                        }
                    }
                }
            },
            None => {}
        }

        for qid in synset.wikidata.iter() {
            if !is_valid_wikidata_id(qid) {
                errors.push(ValidationError::InvalidWikidataId {
                    id: synset_id.clone(),
                    qid: qid.clone()
                });
            } else {
                match wikidata_index.get(qid) {
                    Some(prev) => {
                        errors.push(ValidationError::DuplicateWikidataId {
                            id1: prev.clone(),
                            id2: synset_id.clone(),
                            qid: qid.clone()
                        });
                    },
                    None => {
                        wikidata_index.insert(qid.clone(), synset_id.clone());
                    }
                }
            }
        }

        for defn in synset.definition.iter() {
            if !defn.is_empty() {
                match definition_index.get(defn) {
                    Some(prev) => {
                        errors.push(ValidationError::DuplicateDefinition {
                            id1: prev.clone(),
                            id2: synset_id.clone()
                        });
                    },
                    None => {
                        definition_index.insert(defn.clone(), synset_id.clone());
                    }
                }
            }
        }

        let mut sr_items = HashSet::new();
        for (rel, target) in synset.links_from() {
            if !rel.pos().iter().any(|p| **p == synset.part_of_speech) {
                errors.push(ValidationError::SynsetRelationPOS {
                    id: synset_id.clone(),
                    pos: synset.part_of_speech.clone(),
                    rel: rel.clone()
                });
            }
            if rel == SynsetRelType::Hypernym ||
                rel == SynsetRelType::InstanceHypernym {
                match wn.synset_by_id(&target)? {
                    Some(target_synset) => {
                        if synset.part_of_speech != target_synset.part_of_speech {
                            errors.push(ValidationError::CrossPOSHyper {
                                source: synset_id.clone(),
                                target: target.clone()
                            });
                        }
                        if rel == SynsetRelType::Hypernym &&
                            !target_synset.instance_hypernym.is_empty() {
                            errors.push(ValidationError::HypernymTargetIsInstance {
                                source: synset_id.clone(),
                                target: target.clone()
                            });
                        }
                    },
                    None => {
                        errors.push(ValidationError::SynsetRelTargetMissing {
                            id: synset_id.clone(),
                            rel: rel.clone(),
                            target: target.clone()
                        });
                    }
                }
            }
            if rel == SynsetRelType::Similar {
                if let Some(target_synset) = wn.synset_by_id(&target)? {
                    let expected = match synset.part_of_speech {
                        PartOfSpeech::a => Some(PartOfSpeech::s),
                        PartOfSpeech::s => Some(PartOfSpeech::a),
                        _ => None
                    };
                    if let Some(expected) = expected {
                        if target_synset.part_of_speech != expected {
                            errors.push(ValidationError::SimilarTargetPOS {
                                id: synset_id.clone(),
                                target: target.clone()
                            });
                        }
                    }
                }
            }
            if rel.is_symmetric() {
                if !wn.links_from(&target)?.iter().any(|(r2, t2)| {
                    *r2 == rel && *t2 == synset_id }) {
                    errors.push(ValidationError::SynsetRelationSymmetry {
                        source: synset_id.clone(),
                        rel: rel.clone(),
                        target: target.clone()
                    });
                }
            }
            if synset_id == target {
                errors.push(ValidationError::SelfReferencingSynsetRelation {
                    source: synset_id.clone(),
                    rel: rel.clone(),
                    target: target.clone()
                });
            }
             if sr_items.contains(&(rel.clone(), target.clone())) {
                errors.push(ValidationError::DuplicateSynsetRelation {
                    source: synset_id.clone(),
                    rel, target });
            } else {
                sr_items.insert((rel, target));
            }
         }

        if synset.part_of_speech == PartOfSpeech::s &&
            synset.similar.len() != 1 {
                errors.push(ValidationError::SatelliteSimilar {
                    id: synset_id.clone(),
                    n: synset.similar.len()
                });
        }

        if synset.part_of_speech == PartOfSpeech::n &&
            !synset_id.as_str().starts_with("00001740") &&
            synset.hypernym.is_empty() &&
            synset.instance_hypernym.is_empty() {
            errors.push(ValidationError::NoHypernym {
                id: synset_id.clone()
            });
        }

        if !synset.hypernym.is_empty() && !synset.instance_hypernym.is_empty() {
            errors.push(ValidationError::HypernymInstanceConflict {
                id: synset_id.clone()
            });
        }

        if synset.definition.is_empty() ||
            synset.definition.iter().any(|def| def == "") {
            errors.push(ValidationError::Definition {
                id : synset_id.clone()
            });
        }

        match wn.lex_name_for(&synset_id)? {
            Some(lex_name) => {
                if !wn.pos_for_lexfile(&lex_name)?.iter().any(|pos| {
                    *pos == synset.part_of_speech }) {
                    errors.push(ValidationError::Lexfile {
                        id: synset_id.clone(),
                        lexfile: lex_name.clone()
                    });
                }
            },
            None => { // should never happen
            }
        }

        for member in synset.members.iter() {
            if !wn.entry_by_lemma(member)?.iter().
                any(|entry| {
                    entry.sense.iter().any(
                        |sense| {
                            sense.synset == synset_id
                        })
                }) {
                errors.push(ValidationError::SynsetMemberNotInEntries {
                    id: synset_id.clone(), 
                    member: member.to_string()
                });
            }
        }

        for (i, mem1) in synset.members.iter().enumerate() {
            for (j, mem2) in synset.members.iter().enumerate() {
                if i > j && mem1 == mem2 {
                    errors.push(ValidationError::DuplicateMember {
                        id: synset_id.clone(),
                        member: mem1.clone()
                    });
                }
            }
        }

        check_transitive(wn, &mut errors, &synset_id, &synset)?;

    }
    check_no_loops(wn, &mut errors, bar)?;
    bar.finish();
    Ok(errors)
}

fn check_transitive<L : Lexicon>(wn : &L,
                   errors : &mut Vec<ValidationError>,
                   synset_id : &SynsetId, synset : &Synset) -> Result<()> {
    for target in synset.hypernym.iter() {
        match wn.synset_by_id(target)? {
            Some(synset2) => {
                for target2 in synset2.hypernym.iter() {
                    if synset.hypernym.iter().any(|t| t == target2) {
                        errors.push(ValidationError::Transitivity {
                            id1: synset_id.clone(),
                            id2: target.clone(),
                            id3: target2.clone()
                        });
                    }
                }
            },
            None => {} // fails elsewhere
        }
    }
    Ok(())
}

fn check_no_loops<L : Lexicon, ProgressBar : Progress>(wn : &L,
                  errors : &mut Vec<ValidationError>,
                  bar : &mut ProgressBar) -> Result<()> {
    let mut hypernyms = HashMap::new();
    let mut domains = HashMap::new();
    for synsets in wn.synsets()? {
        let (synset_id, synset) = synsets?;
        bar.inc(1);
        hypernyms.insert(synset_id.clone(), HashSet::new());
        for target in synset.hypernym.iter() {
            match hypernyms.get_mut(&synset_id) {
                Some(h) => { h.insert(target.clone()); },
                None => {}
            }
        }
        domains.insert(synset_id.clone(), HashSet::new());
        for target in synset.domain_region.iter() {
            match domains.get_mut(&synset_id) {
                Some(h) => { h.insert(target.clone()); },
                None => {}
            }
        }
        for target in synset.domain_topic.iter() {
            match domains.get_mut(&synset_id) {
                Some(h) => { h.insert(target.clone()); },
                None => {}
            }
        }
        for target in synset.exemplifies.iter() {
            match domains.get_mut(&synset_id) {
                Some(h) => { h.insert(target.clone()); },
                None => {}
            }
        }
    }
    let mut changed = true;
    while changed {
        changed = false;
        for synsets in wn.synsets()? {
            let (synset_id, _) = synsets?;
            let n_size = hypernyms[&synset_id].len();
            for c in hypernyms[&synset_id].clone() {
                let extension : Vec<SynsetId> = 
                    hypernyms.get(&c).iter().
                    flat_map(|x| x.iter()).
                    map(|x| x.clone()).collect();
                match hypernyms.get_mut(&synset_id) {
                    Some(h) => h.extend(extension.into_iter()),
                    None => {}
                }
            }
            if hypernyms[&synset_id].len() != n_size {
                changed = true;
            }
            if hypernyms[&synset_id].contains(&synset_id) {
                errors.push(ValidationError::Loop {
                    id: synset_id.clone()
                });
            }
            let n_size_dom = domains[&synset_id].len();
            for c in domains[&synset_id].clone() {
                let extension : Vec<SynsetId> = 
                    domains.get(&c).iter().
                    flat_map(|x| x.iter()).
                    map(|x| x.clone()).collect();
                match domains.get_mut(&synset_id) {
                    Some(h) => h.extend(extension.into_iter()),
                    None => {}
                }
            }
            if domains[&synset_id].len() != n_size_dom {
                changed = true;
            }
            if domains[&synset_id].contains(&synset_id) {
                errors.push(ValidationError::DomainLoop {
                    id: synset_id.clone()
                });
            }
         }
    }
    Ok(())
}

lazy_static! {
   static ref VALID_SYNSET_ID : Regex = Regex::new("^[0-9]{8}-[nvars]$").unwrap();
   static ref VALID_ILI : Regex = Regex::new("^i\\d+$").unwrap();
   static ref VALID_WIKIDATA_ID : Regex = Regex::new("^Q[1-9][0-9]*$").unwrap();
}

fn is_valid_synset_id(synset_id : &SynsetId) -> bool {
    VALID_SYNSET_ID.is_match(synset_id.as_str())
}

fn is_valid_ili(iliid : &ILIID) -> bool {
    VALID_ILI.is_match(iliid.as_str())
}

fn is_valid_wikidata_id(qid : &str) -> bool {
    VALID_WIKIDATA_ID.is_match(qid)
}

pub enum ValidationError {
    InvalidSenseId { id : SenseId, expected : SenseId },
    SenseSynsetNotExists { id : SenseId, synset : SynsetId },
    EntryPartOfSpeech { id : SenseId, pos : PosKey, synset_pos : PartOfSpeech },
    SenseRelationPOS { id : SenseId, pos : PartOfSpeech, rel : SenseRelType },
    SynsetRelationPOS { id : SynsetId, pos : PartOfSpeech, rel : SynsetRelType },
    DuplicateSenseRelation { source : SenseId, rel : SenseRelType, target : SenseOrSynsetId },
    SelfReferencingSenseRelation { source : SenseId, rel : SenseRelType, target : SenseOrSynsetId },
    SelfReferencingSynsetRelation { source : SynsetId, rel : SynsetRelType, target : SynsetId },
    DuplicateSynsetRelation { source : SynsetId, rel : SynsetRelType, target : SynsetId },
    DuplicateSenseKey { id : SenseId },
    DuplicateSyntacticBehaviour { id : SenseId },
    DuplicateSense { id1 : SenseId, id2 : SenseId, synset : SynsetId },
    SynsetIdPos { id : SynsetId, pos : PartOfSpeech },
    InvalidSynsetId { id : SynsetId },
    EmptySynset { id : SynsetId },
    InvalidILIId { id : SynsetId, ili: ILIID },
    NoSenses { lemma : String, poskey : PosKey },
    CrossPOSHyper { source : SynsetId, target : SynsetId },
    SenseRelTargetMissing { id : SenseId, rel : SenseRelType, target : UnresolvedSenseOrSynsetId },
    SynsetRelTargetMissing { id : SynsetId, rel : SynsetRelType, target : SynsetId },
    SatelliteSimilar { id: SynsetId, n: usize },
    NoHypernym { id: SynsetId },
    Definition { id: SynsetId },
    Lexfile { id: SynsetId, lexfile : String },
    SenseRelationSymmetry { source : SenseId, rel : SenseRelType, target : SenseOrSynsetId },
    SynsetRelationSymmetry { source : SynsetId, rel : SynsetRelType, target : SynsetId },
    Transitivity { id1 : SynsetId, id2 : SynsetId, id3 : SynsetId },
    Loop { id: SynsetId },
    DomainLoop { id: SynsetId },
    SynsetMemberNotInEntries { id: SynsetId, member: String },
    DuplicateMember { id: SynsetId, member : String },
    SenseNotInSynsetMembers { id: SynsetId, member: String },
    SimilarTargetPOS { id: SynsetId, target: SynsetId },
    HypernymInstanceConflict { id: SynsetId },
    HypernymTargetIsInstance { source: SynsetId, target: SynsetId },
    DuplicateDefinition { id1: SynsetId, id2: SynsetId },
    DuplicateILI { id1: SynsetId, id2: SynsetId, ili: ILIID },
    InvalidWikidataId { id: SynsetId, qid: String },
    DuplicateWikidataId { id1: SynsetId, id2: SynsetId, qid: String }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::InvalidSenseId { id, expected } =>
                write!(f, "Sense has id {} but should be {}", id.as_str(),
                    expected.as_str()),
            ValidationError::SenseSynsetNotExists { id, synset } =>
                write!(f, "Sense {} refers to synset {} that does not exist", 
                       id.as_str(), synset.as_str()),
            ValidationError::EntryPartOfSpeech { id, pos, synset_pos } =>
                write!(f, "Sense {} is an entry with POS key {}, but the synset has part of speech {}", 
                       id.as_str(), pos.as_str(), synset_pos.value()),
            ValidationError::SenseRelationPOS { id, pos, rel } =>
                write!(f, "Sense {} has a relation of type {} but this is not permitted for part of speech {}", 
                       id.as_str(), rel.value(), pos.value()),
            ValidationError::SynsetRelationPOS { id, pos, rel } =>
                write!(f, "Synset {} has a relation of type {} but this is not permitted for part of speech {}", 
                       id.as_str(), rel.value(), pos.value()),
            ValidationError::SelfReferencingSenseRelation { source, rel, target } =>
                write!(f, "Self-referencing relation {} ={}=> {}", 
                       source.as_str(), rel.value(), target.as_str()),
            ValidationError::SelfReferencingSynsetRelation { source, rel, target } =>
                write!(f, "Self-referencing relation {} ={}=> {}", 
                       source.as_str(), rel.value(), target.as_str()),
            ValidationError::DuplicateSenseRelation { source, rel, target } =>
                write!(f, "Duplicate relation {} ={}=> {}", 
                       source.as_str(), rel.value(), target.as_str()),
            ValidationError::DuplicateSynsetRelation { source, rel, target } =>
                write!(f, "Duplicate relation {} ={}=> {}", 
                       source.as_str(), rel.value(), target.as_str()),
            ValidationError::DuplicateSenseKey { id } =>
                write!(f, "Duplicate sense key {}", id.as_str()),
            ValidationError::DuplicateSyntacticBehaviour { id } =>
                write!(f, "Duplicate syntactic behaviour for sense {}", 
                       id.as_str()),
            ValidationError::DuplicateSense { id1, id2, synset } => 
                write!(f, "Duplicate senses {} & {} referring to {}", 
                       id1.as_str(), id2.as_str(), synset.as_str()),
            ValidationError::SynsetIdPos { id, pos } =>
                write!(f, "Synset {} is not valid for a synset with POS {}",
                       id.as_str(), pos.value()),
            ValidationError::InvalidSynsetId { id } =>
                write!(f, "Invalid synset id: {}", id.as_str()),
            ValidationError::EmptySynset { id } =>
                write!(f, "Empty synset: {}", id.as_str()),
            ValidationError::InvalidILIId { id, ili } =>
                write!(f, "Synset {} has an invalid ILI identifier {}", id.as_str(), ili.as_str()),
            ValidationError::NoSenses { lemma, poskey } =>
                write!(f, "Entry for {} ({}) has no senses", lemma, poskey.as_str()),
            ValidationError::CrossPOSHyper { source, target } =>
                write!(f, "Hypernym from {} to {} is across part of speech values",
                       source.as_str(), target.as_str()),
            ValidationError::SenseRelTargetMissing { id, rel, target } =>
                write!(f, "Sense {} refers to {} with relation {}, but this does not exist",
                       id.as_str(), target.as_str(), rel.value()),
            ValidationError::SynsetRelTargetMissing { id, rel, target } =>
                write!(f, "Sense {} refers to {} with relation {}, but this does not exist",
                       id.as_str(), target.as_str(), rel.value()),
           ValidationError::SatelliteSimilar { id, n } => 
               write!(f, "Satellite adjective {} should have exactly one similar link but has {}",
                      id.as_str(), n),
            ValidationError::NoHypernym { id } =>
                write!(f, "No hypernym for {}", id.as_str()),
            ValidationError::Definition { id } =>
                write!(f, "No definition or empty definition for {}", id.as_str()),
            ValidationError::Lexfile { id, lexfile } =>
                write!(f, "{} defined in wrong lexicographer file {}",
                       id.as_str(), lexfile),
            ValidationError::SenseRelationSymmetry { source, rel, target } =>
                write!(f, "No symmetric relation from {} to ({}) {}",
                       source.as_str(), rel.value(), target.as_str()),
            ValidationError::SynsetRelationSymmetry { source, rel, target } =>
                write!(f, "No symmetric relation from {} to ({}) {}",
                       source.as_str(), rel.value(), target.as_str()),
            ValidationError::Transitivity { id1, id2, id3 } => 
                write!(f, "{} has direct link to {} but also indirect link through {}",
                       id1.as_str(), id3.as_str(), id2.as_str()),
            ValidationError::Loop { id } => 
                write!(f, "{} is a hypernym of itself", id.as_str()),
            ValidationError::DomainLoop { id } => 
                write!(f, "{} has a domain loop", id.as_str()),
            ValidationError::SynsetMemberNotInEntries { id, member } =>
                write!(f, "{} has member {} but not listed as a sense", id.as_str(), member),
            ValidationError::DuplicateMember { id, member } =>
                write!(f, "{} has duplicate member {}", id.as_str(), member),
            ValidationError::SenseNotInSynsetMembers { id, member } =>
                write!(f, "{} does not contain {} in member list", id.as_str(), member),
            ValidationError::SimilarTargetPOS { id, target } =>
                write!(f, "Similar relation from {} to {} does not link an adjective to its satellite",
                       id.as_str(), target.as_str()),
            ValidationError::HypernymInstanceConflict { id } =>
                write!(f, "{} has both a hypernym and an instance_hypernym relation", id.as_str()),
            ValidationError::HypernymTargetIsInstance { source, target } =>
                write!(f, "Hypernym from {} targets {}, which is itself an instance",
                       source.as_str(), target.as_str()),
            ValidationError::DuplicateDefinition { id1, id2 } =>
                write!(f, "{} and {} have the same definition text", id1.as_str(), id2.as_str()),
            ValidationError::DuplicateILI { id1, id2, ili } =>
                write!(f, "{} and {} both use ILI {}", id1.as_str(), id2.as_str(), ili.as_str()),
            ValidationError::InvalidWikidataId { id, qid } =>
                write!(f, "{} has an invalid Wikidata id {}", id.as_str(), qid),
            ValidationError::DuplicateWikidataId { id1, id2, qid } =>
                write!(f, "{} and {} both use Wikidata id {}", id1.as_str(), id2.as_str(), qid)
        }
    }
}

/// Fix the validation error if possible
///
/// This function is used to fix the validation error if possible.
///
/// # Arguments
/// 
/// * `error` - The validation error to fix
///
/// # Returns
///
/// * `true` if the error was fixed, `false` otherwise
pub fn fix<L : Lexicon>(wn : &mut L,
           error : &ValidationError, change_list : &mut change_manager::ChangeList) -> Result<bool> {
    Ok(match error {
        ValidationError::InvalidSenseId { id, expected } => {
            wn.update_sense_key(id, expected)?;
            true
        },
        ValidationError::SenseSynsetNotExists { .. } => false,
        ValidationError::EntryPartOfSpeech { .. } => false,
        ValidationError::SenseRelationPOS { .. } => false,
        ValidationError::SynsetRelationPOS { .. } => false,
        ValidationError::SelfReferencingSenseRelation { source, target, .. } => {
            change_manager::delete_sense_rel(wn, source, target, change_list)?;
            true
        },
        ValidationError::SelfReferencingSynsetRelation { source, target, .. } => {
            change_manager::delete_rel(wn, source, target, change_list);
            true
        },
        ValidationError::DuplicateSenseRelation { source, rel, target } => {
            change_manager::delete_sense_rel(wn, source, target, change_list)?;
            change_manager::insert_sense_relation(wn, source.clone(), rel.clone(), target.clone(), change_list)?;
            true
        },
        ValidationError::DuplicateSynsetRelation { source, rel, target } => {
            change_manager::delete_rel(wn, source, target, change_list);
            change_manager::insert_rel(wn, source, rel, target, change_list)?;
            true
        },
        ValidationError::DuplicateSenseKey { .. } => false,
        ValidationError::DuplicateSyntacticBehaviour { .. } => false,
        ValidationError::DuplicateSense { .. } =>  false,
        ValidationError::SynsetIdPos { .. } => false,
        ValidationError::InvalidSynsetId { .. } => false,
        ValidationError::EmptySynset { .. } => false,
        ValidationError::InvalidILIId { .. } => false,
        ValidationError::NoSenses { .. } => false,
        ValidationError::CrossPOSHyper { .. } => false,
        ValidationError::SenseRelTargetMissing { .. } => false,
        ValidationError::SynsetRelTargetMissing { .. } => false,
        ValidationError::SatelliteSimilar { .. } =>  false,
        ValidationError::NoHypernym { .. } => false,
        ValidationError::Definition { .. } => false,
        ValidationError::Lexfile { .. } => false,
        ValidationError::SenseRelationSymmetry { source, rel, target } => {
            change_manager::insert_sense_relation(wn, source.clone(), rel.clone(), target.clone(), change_list)?;
            true
        },
        ValidationError::SynsetRelationSymmetry { source, rel, target } => {

            change_manager::insert_rel(wn, target, rel, source, change_list)?;
            true
        },
        ValidationError::Transitivity { id1, id2, id3 } =>  {
            change_manager::delete_rel(wn, id1, id2, change_list);
            change_manager::delete_rel(wn, id2, id3, change_list);
            true
        },
        ValidationError::Loop { .. } =>  false,
        ValidationError::DomainLoop { .. } =>  false,
        ValidationError::SynsetMemberNotInEntries { .. } => false,
        ValidationError::DuplicateMember { id, .. } => {
            match wn.synset_by_id(id)? {
                Some(synset) => {
                    let mut seen = HashSet::new();
                    let deduped : Vec<String> = synset.members.iter()
                        .filter(|m| seen.insert((*m).clone()))
                        .cloned().collect();
                    change_manager::change_members(wn, id, deduped, change_list)?;
                    true
                },
                None => false
            }
        },
        ValidationError::SenseNotInSynsetMembers { .. } => false,
        ValidationError::SimilarTargetPOS { .. } => false,
        ValidationError::HypernymInstanceConflict { .. } => false,
        ValidationError::HypernymTargetIsInstance { .. } => false,
        ValidationError::DuplicateDefinition { .. } => false,
        ValidationError::DuplicateILI { .. } => false,
        ValidationError::InvalidWikidataId { .. } => false,
        ValidationError::DuplicateWikidataId { .. } => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wordnet::LexiconHashMapBackend;
    use crate::progress::NullProgress;

    fn add_noun<L : Lexicon>(wn : &mut L, ssid : &str, definition : &str,
                pos : char, change_list : &mut change_manager::ChangeList) -> SynsetId {
        let lexfile = match pos {
            'n' => "noun.object",
            'v' => "verb.change",
            'a' | 's' => "adj.all",
            _ => "adv.all"
        };
        change_manager::add_synset(wn, definition.to_owned(), lexfile.to_owned(),
            PosKey::new(pos.to_string()), Some(SynsetId::new(ssid)), change_list).unwrap()
    }

    fn validate_errors<L : Lexicon>(wn : &L) -> Vec<ValidationError> {
        let mut bar = NullProgress;
        validate(wn, &mut bar).unwrap()
    }

    #[test]
    fn test_similar_target_pos() {
        let mut wn = LexiconHashMapBackend::new();
        let mut change_list = change_manager::ChangeList::new();
        let a = add_noun(&mut wn, "00000001-a", "adjective a", 'a', &mut change_list);
        let b = add_noun(&mut wn, "00000002-a", "adjective b (should be satellite)", 'a', &mut change_list);
        wn.update_synset(&a, |ss| { ss.similar.push(b.clone()); }).unwrap();

        let errors = validate_errors(&wn);
        assert!(errors.iter().any(|e| matches!(e,
            ValidationError::SimilarTargetPOS { id, target } if *id == a && *target == b)));
    }

    #[test]
    fn test_hypernym_instance_conflict() {
        let mut wn = LexiconHashMapBackend::new();
        let mut change_list = change_manager::ChangeList::new();
        let main = add_noun(&mut wn, "00000010-n", "main synset", 'n', &mut change_list);
        let hyper_target = add_noun(&mut wn, "00000011-n", "hypernym target", 'n', &mut change_list);
        let instance_target = add_noun(&mut wn, "00000012-n", "instance target", 'n', &mut change_list);
        wn.update_synset(&main, |ss| {
            ss.hypernym.push(hyper_target.clone());
            ss.instance_hypernym.push(instance_target.clone());
        }).unwrap();

        let errors = validate_errors(&wn);
        assert!(errors.iter().any(|e| matches!(e,
            ValidationError::HypernymInstanceConflict { id } if *id == main)));
    }

    #[test]
    fn test_hypernym_target_is_instance() {
        let mut wn = LexiconHashMapBackend::new();
        let mut change_list = change_manager::ChangeList::new();
        let root = add_noun(&mut wn, "00000020-n", "root", 'n', &mut change_list);
        let d = add_noun(&mut wn, "00000021-n", "instance synset", 'n', &mut change_list);
        let e = add_noun(&mut wn, "00000022-n", "hypernym-of-instance synset", 'n', &mut change_list);
        wn.update_synset(&d, |ss| { ss.instance_hypernym.push(root.clone()); }).unwrap();
        wn.update_synset(&e, |ss| { ss.hypernym.push(d.clone()); }).unwrap();

        let errors = validate_errors(&wn);
        assert!(errors.iter().any(|e2| matches!(e2,
            ValidationError::HypernymTargetIsInstance { source, target } if *source == e && *target == d)));
    }

    #[test]
    fn test_duplicate_definition() {
        let mut wn = LexiconHashMapBackend::new();
        let mut change_list = change_manager::ChangeList::new();
        let a = add_noun(&mut wn, "00000030-n", "shared definition", 'n', &mut change_list);
        let b = add_noun(&mut wn, "00000031-n", "shared definition", 'n', &mut change_list);

        let errors = validate_errors(&wn);
        assert!(errors.iter().any(|e| matches!(e,
            ValidationError::DuplicateDefinition { id1, id2 } if
                (*id1 == a && *id2 == b) || (*id1 == b && *id2 == a))));
    }

    #[test]
    fn test_duplicate_ili() {
        let mut wn = LexiconHashMapBackend::new();
        let mut change_list = change_manager::ChangeList::new();
        let a = add_noun(&mut wn, "00000040-n", "ili synset a", 'n', &mut change_list);
        let b = add_noun(&mut wn, "00000041-n", "ili synset b", 'n', &mut change_list);
        wn.update_synset(&a, |ss| { ss.ili = Some(ILIID::new("i12345")); }).unwrap();
        wn.update_synset(&b, |ss| { ss.ili = Some(ILIID::new("i12345")); }).unwrap();

        let errors = validate_errors(&wn);
        assert!(errors.iter().any(|e| matches!(e,
            ValidationError::DuplicateILI { id1, id2, ili } if
                ili.as_str() == "i12345" &&
                ((*id1 == a && *id2 == b) || (*id1 == b && *id2 == a)))));
    }

    #[test]
    fn test_invalid_wikidata_id() {
        let mut wn = LexiconHashMapBackend::new();
        let mut change_list = change_manager::ChangeList::new();
        let a = add_noun(&mut wn, "00000050-n", "wikidata synset", 'n', &mut change_list);
        wn.update_synset(&a, |ss| { ss.wikidata.push("not-a-qid".to_owned()); }).unwrap();

        let errors = validate_errors(&wn);
        assert!(errors.iter().any(|e| matches!(e,
            ValidationError::InvalidWikidataId { id, qid } if *id == a && qid == "not-a-qid")));
    }

    #[test]
    fn test_duplicate_wikidata_id() {
        let mut wn = LexiconHashMapBackend::new();
        let mut change_list = change_manager::ChangeList::new();
        let a = add_noun(&mut wn, "00000060-n", "wikidata synset a", 'n', &mut change_list);
        let b = add_noun(&mut wn, "00000061-n", "wikidata synset b", 'n', &mut change_list);
        wn.update_synset(&a, |ss| { ss.wikidata.push("Q123".to_owned()); }).unwrap();
        wn.update_synset(&b, |ss| { ss.wikidata.push("Q123".to_owned()); }).unwrap();

        let errors = validate_errors(&wn);
        assert!(errors.iter().any(|e| matches!(e,
            ValidationError::DuplicateWikidataId { id1, id2, qid } if
                qid == "Q123" &&
                ((*id1 == a && *id2 == b) || (*id1 == b && *id2 == a)))));
    }

    #[test]
    fn test_fix_duplicate_synset_relation() {
        let mut wn = LexiconHashMapBackend::new();
        let mut change_list = change_manager::ChangeList::new();
        let a = add_noun(&mut wn, "00000070-n", "source synset", 'n', &mut change_list);
        let b = add_noun(&mut wn, "00000071-n", "target synset", 'n', &mut change_list);
        wn.update_synset(&a, |ss| {
            ss.hypernym.push(b.clone());
            ss.hypernym.push(b.clone());
        }).unwrap();

        let errors = validate_errors(&wn);
        let dup = errors.iter().find(|e| matches!(e,
            ValidationError::DuplicateSynsetRelation { source, target, .. } if *source == a && *target == b))
            .expect("expected a DuplicateSynsetRelation error");

        assert!(fix(&mut wn, dup, &mut change_list).unwrap());

        let synset = wn.synset_by_id(&a).unwrap().unwrap();
        assert_eq!(synset.hypernym.iter().filter(|t| **t == b).count(), 1);
    }

    #[test]
    fn test_fix_duplicate_member() {
        let mut wn = LexiconHashMapBackend::new();
        let mut change_list = change_manager::ChangeList::new();
        let a = add_noun(&mut wn, "00000080-n", "member synset", 'n', &mut change_list);
        wn.update_synset(&a, |ss| {
            ss.members.push("duplicated".to_owned());
            ss.members.push("duplicated".to_owned());
        }).unwrap();

        let errors = validate_errors(&wn);
        let dup = errors.iter().find(|e| matches!(e,
            ValidationError::DuplicateMember { id, member } if *id == a && member == "duplicated"))
            .expect("expected a DuplicateMember error");

        assert!(fix(&mut wn, dup, &mut change_list).unwrap());

        let synset = wn.synset_by_id(&a).unwrap().unwrap();
        assert_eq!(synset.members.iter().filter(|m| **m == "duplicated").count(), 1);
    }

    #[test]
    fn test_fix_duplicate_sense_relation() {
        let mut wn = LexiconHashMapBackend::new();
        let mut change_list = change_manager::ChangeList::new();
        let ss1 = add_noun(&mut wn, "00000090-n", "first sense synset", 'n', &mut change_list);
        let ss2 = add_noun(&mut wn, "00000091-n", "second sense synset", 'n', &mut change_list);
        let sense1 = change_manager::add_entry(&mut wn, ss1.clone(), "firstword".to_owned(),
            PosKey::new("n".to_owned()), Vec::new(), None, &mut change_list).unwrap().unwrap();
        let sense2 = change_manager::add_entry(&mut wn, ss2.clone(), "secondword".to_owned(),
            PosKey::new("n".to_owned()), Vec::new(), None, &mut change_list).unwrap().unwrap();

        change_manager::insert_sense_relation(&mut wn, sense1.clone(), SenseRelType::Antonym,
            SenseOrSynsetId::Sense(sense2.clone()), &mut change_list).unwrap();

        let dup = ValidationError::DuplicateSenseRelation {
            source: sense1.clone(), rel: SenseRelType::Antonym,
            target: SenseOrSynsetId::Sense(sense2.clone())
        };
        assert!(fix(&mut wn, &dup, &mut change_list).unwrap());

        let links = wn.sense_links_from_id(&sense1).unwrap();
        assert_eq!(links.iter().filter(|(r, t)|
            *r == SenseRelType::Antonym && *t == UnresolvedSenseOrSynsetId::Sense(sense2.clone())).count(), 1);
    }
}
