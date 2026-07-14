//! Serves the Downloads page's config (`/api/downloads`) and the release
//! files it lists (`/downloads/{filename}`).

use crate::dioxus_fullstack::{body::Body, http::Response};
use dioxus::prelude::*;

/// A `DownloadFile` plus its size on disk, computed fresh on every request
/// so it can't go stale relative to `downloads_dir`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DownloadFileInfo {
    pub filename: String,
    pub format: Option<String>,
    pub description: Option<String>,
    /// Absent if the file is listed in `downloads.toml` but missing on disk.
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DownloadReleaseInfo {
    pub version: String,
    pub date: Option<String>,
    pub description: Option<String>,
    pub files: Vec<DownloadFileInfo>,
}

#[get("/api/downloads")]
pub async fn get_downloads() -> Result<Vec<DownloadReleaseInfo>> {
    let config = crate::DOWNLOADS.get();
    Ok(config
        .release
        .iter()
        .map(|release| DownloadReleaseInfo {
            version: release.version.clone(),
            date: release.date.clone(),
            description: release.description.clone(),
            files: release
                .files
                .iter()
                .map(|file| {
                    let path = std::path::Path::new(&config.downloads_dir).join(&file.filename);
                    DownloadFileInfo {
                        filename: file.filename.clone(),
                        format: file.format.clone(),
                        description: file.description.clone(),
                        size_bytes: std::fs::metadata(path).ok().map(|m| m.len()),
                    }
                })
                .collect(),
        })
        .collect())
}

#[get("/downloads/{filename}")]
pub async fn download_file(filename: String) -> Result<Response<Body>> {
    let config = crate::DOWNLOADS.get();

    // Whitelist check: only ever serve a filename that's explicitly listed
    // in downloads.toml, never an arbitrary path under downloads_dir. This
    // is what rules out path traversal, not sanitizing `filename` itself.
    let known = config
        .release
        .iter()
        .flat_map(|release| release.files.iter())
        .any(|file| file.filename == filename);
    if !known {
        return Ok(Response::builder()
            .status(404)
            .body(Body::from(format!("No such download: {}", filename)))
            .unwrap());
    }

    let path = std::path::Path::new(&config.downloads_dir).join(&filename);
    match std::fs::read(&path) {
        Ok(bytes) => Ok(Response::builder()
            .header("Content-Type", content_type(&filename))
            .header(
                "Content-Disposition",
                format!("attachment; filename=\"{}\"", filename),
            )
            .body(Body::from(bytes))
            .unwrap()),
        Err(e) => Ok(Response::builder()
            .status(404)
            .body(Body::from(format!(
                "File not found at {}: {}",
                path.display(),
                e
            )))
            .unwrap()),
    }
}

#[cfg(feature = "server")]
fn content_type(filename: &str) -> &'static str {
    match std::path::Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("zip") => "application/zip",
        Some("gz") => "application/gzip",
        Some("xml") => "application/xml",
        Some("ttl") => "text/turtle",
        Some("json") => "application/json",
        _ => "application/octet-stream",
    }
}
