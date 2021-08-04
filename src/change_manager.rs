//"""Utility functions for changing the wordnet"""
//from wordnet import *
//import pickle
//import os
//from glob import glob
//import fileinput
//import hashlib
//from merge import wn_merge
//import wordnet_yaml
//from collections import defaultdict
//from sense_keys import get_sense_key
//
//sense_id_re = re.compile(r"ewn-(.*)-(.)-(\d{8})-\d{2}")
//
//
use crate::wordnet_yaml::*;
use crate::rels::*;
use crate::sense_keys::get_sense_key;

pub struct ChangeList(bool);

impl ChangeList {
    pub fn changed(&self) -> bool { self.0 }
    pub fn mark(&mut self) { self.0 = true; }
    pub fn reset(&mut self) { self.0 = false; }
    pub fn new() -> ChangeList { ChangeList(false) }
}


//pub struct ChangeList {
//    lexfiles : HashSet<String>,
//    entry_files : HashSet<char>
//}
//
//impl ChangeList {
//    fn new() -> ChangeList {
//        ChangeList {
//            lexfiles: HashSet::new(),
//            entry_files: HashSet::new()
//        }
//    }
//
//    fn change_entry(&mut self, wn : &Lexicon, entry : &Entry, lemma : &str) {
//        for sense in entry.sense.iter() {
//            match wn.lex_name_for(&sense.synset) {
//                Some(s) => {
//                    self.lexfiles.insert(s);
//                },
//                None => {
//                    eprintln!("Synset without lexfile: {:?}", sense.synset);
//                }
//            }
//            let mut entry_key = lemma.chars().nth(0).expect("Empty lemma!");
//            if entry_key < 'a' || entry_key > 'z' {
//                entry_key = '0';
//            }
//            self.entry_files.insert(entry_key);
//        }
//
//    }
//
//    fn change_synset(&mut self, wn : &Lexicon, synset : &SynsetId) {
//        match wn.lex_name_for(&synset) {
//            Some(s) => {
//                self.lexfiles.insert(s);
//            },
//            None => {
//                eprintln!("Synset without lexfile: {:?}", synset);
//            }
//        }
//    }
//         
//}

pub fn delete_rel(wn : &Lexicon, source_id : &SynsetId,
                  source : &mut Synset, target : &SynsetId) {
    println!("Delete {} =*=> {}", source_id.as_str(), target.as_str());
    source.remove_all_relations(target);
    //match change_list {
    //    Some(cl) => cl.change_synset(wn, source_id),
    //    None => {}
    //};
}

pub fn delete_sense_rel(wn : &Lexicon, 
                        sense : &mut Sense, target : &SenseId) {
    println!("Delete {} =*=> {}", sense.id.as_str(), target.as_str());
    sense.remove_all_relations(target);
}

