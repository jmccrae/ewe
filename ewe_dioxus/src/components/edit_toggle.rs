use dioxus::prelude::*;

#[derive(Clone, PartialEq, Props)]
pub struct EditToggleProps {
    pub editing: bool,
    /// Disables accept/reject while a batch of edits is in flight.
    pub saving: bool,
    /// Click the pencil to enter edit mode.
    pub on_enter: EventHandler<()>,
    /// Click the accept (✓) button to commit every pending edit across all fields as one
    /// batch and leave edit mode.
    pub on_accept: EventHandler<()>,
    /// Click the reject (×) button to discard every pending edit and leave edit mode.
    pub on_reject: EventHandler<()>,
    /// Extra controls (currently just `DeleteSynsetButton`) rendered alongside the accept/
    /// reject pair while editing - grouped here, rather than as a separate sibling, so every
    /// action for this synset lives in the same `edit-toggle-actions` row.
    pub children: Element,
}

/// The synset-wide edit control, styled and positioned like the Wikidata icon it sits next to.
/// Off, it's a pencil that enters edit mode (currently unlocking `EditableDefinition` and
/// `EditableExamples`; eventually lemmas and relations too). On, the pencil is replaced by a
/// single accept/reject pair that applies to every field's pending draft at once - individual
/// fields don't have their own save buttons.
#[cfg(feature = "edit")]
#[component]
pub fn EditToggle(props: EditToggleProps) -> Element {
    rsx! {
        if props.editing {
            span {
                class: "edit-toggle-actions",
                button {
                    class: "edit-save edit-toggle-btn",
                    r#type: "button",
                    title: "Accept all changes",
                    disabled: props.saving,
                    onclick: move |_| props.on_accept.call(()),
                    "✓"
                }
                button {
                    class: "edit-cancel edit-toggle-btn",
                    r#type: "button",
                    title: "Discard all changes",
                    disabled: props.saving,
                    onclick: move |_| props.on_reject.call(()),
                    "×"
                }
                {props.children}
            }
        } else {
            div {
                class: "edit-toggle",
                title: "Edit this synset",
                onclick: move |_| props.on_enter.call(()),
                "🖉"
            }
        }
    }
}

#[cfg(not(feature = "edit"))]
#[component]
#[allow(unused_variables)]
pub fn EditToggle(props: EditToggleProps) -> Element {
    rsx! {}
}
