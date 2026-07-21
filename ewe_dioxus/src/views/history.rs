use dioxus::prelude::*;
use crate::components::WordNet;

/// How many entries `get_changelog` is asked for per page - also what the client compares a
/// page's length against to decide whether there's more to load (a short page means the log ran
/// out before the limit was reached).
#[cfg(feature = "edit")]
const PAGE_SIZE: usize = 50;

/// A page of the Wordnet editor's change history - every batch of automaton actions ever applied,
/// newest first, paginated with a "Load more" button. Not available in builds without the `edit`
/// feature (the public, read-only en-word.net site never has anything to show here).
#[cfg(feature = "edit")]
#[component]
pub fn History() -> Element {
    use crate::backend::edit::{get_changelog, ChangeLogEntryView};

    let Ok(first_page) = use_loader(move || get_changelog(Some(PAGE_SIZE), None)) else {
        return rsx! {
            div {
                WordNet {}
                div { class: "history", p { "Failed to load change history." } }
            }
        };
    };

    let mut entries = use_signal(Vec::<ChangeLogEntryView>::new);
    let mut exhausted = use_signal(|| false);
    let mut seeded = use_signal(|| false);

    // Seed `entries` from the first page once it loads - mirrors `AddSynsetModal`'s default-value
    // effect, which is why this isn't just read directly in the body below.
    use_effect(move || {
        if !first_page.loading() && !seeded() {
            let page = first_page.read().clone();
            if page.len() < PAGE_SIZE {
                exhausted.set(true);
            }
            entries.set(page);
            seeded.set(true);
        }
    });

    let mut load_more = use_action(move |before: u64| async move {
        get_changelog(Some(PAGE_SIZE), Some(before)).await
    });

    use_effect(move || {
        if let Some(Ok(page)) = load_more.value() {
            let page = page.cloned();
            if page.len() < PAGE_SIZE {
                exhausted.set(true);
            }
            if !page.is_empty() {
                entries.write().extend(page);
            }
        }
    });

    rsx! {
        div {
            WordNet {}
            div {
                class: "history",
                h2 { "Change history" }
                if !seeded() {
                    p { "Loading…" }
                } else if entries().is_empty() {
                    p { "No changes have been recorded yet." }
                } else {
                    for entry in entries().iter().cloned() {
                        div {
                            key: "{entry.id}",
                            class: "history-entry",
                            div { class: "history-timestamp", "{entry.timestamp}" }
                            ul {
                                class: "history-summaries",
                                for summary in entry.summaries.iter() {
                                    li { "{summary}" }
                                }
                            }
                        }
                    }
                    if !exhausted() {
                        button {
                            class: "list-add",
                            r#type: "button",
                            disabled: load_more.pending(),
                            onclick: move |_| {
                                if let Some(oldest) = entries().last() {
                                    load_more.call(oldest.id);
                                }
                            },
                            if load_more.pending() { "Loading…" } else { "Load more" }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(not(feature = "edit"))]
#[component]
pub fn History() -> Element {
    rsx! {
        div {
            WordNet {}
            div { class: "history", p { "History is not available in this build." } }
        }
    }
}
