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
            footer {
                class: "footer",
                p {
                    class: "footer1",
                    "Open English Wordnet is derived from "
                    a { href: "http://wordnet.princeton.edu/", "Princeton WordNet" }
                    " by the Open English Wordnet Community and released under the "
                    a {
                        href: "https://creativecommons.org/licenses/by/4.0/",
                        "Creative Commons Attribution (CC-BY) 4.0 License"
                    }
                    ". "
                    a {
                        href: "https://globalwordnet.github.io/gwadoc/",
                        "Further information about Wordnet"
                    }
                    ". We welcome any corrections, improvements or other contributions at "
                    a { href: "http://github.com/globalwordnet/english-wordnet", "GitHub" }
                    ". A full list of contributors is available on "
                    a {
                        href: "https://github.com/globalwordnet/english-wordnet/blob/master/README.md",
                        "GitHub"
                    }
                    "."
                }
                p {
                    class: "footer2",
                    "This interface was created by "
                    a { href: "http://john.mccr.ae/", "John P. McCrae" }
                    " at the "
                    a { href: "https://dsi.nuigalway.ie/", "Data Science Institute" }
                    ", "
                    a { href: "http://www.universityofgalway.ie", "University of Galway" }
                    " ("
                    a { href: "http://github.com/jmccrae/ewe", "GitHub" }
                    "). Development of this interface is supported by "
                    a { href: "https://www.sfi.ie/", "Science Foundation Ireland" }
                    " as part of the "
                    a {
                        href: "https://www.insight-centre.org/",
                        "Insight Centre for Data Analytics"
                    }
                    " and the European Union's Horizon 2020 research and innovation programme under grant agreement No 731015 ("
                    a { href: "http://elex.is/", "ELEXIS" }
                    ")."
                }
            }
        }
    }
}
