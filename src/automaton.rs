use crate::change_manager;
use crate::wordnet::{Lexicon,SynsetId,PosKey};
use crate::change_manager::ChangeList;
use serde::{Serialize, Deserialize};

fn apply_automaton(actions : Vec<Action>, wn : &mut Lexicon,
                    changes : &mut ChangeList) -> Result<(), String> {
    for action in actions {
        match action {
            Action::Entry { synset_id, synset, lemma, action  } => {
                match action {
                    EntryAction::Add { pos } => {
                        change_manager::add_entry(wn, synset_id, 
                            lemma, pos, changes);
                    },
                    EntryAction::Delete => {
                        match wn.pos_for_entry_synset(&lemma, &synset_id) {
                            Some(pos) => {
                                change_manager::delete_entry(wn, 
                                    &synset_id, 
                                    &lemma, &pos, 
                                    true, changes);
                            },
                            None => { 
                                return Err(format!("Entry {} not found in synset {}", lemma, synset_id.as_str()));
                            }
                        }
                    },
                    EntryAction::Move { target_synset_id, .. } => {
                        match wn.pos_for_entry_synset(&lemma, &synset_id) {
                            Some(pos) => {
                                change_manager::move_entry(wn, 
                                    synset_id, 
                                    target_synset_id, 
                                    lemma, pos, changes);
                            },
                            None => return Err(format!("Entry {} not found in synset {}", lemma, synset_id.as_str()))
                        }
                    }
                }

            },
            _ => {},
        }
    }
    Ok(())
}


#[derive(Debug, PartialEq, Serialize, Deserialize)]
enum Action {
    #[serde(rename = "entry")]
    Entry {
        synset_id : SynsetId,
        synset : String,
        lemma : String,
        action : EntryAction
    },
    #[serde(rename = "synset")]
    Synset {

    },
    #[serde(rename = "definition")]
    Definition {

    },
    #[serde(rename = "example")]
    Example {

    },
    #[serde(rename = "relation")]
    Relation {

    },
    #[serde(rename = "validate")]
    Validate
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
enum EntryAction {
    #[serde(rename = "add")]
    Add { pos : PosKey },
    #[serde(rename = "delete")]
    Delete,
    #[serde(rename = "move")]
    Move {
        target_synset_id : SynsetId,
        target_synset : String
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize() {
        let test_str = "---\nentry:\n  synset_id: 00001740-n\n  synset: foo\n  lemma: bar\n  action:\n    add:\n      pos: n\n";        

        let data = Action::Entry {
            synset_id: SynsetId::new("00001740-n"),
            synset: "foo".to_string(),
            lemma: "bar".to_string(),
            action: EntryAction::Add {
                pos: PosKey::new("n".to_string())
            }
        };
        let gen_str : String = serde_yaml::to_string(&data).unwrap();

        assert_eq!(test_str, gen_str);
    }
}
