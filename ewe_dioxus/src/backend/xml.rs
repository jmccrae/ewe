//! WN-LMF XML export web routes.
//!
//! Each export is a self-contained `LexicalResource`/`Lexicon` covering just the requested
//! synset (or all synsets a lemma has a sense in). The actual XML generation lives in
//! `ewe_lib::wordnet::xml` (shared with `ewe_cli`'s whole-lexicon `export xml` command); this
//! module is just the web-route glue that resolves a synset/lemma against this app's lexicon and
//! settings, then hands off to the shared writer. See that module's doc comment for the
//! self-containment caveat a partial export like this implies.

use crate::backend::rdf::{resolve_deprecated_id, resolve_lemma_synsets, resolve_synset};
use crate::dioxus_fullstack::response::IntoResponse;
use crate::dioxus_fullstack::{body::Body, http::Response, Redirect};
use crate::settings::EweSettings;
use dioxus::prelude::*;
use ewe_lib::wordnet::xml::{write_lexicon_xml_subset_report, XmlOutput};
use ewe_lib::wordnet::{LexiconMetadata, SynsetId};

/// `?version=1.5` targets WN-LMF 1.5 (default 1.4). Data the version cannot represent is dropped,
/// and described in the `X-Ewe-Warnings` response header.
#[get("/xml/synset/{id}?{version}")]
pub async fn synset_xml(id: String, version: Option<String>) -> Result<Response<Body>> {
    let id = SynsetId::new_owned(id);
    if let Some(new_id) = resolve_deprecated_id(&id)? {
        return Ok(Redirect::permanent(&format!("/xml/synset/{}", new_id.as_str())).into_response());
    }
    let metadata = match metadata(&crate::db::read_settings(), version.as_deref()) {
        Ok(m) => m,
        Err(msg) => return Ok(bad_request(msg)),
    };
    match resolve_synset(&id) {
        // No frame table for a single-synset fragment - see write_lexicon_xml_subset's doc
        // comment on why a subset export isn't fully self-contained anyway.
        Ok(Some(ms)) => match write_lexicon_xml_subset_report(std::slice::from_ref(&ms), &metadata, &[], &Default::default()) {
            Ok(out) => Ok(xml_response(out)),
            Err(e) => Ok(server_error(e)),
        },
        Ok(None) => Ok(not_found("Synset not found")),
        Err(e) => Ok(server_error(e)),
    }
}

#[get("/xml/lemma/{lemma}?{version}")]
pub async fn lemma_xml(lemma: String, version: Option<String>) -> Result<Response<Body>> {
    let metadata = match metadata(&crate::db::read_settings(), version.as_deref()) {
        Ok(m) => m,
        Err(msg) => return Ok(bad_request(msg)),
    };
    match resolve_lemma_synsets(&lemma) {
        Ok(synsets) if synsets.is_empty() => Ok(not_found("Lemma not found")),
        Ok(synsets) => match write_lexicon_xml_subset_report(&synsets, &metadata, &[], &Default::default()) {
            Ok(out) => Ok(xml_response(out)),
            Err(e) => Ok(server_error(e)),
        },
        Err(e) => Ok(server_error(e)),
    }
}

/// `ewe_lib`'s writer has no notion of a running app's settings, so translate this app's
/// `EweSettings` into the standalone metadata it expects. License/version aren't currently
/// user-configurable in `EweSettings`, so these carry forward the same hardcoded values the
/// exporter has always used.
fn metadata(settings: &EweSettings, version: Option<&str>) -> Result<LexiconMetadata, String> {
    let lmf_version = match version {
        Some(v) => v.parse().map_err(|e: String| e)?,
        None => Default::default(),
    };
    Ok(LexiconMetadata {
        id_prefix: settings.id_prefix.clone(),
        label: settings.project_name.clone(),
        language: settings.language.clone(),
        email: settings.contact_email.clone(),
        license: "https://creativecommons.org/licenses/by/4.0".to_string(),
        version: "2024".to_string(),
        url: settings.source_url.clone(),
        // Not configurable either; absent means the WN-LMF default of 1.0.
        confidence: None,
        lmf_version,
    })
}

fn xml_response(out: XmlOutput) -> Response<Body> {
    let mut builder = Response::builder().header("Content-Type", "application/xml");
    if !out.warnings.is_empty() {
        // Header values must be visible ASCII.
        let joined: String = out.warnings.join("; ").chars()
            .map(|c| if c.is_ascii() && !c.is_ascii_control() { c } else { '?' })
            .collect();
        builder = builder.header("X-Ewe-Warnings", joined);
    }
    builder.body(Body::from(out.bytes)).unwrap()
}

fn bad_request(msg: String) -> Response<Body> {
    Response::builder().status(400).body(Body::from(msg)).unwrap()
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
