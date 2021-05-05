use crate::wordnet::*;
use regex::Regex;
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
//
lazy_static! {
    static ref sense_id_re : Regex = Regex::new("^ewn-(.*)-(.)-(\\d{8})-\\d{2}$").unwrap();
}
//
//def load_wordnet():
//    """Load the wordnet from disk"""
//    mode = None
//    # Use whichever version is latest
//    mtime_xml = max(os.path.getmtime(f) for f in glob("src/xml/*.xml"))
//    mtime_yaml = max(os.path.getmtime(f) for f in glob("src/yaml/*.yaml"))
//    if os.path.exists("wn.xml"):
//        mtime_wn_xml = os.path.getmtime("wn.xml")
//    else:
//        mtime_wn_xml = 0
//    if os.path.exists("wn.pickle"):
//        mtime_pickle = os.path.getmtime("wn.pickle")
//    else:
//        mtime_pickle = 0
//    if mtime_yaml > mtime_xml and mtime_yaml > mtime_wn_xml and mtime_yaml > mtime_pickle:
//        print("Reading from YAML")
//        wn = wordnet_yaml.load()
//        pickle.dump(wn, open("wn.pickle", "wb"))
//    elif mtime_xml > mtime_wn_xml and mtime_xml > mtime_pickle:
//        print("Merging and reading XML")
//        wn_merge()
//        wn = parse_wordnet("wn.xml")
//        pickle.dump(wn, open("wn.pickle", "wb"))
//    elif mtime_wn_xml > mtime_pickle:
//        print("Reading XML")
//        wn = parse_wordnet("wn.xml")
//        pickle.dump(wn, open("wn.pickle", "wb"))
//    else:
//        wn = pickle.load(open("wn.pickle", "rb"))
//    return wn
//
//def save(wn):
//    """Save the wordnet to disk (all formats)"""
//    wordnet_yaml.save(wn)
//    save_all_xml(wn)
//    with codecs.open("wn.xml","w","utf-8") as outp:
//        wn.to_xml(outp, True)
//    pickle.dump(wn, open("wn.pickle", "wb"))
//
//def save_all_xml(wn):
//    by_lex_name = {}
//    for synset in wn.synsets:
//        if synset.lex_name not in by_lex_name:
//            by_lex_name[synset.lex_name] = Lexicon(
//                    "ewn", "English WordNet", "en",
//                    "john@mccr.ae", "https://wordnet.princeton.edu/license-and-commercial-use",
//                    "2019","https://github.com/globalwordnet/english-wordnet")
//        by_lex_name[synset.lex_name].add_synset(synset)
//
//    for entry in wn.entries:
//        sense_no = dict([(e.id,i) for i,e in enumerate(entry.senses)])
//        for lex_name in by_lex_name.keys():
//            senses = [sense for sense in entry.senses if wn.synset_by_id(sense.synset).lex_name == lex_name]
//            if senses:
//                e = LexicalEntry(entry.id)
//                e.set_lemma(entry.lemma)
//                for f in entry.forms:
//                    e.add_form(f)
//                for s in senses:
//                    s.n = sense_no[s.id]
//                    e.add_sense(s)
//                def find_sense_for_sb(sb_sense):
//                    for sense2 in senses:
//                        if sense2.id == sb_sense:
//                            return sense2.id
//                    return None
//                e.syntactic_behaviours = [SyntacticBehaviour(
//                    sb.subcategorization_frame,
//                    [find_sense_for_sb(sense) for sense in sb.senses])
//                    for sb in entry.syntactic_behaviours]
//                e.syntactic_behaviours = [SyntacticBehaviour(
//                    sb.subcategorization_frame, [s for s in sb.senses if s])
//                    for sb in e.syntactic_behaviours if any(sb.senses)]
//                by_lex_name[lex_name].add_entry(e)
// 
//    for lex_name, wn in by_lex_name.items():
//        if os.path.exists("src/xml/wn-%s.xml" % lex_name):
//            wn_lex = parse_wordnet("src/xml/wn-%s.xml" % lex_name)
//            wn.comments = wn_lex.comments
//            entry_order = defaultdict(lambda: 10000000,[(e,i) for i,e in enumerate(entry.id for entry in wn_lex.entries)])
//            wn.entries = sorted(wn.entries, key=lambda e: entry_order[e.id])
//            for entry in wn.entries:
//                if wn_lex.entry_by_id(entry.id):
//                    sense_order = defaultdict(lambda: 10000, [(e,i) for i,e in enumerate(sense.id for sense in wn_lex.entry_by_id(entry.id).senses)])
//                    entry.senses = sorted(entry.senses, key=lambda s: sense_order[s.id])
//                    # This is a bit of a hack as some of the n values are not continguous 
//                    for sense in entry.senses:
//                        if wn_lex.sense_by_id(sense.id):
//                            sense.n = wn_lex.sense_by_id(sense.id).n 
//                            sense_rel_order = defaultdict(lambda: 10000, [((sr.target,sr.rel_type), i)
//                                for i, sr in enumerate(wn_lex.sense_by_id(sense.id).sense_relations)])
//                            sense.sense_relations = sorted(sense.sense_relations, 
//                                key=lambda sr: sense_rel_order[(sr.target,sr.rel_type)])
//                        else:
//                            print("sense not found:" + sense.id)
//                    sb_order = defaultdict(lambda: 10000, [(e,i) for i,e in enumerate(sb.subcategorization_frame for sb in wn_lex.entry_by_id(entry.id).syntactic_behaviours)])
//                    entry.syntactic_behaviours = sorted(entry.syntactic_behaviours,
//                            key=lambda sb: sb_order[sb.subcategorization_frame])
//                    for sb in entry.syntactic_behaviours:
//                        sb2s = [sb2 for sb2 in wn_lex.entry_by_id(entry.id).syntactic_behaviours
//                                    if sb2.subcategorization_frame == sb.subcategorization_frame]
//                        if sb2s:
//                            sbe_order = defaultdict(lambda: 10000, [(e,i) 
//                                for i,e in enumerate(sb2s[0].senses)])
//                            sb.senses = sorted(sb.senses, key=lambda s: sbe_order[s])
//                else:
//                    print("not found:" + entry.id)
//            synset_order = defaultdict(lambda: 1000000, [(e,i) for i,e in enumerate(
//                synset.id for synset in wn_lex.synsets)])
//            wn.synsets = sorted(wn.synsets, key=lambda s: synset_order[s.id])
//            for synset in wn.synsets:
//                if wn_lex.synset_by_id(synset.id):
//                    synset_rel_order = defaultdict(lambda: 10000, [((sr.target, sr.rel_type), i)
//                        for i, sr in enumerate(wn_lex.synset_by_id(synset.id).synset_relations)])
//                    synset.synset_relations = sorted(synset.synset_relations,
//                        key=lambda sr: synset_rel_order[(sr.target, sr.rel_type)])
//        with codecs.open("src/xml/wn-%s.xml" % lex_name,"w","utf-8") as outp:
//            wn.to_xml(outp, True)
//
//

