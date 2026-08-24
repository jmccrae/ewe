use dioxus::prelude::*;
#[allow(unused_imports)]
use ewe_lib::wordnet::{Lexicon, MemberSynset, SynsetId};
use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use std::collections::BTreeSet;
#[cfg(any(feature = "server", feature = "desktop"))]
use crate::db::read_lexicon;

/// CSS custom-property overrides applied on top of the bundled `assets/styling/theme.css`
/// defaults, via `document.documentElement.style.setProperty(...)` at runtime (see
/// `views::wn_layout::WNLayout`) rather than by swapping the whole stylesheet. Every field is
/// optional; `None` means "use the compiled-in default" via the normal CSS cascade - this struct
/// never needs to duplicate those default values itself. Field names map onto
/// `assets/styling/theme.css`'s "Roles" custom properties (e.g. `primary` -> `--color-primary`,
/// `text_secondary` -> `--color-text-secondary`) and its font custom properties (`font_body` ->
/// `--font-body`, etc.).
///
/// Lives here (in `backend::api`, alongside `Branding` which embeds it) rather than in
/// `settings` - `EweSettings::theme` uses this same type, but `settings` is gated to `server`/
/// `desktop` while `backend::api` compiles unconditionally (including into the WASM client on a
/// `web` build, since `Branding` is fetched through a server function view code on every target
/// calls isomorphically), so the type has to live on the side both builds can see.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ThemeOverrides {
    #[serde(default)]
    pub primary: Option<String>,
    #[serde(default)]
    pub accent: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub text_secondary: Option<String>,
    #[serde(default)]
    pub text_muted: Option<String>,
    #[serde(default)]
    pub text_dim: Option<String>,
    #[serde(default)]
    pub text_faint: Option<String>,
    #[serde(default)]
    pub text_mute: Option<String>,
    #[serde(default)]
    pub text_strong: Option<String>,
    #[serde(default)]
    pub text_on_dark: Option<String>,
    #[serde(default)]
    pub border: Option<String>,
    #[serde(default)]
    pub border_light: Option<String>,
    #[serde(default)]
    pub surface_light: Option<String>,
    #[serde(default)]
    pub surface_hover: Option<String>,
    #[serde(default)]
    pub surface_dark: Option<String>,
    #[serde(default)]
    pub font_body: Option<String>,
    #[serde(default)]
    pub font_heading: Option<String>,
    #[serde(default)]
    pub font_heading_weight: Option<String>,
    #[serde(default)]
    pub font_mono: Option<String>,
}

