use dioxus::prelude::*;

/// The "Validate" footer link - runs the validator against the current lexicon on demand,
/// independent of saving (`save_lexicon` also validates as a save-time gate; this is just a
/// standalone sanity check). Owns its own modal-open state, same as the other footer/toolbar
/// triggers.
#[cfg(feature = "edit")]
#[component]
pub fn ValidateButton() -> Element {
    let mut show_modal = use_signal(|| false);

    rsx! {
        a {
            class: "footer-action",
            onclick: move |_| show_modal.set(true),
            "Validate"
        }
        if show_modal() {
            ValidateModal { on_close: move |_| show_modal.set(false) }
        }
    }
}

#[cfg(not(feature = "edit"))]
#[component]
pub fn ValidateButton() -> Element {
    rsx! {}
}

#[cfg(feature = "edit")]
#[derive(Clone, PartialEq, Props)]
struct ValidateModalProps {
    on_close: EventHandler<()>,
}

#[cfg(feature = "edit")]
#[component]
fn ValidateModal(props: ValidateModalProps) -> Element {
    use crate::backend::edit::validate_lexicon;
    use crate::components::EditProgressBar;

    let results = use_loader(validate_lexicon);

    // `use_loader` returns `Err(Loading::Pending)` - not `Ok` with `.loading()` true - for as
    // long as the future hasn't resolved yet; validating the full lexicon can easily take over a
    // minute, so this is the common case for most of a modal's lifetime, not a rare transient.
    let is_validating = match &results {
        Err(dioxus_fullstack::Loading::Pending(_)) => true,
        Ok(loaded) => loaded.loading(),
        _ => false,
    };

    rsx! {
        div {
            class: "modal-overlay",
            onclick: move |_| props.on_close.call(()),
            div {
                class: "modal-content",
                onclick: move |e| e.stop_propagation(),
                div {
                    class: "modal-header",
                    h3 { "Validation results" }
                    button {
                        class: "modal-close",
                        r#type: "button",
                        title: "Close",
                        onclick: move |_| props.on_close.call(()),
                        "×"
                    }
                }
                if is_validating {
                    p { "Validating…" }
                    EditProgressBar { active: true }
                } else {
                    match &results {
                        Ok(loaded) => {
                            let errors = loaded.read().clone();
                            if errors.is_empty() {
                                rsx! { p { "No validation errors." } }
                            } else {
                                rsx! {
                                    ul {
                                        class: "history-summaries",
                                        for err in errors.iter() {
                                            li { "{err}" }
                                        }
                                    }
                                }
                            }
                        }
                        Err(_) => rsx! { p { "Failed to run the validator." } },
                    }
                }
            }
        }
    }
}