/// Delete all relationships between two synsets
pub fn delete_rel(source : &mut Synset, target_id : &str) -> Result<(), WordNetError> {
    println!("Delete {} =*=> {}", source.id, target_id);
    source.synset_relations = source.synset_relations.iter().filter(|r|  r.target != target_id).map(|x| x.clone()).collect();
    Ok(())
}

fn decompose_sense_id(sense_id : &str) -> Result<(String, String), WordNetError> {
    match sense_id_re.captures(sense_id) {
        Some(caps) => {
            let lemma = caps.get(1).ok_or(WordNetError::BadSenseKey(sense_id.to_owned()))?.as_str();
            let pos = caps.get(2).ok_or(WordNetError::BadSenseKey(sense_id.to_owned()))?.as_str();
            let ssid = caps.get(3).ok_or(WordNetError::BadSenseKey(sense_id.to_owned()))?.as_str();
            Ok((format!("ewn-{}-{}", ssid, pos), format!("ewn-{}-{}", lemma, pos)))
        },
        None => {
            Err(WordNetError::BadSenseKey(sense_id.to_owned()))
        }
    }

}

/// Delete all relationships between two senses
fn delete_sense_rel(wn : &mut Lexicon, source : &str, target : &str) -> Result<(),WordNetError> {
    println!("Delete {} =*=> {}", source, target);
    let (source_synset, source_entry) = decompose_sense_id(source)?;
    let entry = wn.entry_by_id_mut(&source_entry)
        .ok_or(WordNetError::EntryNotFound(source_entry))?;
    let mut sense = entry.senses.iter_mut().filter(|sense| sense.id == source)
        .next().ok_or(WordNetError::SenseNotFound(source.to_owned()))?;
    sense.sense_relations = sense.sense_relations.iter().filter(
        |r| r.target != target).map(|x| x.clone()).collect();
    Ok(())
}


