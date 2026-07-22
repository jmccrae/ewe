use dioxus::prelude::*;
use crate::backend::api::get_branding;
use crate::components::{
    provide_display_options, provide_dirty_state, provide_panel_visibility, provide_project_name,
    ProjectName, UnsavedChangesToast, ValidateButton,
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
            Outlet::<Route> {}
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
