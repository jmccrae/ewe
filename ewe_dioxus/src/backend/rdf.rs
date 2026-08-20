//! Implements the key RDF functionality of returning pages as RDF
//! using content negotiation
//!
//! The actual RDF triple model (namespaces, entry/synset/sense triples, dedup) lives in
//! `ewe_lib::wordnet::rdf` - this module is just the HTTP-routing/content-negotiation layer on
//! top of it.

// TODO : URLs need to have a short prefix (oewn-00001740-n) instead of just ID 00001740-n
use crate::dioxus_fullstack::response::IntoResponse;
use crate::dioxus_fullstack::{body::Body, http::Response, HeaderMap, Redirect};
use dioxus::prelude::*;
use ewe_lib::wordnet::rdf::{write_lexicon_rdf_subset, RdfExportOptions, RdfFormat};
use ewe_lib::wordnet::{Lexicon, LexiconMetadata, MemberSynset, SynsetId};
use std::collections::BTreeSet;

/// The site/license/language every `ewe_dioxus`-served RDF resource is exported under -
/// matches the real `en-word.net` deployment. Not currently wired to `EweSettings`
/// (`crate::db::read_settings()`), unlike `id_prefix` below; that's a separate, pre-existing
/// gap this refactor doesn't address.
fn rdf_export_options(format: RdfFormat) -> RdfExportOptions {
    RdfExportOptions {
        format,
        site: "https://en-word.net/".to_owned(),
        metadata: LexiconMetadata {
            id_prefix: String::new(),
            label: "Open English Wordnet".to_owned(),
            language: "en".to_owned(),
            email: None,
            license: "https://creativecommons.org/licenses/by/4.0/".to_owned(),
            version: String::new(),
            url: Some("https://en-word.net/".to_owned()),
        },
    }
}

/// The subcategorization-frame table `synsem:synBehavior` links are built from. Returns an
/// empty table (not an error) if the lexicon isn't loaded, matching
/// [`resolve_lemma_synsets`]'s "empty, not an error" convention for the same reason.
fn resolve_frames() -> Result<Vec<(String, String)>> {
    let Ok(lexicon) = crate::db::read_lexicon() else {
        return Ok(Vec::new());
    };
    Ok(lexicon.frames_get()?.into_owned())
}

/// Legacy WordNet ids come prefixed with the dataset name - the deployment's
/// configured `settings.id_prefix` (`oewn-` for the Open English Wordnet), or
/// `ewn-` as a hardcoded legacy fallback from when this project was called
/// just "English WordNet" - e.g. `oewn-00001740-n`, whereas every route in
/// this app (`/synset/{id}`, `/api/synset/{id}`, etc.) takes the bare id,
/// e.g. `00001740-n`. Returns `None` if `id` carries neither prefix.
fn strip_synset_id_prefix<'a>(id: &'a str, id_prefix: &str) -> Option<&'a str> {
    id.strip_prefix(&format!("{}-", id_prefix))
        .or_else(|| id.strip_prefix("ewn-"))
}

/// Legacy synset lookup paths: `/id/{id}` and `/synset/{id}` where `id`
/// still carries an `oewn-`/`ewn-` prefix (see [`strip_synset_id_prefix`]).
/// Both permanently redirect to the canonical, unprefixed `/synset/{id}`,
/// which then does the normal content negotiation.
#[get("/id/{id}")]
pub async fn synset_id_alias(id: String) -> Result<Response<Body>> {
    let settings = crate::db::read_settings();
    let id_prefix = &settings.id_prefix;
    let bare_id = strip_synset_id_prefix(&id, id_prefix).unwrap_or(&id);
    Ok(Redirect::permanent(&format!("/synset/{}", bare_id)).into_response())
}

#[get("/synset/{id}", headers : HeaderMap)]
pub async fn synset_negotiated(id: String) -> Result<Response<Body>> {
    let settings = crate::db::read_settings();
    let id_prefix = &settings.id_prefix;
    if let Some(bare_id) = strip_synset_id_prefix(&id, id_prefix) {
        return Ok(Redirect::permanent(&format!("/synset/{}", bare_id)).into_response());
    }
    let content_type = negotiate(headers);
    let response = match content_type {
        ContentType::HTML => Redirect::to(&format!("/view/synset/{}", id)).into_response(),
        ContentType::RDFXML => Redirect::to(&format!("/rdf/synset/{}", id)).into_response(),
        ContentType::Turtle => Redirect::to(&format!("/ttl/synset/{}", id)).into_response(),
        ContentType::XML => Redirect::to(&format!("/xml/synset/{}", id)).into_response(),
        ContentType::JSON => Redirect::to(&format!("/api/synset/{}", id)).into_response(),
    };
    Ok(response)
}

