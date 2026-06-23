extern crate indicatif;
extern crate lazy_static;
extern crate regex;
extern crate serde_yaml;

mod indicatif_progress;

use clap::{Parser, Subcommand};
use indicatif_progress::IndicatifProgress;
use lazy_static::lazy_static;
use oewn_lib::change_manager;
use oewn_lib::change_manager::ChangeList;
use oewn_lib::progress::NullProgress;
use oewn_lib::rels::{SenseRelType, SynsetRelType};
use oewn_lib::validate::{fix, validate};
use oewn_lib::wordnet::{Lexicon, LexiconHashMapBackend, PosKey, Sense, SenseId, Synset, SynsetId};
use regex::Regex;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::exit;
use std::result;

/// Supports the user in choosing a synset
fn enter_synset<'a, L: Lexicon>(wn: &'a L, spec_string: &str) -> (SynsetId, Cow<'a, Synset>) {
    loop {
        let input_str = input(&format!("Enter {}synset: ", spec_string));
        let ssid = SynsetId::new(&input_str);
        match wn.synset_by_id(&ssid).expect("Cannot read wordnet") {
            Some(ss) => {
                return (ssid, ss);
            }
            None => {
                let entries = wn.entry_by_lemma(&input_str).expect("Cannot read wordnet");
                if !entries.is_empty() {
                    let senses: Vec<&Sense> = entries
                        .iter()
                        .flat_map(|entry| entry.sense.iter())
                        .collect();
                    if senses.len() == 1 {
                        let ssid = senses[0].synset.clone();
                        match wn.synset_by_id(&ssid).expect("Cannot read wordnet") {
                            Some(ss) => return (ssid, ss),
                            None => {}
                        }
                    } else {
                        println!("0. Search again");
                        for (i, sense) in senses.iter().enumerate() {
                            let ss = wn.synset_by_id(&sense.synset).expect("Cannot read wordnet");
                            match ss {
                                Some(ss) => {
                                    let ex_text = if ss.example.is_empty() {
                                        String::new()
                                    } else {
                                        "(".to_owned()
                                            + &ss
                                                .example
                                                .iter()
                                                .map(|ex| ex.text.clone())
                                                .collect::<Vec<String>>()
                                                .join("; ")
                                            + ")"
                                    };
                                    println!(
                                        "{}. {} - {} {}",
                                        i + 1,
                                        sense.synset.as_str(),
                                        ss.definition[0],
                                        ex_text
                                    );
                                }
                                None => {}
                            }
                        }
                        match input("Enter synset no: ").parse::<usize>() {
                            Ok(synset_no) => {
                                if synset_no > 0 && synset_no <= senses.len() {
                                    let ssid = senses[synset_no - 1].synset.clone();
                                    match wn.synset_by_id(&ssid).expect("Cannot read wordnet") {
                                        Some(ss) => return (ssid, ss),
                                        None => {}
                                    }
                                }
                            }
                            Err(_) => {
                                println!("Invalid input");
                            }
                        }
                    }
                } else {
                    println!("No such synset or entry for '{}'", input_str);
                }
            }
        }
    }
}

fn enter_sense_synset<L: Lexicon>(
    wordnet: &L,
    spec_string: &str,
    synset_id: Option<SynsetId>,
) -> (SynsetId, Option<SenseId>) {
    let synset_id = match synset_id {
        Some(ssid) => ssid,
        None => enter_synset(wordnet, spec_string).0,
    };
    let mems = wordnet
        .members_by_id(&synset_id)
        .expect("Cannot read wordnet");
    println!("0. Synset (no sense)");
    for (i, m) in mems.iter().enumerate() {
        println!("{}. {}", i + 1, m);
    }
    let sense_no = input("Enter sense number: ");
    let sense_id = match sense_no.parse::<usize>() {
        Ok(i) => {
            if i >= 1 && i <= mems.len() {
                wordnet
                    .get_sense(&mems[i - 1], &synset_id)
                    .expect("Cannot read wordnet")
                    .iter()
                    .filter(|sense| sense.synset == synset_id)
                    .map(|sense| sense.id.clone())
                    .nth(0)
            } else {
                None
            }
        }
        Err(_) => None,
    };
    (synset_id, sense_id)
}

