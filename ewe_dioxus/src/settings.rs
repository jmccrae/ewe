/// Setting for running the application
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EweSettings {
    /// The database file
    pub database: String,
    /// The source to load from
    pub wordnet_source: Option<String>,
    /// The corpus database file
    #[serde(default = "default_corpus_database")]
    pub corpus_database: String,
    /// The corpus YAML file to load from
    #[serde(default)]
    pub corpus_source: Option<String>,
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
    /// If true, skip checking whether `wordnet_source`/`corpus_source` are newer
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
    /// Prefix used for synset/entry ids in XML/RDF export and in RDF-URI
    /// lookups (e.g. `oewn-00001740-n`). Deployments of a different wordnet
    /// should set this to their own project's id prefix.
    #[serde(default = "default_id_prefix")]
    pub id_prefix: String,
    /// Contact email recorded in exported WN-LMF XML's `<Lexicon>` element
    #[serde(default)]
    pub contact_email: Option<String>,
    /// Source/homepage URL recorded in exported WN-LMF XML's `<Lexicon>` element
    #[serde(default)]
    pub source_url: Option<String>,
}

fn default_lexicon_cache_mb() -> usize {
    128
}

/// Locates a file under the bundled `assets/assets/` folder (see `ASSETS_FOLDER`'s doc comment in
/// `main.rs`) in a real installed desktop app, where it isn't necessarily anywhere near a bare
/// relative path or even the running executable's own directory.
///
/// `Asset::resolve()` (manganis's own API for this) turned out not to be enough on its own -
/// confirmed live via `strace` on a real `.deb` install: it resolves to an absolute `/assets/...`
/// path (`dioxus_cli_config::base_path()`'s `DIOXUS_ASSET_ROOT` env var isn't set for a plain
/// installed binary, so it falls back to treating `/` as the bundle root), which doesn't exist at
/// all on a real filesystem - Linux's `.deb`/`.rpm` packaging splits the executable
/// (`<prefix>/bin/ewe`) from its resources (`<prefix>/lib/<AppName>/assets/`), two directories
/// that aren't siblings and don't share a fixed, predictable name for `<AppName>` we can hardcode.
/// So this tries `Asset::resolve()` first (in case a future dx release fixes it, or it's simply
/// correct on a platform this couldn't be tested on), then falls back to searching a few
/// directories up from the running executable's own location for a `lib/*/assets/assets/`
/// (Linux), `Resources/assets/assets/` (macOS `.app`), or sibling `assets/assets/` (Windows
/// `.msi`, AppImage, and a raw `dx bundle`/`dx serve` build output, which all keep the executable
/// and its resources together) - covering every `dx bundle --package-types` this project ships
/// without needing to know any bundler's internal naming choices in advance.
#[cfg(feature = "desktop")]
fn resolve_bundled_asset(relative_path: &str) -> Option<std::path::PathBuf> {
    let via_asset_resolve = crate::ASSETS_FOLDER.resolve().join(relative_path);
    if via_asset_resolve.is_file() {
        return Some(via_asset_resolve);
    }

    let exe = std::env::current_exe().ok()?;
    let mut dir = exe.parent()?.to_path_buf();
    for _ in 0..4 {
        for candidate in [
            dir.join("assets").join("assets").join(relative_path),
            dir.join("Resources")
                .join("assets")
                .join("assets")
                .join(relative_path),
        ] {
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        if let Ok(entries) = std::fs::read_dir(dir.join("lib")) {
            for entry in entries.flatten() {
                let candidate = entry.path().join("assets").join("assets").join(relative_path);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
        match dir.parent() {
            Some(parent) => dir = parent.to_path_buf(),
            None => break,
        }
    }
    None
}

/// See `resolve_bundled_asset`'s doc comment.
#[cfg(feature = "desktop")]
fn default_logo() -> String {
    resolve_bundled_asset("gwa.svg")
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "assets/assets/gwa.svg".to_string())
}

#[cfg(not(feature = "desktop"))]
fn default_logo() -> String {
    "assets/gwa.svg".to_string()
}

fn default_corpus_database() -> String {
    "corpus.db".to_string()
}

/// See `resolve_bundled_asset`'s doc comment.
#[cfg(feature = "desktop")]
fn default_theme() -> String {
    resolve_bundled_asset("styling/theme.css")
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "assets/assets/styling/theme.css".to_string())
}

#[cfg(not(feature = "desktop"))]
fn default_theme() -> String {
    "assets/styling/theme.css".to_string()
}

fn default_project_name() -> String {
    "EWE Wordnet Editor".to_string()
}

fn default_tagline() -> String {
    "A free, open lexical database".to_string()
}

fn default_intro() -> String {
    r#"<p>
        Search for a word above to see its meanings, synonyms, and how it
        relates to other words in this Wordnet.
    </p>"#
        .to_string()
}

fn default_footer() -> String {
    r#"<p class="footer2">
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

fn default_id_prefix() -> String {
    "oewn".to_string()
}

impl EweSettings {
    pub fn default() -> Self {
        Self {
            database: "wordnet.db".to_string(),
            wordnet_source: None,
            corpus_database: default_corpus_database(),
            corpus_source: None,
            logo: default_logo(),
            project_name: default_project_name(),
            tagline: default_tagline(),
            intro: default_intro(),
            footer: default_footer(),
            theme: default_theme(),
            disable_auto_reload: false,
            lexicon_cache_mb: default_lexicon_cache_mb(),
            id_prefix: default_id_prefix(),
            contact_email: None,
            source_url: None,
        }
    }

    /// Loads settings from `path`, resolving every path-valued field (`database`,
    /// `wordnet_source`, `corpus_database`, `corpus_source`, `logo`, `theme`) relative to
    /// `path`'s own directory rather than the process's working directory. This lets a
    /// `settings.toml` be dropped into (and moved between) any project folder - e.g. a Wordnet
    /// checkout picked via the desktop setup screen's folder dialog (see
    /// `backend::setup::configure_wordnet_source`) - and have its relative paths keep pointing
    /// at its own sibling files/folders no matter where the server process itself was launched
    /// from. A bare filename like `"settings.toml"` (no directory component) resolves to an
    /// empty base, leaving paths unchanged - so this is a no-op for the common case of a
    /// `settings.toml` sitting directly in the process's working directory.
    pub fn load(path: &str) -> Result<EweSettings, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let mut settings: EweSettings = toml::from_str(&contents)?;

        let base = std::path::Path::new(path)
            .parent()
            .unwrap_or_else(|| std::path::Path::new(""));
        let resolve = |value: &str| -> String {
            if base.as_os_str().is_empty() {
                value.to_string()
            } else {
                base.join(value).to_string_lossy().to_string()
            }
        };

        settings.database = resolve(&settings.database);
        settings.corpus_database = resolve(&settings.corpus_database);
        settings.logo = resolve(&settings.logo);
        settings.theme = resolve(&settings.theme);
        settings.wordnet_source = settings.wordnet_source.as_deref().map(|s| resolve(s));
        settings.corpus_source = settings.corpus_source.as_deref().map(|s| resolve(s));

        Ok(settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A directory under the system temp dir, unique per test run, cleaned up on drop.
    struct ScratchDir(std::path::PathBuf);

    impl ScratchDir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "ewe-settings-test-{name}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(&dir).unwrap();
            ScratchDir(dir)
        }
    }

    impl Drop for ScratchDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn load_resolves_relative_paths_against_settings_toml_directory() {
        let scratch = ScratchDir::new("resolve");
        let settings_path = scratch.0.join("settings.toml");
        std::fs::write(
            &settings_path,
            r#"
                database = "wordnet.db"
                wordnet_source = "."
                corpus_database = "semcor.db"
                corpus_source = "../semcor.yaml"
                logo = "src/assets/english.svg"
            "#,
        )
        .unwrap();

        let settings = EweSettings::load(settings_path.to_str().unwrap()).unwrap();

        assert_eq!(settings.database, scratch.0.join("wordnet.db").to_string_lossy());
        assert_eq!(
            settings.wordnet_source.as_deref(),
            Some(scratch.0.join(".").to_string_lossy().to_string()).as_deref()
        );
        assert_eq!(settings.corpus_database, scratch.0.join("semcor.db").to_string_lossy());
        assert_eq!(
            settings.corpus_source.as_deref(),
            Some(scratch.0.join("../semcor.yaml").to_string_lossy().to_string()).as_deref()
        );
        assert_eq!(settings.logo, scratch.0.join("src/assets/english.svg").to_string_lossy());
    }

    #[test]
    fn bare_filename_has_empty_parent() {
        // `load`'s path resolution special-cases an empty `parent()` to leave path fields
        // untouched rather than joining against it - this is what keeps the common cwd-relative
        // deployment (`EweSettings::load("settings.toml")`, no directory component) resolving
        // exactly as it did before path resolution was added.
        assert_eq!(
            std::path::Path::new("settings.toml").parent(),
            Some(std::path::Path::new(""))
        );
    }

    #[test]
    fn load_leaves_absolute_paths_unchanged() {
        let scratch = ScratchDir::new("absolute");
        let settings_path = scratch.0.join("settings.toml");
        std::fs::write(
            &settings_path,
            r#"database = "/absolute/path/wordnet.db""#,
        )
        .unwrap();

        let settings = EweSettings::load(settings_path.to_str().unwrap()).unwrap();
        assert_eq!(settings.database, "/absolute/path/wordnet.db");
    }
}
