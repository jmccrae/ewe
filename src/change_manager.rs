use crate::wordnet::*;
use crate::rels::*;
use crate::sense_keys::{get_sense_key, get_sense_key2};
use sha2::Sha256;
use crate::sha2::Digest;

/// Monitors is any changes have been made
pub struct ChangeList(bool);

impl ChangeList {
    /// Has the WordNet been changed
    pub fn changed(&self) -> bool { self.0 }
    /// Mark the WordNet as changed
    pub fn mark(&mut self) { self.0 = true; }
    /// Reset all changes (after save)
    pub fn reset(&mut self) { self.0 = false; }
    /// Create an instance
    pub fn new() -> ChangeList { ChangeList(false) }
}

/// Remove a relation between synsets
pub fn delete_rel(wn : &mut Lexicon, source : &SynsetId, 
                  target : &SynsetId, change_list : &mut ChangeList) {
    println!("Delete {} =*=> {}", source.as_str(), target.as_str());
    wn.remove_rel(source, target);
    wn.remove_rel(target, source);
    change_list.mark();
}

/// Remove a relation between senses
pub fn delete_sense_rel(wn : &mut Lexicon, 
                        source : &SenseId, target : &SenseId,
                        change_list : &mut ChangeList) {
    println!("Delete {} =*=> {}", source.as_str(), target.as_str());
    wn.remove_sense_rel(source, target);
    wn.remove_sense_rel(target, source);
    change_list.mark();
}

/// Add a relation between synsets
pub fn insert_rel(wn : &mut Lexicon, source_id : &SynsetId,
                  rel_type : &SynsetRelType,
                  target_id : &SynsetId, change_list : &mut ChangeList) {
    println!("Insert {} ={}=> {}", source_id.as_str(), rel_type.value(),
                    target_id.as_str());
    wn.add_rel(source_id, rel_type.clone(), target_id);
    if rel_type.is_symmetric() {
        wn.add_rel(target_id, rel_type.clone(), source_id);
    }
    if *rel_type == SynsetRelType::Similar {
        let mut changes = Vec::new();
        for id in vec![source_id, target_id] {
            for member in wn.members_by_id(id) {
                for sense in wn.get_sense(&member, id) {
                    match get_sense_key2(wn, &member, Some(&sense.id), id) {
                        Some(calc_key) => {
                            if sense.id != calc_key {
                                changes.push((sense.id.clone(), calc_key.clone()));
                            }
                        },
                        None => {}
                    }
                }
            }
        }
        for (old, new) in changes {
            wn.update_sense_key(&old, &new);
        }
    }
    change_list.mark();
}

/// Add a new entry
pub fn add_entry(wn : &mut Lexicon, synset_id : SynsetId, 
                 lemma : String, 
                 synset_pos : PosKey,
                 change_list : &mut ChangeList) -> Option<SenseId> {
    println!("Adding {} to synset {}", lemma, synset_id.as_str());

    let mut entries = wn.entry_by_lemma_with_pos(&lemma).iter_mut()
        .filter(|(pos, _)| synset_pos == **pos)
        .map(|x| x.1)
        .collect::<Vec<&Entry>>();


    if entries.len() > 1 {
        println!("More than one entry for {} ({}). Please check the YAML file",
            lemma, synset_pos.as_str());
    }

    let entry = entries.pop();

    let sense_id = match entry {
        Some(e) => {
            match wn.synset_by_id(&synset_id) {
                Some(synset) => {
                    let sense_id = 
                            get_sense_key(wn, &lemma, e, None, synset, &synset_id);
                    let sense = Sense::new(sense_id.clone(),
                            synset_id.clone());
                    wn.insert_sense(lemma.clone(), synset_pos.clone(), sense);
                    change_list.mark();
                    Some(sense_id)
                },
                None => None
            }
        },
        None => { 
            match wn.synset_by_id(&synset_id) {
                Some(synset) => {
                    let e = Entry::new();
                    let sense_id = get_sense_key(wn, &lemma, &e, None, synset, &synset_id);
                    let sense = Sense::new(sense_id.clone(),
                            synset_id.clone());
                    wn.insert_entry(lemma.clone(), synset_pos.clone(), e);
                    wn.insert_sense(lemma.clone(), synset_pos.clone(), sense);
                    change_list.mark();
                    Some(sense_id)
                },
                None => None
            }
        }
    };
    match wn.synset_by_id_mut(&synset_id) {
        Some(ref mut synset) => {
            synset.members.push(lemma.clone());
            change_list.mark();
        },
        None => {
            eprintln!("Adding entry to non-existant synset");
        }
    }
    sense_id
}