fn enter_sense<L: Lexicon>(wordnet: &L, spec_string: &str, allow_none: bool) -> SenseId {
    let synset_id = enter_synset(wordnet, spec_string).0;
    let mems = wordnet
        .members_by_id(&synset_id)
        .expect("Cannot read wordnet");
    loop {
        if allow_none {
            println!("0. None");
        }
        for (i, m) in mems.iter().enumerate() {
            println!("{}. {}", i + 1, m);
        }
        let sense_no = input("Enter sense number: ");
        match sense_no.parse::<usize>() {
            Ok(i) => {
                if i >= 1 && i <= mems.len() {
                    match wordnet
                        .get_sense(&mems[i - 1], &synset_id)
                        .expect("Cannot read wordnet")
                        .iter()
                        .filter(|sense| sense.synset == synset_id)
                        .map(|sense| sense.id.clone())
                        .nth(0)
                    {
                        Some(ssid) => {
                            return ssid;
                        }
                        None => {}
                    }
                } else if i == 0 {
                    return SenseId::new(synset_id.as_str().to_string());
                }
            }
            Err(_) => {}
        }
    }
}

fn check_text(defn: &str) -> bool {
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

fn change_entry<L: Lexicon>(wn: &mut L, change_list: &mut ChangeList) {
    let mut action = input("[A]dd/[D]elete/[M]ove/[C]hange> ").to_uppercase();
    while action != "A" && action != "D" && action != "M" && action != "C" {
        println!("Bad action");
        action = input("[A]dd/[D]elete/[M]ove/[C]hange> ").to_uppercase();
    }

    let (synset_id, synset) = enter_synset(wn, "");

    let entries = wn.members_by_id(&synset_id).expect("Cannot read wordnet");

    if !entries.is_empty() {
        println!("Entries: {}", entries.join(", "));
    } else {
        println!("No entries");
    }

    let lemma = if action == "A" {
        input("New entry: ")
    } else if action == "D" {
        input("Entry to remove: ")
    } else if action == "M" {
        input("Entry to move: ")
    } else
    /* action == "C" */
    {
        input("Entry to change: ")
    };

    if action == "A" {
        let pos = synset.part_of_speech.to_pos_key();
        let subcat = if pos.as_str() == "v" {
            input("Enter verb subcats as comma-separated list: ")
                .split(",")
                .map(|s| s.to_string())
                .collect()
        } else {
            Vec::new()
        };
        change_manager::add_entry(wn, synset_id, lemma, pos, subcat, None, change_list)
            .expect("Could not add entry");
    } else if action == "D" {
        match wn
            .pos_for_entry_synset(&lemma, &synset_id)
            .expect("Cannot read wordnet")
        {
            Some(pos) => {
                change_manager::delete_entry(wn, &synset_id, &lemma, &pos, true, change_list)
                    .expect("Could not delete entry");
            }
            None => {
                println!("Could not find entry, skipping change")
            }
        }
    } else if action == "M" {
        let (target_synset_id, target_synset) = enter_synset(wn, "target ");
        match wn
            .pos_for_entry_synset(&lemma, &synset_id)
            .expect("Cannot read wordnet")
        {
            Some(pos) => {
                if synset
                    .part_of_speech
                    .equals_pos(&target_synset.part_of_speech)
                {
                    change_manager::move_entry(
                        wn,
                        synset_id,
                        target_synset_id,
                        lemma,
                        pos,
                        change_list,
                    )
                    .expect("Could not move entry");
                } else {
                    println!("Different part of speech, skipping this change");
                }
            }
            None => {
                println!("Could not find entry, skipping change")
            }
        }
    } else if action == "C" {
        match wn
            .pos_for_entry_synset(&lemma, &synset_id)
            .expect("Cannot read wordnet")
        {
            Some(pos) => {
                change_manager::delete_entry(wn, &synset_id, &lemma, &pos, true, change_list)
                    .expect("Could not delete entry");
                let new_lemma = input("New lemma: ");
                let subcat = if pos.as_str() == "v" {
                    input("Enter verb subcats as comma-separated list: ")
                        .split(",")
                        .map(|s| s.to_string())
                        .collect()
                } else {
                    Vec::new()
                };
                change_manager::add_entry(wn, synset_id, new_lemma, pos, subcat, None, change_list)
                    .expect("Could not add new entry");
            }
            None => {
                println!("Could not find entry, skipping change")
            }
        }
    }
}

lazy_static! {
    static ref REASON_REGEX: Regex = Regex::new("\\w+.*\\(#\\d+\\)$").unwrap();
}

fn change_synset<L: Lexicon>(wn: &mut L, change_list: &mut ChangeList) {
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
        let (supersede_synset_id, _) = enter_synset(wn, "superseding ");

        change_manager::delete_synset(
            wn,
            &synset_id,
            Some(&supersede_synset_id),
            reason,
            change_list,
        )
        .expect("Could not delete synset");
    } else
    /*if mode == "a"*/
    {
        let definition = input("Definition: ");
        let lexfile = input("Lexicographer file: ");
        let poses = wn.pos_for_lexfile(&lexfile).expect("Cannot read wordnet");
        if poses.is_empty() {
            println!("Lexicographer file does not exist");
            return;
        }
        let pos = PosKey::new(
            input("Part of speech (n)oun/(v)erb/(a)djective/adve(r)b/(s)atellite: ").to_lowercase(),
        );
        match pos.to_part_of_speech() {
            Some(p) => {
                if !poses.iter().any(|p2| p == *p2) {
                    println!("Wrong POS for lexicographer file");
                    return;
                }
            }
            None => {
                println!("POS value not valid");
            }
        }
        match change_manager::add_synset(wn, definition, lexfile, pos.clone(), None, change_list) {
            Ok(new_id) => {
                loop {
                    let lemma = input("Add Lemma (blank to stop): ");
                    let subcat = if pos == PosKey::new("v".to_string()) {
                        input("Enter verb subcats as comma-separated list: ")
                            .split(",")
                            .map(|s| s.to_string())
                            .collect()
                    } else {
                        Vec::new()
                    };
                    if lemma.len() > 0 {
                        change_manager::add_entry(
                            wn,
                            new_id.clone(),
                            lemma,
                            pos.clone(),
                            subcat,
                            None,
                            change_list,
                        )
                        .expect("Could not add lemma");
                    } else {
                        break;
                    }
                }
                println!(
                    "New synset created with ID {}. Add at least one relation:",
                    new_id.as_str()
                );
                add_relation(wn, Some(new_id), change_list);
            }
            Err(e) => {
                println!("{}", e);
            }
        }
    }
}

