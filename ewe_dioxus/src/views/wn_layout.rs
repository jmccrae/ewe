use crate::backend::api::get_branding;
use crate::backend::setup::get_setup_status;
use crate::components::{
    provide_dirty_state, provide_display_options, provide_panel_visibility, provide_project_name,
    ProjectName, SetupNeeded, UnsavedChangesToast, ValidateButton,
};
use crate::Route;
use dioxus::prelude::*;

/// The Downloads page and JSON API docs are web-facing features that don't apply to the
/// single-user desktop app (which already has direct local access to its own data).
#[cfg(not(feature = "desktop"))]
#[component]
fn WebOnlyFooterLinks() -> Element {
    rsx! {
        Link { to: Route::Downloads {}, "Downloads" }
        " | "
        a { href: "/api/docs", "JSON API documentation" }
        " | "
    }
}

#[cfg(feature = "desktop")]
#[component]
fn WebOnlyFooterLinks() -> Element {
    rsx! {}
}

/// HACK: reaches past `document::Style`'s public API to mutate a head element it created, using
/// undocumented internals (a hardcoded DOM id, and `document::eval` running raw JS against
/// `document.getElementById`) rather than anything Dioxus actually supports for this. It exists
/// because `document::Style` (see `WNLayout` below) only ever inserts its content once - "Any
/// updates to the props after the first render will not be reflected in the head", per its own
/// doc comment - which is a real problem here: on desktop, there's no SSR pass to resolve
/// `branding` before the first render, so that first render's `theme_css` is still `""`
/// (loading), and without this workaround the tag would be permanently stuck empty. Every fix
/// attempted that stayed within Dioxus's supported API (freezing the prop via `use_hook`, moving
/// `document::Style` up into `main.rs`'s `App2` with its own loader) either failed to actually
/// update the tag on desktop or otherwise didn't work - see git history on this file/`main.rs`
/// around this comment for what was tried.
///
/// Consequences of the hack, worth knowing before touching this again:
/// - Trips `document::Style`'s own "Changing the props of `Style {}` is not supported" console
///   warning every time `theme_css` actually changes (the initial load-resolves transition, or a
///   later desktop reconfigure) - harmless, but expected noise, not a sign something's broken.
/// - Depends on `document::Style` continuing to render a real `<style id="...">` element with
///   exactly the id we pass it, and on `document::eval` continuing to support arbitrary JS - both
///   currently true (dioxus 0.7.9) but neither is a documented contract, so a Dioxus upgrade could
///   silently break this (the id stops appearing, `textContent` writes go missing, etc.) without
///   any compile error to flag it. If the theme ever stops updating after a dioxus bump, look here
///   first.
#[component]
fn ThemeStyleUpdater(css: String) -> Element {
    let last_css = use_hook(|| std::rc::Rc::new(std::cell::RefCell::new(css.clone())));
    let mut last_css = last_css.borrow_mut();
    if css != *last_css {
        document::eval(&format!(
            "var el = document.getElementById('ewe-theme-style'); if (el) el.textContent = \"{}\";",
            escape_js_string(&css)
        ));
        *last_css = css;
    }
    rsx! {}
}

/// A minimal JS string-literal escaper, matching the one `dioxus_document` uses internally for
/// its own `document::eval` calls (it isn't exposed publicly, so this is a small copy rather than
/// pulling in `serde_json` as a dependency just for this one call site).
fn escape_js_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('"', "\\\"")
}

#[component]
pub fn WNLayout() -> Element {
    provide_display_options();
    provide_panel_visibility();
    provide_dirty_state();
    let mut project_name_ctx = provide_project_name();

    // Branding is fetched through a server function rather than reading
    // `crate::SETTINGS` here directly, since this component also runs in the
    // WASM client and `SETTINGS` is a server-only `Lazy`.
    let branding = use_loader(get_branding);
    let (project_name, footer, logo_svg, theme_css) = match &branding {
        Ok(loaded) if !loaded.loading() => {
            let branding = loaded.read();
            (
                branding.project_name.clone(),
                branding.footer.clone(),
                branding.logo_svg.clone(),
                branding.theme_css.clone(),
            )
        }
        _ => (String::new(), String::new(), String::new(), String::new()),
    };
    // `branding` is fetched here at the layout level - once, before it's known whether the app
    // is even configured - so it's already stuck showing stale (pre-configure) project_name/
    // footer by the time the desktop setup screen finishes. Unlike route content (which only
    // starts loading *after* `configured` flips true, so it's never stale), this needs its own
    // explicit restart alongside `setup_status`'s - see `SetupNeeded`'s doc comment.
    let branding_loader = match &branding {
        Ok(loaded) => Some(*loaded),
        Err(_) => None,
    };

    // Shares `project_name` with route views via context so each can compose its own
    // `document::Title` (e.g. "{lemma} - {project_name}") without fetching branding itself.
    // Written directly here (not in a `use_effect`) so it's visible to `Outlet`'s children
    // within the same render pass - an effect would only run after the initial SSR render
    // completes, too late for those children's own `document::Title` to see it.
    if project_name_ctx().0 != project_name {
        project_name_ctx.set(ProjectName(project_name.clone()));
    }

    // The logo/title is centered on the home page (OED-style hero treatment)
    // but stays left-aligned everywhere else, like a normal site header.
    let is_home = matches!(use_route::<Route>(), Route::Home {});

    // Runs on every route (unlike the old SSR-only "Error loading lexicon" gate this replaced
    // in `main.rs`'s `App()`), so it's a consistent, isomorphic screen no matter which page the
    // user navigates to first. While `status` is still loading, optimistically render the
    // normal routed content rather than flash the setup screen - on this app's fullstack setup,
    // server functions like this are awaited during the initial render (same as `branding`
    // above), so "still loading" is not expected to be user-visible in practice.
    let setup_status = use_loader(get_setup_status);
    let not_configured = match &setup_status {
        Ok(loaded) if !loaded.loading() => Some(loaded.read().clone()).filter(|s| !s.configured),
        _ => None,
    };
    // Passed down to `SetupNeeded` so the desktop configure flow can call `.restart()` on this
    // exact loader once done - re-running `get_setup_status` and reactively re-rendering this
    // component with the fresh result, without needing a real page reload (which isn't guaranteed
    // to behave the same way in a desktop webview as it does in a browser).
    let setup_status_loader = match &setup_status {
        Ok(loaded) => Some(*loaded),
        Err(_) => None,
    };

    rsx! {
        document::Style { id: "ewe-theme-style", "{theme_css}" }
        ThemeStyleUpdater { css: theme_css.clone() }
        div {
            class: "container",
            Link {
                id: "logo",
                class: if is_home { "home-logo" },
                to: Route::Home {},
                span {
                    id: "logo-img",
                    dangerous_inner_html: "{logo_svg}"
                }
                span {
                    id: "logo-title",
                    h1 {
                        class: "en-title",
                        "{project_name}"
                    }
                }
            }
            if let (Some(status), Some(loader), Some(branding_loader)) =
                (not_configured.clone(), setup_status_loader, branding_loader)
            {
                SetupNeeded { status, setup_status: loader, branding: branding_loader }
            } else {
                Outlet::<Route> {}
            }
            footer {
                class: "footer",
                div {
                    dangerous_inner_html: "{footer}"
                }
                p {
                    class: "api-docs-link",
                    WebOnlyFooterLinks {}
                    Link { to: Route::History {}, "History" }
                    " | "
                    ValidateButton {}
                }
            }
            UnsavedChangesToast {}
        }
    }
}
