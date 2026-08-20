use dioxus::prelude::*;
#[allow(unused_imports)]
use ewe_lib::wordnet::{Lexicon, MemberSynset, SynsetId};
use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use std::collections::BTreeSet;
#[cfg(any(feature = "server", feature = "desktop"))]
use crate::db::read_lexicon;

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
    /// The theme stylesheet's own contents, inlined into a `<style>` tag by
    /// `views::wn_layout::WNLayout` rather than linked via `<link href="/theme.css">`. A `<link>`
    /// is a plain static-resource URL the browser/webview caches on its own with no reactive
    /// dependency on server state, so after `backend::setup::configure_wordnet_source`
    /// hot-swaps `SETTINGS` to point `theme` at a different file, it kept showing whatever was
    /// cached from before. Bundling the CSS text into this struct instead means it rides along
    /// with `project_name`/`footer` through the same `Loader<Branding>`, which already correctly
    /// refetches on `.restart()`.
    pub theme_css: String,
    /// The logo's own raw SVG markup, inlined directly into the page (via `dangerous_inner_html`)
    /// instead of linked via `<img src="/logo">`, for the same reason as `theme_css` above.
    /// Assumes `settings.toml`'s `logo` points at an SVG file, which is true of every logo
    /// shipped with this app (`assets/gwa.svg`, `assets/english.svg`).
    pub logo_svg: String,
}

// A desktop build embeds the lexicon/settings state directly in-process (see this crate's
// `main.rs`), so it doesn't need - and can't use - a real HTTP round-trip for any of these:
// `#[cfg_attr]` skips the `#[get]`/`#[post]` macro there entirely, leaving a plain async fn that
// view/component code (already only ever calling these isomorphically, e.g. via `use_loader`)
// invokes directly. `web`/`server` still go through the macro as normal - Dioxus's own internal
// `#[cfg(feature = "server")]` split inside its expansion already distinguishes the WASM
// client's RPC stub from the real server-side handler, so this crate never needs to.
#[cfg_attr(not(feature = "desktop"), get("/api/branding"))]
pub async fn get_branding() -> Result<Branding> {
    let settings = crate::db::read_settings();
    Ok(Branding {
        project_name: settings.project_name.clone(),
        footer: settings.footer.clone(),
        theme_css: std::fs::read_to_string(&settings.theme).unwrap_or_default(),
        logo_svg: std::fs::read_to_string(&settings.logo).unwrap_or_default(),
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

#[cfg_attr(not(feature = "desktop"), get("/api/home"))]
pub async fn get_home_info() -> Result<HomeInfo> {
    let settings = crate::db::read_settings();
    let tagline = settings.tagline.clone();
    let intro = settings.intro.clone();
    let lexicon = read_lexicon()?;
    Ok(HomeInfo {
        tagline,
        intro,
        n_synsets: lexicon.n_synsets()?,
        n_entries: lexicon.n_entries()?,
    })
}

/// A uniformly random synset id, for the home page's "Random synset" button.
#[cfg_attr(not(feature = "desktop"), get("/api/random_synset"))]
pub async fn get_random_synset() -> Result<Option<SynsetId>> {
    let lexicon = read_lexicon()?;
    Ok(lexicon.random_synset_id()?)
}

/// Look up the synset carrying a given ILI id, for `/ili/:id`-style links (see issue #20) -
/// the old `en-word.net/ili/iXXX` namespace, and the standard `globalwordnet.org/cili/iXXX`
/// URL now hyperlinked from a synset's own ILI identifier.
#[cfg_attr(not(feature = "desktop"), get("/api/synset_by_ili/{ili}"))]
pub async fn get_synset_by_ili(ili: String) -> Result<Option<SynsetId>> {
    let lexicon = read_lexicon()?;
    Ok(lexicon.synset_by_ili(&ili)?)
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

#[cfg_attr(not(feature = "desktop"), get("/api/by_lemma/{lemma}"))]
pub async fn get_lemma(lemma: String) -> Result<Vec<SynsetId>> {
    let lexicon = read_lexicon()?;
    let lemmas = lexicon.entry_by_lemma(&lemma)?;
    let synset_ids = lemmas
        .iter()
        .flat_map(|entry| entry.sense.iter().map(|sense| sense.synset.clone()))
        .collect();
    Ok(synset_ids)
}

#[cfg_attr(not(feature = "desktop"), get("/api/lemma/{lemma}"))]
pub async fn get_lemma_synsets(lemma: String) -> Result<Vec<MemberSynset>> {
    let lexicon = read_lexicon()?;
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
}

/// A single short definition, for a page's `<meta name="description">` - deliberately far
/// cheaper than `get_lemma_synsets` since it skips `MemberSynset::from_synset`'s reverse-
/// relation expansion entirely, needed only for one sentence of text.
#[cfg_attr(not(feature = "desktop"), get("/api/lemma_description/{lemma}"))]
pub async fn get_lemma_description(lemma: String) -> Result<Option<String>> {
    let lexicon = read_lexicon()?;
    let entries = lexicon.entry_by_lemma(&lemma)?;
    let Some(synset_id) = entries
        .iter()
        .find_map(|entry| entry.sense.first().map(|sense| sense.synset.clone()))
    else {
        return Ok(None);
    };
    let Some(synset) = lexicon.synset_by_id(&synset_id)? else {
        return Ok(None);
    };
    Ok(synset.definition.first().cloned())
}

#[cfg_attr(not(feature = "desktop"), get("/api/autocomplete/{query}?max_results"))]
pub async fn autocomplete(query: String, max_results: Option<usize>) -> Result<Vec<SearchResult>> {
    let max_results = max_results.unwrap_or(100);
    let lexicon = read_lexicon()?;
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
    let settings = crate::db::read_settings();
    let id_prefix = &settings.id_prefix;
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
}

#[cfg_attr(not(feature = "desktop"), get("/api/synset/{id}"))]
pub async fn get_synset(id: SynsetId) -> Result<Option<MemberSynset>> {
    let lexicon = read_lexicon()?;
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
}
