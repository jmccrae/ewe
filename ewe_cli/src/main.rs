extern crate indicatif;
extern crate lazy_static;
extern crate regex;
extern crate serde_yaml;

mod indicatif_progress;

use clap::{Parser, Subcommand};
use indicatif_progress::IndicatifProgress;
use lazy_static::lazy_static;
use ewe_lib::automaton::ActionWrapper;
use ewe_lib::change_manager;
use ewe_lib::change_manager::ChangeList;
use ewe_lib::progress::NullProgress;
use ewe_lib::rels::{SenseRelType, SynsetRelType};
use ewe_lib::validate::{fix, validate};
use ewe_lib::wordnet::rdf::{write_lexicon_rdf, RdfExportOptions, RdfFormat};
use ewe_lib::wordnet::xml::{read_lexicon_xml, write_lexicon_xml};
use ewe_lib::wordnet::{write_wndb, Lexicon, LexiconHashMapBackend, LexiconMetadata, PosKey, Sense, SenseId, SenseOrSynsetId, Synset, SynsetId, WndbExportOptions};
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

/// Prompts for a sense within a synset. If `allow_none`, picking "0" targets
/// the synset itself rather than a specific sense within it - only offer
/// this for relation types where that's meaningful (see
/// `SenseRelType::allows_synset_target`).
fn enter_sense<L: Lexicon>(wordnet: &L, spec_string: &str, allow_none: bool) -> SenseOrSynsetId {
    let synset_id = enter_synset(wordnet, spec_string).0;
    let mems = wordnet
        .members_by_id(&synset_id)
        .expect("Cannot read wordnet");
    loop {
        if allow_none {
            println!("0. Synset (no sense)");
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
                            return SenseOrSynsetId::Sense(ssid);
                        }
                        None => {}
                    }
                } else if i == 0 && allow_none {
                    return SenseOrSynsetId::Synset(synset_id);
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
            input_subcats("Enter verb subcats as comma-separated list: ")
        } else {
            Vec::new()
        };
        println!("Adding {} to synset {}", lemma, synset_id.as_str());
        change_manager::add_entry(wn, synset_id, lemma, pos, subcat, None, change_list)
            .expect("Could not add entry");
    } else if action == "D" {
        match wn
            .pos_for_entry_synset(&lemma, &synset_id)
            .expect("Cannot read wordnet")
        {
            Some(pos) => {
                println!("Removing {} from synset {}", lemma, synset_id.as_str());
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
                    println!(
                        "Moving {} from {} to {}",
                        lemma,
                        synset_id.as_str(),
                        target_synset_id.as_str()
                    );
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
                println!("Removing {} from synset {}", lemma, synset_id.as_str());
                change_manager::delete_entry(wn, &synset_id, &lemma, &pos, true, change_list)
                    .expect("Could not delete entry");
                let new_lemma = input("New lemma: ");
                let subcat = if pos.as_str() == "v" {
                    input_subcats("Enter verb subcats as comma-separated list: ")
                } else {
                    Vec::new()
                };
                println!("Adding {} to synset {}", new_lemma, synset_id.as_str());
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

        println!("Deleting synset {}", synset_id.as_str());
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
                        input_subcats("Enter verb subcats as comma-separated list: ")
                    } else {
                        Vec::new()
                    };
                    if lemma.len() > 0 {
                        println!("Adding {} to synset {}", lemma, new_id.as_str());
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
            let target_sense_id = enter_sense(wn, "target ", rel.allows_synset_target());
            println!(
                "Insert {} ={}=> {}",
                source_sense_id.as_str(),
                rel.value(),
                target_sense_id.as_str()
            );
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
            println!(
                "Insert {} ={}=> {}",
                source_id.as_str(),
                rel.value(),
                target_id.as_str()
            );
            change_manager::insert_rel(wn, &source_id, &rel, &target_id, change_list)
                .expect("Could not add relation");
        }
    }
}

