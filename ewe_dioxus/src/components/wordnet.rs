use dioxus::prelude::*;
use dioxus::events::{FormEvent, KeyboardEvent};
use dioxus::prelude::Key;
use crate::backend::api::{autocomplete, SearchResult, SearchResultKind};
use crate::components::{AddSynsetTrigger, DisplayOptionsButton};
use crate::Route;

#[component]
pub fn WordNet() -> Element {
    let mut lemma = use_signal(|| String::new());
    let mut show_suggestions = use_signal(|| false);
    let mut selected = use_signal(|| 0usize);
    let mut suggestions = use_action(move |query| async move {
        autocomplete(query, None).await
    });
    // `suggestions.value()` reverts to `None` for the duration of every
    // in-flight call (see `use_action`), so rendering the dropdown directly
    // from it blanked the whole list on every keystroke while the next
    // request was pending, then snapped it back in once the response
    // landed - visible as a flicker while typing. Keeping the last
    // successful list here and only replacing it once a new one actually
    // arrives keeps the dropdown populated in between.
    let mut visible_suggestions = use_signal(Vec::<SearchResult>::new);
    use_effect(move || {
        if let Some(Ok(list)) = suggestions.value() {
            visible_suggestions.set(list.cloned());
        }
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
        match s.kind {
            SearchResultKind::Lemma => {
                navigator.push(Route::ByLemma { lemma: s.value.clone() });
            }
            SearchResultKind::Synset => {
                navigator.push(Route::BySynset { synset: s.value.clone() });
            }
        }
        // Leave the box empty rather than filled with the just-completed
        // query, so it's ready for the next search.
        lemma.set(String::new());
        show_suggestions.set(false);
    };

    let onkeydown = move |e: KeyboardEvent| {
        let current = visible_suggestions.read().clone();
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
                    // A single positioned dropdown box containing the matched results (when
                    // any) and, always, the "add a new synset" row - the latter deliberately
                    // isn't one of the `<li>` results (it isn't a search *result*), so it stays
                    // available regardless of whether there are any matches, any suggestions
                    // loaded yet, or the fetch errored.
                    if !lemma().trim().is_empty() {
                        div {
                            class: "suggestions-dropdown",
                            if *show_suggestions.read() {
                                ul {
                                    class: "suggestions",
                                    if let Some(Err(_e)) = suggestions.value() {
                                        div { "Failed to load suggestions" }
                                    } else {
                                        for (i, s) in visible_suggestions.read().iter().cloned().enumerate() {
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
                                }
                            }
                            AddSynsetTrigger { query: lemma() }
                        }
                    }
                }
                DisplayOptionsButton {}
            }
        }
    }
}
