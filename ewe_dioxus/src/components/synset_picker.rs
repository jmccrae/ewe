#![cfg(feature = "edit")]

use dioxus::prelude::*;

use crate::backend::edit::{search_synsets, SynsetCandidate};

#[derive(Clone, PartialEq, Props)]
pub struct SynsetPickerProps {
    pub on_select: EventHandler<SynsetCandidate>,
    #[props(default = "Search for a word or synset…".to_string())]
    pub placeholder: String,
}

/// A search-as-you-type synset picker: an input plus a dropdown of matching synsets (one entry
/// per synset, with members/POS/definition so the user can tell candidates apart rather than
/// needing to know a synset id). Shared by the relation editor's target picker and the delete
/// modal's "superseding synset" field.
#[component]
pub fn SynsetPicker(props: SynsetPickerProps) -> Element {
    let mut search_query = use_signal(String::new);
    let mut suggestions = use_action(move |query: String| async move {
        search_synsets(query, None).await
    });

    rsx! {
        div {
            class: "relation-search-wrapper",
            input {
                class: "relation-search-input",
                r#type: "text",
                placeholder: "{props.placeholder}",
                value: "{search_query}",
                oninput: move |e| {
                    let value = e.value();
                    search_query.set(value.clone());
                    if !value.is_empty() {
                        suggestions.call(value);
                    }
                },
            }
            if let Some(Ok(results)) = suggestions.value() {
                {
                    let results = results.cloned();
                    rsx! {
                        if !results.is_empty() {
                            ul {
                                class: "relation-search-results",
                                for candidate in results {
                                    li {
                                        key: "{candidate.id}",
                                        onclick: {
                                            let candidate = candidate.clone();
                                            move |_| {
                                                props.on_select.call(candidate.clone());
                                                search_query.set(String::new());
                                            }
                                        },
                                        "{candidate.members.join(\", \")} ({candidate.part_of_speech}) — {candidate.definition}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
