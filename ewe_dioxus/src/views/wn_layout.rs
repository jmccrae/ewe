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
            footer {
                class: "footer",
                div {
                    class: "container",
                   div {
                        class: "footer1",
                        dangerous_inner_html: "Open English Wordnet is derived from <a href=\"http://wordnet.princeton.edu/\">Princeton WordNet</a> by the Open English Wordnet Community and released under the <a href=\"https://creativecommons.org/licenses/by/4.0/\">Creative Commons Attribution (CC-BY) 4.0 License</a>. <a href=\"https://globalwordnet.github.io/gwadoc/\">Further information about Wordnet</a>. We welcome any corrections, improvements or other contributions at <a href=\"http://github.com/globalwordnet/english-wordnet\">GitHub</a>. A full list of contributors is available on <a href=\"https://github.com/globalwordnet/english-wordnet/blob/master/README.md\">GitHub</a>.</div>"
                    }
                    div {
                        class: "footer2",
                        dangerous_inner_html:  "This interface was created by <a href=\"http://john.mccr.ae/\">John P. McCrae</a> at the <a href=\"https://dsi.nuigalway.ie/\">Data Science Institute</a>, <a href=\"http://www.universityofgalway.ie\">University of Galway</a> (<a href=\"http://github.com/jmccrae/wordnet-angular\">GitHub</a>). Development of this interface is supported by <a href=\"https://www.sfi.ie/\">Science Foundation Ireland</a> as part of the <a href=\"https://www.insight-centre.org/\">Insight Centre for Data Analytics</a> and the European Union's Horizon 2020 research and innovation programme under grant agreement No 731015 (<a href=\"http://elex.is/\">ELEXIS</a>)."
                    }
                    div {
                        class: "footer1",
                        a {
                            href: "https://github.com/globalwordnet/english-wordnet/releases",
                            "Download Open English Wordnet"
                        }
                        span {
                            " | "
                        }
                        a {
                            href: "https://github.com/globalwordnet/english-wordnet/issues",
                            "Report an issue"
                        }

                    }
                }
 
            }
        }
    }
}
