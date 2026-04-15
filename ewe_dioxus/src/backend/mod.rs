pub mod api;
#[cfg(feature="server")]
pub use api::{get_lemma, autocomplete};

#[cfg(feature="server")]
pub mod rdf;
#[cfg(feature="server")]
pub use rdf::synset_negotiated;
