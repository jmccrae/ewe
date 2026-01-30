use crate::wordnet::*;
use crate::rels::*;
use crate::sense_keys::get_sense_key2;
use std::fmt;
use std::collections::{HashSet,HashMap};
use indicatif::{ProgressBar,ProgressStyle};
use lazy_static::lazy_static;
use regex::Regex;
use crate::change_manager;

pub fn validate<L : Lexicon>(wn : &L) -> Result<Vec<ValidationError>> {
    let mut errors = Vec::new();
    println!("Validating");
    let bar = ProgressBar::new((wn.n_entries()? + 2 * wn.n_synsets()?) as u64);
    bar.set_style(ProgressStyle::default_bar()
                  .template("{wide_bar} {percent}%"));
    let mut sense_keys = HashSet::new();
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
           for (rel, target) in sense.sense_links_from() {
               if !wn.has_sense(&target)? && wn.synset_by_id(&SynsetId::new(target.as_str()))?.is_none() {
                   errors.push(ValidationError::SenseRelTargetMissing {
                       id: sense.id.clone(),
                       rel: rel.clone(),
                       target: target.clone()
                   });
               }

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
               if rel.is_symmetric() {
                   if !wn.sense_links_from_id(&target)?.iter().any(|(r2, t2)| {
                       *r2 == rel && *t2 == sense.id }) {
                       errors.push(ValidationError::SenseRelationSymmetry {
                           source: sense.id.clone(),
                           rel: rel.clone(),
                           target: target.clone()
                       });
                   }
               }
               if sense.id == target {
                   errors.push(ValidationError::SelfReferencingSenseRelation {
                       source: sense.id.clone(),
                       rel: rel.clone(), 
                       target: target.clone() });
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
            },
            None => {}
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
    check_no_loops(wn, &mut errors, &bar)?;
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

fn check_no_loops<L : Lexicon>(wn : &L,
                  errors : &mut Vec<ValidationError>,
                  bar : &ProgressBar) -> Result<()> {
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
}

fn is_valid_synset_id(synset_id : &SynsetId) -> bool {
    VALID_SYNSET_ID.is_match(synset_id.as_str())
}

fn is_valid_ili(iliid : &ILIID) -> bool {
    VALID_ILI.is_match(iliid.as_str())
}

pub enum ValidationError {
    InvalidSenseId { id : SenseId, expected : SenseId },
    SenseSynsetNotExists { id : SenseId, synset : SynsetId },
    EntryPartOfSpeech { id : SenseId, pos : PosKey, synset_pos : PartOfSpeech },
    SenseRelationPOS { id : SenseId, pos : PartOfSpeech, rel : SenseRelType },
    SynsetRelationPOS { id : SynsetId, pos : PartOfSpeech, rel : SynsetRelType },
    DuplicateSenseRelation { source : SenseId, rel : SenseRelType, target : SenseId },
    SelfReferencingSenseRelation { source : SenseId, rel : SenseRelType, target : SenseId },
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
    SenseRelTargetMissing { id : SenseId, rel : SenseRelType, target : SenseId },
    SynsetRelTargetMissing { id : SynsetId, rel : SynsetRelType, target : SynsetId },
    SatelliteSimilar { id: SynsetId, n: usize },
    NoHypernym { id: SynsetId },
    Definition { id: SynsetId },
    Lexfile { id: SynsetId, lexfile : String },
    SenseRelationSymmetry { source : SenseId, rel : SenseRelType, target : SenseId },
    SynsetRelationSymmetry { source : SynsetId, rel : SynsetRelType, target : SynsetId },
    Transitivity { id1 : SynsetId, id2 : SynsetId, id3 : SynsetId },
    Loop { id: SynsetId },
    DomainLoop { id: SynsetId },
    SynsetMemberNotInEntries { id: SynsetId, member: String },
    DuplicateMember { id: SynsetId, member : String },
    SenseNotInSynsetMembers { id: SynsetId, member: String }
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
                write!(f, "{} does not contain {} in member list", id.as_str(), member)


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
        ValidationError::DuplicateSenseRelation { source:_, rel:_, target:_ } => {
            // TODO
            false
        },
        ValidationError::DuplicateSynsetRelation { source:_, rel:_, target:_ } => {
            // TODO
            false
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
        ValidationError::DuplicateMember { .. } => {
            // TODO
            false
        },
        ValidationError::SenseNotInSynsetMembers { .. } => false,
    })
}