/// Delete an entry
pub fn delete_entry(wn : &mut Lexicon, synset_id : &SynsetId, lemma : &str, 
                    pos : &PosKey, warn : bool, change_list : &mut ChangeList) {
    println!("Removing {} from synset {}", lemma, synset_id.as_str());
    let links = wn.sense_links_to(lemma, pos, synset_id);
    for sense_id in  wn.remove_sense(lemma, pos, synset_id) {
        for (_, source) in links.iter() {
            wn.remove_sense_rel(&source, &sense_id);
        }
    }
    change_list.mark();
    match wn.synset_by_id_mut(&synset_id) {
        Some(ref mut synset) => {
            synset.members.retain(|l| l != lemma);
            if warn && synset.members.is_empty() {
                println!("{} is now empty! Please add at least one new member before saving", synset_id.as_str());
            }
        },
        None => {
            eprintln!("Removing entry from non-existant synset");
        }
    }

}

/// Move an entry to another synset
pub fn move_entry(wn : &mut Lexicon, synset_id : SynsetId, 
              target_synset_id : SynsetId,
              lemma : String, pos : PosKey,
              change_list : &mut ChangeList) {

    let links_from = wn.sense_links_from(&lemma, &pos, &synset_id);
    let links_to   = wn.sense_links_to(&lemma, &pos, &synset_id);
    let forms = wn.get_forms(&lemma, &pos);
    delete_entry(wn, &synset_id, &lemma, &pos, true, change_list);
    match add_entry(wn, target_synset_id, 
                    lemma.clone(), pos.clone(), change_list) {
        Some(sense_id) => {
            for (rel, target) in links_from {
                wn.add_sense_rel(&sense_id, rel, &target);
            }
            for (rel, source) in links_to {
                wn.add_sense_rel(&source, rel, &sense_id);
            }
        },
        None => {
            println!("New synset not created");
        }
    };
    for form in forms {
        wn.add_form(&lemma, &pos, form);
    }
}

/// Delete a synset
pub fn delete_synset(wn : &mut Lexicon, synset_id : &SynsetId,
                 supersede_id : Option<&SynsetId>,
                 reason : String, delent : bool, change_list: &mut ChangeList) {
    println!("Deleting synset {}", synset_id.as_str());

    if delent {
        let entries = wn.members_by_id(synset_id);
        for entry in entries {
            match wn.pos_for_entry_synset(&entry, synset_id) {
                Some(pos) =>
                    delete_entry(wn, synset_id, &entry, &pos, false, change_list),
                    None => {}
            }
        }
    }

    match supersede_id {
        Some(supersede_id) => {
            match wn.synset_by_id(synset_id) {
                Some(ss) => {
                    for (rel, target) in ss.links_from() {
                        delete_rel(wn, synset_id, &target, change_list);
                        wn.add_rel(supersede_id, rel, &target);
                    }
                    for (rel, source) in wn.links_to(synset_id) {
                        delete_rel(wn, &source, synset_id, change_list);
                        wn.add_rel(&source, rel, supersede_id);
                    }
                },
                None => {}
            }
        },
        None => {
            match wn.synset_by_id(synset_id) {
                Some(ss) => {
                    for (_, target) in ss.links_from() {
                        delete_rel(wn, synset_id, &target, change_list);
                    }
                    for (_, source) in wn.links_to(synset_id) {
                        delete_rel(wn, &source, synset_id, change_list);
                    }
                },
                None => {}
            }
        }
    }

    wn.remove_synset(synset_id);
    
    match supersede_id {
        Some(ss_id) => {
            wn.deprecate(synset_id, ss_id, reason);
        },
        None => {}
    }

    change_list.mark();
}

