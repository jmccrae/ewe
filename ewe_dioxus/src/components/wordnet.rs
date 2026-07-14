use dioxus::prelude::*;
use dioxus::events::{FormEvent, KeyboardEvent};
use dioxus::prelude::Key;
use crate::backend::api::{autocomplete, SearchResult, SearchResultKind};
use crate::components::DisplayOptionsButton;
use crate::Route;

#[component]
pub fn WordNet() -> Element {
    let mut lemma = use_signal(|| String::new());
    let mut show_suggestions = use_signal(|| false);
    let mut selected = use_signal(|| 0usize);
    let mut suggestions = use_action(move |query| async move {
        autocomplete(query, None).await
    });

    let update_lemma = move |e: FormEvent| async move {
        lemma.set(e.value().to_string());
        selected.set(0);
        if !e.value().is_empty() {
            suggestions.call(e.value().to_string()).await;
        }
        show_suggestions.set(!e.value().is_empty());
    };

    let mut go_to = move |s: SearchResult| {
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
    };

    let onkeydown = move |e: KeyboardEvent| {
        let Some(Ok(current)) = suggestions.value() else {
            return;
        };
        let current = current.cloned();
        if current.is_empty() {
            return;
        }
        match e.key() {
            Key::ArrowDown => {
                e.prevent_default();
                selected.set((selected() + 1).min(current.len() - 1));
            }
            Key::ArrowUp => {
                e.prevent_default();
                selected.set(selected().saturating_sub(1));
            }
            Key::Enter => {
                e.prevent_default();
                let idx = selected().min(current.len() - 1);
                go_to(current[idx].clone());
            }
            _ => {}
        }
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
                        oninput: update_lemma,
                        onkeydown,
                    }
                    if *show_suggestions.read() {
                        ul {
                            class: "suggestions",
                            match suggestions.value() {
                                Some(Ok(suggestions)) => {
                                    rsx! {
                                        for (i, s) in suggestions.cloned().into_iter().enumerate() {
                                            li {
                                                key: "{s.display}",
                                                class: if i == selected() { "selected" },
                                                onmousedown: {
                                                    let s = s.clone();
                                                    move |_| go_to(s.clone())
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
