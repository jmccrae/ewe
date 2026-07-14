/// Configuration for the Downloads page: which release archives are
/// servable and how they're described, loaded from `downloads.toml`.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadsConfig {
    /// Path (relative to the working directory) that `filename`s below are
    /// resolved against when serving `/downloads/{filename}`.
    #[serde(default = "default_downloads_dir")]
    pub downloads_dir: String,
    /// Releases to list, in display order (maintainer controls ordering,
    /// e.g. newest first).
    #[serde(default)]
    pub release: Vec<DownloadRelease>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadRelease {
    /// A free-form version label, e.g. "2025".
    pub version: String,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub files: Vec<DownloadFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadFile {
    /// Must match a real file under `downloads_dir`; served at
    /// `/downloads/{filename}`. Only filenames listed here are ever
    /// served - this is a whitelist, not a directory listing.
    pub filename: String,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

fn default_downloads_dir() -> String {
    "downloads".to_string()
}

impl DownloadsConfig {
    pub fn default() -> Self {
        Self {
            downloads_dir: default_downloads_dir(),
            release: Vec::new(),
        }
    }

    pub fn load(path: &str) -> Result<DownloadsConfig, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let config: DownloadsConfig = toml::from_str(&contents)?;
        Ok(config)
    }
}
