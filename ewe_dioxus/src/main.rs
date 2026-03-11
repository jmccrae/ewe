// The dioxus prelude contains a ton of common items used in dioxus apps. It's a good idea to import wherever you
// need dioxus
use dioxus::prelude::*;

use views::{Home, WNLayout, ByLemma};
#[cfg(feature="server")]
use oewn_lib::wordnet::{Lexicon, ReDBLexicon};
use dioxus_fullstack::Lazy;
use oewn_lib::progress::NullProgress;

/// Define a components module that contains all shared components for our app.
mod components;
/// Define a views module that contains the UI for all Layouts and Routes for our app.
mod views;
/// Define a backend module that contains all business logic for our app.
mod backend;
/// The settings file
mod settings;

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

        #[route("/lemma/:lemma")]
        ByLemma { lemma: String },
}

// We can import assets in dioxus with the `asset!` macro. This macro takes a path to an asset relative to the crate root.
// The macro returns an `Asset` type that will display as the path to the asset in the browser or a local path in desktop bundles.
const FAVICON: Asset = asset!("/assets/favicon.ico");
// The asset macro also minifies some assets like CSS and JS to make bundled smaller
const MAIN_CSS: Asset = asset!("/assets/styling/main.css");

static SETTINGS : Lazy<settings::EweSettings> = Lazy::new(|| async move {
    let settings = if std::path::Path::new("settings.toml").exists() {
        EweSettings::load("settings.toml").expect("Failed to load settings")
    } else {
        EweSettings::default()
    };
    dioxus::Ok(settings)
});

#[cfg(feature="server")]
static LEXICON : Lazy<Option<ReDBLexicon>> = Lazy::new(|| async move {
    if let Ok(lexicon) = ReDBLexicon::open("wordnet.db") {
        dioxus::Ok(Some(lexicon))
    } else {
        dioxus::Ok(None)
    }
});

fn main() {
    // The `launch` function is the main entry point for a dioxus app. It takes a component and renders it with the platform feature
    // you have enabled
    #[cfg(not(feature="server"))]
    dioxus::launch(App);

    #[cfg(feature="server")]
    dioxus::serve(|| async move {
        let router = dioxus::server::router(App);

        Ok(router)
    });
}

/// App is the main component of our app. Components are the building blocks of dioxus apps. Each component is a function
/// that takes some props and returns an Element. In this case, App takes no props because it is the root of our app.
///
/// Components should be annotated with `#[component]` to support props, better error messages, and autocomplete
#[cfg(feature="server")]
#[component]
fn App() -> Element {
    match LEXICON.get() {
        Some(_) => {
            App2()
        },
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

#[cfg(not(feature="server"))]
#[component]
fn App() -> Element {
    App2()
}

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
