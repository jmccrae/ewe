/// Setting for running the application
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EweSettings {
    /// The database file
    pub database: String,
    /// The source to load from
    pub wordnet_source: Option<String>,
    /// The Semcor corpus database file
    #[serde(default = "default_semcor_database")]
    pub semcor_database: String,
    /// The Semcor corpus YAML file to load from
    #[serde(default)]
    pub semcor_source: Option<String>,
    /// Path (relative to the working directory) of the logo image, served at `/logo`
    #[serde(default = "default_logo")]
    pub logo: String,
    /// The project name shown in the header
    #[serde(default = "default_project_name")]
    pub project_name: String,
    /// Short tagline shown centered on the home page, below the search box
    #[serde(default = "default_tagline")]
    pub tagline: String,
    /// Introduction HTML shown centered on the home page, below the tagline
    #[serde(default = "default_intro")]
    pub intro: String,
    /// Footer HTML, rendered as-is beneath the main content
    #[serde(default = "default_footer")]
    pub footer: String,
    /// Path (relative to the working directory) of the theme stylesheet
    /// (colours and fonts), served at `/theme.css`
    #[serde(default = "default_theme")]
    pub theme: String,
    /// If true, skip checking whether `wordnet_source`/`semcor_source` are newer
    /// than the existing databases on startup, so the databases are never
    /// automatically rebuilt (they're still built if missing). Useful to avoid
    /// a slow source scan on startup with very large sources such as NameNet.
    #[serde(default)]
    pub disable_auto_reload: bool,
    /// Bounds the lexicon database's in-memory page cache. redb (the
    /// database engine) defaults to a 1GiB cache regardless of the database
    /// file's actual size, which is wasteful on memory-constrained servers.
    #[serde(default = "default_lexicon_cache_mb")]
    pub lexicon_cache_mb: usize,
}

fn default_lexicon_cache_mb() -> usize {
    128
}

fn default_logo() -> String {
    "assets/english.svg".to_string()
}

fn default_semcor_database() -> String {
    "semcor.db".to_string()
}

fn default_theme() -> String {
    "assets/styling/theme.css".to_string()
}

fn default_project_name() -> String {
    "Open English Wordnet".to_string()
}

fn default_tagline() -> String {
    "The free, open lexical database of English".to_string()
}

fn default_intro() -> String {
    r#"<p>
        Search for a word above to see its meanings, synonyms, and how it
        relates to other words in the Open English Wordnet.
    </p>"#
        .to_string()
}

fn default_footer() -> String {
    r#"<p class="footer1">
        Open English Wordnet is derived from <a href="http://wordnet.princeton.edu/">Princeton WordNet</a>
        by the Open English Wordnet Community and released under the
        <a href="https://creativecommons.org/licenses/by/4.0/">Creative Commons Attribution (CC-BY) 4.0 License</a>.
        <a href="https://globalwordnet.github.io/gwadoc/">Further information about Wordnet</a>.
        We welcome any corrections, improvements or other contributions at
        <a href="http://github.com/globalwordnet/english-wordnet">GitHub</a>.
        A full list of contributors is available on
        <a href="https://github.com/globalwordnet/english-wordnet/blob/master/README.md">GitHub</a>.
    </p>
    <p class="footer2">
        This interface was created by <a href="http://john.mccr.ae/">John P. McCrae</a> at the
        <a href="https://dsi.nuigalway.ie/">Data Science Institute</a>,
        <a href="http://www.universityofgalway.ie">University of Galway</a>
        (<a href="http://github.com/jmccrae/ewe">GitHub</a>). Development of this interface is supported by
        <a href="https://www.sfi.ie/">Science Foundation Ireland</a> as part of the
        <a href="https://www.insight-centre.org/">Insight Centre for Data Analytics</a>
        and the European Union's Horizon 2020 research and innovation programme under grant agreement No 731015
        (<a href="http://elex.is/">ELEXIS</a>).
    </p>"#
        .to_string()
}

impl EweSettings {
    pub fn default() -> Self {
        Self {
            database: "wordnet.db".to_string(),
            wordnet_source: None,
            semcor_database: default_semcor_database(),
            semcor_source: None,
            logo: default_logo(),
            project_name: default_project_name(),
            tagline: default_tagline(),
            intro: default_intro(),
            footer: default_footer(),
            theme: default_theme(),
            disable_auto_reload: false,
            lexicon_cache_mb: default_lexicon_cache_mb(),
        }
    }

    pub fn load(path: &str) -> Result<EweSettings, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let settings: EweSettings = toml::from_str(&contents)?;
        Ok(settings)
    }
}