//def insert_rel(source, rel_type, target):
//    """Insert a single relation between two synsets"""
fn insert_rel(source : &mut Synset, rel_type : SynsetRelType, target_id : &str) -> Result<(),WordNetError> {
    println!("Insert {} ={:?}=> {}", source.id, rel_type, target_id);
    if source.synset_relations.iter().any(|r| r.target == target_id && r.rel_type == rel_type) {
        println!("Already exists");
        Ok(())
    } else {
        source.synset_relations.push(SynsetRelation::new(target_id.to_string(),
            rel_type.clone()));
        Ok(())
    }
}

fn empty_if_none<R>(list : Option<Vec<R>>) -> Vec<R> {
    match list {
        Some(l) => l,
        None => Vec::new()
    }
}

fn synset_key(synset_id : &str) -> String {
    return synset_id[4..synset_id.len()-2].to_string()
}

fn change_entry(wn : &mut Lexicon, synset_id : &String, target_synset_id : &String,
    lemma : String) -> Result<(), WordNetError> {
    println!("Adding {} to synset {}", lemma, synset_id);
    let n_entries = wn.members_by_id(target_synset_id).len();
    let synset_part_of_speech = wn.synset_by_id(synset_id)
        .ok_or(WordNetError::SynsetNotFound(synset_id.to_owned()))?
        .part_of_speech.clone();
    let target_synset_part_of_speech = wn.synset_by_id(target_synset_id)
        .ok_or(WordNetError::SynsetNotFound(target_synset_id.to_owned()))?
        .part_of_speech.clone();
    let idx = n_entries + 1;
     
    let entries = match wn.entry_by_lemma(&lemma) {
        Some(es) => es.iter().filter(|entry| {
        wn.entry_by_id(&entry.id).unwrap().lemma.part_of_speech == synset_part_of_speech }).
        map(|entry| entry.id.to_owned()).collect(),
        None => Vec::new()
    };

    let mut sense_ids_to_change = Vec::new();
    for entry_id in entries {
        for mut sense in wn.entry_by_id_mut(&entry_id).unwrap().senses.iter_mut() {
            if sense.synset == *synset_id {
                println!("Moving {} to {}", sense.id, target_synset_id);
                sense.synset = target_synset_id.to_owned();
                sense_ids_to_change.push((sense.id.to_owned(),
                //wn.change_sense_id(&sense.id,
                    format!("ewn-{}-{}-{}-{:02}", escape_lemma(&lemma),
                        target_synset_part_of_speech.value(),
                        synset_key(target_synset_id), idx)));
            }
        }
    }
    for (sense_id, new_id) in sense_ids_to_change {
        wn.change_sense_id(&sense_id, new_id);
    }
    Ok(())
}

