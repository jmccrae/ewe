// The dioxus prelude contains a ton of common items used in dioxus apps. It's a good idea to import wherever you
// need dioxus
use dioxus::prelude::*;

// `server` needs these to run the real fullstack HTTP server; `desktop` needs them too, since a
// desktop build embeds the exact same lexicon/corpus/settings state in-process (see the
// `cfg_attr(not(feature = "desktop"), get(...)/post(...))` pattern throughout `backend/`) rather
// than talking to a separately-running server binary over HTTP.
#[cfg(any(feature = "server", feature = "desktop"))]
use ewe_lib::wordnet::ReDBLexicon;
#[cfg(any(feature = "server", feature = "desktop"))]
use std::sync::RwLock;
#[cfg(any(feature = "server", feature = "desktop"))]
use teanga::disk_corpus::RedbDb;
#[cfg(any(feature = "server", feature = "desktop"))]
use teanga::DiskCorpus;
#[cfg(not(feature = "desktop"))]
use views::Downloads;
use views::{ByIli, ByLemma, BySenses, BySynset, History, Home, WNLayout};

/// Define a backend module that contains all business logic for our app.
mod backend;
/// Define a components module that contains all shared components for our app.
mod components;
/// Opening (and automatically rebuilding, if stale) the lexicon database.
#[cfg(any(feature = "server", feature = "desktop"))]
mod db;
/// Downloads page configuration (`downloads.toml`)
mod downloads_config;
/// The settings file
#[cfg(any(feature = "server", feature = "desktop"))]
mod settings;
/// Define a views module that contains the UI for all Layouts and Routes for our app.
mod views;

use downloads_config::DownloadsConfig;
#[cfg(any(feature = "server", feature = "desktop"))]
use settings::EweSettings;

/// The Route enum is used to define the structure of internal routes in our app. All route enums need to derive
/// the [`Routable`] trait, which provides the necessary methods for the router to work.
///
/// Each variant represents a different URL pattern that can be matched by the router. If that pattern is matched,
/// the components for that route will be rendered.
#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    // The layout attribute defines a wrapper for all routes under the layout. Layouts are great for wrapping
    // many routes with a common UI like a navbar.
    #[layout(WNLayout)]
        #[route("/")]
        Home {},

        #[route("/view/lemma/:lemma")]
        ByLemma { lemma: String },

        #[route("/view/synset/:synset")]
        BySynset { synset: String },

        // Resolves the old `en-word.net/ili/iXXX` namespace (issue #20) to whichever synset
        // currently carries that ILI.
        #[route("/ili/:id")]
        ByIli { id: String },

        #[route("/view/senses/:id?:page")]
        BySenses { id: String, page: usize },

        #[cfg(not(feature = "desktop"))]
        #[route("/downloads")]
        Downloads {},

        #[route("/history")]
        History {},
}

// We can import assets in dioxus with the `asset!` macro. This macro takes a path to an asset relative to the crate root.
// The macro returns an `Asset` type that will display as the path to the asset in the browser or a local path in desktop bundles.
const FAVICON: Asset = asset!("/assets/favicon.ico");
// The asset macro also minifies some assets like CSS and JS to make bundled smaller
const MAIN_CSS: Asset = asset!("/assets/styling/main.css");

// The app's own default `logo`/`theme` (`settings::default_logo`/`default_theme`) live under
// `assets/`, not named individually via their own `asset!()` call like `FAVICON`/`MAIN_CSS` above -
// `dx bundle` otherwise only ships whatever's explicitly named in an `asset!()` call, so they'd
// silently be missing from an installed app even though they work fine in a `dx serve` checkout
// (which just reads straight off the source tree). Bundling the whole folder as one asset (rather
// than listing files individually, which `[bundle] resources` in `Dioxus.toml` doesn't do
// recursively anyway) keeps every file's original name and relative layout intact
// (`with_hash_suffix(false)`) - `settings::default_logo`/`default_theme` resolve their path
// against `ASSETS_FOLDER.resolve()` (see their doc comments) rather than a bare relative string,
// since the bundled copy doesn't land at the same path a relative `"assets/..."` string would
// mean on a plain checkout (`Asset::resolve()` knows how to find it either way). Desktop-only:
// `web`/`server` read `assets/` directly off disk from wherever the process is running, so
// bundling it into the binary doesn't apply there.
#[cfg(feature = "desktop")]
#[used]
pub(crate) static ASSETS_FOLDER: Asset =
    asset!("/assets", AssetOptions::folder().with_hash_suffix(false));

