extern crate lazy_static;
extern crate serde;
extern crate serde_yaml;
extern crate indicatif;
extern crate regex;
extern crate csv;

//mod wordnet;
mod rels;
mod change_manager;
mod wordnet_yaml;
mod sense_keys;

use crate::wordnet_yaml::{Lexicon,SynsetId,Synset,Sense,PosKey};
use std::io;
use std::io::Write;
use crate::change_manager::{ChangeList};
use lazy_static::lazy_static;
use regex::Regex;

//import change_manager
//from change_manager import ChangeList
//from autocorrect import Speller
//import wordnet
//import re

//#####################################
//# English WordNet Editor (EWE)
//
//
fn enter_synset<'a>(wn : &'a Lexicon, spec_string : &str) -> (SynsetId, &'a Synset) {
    let mut synset = None;
    while synset.is_none() {
        let synset_id = input(&format!("Enter {}synset ID : ewn-", spec_string));
        while synset_id == "" {
            let lemma = input("Search by lemma: ");
            let entries = wn.entry_by_lemma(&lemma);
            if !entries.is_empty() {
                let senses : Vec<&Sense> = 
                    entries.iter().flat_map(|entry|
                                entry.sense.iter()).collect();
                println!("0. Search again");
                for (i, sense) in senses.iter().enumerate() {
                    let ss = wn.synset_by_id(&sense.synset);
                    match ss {
                        Some(ss) => {
                            let ex_text = if ss.example.is_empty() {
                                String::new()
                            } else {
                                "(".to_owned() + &ss.example.iter().map(
                                        |ex| ex.text.clone()).collect::<Vec<String>>().
                                    join("; ") + ")"
                            };
                            println!("{}. {} - {} {}", i + 1, 
                                     sense.synset.as_str(), ss.definition[0], 
                                     ex_text);
                        },
                        None => {}
                    }
                }
                match input("Enter synset no: ").parse::<usize>() {
                    Ok(synset_no) => {
                        if synset_no > 0 && synset_no <= senses.len() {
                            let ssid = senses[synset_no - 1].synset.clone();
                            match wn.synset_by_id(&ssid) {
                                Some(ss) => return (ssid, ss),
                                None => {}
                            }
                        }
                    },
                    Err(_) => {
                        println!("Invalid input");
                    }
                }
            } else {
                println!("Not found");
            }
        }
        
        let ssid = SynsetId::new(&synset_id);
        match wn.synset_by_id(&ssid) {
            Some(ss) => {
                synset = Some((ssid, ss));
            },
            None => {}
        }
        if synset.is_none() {
            println!("Synset not found");
        }
    }
    synset.unwrap()
}

//def enter_sense_synset(wordnet, spec_string="", synset_id=None):
//    '''Handle the user input of a single synset or sense'''
//    if not synset_id:
//        synset = enter_synset(wordnet, spec_string)
//    else:
//        synset = wordnet.synset_by_id(synset_id)
//    if not synset:
//        print("Synset not found")
//    print("0. Synset (No sense)")
//    mems = wordnet.members_by_id(synset.id)
//    for i, m in enumerate(mems):
//        print("%d. %s" % (i + 1, m))
//    sense_no = input("Enter sense number: ")
//    sense_id = None
//    if sense_no >= '1' and sense_no <= str(len(mems)):
//        lemma = mems[int(sense_no) - 1]
//        sense_id = [sense.id for entry_id in wordnet.entry_by_lemma(lemma)
//                    for sense in wordnet.entry_by_id(entry_id).senses
//                    if sense.synset == synset.id][0]
//    return synset.id, sense_id
//
//
//def enter_sense(wordnet, synset_id, spec_string=""):
//    '''Handle the user input of a single synset or sense'''
//    print("0. Synset (No sense)")
//    mems = wordnet.members_by_id(synset_id)
//    for i, m in enumerate(mems):
//        print("%d. %s" % (i + 1, m))
//    sense_no = input("Enter sense number: ")
//    sense_id = None
//    if sense_no >= '1' and sense_no <= str(len(mems)):
//        lemma = mems[int(sense_no) - 1]
//        sense_id = [sense.id for entry_id in wordnet.entry_by_lemma(lemma)
//                    for sense in wordnet.entry_by_id(entry_id).senses
//                    if sense.synset == synset_id][0]
//    return sense_id
//
//
//
//spell = Speller(lang='en')
//
//
fn check_text(defn : &str) -> bool {
    if defn == "" {
        println!("Defintion is empty");
//    if any([spell(w) != w for w in defn.split()]):
//        return input(
//            "There may be spelling errors in this %s. Proceed [y/N] : " %
//            text) == "y"
        false
    } else {
        true
    }
}

