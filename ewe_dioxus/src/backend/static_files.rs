//! Serves files whose path is configured in `settings.toml` (the logo and
//! the theme stylesheet), read from disk at request time so they can be
//! swapped without rebuilding the app.

use crate::dioxus_fullstack::{body::Body, http::Response, HeaderMap};
use dioxus::prelude::*;

#[get("/logo", headers : HeaderMap)]
pub async fn logo() -> Result<Response<Body>> {
    serve_settings_file(&crate::SETTINGS.get().logo, headers)
}

#[get("/theme.css", headers : HeaderMap)]
pub async fn theme_css() -> Result<Response<Body>> {
    serve_settings_file(&crate::SETTINGS.get().theme, headers)
}

/// Neither route was sending any caching headers, so the browser re-fetched
/// the logo/theme from scratch (a full request, server-side disk read, and
/// response body) on every single page load - the "noticeable delay"
/// rendering the logo. An `ETag` derived from the file's size and mtime lets
/// the browser skip the download entirely (a bodyless 304) on every request
/// after the first, while still picking up a swapped-in file (the whole
/// point of reading it from disk per-request rather than bundling it as a
/// static asset) as soon as its mtime changes.
fn serve_settings_file(path: &str, headers: HeaderMap) -> Result<Response<Body>> {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(e) => {
            return Ok(Response::builder()
                .status(404)
                .body(Body::from(format!("File not found at {}: {}", path, e)))
                .unwrap())
        }
    };
    let etag = format!(
        "\"{}-{}\"",
        metadata.len(),
        metadata
            .modified()
            .ok()
            .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|since_epoch| since_epoch.as_secs())
            .unwrap_or(0)
    );

    if headers.get("If-None-Match").and_then(|v| v.to_str().ok()) == Some(etag.as_str()) {
        return Ok(Response::builder()
            .status(304)
            .header("Cache-Control", "public, max-age=3600, must-revalidate")
            .header("ETag", etag)
            .body(Body::empty())
            .unwrap());
    }

    match std::fs::read(path) {
        Ok(bytes) => Ok(Response::builder()
            .header("Content-Type", content_type(path))
            .header("Cache-Control", "public, max-age=3600, must-revalidate")
            .header("ETag", etag)
            .body(Body::from(bytes))
            .unwrap()),
        Err(e) => Ok(Response::builder()
            .status(404)
            .body(Body::from(format!("File not found at {}: {}", path, e)))
            .unwrap()),
    }
}

fn content_type(path: &str) -> &'static str {
    match std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("ico") => "image/x-icon",
        Some("css") => "text/css",
        _ => "application/octet-stream",
    }
}
