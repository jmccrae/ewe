//! Serves the favicon at `/favicon` (and `/favicon.ico`): the file named by `settings.toml`'s
//! `favicon` key, or the bundled `assets/favicon.ico` if that's unset or can't be read.
//!
//! Unlike the `[theme]` overrides, this needs no client-side fetch: the page's `<link rel="icon">`
//! always points at the same URL (see `App2` in `main.rs`) and the server decides what's behind
//! it, so the right icon shows from first paint without a per-project build flag. Settings are
//! read on every request, so swapping the file or repointing `favicon` takes effect without a
//! restart, like `logo`.

use crate::dioxus_fullstack::{body::Body, http::Response};

/// The fallback icon, embedded in the binary so it needs no path resolution at runtime.
const DEFAULT_FAVICON: &[u8] = include_bytes!("../../assets/favicon.ico");

/// Short enough that a changed icon shows up within the hour, though browsers keep their own
/// favicon cache too, so a hard refresh may still be needed to see a change right away.
const CACHE_CONTROL: &str = "public, max-age=3600";

/// A plain axum handler (not a `#[get(...)]` server function), for the same reason as
/// `backend::downloads::download_file`: dioxus-server rewrites server-function responses to
/// browser-looking requests into a 302 back to the Referer.
pub async fn favicon() -> Response<Body> {
    let path = crate::db::read_settings().favicon.clone();
    favicon_response(path.as_deref())
}

fn favicon_response(path: Option<&str>) -> Response<Body> {
    let (bytes, content_type) = match path.map(|p| (std::fs::read(p), p)) {
        Some((Ok(bytes), p)) => (bytes, content_type(p)),
        Some((Err(e), p)) => {
            eprintln!("Could not read favicon at {p}, using the default: {e}");
            (DEFAULT_FAVICON.to_vec(), "image/x-icon")
        }
        None => (DEFAULT_FAVICON.to_vec(), "image/x-icon"),
    };
    Response::builder()
        .header("Content-Type", content_type)
        .header("Cache-Control", CACHE_CONTROL)
        .body(Body::from(bytes))
        .unwrap()
}

fn content_type(path: &str) -> &'static str {
    match std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("svg") => "image/svg+xml",
        Some("gif") => "image/gif",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        _ => "image/x-icon",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header<'a>(response: &'a Response<Body>, name: &str) -> &'a str {
        response.headers().get(name).unwrap().to_str().unwrap()
    }

    #[test]
    fn unset_serves_the_bundled_default() {
        let response = favicon_response(None);
        assert_eq!(response.status(), 200);
        assert_eq!(header(&response, "Content-Type"), "image/x-icon");
        assert_eq!(header(&response, "Cache-Control"), CACHE_CONTROL);
    }

    #[test]
    fn missing_file_falls_back_to_the_default() {
        let response = favicon_response(Some("/nonexistent/favicon.png"));
        assert_eq!(response.status(), 200);
        assert_eq!(header(&response, "Content-Type"), "image/x-icon");
    }

    #[test]
    fn configured_file_is_served_with_its_type() {
        let path = std::env::temp_dir().join(format!("ewe-favicon-test-{}.svg", std::process::id()));
        std::fs::write(&path, "<svg/>").unwrap();
        let response = favicon_response(path.to_str());
        std::fs::remove_file(&path).unwrap();
        assert_eq!(header(&response, "Content-Type"), "image/svg+xml");
    }

    #[test]
    fn content_type_by_extension() {
        assert_eq!(content_type("icons/site.PNG"), "image/png");
        assert_eq!(content_type("favicon.ico"), "image/x-icon");
        assert_eq!(content_type("no-extension"), "image/x-icon");
    }
}
