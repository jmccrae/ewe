pub mod api;

#[cfg(feature="server")]
pub mod rdf;

#[cfg(feature="server")]
pub mod xml;

#[cfg(feature="server")]
pub mod openapi;

#[cfg(feature="server")]
pub mod seo;

// The Downloads page isn't part of a desktop build.
#[cfg(not(feature = "desktop"))]
pub mod downloads;

pub mod senses;

pub mod setup;

#[cfg(feature = "edit")]
pub mod edit;
