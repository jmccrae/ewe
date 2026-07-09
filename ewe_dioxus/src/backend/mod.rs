pub mod api;
#[cfg(feature="server")]
pub use api::{get_lemma, autocomplete};

#[cfg(feature="server")]
pub mod rdf;
#[cfg(feature="server")]
pub use rdf::synset_negotiated;

#[cfg(feature="server")]
pub mod xml;

#[cfg(feature="server")]
pub mod static_files;