fn change_entry(wn : &mut Lexicon, change_list : &mut ChangeList) {
    let mut action = input("[A]dd/[D]elete/[M]ove> ").to_uppercase();
    while action != "A" && action != "D" && action != "M" {
        println!("Bad action");
        action = input("[A]dd/[D]elete/[M]ove> ").to_uppercase();
    }

    let (synset_id, synset) = enter_synset(wn, "");

    let entries = wn.members_by_id(&synset_id);

    if !entries.is_empty() {
        println!("Entries: {}", entries.join(", "));
    } else {
        println!("No entries");
    }

    let lemma = if action == "A" {
        input("New entry: ")
    } else if action == "D" {
        input("Entry to remove: ")
    } else /* action == "M" */ {
        input("Entry to move: ")
    };

    if action == "A" {
        let pos = synset.part_of_speech.to_pos_key();
        change_manager::add_entry(wn, synset_id, 
                                  lemma, pos, change_list); 
    } else if action == "D" {
        match wn.pos_for_entry_synset(&lemma, &synset_id) {
            Some(pos) => {
                change_manager::delete_entry(wn, &synset_id, &lemma, &pos, true, change_list);
            },
            None => {
                println!("Could not find entry, skipping change")
            }
        }
    } else if action == "M" {
        let (target_synset_id, target_synset) = enter_synset(wn, "target ");
        match wn.pos_for_entry_synset(&lemma, &synset_id) {
            Some(pos) => {
                if synset.part_of_speech == target_synset.part_of_speech {
                    change_manager::move_entry(wn, synset_id, target_synset_id,
                                               lemma, pos, change_list);
                } else {
                    println!("Different part of speech, skipping this change");
                }
            },
            None => {
                println!("Could not find entry, skipping change")
            }
        }
    }
}

lazy_static! {
    static ref REASON_REGEX : Regex = Regex::new("\\w+.*\\(#\\d+\\)$").unwrap();
}

fn change_synset(wn : &mut Lexicon, change_list : &mut ChangeList) {
    let mut mode = String::new();
    while mode != "a" && mode != "d" {
        mode = input("(A)dd synset/(d)elete synset: ").to_lowercase();
    }

    if mode == "d" {
        let (synset_id, _) = enter_synset(wn, "");
        let mut reason = input("Reason for deletion with (#IssueNo): ");
        while !REASON_REGEX.is_match(&reason) {
            println!("Bad reason please state at least one word with the issue number in parentheses, e.g., duplicate (#123)");
            reason = input("Reason for deletion with (#IssueNo): ");
        }
        let (supersede_synset_id, _) = 
            enter_synset(wn, "superseding ");
        change_manager::delete_synset(wn, 
                                      &synset_id, Some(&supersede_synset_id),
                                      reason, true, change_list);
    } else /*if mode == "a"*/ {
        let definition = input("Definition: ");
        let lexfile = input("Lexicographer file: ");
        let pos = PosKey::new(input(
            "Part of speech (n)oun/(v)erb/(a)djective/adve(r)b/(s)atellite: ")
            .to_lowercase());
        let new_id = change_manager::add_synset(
            wn, definition, lexfile, pos.clone(), None, change_list);
        loop {
            let lemma = input("Add Lemma (blank to stop): ");
            if lemma.len() > 0 {
                change_manager::add_entry(wn, new_id.clone(),
                    lemma, pos.clone(), change_list);
            } else {
                break;
            }
        }
        println!("New synset created with ID {}. Add at least one relation:",
                 new_id.as_str());
        // TODO: change_relation
//        change_relation(wn, change_list, new_id)
    }
}

