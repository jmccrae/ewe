use dioxus::prelude::*;
use crate::backend::api::get_branding;
use crate::components::{provide_display_options, provide_panel_visibility};
use crate::Route;

#[component]
pub fn WNLayout() -> Element {
    provide_display_options();
    provide_panel_visibility();

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
                    Link { to: Route::Downloads {}, "Downloads" }
                    " | "
                    a { href: "/api/docs", "JSON API documentation" }
                }
            }
        }
    }
}
