/// Setting for running the application
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EweSettings {
    /// The database file
    pub database: String,
    /// The source to load from
    pub wordnet_source: Option<String>,
}

impl EweSettings {
    pub fn default() -> Self {
        Self {
            database: "wordnet.db".to_string(),
            wordnet_source: None,
        }
    }

    pub fn load(path: &str) -> Result<EweSettings, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let settings: EweSettings = toml::from_str(&contents)?;
        Ok(settings)
    }
}