fn change_definition(wn : &mut Lexicon, change_list : &mut ChangeList) {
    let (synset_id, synset) = enter_synset(wn, "");

    println!("Definition     : {}", synset.definition[0]);
    let mut defn;
    loop { 
        defn = input("New Definition : ");
        if check_text(&defn) {
            break;
        }
    }
    change_manager::update_def(
        wn, &synset_id, defn, false);

    change_list.mark();
}
//def change_definition(wn, change_list):
//    synset = enter_synset(wn)
//
//    ili = input("Set ILI Definition (y/N)? ")
//
//    if ili == "y":
//        change_manager.update_ili_def(wn, synset, synset.definitions[0].text)
//    else:
//        print("Definition     : " + synset.definitions[0].text)
//        while True:
//            defn = input("New Definition : ").strip()
//            if check_text(defn, "definition"):
//                break
//        change_manager.update_def(
//            wn, synset, defn, False, change_list=change_list)
//    return True
//
//
fn change_example(wn : &mut Lexicon, change_list : &mut ChangeList) {}
//def change_example(wn, change_list):
//    synset = enter_synset(wn)
//
//    mode = None
//    while mode != "a" and mode != "d":
//        mode = input("[A]dd/[D]elete example: ").lower()
//
//    if mode == "a":
//        while True:
//            example = input("Example: ")
//
//            if not example.startswith("\""):
//                print("Examples must start and end with a quotation")
//                continue
//
//            if check_text(example, "example"):
//                break
//
//        change_manager.add_ex(wn, synset, example, change_list=change_list)
//    else:
//        if synset.examples:
//            for i, ex in enumerate(synset.examples):
//                print("%d. %s" % (i + 1, ex.text))
//            number = "0"
//            while not number.isdigit() or int(number) < 1 or int(
//                    number) > len(synset.examples):
//                number = input("Example Number> ")
//            example = synset.examples[int(number) - 1].text
//        change_manager.delete_ex(wn, synset, example, change_list=change_list)
//    return True
//
//
fn change_relation(wn : &mut Lexicon, change_list : &mut ChangeList) {}
//def change_relation(wn, change_list, source_id=None):
//    mode = None
//    new_source = None
//    new_target = None
//    new_relation = None
//    delete = False
//    reverse = False
//    add = False
//    delete = False
//    if source_id:
//        source_entry_id = None
//        mode = "a"
//        add = True
//        new_relation = input("Enter new relation: ")
//
//    while mode != "a" and mode != "d" and mode != "r" and mode != "c":
//        mode = input("[A]dd new relation/[D]elete existing relation/" +
//                     "[R]everse relation/[C]hange relation: ").lower()
//        if mode == "a":
//            add = True
//            new_relation = input("Enter new relation: ")
//        elif mode == "c":
//            mode = input("Change [S]ubject/[T]arget/[R]elation: ").lower()
//            if mode == "s":
//                new_source, new_source_sense_id = enter_sense_synset(
//                    wn, "new source ")
//            elif mode == "t":
//                new_target, new_target_sense_id = enter_sense_synset(
//                    wn, "new target ")
//            elif mode == "r":
//                new_relation = input("Enter new relation: ")
//            else:
//                print("Bad choice")
//                return False
//        elif mode == "d":
//            delete = True
//        elif mode == "r":
//            reverse = True
//
//    if not source_id:
//        if (new_relation and
//                new_relation not in wordnet.SenseRelType._value2member_map_):
//            source_id = enter_synset(wn, "source ").id
//            source_sense_id = None
//        elif new_source and new_source_sense_id:
//            source_id, source_sense_id = enter_sense_synset(wn, "old source ")
//        elif new_source:
//            source_id = enter_synset(wn, "old source ").id
//            source_sense_id = None
//        else:
//            source_id, source_sense_id = enter_sense_synset(wn, "source ")
//    else:
//        source_sense_id = None
//
//    if (new_relation and
//            new_relation not in wordnet.SenseRelType._value2member_map_):
//        target_id = enter_synset(wn, "target ").id
//        target_sense_id = None
//    elif new_target and new_target_sense_id:
//        target_id, target_sense_id = enter_sense_synset(wn, "old target ")
//    elif new_target:
//        target_id = enter_synset(wn, "old target ").id
//        target_sense_id = None
//    else:
//        target_id, target_sense_id = enter_sense_synset(wn, "target ")
//        if not source_sense_id: # Occurs when creating a new entry
//            source_id, source_sense_id = enter_sense_synset(wn, "source ", source_id)
//
//    source_synset = wn.synset_by_id(source_id)
//    if not source_synset:
//        print("Could not find source synset " + source_id)
//        return False
//    target_synset = wn.synset_by_id(target_id)
//    if not target_synset:
//        print("Could not find target synset " + target_id)
//        return False
//
//    if new_source:
//        if source_sense_id or target_sense_id:
//            if not change_manager.sense_exists(wn, source_sense_id):
//                print("Source sense %d does not exist" % source_sense_id)
//                return False
//            if not change_manager.sense_exists(wn, target_sense_id):
//                print("Target sense %d does not exist" % target_sense_id)
//                return False
//            if not change_manager.sense_exists(wn, new_source_sense_id):
//                print("New source sense %d does not exist" %
//                      new_source_sense_id)
//                return False
//            change_manager.update_source_sense(wn, source_sense_id,
//                                               target_sense_id,
//                                               new_source_sense_id,
//                                               change_list=change_list)
//        else:
//            new_source = wn.synset_by_id(new_source)
//
//            if not new_source:
//                print("Could not find the new source synset %s" % new_source)
//                return False
//
//            change_manager.update_source(wn, source_synset, target_synset,
//                                         new_source, change_list=change_list)
//
//    elif new_target:
//        if source_sense_id or target_sense_id:
//            if not change_manager.sense_exists(wn, source_sense_id):
//                print("Source sense %d does not exist" % source_sense_id)
//                return False
//            if not change_manager.sense_exists(wn, target_sense_id):
//                print("Target sense %d does not exist" % target_sense_id)
//                return False
//            if not change_manager.sense_exists(wn, new_target_sense_id):
//                print("New target sense %d does not exist" %
//                      new_target_sense_id)
//                return False
//            change_manager.update_target_sense(wn, source_sense_id,
//                                               target_sense_id,
//                                               new_target_sense_id,
//                                               change_list=change_list)
//        else:
//            new_target = wn.synset_by_id(new_target)
//
//            if not new_target:
//                print("Could not find the new target synset %s" % new_target)
//                return False
//
//            change_manager.update_target(wn, source_synset, target_synset,
//                                         new_target, change_list=change_list)
//
//    elif new_relation:
//        if source_sense_id:
//            if new_relation not in wordnet.SenseRelType._value2member_map_:
//                print("Not a valid relation type %s" % new_relation)
//                return False
//        else:
//            if new_relation not in wordnet.SynsetRelType._value2member_map_:
//                print("Not a valid relation type %s" % new_relation)
//                return False
//
//        if add:
//            if target_sense_id and not source_sense_id:
//                source_sense_id = enter_sense(wn, source_id)
//            if source_sense_id or target_sense_id:
//                if not change_manager.sense_exists(wn, source_sense_id):
//                    print("Source sense %s does not exist" % source_sense_id)
//                    return False
//                if not change_manager.sense_exists(wn, target_sense_id):
//                    print("Target sense %s does not exist" % target_sense_id)
//                    return False
//                change_manager.add_sense_relation(wn, source_sense_id,
//                                                  target_sense_id,
//                                                  wordnet.SenseRelType(
//                                                      new_relation),
//                                                  change_list=change_list)
//            else:
//                change_manager.add_relation(wn, source_synset, target_synset,
//                                            wordnet.SynsetRelType(
//                                                new_relation),
//                                            change_list=change_list)
//        elif delete:
//            if source_entry_id or target_entry_id:
//                if not change_manager.sense_exists(wn, source_id):
//                    print("Source sense %s does not exist" % source_id)
//                    return False
//                if not change_manager.sense_exists(wn, target_id):
//                    print("Target sense %s does not exist" % target_id)
//                    return False
//                change_manager.delete_sense_relation(wn, source_id, target_id,
//                                                     change_list=change_list)
//            else:
//                change_manager.delete_relation(wn, source_synset,
//                                               target_synset,
//                                               change_list=change_list)
//        else:
//            if source_sense_id or target_sense_id:
//                if not change_manager.sense_exists(wn, source_sense_id):
//                    print("Source sense %s does not exist" % source_sense_id)
//                    return False
//                if not change_manager.sense_exists(wn, target_sense_id):
//                    print("Target sense %s does not exist" % target_sense_id)
//                    return False
//                change_manager.update_sense_relation(wn, source_sense_id,
//                                                     target_sense_id,
//                                                     wordnet.SenseRelType(
//                                                         new_relation),
//                                                     change_list=change_list)
//            else:
//                change_manager.update_relation(wn, source_synset,
//                                               target_synset,
//                                               wordnet.SynsetRelType(
//                                                   new_relation),
//                                               change_list=change_list)
//    elif delete:
//        if source_sense_id or target_sense_id:
//            if not change_manager.sense_exists(wn, source_sense_id):
//                print("Source sense %s does not exist" % source_sense_id)
//                return False
//            if not change_manager.sense_exists(wn, target_sense_id):
//                print("Target sense %s does not exist" % target_sense_id)
//                return False
//            change_manager.delete_sense_relation(wn, source_sense_id,
//                                                 target_sense_id,
//                                                 change_list=change_list)
//        else:
//            change_manager.delete_relation(wn, source_synset, target_synset,
//                                           change_list=change_list)
//    elif reverse:
//        if source_entry_id or target_entry_id:
//            if not change_manager.sense_exists(wn, source_id):
//                print("Source sense %s does not exist" % source_id)
//                return False
//            if not change_manager.sense_exists(wn, target_id):
//                print("Target sense %s does not exist" % target_id)
//                return False
//            change_manager.reverse_sense_rel(wn, source_id, target_id,
//                                             change_list=change_list)
//        else:
//            change_manager.reverse_rel(wn, source_synset, target_synset,
//                                       change_list=change_list)
//
//    else:
//        print("No change specified")
//    return True
//
//
fn split_synset(wn : &mut Lexicon, change_list : &mut ChangeList) {}
//def split_synset(wn, change_list):
//    synset = enter_synset(wn)
//
//    definition = []
//    print("Enter definitions (empty line to finish)")
//    while True:
//        d1 = input("Definition: ")
//        if d1:
//            definition.append(d1)
//        else:
//            break
//
//    reason = input("Reason for deletion (#IssueNo): ")
//
//    new_ids = []
//    for definition in definition:
//        new_ids.append(change_manager.add_synset(wn, definition,
//                                                 synset.lex_name,
//                                                 synset.part_of_speech.value,
//                                                 change_list=change_list))
//
//    change_manager.delete_synset(wn, synset,
//                                 [wn.synset_by_id(new_id)
//                                  for new_id in new_ids],
//                                 reason, change_list=change_list)
//    return True
//
//
//ewe_changed = False
//change_list = ChangeList()


