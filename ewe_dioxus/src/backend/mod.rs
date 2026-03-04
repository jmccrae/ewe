pub mod api;
#[cfg(feature="server")]
pub use api::{get_lemma, autocomplete};
