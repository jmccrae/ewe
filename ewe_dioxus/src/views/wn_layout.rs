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

    rsx! {
        div {
            class: "container",
            div {
                id: "logo",
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
                dangerous_inner_html: "{footer}"
            }
        }
    }
}
