//! `ewe init` - creates a new, empty wordnet project, prompting for its key metadata.

use crate::commands::import::write_project_structure;
use crate::common::input;
use ewe_lib::wordnet::{LexiconHashMapBackend, LexiconMetadata};
use std::path::Path;

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

pub(crate) fn run(path: &Path) {
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
