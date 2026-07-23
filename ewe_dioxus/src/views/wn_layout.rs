use dioxus::prelude::*;
use crate::backend::api::get_branding;
use crate::backend::setup::get_setup_status;
use crate::components::{
    provide_display_options, provide_dirty_state, provide_panel_visibility, provide_project_name,
    ProjectName, SetupNeeded, UnsavedChangesToast, ValidateButton,
};
use crate::Route;

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
    let (project_name, footer) = match &branding {
        Ok(loaded) if !loaded.loading() => {
            let branding = loaded.read();
            (branding.project_name.clone(), branding.footer.clone())
        }
        _ => (String::new(), String::new()),
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
        div {
            class: "container",
            div {
                id: "logo",
                class: if is_home { "home-logo" },
                span {
                    id: "logo-img",
                    img {
                        src: "/logo"
                    }
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
