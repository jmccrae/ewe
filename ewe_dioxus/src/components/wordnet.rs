use dioxus::prelude::*;
use dioxus::events::FormEvent;
use crate::backend::api::{autocomplete, SearchResultKind};
use crate::components::DisplayOptionsButton;
use crate::Route;

#[component]
pub fn WordNet() -> Element {
    let mut lemma = use_signal(|| String::new());
    let mut show_suggestions = use_signal(|| false);
    let mut suggestions = use_action(move |query| async move {
        autocomplete(query, None).await
    });

    let update_lemma = move |e: FormEvent| async move {
        lemma.set(e.value().to_string());
        if !e.value().is_empty() {
            suggestions.call(e.value().to_string()).await;
        }
        show_suggestions.set(!e.value().is_empty());
    };

    rsx! {
        div {
            id: "wordnet",
            div {
                class: "wordnet-input",
                span {
                    class: "suggestions-span",
                    input {
                        class: "wordnet-lemma",
                        r#type: "text",
                        placeholder: "Enter a word or identifier",
                        value: "{lemma}",
                        oninput: update_lemma
                    }
                    if *show_suggestions.read() {
                        ul {
                            class: "suggestions",
                            match suggestions.value() {
                                Some(Ok(suggestions)) => {
                                    rsx! {
                                        for s in suggestions.cloned().into_iter() {
                                            li {
                                                key: "{s.display}",
                                                onmousedown: move |_| {
                                                    let navigator = navigator();
                                                    lemma.set(s.display.clone());
                                                    match s.kind {
                                                        SearchResultKind::Lemma => {
                                                            navigator.push(Route::ByLemma { lemma: s.value.clone() });
                                                        }
                                                        SearchResultKind::Synset => {
                                                            navigator.push(Route::BySynset { synset: s.value.clone() });
                                                        }
                                                    }
                                                    show_suggestions.set(false);
                                                },
                                                "{s.display}"
                                            }

                                        }
                                    }
                                },
                                Some(Err(_e)) => {
                                    rsx! {
                                        div { "Failed to load suggestions" }
                                    }
                                },
                                None => {
                                    rsx! {
                                        
                                    }
                                }
                            }
                        }
                    }
                }
                DisplayOptionsButton {}
            }
        }
    }
}
