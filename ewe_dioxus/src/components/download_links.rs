use dioxus::prelude::*;
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};

static CSS: Asset = asset!("/assets/styling/download_links.css");

const ID_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b'/')
    .add(b'#')
    .add(b'?')
    .add(b'&')
    .add(b'=')
    .add(b'+')
    .add(b'$')
    .add(b',')
    .add(b';')
    .add(b':')
    .add(b'@')
    .add(b' ');

/// The "Download As: JSON | RDF/XML | Turtle | XML" links shown on a synset or lemma page,
/// pointing at the machine-readable exports served by `backend::api`/`backend::rdf`/`backend::xml`.
#[component]
pub fn DownloadLinks(kind: &'static str, id: String) -> Element {
    let id = utf8_percent_encode(&id, ID_ENCODE_SET).to_string();

    rsx! {
        document::Style { href: CSS },
        div {
            class: "download-links",
            b { "Download As: " }
            a { href: "/api/{kind}/{id}", "JSON" }
            a { href: "/rdf/{kind}/{id}", "RDF/XML" }
            a { href: "/ttl/{kind}/{id}", "Turtle" }
            a { href: "/xml/{kind}/{id}", "XML" }
        }
    }
}
