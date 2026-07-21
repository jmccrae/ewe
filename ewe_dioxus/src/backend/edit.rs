//! Write endpoints for the Wordnet editor. Every edit goes through
//! [`oewn_lib::automaton::apply_automaton`] rather than mutating the lexicon directly, so edits
//! made through the UI are subject to the same rules as edits made through the `ewe_cli`
//! automaton scripts.
//!
//! Only compiled when the `edit` feature is enabled (see `ewe_dioxus/Cargo.toml`).

use dioxus::prelude::*;
#[allow(unused_imports)]
use oewn_lib::automaton::{apply_automaton, changelog_recent, Action, SynsetRef};
#[allow(unused_imports)]
use oewn_lib::change_manager::ChangeList;
#[allow(unused_imports)]
use oewn_lib::wordnet::{Lexicon, MemberSynset, PartOfSpeech, PosKey, SynsetId};
#[cfg(feature = "server")]
use oewn_lib::wordnet::ReDBLexicon;
#[cfg(feature = "server")]
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
enum EweEditError {
    #[error("Lexicon not available")]
    LexiconUnavailable,
    #[error("{0}")]
    Automaton(String),
    #[error("Synset {0} not found after edit")]
    SynsetNotFoundAfterEdit(String),
}

/// Takes a write lock on the shared lexicon, or an error if it failed to load at startup.
#[cfg(feature = "server")]
fn write_lexicon() -> Result<std::sync::RwLockWriteGuard<'static, ReDBLexicon>> {
    match crate::LEXICON.get() {
        Some(lock) => Ok(lock.write().unwrap()),
        None => Err(EweEditError::LexiconUnavailable.into()),
    }
}

/// Takes a read lock on the shared lexicon, or an error if it failed to load at startup.
#[cfg(feature = "server")]
fn read_lexicon() -> Result<std::sync::RwLockReadGuard<'static, ReDBLexicon>> {
    match crate::LEXICON.get() {
        Some(lock) => Ok(lock.read().unwrap()),
        None => Err(EweEditError::LexiconUnavailable.into()),
    }
}

/// Runs every pending edit to `synset` (a definition change, example add/update/delete, and
/// eventually relations/lemmas too) through the automaton as a single batch, then re-fetches
/// `synset` so the client can update its view without a separate round trip.
///
/// The caller (the "accept" button next to the Wikidata icon) is responsible for building
/// `actions` in an order that's valid against the *original* positions everything was drafted
/// from - notably, `UpdateExample`/`DeleteExample` reference an example's original 1-indexed
/// position, and deletes must run in descending-number order so an earlier delete doesn't shift
/// the position a later action expects.
#[post("/api/edit/apply")]
pub async fn apply_edits(synset: SynsetId, actions: Vec<Action>) -> Result<MemberSynset> {
    let mut lexicon = write_lexicon()?;

    apply_automaton(actions, &mut *lexicon, &mut ChangeList::new())
        .map_err(EweEditError::Automaton)?;

    let updated = lexicon
        .synset_by_id(&synset)?
        .ok_or_else(|| EweEditError::SynsetNotFoundAfterEdit(synset.as_str().to_string()))?;
    Ok(MemberSynset::from_synset(
        &synset,
        updated.into_owned(),
        &*lexicon,
    )?)
}

/// A synset candidate for the relation editor's target picker: unlike the main search box
/// (which returns one entry per lemma), the user needs to pick a specific *sense*, so each
/// candidate is a synset with enough context (members, definition) to tell them apart.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SynsetCandidate {
    pub id: SynsetId,
    pub members: Vec<String>,
    pub definition: String,
    pub part_of_speech: PartOfSpeech,
}

/// Looks up synsets by lemma prefix (expanded to every synset each matching lemma belongs to)
/// or by synset/ILI id prefix, for the relation editor's target picker.
#[get("/api/edit/search_synsets/{query}?max_results")]
pub async fn search_synsets(
    query: String,
    max_results: Option<usize>,
) -> Result<Vec<SynsetCandidate>> {
    let lexicon = read_lexicon()?;
    let max_results = max_results.unwrap_or(20);

    let mut synset_ids: Vec<SynsetId> = Vec::new();
    'lemmas: for lemma in lexicon.lemma_by_prefix(&query, Some(max_results))? {
        for entry in lexicon.entry_by_lemma(&lemma)? {
            for sense in entry.sense.iter() {
                if !synset_ids.contains(&sense.synset) {
                    synset_ids.push(sense.synset.clone());
                }
                if synset_ids.len() >= max_results {
                    break 'lemmas;
                }
            }
        }
    }
    if synset_ids.len() < max_results {
        for ssid in lexicon.ssid_by_prefix(&query, Some(max_results - synset_ids.len()))? {
            let id = SynsetId::new_owned(ssid);
            if !synset_ids.contains(&id) {
                synset_ids.push(id);
            }
        }
    }

    let mut candidates = Vec::with_capacity(synset_ids.len());
    for id in &synset_ids {
        if let Some(synset) = lexicon.synset_by_id(id)? {
            candidates.push(SynsetCandidate {
                id: id.clone(),
                members: synset.members.clone(),
                definition: synset.definition.get(0).cloned().unwrap_or_default(),
                part_of_speech: synset.part_of_speech.clone(),
            });
        }
    }
    Ok(candidates)
}