fn change_definition<L: Lexicon>(wn: &mut L, change_list: &mut ChangeList) {
    let (synset_id, synset) = enter_synset(wn, "");

    println!("Definition     : {}", synset.definition[0]);
    let mut defn;
    loop {
        defn = input("New Definition : ");
        if check_text(&defn) {
            break;
        }
    }
    change_manager::update_def(wn, &synset_id, defn, false);

    change_list.mark();
}

fn change_example<L: Lexicon>(wn: &mut L, change_list: &mut ChangeList) {
    let (synset_id, synset) = enter_synset(wn, "");

    let mut mode = String::new();
    while mode != "a" && mode != "d" {
        mode = input("[A]dd/[D]elete example: ").to_lowercase();
    }
    if mode == "a" {
        loop {
            let example = input("Example: ");

            if example.starts_with("\"") {
                println!("Examples cannot start with a quotation");
                continue;
            }

            if !check_text(&example) {
                continue;
            }

            let source = input("Source (blank for no source): ");
            let source = if source == "" { None } else { Some(source) };

            change_manager::add_ex(wn, &synset_id, example, source, change_list);
            break;
        }
    } else
    /* mode == "d" */
    {
        if synset.example.is_empty() {
            println!("No examples to delete!");
        } else {
            for (i, ex) in synset.example.iter().enumerate() {
                println!("{}. {}", i + 1, ex.text);
            }
            let mut number = 0;
            while number < 1 || number > synset.example.len() {
                let n = input("Example Number: ");
                match n.parse() {
                    Ok(n) => number = n,
                    Err(_) => {
                        eprintln!("Please enter a number!");
                    }
                }
            }
            change_manager::delete_ex(wn, &synset_id, number - 1, change_list);
        }
    }
}

