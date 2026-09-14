//! `ewe automaton` - runs a scripted sequence of edits from a YAML automaton file.

use crate::common::{locate_wordnet, save};
use ewe_lib::automaton::ActionWrapper;
use ewe_lib::change_manager::ChangeList;
use std::fs::File;
use std::io;
use std::path::PathBuf;
use std::process::exit;

pub(crate) fn run(script: &str, wordnet: Option<PathBuf>) {
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

    let (_, validation_report) =
        ewe_lib::automaton::apply_automaton(actions, &mut wn, &mut ewe_changed).unwrap_or_else(
            |e| {
                eprintln!("Could not apply automaton: {}", e);
                exit(-1);
            },
        );
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