fn save(wn : &Lexicon) -> std::io::Result<()> {
    wn.save("/home/jmccrae/projects/globalwordnet/english-wordnet/src/yaml/")
}

fn input(prompt : &str) -> String {
    io::stdout().lock().write_all(prompt.as_bytes()).expect("Cannot write to STDOUT");
    io::stdout().flush().expect("Cannot flush STDOUT");
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer).expect("Cannot read from STDIN");
    buffer.trim().to_string()
}

fn main_menu(wn : &mut Lexicon, ewe_changed : &mut ChangeList) -> bool {
    println!("Please choose an option:");
    println!("1. Add/delete/move entry");
    println!("2. Add/delete a synset");
    println!("3. Change a definition");
    println!("4. Change an example");
    println!("5. Change a relation");
    println!("6. Split a synset");
    if ewe_changed.changed() {
        println!("7. Save changes");
    }
    println!("X. Exit EWE");


    let mut mode = input("Option> ");
    match mode.to_lowercase().as_str() {
        "1" => change_entry(wn, ewe_changed),
        "2" => change_synset(wn, ewe_changed),
        "3" => change_definition(wn, ewe_changed),
        "4" => change_example(wn, ewe_changed),
        "5" => change_relation(wn, ewe_changed),
        "6" => split_synset(wn, ewe_changed),
        "7" => {
            save(wn).expect("Could not save");
            ewe_changed.reset();
        },
        "x" => {
            if ewe_changed.changed() {
                if input("Save changes (Y/n)? ").to_lowercase() != "n" {
                    save(wn).expect("Could not save");
                    ewe_changed.reset();
                }
            }
            return false;
        },
        _ => println!("Please enter a valid option")
    }
    true
}


fn main() {
    println!("");
    println!("         ,ww                             ");
    println!("   wWWWWWWW_)  Welcome to EWE            ");
    println!("   `WWWWWW'    - English WordNet Editor  ");
    println!("    II  II                               ");
    println!("");

    let mut wn = wordnet_yaml::Lexicon::load("/home/jmccrae/projects/globalwordnet/english-wordnet/src/yaml/").unwrap();

    let mut ewe_changed = ChangeList::new();

    while main_menu(&mut wn, &mut ewe_changed) {}
}
