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
                    EntryAction::Add { pos, subcat } => {
                        change_manager::add_entry(wn, synset, 
                            lemma, pos, subcat, changes);
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
                    SynsetAction::Add { definition, lexfile, pos, lemmas, subcats } => {
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
                                    if subcats.is_empty() {
                                        for lemma in lemmas {
                                            change_manager::add_entry(wn, 
                                                new_id.clone(), 
                                                lemma, pos.clone(), 
                                                Vec::new(),
                                                changes);
                                        }
                                    } else {
                                        for (lemma, subcat) in lemmas.into_iter().zip(subcats.into_iter()) {
                                            change_manager::add_entry(wn, 
                                                new_id.clone(), 
                                                lemma, pos.clone(), 
                                                subcat,
                                                changes);
                                        }
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
    Add { 
        pos : PosKey,
        #[serde(default)]
        #[serde(skip_serializing_if = "Vec::is_empty")]
        subcat : Vec<String>,
    },
    #[serde(rename = "delete")]
    Delete,
    #[serde(rename = "move")]
    Move {
        target_synset : SynsetId
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum SynsetAction {
    #[serde(rename = "add")]
    Add {
        definition : String,
        lexfile : String,
        pos : PosKey,
        lemmas : Vec<String>,
        #[serde(default)]
        #[serde(skip_serializing_if = "Vec::is_empty")]
        subcats: Vec<Vec<String>>
    },
    #[serde(rename = "delete")]
    Delete {
        synset : SynsetId,
        reason : String,
        superseded_by : SynsetId
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum ExampleAction {
    #[serde(rename = "add")]
    Add {
        example : String,
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        source : Option<String>
    },
    #[serde(rename = "delete")]
    Delete {
        number : usize
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum RelationAction {
    #[serde(rename = "add")]
    Add {
        source : SynsetId,
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        source_sense : Option<SenseId>,
        relation : String,
        target : SynsetId,
        #[serde(default)]    
        #[serde(skip_serializing_if = "Option::is_none")]
        target_sense : Option<SenseId>
    },
    #[serde(rename = "delete")]
    Delete {
        source : SynsetId,
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        source_sense : Option<SenseId>,
        target : SynsetId,
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        target_sense : Option<SenseId>
    },
    #[serde(rename = "reverse")]
    Reverse {
        source : SynsetId,
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        source_sense : Option<SenseId>,
        target : SynsetId,
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        target_sense : Option<SenseId>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize() {
        let test_str = "---\n- entry:\n    synset: 00001740-n\n    lemma: bar\n    action:\n      add:\n        pos: n\n- entry:\n    synset: 00001740-n\n    lemma: bar\n    action: delete\n- entry:\n    synset: 00001740-n\n    lemma: bar\n    action:\n      move:\n        target_synset: 00001741-n\n- synset:\n    action:\n      add:\n        definition: something or someone\n        lexfile: noun.animal\n        pos: n\n        lemmas:\n          - bar\n- synset:\n    action:\n      delete:\n        synset: 00001740-n\n        reason: \"Duplicate (#123)\"\n        superseded_by: 00001741-n\n- definition:\n    synset: 00001740-n\n    definition: This is a definition\n- example:\n    synset: 00001740-n\n    action:\n      add:\n        example: This is an example\n        source: This is a source\n- example:\n    synset: 00001740-n\n    action:\n      delete:\n        number: 1\n- relation:\n    action:\n      add:\n        source: 00001740-n\n        relation: hypernym\n        target: 00001741-n\n- relation:\n    action:\n      delete:\n        source: 00001740-n\n        source_sense: \"example%1:09:00::\"\n        target: 00001741-n\n        target_sense: \"target%1:10:00::'\"\n- relation:\n    action:\n      reverse:\n        source: 00001740-n\n        target: 00001741-n\n- validate\n";
        let data = vec![Action::Entry {
                synset: SynsetId::new("00001740-n"),
                lemma: "bar".to_string(),
                action: EntryAction::Add {
                    pos: PosKey::new("n".to_string()),
                    subcat: Vec::new()
                }
            },
            Action::Entry {
                synset: SynsetId::new("00001740-n"),
                lemma: "bar".to_string(),
                action: EntryAction::Delete
            },
            Action::Entry {
                synset: SynsetId::new("00001740-n"),
                lemma: "bar".to_string(),
                action: EntryAction::Move {
                    target_synset: SynsetId::new("00001741-n")
                }
            },
            Action::Synset {
                action: SynsetAction::Add {
                    definition: "something or someone".to_string(),
                    lexfile: "noun.animal".to_string(),
                    pos: PosKey::new("n".to_string()),
                    lemmas: vec!["bar".to_string()],
                    subcats: vec![]
                },
            },
            Action::Synset {
                action: SynsetAction::Delete {
                    synset: SynsetId::new("00001740-n"),
                    reason: "Duplicate (#123)".to_string(),
                    superseded_by: SynsetId::new("00001741-n")
                }
            },
            Action::Definition {
                synset: SynsetId::new("00001740-n"),
                definition: "This is a definition".to_string()
            },
            Action::Example {
                synset: SynsetId::new("00001740-n"),
                action: ExampleAction::Add {
                    example: "This is an example".to_string(),
                    source: Some("This is a source".to_string())
                }
            },
            Action::Example {
                synset: SynsetId::new("00001740-n"),
                action: ExampleAction::Delete {
                    number: 1
                }
            },
            Action::Relation {
                action: RelationAction::Add {
                    source: SynsetId::new("00001740-n"),
                    source_sense: None,
                    relation: "hypernym".to_string(),
                    target: SynsetId::new("00001741-n"),
                    target_sense: None
                }
            },
            Action::Relation {
                action: RelationAction::Delete {
                    source: SynsetId::new("00001740-n"),
                    source_sense: Some(SenseId::new("example%1:09:00::".to_string())),
                    target: SynsetId::new("00001741-n"),
                    target_sense: Some(SenseId::new("target%1:10:00::'".to_string()))
                }
            },
            Action::Relation {
                action: RelationAction::Reverse {
                    source: SynsetId::new("00001740-n"),
                    source_sense: None,
                    target: SynsetId::new("00001741-n"),
                    target_sense: None
                }
            },
            Action::Validate
        ];
                
        let gen_str : String = serde_yaml::to_string(&data).unwrap();

        assert_eq!(test_str, gen_str);
    }
}
