//! The components module contains all shared components for our app. Components are the building blocks of dioxus apps.
//! They can be used to defined common UI elements like buttons, forms, and modals. In this template, we define a Hero
//! component and an Echo component for fullstack apps to be used in our app.

mod wordnet;
pub use wordnet::WordNet;

mod synset;
pub use synset::Synset;

mod editable_definition;
pub use editable_definition::EditableDefinition;

mod editable_examples;
pub use editable_examples::{EditableExamples, ExampleDraft};

mod editable_lemmas;
pub use editable_lemmas::EditableLemmas;

mod edit_toggle;
pub use edit_toggle::EditToggle;

mod subcat;
pub use subcat::Subcat;

mod relation;
pub use relation::Relation;

mod display_options;
pub use display_options::{
    provide_display_options, provide_panel_visibility, DisplayOptions, DisplayOptionsButton,
};

mod download_links;
pub use download_links::DownloadLinks;