fn add_relation<L: Lexicon>(wn: &mut L, source_id: Option<SynsetId>, change_list: &mut ChangeList) {
    let (source_id, source_sense_id) = enter_sense_synset(wn, "source ", source_id);
    match source_sense_id {
        Some(source_sense_id) => {
            let mut relation = input("Enter new relation: ");
            while SenseRelType::from(&relation).is_none() {
                println!("Bad relation type");
                relation = input("Enter new relation: ");
            }
            let rel = SenseRelType::from(&relation).unwrap();
            let target_sense_id = enter_sense(wn, "target ", true);
            change_manager::insert_sense_relation(
                wn,
                source_sense_id,
                rel,
                target_sense_id,
                change_list,
            )
            .expect("Could not add relation");
        }
        None => {
            let mut relation = input("Enter new relation: ");
            while SynsetRelType::from(&relation).is_none() {
                println!("Bad relation type");
                relation = input("Enter new relation: ");
            }
            let rel = SynsetRelType::from(&relation).unwrap();
            let target_id = enter_synset(wn, "target ").0;
            change_manager::insert_rel(wn, &source_id, &rel, &target_id, change_list)
                .expect("Could not add relation");
        }
    }
}

fn delete_relation<L: Lexicon>(wn: &mut L, change_list: &mut ChangeList) {
    let (source_id, source_sense_id) = enter_sense_synset(wn, "source ", None);
    match source_sense_id {
        Some(source_sense_id) => {
            let target_sense_id = enter_sense(wn, "target ", false);
            change_manager::delete_sense_rel(wn, &source_sense_id, &target_sense_id, change_list)
                .expect("Could not delete relation");
        }
        None => {
            let target_id = enter_synset(wn, "target ").0;
            change_manager::delete_rel(wn, &source_id, &target_id, change_list);
        }
    }
}

fn reverse_relation<L: Lexicon>(wn: &mut L, change_list: &mut ChangeList) {
    let (source_id, source_sense_id) = enter_sense_synset(wn, "source ", None);
    match source_sense_id {
        Some(source_sense_id) => {
            let target_sense_id = enter_sense(wn, "target ", false);
            change_manager::reverse_sense_rel(wn, &source_sense_id, &target_sense_id, change_list)
                .expect("Could not reverse relation");
        }
        None => {
            let target_id = enter_synset(wn, "target ").0;
            change_manager::reverse_rel(wn, &source_id, &target_id, change_list)
                .expect("Could not reverse relation");
        }
    }
}

fn change_relation<L: Lexicon>(wn: &mut L, change_list: &mut ChangeList) {
    let mut mode = String::new();
    while mode != "a" && mode != "d" && mode != "r" && mode != "c" {
        mode = input("[A]dd new relation/[D]elete existing relation/[R]everse relation: ")
            .to_lowercase();
        if mode == "a" {
            add_relation(wn, None, change_list);
        } else if mode == "d" {
            delete_relation(wn, change_list);
        } else if mode == "r" {
            reverse_relation(wn, change_list);
        }
    }
}

