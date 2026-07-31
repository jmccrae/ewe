//! `sitemap.xml`/`robots.txt` for search-engine crawlers. Pure crawler/browser-facing HTTP
//! endpoints, never called isomorphically from Dioxus view code - same category as `rdf.rs`/
//! `xml.rs`, so gated the same way (module-level `#[cfg(feature = "server")]` in `mod.rs`,
//! plain `#[get(...)]` rather than `cfg_attr(not(desktop), ...)`).

use crate::dioxus_fullstack::{body::Body, http::Response};
use dioxus::prelude::*;
use ewe_lib::wordnet::{Lexicon, SynsetId};
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;

/// Stays comfortably under the sitemap protocol's hard cap of 50,000 <url> entries
/// (and 50MB uncompressed) per file - see sitemaps.org.
const SITEMAP_CHUNK_SIZE: usize = 45_000;

#[get("/sitemap.xml")]
pub async fn sitemap_index() -> Result<Response<Body>> {
    let base_url = match crate::db::read_settings().base_url.clone() {
        Some(base_url) => base_url,
        None => {
            return Ok(not_found(
                "Sitemap not configured: set `base_url` in settings.toml",
            ))
        }
    };
    let lexicon = match crate::db::read_lexicon() {
        Ok(lexicon) => lexicon,
        Err(_) => return Ok(unavailable("Wordnet not loaded")),
    };
    let n = lexicon.n_synsets()?;
    drop(lexicon);
    let n_chunks = n.div_ceil(SITEMAP_CHUNK_SIZE).max(1);
    Ok(xml_response(gen_sitemap_index_xml(&base_url, n_chunks)?))
}

fn gen_sitemap_index_xml(base_url: &str, n_chunks: usize) -> Result<Vec<u8>> {
    let mut writer = Writer::new(Vec::new());
    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;
    let mut sitemapindex = BytesStart::new("sitemapindex");
    sitemapindex.push_attribute(("xmlns", "http://www.sitemaps.org/schemas/sitemap/0.9"));
    writer.write_event(Event::Start(sitemapindex))?;
    for i in 0..n_chunks {
        writer.write_event(Event::Start(BytesStart::new("sitemap")))?;
        writer.write_event(Event::Start(BytesStart::new("loc")))?;
        writer.write_event(Event::Text(BytesText::new(&join_url(
            base_url,
            &format!("sitemap/{}", i),
        ))))?;
        writer.write_event(Event::End(BytesEnd::new("loc")))?;
        writer.write_event(Event::End(BytesEnd::new("sitemap")))?;
    }
    writer.write_event(Event::End(BytesEnd::new("sitemapindex")))?;
    Ok(writer.into_inner())
}

/// `n` is 0-indexed. Out-of-range `n` (past the last chunk) isn't an error - it just yields no
/// synsets, so still returns a valid, empty `<urlset>` with 200 rather than 404 (protects a
/// crawler that cached an older, larger chunk count from an index that later shrinks).
#[get("/sitemap/{n}")]
pub async fn sitemap_chunk(n: String) -> Result<Response<Body>> {
    let Ok(n) = n.parse::<usize>() else {
        return Ok(not_found("Invalid sitemap chunk"));
    };
    let base_url = match crate::db::read_settings().base_url.clone() {
        Some(base_url) => base_url,
        None => {
            return Ok(not_found(
                "Sitemap not configured: set `base_url` in settings.toml",
            ))
        }
    };
    let lexicon = match crate::db::read_lexicon() {
        Ok(lexicon) => lexicon,
        Err(_) => return Ok(unavailable("Wordnet not loaded")),
    };
    let mut ids = Vec::new();
    for entry in lexicon
        .synsets()?
        .skip(n * SITEMAP_CHUNK_SIZE)
        .take(SITEMAP_CHUNK_SIZE)
    {
        let (id, _) = entry?;
        ids.push(id);
    }
    drop(lexicon);
    Ok(xml_response(gen_sitemap_chunk_xml(&base_url, &ids)?))
}

fn gen_sitemap_chunk_xml(base_url: &str, ids: &[SynsetId]) -> Result<Vec<u8>> {
    let mut writer = Writer::new(Vec::new());
    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;
    let mut urlset = BytesStart::new("urlset");
    urlset.push_attribute(("xmlns", "http://www.sitemaps.org/schemas/sitemap/0.9"));
    writer.write_event(Event::Start(urlset))?;
    for id in ids {
        writer.write_event(Event::Start(BytesStart::new("url")))?;
        writer.write_event(Event::Start(BytesStart::new("loc")))?;
        writer.write_event(Event::Text(BytesText::new(&join_url(
            base_url,
            &format!("view/synset/{}", id.as_str()),
        ))))?;
        writer.write_event(Event::End(BytesEnd::new("loc")))?;
        writer.write_event(Event::End(BytesEnd::new("url")))?;
    }
    writer.write_event(Event::End(BytesEnd::new("urlset")))?;
    Ok(writer.into_inner())
}

/// Disallows every non-canonical/machine-format route (content-negotiation entry points and
/// raw RDF/XML/Turtle/JSON exports) so crawlers only index `/view/synset/{id}`, avoiding
/// duplicate-content signals across the several representations each synset has.
/// `/view/lemma/`, `/history`, `/downloads` are deliberately left unmentioned (default-allow) -
/// they're real, canonical, crawlable HTML pages.
#[get("/robots.txt")]
pub async fn robots_txt() -> Result<Response<Body>> {
    let settings = crate::db::read_settings();
    let mut body = String::from(
        "User-agent: *\n\
         Disallow: /id/\n\
         Disallow: /synset/\n\
         Disallow: /lemma/\n\
         Disallow: /rdf/\n\
         Disallow: /xml/\n\
         Disallow: /ttl/\n\
         Disallow: /api/\n",
    );
    if let Some(base_url) = &settings.base_url {
        body.push_str(&format!(
            "\nSitemap: {}\n",
            join_url(base_url, "sitemap.xml")
        ));
    }
    Ok(Response::builder()
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(Body::from(body))
        .unwrap())
}

/// Joins a configured base URL (with or without a trailing slash) and a path (with or
/// without a leading slash) with exactly one `/` between them.
fn join_url(base: &str, path: &str) -> String {
    format!(
        "{}/{}",
        base.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
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

fn unavailable(msg: &'static str) -> Response<Body> {
    Response::builder()
        .status(503)
        .body(Body::from(msg))
        .unwrap()
}
