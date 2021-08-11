use crate::wordnet::*;
use crate::rels::*;
use crate::sense_keys::{get_sense_key2};
use std::fmt;
use std::collections::HashSet;
use indicatif::ProgressBar;
use lazy_static::lazy_static;
use regex::Regex;

pub fn validate(wn : &Lexicon) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    println!("Validating");
    let bar = ProgressBar::new(1);
    let mut sense_keys = HashSet::new();
    for (lemma, poskey, entry) in wn.entries() {
        for sense in entry.sense.iter() {
           match get_sense_key2(wn, lemma, Some(&sense.id), &sense.synset) {
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
           match wn.synset_by_id(&sense.synset) {
               Some(synset) => {
                   if poskey.to_part_of_speech() == None ||
                       synset.part_of_speech != poskey.to_part_of_speech().unwrap() {
                        errors.push(ValidationError::EntryPartOfSpeech {
                            id: sense.id.clone(),
                            pos: poskey.clone(),
                            synset_pos: synset.part_of_speech.clone()
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
        }
        // Sense not empty
    }
    bar.inc(1);
    for (synset_id, synset) in wn.synsets() {
        let ssid = synset_id.as_str();
        if ssid[(ssid.len() - 1)..ssid.len()] != *synset.part_of_speech.value() {
            errors.push(ValidationError::SynsetIdPos {
                id: synset_id.clone(),
                pos: synset.part_of_speech.clone()
            });
        }
        if !is_valid_synset_id(synset_id) {
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

        // Part of speech of relations
        // Cross part of speech hypernyms
        // Single similar for satellites
        // Duplicate relations
        // At least one hypernym for nouns
        // Empty definitions
        // Lex file matches POS
    }
    bar.inc(1);
    // Symmetry errors
    // Transitivity errors
    // Loops in hypernym graph
    bar.finish();
    errors
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
    DuplicateSenseRelation { source : SenseId, rel : SenseRelType, target : SenseId },
    DuplicateSenseKey { id : SenseId },
    DuplicateSyntacticBehaviour { id : SenseId },
    SynsetIdPos { id : SynsetId, pos : PartOfSpeech },
    InvalidSynsetId { id : SynsetId },
    EmptySynset { id : SynsetId },
    InvalidILIId { id : SynsetId, ili: ILIID }
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
            ValidationError::DuplicateSenseRelation { source, rel, target } =>
                write!(f, "Duplicate relation {} ={}=> {}", 
                       source.as_str(), rel.value(), target.as_str()),
            ValidationError::DuplicateSenseKey { id } =>
                write!(f, "Duplicate sense key {}", id.as_str()),
            ValidationError::DuplicateSyntacticBehaviour { id } =>
                write!(f, "Duplicate syntactic behaviour for sense {}", 
                       id.as_str()),
            ValidationError::SynsetIdPos { id, pos } =>
                write!(f, "Synset {} is not valid for a synset with POS {}",
                       id.as_str(), pos.value()),
            ValidationError::InvalidSynsetId { id } =>
                write!(f, "Invalid synset id: {}", id.as_str()),
            ValidationError::EmptySynset { id } =>
                write!(f, "Empty synset: {}", id.as_str()),
            ValidationError::InvalidILIId { id, ili } =>
                write!(f, "Synset {} has an invalid ILI identifier {}", id.as_str(), ili.as_str())

        }
    }
}