fn save<L: Lexicon>(
    wn: &L,
    path: &str,
) -> result::Result<bool, oewn_lib::wordnet::LexiconSaveError> {
    let mut progress = IndicatifProgress::new();
    let errors = validate(wn, &mut progress)?;
    if !errors.is_empty() {
        println!("There were validation errors");
        for error in errors {
            println!("{}", error);
        }
        let really_save = input("Do you really want to save [y/N]? ").to_lowercase();
        if really_save == "y" {
            let mut progress = IndicatifProgress::new();
            wn.save(path, &mut progress)?;
            Ok(true)
        } else {
            Ok(false)
        }
    } else {
        let mut progress = IndicatifProgress::new();
        wn.save(path, &mut progress)?;
        Ok(true)
    }
}

fn input(prompt: &str) -> String {
    io::stdout()
        .lock()
        .write_all(prompt.as_bytes())
        .expect("Cannot write to STDOUT");
    io::stdout().flush().expect("Cannot flush STDOUT");
    let mut buffer = String::new();
    io::stdin()
        .read_line(&mut buffer)
        .expect("Cannot read from STDIN");
    buffer.trim().to_string()
}

fn main_menu<L: Lexicon>(wn: &mut L, path: &str, ewe_changed: &mut ChangeList) -> bool {
    println!("");
    println!("Please choose an option:");
    println!("1. Add/delete/move entry");
    println!("2. Add/delete a synset");
    println!("3. Change a definition");
    println!("4. Change an example");
    println!("5. Change a relation");
    println!("6. Validate");
    println!("7. Fix validation errors");
    if ewe_changed.changed() {
        println!("8. Save changes");
    }
    println!("X. Exit EWE");

    let mode = input("Option> ");
    match mode.to_lowercase().as_str() {
        "1" => change_entry(wn, ewe_changed),
        "2" => change_synset(wn, ewe_changed),
        "3" => change_definition(wn, ewe_changed),
        "4" => change_example(wn, ewe_changed),
        "5" => change_relation(wn, ewe_changed),
        "6" => {
            let mut progress = IndicatifProgress::new();
            let errors = validate(wn, &mut progress).expect("Could not complete validation");
            for error in errors.iter() {
                println!("{}", error);
            }
            if errors.is_empty() {
                println!("No validation errors!");
            } else {
                println!("{} validation errors", errors.len());
            }
        }
        "7" => {
            let mut progress = IndicatifProgress::new();
            let errors = validate(wn, &mut progress).expect("Could not complete validation");
            let mut fixed = 0;
            for error in errors.iter() {
                if fix(wn, error, ewe_changed).expect("Could not fix error") {
                    fixed += 1;
                }
            }
            println!("{}/{} validation errors fixed", fixed, errors.len());
        }
        "8" => {
            let saved = save(wn, path).expect("Could not save");
            if saved {
                ewe_changed.reset();
            }
        }
        "x" => {
            if ewe_changed.changed() {
                if input("Save changes (Y/n)? ").to_lowercase() != "n" {
                    let saved = save(wn, path).expect("Could not save");
                    if saved {
                        ewe_changed.reset();
                        return false;
                    }
                } else {
                    return false;
                }
            } else {
                return false;
            }
        }
        _ => println!("Please enter a valid option"),
    }
    true
}

#[derive(Parser, Debug)]
#[command(
    name="ewe",
    version="0.2.0",
    about="English Wordnet Editor",
    long_about=None)]
struct EweCli {
    #[command(subcommand)]
    command: Option<Command>,
    /// Optional path to the WordNet database, available in all modes
    #[arg(long, global = true, value_name = "PATH")]
    wordnet: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run an automaton script
    Automaton {
        /// The path to the script file to execute
        #[arg(short, long, value_name = "FILE")]
        script: PathBuf,
    },
    /// Search for a specific word
    Word {
        /// The word you want to search for
        word: String,

        /// Perform a case-insensitive search
        #[arg(short, long)]
        ignore_case: bool,
    },
    /// Search for an item by its unique ID
    Id {
        /// The numeric or textual ID to look up
        id: String,
    },
}

