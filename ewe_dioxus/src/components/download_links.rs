use dioxus::prelude::*;
#[cfg(not(feature = "desktop"))]
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};

#[cfg(not(feature = "desktop"))]
static CSS: Asset = asset!("/assets/styling/download_links.css");

#[cfg(not(feature = "desktop"))]
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
/// Web-only (see the `desktop` variant below): these are plain `<a href>` links reaching routes
/// only a real running HTTP server can serve, and `desktop` doesn't run one at all - see
/// `backend::setup`'s module doc comment on the `cfg_attr` pattern used everywhere else in
/// `backend/` to make its *isomorphically-called* endpoints work without a server; these
/// link-only export routes aren't called that way from anywhere, so they're simply not part of
/// what desktop needs to support.
#[cfg(not(feature = "desktop"))]
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

#[cfg(feature = "desktop")]
#[component]
#[allow(unused_variables)]
pub fn DownloadLinks(kind: &'static str, id: String) -> Element {
    rsx! {}
}
