//! Serves files whose path is configured in `settings.toml` (the logo and
//! the theme stylesheet), read from disk at request time so they can be
//! swapped without rebuilding the app.

use crate::dioxus_fullstack::{body::Body, http::Response};
use dioxus::prelude::*;

#[get("/logo")]
pub async fn logo() -> Result<Response<Body>> {
    serve_settings_file(&crate::SETTINGS.get().logo)
}

#[get("/theme.css")]
pub async fn theme_css() -> Result<Response<Body>> {
    serve_settings_file(&crate::SETTINGS.get().theme)
}

fn serve_settings_file(path: &str) -> Result<Response<Body>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Response::builder()
            .header("Content-Type", content_type(path))
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
