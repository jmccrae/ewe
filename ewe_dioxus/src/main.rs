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
use views::{ByLemma, BySenses, BySynset, History, Home, WNLayout};
#[cfg(not(feature = "desktop"))]
use views::Downloads;

/// Define a backend module that contains all business logic for our app.
mod backend;
/// Define a components module that contains all shared components for our app.
mod components;
/// Opening (and automatically rebuilding, if stale) the lexicon database.
#[cfg(feature = "server")]
mod db;
/// The settings file
mod settings;
/// Downloads page configuration (`downloads.toml`)
mod downloads_config;
/// Define a views module that contains the UI for all Layouts and Routes for our app.
mod views;

use settings::EweSettings;
use downloads_config::DownloadsConfig;

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

#[allow(dead_code)]
static SETTINGS: Lazy<settings::EweSettings> = Lazy::new(|| async move {
    let settings = if std::path::Path::new("settings.toml").exists() {
        EweSettings::load("settings.toml").expect("Failed to load settings")
    } else {
        EweSettings::default()
    };
    dioxus::Ok(settings)
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
// actions, while ordinary lookups take a read lock and don't block each other.
#[cfg(feature = "server")]
static LEXICON: Lazy<Option<RwLock<ReDBLexicon>>> = Lazy::new(|| async move {
    match db::open_lexicon(SETTINGS.get()) {
        Ok(lexicon) => dioxus::Ok(Some(RwLock::new(lexicon))),
        Err(e) => {
            eprintln!("Failed to open lexicon: {}", e);
            dioxus::Ok(None)
        }
    }
});

#[cfg(feature = "server")]
static CORPUS: Lazy<Option<DiskCorpus<RedbDb>>> = Lazy::new(|| async move {
    match db::open_corpus(SETTINGS.get()) {
        Ok(corpus) => dioxus::Ok(Some(corpus)),
        Err(e) => {
            eprintln!("Failed to open corpus: {}", e);
            dioxus::Ok(None)
        }
    }
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
    match LEXICON.get() {
        Some(_) => App2(),
        None => {
            rsx! {
                div { class: "error",
                    h1 { "Error loading lexicon" }
                    p { "The lexicon failed to load. Please check the console for more details." }
                }
            }
        }
    }
}

#[cfg(not(feature = "server"))]
#[component]
fn App() -> Element {
    App2()
}

#[allow(non_snake_case)]
fn App2() -> Element {
    // The desktop webview loads pages from Dioxus's own `dioxus://` origin, not from the
    // fullstack HTTP server, so root-relative requests like `<img src="/logo">` never reach
    // `backend::static_files` - they need an explicit asset handler instead. See
    // `register_desktop_settings_assets` below.
    #[cfg(feature = "desktop")]
    register_desktop_settings_assets();

    // The `rsx!` macro lets us define HTML inside of rust. It expands to an Element with all of our HTML inside.
    rsx! {
        // In addition to element and text (which we will see later), rsx can contain other components. In this case,
        // we are using the `document::Link` component to add a link to our favicon and main CSS file into the head of our app.
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: "/theme.css" }
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        // The router component renders the route enum we defined above. It will handle synchronization of the URL and render
        // the layouts and components for the active route.
        Router::<Route> {}
    }
}

/// Registers desktop asset handlers for `/logo` and `/theme.css` so the webview can load them.
///
/// These two paths are dynamic `backend::static_files` routes on the fullstack HTTP server
/// (deliberately read from disk per-request rather than bundled via `asset!`, so a deployment
/// can be rebranded without rebuilding), but the desktop webview never talks to that server for
/// plain resource loads - it resolves root-relative URLs against its own internal `dioxus://`
/// origin, which only knows about bundled assets and explicitly-registered handlers like these.
/// Settings are loaded directly here (rather than through the `SETTINGS` static) because that
/// static's lazy initializer only supports being driven from the `server` feature's tokio
/// runtime, which this - the desktop client process - doesn't have.
#[cfg(feature = "desktop")]
fn register_desktop_settings_assets() {
    use dioxus::desktop::use_asset_handler;

    let settings = if std::path::Path::new("settings.toml").exists() {
        EweSettings::load("settings.toml").unwrap_or_else(|_| EweSettings::default())
    } else {
        EweSettings::default()
    };

    let logo_path = settings.logo;
    use_asset_handler("logo", move |_request, responder| {
        responder.respond(desktop_settings_asset_response(&logo_path));
    });

    let theme_path = settings.theme;
    use_asset_handler("theme.css", move |_request, responder| {
        responder.respond(desktop_settings_asset_response(&theme_path));
    });
}

#[cfg(feature = "desktop")]
fn desktop_settings_asset_response(path: &str) -> dioxus::desktop::wry::http::Response<Vec<u8>> {
    use dioxus::desktop::wry::http::{Response, StatusCode};

    match std::fs::read(path) {
        Ok(bytes) => Response::builder()
            .header("Content-Type", desktop_settings_asset_content_type(path))
            .body(bytes)
            .unwrap(),
        Err(e) => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(format!("File not found at {}: {}", path, e).into_bytes())
            .unwrap(),
    }
}

#[cfg(feature = "desktop")]
fn desktop_settings_asset_content_type(path: &str) -> &'static str {
    match std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("ico") => "image/x-icon",
        Some("css") => "text/css",
        _ => "application/octet-stream",
    }
}
