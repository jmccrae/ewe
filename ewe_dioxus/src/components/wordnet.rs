use dioxus::prelude::*;
use dioxus::events::FormEvent;
use crate::backend::api::autocomplete;

#[component]
pub fn WordNet() -> Element {
    let mut lemma = use_signal(|| String::new());
    let mut show_suggestions = use_signal(|| false);
    let mut suggestions = use_action(move |query| async move {
        autocomplete(query).await
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
                    class: "wordnet-search-label",
                    "LEMMA"
                },
                span {
                    class: "suggestions-span",
                    input {
                        class: "wordnet-lemma",
                        r#type: "text",
                        placeholder: "Enter a word",
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
                                                key: "{s}", 
                                                onmousedown: move |_| {
                                                    let navigator = navigator();
                                                    lemma.set(s.clone());
                                                    navigator.push(format!("/lemma/{}", s));
                                                    show_suggestions.set(false);
                                                },
                                                "{s}"
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
            }
        }
    }
}