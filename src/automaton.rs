use crate::change_manager;
use crate::wordnet::{Lexicon,SynsetId,PosKey,SenseId};
use crate::rels::{SenseRelType, SynsetRelType};
use crate::change_manager::ChangeList;
use crate::validate;
use serde::{Serialize, Deserialize, Serializer, Deserializer};

pub fn apply_automaton(actions : Vec<Action>, wn : &mut Lexicon,
                    changes : &mut ChangeList) -> Result<(), String> {
    let mut last_synset_id : Option<SynsetId> = None;
    let mut last_sense_id : Option<SenseId> = None;
    for action in actions {
        match action {
            Action::AddEntry { synset, lemma, pos, subcat } => {
                last_sense_id = change_manager::add_entry(wn, synset.resolve(&last_synset_id)?, 
                    lemma, pos, subcat, changes);
            },
            Action::DeleteEntry { synset, lemma } => {
                match wn.pos_for_entry_synset(&lemma, &synset.clone().resolve(&last_synset_id)?) {
                    Some(pos) => {
                        change_manager::delete_entry(wn, 
                            &synset.resolve(&last_synset_id)?, 
                            &lemma, &pos, 
                            true, changes);
                    },
                    None => { 
                        return Err(format!("Entry {} not found in synset {}", lemma, synset.as_str()));
                    }
                }
            },
            Action::MoveEntry { synset, lemma, target_synset } => {
                match wn.pos_for_entry_synset(&lemma, &synset.clone().resolve(&last_synset_id)?) {
                    Some(pos) => {
                        change_manager::move_entry(wn, 
                            synset.resolve(&last_synset_id)?, 
                            target_synset.resolve(&last_synset_id)?, 
                            lemma, pos, changes);
                    },
                    None => return Err(format!("Entry {} not found in synset {}", lemma, synset.as_str()))
                }
            },
            Action::AddSynset { definition, lexfile, pos, lemmas, subcats } => {
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
                            last_synset_id = Some(new_id);
                        },
                        Err(e) => return Err(e)
                }
            },
            Action::DeleteSynset { synset, reason, superseded_by } => {
                change_manager::delete_synset(wn, 
                    &synset.resolve(&last_synset_id)?, Some(&superseded_by.resolve(&last_synset_id)?), 
                    reason, true, changes);
            },
            Action::Definition { synset, definition } => {
                change_manager::update_def(wn, 
                    &synset.resolve(&last_synset_id)?, definition, false);
            },
            Action::AddExample { synset, example, source } => {
                change_manager::add_ex(wn, 
                    &synset.resolve(&last_synset_id)?, example, source, changes);
            },
            Action::DeleteExample { synset, number } => {
                change_manager::delete_ex(wn, 
                    &synset.resolve(&last_synset_id)?, number - 1, changes);
            },
            Action::AddRelation { source, source_sense, relation, target, target_sense } => {
                match source_sense {
                    Some(sense) => {
                        let sense = sense.resolve(&last_sense_id, wn, 
                            &source.resolve(&last_synset_id)?)?;

                        change_manager::insert_sense_relation(wn, 
                            sense.clone(), 
                            SenseRelType::from(&relation)
                                .ok_or(format!("Bad relation {}", relation))?,
                            target_sense.ok_or(format!("Source sense {} with target sense", sense.as_str()))?.resolve(&last_sense_id,
                            wn, &target.resolve(&last_synset_id)?)?, changes);
                    },
                    None => {
                        change_manager::insert_rel(wn, 
                            &source.resolve(&last_synset_id)?,
                            &SynsetRelType::from(&relation)
                                .ok_or(format!("Bad relation {}", relation))?,
                            &target.resolve(&last_synset_id)?, changes);
                    }
                }
            },
            Action::DeleteRelation { source, source_sense, target, target_sense } => {
                match source_sense {
                    Some(source_sense) => {
                        let source_sense = source_sense.resolve(&last_sense_id, wn, 
                            &source.resolve(&last_synset_id)?)?;

                        change_manager::delete_sense_rel(wn, 
                            &source_sense,
                            &target_sense.ok_or(format!("Source sense {} with target sense", source_sense.as_str()))?.resolve(&last_sense_id, wn, &target.resolve(&last_synset_id)?)?, changes);
                    },
                    None => {
                        change_manager::delete_rel(wn, 
                            &source.resolve(&last_synset_id)?, 
                            &target.resolve(&last_synset_id)?, changes);
                    }
                }
            },
            Action::ReverseRelation { source, source_sense, target, target_sense } => {
                match source_sense {
                    Some(source_sense) => {
                        let source_sense = source_sense.resolve(&last_sense_id, wn, 
                            &source.resolve(&last_synset_id)?)?;

                        
                        change_manager::reverse_sense_rel(wn, 
                            &source_sense,
                            &target_sense.ok_or(format!("Source sense {} with target sense", source_sense.as_str()))?.resolve(&last_sense_id, wn, &target.resolve(&last_synset_id)?)?, changes);
                    },
                    None => {
                        change_manager::reverse_rel(wn, 
                            &source.resolve(&last_synset_id)?, 
                            &target.resolve(&last_synset_id)?, changes);
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

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum SynsetRef {
    Id(SynsetId),
    Last
}

impl SynsetRef {
    fn resolve(self, last : &Option<SynsetId>) -> Result<SynsetId, String> {
        match self {
            SynsetRef::Id(id) => Ok(id),
            SynsetRef::Last => last.clone().ok_or("No last synset id".to_string())
        }
    }

    #[cfg(test)]
    fn id(s : &str) -> SynsetRef {
        SynsetRef::Id(SynsetId::new(s))
    }

    fn as_str(&self) -> &str {
        match self {
            SynsetRef::Id(id) => id.as_str(),
            SynsetRef::Last => "last"
        }
    }
}

impl Serialize for SynsetRef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        match self {
            SynsetRef::Id(id) => id.serialize(serializer),
            SynsetRef::Last => serializer.serialize_str("last")
        }
    }
}

impl<'de> Deserialize<'de> for SynsetRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "last" => Ok(SynsetRef::Last),
            _ => Ok(SynsetRef::Id(SynsetId::new_owned(s)))
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SenseRef {
    Id(SenseId),
    Lemma(String),
    Last
}

impl SenseRef {
    fn resolve(self, last : &Option<SenseId>, wn : &Lexicon, synset : &SynsetId) -> Result<SenseId, String> {
        match self {
            SenseRef::Id(id) => Ok(id),
            SenseRef::Lemma(lemma) => {
                wn.entry_by_lemma(&lemma).iter().flat_map(|entry| entry.sense.iter())
                    .filter(|sense| sense.synset == *synset)
                    .map(|sense| sense.id.clone())
                    .next()
                    .ok_or(format!("No sense with lemma {} in {}", lemma, synset.as_str()))
            }   
            SenseRef::Last => last.clone().ok_or("No last sense id".to_string())
        }
    }
    #[cfg(test)]
    fn id(s : &str) -> SenseRef {
        SenseRef::Id(SenseId::new(s.to_owned()))
    }

    #[cfg(test)]
    fn lemma(s : &str) -> SenseRef {
        SenseRef::Lemma(s.to_owned())
    }
}

impl Serialize for SenseRef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        match self {
            SenseRef::Id(id) => id.serialize(serializer),
            SenseRef::Lemma(lemma) => {
                serializer.serialize_str(&format!("lemma={}", lemma))
            },
            SenseRef::Last => serializer.serialize_str("last")
        }
    }
}