pub fn insert_rel(source : &mut Synset, source_id : &SynsetId,
                  rel_type : &SynsetRelType,
                  target : &mut Synset, target_id : &SynsetId) {
    println!("Insert {} ={}=> {}", source_id.as_str(), rel_type.value(),
                    target_id.as_str());
    let (non_inv, yaml_rel_type) = rel_type.clone().to_yaml();
    if non_inv {
        source.insert_rel(&yaml_rel_type, target_id);
    } else {
        target.insert_rel(&yaml_rel_type, source_id);
    }
}
//def insert_rel(source, rel_type, target, change_list=None):
//    """Insert a single relation between two synsets"""
//    print("Insert %s =%s=> %s" % (source.id, rel_type, target.id))
//    ss = source
//    if [r for r in ss.synset_relations if r.target ==
//            target.id and r.rel_type == rel_type]:
//        print("Already exists")
//        return
//    ss.synset_relations.append(SynsetRelation(target.id, rel_type))
//    if change_list:
//        change_list.change_synset(target)
//
//
//def empty_if_none(x):
//    """Returns an empty list if passed None otherwise the argument"""
//    if x:
//        return x
//    else:
//        return []
//
//
//def synset_key(synset_id):
//    return synset_id[4:-2]
//
//
//def change_entry(wn, synset, target_synset, lemma, change_list=None):
//    """Change an entry, only works if both synsets are in the same file"""
//    print("Adding %s to synset %s" % (lemma, synset.id))
//    n_entries = len(empty_if_none(wn.members_by_id(target_synset.id)))
//    entry_global = [entry for entry in empty_if_none(wn.entry_by_lemma(lemma))
//                    if wn.entry_by_id(entry).lemma.part_of_speech == synset.part_of_speech or
//                    wn.entry_by_id(entry).lemma.part_of_speech == PartOfSpeech.ADJECTIVE and synset.part_of_speech == PartOfSpeech.ADJECTIVE_SATELLITE or
//                    wn.entry_by_id(entry).lemma.part_of_speech == PartOfSpeech.ADJECTIVE_SATELLITE and synset.part_of_speech == PartOfSpeech.ADJECTIVE]
//
//    if len(entry_global) == 1:
//        entry_global = wn.entry_by_id(entry_global[0])
//        n_senses = len(entry_global.senses)
//    else:
//        entry_global = None
//        n_senses = 0
//
//    idx = n_entries + 1
//    n = n_senses
//
//    wn_synset = wn
//    entries = [entry for entry in empty_if_none(wn_synset.entry_by_lemma(
//        lemma)) if wn.entry_by_id(entry).lemma.part_of_speech == synset.part_of_speech]
//
//    for entry in entries:
//        for sense in wn_synset.entry_by_id(entry).senses:
//            if sense.synset == synset.id:
//                print("Moving %s to %s" % (sense.id, target_synset.id))
//                sense.synset = target_synset.id
//                wn.change_sense_id(
//                    sense,
//                    "ewn-%s-%s-%s-%02d" %
//                    (escape_lemma(lemma),
//                     target_synset.part_of_speech.value,
//                     synset_key(
//                        target_synset.id),
//                        idx),
//                    change_list)
//    if change_list:
//        change_list.change_entry(wn, entry)
//
//
pub fn add_entry(wn : &mut Lexicon, synset_id : SynsetId, 
                 synset_pos : String,
                 lemma : String, 
                 change_list : &mut ChangeList) {
    println!("Adding {} to synset {}", lemma, synset_id.as_str());

    let mut entries = wn.entry_by_lemma_with_pos(&lemma).iter_mut()
        .filter(|(pos, lemma)| synset_pos == **pos)
        .map(|x| x.1)
        .collect::<Vec<&Entry>>();


    if entries.len() > 1 {
        println!("More than one entry for {} ({}). Please check the YAML file",
            lemma, synset_pos);
    }

    let mut entry = entries.pop();

    match entry {
        Some(e) => {
            match wn.synset_by_id(&synset_id) {
                Some(synset) => {
                    let sense = Sense::new(
                            get_sense_key(wn, &lemma, e, None, synset, &synset_id),
                            synset_id.clone());
                    wn.insert_sense(lemma.clone(), synset_pos.clone(), sense);
                    change_list.mark();
                },
                None => {}
            }
        },
        None => { 
            match wn.synset_by_id(&synset_id) {
                Some(synset) => {
                    let e = Entry::new();
                    let sense = Sense::new(
                            get_sense_key(wn, &lemma, &e, None, synset, &synset_id),
                            synset_id.clone());
                    wn.insert_entry(lemma.clone(), synset_pos.clone(), e);
                    wn.insert_sense(lemma.clone(), synset_pos.clone(), sense);
                    change_list.mark();
                },
                None => {}
            }
        }
    }
    match wn.synset_by_id_mut(&synset_id) {
        Some(ref mut synset) => {
            synset.members.push(lemma.clone());
            change_list.mark();
        },
        None => {
            eprintln!("Adding entry to non-existant synset");
        }
    }
}


