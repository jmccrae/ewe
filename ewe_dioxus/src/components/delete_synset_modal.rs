use dioxus::prelude::*;

#[allow(unused_imports)]
use oewn_lib::wordnet::SynsetId;
#[allow(unused_imports)]
use crate::Route;

#[derive(Clone, PartialEq, Props)]
pub struct DeleteSynsetButtonProps {
    pub synset_id: SynsetId,
}

/// The delete button shown next to `EditToggle`'s accept/reject pair while editing - a separate
/// action from the batch those two commit, since deleting the synset outright has nothing to do
/// with (and shouldn't be gated behind) whatever definition/example/lemma/relation drafts happen
/// to be pending. Owns its own modal-open state, same as `AddSynsetTrigger`.
#[cfg(feature = "edit")]
#[component]
pub fn DeleteSynsetButton(props: DeleteSynsetButtonProps) -> Element {
    let mut show_modal = use_signal(|| false);

    rsx! {
        button {
            class: "edit-delete edit-toggle-btn",
            r#type: "button",
            title: "Delete this synset",
            onclick: move |_| show_modal.set(true),
            span { class: "delete-synset-icon", "🗑" }
        }
        if show_modal() {
            DeleteSynsetModal {
                synset_id: props.synset_id.clone(),
                on_close: move |_| show_modal.set(false),
            }
        }
    }
}

#[cfg(not(feature = "edit"))]
#[component]
#[allow(unused_variables)]
pub fn DeleteSynsetButton(props: DeleteSynsetButtonProps) -> Element {
    rsx! {}
}

#[cfg(feature = "edit")]
#[derive(Clone, PartialEq, Props)]
struct DeleteSynsetModalProps {
    synset_id: SynsetId,
    on_close: EventHandler<()>,
}

/// Two ways to delete a synset:
/// - Deprecate & delete: the traditional, deliberate-edit path - hands off entries/examples/
///   relations to `superseded_by` and leaves a deprecation record. Requires both a reason and a
///   superseding synset (picked the same way relation targets are, since the user can't be
///   expected to know synset ids).
/// - Delete permanently: no reason or superseding synset needed - for a synset a user just
///   created through this same UI and immediately decided against, where there's nothing
///   meaningful to hand off or record.
#[cfg(feature = "edit")]
#[component]
fn DeleteSynsetModal(props: DeleteSynsetModalProps) -> Element {
    use crate::backend::edit::{delete_synset, SynsetCandidate};
    use crate::components::SynsetPicker;

    let mut dirty = use_context::<Signal<bool>>();
    let mut reason = use_signal(String::new);
    let mut superseding = use_signal(|| None::<SynsetCandidate>);
    let mut submitting = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    let can_deprecate = !reason().trim().is_empty() && superseding().is_some();

    rsx! {
        div {
            class: "modal-overlay",
            onclick: move |_| props.on_close.call(()),
            div {
                class: "modal-content",
                onclick: move |e| e.stop_propagation(),
                div {
                    class: "modal-header",
                    h3 { "Delete this synset" }
                    button {
                        class: "modal-close",
                        r#type: "button",
                        title: "Close",
                        onclick: move |_| props.on_close.call(()),
                        "×"
                    }
                }
                div {
                    class: "add-synset-field",
                    label { "Reason" }
                    input {
                        r#type: "text",
                        placeholder: "Why is this synset being deleted?",
                        value: "{reason}",
                        oninput: move |e| reason.set(e.value()),
                    }
                }
                div {
                    class: "add-synset-field",
                    label { "Superseding synset" }
                    if let Some(target) = superseding() {
                        div {
                            class: "relation-selected-target",
                            "→ {target.members.join(\", \")} ({target.part_of_speech}) — {target.definition}"
                            button {
                                r#type: "button",
                                class: "edit-delete",
                                title: "Change target",
                                onclick: move |_| superseding.set(None),
                                "×"
                            }
                        }
                    } else {
                        SynsetPicker {
                            placeholder: "Search for the synset that supersedes this one…",
                            on_select: move |candidate| superseding.set(Some(candidate)),
                        }
                    }
                }
                if let Some(err) = error() {
                    div { class: "edit-error", "{err}" }
                }
                div {
                    class: "modal-actions",
                    button {
                        class: "edit-cancel",
                        r#type: "button",
                        disabled: submitting(),
                        onclick: move |_| props.on_close.call(()),
                        "Cancel"
                    }
                    button {
                        class: "edit-delete-synset",
                        r#type: "button",
                        title: "No reason or superseding synset needed - removes this synset with no deprecation record",
                        disabled: submitting(),
                        onclick: {
                            let synset_id = props.synset_id.clone();
                            move |_| {
                                let synset_id = synset_id.clone();
                                spawn(async move {
                                    submitting.set(true);
                                    error.set(None);
                                    match delete_synset(synset_id, String::new(), None).await {
                                        Ok(_) => {
                                            dirty.set(true);
                                            navigator().push(Route::Home {});
                                            props.on_close.call(());
                                        }
                                        Err(e) => error.set(Some(e.to_string())),
                                    }
                                    submitting.set(false);
                                });
                            }
                        },
                        "Delete permanently"
                    }
                    button {
                        class: "edit-save",
                        r#type: "button",
                        disabled: !can_deprecate || submitting(),
                        onclick: {
                            let synset_id = props.synset_id.clone();
                            move |_| {
                                let synset_id = synset_id.clone();
                                let reason_val = reason();
                                let target_id = superseding().map(|t| t.id);
                                spawn(async move {
                                    submitting.set(true);
                                    error.set(None);
                                    match delete_synset(synset_id, reason_val, target_id).await {
                                        Ok(navigate_to) => {
                                            dirty.set(true);
                                            match navigate_to {
                                                Some(id) => {
                                                    navigator().push(Route::BySynset { synset: id.as_str().to_string() });
                                                }
                                                None => {
                                                    navigator().push(Route::Home {});
                                                }
                                            }
                                            props.on_close.call(());
                                        }
                                        Err(e) => error.set(Some(e.to_string())),
                                    }
                                    submitting.set(false);
                                });
                            }
                        },
                        if submitting() { "Deleting…" } else { "Deprecate & delete" }
                    }
                }
            }
        }
    }
}
