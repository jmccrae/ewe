use crate::change_manager;
use crate::wordnet::{Lexicon,SynsetId,PosKey,SenseId};
use crate::rels::{SenseRelType, SynsetRelType};
use crate::change_manager::ChangeList;
use crate::validate;
use serde::{Serialize, Deserialize};

pub fn apply_automaton(actions : Vec<Action>, wn : &mut Lexicon,
                    changes : &mut ChangeList) -> Result<(), String> {
    for action in actions {
        match action {
            Action::Entry { synset, lemma, action  } => {
                match action {
                    EntryAction::Add { pos } => {
                        change_manager::add_entry(wn, synset, 
                            lemma, pos, changes);
                    },
                    EntryAction::Delete => {
                        match wn.pos_for_entry_synset(&lemma, &synset) {
                            Some(pos) => {
                                change_manager::delete_entry(wn, 
                                    &synset, 
                                    &lemma, &pos, 
                                    true, changes);
                            },
                            None => { 
                                return Err(format!("Entry {} not found in synset {}", lemma, synset.as_str()));
                            }
                        }
                    },
                    EntryAction::Move { target_synset } => {
                        match wn.pos_for_entry_synset(&lemma, &synset) {
                            Some(pos) => {
                                change_manager::move_entry(wn, 
                                    synset, 
                                    target_synset, 
                                    lemma, pos, changes);
                            },
                            None => return Err(format!("Entry {} not found in synset {}", lemma, synset.as_str()))
                        }
                    }
                }

            },
            Action::Synset { action } => {
                match action {
                    SynsetAction::Add { definition, lexfile, pos, lemmas } => {
                        let poses = wn.pos_for_lexfile(&lexfile);
                        match pos.to_part_of_speech() {
                            Some(p) => if !poses.iter().any(|p2| p == *p2) {
                                return Err(format!("Wrong POS for lexicographer file {} : {}", lexfile, pos.as_str()));
                            },
                            None => {
                                return Err(format!("POS value not valid : {}", pos.as_str()));
                            }
                        }
    
                        match change_manager::add_synset(wn, 
                            definition, lexfile, pos.clone(), 
                            None, changes) {
                            Ok(new_id) => {
                                for lemma in lemmas {
                                        change_manager::add_entry(wn, 
                                            new_id.clone(), 
                                            lemma, pos.clone(), 
                                            changes);
                                }
                            },
                            Err(e) => return Err(e)
                        }
                    },
                    SynsetAction::Delete { synset, reason, superseded_by } => {
                        change_manager::delete_synset(wn, 
                            &synset, Some(&superseded_by), 
                            reason, true, changes);
                    }
                }
            }
            Action::Definition { synset, definition } => {
                change_manager::update_def(wn, 
                    &synset, definition, false);
            },
            Action::Example { synset, action } => {
                match action {
                    ExampleAction::Add { example, source } => {
                        change_manager::add_ex(wn, 
                            &synset, example, source, changes);
                    },
                    ExampleAction::Delete { number } => {
                        change_manager::delete_ex(wn, 
                            &synset, number - 1, changes);
                    }
                }
            },
            Action::Relation { action } => {
                match action {
                    RelationAction::Add { source, source_sense, relation, target, target_sense } => {
                        match source_sense {
                            Some(sense) => {

                                change_manager::insert_sense_relation(wn, 
                                    sense.clone(), 
                                    SenseRelType::from(&relation)
                                        .ok_or(format!("Bad relation {}", relation))?,
                                    target_sense.ok_or(format!("Source sense {} with target sense", sense.as_str()))?, changes);
                            },
                            None => {
                                change_manager::insert_rel(wn, 
                                    &source,
                                    &SynsetRelType::from(&relation)
                                        .ok_or(format!("Bad relation {}", relation))?,
                                    &target, changes);
                            }
                        }
                    },
                    RelationAction::Delete { source, source_sense, target, target_sense } => {
                        match source_sense {
                            Some(source_sense) => {
                                change_manager::delete_sense_rel(wn, 
                                    &source_sense, 
                                    &target_sense.ok_or(format!("Source sense {} with target sense", source_sense.as_str()))?, changes);
                            },
                            None => {
                                change_manager::delete_rel(wn, 
                                    &source, 
                                    &target, changes);
                            }
                        }
                    },
                    RelationAction::Reverse { source, source_sense, target, target_sense } => {
                        match source_sense {
                            Some(source_sense) => {
                                change_manager::reverse_sense_rel(wn, 
                                    &source_sense, 
                                    &target_sense.ok_or(format!("Source sense {} with target sense", source_sense.as_str()))?, changes);
                            },
                            None => {
                                change_manager::reverse_rel(wn, 
                                    &source, 
                                    &target, changes);
                            }
                        }
                    }
                }
            },
            Action::Validate => {
                let errors = validate(wn);
                for error in errors.iter() {
                    println!("{}", error);
                }
                if errors.is_empty() {
                    println!("No validation errors!");
                } else {
                    println!("{} validation errors", errors.len());
                }

            }
        }
    }
    Ok(())
}


#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum Action {
    #[serde(rename = "entry")]
    Entry {
        synset : SynsetId,
        lemma : String,
        action : EntryAction
    },
    #[serde(rename = "synset")]
    Synset {
        action : SynsetAction
    },
    #[serde(rename = "definition")]
    Definition {
        synset : SynsetId,
        definition : String
    },
    #[serde(rename = "example")]
    Example {
        synset : SynsetId,
        action : ExampleAction
    },
    #[serde(rename = "relation")]
    Relation {
        action : RelationAction
    },
    #[serde(rename = "validate")]
    Validate
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum EntryAction {
    #[serde(rename = "add")]
    Add { pos : PosKey },
    #[serde(rename = "delete")]
    Delete,
    #[serde(rename = "move")]
    Move {
        target_synset : SynsetId
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum SynsetAction {
    Add {
        definition : String,
        lexfile : String,
        pos : PosKey,
        lemmas : Vec<String>,
    },
    Delete {
        synset : SynsetId,
        reason : String,
        superseded_by : SynsetId
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum ExampleAction {
    Add {
        example : String,
        #[serde(default)]
        source : Option<String>
    },
    Delete {
        number : usize
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum RelationAction {
    Add {
        source : SynsetId,
        #[serde(default)]
        source_sense : Option<SenseId>,
        relation : String,
        target : SynsetId,
        #[serde(default)]    
        target_sense : Option<SenseId>
    },
    Delete {
        source : SynsetId,
        #[serde(default)]
        source_sense : Option<SenseId>,
        target : SynsetId,
        #[serde(default)]
        target_sense : Option<SenseId>
    },
    Reverse {
        source : SynsetId,
        #[serde(default)]
        source_sense : Option<SenseId>,
        target : SynsetId,
        #[serde(default)]
        target_sense : Option<SenseId>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize() {
        let test_str = "---\nentry:\n  synset: 00001740-n\n  lemma: bar\n  action:\n    add:\n      pos: n\n";        

        let data = Action::Entry {
            synset: SynsetId::new("00001740-n"),
            lemma: "bar".to_string(),
            action: EntryAction::Add {
                pos: PosKey::new("n".to_string())
            }
        };
        let gen_str : String = serde_yaml::to_string(&data).unwrap();

        assert_eq!(test_str, gen_str);
    }
}
