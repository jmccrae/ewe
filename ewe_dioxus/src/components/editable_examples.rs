use dioxus::prelude::*;
use oewn_lib::wordnet::Example;

/// One example row as currently drafted in the editor. Nothing here is saved until the shared
/// accept button (`EditToggle`) commits every field's draft as a single batch.
#[derive(Clone, PartialEq, Debug)]
pub struct ExampleDraft {
    /// `Some(1-indexed original position)` for an existing example - the same numbering
    /// `Action::UpdateExample`/`Action::DeleteExample` use. `None` for a newly-added row that
    /// hasn't been saved yet.
    pub original_number: Option<usize>,
    pub text: String,
    /// Empty means no source; normalized to `None` when building the automaton action (an
    /// empty source is not a valid value).
    pub source: String,
    /// Marked for deletion (existing rows only): hidden from the editing view, but kept around
    /// so the accept handler still knows to emit a `DeleteExample` for it.
    pub deleted: bool,
}

impl ExampleDraft {
    pub fn from_examples(examples: &[Example]) -> Vec<ExampleDraft> {
        examples
            .iter()
            .enumerate()
            .map(|(i, ex)| ExampleDraft {
                original_number: Some(i + 1),
                text: ex.text.clone(),
                source: ex.source.clone().unwrap_or_default(),
                deleted: false,
            })
            .collect()
    }
}

#[derive(Clone, PartialEq, Props)]
pub struct EditableExamplesProps {
    /// Whether the synset-wide edit toggle (`components::EditToggle`, next to the Wikidata
    /// icon) is on.
    pub editing: bool,
    /// The saved examples, shown when not editing.
    pub examples: Vec<Example>,
    /// The draft rows, shown (and edited) while `editing` is on.
    pub drafts: Vec<ExampleDraft>,
    pub on_drafts_changed: EventHandler<Vec<ExampleDraft>>,
}

/// Renders `examples` as plain, non-editable text - the shape shown on the public site and
/// whenever the synset-wide edit toggle is off.
fn plain_examples(examples: &[Example]) -> Element {
    rsx! {
        for (index, example) in examples.iter().enumerate() {
            if let Some(source) = &example.source {
                if source.starts_with("http") {
                    a {
                        class: "example",
                        href: "{source}",
                        "“{example.text}”"
                    }
                } else {
                    span {
                        class: "example",
                        "“{example.text}” ({source})"
                    }
                }
            } else {
                span {
                    class: "example",
                    "“{example.text}”"
                }
            },
            if index < examples.len() - 1 {
                ", "
            }
        }
    }
}

/// The synset's examples. While `editing` is on, every non-deleted draft row renders as its
/// own text+source line with a delete button, plus an "add example" button that appends a new
/// blank row. Nothing here calls the server - deleting just marks a row (or, for a
/// not-yet-saved row, removes it outright); saving happens all at once via `EditToggle`'s
/// accept button.
#[cfg(feature = "edit")]
#[component]
pub fn EditableExamples(props: EditableExamplesProps) -> Element {
    let editing = props.editing;
    let examples = props.examples;
    let drafts = props.drafts;
    let on_drafts_changed = props.on_drafts_changed;

    rsx! {
        if editing {
            for (index, draft) in drafts.iter().enumerate() {
                if !draft.deleted {
                    div {
                        key: "{index}",
                        class: "example example-editing",
                        input {
                            class: "example-text-input",
                            r#type: "text",
                            placeholder: "Example",
                            value: "{draft.text}",
                            oninput: {
                                let drafts = drafts.clone();
                                move |e: FormEvent| {
                                    let mut drafts = drafts.clone();
                                    if let Some(row) = drafts.get_mut(index) {
                                        row.text = e.value();
                                    }
                                    on_drafts_changed.call(drafts);
                                }
                            },
                        }
                        input {
                            class: "example-source-input",
                            r#type: "text",
                            placeholder: "Source (optional)",
                            value: "{draft.source}",
                            oninput: {
                                let drafts = drafts.clone();
                                move |e: FormEvent| {
                                    let mut drafts = drafts.clone();
                                    if let Some(row) = drafts.get_mut(index) {
                                        row.source = e.value();
                                    }
                                    on_drafts_changed.call(drafts);
                                }
                            },
                        }
                        button {
                            class: "edit-delete",
                            r#type: "button",
                            title: if draft.original_number.is_some() { "Delete this example" } else { "Remove this new example" },
                            onclick: {
                                let drafts = drafts.clone();
                                let is_new = draft.original_number.is_none();
                                move |_| {
                                    let mut drafts = drafts.clone();
                                    if is_new {
                                        drafts.remove(index);
                                    } else if let Some(row) = drafts.get_mut(index) {
                                        row.deleted = true;
                                    }
                                    on_drafts_changed.call(drafts);
                                }
                            },
                            "🗑"
                        }
                    }
                }
            }
            button {
                class: "list-add example-add",
                r#type: "button",
                onclick: {
                    let drafts = drafts.clone();
                    move |_| {
                        let mut drafts = drafts.clone();
                        drafts.push(ExampleDraft {
                            original_number: None,
                            text: String::new(),
                            source: String::new(),
                            deleted: false,
                        });
                        on_drafts_changed.call(drafts);
                    }
                },
                "+ Add example"
            }
        } else {
            {plain_examples(&examples)}
        }
    }
}

#[cfg(not(feature = "edit"))]
#[component]
pub fn EditableExamples(props: EditableExamplesProps) -> Element {
    rsx! {
        {plain_examples(&props.examples)}
    }
}