pub fn delete_entry(wn : &mut Lexicon, synset_id : &SynsetId, lemma : &str, 
                    pos : &str, change_list : &mut ChangeList) {
    wn.remove_sense(lemma, pos, synset_id);
    change_list.mark();
    match wn.synset_by_id_mut(&synset_id) {
        Some(ref mut synset) => {
            synset.members.retain(|l| l != lemma);
            if synset.members.is_empty() {
                println!("{} is now empty! Please add at least one new member before saving", synset_id.as_str());
            }
        },
        None => {
            eprintln!("Removing entry from non-existant synset");
        }
    }

}
//def delete_entry(wn, synset, entry_id, change_list=None):
//    """Delete a lemma from a synset"""
//    print("Deleting %s from synset %s" % (entry_id, synset.id))
//    n_entries = len(wn.members_by_id(synset.id))
//    entry_global = wn.entry_by_id(entry_id)
//
//    if entry_global:
//        idxs = [int(sense.id[-2:])
//                for sense in entry_global.senses if sense.synset == synset.id]
//        if not idxs:
//            print("Entry not in synset")
//            return
//        idx = idxs[0]
//        n_senses = len(entry_global.senses)
//    else:
//        print("No entry for this lemma")
//        return
//    
//    if n_senses == 0:
//        entry = wn_synset.entry_by_id(entry_global.id)
//        if entry:
//            wn.del_entry(entry)
//        return
//
//    if n_senses != 1:
//        n = [ind for ind, sense in enumerate(
//            entry_global.senses) if sense.synset == synset.id][0]
//        sense_n = 0
//        for sense in entry_global.senses:
//            if sense_n >= n:
//                change_sense_n(wn, entry_global, sense.id, sense_n - 1)
//            sense_n += 1
//
//    for sense_id in sense_ids_for_synset(wn, synset):
//        this_idx = int(sense_id[-2:])
//        if this_idx > idx:
//            change_sense_idx(wn, sense_id, this_idx - 1)
//
//    for sense in entry_global.senses:
//        if sense.synset == synset.id:
//            for rel in sense.sense_relations:
//                delete_sense_rel(wn, rel.target, sense.id, change_list)
//                delete_sense_rel(wn, sense.id, rel.target, change_list)
//
//    if n_senses == 1:  # then delete the whole entry
//        wn_synset = wn
//        entry = wn_synset.entry_by_id(entry_global.id)
//        if change_list:
//            change_list.change_entry(wn, entry)
//        wn_synset.del_entry(entry)
//        wn.del_entry(entry)
//    else:
//        wn_synset = wn
//        entry = wn_synset.entry_by_id(entry_global.id)
//        if change_list:
//            change_list.change_entry(wn, entry)
//        sense = [s for s in entry.senses if s.synset == synset.id]
//        if sense:
//            sense = sense[0]
//            wn_synset.del_sense(entry, sense)
//            wn.del_sense(entry, sense)
//        else:
//            print("this may be a bug")
//
//
//def delete_synset(
//        wn,
//        synset,
//        supersede,
//        reason,
//        delent=True,
//        change_list=None):
//    """Delete a synset"""
//    print("Deleting synset %s" % synset.id)
//
//    if delent:
//        entries = empty_if_none(wn.members_by_id(synset.id))
//
//        for entry in entries:
//            delete_entry(
//                wn, synset, "ewn-%s-%s" %
//                (escape_lemma(entry), synset.part_of_speech.value), change_list)
//
//    for rel in synset.synset_relations:
//        delete_rel(wn.synset_by_id(rel.target), synset, change_list)
//
//    wn_synset = wn
//    wn_synset.synsets = [ss for ss in wn_synset.synsets
//                         if synset.id != ss.id]
//    if supersede:
//        if not isinstance(supersede, list):
//            supersede = [supersede]
//    else:
//        supersede = []
//    with open("src/deprecations.csv", "a") as out:
//        out.write("\"%s\",\"%s\",\"%s\",\"%s\",\"%s\"\n" %
//                  (synset.id, synset.ili,
//                   ",".join(s.id for s in supersede),
//                   ",".join(s.ili for s in supersede),
//                   reason.replace("\n", "").replace("\"", "\"\"")))
//    if change_list:
//        change_list.change_synset(synset)
//
//
//def change_sense_n(wn, entry, sense_id, new_n, change_list=None):
//    """Change the position of a sense within an entry (changes only this sense)"""
//    print("Changing n of sense %s of %s to %s" %
//          (sense_id, entry.lemma.written_form, new_n))
//    if new_n <= 0:
//        return
//
//    senses = [sense for sense in entry.senses if sense.id == sense_id]
//    if len(senses) != 1:
//        raise Exception("Could not find sense")
//    sense = senses[0]
//    synset = wn.synset_by_id(sense.synset)
//    lexname = synset.lex_name
//
//    wn_synset = wn
//    entry = wn_synset.entry_by_id(entry.id)
//    sense = [sense for sense in entry.senses if sense.id == sense_id][0]
//    sense.n = new_n
//    if change_list:
//        change_list.change_entry(wn, entry)
//
//
//def change_sense_idx(wn, sense_id, new_idx, change_list=None):
//    """Change the position of a lemma within a synset"""
//    print("Changing idx of sense %s to %s" % (sense_id, new_idx))
//    new_sense_id = "%s-%02d" % (sense_id[:-3], new_idx)
//    for entry in wn.entries:
//        for sense in entry.senses:
//            if sense.id == sense_id:
//                wn.change_sense_id(sense, new_sense_id)
//            for sr in sense.sense_relations:
//                if sr.target == sense_id:
//                    sr.target = new_sense_id
//        for sb in entry.syntactic_behaviours:
//            sb.senses = [
//                new_sense_id if s == sense_id else s
//                for s in sb.senses]
//        if change_list:
//            change_list.change_entry(wn, entry)
//
//
//def sense_ids_for_synset(wn, synset):
//    return [sense.id for lemma in wn.members_by_id(synset.id)
//            for entry in wn.entry_by_lemma(lemma)
//            for sense in wn.entry_by_id(entry).senses
//            if sense.synset == synset.id]
//
//
//def new_id(wn, pos, definition):
//    s = hashlib.sha256()
//    s.update(definition.encode())
//    nid = "ewn-8%07d-%s" % ((int(s.hexdigest(), 16) % 10000000), pos)
//    if wn.synset_by_id(nid):
//        print(
//            "Could not find ID for new synset. Either a duplicate definition or a hash collision for " +
//            nid +
//            ". Note it is possible to force a synset ID by giving it as an argument")
//        sys.exit(-1)
//    return nid
//
//
//def add_synset(wn, definition, lexfile, pos, ssid=None, change_list=None):
//    if not ssid:
//        ssid = new_id(wn, pos, definition)
//    ss = Synset(ssid, "in",
//                PartOfSpeech(pos), lexfile)
//    ss.definitions = [Definition(definition)] 
//    ss.ili_definition = Definition(definition)
//    wn.add_synset(ss)
//    if change_list:
//        change_list.change_synset(ss)
//    return ssid
//
//
//def merge_synset(wn, synsets, reason, lexfile, ssid=None, change_list=None):
//    """Create a new synset merging all the facts from other synsets"""
//    pos = synsets[0].part_of_speech.value
//    if not ssid:
//        ssid = new_id(wn, pos, synsets[0].definitions[0].text)
//    ss = Synset(ssid, "in",
//                PartOfSpeech(pos), lexfile)
//    ss.definitions = [d for s in synsets for d in s.definitions]
//    ss.examples = [x for s in synsets for x in s.examples]
//    members = {}
//    wn.add_synset(ss)
//
//    for s in synsets:
//        # Add all relations
//        for r in s.synset_relations:
//            if not any(r == r2 for r2 in ss.synset_relations):
//                add_relation(
//                    wn, ss, wn.synset_by_id(
//                        r.target), r.rel_type, change_list)
//        # Add members
//        for m in wn.members_by_id(s.id):
//            if m not in members:
//                members[m] = add_entry(wn, ss, m, change_list)
//                add_entry(wn, ss, m, change_list)
//            e = [e for e in [wn.entry_by_id(e2) for e2 in wn.entry_by_lemma(m)]
//                 if e.lemma.part_of_speech.value == pos][0]
//            for f in e.forms:
//                if not any(f2 == f for f in members[m].forms):
//                    members[m].add_form(f)
//            # syn behaviours - probably fix manually for the moment
//    if change_list:
//        change_list.change_synset(ss)
//    return ss
//
//
//def find_type(source, target):
//    """Get the first relation type between the synsets"""
//    x = [r for r in source.synset_relations if r.target == target.id]
//    if len(x) != 1:
//        raise Exception(
//            "Synsets not linked or linked by more than one property")
//    return x[0].rel_type
//
//
//def update_source(wn, old_source, target, new_source, change_list=None):
//    """Change the source of a link"""
//    rel_type = find_type(old_source, target)
//    delete_rel(old_source, target, change_list)
//    insert_rel(new_source, rel_type, target, change_list)
//    if rel_type in wordnet.inverse_synset_rels:
//        inv_rel_type = wordnet.inverse_synset_rels[rel_type]
//        delete_rel(target, old_source, change_list)
//        insert_rel(target, inv_rel_type, new_source, change_list)
//
//
//def update_target(wn, source, old_target, new_target, change_list=None):
//    """Change the target of a link"""
//    rel_type = find_type(source, old_target)
//    delete_rel(source, old_target, change_list)
//    insert_rel(source, rel_type, new_target, change_list)
//    if rel_type in wordnet.inverse_synset_rels:
//        inv_rel_type = wordnet.inverse_synset_rels[rel_type]
//        delete_rel(old_target, source, change_list)
//        insert_rel(new_target, inv_rel_type, source, change_list)
//
//
//def update_relation(wn, source, target, new_rel, change_list=None):
//    """Change the type of a link"""
//    delete_rel(source, target, change_list)
//    insert_rel(source, new_rel, target, change_list)
//    if new_rel in inverse_synset_rels:
//        inv_rel_type = inverse_synset_rels[new_rel]
//        delete_rel(target, source, change_list)
//        insert_rel(target, inv_rel_type, source, change_list)
//
//
//def add_relation(wn, source, target, new_rel, change_list=None):
//    """Change the type of a link"""
//    insert_rel(source, new_rel, target, change_list)
//    if new_rel in inverse_synset_rels:
//        inv_rel_type = inverse_synset_rels[new_rel]
//        insert_rel(target, inv_rel_type, source, change_list)
//
//
//def delete_relation(wn, source, target, change_list=None):
//    """Change the type of a link"""
//    delete_rel(source, target, change_list)
//    delete_rel(target, source, change_list)
//
//
//def reverse_rel(wn, source, target, change_list=None):
//    """Reverse the direction of relations"""
//    rel_type = find_type(source, target)
//    delete_rel(source, target, change_list)
//    if rel_type in inverse_synset_rels:
//        delete_rel(target, source, change_list)
//    insert_rel(target, rel_type, source, change_list)
//    if rel_type in inverse_synset_rels:
//        inv_rel_type = inverse_synset_rels[rel_type]
//        insert_rel(source, inv_rel_type, target, change_list)
//
//
//def delete_sense_rel(wn, source, target, change_list=None):
//    """Delete all relationships between two senses"""
//    print("Delete %s =*=> %s" % (source, target))
//    (source_synset, source_entry) = decompose_sense_id(source)
//    lex_name = wn.synset_by_id(source_synset).lex_name
//    wn_source = wn
//    entry = wn_source.entry_by_id(source_entry)
//    if entry:
//        sense = [sense for sense in entry.senses if sense.id == source][0]
//        if not any(r for r in sense.sense_relations if r.target == target):
//            print("No sense relations deleted")
//        else:
//            sense.sense_relations = [
//                r for r in sense.sense_relations if r.target != target]
//            if change_list:
//                change_list.change_entry(wn, entry)
//    else:
//        print("No entry for " + source_entry)
//
//
//def insert_sense_rel(wn, source, rel_type, target, change_list=None):
//    """Insert a single relation between two senses"""
//    print("Insert %s =%s=> %s" % (source, rel_type, target))
//    (source_synset, source_entry) = decompose_sense_id(source)
//    lex_name = wn.synset_by_id(source_synset).lex_name
//    wn_source = wn
//    entry = wn_source.entry_by_id(source_entry)
//    sense = [sense for sense in entry.senses if sense.id == source][0]
//    sense.sense_relations.append(SenseRelation(target, rel_type))
//    if change_list:
//        change_list.change_entry(wn, entry)
//
//
//def find_sense_type(wn, source, target):
//    """Get the first relation type between the senses"""
//    (source_synset, source_entry) = decompose_sense_id(source)
//    entry = wn.entry_by_id(source_entry)
//    sense = [sense for sense in entry.senses if sense.id == source][0]
//    x = set([r for r in sense.sense_relations if r.target == target])
//    if len(x) == 0:
//        raise Exception(
//            "Synsets not linked or linked by more than one property")
//    return next(iter(x)).rel_type
//
//
//def update_source_sense(wn, old_source, target, new_source, change_list=None):
//    """Change the source of a link"""
//    rel_type = find_sense_type(wn, old_source, target)
//    delete_sense_rel(wn, old_source, target, change_list)
//    insert_sense_rel(wn, new_source, rel_type, target, change_list)
//    if rel_type in inverse_sense_rels:
//        inv_rel_type = inverse_sense_rels[rel_type]
//        delete_sense_rel(wn, target, old_source, change_list)
//        insert_sense_rel(wn, target, inv_rel_type, new_source, change_list)
//
//
//def update_target_sense(wn, source, old_target, new_target, change_list=None):
//    """Change the target of a link"""
//    rel_type = find_sense_type(wn, source, old_target)
//    delete_sense_rel(wn, source, old_target, change_list)
//    insert_sense_rel(wn, source, rel_type, new_target, change_list)
//    if rel_type in inverse_sense_rels:
//        inv_rel_type = inverse_sense_rels[rel_type]
//        delete_sense_rel(wn, old_target, source, change_list)
//        insert_sense_rel(wn, new_target, inv_rel_type, source, change_list)
//
//
//def update_sense_relation(wn, source, target, new_rel, change_list=None):
//    """Change the type of a link"""
//    delete_sense_rel(wn, source, target, change_list)
//    insert_sense_rel(wn, source, new_rel, target, change_list)
//    if new_rel in inverse_sense_rels:
//        inv_rel_type = inverse_sense_rels[new_rel]
//        delete_sense_rel(wn, target, source, change_list)
//        insert_sense_rel(wn, target, inv_rel_type, source, change_list)
//
//
//def add_sense_relation(wn, source, target, new_rel, change_list=None):
//    """Change the type of a link"""
//    insert_sense_rel(wn, source, new_rel, target, change_list)
//    if new_rel in inverse_sense_rels:
//        inv_rel_type = inverse_sense_rels[new_rel]
//        insert_sense_rel(wn, target, inv_rel_type, source, change_list)
//
//
//def delete_sense_relation(wn, source, target, change_list=None):
//    """Change the type of a link"""
//    delete_sense_rel(wn, source, target, change_list)
//    delete_sense_rel(wn, target, source, change_list)
//
//
//def reverse_sense_rel(wn, source, target, change_list=None):
//    """Reverse the direction of a sense relation"""
//    rel_type = find_sense_type(wn, source, target)
//    delete_sense_rel(wn, source, target, change_list)
//    if rel_type in inverse_sense_rels:
//        delete_sense_rel(wn, target, source, change_list)
//    insert_sense_rel(wn, target, rel_type, source, change_list)
//    if rel_type in inverse_sense_rels:
//        inv_rel_type = inverse_sense_rels[rel_type]
//        insert_sense_rel(wn, source, inv_rel_type, target, change_list)
//
//
//def sense_exists(wn, sense_id):
//    if sense_id_re.match(sense_id):
//        (_, entry_id) = decompose_sense_id(sense_id)
//        entry = wn.entry_by_id(entry_id)
//        if entry:
//            senses = [sense for sense in entry.senses if sense.id == sense_id]
//            return len(senses) == 1
//    return False
//
//
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
//def update_def(wn, synset, defn, add, change_list=None):
//    wn_synset = wn
//    ss = wn_synset.synset_by_id(synset.id)
//    if add:
//        ss.definitions = ss.definitions + [Definition(defn)]
//    else:
//        ss.definitions = [Definition(defn)]
//    if change_list:
//        change_list.change_synset(synset)
//
//
//def update_ili_def(wn, synset, defn, change_list=None):
//    wn_synset = wn
//    ss = wn_synset.synset_by_id(synset.id)
//    ss.ili_definition = Definition(defn)
//    if change_list:
//        change_list.change_synset(synset)
//
//
//def add_ex(wn, synset, example, change_list=None):
//    wn_synset = wn
//    ss = wn_synset.synset_by_id(synset.id)
//    ss.examples = ss.examples + [Example(example)]
//    if change_list:
//        change_list.change_synset(synset)
//
//
//def delete_ex(wn, synset, example, change_list=None):
//    wn_synset = wn
//    ss = wn_synset.synset_by_id(synset.id)
//    n_exs = len(ss.examples)
//    ss.examples = [ex for ex in ss.examples if ex.text != example]
//    if len(ss.examples) == n_exs:
//        print("No change")
//    if change_list:
//        change_list.change_synset(synset)
