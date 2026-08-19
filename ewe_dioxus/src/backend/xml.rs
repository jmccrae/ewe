//! WN-LMF XML export web routes.
//!
//! Each export is a self-contained `LexicalResource`/`Lexicon` covering just the requested
//! synset (or all synsets a lemma has a sense in). The actual XML generation lives in
//! `ewe_lib::wordnet::xml` (shared with `ewe_cli`'s whole-lexicon `export xml` command); this
//! module is just the web-route glue that resolves a synset/lemma against this app's lexicon and
//! settings, then hands off to the shared writer. See that module's doc comment for the
//! self-containment caveat a partial export like this implies.

use crate::backend::rdf::{resolve_lemma_synsets, resolve_synset};
use crate::dioxus_fullstack::{body::Body, http::Response};
use crate::settings::EweSettings;
use dioxus::prelude::*;
use ewe_lib::wordnet::xml::write_lexicon_xml_subset;
use ewe_lib::wordnet::{LexiconMetadata, SynsetId};

#[get("/xml/synset/{id}")]
pub async fn synset_xml(id: String) -> Result<Response<Body>> {
    let id = SynsetId::new_owned(id);
    match resolve_synset(&id) {
        // No frame table for a single-synset fragment - see write_lexicon_xml_subset's doc
        // comment on why a subset export isn't fully self-contained anyway.
        Ok(Some(ms)) => match write_lexicon_xml_subset(std::slice::from_ref(&ms), &metadata(&crate::db::read_settings()), &[]) {
            Ok(xml) => Ok(xml_response(xml)),
            Err(e) => Ok(server_error(e)),
        },
        Ok(None) => Ok(not_found("Synset not found")),
        Err(e) => Ok(server_error(e)),
    }
}

#[get("/xml/lemma/{lemma}")]
pub async fn lemma_xml(lemma: String) -> Result<Response<Body>> {
    match resolve_lemma_synsets(&lemma) {
        Ok(synsets) if synsets.is_empty() => Ok(not_found("Lemma not found")),
        Ok(synsets) => match write_lexicon_xml_subset(&synsets, &metadata(&crate::db::read_settings()), &[]) {
            Ok(xml) => Ok(xml_response(xml)),
            Err(e) => Ok(server_error(e)),
        },
        Err(e) => Ok(server_error(e)),
    }
}

/// `ewe_lib`'s writer has no notion of a running app's settings, so translate this app's
/// `EweSettings` into the standalone metadata it expects. Language/license/version aren't
/// currently user-configurable in `EweSettings`, so these carry forward the same hardcoded
/// values the exporter has always used.
fn metadata(settings: &EweSettings) -> LexiconMetadata {
    LexiconMetadata {
        id_prefix: settings.id_prefix.clone(),
        label: settings.project_name.clone(),
        language: "en".to_string(),
        email: settings.contact_email.clone(),
        license: "https://creativecommons.org/licenses/by/4.0".to_string(),
        version: "2024".to_string(),
        url: settings.source_url.clone(),
    }
}

fn xml_response(xml: Vec<u8>) -> Response<Body> {
    Response::builder()
        .header("Content-Type", "application/xml")
        .body(Body::from(xml))
        .unwrap()
}

fn not_found(msg: &'static str) -> Response<Body> {
    Response::builder()
        .status(404)
        .body(Body::from(msg))
        .unwrap()
}

fn server_error(e: impl std::fmt::Display) -> Response<Body> {
    Response::builder()
        .status(500)
        .body(Body::from(format!("Internal server error: {}", e)))
        .unwrap()
}
