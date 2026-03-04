use dioxus::prelude::*;
use crate::Route;

const LOGO_ASSET: Asset = asset!("/assets/english.svg");

#[component]
pub fn WNLayout() -> Element {
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
