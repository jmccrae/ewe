use dioxus::prelude::*;
#[allow(unused_imports)]
use ewe_lib::wordnet::{Lexicon, MemberSynset, SynsetId};
use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use std::collections::BTreeSet;
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
enum EweAPIError {
    #[error("Lexicon not available")]
    LexiconUnavailable,
}

/// The branding fields configurable via `settings.toml` that need to reach
/// client-rendered pages. Fetched through a server function (rather than
/// reading `crate::SETTINGS` directly from view code) because `SETTINGS` is
/// a `Lazy` that can only initialize on the server: touching it from code
/// that also compiles into the WASM client panics with "Lazy initialization
/// is only supported with tokio and threads enabled."
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Branding {
    pub project_name: String,
    pub footer: String,
}

#[get("/api/branding")]
pub async fn get_branding() -> Result<Branding> {
    let settings = crate::SETTINGS.get();
    Ok(Branding {
        project_name: settings.project_name.clone(),
        footer: settings.footer.clone(),
    })
}

/// Everything the home page needs in one round trip: the configurable
/// tagline/intro text plus live counts, so the page can show e.g.
/// "142,384 synsets".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HomeInfo {
    pub tagline: String,
    pub intro: String,
    pub n_synsets: usize,
    pub n_entries: usize,
}

#[get("/api/home")]
pub async fn get_home_info() -> Result<HomeInfo> {
    let settings = crate::SETTINGS.get();
    let tagline = settings.tagline.clone();
    let intro = settings.intro.clone();
    if let Some(lexicon) = crate::LEXICON.get() {
        let lexicon = lexicon.read().unwrap();
        Ok(HomeInfo {
            tagline,
            intro,
            n_synsets: lexicon.n_synsets()?,
            n_entries: lexicon.n_entries()?,
        })
    } else {
        Err(EweAPIError::LexiconUnavailable.into())
    }
}

/// A uniformly random synset id, for the home page's "Random synset" button.
#[get("/api/random_synset")]
pub async fn get_random_synset() -> Result<Option<SynsetId>> {
    if let Some(lexicon) = crate::LEXICON.get() {
        let lexicon = lexicon.read().unwrap();
        Ok(lexicon.random_synset_id()?)
    } else {
        Err(EweAPIError::LexiconUnavailable.into())
    }
}

/// What a [`SearchResult`] refers to, so the frontend knows which page to
/// navigate to when a suggestion is picked.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SearchResultKind {
    Lemma,
    Synset,
}

/// A single autocomplete suggestion. `value` is what to look up (a lemma, or
/// a bare synset id like `00001740-n`); `display` is what to show the user.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResult {
    pub display: String,
    pub kind: SearchResultKind,
    pub value: String,
}

/// Users may search by lemma, by synset id (with or without the configured
/// `id_prefix` used in the RDF/XML/Turtle exports), or by ILI. Strips a
/// leading, case-insensitive `{id_prefix}-` so both `00001740-n` and
/// `oewn-00001740-n` (with the default `id_prefix`) match.
#[allow(dead_code)]
fn strip_id_prefix<'a>(query: &'a str, id_prefix: &str) -> &'a str {
    let prefixed = format!("{}-", id_prefix);
    match query.get(..prefixed.len()) {
        Some(prefix) if prefix.eq_ignore_ascii_case(&prefixed) => &query[prefixed.len()..],
        _ => query,
    }
}

#[get("/api/by_lemma/{lemma}")]
pub async fn get_lemma(lemma: String) -> Result<Vec<SynsetId>> {
    if let Some(lexicon) = crate::LEXICON.get() {
        let lexicon = lexicon.read().unwrap();
        let lemmas = lexicon.entry_by_lemma(&lemma)?;
        let synset_ids = lemmas
            .iter()
            .flat_map(|entry| entry.sense.iter().map(|sense| sense.synset.clone()))
            .collect();
        Ok(synset_ids)
    } else {
        Err(EweAPIError::LexiconUnavailable.into())
    }
}

#[get("/api/lemma/{lemma}")]
pub async fn get_lemma_synsets(lemma: String) -> Result<Vec<MemberSynset>> {
    if let Some(lexicon) = crate::LEXICON.get() {
        let lexicon = lexicon.read().unwrap();
        let entries = lexicon.entry_by_lemma(&lemma)?;
        let synset_ids: BTreeSet<SynsetId> = entries
            .iter()
            .flat_map(|entry| entry.sense.iter().map(|sense| sense.synset.clone()))
            .collect();

        let mut synsets = Vec::with_capacity(synset_ids.len());
        for id in &synset_ids {
            if let Some(synset) = lexicon.synset_by_id(id)? {
                synsets.push(MemberSynset::from_synset(id, synset.into_owned(), &*lexicon)?);
            }
        }
        Ok(synsets)
    } else {
        Err(EweAPIError::LexiconUnavailable.into())
    }
}

#[get("/api/autocomplete/{query}?max_results")]
pub async fn autocomplete(query: String, max_results: Option<usize>) -> Result<Vec<SearchResult>> {
    let max_results = max_results.unwrap_or(100);
    if let Some(lexicon) = crate::LEXICON.get() {
        let lexicon = lexicon.read().unwrap();
        let mut results = Vec::new();

        for lemma in lexicon.lemma_by_prefix(&query, Some(max_results))? {
            results.push(SearchResult {
                display: lemma.clone(),
                kind: SearchResultKind::Lemma,
                value: lemma,
            });
        }

        // Synset ids may be typed bare ("00001740-n") or with the configured
        // id_prefix used in the RDF/XML/Turtle exports.
        let id_prefix = &crate::SETTINGS.get().id_prefix;
        for ssid in lexicon.ssid_by_prefix(strip_id_prefix(&query, id_prefix), Some(max_results))? {
            results.push(SearchResult {
                display: format!("{}-{}", id_prefix, ssid),
                kind: SearchResultKind::Synset,
                value: ssid,
            });
        }

        for (ili, ssid) in lexicon.ili_by_prefix(&query, Some(max_results))? {
            results.push(SearchResult {
                display: format!("{} ({})", ili, ssid.as_str()),
                kind: SearchResultKind::Synset,
                value: ssid.as_str().to_string(),
            });
        }

        let mut results = results.into_iter().take(max_results).collect::<Vec<_>>();
        results.sort_by(|a, b| match a.display.to_lowercase().cmp(&b.display.to_lowercase()) {
            std::cmp::Ordering::Equal => a.display.cmp(&b.display).reverse(),
            x => x,
        });
        Ok(results)
    } else {
        Err(EweAPIError::LexiconUnavailable.into())
    }
}

#[get("/api/synset/{id}")]
pub async fn get_synset(id: SynsetId) -> Result<Option<MemberSynset>> {
    if let Some(lexicon) = crate::LEXICON.get() {
        let lexicon = lexicon.read().unwrap();
        let synset = lexicon.synset_by_id(&id)?;
        if let Some(synset) = synset {
            Ok(Some(MemberSynset::from_synset(
                &id,
                synset.into_owned(),
                &*lexicon,
            )?))
        } else {
            Ok(None)
        }
    } else {
        Err(EweAPIError::LexiconUnavailable.into())
    }
}
