use dioxus::prelude::*;

#[derive(Clone, PartialEq, Props)]
pub struct EditableWikidataProps {
    pub drafts: Vec<String>,
    pub on_drafts_changed: EventHandler<Vec<String>>,
}

/// The synset's Wikidata identifiers (e.g. `Q1234`), editable as a list of text tokens with
/// add/remove controls - similar to `EditableLemmas` but without reordering, since list order
/// carries no meaning here. The caller only mounts this while the synset-wide edit toggle is
/// on, regardless of whether the "Show Synset Identifier" display option is enabled - editing
/// Wikidata links shouldn't require that switch. Nothing here calls the server; saving (via
/// `Action::ChangeWikidata`) happens all at once for the whole synset when the shared accept
/// button is clicked - see `EditToggle`.
#[cfg(feature = "edit")]
#[component]
pub fn EditableWikidata(props: EditableWikidataProps) -> Element {
    let drafts = props.drafts;
    let on_drafts_changed = props.on_drafts_changed;

    rsx! {
        span {
            class: "wikidata-editing",
            b { class: "synset-id-title", "Wikidata: " }
            for (index, wikidata) in drafts.iter().enumerate() {
                span {
                    key: "{index}",
                    class: "wikidata-editing-item",
                    input {
                        class: "wikidata-input",
                        r#type: "text",
                        value: "{wikidata}",
                        oninput: {
                            let drafts = drafts.clone();
                            move |e: FormEvent| {
                                let mut drafts = drafts.clone();
                                if let Some(w) = drafts.get_mut(index) {
                                    *w = e.value();
                                }
                                on_drafts_changed.call(drafts);
                            }
                        },
                    }
                    button {
                        class: "edit-delete",
                        r#type: "button",
                        title: "Remove this Wikidata identifier",
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
                class: "list-add wikidata-add",
                r#type: "button",
                onclick: {
                    let drafts = drafts.clone();
                    move |_| {
                        let mut drafts = drafts.clone();
                        drafts.push(String::new());
                        on_drafts_changed.call(drafts);
                    }
                },
                "+ Add Wikidata ID"
            }
        }
    }
}

#[cfg(not(feature = "edit"))]
#[component]
#[allow(unused_variables)]
pub fn EditableWikidata(props: EditableWikidataProps) -> Element {
    rsx! {}
}