/// Add a new lemma to a synset
fn add_entry(wn : &mut Lexicon, synset_id : &String, lemma : String,
    idx : i32, n : i32) -> Result<String,WordNetError> {
    println!("Adding {} to synset {}", lemma, synset_id);
    let n_entries = wn.members_by_id(synset_id).len() as i32;
    let synset_part_of_speech = wn.synset_by_id(synset_id)
        .ok_or(WordNetError::SynsetNotFound(synset_id.clone()))?
        .part_of_speech
        .clone();

    let entry : Option<LexicalEntry> = match wn.entry_by_lemma(&lemma) {
        Some(l) => match l.iter().filter(|entry| {

            wn.entry_by_id(&entry.id).unwrap().lemma.part_of_speech == synset_part_of_speech ||
            wn.entry_by_id(&entry.id).unwrap().lemma.part_of_speech == PartOfSpeech::Adjective && synset_part_of_speech == PartOfSpeech::AdjectiveSatellite ||
            wn.entry_by_id(&entry.id).unwrap().lemma.part_of_speech == PartOfSpeech::AdjectiveSatellite && synset_part_of_speech == PartOfSpeech::Adjective
        }).next() {
            Some(e) => Some(e.clone().clone()),
            None => None
        },
        None => None
    };
                

    let n_senses = entry.as_ref().map(|e| e.senses.len()).unwrap_or(0) as i32;

    let idx = if idx <= 0 {
        n_entries + 1
    } else if idx > n_entries + 1 {
        return Err(WordNetError::SenseIDXNotValid(synset_id.to_owned(), idx, n_entries + 1));
    } else if idx == n_entries + 1 {
        idx
    } else {
        for sense_id in sense_ids_for_synset(wn, synset_id) {
            let this_idx = sense_id[sense_id.len()-2..sense_id.len()].parse::<i32>()
                .map_err(|_| WordNetError::BadSenseKey(sense_id))?;
            if this_idx > idx {
                //change_sense_id(wn, sense_id, this_idx + 1);
            }
        }
        idx
    };

    let n = if n < 0 {
        n_senses
    } else if n > n_senses {
        return Err(WordNetError::SynsetNotFound(synset_id.to_owned()));
    } else if n == n_senses {
        n
    } else {
        let mut sense_n = 0;
        match entry {
            Some(ref e) => {
                for sense in e.senses.iter() {
                    if sense_n >= n {
                        //change_sense_n(wn, entry.id, sense.id, sense_n + 1);
                        sense_n += 1;
                    }
                }
            },
            None => {}
        };
        n
    };

    match entry {
        Some(ref entry) => {
            let sense = Sense::new(
            format!("ewn-{}-{}-{}-{:02}", escape_lemma(&lemma), 
                synset_part_of_speech.value(),  synset_key(synset_id), 
                idx),
            synset_id.clone(),
            None,
            n,
            None);
            wn.add_entry_sense(&entry.id, sense);
            Ok(entry.id.clone())
        },
        None => {
            let mut entry = LexicalEntry::new(
                format!("ewn-{}-{}", escape_lemma(&lemma),
                    synset_part_of_speech.value()),
                Lemma::new(lemma.clone(), synset_part_of_speech.clone()));
            entry.add_sense(Sense::new(
                format!("ewn-{}-{}-{}-{:02}",
                    escape_lemma(&lemma), synset_part_of_speech.value(),
                    synset_key(synset_id), idx),
                synset_id.clone(), None, n, None));
            let entry_id = entry.id.clone();
            wn.add_entry(entry);
            Ok(entry_id)
        }
    }
}
//
//def delete_entry(wn, synset, entry_id):
//    """Delete a lemma from a synset"""
//    print("Deleting %s from synset %s" % (entry_id, synset.id))
//    n_entries = len(wn.members_by_id(synset.id))
//    entry_global = wn.entry_by_id(entry_id)
//    
//    if entry_global:
//        idxs = [int(sense.id[-2:]) for sense in entry_global.senses if sense.synset == synset.id]
//        if not idxs:
//            print("Entry not in synset")
//            return
//        idx = idxs[0]
//        n_senses = len(entry_global.senses)
//    else:
//        print("No entry for this lemma")
//        return
//
//    if n_senses != 1:
//        n = [ind for ind, sense in enumerate(entry_global.senses) if sense.synset == synset.id][0]
//        sense_n = 0
//        for sense in entry_global.senses:
//            if sense_n >= n:
//                change_sense_n(wn, entry_global, sense.id, sense_n - 1)
//            sense_n += 1
//
//    for sense_id in sense_ids_for_synset(wn, synset):
//        this_idx = int(sense_id[-2:])
//        if this_idx >= idx:
//            change_sense_idx(wn, sense_id, this_idx - 1)
//
//    for sense in entry_global.senses:
//        if sense.synset == synset.id:
//            for rel in sense.sense_relations:
//                delete_sense_rel(wn, rel.target, sense.id)
//
//    if n_senses == 1: # then delete the whole entry
//        wn_synset = wn
//        wn_synset.entries = [entry for entry in wn_synset.entries if entry.id != entry_global.id]
//        wn.entries = [entry for entry in wn.entries if entry.id != entry_global.id]
//    else:
//        wn_synset = wn
//        entry = wn_synset.entry_by_id(entry_global.id)
//        entry.senses = [sense for sense in entry.senses if sense.synset != synset.id]
//        entry_global.senses = [sense for sense in entry_global.senses if sense.synset != synset.id]
//
//def delete_synset(wn, synset, supersede, reason, delent=True):
//    """Delete a synset"""
//    print("Deleting synset %s" % synset.id)
//    
//    if delent:
//        entries = empty_if_none(wn.members_by_id(synset.id))
//
//        for entry in entries:
//            delete_entry(wn, synset, 
//                    "ewn-%s-%s" % (escape_lemma(entry), synset.part_of_speech.value))
//
//    for rel in synset.synset_relations:
//        delete_rel(wn.synset_by_id(rel.target), synset)
//
//    wn_synset = wn
//    wn_synset.synsets = [ss for ss in wn_synset.synsets
//            if synset.id != ss.id]
//    if supersede:
//        if not isinstance(supersede, list):
//            supersede = [supersede]
//    else:
//        supersede = []
//    with open("src/deprecations.csv", "a") as out:
//        out.write("\"%s\",\"%s\",\"%s\",\"%s\",\"%s\"\n" %
//                (synset.id, synset.ili,
//                    ",".join(s.id for s in supersede),
//                    ",".join(s.ili for s in supersede),
//                    reason.replace("\n","").replace("\"","\"\"")))
//
//
//def change_sense_n(wn, entry, sense_id, new_n):
//    """Change the position of a sense within an entry (changes only this sense)"""
//    print("Changing n of sense %s of %s to %s" % (sense_id, entry.lemma.written_form, new_n))
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
//
//def change_sense_idx(wn, sense_id, new_idx):
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
//                    new_sense_id if s == sense_id else s
//                    for s in sb.senses]
//
fn sense_ids_for_synset(wn : &Lexicon, synset_id : &String) -> Vec<String> {
    panic!("TODO")
}
//def sense_ids_for_synset(wn, synset):
//    return [sense.id for lemma in wn.members_by_id(synset.id)
//            for entry in wn.entry_by_lemma(lemma)
//            for sense in wn.entry_by_id(entry).senses
//            if sense.synset == synset.id]
//
//def new_id(wn, pos, definition):
//    s = hashlib.sha256()
//    s.update(definition.encode())
//    nid = "ewn-8%07d-%s" % ((int(s.hexdigest(),16) % 10000000), pos)
//    if wn.synset_by_id(nid):
//        print("Could not find ID for new synset. Either a duplicate definition or a hash collision for " + nid + ". Note it is possible to force a synset ID by giving it as an argument")
//        sys.exit(-1)
//    return nid
//
//
//def add_synset(wn, definition, lexfile, pos, ssid=None):
//    if not ssid:
//        ssid = new_id(wn, pos, definition)
//    ss = Synset(ssid, "in",
//            PartOfSpeech(pos), lexfile)
//    ss.definitions = [Definition(definition)]
//    wn.add_synset(ss)
//    return ssid
//
//def merge_synset(wn, synsets, reason, lexfile, ssid=None):
//    """Create a new synset merging all the facts from other synsets"""
//    pos = synsets[0].part_of_speech.value
//    if not ssid:
//        ssid = new_id(wn, pos, synsets[0].definitions[0].text)
//    ss = Synset(ssid, "in",
//            PartOfSpeech(pos), lexfile)
//    ss.definitions = [d for s in synsets for d in s.definitions]
//    ss.examples = [x for s in synsets for x in s.examples]
//    members = {}
//    wn.add_synset(ss)
//
//    for s in synsets:
//        # Add all relations
//        for r in s.synset_relations:
//            if not any(r == r2 for r2 in ss.synset_relations):
//                add_relation(wn, ss, wn.synset_by_id(r.target), r.rel_type)
//        # Add members
//        for m in wn.members_by_id(s.id):
//            if m not in members:
//                members[m] = add_entry(wn, ss, m)
//                add_entry(wn, ss, m)
//            e = [e for e in [wn.entry_by_id(e2) for e2 in wn.entry_by_lemma(m)]
//                    if e.lemma.part_of_speech.value == pos][0]
//            for f in e.forms:
//                if not any(f2 == f for f in members[m].forms):
//                    members[m].add_form(f)
//            # syn behaviours - probably fix manually for the moment
//    return ss
//
//
//def find_type(source, target):
//    """Get the first relation type between the synsets"""
//    x = [r for r in source.synset_relations if r.target == target.id]
//    if len(x) != 1:
//        raise Exception("Synsets not linked or linked by more than one property")
//    return x[0].rel_type
//
//def update_source(wn, old_source, target, new_source):
//    """Change the source of a link"""
//    rel_type = find_type(old_source, target)
//    delete_rel(old_source, target)
//    insert_rel(new_source, rel_type, target)
//    if rel_type in wordnet.inverse_synset_rels:
//        inv_rel_type = wordnet.inverse_synset_rels[rel_type]
//        delete_rel(target, old_source)
//        insert_rel(target, inv_rel_type, new_source)
//
//def update_target(wn, source, old_target, new_target):
//    """Change the target of a link"""
//    rel_type = find_type(source, old_target)
//    delete_rel(source, old_target)
//    insert_rel(source, rel_type, new_target)
//    if rel_type in wordnet.inverse_synset_rels:
//        inv_rel_type = wordnet.inverse_synset_rels[rel_type]
//        delete_rel(old_target, source)
//        insert_rel(new_target, inv_rel_type, source)
//
//def update_relation(wn, source, target, new_rel):
//    """Change the type of a link"""
//    delete_rel(source, target)
//    insert_rel(source, new_rel, target)
//    if new_rel in inverse_synset_rels:
//        inv_rel_type = inverse_synset_rels[new_rel]
//        delete_rel(target, source)
//        insert_rel(target, inv_rel_type, source)
//
//def add_relation(wn, source, target, new_rel):
//    """Change the type of a link"""
//    insert_rel(source, new_rel, target)
//    if new_rel in inverse_synset_rels:
//        inv_rel_type = inverse_synset_rels[new_rel]
//        insert_rel(target, inv_rel_type, source)
//
//def delete_relation(wn, source, target):
//    """Change the type of a link"""
//    delete_rel(source, target)
//    delete_rel(target, source)
//
//def reverse_rel(wn, source, target):
//    """Reverse the direction of relations"""
//    rel_type = find_type(source, target)
//    delete_rel(source, target)
//    if rel_type in inverse_synset_rels:
//        delete_rel(target, source)
//    insert_rel(target, rel_type, source)
//    if rel_type in inverse_synset_rels:
//        inv_rel_type = inverse_synset_rels[rel_type]
//        insert_rel(source, inv_rel_type, target)
//
//def delete_sense_rel(wn, source, target):
//    """Delete all relationships between two senses"""
//    print("Delete %s =*=> %s" % (source, target))
//    (source_synset, source_entry) = decompose_sense_id(source)
//    lex_name = wn.synset_by_id(source_synset).lex_name
//    wn_source = wn
//    entry = wn_source.entry_by_id(source_entry)
//    sense = [sense for sense in entry.senses if sense.id == source][0]
//    sense.sense_relations = [r for r in sense.sense_relations if r.target != target]
//
//def insert_sense_rel(wn, source, rel_type, target):
//    """Insert a single relation between two senses"""
//    print("Insert %s =%s=> %s" % (source, rel_type, target))
//    (source_synset, source_entry) = decompose_sense_id(source)
//    lex_name = wn.synset_by_id(source_synset).lex_name
//    wn_source = wn
//    entry = wn_source.entry_by_id(source_entry)
//    sense = [sense for sense in entry.senses if sense.id == source][0]
//    sense.sense_relations.append(SenseRelation(target, rel_type))
//
//    
//def find_sense_type(wn, source, target):
//    """Get the first relation type between the senses"""
//    (source_synset, source_entry) = decompose_sense_id(source)
//    entry = wn.entry_by_id(source_entry)
//    sense = [sense for sense in entry.senses if sense.id == source][0]
//    x = set([r for r in sense.sense_relations if r.target == target])
//    if len(x) == 0:
//        raise Exception("Synsets not linked or linked by more than one property")
//    return next(iter(x)).rel_type
//    
//
//def update_source_sense(wn, old_source, target, new_source):
//    """Change the source of a link"""
//    rel_type = find_sense_type(wn, old_source, target)
//    delete_sense_rel(wn, old_source, target)
//    insert_sense_rel(wn, new_source, rel_type, target)
//    if rel_type in inverse_sense_rels:
//        inv_rel_type = inverse_sense_rels[rel_type]
//        delete_sense_rel(wn, target, old_source)
//        insert_sense_rel(wn, target, inv_rel_type, new_source)
//
//def update_target_sense(wn, source, old_target, new_target):
//    """Change the target of a link"""
//    rel_type = find_sense_type(wn, source, old_target)
//    delete_sense_rel(wn, source, old_target)
//    insert_sense_rel(wn, source, rel_type, new_target)
//    if rel_type in inverse_sense_rels:
//        inv_rel_type = inverse_sense_rels[rel_type]
//        delete_sense_rel(wn, old_target, source)
//        insert_sense_rel(wn, new_target, inv_rel_type, source)
//
//def update_sense_relation(wn, source, target, new_rel):
//    """Change the type of a link"""
//    delete_sense_rel(wn, source, target)
//    insert_sense_rel(wn, source, new_rel, target)
//    if new_rel in inverse_sense_rels:
//        inv_rel_type = inverse_sense_rels[new_rel]
//        delete_sense_rel(wn, target, source)
//        insert_sense_rel(wn, target, inv_rel_type, source)
//
//def add_sense_relation(wn, source, target, new_rel):
//    """Change the type of a link"""
//    insert_sense_rel(wn, source, new_rel, target)
//    if new_rel in inverse_sense_rels:
//        inv_rel_type = inverse_sense_rels[new_rel]
//        insert_sense_rel(wn, target, inv_rel_type, source)
//
//def delete_sense_relation(wn, source, target):
//    """Change the type of a link"""
//    delete_sense_rel(wn, source, target)
//    delete_sense_rel(wn, target, source)
//
//def reverse_sense_rel(wn, source, target):
//    """Reverse the direction of a sense relation"""
//    rel_type = find_sense_type(wn, source, target)
//    delete_sense_rel(wn, source, target)
//    if rel_type in inverse_sense_rels:
//        delete_sense_rel(wn, target, source)
//    insert_sense_rel(wn, target, rel_type, source)
//    if rel_type in inverse_sense_rels:
//        inv_rel_type = inverse_sense_rels[rel_type]
//        insert_sense_rel(wn, source, inv_rel_type, target)
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
//def update_def(wn, synset, defn, add):
//    wn_synset = wn
//    ss = wn_synset.synset_by_id(synset.id)
//    if add:
//        ss.definitions = ss.definitions + [Definition(defn)]
//    else:
//        ss.definitions = [Definition(defn)]
//
//def update_ili_def(wn, synset, defn):
//    wn_synset = wn
//    ss = wn_synset.synset_by_id(synset.id)
//    ss.ili_definition = Definition(defn)
//
//def add_ex(wn, synset, example):
//    wn_synset = wn
//    ss = wn_synset.synset_by_id(synset.id)
//    ss.examples = ss.examples + [Example(example)]
//
//
//def delete_ex(wn, synset, example):
//    wn_synset = wn
//    ss = wn_synset.synset_by_id(synset.id)
//    n_exs = len(ss.examples)
//    ss.examples = [ex for ex in ss.examples if ex.text != example]
//    if len(ss.examples) == n_exs:
//        print("No change")
//
