use dioxus::prelude::*;

#[derive(Clone, PartialEq, Props)]
pub struct EditableSourceProps {
    pub value: String,
    /// Called on every keystroke with the current draft. Nothing is saved until the shared
    /// accept button (`EditToggle`) commits it - this component makes no server calls itself.
    pub on_input: EventHandler<String>,
}

/// The synset's `dc:source`, editable as a single text field. An empty value clears the
/// source. Saving (via `Action::ChangeSource`) happens all at once for the whole synset when
/// the shared accept button is clicked - see `EditToggle`.
#[cfg(feature = "edit")]
#[component]
pub fn EditableSource(props: EditableSourceProps) -> Element {
    rsx! {
        span {
            class: "synset-id-editing",
            b {
                class: "synset-id-title",
                "Source: "
            }
            input {
                class: "synset-source-input",
                r#type: "text",
                placeholder: "Source (optional)",
                value: "{props.value}",
                oninput: move |e| props.on_input.call(e.value()),
            }
        }
    }
}

#[cfg(not(feature = "edit"))]
#[component]
#[allow(unused_variables)]
pub fn EditableSource(props: EditableSourceProps) -> Element {
    rsx! {}
}