impl<'de> Deserialize<'de> for SenseRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "last" => Ok(SenseRef::Last),
            _ => {
                if s.starts_with("lemma=") {
                    Ok(SenseRef::Lemma(s[6..].to_owned()))
                } else {
                    Ok(SenseRef::Id(SenseId::new(s)))
                }
            }
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum Action {
    #[serde(rename = "add_entry")]
    AddEntry { 
        synset : SynsetRef,
        lemma : String,
        pos : PosKey,
        #[serde(default)]
        #[serde(skip_serializing_if = "Vec::is_empty")]
        subcat : Vec<String>,
    },
    #[serde(rename = "delete_entry")]
    DeleteEntry {
        synset : SynsetRef,
        lemma : String,
    },
    #[serde(rename = "move_entry")]
    MoveEntry {
        synset : SynsetRef,
        lemma : String,
        target_synset : SynsetRef
    },
    #[serde(rename = "add_synset")]
    AddSynset {
        definition : String,
        lexfile : String,
        pos : PosKey,
        lemmas : Vec<String>,
        #[serde(default)]
        #[serde(skip_serializing_if = "Vec::is_empty")]
        subcats: Vec<Vec<String>>
    },
    #[serde(rename = "delete_synset")]
    DeleteSynset {
        synset : SynsetRef,
        reason : String,
        superseded_by : SynsetRef
    },
    #[serde(rename = "change_definition")]
    Definition {
        synset : SynsetRef,
        definition : String
    },
    #[serde(rename = "add_example")]
    AddExample {
        synset : SynsetRef,
        example : String,
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        source : Option<String>
    },
    #[serde(rename = "delete_example")]
    DeleteExample {
        synset : SynsetRef,
        number : usize
    },
    #[serde(rename = "add_relation")]
    AddRelation {
        source : SynsetRef,
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        source_sense : Option<SenseRef>,
        relation : String,
        target : SynsetRef,
        #[serde(default)]    
        #[serde(skip_serializing_if = "Option::is_none")]
        target_sense : Option<SenseRef>
    },
    #[serde(rename = "delete_relation")]
    DeleteRelation {
        source : SynsetRef,
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        source_sense : Option<SenseRef>,
        target : SynsetRef,
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        target_sense : Option<SenseRef>
    },
    #[serde(rename = "reverse_relation")]
    ReverseRelation {
        source : SynsetRef,
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        source_sense : Option<SenseRef>,
        target : SynsetRef,
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        target_sense : Option<SenseRef>
    },
    #[serde(rename = "validate")]
    Validate
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize() {
        let test_str = "---\n- add_entry:\n    synset: 00001740-n\n    lemma: bar\n    pos: n\n- delete_entry:\n    synset: 00001740-n\n    lemma: bar\n- move_entry:\n    synset: 00001740-n\n    lemma: bar\n    target_synset: 00001741-n\n- add_synset:\n    definition: something or someone\n    lexfile: noun.animal\n    pos: n\n    lemmas:\n      - bar\n- delete_synset:\n    synset: last\n    reason: \"Duplicate (#123)\"\n    superseded_by: 00001741-n\n- change_definition:\n    synset: 00001740-n\n    definition: This is a definition\n- add_example:\n    synset: 00001740-n\n    example: This is an example\n    source: This is a source\n- delete_example:\n    synset: 00001740-n\n    number: 1\n- add_relation:\n    source: 00001740-n\n    relation: hypernym\n    target: 00001741-n\n- delete_relation:\n    source: 00001740-n\n    source_sense: \"example%1:09:00::\"\n    target: 00001741-n\n    target_sense: \"target%1:10:00::'\"\n- reverse_relation:\n    source: 00001740-n\n    target: 00001741-n\n- validate\n";
        let data = vec![Action::AddEntry {
                synset: SynsetRef::id("00001740-n"),
                lemma: "bar".to_string(),
                pos: PosKey::new("n".to_string()),
                subcat: Vec::new()
            },
            Action::DeleteEntry {
                synset: SynsetRef::id("00001740-n"),
                lemma: "bar".to_string(),
            },
            Action::MoveEntry {
                synset: SynsetRef::id("00001740-n"),
                lemma: "bar".to_string(),
                target_synset: SynsetRef::id("00001741-n")
            },
            Action::AddSynset {
                    definition: "something or someone".to_string(),
                    lexfile: "noun.animal".to_string(),
                    pos: PosKey::new("n".to_string()),
                    lemmas: vec!["bar".to_string()],
                    subcats: vec![]
            },
            Action::DeleteSynset {
                    synset: SynsetRef::Last,
                    reason: "Duplicate (#123)".to_string(),
                    superseded_by: SynsetRef::id("00001741-n")
            },
            Action::Definition {
                synset: SynsetRef::id("00001740-n"),
                definition: "This is a definition".to_string()
            },
            Action::AddExample {
                synset: SynsetRef::id("00001740-n"),
                    example: "This is an example".to_string(),
                    source: Some("This is a source".to_string())
            },
            Action::DeleteExample {
                synset: SynsetRef::id("00001740-n"),
                    number: 1
            },
            Action::AddRelation {
                    source: SynsetRef::id("00001740-n"),
                    source_sense: None,
                    relation: "hypernym".to_string(),
                    target: SynsetRef::id("00001741-n"),
                    target_sense: None
            },
            Action::DeleteRelation {
                    source: SynsetRef::id("00001740-n"),
                    source_sense: Some(SenseRef::id("example%1:09:00::")),
                    target: SynsetRef::id("00001741-n"),
                    target_sense: Some(SenseRef::id("target%1:10:00::'"))
            },
            Action::ReverseRelation {
                    source: SynsetRef::id("00001740-n"),
                    source_sense: None,
                    target: SynsetRef::id("00001741-n"),
                    target_sense: None
            },
            Action::Validate
        ];
                
        let gen_str : String = serde_yaml::to_string(&data).unwrap();

        assert_eq!(test_str, gen_str);
    }

    #[test]
    fn test_last() {
        let actions = vec![
            Action::AddSynset {
                definition: "something or someone".to_string(),
                lexfile: "noun.animal".to_string(),
                pos: PosKey::new("n".to_string()),
                lemmas: vec!["bar".to_string()],
                subcats: vec![]
            },
            Action::AddRelation {
                source: SynsetRef::Last,
                source_sense: None,
                relation: "hypernym".to_string(),
                target: SynsetRef::id("00001741-n"),
                target_sense: None
            }];
        let mut lexicon = Lexicon::new();
        lexicon.add_lexfile("noun.animal");
        apply_automaton(actions, &mut lexicon, &mut ChangeList::new()).unwrap();

    }

    #[test]
    fn test_sense_by_lemma() {
                let mut lexicon = Lexicon::new();
        let mut change_list = ChangeList::new();
        lexicon.add_lexfile("noun.animal");
        let ssid1 = change_manager::add_synset(&mut lexicon, 
            "def 1".to_string(), 
            "noun.animal".to_string(), 
            PosKey::new("n".to_string()), 
            None, 
            &mut change_list).expect("Could not create synset");
        change_manager::add_entry(&mut lexicon, 
            ssid1.clone(), 
            "bar".to_owned(), 
            PosKey::new("n".to_string()), 
            Vec::new(), &mut change_list);
        let ssid2 = change_manager::add_synset(&mut lexicon, 
            "def 2".to_string(), 
            "noun.animal".to_string(), 
            PosKey::new("n".to_string()), 
            None, 
            &mut change_list).expect("Could not create synset");
        change_manager::add_entry(&mut lexicon, 
            ssid2.clone(), 
            "baz".to_owned(), 
            PosKey::new("n".to_string()), 
            Vec::new(), &mut change_list);
        let actions = vec![
            Action::AddRelation {
                source: SynsetRef::Id(ssid1),
                target: SynsetRef::Id(ssid2),
                relation: "antonym".to_string(),
                source_sense: Some(SenseRef::lemma("bar")),
                target_sense: Some(SenseRef::lemma("baz"))
        }];
    apply_automaton(actions, &mut lexicon, &mut ChangeList::new()).unwrap();
    }
}