/// Everything the "add a new synset" form needs to populate itself: every lexicographer file
/// actually present in this database (rather than hardcoding the standard list, so it only
/// offers files this data really has), and the subcategorization frames available to verb
/// lemmas (key, human-readable description), loaded from `frames.yaml` at startup.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AddSynsetMetadata {
    pub lexfiles: Vec<String>,
    pub frames: Vec<(String, String)>,
}

#[get("/api/edit/add_synset_metadata")]
pub async fn add_synset_metadata() -> Result<AddSynsetMetadata> {
    let lexicon = read_lexicon()?;
    let mut lexfiles: Vec<String> = lexicon
        .synsets_iter()?
        .filter_map(|r| r.ok().map(|(k, _)| k.clone()))
        .collect();
    lexfiles.sort();
    let frames = lexicon.frames_get()?.into_owned();
    Ok(AddSynsetMetadata { lexfiles, frames })
}

/// Creates a new synset (and an entry for each lemma) and returns it, so the client can
/// navigate straight to its page. `subcats`, if given, must be the same length as `lemmas`
/// (each lemma's applicable frame keys) - only meaningful for verb lemmas.
#[post("/api/edit/add_synset")]
pub async fn add_synset(
    definition: String,
    lexfile: String,
    pos: Option<PosKey>,
    lemmas: Vec<String>,
    subcats: Vec<Vec<String>>,
) -> Result<MemberSynset> {
    let mut lexicon = write_lexicon()?;
    let actions = vec![Action::AddSynset {
        definition,
        lexfile,
        pos,
        lemmas,
        subcats,
    }];
    let new_id = apply_automaton(actions, &mut *lexicon, &mut ChangeList::new())
        .map_err(EweEditError::Automaton)?
        .ok_or_else(|| EweEditError::Automaton("No synset was created".to_string()))?;
    let synset = lexicon
        .synset_by_id(&new_id)?
        .ok_or_else(|| EweEditError::SynsetNotFoundAfterEdit(new_id.as_str().to_string()))?;
    Ok(MemberSynset::from_synset(
        &new_id,
        synset.into_owned(),
        &*lexicon,
    )?)
}

/// Deletes a synset, either deprecating it in favor of `superseded_by` (hands off its entries,
/// examples and relations, and leaves a deprecation record - the traditional, deliberate-edit
/// path) or, if `superseded_by` is omitted, removing it outright with no trail - appropriate for
/// a synset a user just created through this same UI and immediately decided against. Returns
/// the id of the synset the client should navigate to next (`superseded_by`, if given).
#[post("/api/edit/delete_synset")]
pub async fn delete_synset(
    synset: SynsetId,
    reason: String,
    superseded_by: Option<SynsetId>,
) -> Result<Option<SynsetId>> {
    let mut lexicon = write_lexicon()?;
    let navigate_to = superseded_by.clone();
    let actions = vec![Action::DeleteSynset {
        synset: SynsetRef::Id(synset),
        reason,
        superseded_by: superseded_by.map(SynsetRef::Id),
    }];
    apply_automaton(actions, &mut *lexicon, &mut ChangeList::new())
        .map_err(EweEditError::Automaton)?;
    Ok(navigate_to)
}

#[cfg(feature = "server")]
fn format_timestamp(timestamp_ms: u64) -> String {
    DateTime::<Utc>::from_timestamp_millis(timestamp_ms as i64)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "unknown time".to_string())
}

/// One batch of actions from the change log, ready to display: a formatted timestamp and a short
/// summary per action (see `Action::summary`) rather than the raw action data, since the client
/// only ever displays it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangeLogEntryView {
    pub id: u64,
    pub timestamp: String,
    pub summaries: Vec<String>,
}

/// The `limit` most recent change log entries, newest first. `before` (exclusive), if given,
/// paginates further back than a previous page's oldest id - see `History`'s "Load more".
#[get("/api/edit/changelog?limit&before")]
pub async fn get_changelog(
    limit: Option<usize>,
    before: Option<u64>,
) -> Result<Vec<ChangeLogEntryView>> {
    let lexicon = read_lexicon()?;
    let limit = limit.unwrap_or(50).min(200);
    let entries = changelog_recent(&*lexicon, limit, before).map_err(EweEditError::Automaton)?;
    Ok(entries
        .into_iter()
        .map(|(id, entry)| ChangeLogEntryView {
            id,
            timestamp: format_timestamp(entry.timestamp_ms),
            summaries: entry.actions.iter().map(Action::summary).collect(),
        })
        .collect())
}
