use dioxus::prelude::*;

#[derive(Clone, PartialEq, Props)]
pub struct EditableLemmasProps {
    /// The draft lemma list. Only rendered while the synset-wide edit toggle is on - the
    /// caller is responsible for showing the normal read-only lemma list (with links, sense
    /// keys, pronunciations, etc.) otherwise, since none of that applies while editing.
    pub drafts: Vec<String>,
    pub on_drafts_changed: EventHandler<Vec<String>>,
}

/// The synset's member lemmas, editable as a compact inline list of text tokens (unlike
/// examples, a lemma is just one string, so there's no separate text/source pair or block
/// layout needed) that can also be dragged to reorder - lemma order is meaningful (it's what
/// `entry_no` superscripts are derived from), so it has to be preserved deliberately rather
/// than left to whatever order edits happened in. Nothing here calls the server - saving
/// happens all at once via `Action::ChangeMembers`, built and submitted by `EditToggle`'s
/// accept button, which also handles the underlying add/delete of entries.
#[cfg(feature = "edit")]
#[component]
pub fn EditableLemmas(props: EditableLemmasProps) -> Element {
    let drafts = props.drafts;
    let on_drafts_changed = props.on_drafts_changed;

    // The index currently being dragged, so `ondrop` on another row knows what to move.
    // Reorders are resolved locally (not via the HTML5 DataTransfer payload) since it's the
    // same list on both ends - simpler and avoids DataTransfer serialization quirks across
    // web/desktop webviews.
    let mut dragging_index = use_signal(|| None::<usize>);

    rsx! {
        span {
            class: "lemmas-editing",
            for (index, lemma) in drafts.iter().enumerate() {
                span {
                    key: "{index}",
                    class: if dragging_index() == Some(index) { "lemma-editing lemma-dragging" } else { "lemma-editing" },
                    draggable: "true",
                    ondragstart: move |_| dragging_index.set(Some(index)),
                    ondragover: move |e| e.prevent_default(),
                    ondragend: move |_| dragging_index.set(None),
                    ondrop: {
                        let drafts = drafts.clone();
                        move |e| {
                            e.prevent_default();
                            if let Some(from) = dragging_index() {
                                if from != index {
                                    let mut drafts = drafts.clone();
                                    let item = drafts.remove(from);
                                    // After removing `from`, `index` (in the original list) is
                                    // already the right splice point regardless of direction:
                                    // dragging downward (from < index) lands the item just
                                    // after where the target used to be; dragging upward lands
                                    // it just before the target.
                                    drafts.insert(index.min(drafts.len()), item);
                                    on_drafts_changed.call(drafts);
                                }
                            }
                            dragging_index.set(None);
                        }
                    },
                    span {
                        class: "lemma-drag-handle",
                        title: "Drag to reorder",
                        "⠿"
                    }
                    input {
                        class: "lemma-input",
                        r#type: "text",
                        value: "{lemma}",
                        oninput: {
                            let drafts = drafts.clone();
                            move |e: FormEvent| {
                                let mut drafts = drafts.clone();
                                if let Some(l) = drafts.get_mut(index) {
                                    *l = e.value();
                                }
                                on_drafts_changed.call(drafts);
                            }
                        },
                    }
                    button {
                        class: "edit-delete",
                        r#type: "button",
                        title: "Remove this lemma",
                        onclick: {
                            let drafts = drafts.clone();
                            move |_| {
                                let mut drafts = drafts.clone();
                                drafts.remove(index);
                                on_drafts_changed.call(drafts);
                            }
                        },
                        "🗑"
                    }
                }
            }
            button {
                class: "list-add lemma-add",
                r#type: "button",
                onclick: {
                    let drafts = drafts.clone();
                    move |_| {
                        let mut drafts = drafts.clone();
                        drafts.push(String::new());
                        on_drafts_changed.call(drafts);
                    }
                },
                "+ Add lemma"
            }
        }
    }
}

#[cfg(not(feature = "edit"))]
#[component]
#[allow(unused_variables)]
pub fn EditableLemmas(props: EditableLemmasProps) -> Element {
    rsx! {}
}
