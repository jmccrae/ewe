use crate::wordnet::*;
use crate::rels::*;
use crate::sense_keys::{get_sense_key, get_sense_key2};
use sha2::Sha256;
use crate::sha2::Digest;
use std::borrow::Cow;
use std::result;

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
pub fn delete_rel<L : Lexicon>(wn : &mut L, source : &SynsetId, 
                  target : &SynsetId, change_list : &mut ChangeList) {
    println!("Delete {} =*=> {}", source.as_str(), target.as_str());
    wn.remove_rel(source, target).
        unwrap_or_else(|_| {
            eprintln!("Removing relation from non-existant synset");
        });
    wn.remove_rel(target, source).
        unwrap_or_else(|_| {
            eprintln!("Removing relation from non-existant synset");
        });
    change_list.mark();
}

/// Remove a relation between senses
pub fn delete_sense_rel<L : Lexicon>(wn : &mut L,
                        source : &SenseId, target : &SenseId,
                        change_list : &mut ChangeList) -> Result<()> {
    println!("Delete {} =*=> {}", source.as_str(), target.as_str());
    wn.remove_sense_rel(source, target)?;
    wn.remove_sense_rel(target, source)?;
    change_list.mark();
    Ok(())
}

/// Add a relation between synsets
pub fn insert_rel<L : Lexicon>(wn : &mut L,
                  source_id : &SynsetId,
                  rel_type : &SynsetRelType,
                  target_id : &SynsetId, change_list : &mut ChangeList) -> Result<()> {
    println!("Insert {} ={}=> {}", source_id.as_str(), rel_type.value(),
                    target_id.as_str());
    wn.add_rel(source_id, rel_type.clone(), target_id).unwrap_or_else(|_| {
        eprintln!("Adding relation to non-existant synset");
    });
    if rel_type.is_symmetric() {
        wn.add_rel(target_id, rel_type.clone(), source_id).unwrap_or_else(|_| {
            eprintln!("Adding relation to non-existant synset");
        });
    }
    if *rel_type == SynsetRelType::Similar {
        let mut changes = Vec::new();
        for id in vec![source_id, target_id] {
            for member in wn.members_by_id(id)? {
                for sense in wn.get_sense(&member, id)? {
                    match get_sense_key2(wn, &member, Some(&sense.id), id)? {
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
            wn.update_sense_key(&old, &new)?;
        }
    }
    change_list.mark();
    Ok(())
}

pub enum RelationUpdate {
    Synset(SynsetId, SynsetRelType, SynsetId),
    Sense(SenseId, SenseRelType, SenseId)
}

pub fn update_rels<L : Lexicon>(wn : &mut L,
    source : &SynsetId,
    relations : Vec<RelationUpdate>,
    change_list : &mut ChangeList) -> Result<()> {
    // First remove all links referring to and from this synset
    for (_, target_id) in wn.links_from(source)? {
        delete_rel(wn, source, &target_id, change_list);
    }
    for (_, source_id) in wn.links_to(source)? {
        delete_rel(wn, &source_id, source, change_list);
    }
    for (source_id, _, target_id) in wn.all_sense_links(source)? {
        delete_sense_rel(wn, &source_id, &target_id, change_list)?;
    }
    
    // Now add the relations
    for rel in relations {
        match rel {
            RelationUpdate::Synset(source_id, rel_type, target_id) => {
                insert_rel(wn, &source_id, &rel_type, &target_id, change_list)?;
            },
            RelationUpdate::Sense(source_id, rel_type, target_id) => {
                insert_sense_relation(wn, source_id, rel_type, target_id, change_list)?;
            }
        }
    }
    Ok(())
}

/// Add a new entry
pub fn add_entry<L : Lexicon>(wn : &mut L,
                 synset_id : SynsetId, 
                 lemma : String, 
                 synset_pos : PosKey,
                 subcat : Vec<String>,
                 old_sense_id : Option<&SenseId>,
                 change_list : &mut ChangeList) -> Result<Option<SenseId>> {
    println!("Adding {} to synset {}", lemma, synset_id.as_str());

    let mut entries = wn.entry_by_lemma_with_pos(&lemma)?.iter_mut()
        .filter(|(pos, _)| synset_pos == *pos)
        .map(|x| x.1.clone())
        .collect::<Vec<Cow<Entry>>>();

    if entries.len() > 1 {
        println!("More than one entry for {} ({}). Please check the YAML file",
            lemma, synset_pos.as_str());
    }

    let entry = entries.pop();

    let sense_id = match entry {
        Some(e) => {
            match e.sense.iter().filter(|s| s.synset == synset_id).next() {
                Some(sense) => Some(sense.id.clone()),
                None => {
                    match wn.synset_by_id(&synset_id)? {
                        Some(synset) => {
                            let sense_id = 
                                    get_sense_key(wn, &lemma, old_sense_id, &synset, &synset_id)?;
                            let mut sense = Sense::new(sense_id.clone(),
                                    synset_id.clone());
                            sense.subcat = subcat;
                            wn.insert_sense(lemma.clone(), synset_pos.clone(), sense)?;
                            change_list.mark();
                            Some(sense_id)
                        },
                        None => None
                    }
                }
            }
        },
        None => { 
            match wn.synset_by_id(&synset_id)? {
                Some(synset) => {
                    let e = Entry::new();
                    let sense_id = get_sense_key(wn, &lemma, old_sense_id, &synset, &synset_id)?;
                    let mut sense = Sense::new(sense_id.clone(),
                            synset_id.clone());
                    sense.subcat = subcat;
                    wn.insert_entry(lemma.clone(), synset_pos.clone(), e)?;
                    wn.insert_sense(lemma.clone(), synset_pos.clone(), sense)?;
                    change_list.mark();
                    Some(sense_id)
                },
                None => return Err(LexiconError::SynsetIdNotFound(synset_id))
            }
        }
    };
    wn.update_synset(&synset_id, |synset| {
            if !synset.members.contains(&lemma) {
                synset.members.push(lemma.clone());
            }
            change_list.mark();
    }).unwrap_or_else(|_| {
        eprintln!("Adding entry to non-existant synset");
    });
    Ok(sense_id)
}

/// Delete an entry
pub fn delete_entry<L : Lexicon>(wn : &mut L,
                    synset_id : &SynsetId, lemma : &str, 
                    pos : &PosKey, warn : bool, change_list : &mut ChangeList) -> Result<()> {
    println!("Removing {} from synset {}", lemma, synset_id.as_str());
    let links = wn.sense_links_to(lemma, pos, synset_id)?;
    for sense_id in  wn.remove_sense(lemma, pos, synset_id)? {
        for (_, source) in links.iter() {
            wn.remove_sense_rel(&source, &sense_id)?;
        }
    }
    change_list.mark();
    wn.update_synset(&synset_id, |synset| {
        synset.members.retain(|l| l != lemma);
        if warn && synset.members.is_empty() {
            println!("{} is now empty! Please add at least one new member before saving", synset_id.as_str());
        }
    }).unwrap_or_else(|_| {
        eprintln!("Removing entry from non-existant synset");
    });
    Ok(())
}

/// Change the order of members in a synset
pub fn change_members<L : Lexicon>(wn : &mut L,
                  synset_id : &SynsetId, members : Vec<String>,
                  change_list : &mut ChangeList) -> Result<()> {
    let mut to_add = Vec::new();
    let mut to_delete = Vec::new();
    wn.update_synset(synset_id, |synset| {
        for member in synset.members.iter() {
            if !members.contains(member) {
                to_delete.push(member.clone());
            }
        }
        for member in members.iter() {
            if !synset.members.contains(member) {
                to_add.push(member.clone());
            }
        }
        synset.members = members;
        change_list.mark();
    }).unwrap_or_else(|_| {
            eprintln!("Changing members of non-existant synset");
    });
    for member in to_delete {
        if let Some(pos_key) = wn.pos_for_entry_synset(&member, synset_id)? { 
            delete_entry(wn, synset_id, &member, 
                &pos_key,
                false, change_list)?;
        } else {
            eprintln!("Member {} not found in synset {} (skipping)", member, synset_id.as_str());
        }
    }
    for member in to_add {
        add_entry(wn, synset_id.clone(), member.clone(), 
            wn.pos_for_entry_synset(&member, synset_id)?.unwrap_or_else(|| {
            let synset = wn.synset_by_id(synset_id).unwrap().unwrap();
            synset.part_of_speech.to_pos_key()
        }),
            Vec::new(), None, change_list)?;
    }
    Ok(())
}

/// Move an entry to another synset
pub fn move_entry<L : Lexicon>(wn : &mut L,
              synset_id : SynsetId, 
              target_synset_id : SynsetId,
              lemma : String, pos : PosKey,
              change_list : &mut ChangeList) -> Result<()> {

    let links_from = wn.sense_links_from(&lemma, &pos, &synset_id)?;
    let links_to   = wn.sense_links_to(&lemma, &pos, &synset_id)?;
    let forms = wn.get_forms(&lemma, &pos)?;
    let pronunciations = wn.get_pronunciations(&lemma, &pos)?;
    let subcat : Vec<String> = wn.get_sense(&lemma, &synset_id)?.get(0)
        .map(|s| s.subcat.clone())
        .unwrap_or(Vec::new());
    let old_sense_id = match wn.get_sense(&lemma, &synset_id)?.iter().next() {
        Some(sense) => if wn.lex_name_for(&synset_id)? == wn.lex_name_for(&target_synset_id)? {
                Some(sense.id.clone())
        } else {
            None
        },
        None => None
    };
    delete_entry(wn, &synset_id, &lemma, &pos, true, change_list)?;
    match add_entry(wn, target_synset_id, 
                    lemma.clone(), pos.clone(), subcat, 
                    old_sense_id.as_ref(), change_list)? {
        Some(sense_id) => {
            for (rel, target) in links_from {
                wn.add_sense_rel(&sense_id, rel, &target)?;
            }
            for (rel, source) in links_to {
                wn.add_sense_rel(&source, rel, &sense_id)?;
            }
        },
        None => {
            println!("New synset not created");
        }
    };
    for form in forms {
        wn.add_form(&lemma, &pos, form)?;
    }
    for pron in pronunciations {
        wn.add_pronunciation(&lemma, &pos, pron)?;
    }
    Ok(())
}

/// Delete a synset
pub fn delete_synset<L : Lexicon>(wn : &mut L,
                 synset_id : &SynsetId,
                 supersede_id : Option<&SynsetId>,
                 reason : String, change_list: &mut ChangeList) -> Result<()> {
    println!("Deleting synset {}", synset_id.as_str());

    if let Some(supersede_id) = supersede_id {
        let entries = wn.members_by_id(synset_id)?;
        for entry in entries {
                if let Some(pos) = wn.pos_for_entry_synset(&entry, synset_id)? {
                    move_entry(wn, synset_id.clone(), supersede_id.clone(),
                        entry, pos, change_list)?;

                }
        }

        let mut examples = Vec::new();
        for example in wn.synset_by_id(synset_id)?.iter().flat_map(|ss| ss.example.iter()) {
            examples.push(example.clone());
        }
        for example in examples {
            wn.update_synset(supersede_id, |ss_sup| {
                ss_sup.example.push(example.clone());
                change_list.mark();
            }).unwrap_or_else(|_| {
                eprintln!("Adding example to non-existant synset");
            });
        }
    } else {
        let entries = wn.members_by_id(synset_id)?;
        for entry in entries {
            if let Some(pos) = wn.pos_for_entry_synset(&entry, synset_id)? {
                delete_entry(wn, synset_id, &entry, &pos, false, change_list)?;
            }
        }
    }
 
    match supersede_id {
        Some(supersede_id) => {
            match wn.synset_by_id(synset_id)? {
                Some(ss) => {
                    let mut hyp_targets = Vec::new();
                    for (rel, target) in ss.links_from() {
                        delete_rel(wn, synset_id, &target, change_list);
                        if rel == SynsetRelType::Hypernym {
                            hyp_targets.push(target.clone());
                        } else {
                            wn.add_rel(supersede_id, rel, &target).unwrap_or_else(|_| {
                                eprintln!("Adding relation to non-existant synset");
                            });
                        }
                    }
                    let mut hyp_sources = Vec::new();
                    for (rel, source) in wn.links_to(synset_id)? {
                        delete_rel(wn, &source, synset_id, change_list);
                        if rel == SynsetRelType::Hypernym {
                            hyp_sources.push(source.clone());
                        } else {
                            wn.add_rel(&source, rel, supersede_id).unwrap_or_else(|_| {
                                eprintln!("Adding relation to non-existant synset");
                            });
                        }
                    }
                    for source in hyp_sources {
                        for target in hyp_targets.iter() {
                            wn.add_rel(&source, SynsetRelType::Hypernym, target).unwrap_or_else(|_| {
                                eprintln!("Adding relation to non-existant synset");
                            });
                        }
                    }
                },
                None => {}
            }
        },
        None => {
            match wn.synset_by_id(synset_id)? {
                Some(ss) => {
                    for (_, target) in ss.links_from() {
                        delete_rel(wn, synset_id, &target, change_list);
                    }
                    for (_, source) in wn.links_to(synset_id)? {
                        delete_rel(wn, &source, synset_id, change_list);
                    }
                },
                None => {}
            }
        }
    }

    match supersede_id {
        Some(ss_id) => {
            wn.deprecate(synset_id, ss_id, reason)?;
        },
        None => {}
    }

    wn.remove_synset(synset_id)?;

    change_list.mark();
    Ok(())
}

fn new_id<L : Lexicon>(wn : &L,
    pos : &PartOfSpeech, definition : &str) -> result::Result<SynsetId, String> {
    let s = Sha256::digest(definition.as_bytes());
    let mut key : u32 = 0;
    for x in s.into_iter() {
        key = (key * 16 + x as u32) % 10000000;
    }
    let nid = SynsetId::new_owned(format!("8{:07}-{}", key, pos.value()));
    match wn.synset_by_id(&nid).map_err(|e| e.to_string())? {
        Some(ss) => Err(format!("Duplicate Synset ID. This is likely due to a duplicate definition ({} \"{}\"/\"{}\")", nid.as_str(), ss.definition[0], definition)),
        None => Ok(nid)
    }
}

/// Add a synset. Fails if POS key has invalid value.
pub fn add_synset<L : Lexicon>(wn : &mut L,
              definition : String, lexfile : String,
              pos : PosKey, ssid : Option<SynsetId>, 
              change_list : &mut ChangeList) -> result::Result<SynsetId, String> {
   match pos.to_part_of_speech() {
        Some(pos) => {
            let ssid = match ssid {
                Some(ssid) => ssid,
                None => new_id(wn, &pos, &definition)?
            };
            let mut synset = Synset::new(pos);
            synset.definition.push(definition);
            wn.insert_synset(lexfile, ssid.clone(), synset).map_err(|e| e.to_string())?;
            change_list.mark();
            Ok(ssid)
        },
        None => {
            Err(format!("Part of speech value is not valid"))
        }
    }
}

fn find_rel_type<L : Lexicon>(wn : &L,
    source : &SynsetId, target : &SynsetId) 
    -> Result<Vec<SynsetRelType>> {
        Ok(wn.links_from(source)?.into_iter()
            .filter(|x| x.1 == *target).map(|x| x.0).chain(
            wn.links_from(target)?.into_iter()
            .filter(|x| x.1 == *source).map(|x| x.0)).collect())
}



/// Reverse the direction of relations
pub fn reverse_rel<L : Lexicon>(wn : &mut L,
               source : &SynsetId,
               target : &SynsetId, change_list : &mut ChangeList) -> Result<()> {
    for rel_type in find_rel_type(wn, source, target)? {
        delete_rel(wn, source, target, change_list);
        insert_rel(wn, target, &rel_type, source, change_list)?;
    }
    Ok(())
}

/// Add a relation between senses
pub fn insert_sense_relation<L : Lexicon>(wn : &mut L,
                      source : SenseId, rel : SenseRelType,
                      target : SenseId, change_list : &mut ChangeList) -> Result<()> {
    println!("Insert {} ={}=> {}", source.as_str(), rel.value(), target.as_str());
    if rel.is_symmetric() {
        wn.add_sense_rel(&target, rel.clone(), &source)?;
    }
    wn.add_sense_rel(&source, rel, &target)?;
    change_list.mark();
    Ok(())
}

fn find_sense_rel_type<L : Lexicon>(wn : &L,
    source : &SenseId, target : &SenseId) 
    -> Result<Vec<SenseRelType>> {
        Ok(wn.sense_links_from_id(source)?.into_iter()
            .filter(|x| x.1 == *target).map(|x| x.0).chain(
            wn.sense_links_from_id(target)?.into_iter()
            .filter(|x| x.1 == *source).map(|x| x.0)).collect())
}

/// Reverse the direction of a sense relation
pub fn reverse_sense_rel<L : Lexicon>(wn : &mut L,
                      source : &SenseId,
                      target : &SenseId, change_list : &mut ChangeList) -> Result<()> {
    for rel_type in find_sense_rel_type(wn, source, target)? {
        delete_sense_rel(wn, source, target, change_list)?;
        insert_sense_relation(wn, target.clone(), rel_type, source.clone(), change_list)?;
    }
    Ok(())
}

/// Change a definition
pub fn update_def<L : Lexicon>(wn : &mut L,
              synset_id : &SynsetId, defn : String,
              add : bool) {
    wn.update_synset(synset_id, |synset| {
        if add {
            synset.definition.push(defn.to_string())
        } else {
            synset.definition = vec![defn.to_string()]
        }
    }).unwrap_or_else(|_| {
        eprintln!("Changing definition of non-existant synset {}", synset_id.as_str());
    });
}

/// Add an example
pub fn add_ex<L : Lexicon>(wn : &mut L,
          synset_id : &SynsetId, example : String,
          source : Option<String>, change_list : &mut ChangeList) {
    wn.update_synset(synset_id, |ss| {
        ss.example.push(Example::new(example, source));
        change_list.mark();
    }).unwrap_or_else(|_| {
        eprintln!("Adding example to non-existant synset");
    });
}

/// Remove the nth example
pub fn delete_ex<L : Lexicon>(wn : &mut L,
             synset_id : &SynsetId, idx : usize,
             change_list : &mut ChangeList) {
    wn.update_synset(synset_id, |ss| {
        if idx >= ss.example.len() {
            eprintln!("Example index {} out of range", idx);
        } else {
            ss.example.remove(idx);
            change_list.mark();
        }
    }).unwrap_or_else(|_| {
        eprintln!("Adding example to non-existant synset");
    });
}

/// Remove all indirect relations
pub fn fix_indirect_relations<L : Lexicon>(wn : &mut L,
            change_list : &mut ChangeList) -> Result<()> {
    let mut to_delete = Vec::new();
    for synset in wn.synsets()? {
        let (synset_id, synset) = synset?;
        for target in synset.hypernym.iter() {
            match wn.synset_by_id(target)? {
                Some(synset2) => {
                    for target2 in synset2.hypernym.iter() {
                        if synset.hypernym.iter().any(|t| t == target2) {
                            to_delete.push((synset_id.clone(), target2.clone()));
                        }
                    }
                },
                None => {}
            }
        }
    }
    for (source, target2) in to_delete {
        delete_rel(wn, &source, &target2, change_list);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wordnet::LexiconHashMapBackend;

    #[test]
    fn test_move_entry() {
        let mut wn = LexiconHashMapBackend::new();
        let mut change_list = ChangeList::new();
        let synset_id = add_synset(&mut wn, "test".to_owned(), "noun.object".to_owned(), 
            PosKey::new("n".to_owned()), None, &mut change_list).unwrap();
        eprintln!("synset {:?}", wn.synset_by_id(&synset_id).unwrap());
        let target_synset_id = add_synset(&mut wn, "another test".to_owned(), "noun.object".to_owned(), 
            PosKey::new("n".to_owned()), None, &mut change_list).unwrap();
        let synset3 = add_synset(&mut wn, "third test".to_owned(), "noun.object".to_owned(), 
            PosKey::new("n".to_owned()), None, &mut change_list).unwrap();
        let lemma = "test".to_owned();
        let pos = PosKey::new("n".to_owned());
        add_entry(&mut wn, synset_id.clone(), lemma.clone(), pos.clone(), Vec::new(), None, &mut change_list).unwrap();
        add_entry(&mut wn, synset3.clone(), lemma.clone(), pos.clone(), Vec::new(), None, &mut change_list).unwrap();
        let entry = wn.entry_by_lemma("test").unwrap();
        eprintln!("entry={:?}", entry);
        let entry1 = entry.iter().next().unwrap();
        let sense = entry1.sense.iter().next().unwrap(); 
        assert_eq!(sense.id, SenseId::new("test%1:17:00::".to_owned()));
        move_entry(&mut wn, synset_id.clone(), target_synset_id.clone(), lemma.clone(), pos.clone(), &mut change_list).unwrap();
        let entry = wn.entry_by_lemma("test").unwrap();
        let sense = entry.iter().next().unwrap().sense.iter()
            .filter(|sense| sense.synset == target_synset_id).next().unwrap();
        assert_eq!(sense.id, SenseId::new("test%1:17:00::".to_owned()));
    }
}
