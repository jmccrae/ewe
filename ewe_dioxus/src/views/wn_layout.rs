use dioxus::prelude::*;
use crate::components::{provide_display_options, provide_panel_visibility};
use crate::Route;

#[component]
pub fn WNLayout() -> Element {
    provide_display_options();
    provide_panel_visibility();

    let settings = crate::SETTINGS.get();

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
                        "{settings.project_name}"
                    }
                }
            }
            Outlet::<Route> {}
            footer {
                class: "footer",
                dangerous_inner_html: "{settings.footer}"
            }
        }
    }
}
