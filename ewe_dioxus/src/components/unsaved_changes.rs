use dioxus::prelude::*;

/// Whether there are edits not yet reflected in `settings.wordnet_source`. Provided via context
/// (call once from `WNLayout`, mirroring `provide_display_options`) so any component that makes
/// an edit can flip it to `true` immediately, without a server round trip - `UnsavedChangesToast`
/// just reacts to it wherever it's mounted. Seeded from the server once per app load (see
/// `UnsavedChangesToast`), since the database may already have unsaved edits from a previous
/// session.
pub fn provide_dirty_state() -> Signal<bool> {
    use_context_provider(|| Signal::new(false))
}

/// A small fixed banner shown whenever the dirty-state context is `true`, offering "Save"
/// (validates first, showing errors with a "Save anyway" override) and "Revert" (behind a
/// confirmation modal, since it discards every unsaved edit). Mounted once in `WNLayout`.
#[cfg(feature = "edit")]
#[component]
pub fn UnsavedChangesToast() -> Element {
    use crate::backend::edit::{get_dirty, save_lexicon, SaveResult};
    use crate::components::EditProgressBar;
    use crate::Route;

    let mut dirty = use_context::<Signal<bool>>();
    let mut seeded = use_signal(|| false);
    let mut show_revert_confirm = use_signal(|| false);
    let mut saving = use_signal(|| false);
    let mut validation_errors = use_signal(Vec::<String>::new);
    let mut error = use_signal(|| None::<String>);

    let initial = use_loader(get_dirty);
    use_effect(move || {
        if let Ok(loaded) = &initial {
            if !loaded.loading() && !seeded() {
                dirty.set(*loaded.read());
                seeded.set(true);
            }
        }
    });

    let do_save = move |force: bool| {
        spawn(async move {
            saving.set(true);
            error.set(None);
            match save_lexicon(force).await {
                Ok(SaveResult { saved, validation_errors: errs }) => {
                    if saved {
                        validation_errors.set(Vec::new());
                        dirty.set(false);
                    } else {
                        validation_errors.set(errs);
                    }
                }
                Err(e) => error.set(Some(e.to_string())),
            }
            saving.set(false);
        });
    };

    rsx! {
        if dirty() {
            div {
                class: "unsaved-toast",
                div {
                    class: "unsaved-toast-row",
                    span { class: "unsaved-toast-message", "You have unsaved changes." }
                    div {
                        class: "unsaved-toast-actions",
                        button {
                            class: "edit-save",
                            r#type: "button",
                            disabled: saving(),
                            onclick: move |_| do_save(false),
                            if saving() { "Saving…" } else { "Save" }
                        }
                        button {
                            class: "edit-delete",
                            r#type: "button",
                            disabled: saving(),
                            onclick: move |_| show_revert_confirm.set(true),
                            "Revert"
                        }
                    }
                }
                if saving() {
                    EditProgressBar { active: true }
                }
                if let Some(err) = error() {
                    div { class: "edit-error", "{err}" }
                }
                if !validation_errors().is_empty() {
                    div {
                        class: "unsaved-toast-validation",
                        p { "There are validation errors:" }
                        ul {
                            class: "history-summaries",
                            for err in validation_errors().iter() {
                                li { "{err}" }
                            }
                        }
                        button {
                            class: "edit-delete-synset",
                            r#type: "button",
                            disabled: saving(),
                            onclick: move |_| do_save(true),
                            "Save anyway"
                        }
                        button {
                            class: "edit-cancel",
                            r#type: "button",
                            onclick: move |_| validation_errors.set(Vec::new()),
                            "Cancel"
                        }
                    }
                }
            }
        }
        if show_revert_confirm() {
            RevertConfirmModal {
                on_close: move |_| show_revert_confirm.set(false),
                on_reverted: move |_| {
                    dirty.set(false);
                    navigator().push(Route::Home {});
                },
            }
        }
    }
}

#[cfg(not(feature = "edit"))]
#[component]
pub fn UnsavedChangesToast() -> Element {
    rsx! {}
}

#[cfg(feature = "edit")]
#[derive(Clone, PartialEq, Props)]
struct RevertConfirmModalProps {
    on_close: EventHandler<()>,
    on_reverted: EventHandler<()>,
}

#[cfg(feature = "edit")]
#[component]
fn RevertConfirmModal(props: RevertConfirmModalProps) -> Element {
    use crate::backend::edit::revert_lexicon;

    let mut reverting = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    rsx! {
        div {
            class: "modal-overlay",
            onclick: move |_| props.on_close.call(()),
            div {
                class: "modal-content",
                onclick: move |e| e.stop_propagation(),
                div {
                    class: "modal-header",
                    h3 { "Revert all unsaved changes?" }
                    button {
                        class: "modal-close",
                        r#type: "button",
                        title: "Close",
                        onclick: move |_| props.on_close.call(()),
                        "×"
                    }
                }
                p {
                    "This reloads the database from the YAML source files, discarding every "
                    "edit made since the last save. This cannot be undone."
                }
                if let Some(err) = error() {
                    div { class: "edit-error", "{err}" }
                }
                div {
                    class: "modal-actions",
                    button {
                        class: "edit-cancel",
                        r#type: "button",
                        disabled: reverting(),
                        onclick: move |_| props.on_close.call(()),
                        "Cancel"
                    }
                    button {
                        class: "edit-delete-synset",
                        r#type: "button",
                        disabled: reverting(),
                        onclick: move |_| {
                            spawn(async move {
                                reverting.set(true);
                                error.set(None);
                                match revert_lexicon().await {
                                    Ok(_) => {
                                        props.on_reverted.call(());
                                        props.on_close.call(());
                                    }
                                    Err(e) => error.set(Some(e.to_string())),
                                }
                                reverting.set(false);
                            });
                        },
                        if reverting() { "Reverting…" } else { "Revert" }
                    }
                }
            }
        }
    }
}
