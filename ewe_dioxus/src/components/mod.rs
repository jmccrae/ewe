//! The components module contains all shared components for our app. Components are the building blocks of dioxus apps.
//! They can be used to defined common UI elements like buttons, forms, and modals. In this template, we define a Hero
//! component and an Echo component for fullstack apps to be used in our app.

mod wordnet;
pub use wordnet::WordNet;

mod synset;
pub use synset::Synset;

mod subcat;
pub use subcat::Subcat;

mod relation;
pub use relation::Relation;