#[get("/rdf/synset/{id}")]
pub async fn synset_rdf(id: String) -> Result<Response<Body>> {
    synset_serialized(id, RdfFormat::RdfXml).await
}

#[get("/ttl/synset/{id}")]
pub async fn synset_ttl(id: String) -> Result<Response<Body>> {
    synset_serialized(id, RdfFormat::Turtle).await
}

async fn synset_serialized(id: String, format: RdfFormat) -> Result<Response<Body>> {
    let id = SynsetId::new_owned(id);
    let frames = resolve_frames()?;
    match resolve_synset(&id) {
        Ok(Some(ms)) => {
            match write_lexicon_rdf_subset(
                std::slice::from_ref(&ms),
                &frames,
                &rdf_export_options(format),
            ) {
                Ok(rdf_data) => Ok(Response::builder()
                    .header("Content-Type", format.media_type())
                    .body(Body::from(rdf_data))
                    .unwrap()),
                Err(e) => Ok(Response::builder()
                    .status(500)
                    .body(Body::from(format!("Internal server error: {}", e)))
                    .unwrap()),
            }
        }
        Ok(None) => Ok(Response::builder()
            .status(404)
            .body(Body::from("Synset not found"))
            .unwrap()),
        Err(e) => Ok(Response::builder()
            .status(500)
            .body(Body::from(format!("Internal server error: {}", e)))
            .unwrap()),
    }
}

#[get("/lemma/{lemma}", headers : HeaderMap)]
pub async fn lemma_negotiated(lemma: String) -> Result<Response<Body>> {
    let content_type = negotiate(headers);
    let response = match content_type {
        ContentType::HTML => Redirect::to(&format!("/view/lemma/{}", lemma)).into_response(),
        ContentType::RDFXML => Redirect::to(&format!("/rdf/lemma/{}", lemma)).into_response(),
        ContentType::Turtle => Redirect::to(&format!("/ttl/lemma/{}", lemma)).into_response(),
        ContentType::XML => Redirect::to(&format!("/xml/lemma/{}", lemma)).into_response(),
        ContentType::JSON => Redirect::to(&format!("/api/lemma/{}", lemma)).into_response(),
    };
    Ok(response)
}

#[get("/rdf/lemma/{lemma}")]
pub async fn lemma_rdf(lemma: String) -> Result<Response<Body>> {
    lemma_serialized(lemma, RdfFormat::RdfXml).await
}

#[get("/ttl/lemma/{lemma}")]
pub async fn lemma_ttl(lemma: String) -> Result<Response<Body>> {
    lemma_serialized(lemma, RdfFormat::Turtle).await
}

async fn lemma_serialized(lemma: String, format: RdfFormat) -> Result<Response<Body>> {
    match lemma_rdf_bytes(&lemma, format) {
        Ok(Some(rdf_data)) => Ok(Response::builder()
            .header("Content-Type", format.media_type())
            .body(Body::from(rdf_data))
            .unwrap()),
        Ok(None) => Ok(Response::builder()
            .status(404)
            .body(Body::from("Lemma not found"))
            .unwrap()),
        Err(e) => Ok(Response::builder()
            .status(500)
            .body(Body::from(format!("Internal server error: {}", e)))
            .unwrap()),
    }
}

fn lemma_rdf_bytes(lemma: &str, format: RdfFormat) -> Result<Option<Vec<u8>>> {
    let member_synsets = resolve_lemma_synsets(lemma)?;
    if member_synsets.is_empty() {
        return Ok(None);
    }

    let frames = resolve_frames()?;
    let rdf_data = write_lexicon_rdf_subset(&member_synsets, &frames, &rdf_export_options(format))?;

    Ok(Some(rdf_data))
}

/// Looks up a single synset by id and expands it into a [`MemberSynset`], the
/// enriched representation (with reverse relation links) shared by the RDF,
/// Turtle, XML, and JSON exports. Returns `Ok(None)` if the lexicon isn't
/// loaded or the id doesn't exist.
pub(crate) fn resolve_synset(id: &SynsetId) -> Result<Option<MemberSynset>> {
    let Ok(lexicon) = crate::db::read_lexicon() else {
        return Ok(None);
    };
    let Some(synset) = lexicon.synset_by_id(id)? else {
        return Ok(None);
    };
    Ok(Some(MemberSynset::from_synset(
        id,
        synset.into_owned(),
        &*lexicon,
    )?))
}

