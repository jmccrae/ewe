//! Write endpoints for the Wordnet editor. Every edit goes through
//! [`oewn_lib::automaton::apply_automaton`] rather than mutating the lexicon directly, so edits
//! made through the UI are subject to the same rules as edits made through the `ewe_cli`
//! automaton scripts.
//!
//! Only compiled when the `edit` feature is enabled (see `ewe_dioxus/Cargo.toml`).

use dioxus::prelude::*;
#[allow(unused_imports)]
use oewn_lib::automaton::{apply_automaton, Action};
#[allow(unused_imports)]
use oewn_lib::change_manager::ChangeList;
#[allow(unused_imports)]
use oewn_lib::wordnet::{Lexicon, MemberSynset, SynsetId};
#[cfg(feature = "server")]
use oewn_lib::wordnet::ReDBLexicon;
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