/// A minimal, always-synchronous stand-in for `dioxus_fullstack::Lazy`, used below for state that
/// needs to initialize on `desktop` too (not just `server`). `dioxus_fullstack::Lazy`'s own
/// blocking-initialization path hard-`unimplemented!()`s unless dioxus's own "server" Cargo
/// feature is active (see its `lazy.rs` source) - true for this crate's `server` feature, never
/// true for `desktop` alone, so touching a `dioxus_fullstack::Lazy` on a desktop-only build panics
/// with "Lazy initialization is only supported with tokio and threads enabled." at first access.
/// None of `SETTINGS`/`LEXICON`/`CORPUS`/`DOWNLOADS` below actually do anything asynchronous
/// (they're synchronous disk I/O wrapped in `async move` purely to match `Lazy::new`'s signature),
/// so a plain `OnceLock` with a genuinely synchronous initializer sidesteps the problem entirely
/// instead of needing dioxus's own runtime machinery at all.
struct SyncLazy<T: 'static> {
    cell: std::sync::OnceLock<T>,
    init: fn() -> T,
}

impl<T: 'static> SyncLazy<T> {
    const fn new(init: fn() -> T) -> Self {
        Self { cell: std::sync::OnceLock::new(), init }
    }

    fn get(&self) -> &T {
        self.cell.get_or_init(self.init)
    }
}

// Wrapped in a `RwLock` (like `LEXICON` below) so `backend::setup::configure_wordnet_source`
// can hot-swap in a whole new settings file - project name, branding, id_prefix, and all -
// picked up from a folder the desktop user chooses, without needing to restart the server.
#[cfg(any(feature = "server", feature = "desktop"))]
#[allow(dead_code)]
static SETTINGS: SyncLazy<RwLock<settings::EweSettings>> = SyncLazy::new(|| {
    let settings = if std::path::Path::new("settings.toml").exists() {
        EweSettings::load("settings.toml").expect("Failed to load settings")
    } else {
        EweSettings::default()
    };
    RwLock::new(settings)
});

/// Downloads are entirely optional: if `downloads.toml` doesn't exist, the
/// Downloads page just has nothing to list rather than erroring.
#[allow(dead_code)]
static DOWNLOADS: SyncLazy<DownloadsConfig> = SyncLazy::new(|| {
    if std::path::Path::new("downloads.toml").exists() {
        DownloadsConfig::load("downloads.toml").expect("Failed to load downloads.toml")
    } else {
        DownloadsConfig::default()
    }
});

// Wrapped in a `RwLock` (rather than the plain `ReDBLexicon` reads elsewhere use) so that
// edit-mode server functions (see `backend::edit`) can take a write lock to apply automaton
// actions, while ordinary lookups take a read lock and don't block each other. The lock wraps
// an `Option` (rather than the whole thing being `Option<RwLock<...>>`) so a lexicon that failed
// to open at startup isn't a permanent dead end: `backend::setup::configure_wordnet_source` can
// hot-swap a freshly-opened lexicon into the `Some` slot later, without ever needing to
// re-touch this `Lazy` itself (which only ever initializes once).
#[cfg(any(feature = "server", feature = "desktop"))]
static LEXICON: SyncLazy<RwLock<Option<ReDBLexicon>>> = SyncLazy::new(|| {
    let settings = SETTINGS.get().read().unwrap();
    let lexicon = if db::is_unconfigured_lexicon(&settings) {
        // Nothing set up yet (e.g. a desktop app launched fresh, with no persisted settings.toml
        // - see `backend::setup`) - the normal starting state, not worth logging as a failure.
        None
    } else {
        match db::open_lexicon(&settings) {
            Ok(lexicon) => Some(lexicon),
            Err(e) => {
                eprintln!("Failed to open lexicon: {}", e);
                None
            }
        }
    };
    RwLock::new(lexicon)
});

// Wrapped in a `RwLock<Option<...>>` for the same reason as `LEXICON` - the corpus (like the
// lexicon) is reloaded from a freshly-picked project's `corpus_source`/`corpus_database` by
// `backend::setup::configure_wordnet_source`. A missing/failed corpus is non-fatal either way
// (it's only used to show usage examples), hence the `Option`.
#[cfg(any(feature = "server", feature = "desktop"))]
static CORPUS: SyncLazy<RwLock<Option<DiskCorpus<RedbDb>>>> = SyncLazy::new(|| {
    let settings = SETTINGS.get().read().unwrap();
    let corpus = if db::is_unconfigured_corpus(&settings) {
        // See `LEXICON`'s equivalent check just above - nothing set up yet, not a failure.
        None
    } else {
        match db::open_corpus(&settings) {
            Ok(corpus) => Some(corpus),
            Err(e) => {
                eprintln!("Failed to open corpus: {}", e);
                None
            }
        }
    };
    RwLock::new(corpus)
});