fn new_id(wn : &Lexicon, pos : &PartOfSpeech, definition : &str) -> Result<SynsetId, String> {
    let s = Sha256::digest(definition.as_bytes());
    let mut key : u32 = 0;
    for x in s.into_iter() {
        key = (key * 16 + x as u32) % 10000000;
    }
    let nid = SynsetId::new_owned(format!("8{:07}-{}", key, pos.value()));
    match wn.synset_by_id(&nid) {
        Some(_) => Err(format!("Duplicate Synset ID. This is likely due to a duplicate definition")),
        None => Ok(nid)
    }
}

/// Add a synset. Fails if POS key has invalid value.
pub fn add_synset(wn : &mut Lexicon, definition : String, lexfile : String,
              pos : PosKey, ssid : Option<SynsetId>, 
              change_list : &mut ChangeList) -> Result<SynsetId, String> {
   match pos.to_part_of_speech() {
        Some(pos) => {
            let ssid = match ssid {
                Some(ssid) => ssid,
                None => new_id(wn, &pos, &definition)?
            };
            let mut synset = Synset::new(pos);
            synset.definition.push(definition);
            wn.insert_synset(lexfile, ssid.clone(), synset);
            change_list.mark();
            Ok(ssid)
        },
        None => {
            Err(format!("Part of speech value is not valid"))
        }
    }
}

fn find_rel_type(wn : &Lexicon, source : &SynsetId, target : &SynsetId) 
    -> Vec<SynsetRelType> {
        wn.links_from(source).into_iter()
            .filter(|x| x.1 == *target).map(|x| x.0).chain(
            wn.links_from(target).into_iter()
            .filter(|x| x.1 == *source).map(|x| x.0)).collect()
}



/// Reverse the direction of relations
pub fn reverse_rel(wn : &mut Lexicon, source : &SynsetId,
               target : &SynsetId, change_list : &mut ChangeList) {
    for rel_type in find_rel_type(wn, source, target) {
        delete_rel(wn, source, target, change_list);
        insert_rel(wn, target, &rel_type, source, change_list);
    }
}

/// Add a relation between senses
pub fn insert_sense_relation(wn : &mut Lexicon, source : SenseId, rel : SenseRelType,
                      target : SenseId, change_list : &mut ChangeList) {
    if rel.is_symmetric() {
        wn.add_sense_rel(&target, rel.clone(), &source);
    }
    wn.add_sense_rel(&source, rel, &target);
    change_list.mark();
}

fn find_sense_rel_type(wn : &Lexicon, source : &SenseId, target : &SenseId) 
    -> Vec<SenseRelType> {
        wn.sense_links_from_id(source).into_iter()
            .filter(|x| x.1 == *target).map(|x| x.0).chain(
            wn.sense_links_from_id(target).into_iter()
            .filter(|x| x.1 == *source).map(|x| x.0)).collect()
}

/// Reverse the direction of a sense relation
pub fn reverse_sense_rel(wn : &mut Lexicon, source : &SenseId,
                      target : &SenseId, change_list : &mut ChangeList) {
    for rel_type in find_sense_rel_type(wn, source, target) {
        delete_sense_rel(wn, source, target, change_list);
        insert_sense_relation(wn, target.clone(), rel_type, source.clone(), change_list);
    }
}

/// Change a definition
pub fn update_def(wn : &mut Lexicon, synset_id : &SynsetId, defn : String,
              add : bool) {
    match wn.synset_by_id_mut(synset_id) {
        Some(synset) => {
            if add {
                synset.definition.push(defn.to_string())
            } else {
                synset.definition = vec![defn.to_string()]
            }
        },
        None => {
            eprintln!("Changing definition of non-existant synset {}", synset_id.as_str());
        }
    }
}

/// Add an example
pub fn add_ex(wn : &mut Lexicon, synset_id : &SynsetId, example : String,
          source : Option<String>, change_list : &mut ChangeList) {
    match wn.synset_by_id_mut(synset_id) {
        Some(ss) => {
            ss.example.push(Example::new(example, source));
            change_list.mark();
        },
        None => {
            eprintln!("Adding example to non-existant synset");
        }
    }
}

/// Remove the nth example
pub fn delete_ex(wn : &mut Lexicon, synset_id : &SynsetId, idx : usize,
             change_list : &mut ChangeList) {
    match wn.synset_by_id_mut(synset_id) {
        Some(ss) => {
            ss.example.remove(idx);
            change_list.mark();
        },
        None => {
            eprintln!("Adding example to non-existant synset");
        }
    }
}