impl ThemeOverrides {
    /// The full set of overridable color properties as `(field name, value)` pairs, for
    /// `validate`'s loop and any other code that needs to walk every field generically rather
    /// than naming each one - keeps that code from silently missing a field added here later.
    fn color_fields(&self) -> [(&'static str, &Option<String>); 15] {
        [
            ("theme.primary", &self.primary),
            ("theme.accent", &self.accent),
            ("theme.text", &self.text),
            ("theme.text_secondary", &self.text_secondary),
            ("theme.text_muted", &self.text_muted),
            ("theme.text_dim", &self.text_dim),
            ("theme.text_faint", &self.text_faint),
            ("theme.text_mute", &self.text_mute),
            ("theme.text_strong", &self.text_strong),
            ("theme.text_on_dark", &self.text_on_dark),
            ("theme.border", &self.border),
            ("theme.border_light", &self.border_light),
            ("theme.surface_light", &self.surface_light),
            ("theme.surface_hover", &self.surface_hover),
            ("theme.surface_dark", &self.surface_dark),
        ]
    }

    fn font_fields(&self) -> [(&'static str, &Option<String>); 4] {
        [
            ("theme.font_body", &self.font_body),
            ("theme.font_heading", &self.font_heading),
            ("theme.font_heading_weight", &self.font_heading_weight),
            ("theme.font_mono", &self.font_mono),
        ]
    }

    /// Checks every set field against `is_valid_hex_color`/`is_valid_font_value`, returning a
    /// descriptive error naming the first offending field. Called from `EweSettings::load` (via
    /// the `settings` module's `pub use` of this type) so a malformed `settings.toml` fails
    /// loudly at load time rather than silently rendering half-styled (these are
    /// deployment-author-controlled config, not end-user input, and every valid value ends up
    /// interpolated into JS run via `document::eval` - see `views::wn_layout::build_theme_eval_js`
    /// - so a tight allowlist here is worth enforcing up front).
    pub fn validate(&self) -> std::result::Result<(), String> {
        for (name, value) in self.color_fields() {
            if let Some(v) = value {
                if !is_valid_hex_color(v) {
                    return Err(format!(
                        "{name} = \"{v}\" is not a valid CSS hex color (expected e.g. \"#002868\")"
                    ));
                }
            }
        }
        for (name, value) in self.font_fields() {
            if let Some(v) = value {
                if !is_valid_font_value(v) {
                    return Err(format!(
                        "{name} = \"{v}\" contains a character not allowed in a theme font value"
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Matches a CSS hex color: `#` followed by 3, 4, 6, or 8 hex digits (matching CSS's own
/// shorthand/alpha forms). Deliberately stricter than valid CSS overall (no named colors, no
/// `rgb()`/`hsl()`) since every real-world override so far is a hex color, and a narrow allowlist
/// is easiest to reason about as safe to interpolate into `document::eval`'d JS.
fn is_valid_hex_color(s: &str) -> bool {
    let hex = s.strip_prefix('#').unwrap_or_default();
    s.starts_with('#') && matches!(hex.len(), 3 | 4 | 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit())
}

/// Rejects only the characters that could break out of the double-quoted JS string literal a
/// theme font value is later interpolated into (see `views::wn_layout::build_theme_eval_js`), or
/// inject extra CSS/JS: `"`, `\`, `<`, `>`, `;`, and control characters. Single quotes are allowed
/// - `'Times New Roman'` is a normal, legitimate CSS font-stack style, and harmless inside a
/// double-quoted JS string.
fn is_valid_font_value(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii() && !c.is_control() && !"\"\\<>;".contains(c))
}

#[cfg(test)]
mod theme_overrides_tests {
    use super::*;

    #[test]
    fn is_valid_hex_color_accepts_3_4_6_8_digit_forms_and_rejects_others() {
        assert!(is_valid_hex_color("#fff"));
        assert!(is_valid_hex_color("#ffff"));
        assert!(is_valid_hex_color("#002868"));
        assert!(is_valid_hex_color("#002868ff"));
        assert!(!is_valid_hex_color("002868")); // missing #
        assert!(!is_valid_hex_color("#02868")); // 5 digits
        assert!(!is_valid_hex_color("#00286z")); // non-hex char
        assert!(!is_valid_hex_color("navy")); // named color, not hex
        assert!(!is_valid_hex_color("#"));
    }

    #[test]
    fn is_valid_font_value_allows_quotes_and_commas_rejects_injection_chars() {
        assert!(is_valid_font_value("'Times New Roman', serif"));
        assert!(is_valid_font_value("Dosis, sans-serif"));
        for bad in ["Evil\"", "Evil\\", "Evil<b>", "Evil>", "Evil;alert(1)", "Evil\n"] {
            assert!(!is_valid_font_value(bad), "expected {bad:?} to be rejected");
        }
        assert!(!is_valid_font_value(""));
    }

    #[test]
    fn validate_rejects_first_bad_color_field() {
        let theme = ThemeOverrides { primary: Some("notacolor".to_string()), ..Default::default() };
        assert!(theme.validate().is_err());
    }

    #[test]
    fn validate_accepts_valid_overrides_and_leaves_rest_none() {
        let theme = ThemeOverrides {
            primary: Some("#002868".to_string()),
            font_body: Some("'Times New Roman', serif".to_string()),
            ..Default::default()
        };
        assert!(theme.validate().is_ok());
    }
}

/// The branding fields configurable via `settings.toml` that need to reach
/// client-rendered pages. Fetched through a server function (rather than
/// reading `crate::SETTINGS` directly from view code) because `SETTINGS` is
/// a `Lazy` that can only initialize on the server: touching it from code
/// that also compiles into the WASM client panics with "Lazy initialization
/// is only supported with tokio and threads enabled."
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Branding {
    pub project_name: String,
    pub footer: String,
    /// CSS custom-property overrides from `settings.toml`'s `[theme]` table, applied by
    /// `views::wn_layout::WNLayout` via `document.documentElement.style.setProperty(...)` rather
    /// than by swapping a stylesheet. Riding along in this struct means it rides along with
    /// `project_name`/`footer` through the same `Loader<Branding>`, which already correctly
    /// refetches on `.restart()` after `backend::setup::configure_wordnet_source` hot-swaps
    /// `SETTINGS`.
    pub theme: ThemeOverrides,
    /// The logo's own raw SVG markup, inlined directly into the page (via `dangerous_inner_html`)
    /// instead of linked via `<img src="/logo">`: a `<link>`/`<img src>` is a plain
    /// static-resource URL the browser/webview caches on its own with no reactive dependency on
    /// server state, so after a hot-swap it'd keep showing whatever was cached from before.
    /// Assumes `settings.toml`'s `logo` points at an SVG file, which is true of every logo
    /// shipped with this app (`assets/gwa.svg`, `assets/english.svg`).
    pub logo_svg: String,
}

// A desktop build embeds the lexicon/settings state directly in-process (see this crate's
// `main.rs`), so it doesn't need - and can't use - a real HTTP round-trip for any of these:
// `#[cfg_attr]` skips the `#[get]`/`#[post]` macro there entirely, leaving a plain async fn that
// view/component code (already only ever calling these isomorphically, e.g. via `use_loader`)
// invokes directly. `web`/`server` still go through the macro as normal - Dioxus's own internal
// `#[cfg(feature = "server")]` split inside its expansion already distinguishes the WASM
// client's RPC stub from the real server-side handler, so this crate never needs to.
#[cfg_attr(not(feature = "desktop"), get("/api/branding"))]
pub async fn get_branding() -> Result<Branding> {
    let settings = crate::db::read_settings();
    Ok(Branding {
        project_name: settings.project_name.clone(),
        footer: settings.footer.clone(),
        theme: settings.theme.clone(),
        logo_svg: std::fs::read_to_string(&settings.logo).unwrap_or_default(),
    })
}

/// Everything the home page needs in one round trip: the configurable
/// tagline/intro text plus live counts, so the page can show e.g.
/// "142,384 synsets".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HomeInfo {
    pub tagline: String,
    pub intro: String,
    pub n_synsets: usize,
    pub n_entries: usize,
}

#[cfg_attr(not(feature = "desktop"), get("/api/home"))]
pub async fn get_home_info() -> Result<HomeInfo> {
    let settings = crate::db::read_settings();
    let tagline = settings.tagline.clone();
    let intro = settings.intro.clone();
    let lexicon = read_lexicon()?;
    Ok(HomeInfo {
        tagline,
        intro,
        n_synsets: lexicon.n_synsets()?,
        n_entries: lexicon.n_entries()?,
    })
}

/// A uniformly random synset id, for the home page's "Random synset" button.
#[cfg_attr(not(feature = "desktop"), get("/api/random_synset"))]
pub async fn get_random_synset() -> Result<Option<SynsetId>> {
    let lexicon = read_lexicon()?;
    Ok(lexicon.random_synset_id()?)
}

/// Look up the synset carrying a given ILI id, for `/ili/:id`-style links (see issue #20) -
/// the old `en-word.net/ili/iXXX` namespace, and the standard `globalwordnet.org/cili/iXXX`
/// URL now hyperlinked from a synset's own ILI identifier.
#[cfg_attr(not(feature = "desktop"), get("/api/synset_by_ili/{ili}"))]
pub async fn get_synset_by_ili(ili: String) -> Result<Option<SynsetId>> {
    let lexicon = read_lexicon()?;
    Ok(lexicon.synset_by_ili(&ili)?)
}

/// What a [`SearchResult`] refers to, so the frontend knows which page to
/// navigate to when a suggestion is picked.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SearchResultKind {
    Lemma,
    Synset,
}

/// A single autocomplete suggestion. `value` is what to look up (a lemma, or
/// a bare synset id like `00001740-n`); `display` is what to show the user.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResult {
    pub display: String,
    pub kind: SearchResultKind,
    pub value: String,
}

/// Users may search by lemma, by synset id (with or without the configured
/// `id_prefix` used in the RDF/XML/Turtle exports), or by ILI. Strips a
/// leading, case-insensitive `{id_prefix}-` so both `00001740-n` and
/// `oewn-00001740-n` (with the default `id_prefix`) match.
#[allow(dead_code)]
fn strip_id_prefix<'a>(query: &'a str, id_prefix: &str) -> &'a str {
    let prefixed = format!("{}-", id_prefix);
    match query.get(..prefixed.len()) {
        Some(prefix) if prefix.eq_ignore_ascii_case(&prefixed) => &query[prefixed.len()..],
        _ => query,
    }
}

#[cfg_attr(not(feature = "desktop"), get("/api/by_lemma/{lemma}"))]
pub async fn get_lemma(lemma: String) -> Result<Vec<SynsetId>> {
    let lexicon = read_lexicon()?;
    let lemmas = lexicon.entry_by_lemma(&lemma)?;
    let synset_ids = lemmas
        .iter()
        .flat_map(|entry| entry.sense.iter().map(|sense| sense.synset.clone()))
        .collect();
    Ok(synset_ids)
}

#[cfg_attr(not(feature = "desktop"), get("/api/lemma/{lemma}"))]
pub async fn get_lemma_synsets(lemma: String) -> Result<Vec<MemberSynset>> {
    let lexicon = read_lexicon()?;
    let entries = lexicon.entry_by_lemma(&lemma)?;
    let synset_ids: BTreeSet<SynsetId> = entries
        .iter()
        .flat_map(|entry| entry.sense.iter().map(|sense| sense.synset.clone()))
        .collect();

    let mut synsets = Vec::with_capacity(synset_ids.len());
    for id in &synset_ids {
        if let Some(synset) = lexicon.synset_by_id(id)? {
            synsets.push(MemberSynset::from_synset(id, synset.into_owned(), &*lexicon)?);
        }
    }
    Ok(synsets)
}

/// A single short definition, for a page's `<meta name="description">` - deliberately far
/// cheaper than `get_lemma_synsets` since it skips `MemberSynset::from_synset`'s reverse-
/// relation expansion entirely, needed only for one sentence of text.
#[cfg_attr(not(feature = "desktop"), get("/api/lemma_description/{lemma}"))]
pub async fn get_lemma_description(lemma: String) -> Result<Option<String>> {
    let lexicon = read_lexicon()?;
    let entries = lexicon.entry_by_lemma(&lemma)?;
    let Some(synset_id) = entries
        .iter()
        .find_map(|entry| entry.sense.first().map(|sense| sense.synset.clone()))
    else {
        return Ok(None);
    };
    let Some(synset) = lexicon.synset_by_id(&synset_id)? else {
        return Ok(None);
    };
    Ok(synset.definition.first().cloned())
}

#[cfg_attr(not(feature = "desktop"), get("/api/autocomplete/{query}?max_results"))]
pub async fn autocomplete(query: String, max_results: Option<usize>) -> Result<Vec<SearchResult>> {
    let max_results = max_results.unwrap_or(100);
    let lexicon = read_lexicon()?;
    let mut results = Vec::new();

    for lemma in lexicon.lemma_by_prefix(&query, Some(max_results))? {
        results.push(SearchResult {
            display: lemma.clone(),
            kind: SearchResultKind::Lemma,
            value: lemma,
        });
    }

    // Synset ids may be typed bare ("00001740-n") or with the configured
    // id_prefix used in the RDF/XML/Turtle exports.
    let settings = crate::db::read_settings();
    let id_prefix = &settings.id_prefix;
    for ssid in lexicon.ssid_by_prefix(strip_id_prefix(&query, id_prefix), Some(max_results))? {
        results.push(SearchResult {
            display: format!("{}-{}", id_prefix, ssid),
            kind: SearchResultKind::Synset,
            value: ssid,
        });
    }

    for (ili, ssid) in lexicon.ili_by_prefix(&query, Some(max_results))? {
        results.push(SearchResult {
            display: format!("{} ({})", ili, ssid.as_str()),
            kind: SearchResultKind::Synset,
            value: ssid.as_str().to_string(),
        });
    }

    let mut results = results.into_iter().take(max_results).collect::<Vec<_>>();
    results.sort_by(|a, b| match a.display.to_lowercase().cmp(&b.display.to_lowercase()) {
        std::cmp::Ordering::Equal => a.display.cmp(&b.display).reverse(),
        x => x,
    });
    Ok(results)
}

#[cfg_attr(not(feature = "desktop"), get("/api/synset/{id}"))]
pub async fn get_synset(id: SynsetId) -> Result<Option<MemberSynset>> {
    let lexicon = read_lexicon()?;
    let synset = lexicon.synset_by_id(&id)?;
    if let Some(synset) = synset {
        Ok(Some(MemberSynset::from_synset(
            &id,
            synset.into_owned(),
            &*lexicon,
        )?))
    } else {
        Ok(None)
    }
}
