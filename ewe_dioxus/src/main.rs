// The dioxus prelude contains a ton of common items used in dioxus apps. It's a good idea to import wherever you
// need dioxus
use dioxus::prelude::*;

use dioxus_fullstack::Lazy;
#[cfg(feature = "server")]
use ewe_lib::wordnet::ReDBLexicon;
#[cfg(feature = "server")]
use std::sync::RwLock;
#[cfg(feature = "server")]
use teanga::disk_corpus::RedbDb;
#[cfg(feature = "server")]
use teanga::DiskCorpus;
#[cfg(not(feature = "desktop"))]
use views::Downloads;
use views::{ByLemma, BySenses, BySynset, History, Home, WNLayout};

/// Define a backend module that contains all business logic for our app.
mod backend;
/// Define a components module that contains all shared components for our app.
mod components;
/// Opening (and automatically rebuilding, if stale) the lexicon database.
#[cfg(feature = "server")]
mod db;
/// Downloads page configuration (`downloads.toml`)
mod downloads_config;
/// The settings file
#[cfg(feature = "server")]
mod settings;
/// Define a views module that contains the UI for all Layouts and Routes for our app.
mod views;

use downloads_config::DownloadsConfig;
#[cfg(feature = "server")]
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

// Wrapped in a `RwLock` (like `LEXICON` below) so `backend::setup::configure_wordnet_source`
// can hot-swap in a whole new settings file - project name, branding, id_prefix, and all -
// picked up from a folder the desktop user chooses, without needing to restart the server.
#[cfg(feature = "server")]
#[allow(dead_code)]
static SETTINGS: Lazy<RwLock<settings::EweSettings>> = Lazy::new(|| async move {
    let settings = if std::path::Path::new("settings.toml").exists() {
        EweSettings::load("settings.toml").expect("Failed to load settings")
    } else {
        EweSettings::default()
    };
    dioxus::Ok(RwLock::new(settings))
});

/// Downloads are entirely optional: if `downloads.toml` doesn't exist, the
/// Downloads page just has nothing to list rather than erroring.
#[allow(dead_code)]
static DOWNLOADS: Lazy<DownloadsConfig> = Lazy::new(|| async move {
    let downloads = if std::path::Path::new("downloads.toml").exists() {
        DownloadsConfig::load("downloads.toml").expect("Failed to load downloads.toml")
    } else {
        DownloadsConfig::default()
    };
    dioxus::Ok(downloads)
});

// Wrapped in a `RwLock` (rather than the plain `ReDBLexicon` reads elsewhere use) so that
// edit-mode server functions (see `backend::edit`) can take a write lock to apply automaton
// actions, while ordinary lookups take a read lock and don't block each other. The lock wraps
// an `Option` (rather than the whole thing being `Option<RwLock<...>>`) so a lexicon that failed
// to open at startup isn't a permanent dead end: `backend::setup::configure_wordnet_source` can
// hot-swap a freshly-opened lexicon into the `Some` slot later, without ever needing to
// re-touch this `Lazy` itself (which only ever initializes once).
#[cfg(feature = "server")]
static LEXICON: Lazy<RwLock<Option<ReDBLexicon>>> = Lazy::new(|| async move {
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
    dioxus::Ok(RwLock::new(lexicon))
});

// Wrapped in a `RwLock<Option<...>>` for the same reason as `LEXICON` - the corpus (like the
// lexicon) is reloaded from a freshly-picked project's `corpus_source`/`corpus_database` by
// `backend::setup::configure_wordnet_source`. A missing/failed corpus is non-fatal either way
// (it's only used to show usage examples), hence the `Option`.
#[cfg(feature = "server")]
static CORPUS: Lazy<RwLock<Option<DiskCorpus<RedbDb>>>> = Lazy::new(|| async move {
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
    dioxus::Ok(RwLock::new(corpus))
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
#[cfg(feature = "server")]
#[component]
fn App() -> Element {
    // Eagerly load the corpus alongside the lexicon. It's supplementary
    // (used for showing usages), so a failure is logged but doesn't block the app.
    CORPUS.get();
    App2()
}

#[cfg(not(feature = "server"))]
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