fn delete_relation<L: Lexicon>(wn: &mut L, change_list: &mut ChangeList) {
    let (source_id, source_sense_id) = enter_sense_synset(wn, "source ", None);
    match source_sense_id {
        Some(source_sense_id) => {
            // `delete_sense_rel` removes whatever relation exists between the
            // pair regardless of type, so a synset target is always offered
            // here (it's simply a no-op if nothing was ever stored there).
            let target_sense_id = enter_sense(wn, "target ", true);
            println!(
                "Delete {} =*=> {}",
                source_sense_id.as_str(),
                target_sense_id.as_str()
            );
            change_manager::delete_sense_rel(wn, &source_sense_id, &target_sense_id, change_list)
                .expect("Could not delete relation");
        }
        None => {
            let target_id = enter_synset(wn, "target ").0;
            println!("Delete {} =*=> {}", source_id.as_str(), target_id.as_str());
            change_manager::delete_rel(wn, &source_id, &target_id, change_list);
        }
    }
}

fn reverse_relation<L: Lexicon>(wn: &mut L, change_list: &mut ChangeList) {
    let (source_id, source_sense_id) = enter_sense_synset(wn, "source ", None);
    match source_sense_id {
        Some(source_sense_id) => {
            let target_sense_id = enter_sense(wn, "target ", false);
            println!(
                "Reversing relation between {} and {}",
                source_sense_id.as_str(),
                target_sense_id.as_str()
            );
            change_manager::reverse_sense_rel(wn, &source_sense_id, &target_sense_id, change_list)
                .expect("Could not reverse relation");
        }
        None => {
            let target_id = enter_synset(wn, "target ").0;
            println!(
                "Reversing relation between {} and {}",
                source_id.as_str(),
                target_id.as_str()
            );
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
) -> result::Result<bool, ewe_lib::wordnet::LexiconSaveError> {
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

fn input_subcats(prompt: &str) -> Vec<String> {
    let subcats = input(prompt);
    if subcats.is_empty() {
        Vec::new()
    } else {
        subcats.split(",").map(|s| s.to_string()).collect()
    }
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
    about="EWE Wordnet Editor",
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
        script: String,
    },
    /// Search for a specific word
    Word {
        /// The word you want to search for
        word: String,

        /// Perform a case-insensitive search
        #[arg(short, long)]
        ignore_case: bool,

        /// Show the sense ID for each sense
        #[arg(short, long)]
        sense_ids: bool,
    },
    /// Search for an item by its unique ID
    Id {
        /// The numeric or textual ID to look up
        id: String,
    },
    /// Export the wordnet to another format
    Export {
        #[command(subcommand)]
        format: ExportFormat,
    },
    /// Import a wordnet from another format
    Import {
        #[command(subcommand)]
        format: ImportFormat,
    },
    /// Create a new, empty wordnet project, prompting for its key metadata
    Init {
        /// Directory to create the new project in (created if it doesn't exist)
        #[arg(default_value = "./")]
        path: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum ExportFormat {
    /// Export as a whole-lexicon, self-contained WN-LMF XML document
    /// (https://globalwordnet.github.io/schemas/)
    Xml {
        /// Path to write the XML document to
        path: PathBuf,

        /// The `Lexicon/@id` prefix used to build every element id in the document
        #[arg(long, default_value = "oewn")]
        id_prefix: String,
        /// The `Lexicon/@label`
        #[arg(long, default_value = "Open English Wordnet")]
        label: String,
        /// The `Lexicon/@language` (BCP 47 code)
        #[arg(long, default_value = "en")]
        language: String,
        /// The `Lexicon/@email` contact address
        #[arg(long)]
        email: Option<String>,
        /// The `Lexicon/@license` URL
        #[arg(long, default_value = "https://creativecommons.org/licenses/by/4.0")]
        license: String,
        /// The `Lexicon/@version`
        #[arg(long, default_value = "1")]
        version: String,
        /// The `Lexicon/@url` project homepage
        #[arg(long)]
        url: Option<String>,
    },
    /// Export as the classic WNDB (Princeton WordNet database) file set
    /// (data.*/index.*/index.sense/*.exc)
    Wndb {
        /// Directory to write the WNDB files to (created if it doesn't exist)
        path: PathBuf,

        /// A license/header file, prepended verbatim to every `data.*`/`index.*` file. Omit for
        /// no header at all.
        #[arg(long)]
        license_file: Option<PathBuf>,

        /// A `lemma,pos,synset_id1 synset_id2 ...` CSV recording sense order for lemmas that fold
        /// together case-insensitively in `index.{pos}` (e.g. `afghani`/`Afghani`)
        #[arg(long)]
        sense_orders: Option<PathBuf>,
    },
    /// Export as a whole-lexicon RDF document (Turtle or RDF/XML), suitable for an
    /// `en-word.net`-style RDF release - every `LexicalEntry` is declared exactly once,
    /// regardless of how many senses/synsets it appears in, so no external dedup pass
    /// (the old release process's `rapper -i turtle -o turtle`) is needed
    Rdf {
        /// Path to write the RDF document to
        path: PathBuf,

        /// The RDF serialization to write
        #[arg(long, value_enum, default_value_t = RdfSyntax::Turtle)]
        format: RdfSyntax,
        /// Base URI resources (`{site}synset/...`, `{site}lemma/...`) are built under, and the
        /// subject of the export's `lime:Lexicon` header
        #[arg(long, default_value = "https://en-word.net/")]
        site: String,
        /// The `lime:Lexicon` header's `rdfs:label`
        #[arg(long, default_value = "Open English Wordnet")]
        label: String,
        /// BCP 47 language tag applied to lemma/definition/example literals and the header's
        /// `dc:language`
        #[arg(long, default_value = "en")]
        language: String,
        /// The header's `schema:email` contact address
        #[arg(long)]
        email: Option<String>,
        /// License URL asserted as the header's `cc:license`
        #[arg(long, default_value = "https://creativecommons.org/licenses/by/4.0/")]
        license: String,
        /// The header's `owl:versionInfo`
        #[arg(long, default_value = "1")]
        version: String,
        /// The header's `schema:url` project homepage (defaults to `--site` if unset)
        #[arg(long)]
        url: Option<String>,
    },
}

/// The RDF serialization to write - maps directly onto `ewe_lib::wordnet::rdf::RdfFormat`
/// (`oxrdfio::RdfFormat`), just with kebab-case CLI-friendly variant names.
#[derive(clap::ValueEnum, Clone, Debug)]
enum RdfSyntax {
    Turtle,
    #[value(name = "rdf-xml")]
    RdfXml,
}

impl std::fmt::Display for RdfSyntax {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RdfSyntax::Turtle => write!(f, "turtle"),
            RdfSyntax::RdfXml => write!(f, "rdf-xml"),
        }
    }
}

impl From<RdfSyntax> for RdfFormat {
    fn from(syntax: RdfSyntax) -> RdfFormat {
        match syntax {
            RdfSyntax::Turtle => RdfFormat::Turtle,
            RdfSyntax::RdfXml => RdfFormat::RdfXml,
        }
    }
}

#[derive(Subcommand, Debug)]
enum ImportFormat {
    /// Import from a WN-LMF XML document (https://globalwordnet.github.io/schemas/), saving the
    /// result as a YAML source tree
    Xml {
        /// Path to the XML document to import
        path: PathBuf,
    },
}

/// Whether `path` looks like a YAML wordnet source directory. `entries-a.yaml` is the
/// traditional check, but a brand new project (e.g. fresh out of `ewe init`) has no entries at
/// all yet, so also accept `frames.yaml` - unlike any particular `entries-*.yaml`, `Lexicon::save`
/// always writes that one, empty or not.
fn looks_like_wordnet_dir(path: &Path) -> bool {
    path.join("entries-a.yaml").exists() || path.join("frames.yaml").exists()
}

fn locate_wordnet(path: Option<PathBuf>) -> Result<(String, LexiconHashMapBackend), String> {
    let path = if let Some(path) = path {
        if looks_like_wordnet_dir(&path) {
            path.to_string_lossy().to_string()
        } else if looks_like_wordnet_dir(&path.join("src/yaml")) {
            path.join("src/yaml/").to_string_lossy().to_string()
        } else {
            return Err(format!(
                "Could not find WordNet at {}",
                path.to_string_lossy()
            ));
        }
    } else if looks_like_wordnet_dir(Path::new("./src/yaml")) {
        "./src/yaml/".to_owned()
    } else if looks_like_wordnet_dir(Path::new("./")) {
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

fn run_automaton(script: &str, wordnet: Option<PathBuf>) {
    let actions = if script == "-" {
        let wrapped: Vec<ActionWrapper> =
            serde_yaml::from_reader(io::stdin()).unwrap_or_else(|e| {
                eprintln!("Could not parse automaton file: {}", e);
                exit(-1);
            });
        wrapped.into_iter().map(|x: ActionWrapper| x.0).collect()
    } else {
        let f = File::open(&script).unwrap_or_else(|_| {
            eprintln!("Could not open automaton file: {}", script);
            exit(-1);
        });
        let wrapped: Vec<ActionWrapper> = serde_yaml::from_reader(f).unwrap_or_else(|e| {
            eprintln!("Could not parse automaton file: {}", e);
            exit(-1);
        });
        wrapped.into_iter().map(|x: ActionWrapper| x.0).collect()
    };
    let (path, mut wn) = locate_wordnet(wordnet).unwrap_or_else(|e| {
        eprintln!("{}", e);
        exit(-1);
    });

    let mut ewe_changed = ChangeList::new();

    let (_, validation_report) = ewe_lib::automaton::apply_automaton(actions, &mut wn, &mut ewe_changed)
        .unwrap_or_else(|e| {
            eprintln!("Could not apply automaton: {}", e);
            exit(-1);
        });
    if let Some(report) = validation_report {
        for error in report.errors.iter() {
            println!("{}", error);
        }
        if report.errors.is_empty() {
            println!("No validation errors!");
        } else {
            println!("{} validation errors", report.errors.len());
        }
    }

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

fn print_synset(synset_id: &SynsetId, synset: &Synset, lexicon: &impl Lexicon, sense_id: Option<&SenseId>) {
    println!("{}: {}", synset_id, synset.members.join(", "));
    if let Some(sid) = sense_id {
        println!("    Sense: {}", sid);
    }
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
    for (rel, targets) in group_by_key(links_to.into_iter().filter(|(r, _)| !r.is_symmetric()).collect()) {
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

fn run_word(word: &str, ignore_case: bool, sense_ids: bool, wordnet: Option<PathBuf>) {
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
                let sid = if sense_ids { Some(&sense.id) } else { None };
                print_synset(&sense.synset, &synset, &wn, sid);
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
        print_synset(&SynsetId::new(&id), &synset, &wn, None);
    } else if let Some((_, _, sense)) = wn
        .get_sense_by_id(&SenseId::new(id))
        .expect("Cannot read wordnet")
    {
        let synset = wn
            .synset_by_id(&sense.synset)
            .expect("Cannot read wordnet")
            .unwrap();
        print_synset(&sense.synset, &synset, &wn, Some(&sense.id));
    } else {
        println!("No synset or sense found for '{}'", id);
    }
}

fn run_export_wndb(
    path: &Path,
    license_file: Option<PathBuf>,
    sense_orders: Option<PathBuf>,
    wordnet: Option<PathBuf>,
) {
    let (_, wn) = locate_wordnet(wordnet).unwrap_or_else(|e| {
        eprintln!("{}", e);
        exit(-1);
    });
    let options = WndbExportOptions {
        license_file,
        sense_orders,
    };
    write_wndb(&wn, path, &options).unwrap_or_else(|e| {
        eprintln!("Could not generate WNDB: {}", e);
        exit(-1);
    });
    println!("Wrote {}", path.display());
}

fn run_export_xml(path: &Path, metadata: LexiconMetadata, wordnet: Option<PathBuf>) {
    let (_, wn) = locate_wordnet(wordnet).unwrap_or_else(|e| {
        eprintln!("{}", e);
        exit(-1);
    });
    let xml = write_lexicon_xml(&wn, &metadata).unwrap_or_else(|e| {
        eprintln!("Could not generate XML: {}", e);
        exit(-1);
    });
    std::fs::write(path, xml).unwrap_or_else(|e| {
        eprintln!("Could not write {}: {}", path.display(), e);
        exit(-1);
    });
    println!("Wrote {}", path.display());
}

fn run_export_rdf(path: &Path, options: RdfExportOptions, wordnet: Option<PathBuf>) {
    let (_, wn) = locate_wordnet(wordnet).unwrap_or_else(|e| {
        eprintln!("{}", e);
        exit(-1);
    });
    let rdf = write_lexicon_rdf(&wn, &options).unwrap_or_else(|e| {
        eprintln!("Could not generate RDF: {}", e);
        exit(-1);
    });
    std::fs::write(path, rdf).unwrap_or_else(|e| {
        eprintln!("Could not write {}: {}", path.display(), e);
        exit(-1);
    });
    println!("Wrote {}", path.display());
}

/// Escapes a value for a TOML basic string (`"..."`).
fn toml_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// A minimal `settings.toml` for the freshly-imported project, in the same format
/// `ewe_dioxus/english-wordnet-settings.toml` documents (see that crate's README) - populated
/// from the imported `Lexicon` element's own metadata rather than OEWN's specific branding,
/// which wouldn't be correct for an arbitrary imported GWA wordnet.
fn generate_settings_toml(metadata: &LexiconMetadata) -> String {
    let mut out = String::new();
    out.push_str("database = \"wordnet.db\"\n");
    out.push_str("wordnet_source = \"src/yaml/\"\n");
    out.push_str(&format!("id_prefix = \"{}\"\n", toml_escape(&metadata.id_prefix)));
    if !metadata.label.is_empty() {
        out.push_str(&format!("project_name = \"{}\"\n", toml_escape(&metadata.label)));
    }
    if let Some(email) = &metadata.email {
        out.push_str(&format!("contact_email = \"{}\"\n", toml_escape(email)));
    }
    if let Some(url) = &metadata.url {
        out.push_str(&format!("source_url = \"{}\"\n", toml_escape(url)));
    }
    out
}

/// Writes `wn`/`metadata` out as a full project directory, mirroring the real OEWN layout:
/// entries-*.yaml/lexfile.yaml/frames.yaml under `src/yaml/`, `deprecations.csv` as its sibling
/// (`Lexicon::save` already writes that one via `src/yaml/../deprecations.csv`), and a
/// `settings.toml` an `ewe_dioxus` deployment can point straight at. Shared by `ewe import xml`
/// and `ewe init`, so both produce the same project shape.
fn write_project_structure<L: Lexicon>(wn: &L, metadata: &LexiconMetadata, out_dir: &Path) {
    let yaml_dir = out_dir.join("src").join("yaml");
    std::fs::create_dir_all(&yaml_dir).unwrap_or_else(|e| {
        eprintln!("Could not create {}: {}", yaml_dir.display(), e);
        exit(-1);
    });
    let mut progress = IndicatifProgress::new();
    wn.save(&yaml_dir, &mut progress).unwrap_or_else(|e| {
        eprintln!("Could not save to {}: {}", yaml_dir.display(), e);
        exit(-1);
    });
    let settings_path = out_dir.join("settings.toml");
    std::fs::write(&settings_path, generate_settings_toml(metadata)).unwrap_or_else(|e| {
        eprintln!("Could not write {}: {}", settings_path.display(), e);
        exit(-1);
    });
}

fn run_import_xml(path: &Path, out_dir: Option<PathBuf>) {
    let out_dir = out_dir.unwrap_or_else(|| PathBuf::from("./"));
    let file = File::open(path).unwrap_or_else(|e| {
        eprintln!("Could not open {}: {}", path.display(), e);
        exit(-1);
    });
    let (wn, metadata) = read_lexicon_xml(LexiconHashMapBackend::new(), file).unwrap_or_else(|e| {
        eprintln!("Could not import {}: {}", path.display(), e);
        exit(-1);
    });
    println!(
        "Imported {} entries, {} synsets from lexicon {:?} ({})",
        wn.n_entries().expect("Cannot read imported lexicon"),
        wn.n_synsets().expect("Cannot read imported lexicon"),
        metadata.label,
        metadata.id_prefix
    );

    write_project_structure(&wn, &metadata, &out_dir);
    println!("Saved project to {}", out_dir.display());
}

fn input_with_default(prompt: &str, default: &str) -> String {
    let value = input(&format!("{} [{}]: ", prompt, default));
    if value.is_empty() {
        default.to_string()
    } else {
        value
    }
}

fn input_optional(prompt: &str) -> Option<String> {
    let value = input(&format!("{} (optional): ", prompt));
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn run_init(path: &Path) {
    println!("Creating a new wordnet project at {}", path.display());
    let metadata = LexiconMetadata {
        id_prefix: input_with_default("Id prefix", "wn"),
        label: input_with_default("Project name", "My Wordnet"),
        language: input_with_default("Language (BCP 47 code)", "en"),
        license: input_with_default("License URL", "https://creativecommons.org/licenses/by/4.0"),
        version: input_with_default("Version", "1"),
        email: input_optional("Contact email"),
        url: input_optional("Source/homepage URL"),
    };

    write_project_structure(&LexiconHashMapBackend::new(), &metadata, path);
    println!("Created new wordnet project at {}", path.display());
}

fn run_tui() {
    println!("");
    println!("         ,ww                             ");
    println!("   wWWWWWWW_)  Welcome to EWE            ");
    println!("   `WWWWWW'    - EWE Wordnet Editor       ");
    println!("    II  II                               ");
    println!("");

    let path = if looks_like_wordnet_dir(Path::new("./src/yaml")) {
        "./src/yaml/".to_owned()
    } else if looks_like_wordnet_dir(Path::new("./")) {
        "./".to_owned()
    } else {
        let mut s = input("WordNet Home Folder: ");
        while !looks_like_wordnet_dir(Path::new(&s)) && !looks_like_wordnet_dir(&Path::new(&s).join("src/yaml")) {
            println!("Could not find WordNet at this path.");
            s = input("WordNet Home Folder: ");
        }
        if looks_like_wordnet_dir(Path::new(&s)) {
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
            sense_ids,
        }) => {
            run_word(word, *ignore_case, *sense_ids, cli.wordnet);
        }
        Some(Command::Export {
            format:
                ExportFormat::Xml {
                    ref path,
                    id_prefix,
                    label,
                    language,
                    email,
                    license,
                    version,
                    url,
                },
        }) => {
            let metadata = LexiconMetadata {
                id_prefix: id_prefix.clone(),
                label: label.clone(),
                language: language.clone(),
                email: email.clone(),
                license: license.clone(),
                version: version.clone(),
                url: url.clone(),
            };
            run_export_xml(path, metadata, cli.wordnet);
        }
        Some(Command::Export {
            format:
                ExportFormat::Wndb {
                    ref path,
                    license_file,
                    sense_orders,
                },
        }) => {
            run_export_wndb(path, license_file.clone(), sense_orders.clone(), cli.wordnet);
        }
        Some(Command::Export {
            format:
                ExportFormat::Rdf {
                    ref path,
                    format,
                    site,
                    label,
                    language,
                    email,
                    license,
                    version,
                    url,
                },
        }) => {
            let options = RdfExportOptions {
                format: format.clone().into(),
                site: site.clone(),
                metadata: LexiconMetadata {
                    // Unused by the RDF export (its URIs are built from bare lemma/synset
                    // ids, not a `Lexicon/@id` prefix) - not exposed as a CLI flag.
                    id_prefix: String::new(),
                    label: label.clone(),
                    language: language.clone(),
                    email: email.clone(),
                    license: license.clone(),
                    version: version.clone(),
                    url: url.clone().or_else(|| Some(site.clone())),
                },
            };
            run_export_rdf(path, options, cli.wordnet);
        }
        Some(Command::Import {
            format: ImportFormat::Xml { ref path },
        }) => {
            run_import_xml(path, cli.wordnet);
        }
        Some(Command::Init { ref path }) => {
            run_init(path);
        }
        None => {
            run_tui();
        }
    }
}