fn locate_wordnet(path: Option<PathBuf>) -> Result<(String, LexiconHashMapBackend), String> {
    let path = if let Some(path) = path {
        if path.join("entries-a.yaml").exists() {
            path.to_string_lossy().to_string()
        } else if path.join("src/yaml/entries-a.yaml").exists() {
            path.join("src/yaml/").to_string_lossy().to_string()
        } else {
            return Err(format!(
                "Could not find WordNet at {}",
                path.to_string_lossy()
            ));
        }
    } else if Path::new("./src/yaml/entries-a.yaml").exists() {
        "./src/yaml/".to_owned()
    } else if Path::new("./entries-a.yaml").exists() {
        "./".to_owned()
    } else {
        return Err(format!("Please specify WordNet home"));
    };
    let mut progress = NullProgress;
    let wn = LexiconHashMapBackend::new()
        .load(&path, &mut progress)
        .map_err(|e| format!("Could not load WordNet from {}: {}", path, e))?;
    Ok((path, wn))
}

fn run_automaton(script: impl AsRef<Path>, wordnet: Option<PathBuf>) {
    let f = File::open(&script).unwrap_or_else(|_| {
        eprintln!(
            "Could not open automaton file: {}",
            script.as_ref().display()
        );
        exit(-1);
    });
    let actions: Vec<oewn_lib::automaton::Action> =
        serde_yaml::from_reader(f).unwrap_or_else(|e| {
            eprintln!("Could not parse automaton file: {}", e);
            exit(-1);
        });
    let (path, mut wn) = locate_wordnet(wordnet).unwrap_or_else(|e| {
        eprintln!("{}", e);
        exit(-1);
    });

    let mut ewe_changed = ChangeList::new();

    oewn_lib::automaton::apply_automaton(actions, &mut wn, &mut ewe_changed).unwrap_or_else(|e| {
        eprintln!("Could not apply automaton: {}", e);
        exit(-1);
    });

    save(&wn, &path).expect("Could not save");
}

fn group_by_key<A, B>(pairs: Vec<(A, B)>) -> HashMap<A, Vec<B>>
where
    A: Eq + std::hash::Hash,
{
    let mut map = HashMap::new();

    for (a, b) in pairs {
        map.entry(a).or_insert_with(Vec::new).push(b);
    }

    map
}

fn print_synset(synset_id: &SynsetId, synset: &Synset, lexicon: &impl Lexicon) {
    println!("{}: {}", synset_id, synset.members.join(", "));
    println!("    {}", synset.definition[0]);
    if !synset.example.is_empty() {
        println!(
            "    ({})",
            synset
                .example
                .iter()
                .map(|ex| ex.text.clone())
                .collect::<Vec<String>>()
                .join("; ")
        );
    }
    let links_from = lexicon.links_from(synset_id).expect("Cannot read lexicon");
    for (rel, targets) in group_by_key(links_from) {
        let target_strs: Vec<String> = targets
            .into_iter()
            .map(|t| {
                let s: String = format!(
                    "{} ({})",
                    t,
                    lexicon
                        .synset_by_id(&t)
                        .expect("Cannot read lexicon")
                        .expect("ID not in lexicon")
                        .members
                        .join(", ")
                );
                s
            })
            .collect();
        let target_str = target_strs.join("; ");
        println!("    {}: {}", rel.value(), target_str);
    }
    let links_to = lexicon.links_to(synset_id).expect("Cannot read lexicon");
    for (rel, targets) in group_by_key(links_to) {
        let target_strs: Vec<String> = targets
            .into_iter()
            .map(|t| {
                let s: String = format!(
                    "{} ({})",
                    t,
                    lexicon
                        .synset_by_id(&t)
                        .expect("Cannot read lexicon")
                        .expect("ID not in lexicon")
                        .members
                        .join(", ")
                );
                s
            })
            .collect();
        let target_str = target_strs.join("; ");
        if let Some(inv_rel) = rel.inverse() {
            println!("    {}: {}", inv_rel.value(), target_str);
        } else {
            println!("    Inverse {}: {}", rel.value(), target_str);
        }
    }

    println!("");
}

