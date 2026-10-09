use crate::common::{load_validation_options, locate_wordnet};
use ewe_lib::progress::NullProgress;
use ewe_lib::validate::{validate_with, ValidationOptions};
use std::path::PathBuf;
use std::process::exit;

/// Validate non-interactively (suitable for CI). Exits 0 if valid, 1 if there were validation
/// errors and 2 if the wordnet (or its `settings.toml`) could not be loaded or validation could
/// not complete. A check is skipped if either `options` (the CLI flags) or the project's
/// `settings.toml` `[validation]` table skips it.
pub(crate) fn run(wordnet: Option<PathBuf>, options: ValidationOptions) {
    let (path, wn) = locate_wordnet(wordnet).unwrap_or_else(|e| {
        eprintln!("{}", e);
        exit(2);
    });
    let options = options.union(load_validation_options(&path));
    let mut progress = NullProgress;
    let errors = validate_with(&wn, &mut progress, &options).unwrap_or_else(|e| {
        eprintln!("Could not complete validation: {}", e);
        exit(2);
    });
    for error in errors.iter() {
        println!("{}", error);
    }
    if errors.is_empty() {
        println!("No validation errors!");
    } else {
        println!("{} validation errors", errors.len());
        exit(1);
    }
}