fn main() {
    // The `launch` function is the main entry point for a dioxus app. It takes a component and renders it with the platform feature
    // you have enabled
    #[cfg(not(feature = "server"))]
    dioxus::launch(App);

    #[cfg(feature = "server")]
    dioxus::serve(|| async move {
        // Registered as a plain axum route, not a `#[get(...)]` server
        // function: server functions get their successful responses
        // rewritten into a 302-redirect-to-Referer by dioxus-server for any
        // request that looks like a real browser navigation (see the doc
        // comment on `backend::downloads::download_file`), which broke
        // clicking a download link.
        let router = dioxus::server::router(App)
            .route(
                "/downloads/{filename}",
                dioxus_fullstack::axum::routing::get(backend::downloads::download_file),
            )
            .layer(dioxus_fullstack::axum::middleware::from_fn(
                strip_referer_from_export_links,
            ));

        Ok(router)
    });
}

/// Every plain `<a href>` link that points straight at a `#[get(...)]`
/// server function with no `Location` header hits the same dioxus-server
/// bug: the "Download As: JSON | RDF/XML | Turtle | XML" links on a
/// synset/lemma page (see `components::download_links::DownloadLinks`,
/// `backend::api`/`backend::rdf`/`backend::xml`), and the footer's "JSON API
/// documentation" link (`views::wn_layout`, `backend::openapi::api_docs`).
/// Clicking one is a real `<a href>` navigation, which sends
/// `Accept: text/html` plus a `Referer` header - exactly the combination
/// dioxus-server's server-function post-processing treats as a
/// progressive-enhancement `<form>` post, silently rewriting the (correct)
/// 200 response into a 302 back to the Referer (the same root cause as
/// `backend::downloads::download_file`, see its doc comment). None of these
/// routes are ever posted to by a `<form>`, so stripping the Referer before
/// it reaches the server function disables that post-processing without
/// having to pull each handler out of the server-function machinery (unlike
/// `download_file`, `backend::api::get_synset` is also called directly,
/// isomorphically, from `components::synset` and `views::by_senses`, so it
/// has to stay a real server function).
#[cfg(feature = "server")]
async fn strip_referer_from_export_links(
    mut req: dioxus_fullstack::axum::extract::Request,
    next: dioxus_fullstack::axum::middleware::Next,
) -> dioxus_fullstack::axum::response::Response {
    let path = req.uri().path();
    let is_export_link = path == "/api/docs"
        || path.starts_with("/api/synset/")
        || path.starts_with("/api/lemma/")
        || path.starts_with("/rdf/synset/")
        || path.starts_with("/rdf/lemma/")
        || path.starts_with("/ttl/synset/")
        || path.starts_with("/ttl/lemma/")
        || path.starts_with("/xml/synset/")
        || path.starts_with("/xml/lemma/");
    if is_export_link {
        req.headers_mut()
            .remove(dioxus_fullstack::http::header::REFERER);
    }
    next.run(req).await
}

/// App is the main component of our app. Components are the building blocks of dioxus apps. Each component is a function
/// that takes some props and returns an Element. In this case, App takes no props because it is the root of our app.
///
/// Components should be annotated with `#[component]` to support props, better error messages, and autocomplete
#[cfg(any(feature = "server", feature = "desktop"))]
#[component]
fn App() -> Element {
    // Eagerly load the corpus alongside the lexicon. It's supplementary
    // (used for showing usages), so a failure is logged but doesn't block the app. Also runs on
    // desktop now - a desktop build embeds this same state in-process (see `backend::setup`'s
    // `cfg_attr` doc comment) rather than talking to a separately-running server, so this is the
    // first point at which anything actually forces `LEXICON`/`CORPUS`/`SETTINGS`'s `Lazy`
    // initializers to run there too.
    CORPUS.get();
    App2()
}

#[cfg(not(any(feature = "server", feature = "desktop")))]
#[component]
fn App() -> Element {
    App2()
}

#[allow(non_snake_case)]
fn App2() -> Element {
    // The `rsx!` macro lets us define HTML inside of rust. It expands to an Element with all of our HTML inside.
    rsx! {
        // In addition to element and text (which we will see later), rsx can contain other components. In this case,
        // we are using the `document::Link` component to add a link to our favicon and main CSS file into the head of our app.
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        // The router component renders the route enum we defined above. It will handle synchronization of the URL and render
        // the layouts and components for the active route.
        Router::<Route> {}
    }
}