/// Looks up every distinct synset that a lemma has a sense in. Returns an
/// empty vec (not an error) if the lexicon isn't loaded or the lemma is
/// unknown.
pub(crate) fn resolve_lemma_synsets(lemma: &str) -> Result<Vec<MemberSynset>> {
    let Ok(lexicon) = crate::db::read_lexicon() else {
        return Ok(Vec::new());
    };

    let entries = lexicon.entry_by_lemma(lemma)?;
    let synset_ids: BTreeSet<SynsetId> = entries
        .iter()
        .flat_map(|entry| entry.sense.iter().map(|sense| sense.synset.clone()))
        .collect();

    let mut member_synsets = Vec::with_capacity(synset_ids.len());
    for id in &synset_ids {
        if let Some(synset) = resolve_synset(id)? {
            member_synsets.push(synset);
        }
    }
    Ok(member_synsets)
}

enum ContentType {
    HTML,
    RDFXML,
    Turtle,
    XML,
    JSON,
}

fn negotiate(headers: HeaderMap) -> ContentType {
    let accept_str = headers
        .get("Accept")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("*/*");

    let mut types: Vec<(f32, &str)> = accept_str
        .split(',')
        .filter_map(|part| {
            let mut pieces = part.split(';');
            let mime = pieces.next()?.trim();

            // Default quality is 1.0 if not specified
            let mut q = 1.0;
            for piece in pieces {
                let piece = piece.trim();
                if piece.starts_with("q=") {
                    q = piece[2..].parse::<f32>().unwrap_or(0.0);
                }
            }
            Some((q, mime))
        })
        .collect();

    types.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    types
        .into_iter()
        .find_map(|(_, mime)| {
            match mime {
                "text/html" => Some(ContentType::HTML),
                "application/rdf+xml" => Some(ContentType::RDFXML),
                "text/turtle" => Some(ContentType::Turtle),
                "application/xml" => Some(ContentType::XML),
                "application/json" => Some(ContentType::JSON),
                "*/*" => Some(ContentType::HTML), // Default to HTML if any type is accepted
                _ => None,
            }
        })
        .unwrap_or(ContentType::HTML) // Default to HTML if no acceptable type is found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_synset_id_prefix_oewn() {
        assert_eq!(
            strip_synset_id_prefix("oewn-00001740-n", "oewn"),
            Some("00001740-n")
        );
    }

    #[test]
    fn test_strip_synset_id_prefix_ewn() {
        assert_eq!(
            strip_synset_id_prefix("ewn-00001740-n", "oewn"),
            Some("00001740-n")
        );
    }

    #[test]
    fn test_strip_synset_id_prefix_none() {
        assert_eq!(strip_synset_id_prefix("00001740-n", "oewn"), None);
    }

    #[test]
    fn test_negotiate_html() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"
                .parse()
                .unwrap(),
        );
        assert!(matches!(negotiate(headers), ContentType::HTML));
    }

    #[test]
    fn test_negotiate_rdf_xml() {
        let mut headers = HeaderMap::new();
        headers.insert("Accept", "application/rdf+xml".parse().unwrap());
        assert!(matches!(negotiate(headers), ContentType::RDFXML));
    }

    #[test]
    fn test_negotiate_turtle() {
        let mut headers = HeaderMap::new();
        headers.insert("Accept", "text/turtle".parse().unwrap());
        assert!(matches!(negotiate(headers), ContentType::Turtle));
    }

    #[test]
    fn test_negotiate_json() {
        let mut headers = HeaderMap::new();
        headers.insert("Accept", "application/json".parse().unwrap());
        assert!(matches!(negotiate(headers), ContentType::JSON));
    }

    #[test]
    fn test_negotiate_xml() {
        let mut headers = HeaderMap::new();
        headers.insert("Accept", "application/xml".parse().unwrap());
        assert!(matches!(negotiate(headers), ContentType::XML));
    }

    #[test]
    fn test_negotiate_quality_preference() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Accept",
            "text/html;q=0.5,application/rdf+xml;q=0.9".parse().unwrap(),
        );
        assert!(matches!(negotiate(headers), ContentType::RDFXML));
    }

    #[test]
    fn test_negotiate_default() {
        let headers = HeaderMap::new();
        assert!(matches!(negotiate(headers), ContentType::HTML));
    }
}
