//! Serves the hand-written OpenAPI spec for the `/api/*` JSON endpoints
//! (see `../../openapi.yaml`), plus a Swagger UI page to browse it.
//!
//! The spec is embedded at compile time with `include_str!` rather than
//! read from a configurable path like the logo/theme in `static_files.rs`:
//! it's a source artifact that describes this binary's own routes, not a
//! deployment-time asset.

use crate::dioxus_fullstack::{body::Body, http::Response};
use dioxus::prelude::*;

const OPENAPI_YAML: &str = include_str!("../../openapi.yaml");

#[get("/api/openapi.yaml")]
pub async fn openapi_spec() -> Result<Response<Body>> {
    Ok(Response::builder()
        .header("Content-Type", "application/yaml")
        .body(Body::from(OPENAPI_YAML))
        .unwrap())
}

#[get("/api/docs")]
pub async fn api_docs() -> Result<Response<Body>> {
    Ok(Response::builder()
        .header("Content-Type", "text/html; charset=utf-8")
        .body(Body::from(API_DOCS_HTML))
        .unwrap())
}

const API_DOCS_HTML: &str = r##"<!doctype html>
<html>
<head>
    <meta charset="utf-8" />
    <title>Open English WordNet API docs</title>
    <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css" />
</head>
<body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
    <script>
        window.onload = () => {
            SwaggerUIBundle({
                url: "/api/openapi.yaml",
                dom_id: "#swagger-ui",
            });
        };
    </script>
</body>
</html>
"##;
