use dioxus::prelude::*;
use crate::components::{provide_display_options, provide_panel_visibility};
use crate::Route;

const LOGO_ASSET: Asset = asset!("/assets/english.svg");

#[component]
pub fn WNLayout() -> Element {
    provide_display_options();
    provide_panel_visibility();

    rsx! {
        div {
            class: "container",
            div {
                id: "logo",
                span {
                    id: "logo-img",                    
                    img { 
                        src: LOGO_ASSET
                    }
                }
                span {
                    id: "logo-title",
                    h1 {
                        class: "en-title",
                        "Open English Wordnet"
                    }
                }
            }
            Outlet::<Route> {}
            // TODO: Footer
        }
    }
}