fn run_word(word: &str, ignore_case: bool, wordnet: Option<PathBuf>) {
    let (_, wn) = locate_wordnet(wordnet).unwrap_or_else(|e| {
        eprintln!("{}", e);
        exit(-1);
    });
    let entries = if ignore_case {
        wn.entry_by_lemma_ignore_case(word)
            .expect("Cannot read wordnet")
    } else {
        wn.entry_by_lemma(word).expect("Cannot read wordnet")
    };
    if entries.is_empty() {
        println!("No entries found for '{}'", word);
    } else {
        for entry in entries {
            for sense in &entry.sense {
                let synset = wn
                    .synset_by_id(&sense.synset)
                    .expect("Cannot read wordnet")
                    .unwrap();
                print_synset(&sense.synset, &synset, &wn);
            }
        }
    }
}

fn run_id(id: &str, wordnet: Option<PathBuf>) {
    let (_, wn) = locate_wordnet(wordnet).unwrap_or_else(|e| {
        eprintln!("{}", e);
        exit(-1);
    });
    if let Some(synset) = wn
        .synset_by_id(&SynsetId::new(&id))
        .expect("Cannot read wordnet")
    {
        print_synset(&SynsetId::new(&id), &synset, &wn);
    } else if let Some((_, _, sense)) = wn
        .get_sense_by_id(&SenseId::new(id))
        .expect("Cannot read wordnet")
    {
        let synset = wn
            .synset_by_id(&sense.synset)
            .expect("Cannot read wordnet")
            .unwrap();
        print_synset(&sense.synset, &synset, &wn);
    } else {
        println!("No synset or sense found for '{}'", id);
    }
}

fn run_tui() {
    println!("");
    println!("         ,ww                             ");
    println!("   wWWWWWWW_)  Welcome to EWE            ");
    println!("   `WWWWWW'    - English WordNet Editor  ");
    println!("    II  II                               ");
    println!("");

    let path = if Path::new("./src/yaml/entries-a.yaml").exists() {
        "./src/yaml/".to_owned()
    } else if Path::new("./entries-a.yaml").exists() {
        "./".to_owned()
    } else {
        let mut s = input("WordNet Home Folder: ");
        while !Path::new(&s).join("entries-a.yaml").exists()
            && !Path::new(&s).join("src/yaml/entries-a.yaml").exists()
        {
            println!("Could not find WordNet at this path.");
            s = input("WordNet Home Folder: ");
        }
        if Path::new(&s).join("entries-a.yaml").exists() {
            s
        } else {
            Path::new(&s)
                .join("src/yaml/")
                .to_string_lossy()
                .to_string()
        }
    };

    let mut progress = IndicatifProgress::new();
    let mut wn = LexiconHashMapBackend::new()
        .load(&path, &mut progress)
        .unwrap();

    let mut ewe_changed = ChangeList::new();

    while main_menu(&mut wn, &path, &mut ewe_changed) {}
}

fn main() {
    let cli = EweCli::parse();
    match &cli.command {
        Some(Command::Automaton { ref script }) => {
            run_automaton(script, cli.wordnet);
        }
        Some(Command::Id { ref id }) => {
            run_id(id, cli.wordnet);
        }
        Some(Command::Word {
            ref word,
            ignore_case,
        }) => {
            run_word(word, *ignore_case, cli.wordnet);
        }
        None => {
            run_tui();
        }
    }
}
