use dioxus::prelude::*;

#[derive(Clone, PartialEq, Props)]
pub struct EditableDefinitionProps {
    /// Whether the synset-wide edit toggle (`components::EditToggle`, next to the Wikidata
    /// icon) is on.
    pub editing: bool,
    pub value: String,
    /// Called on every keystroke with the current draft. Nothing is saved until the shared
    /// accept button (`EditToggle`) commits it - this component makes no server calls itself.
    pub on_input: EventHandler<String>,
}

/// The synset definition. While `editing` is on, this renders as a plain text input holding
/// the draft value; otherwise it's just text. Saving (via `Action::Definition`) happens all at
/// once for the whole synset when the shared accept button is clicked - see `EditToggle`.
#[cfg(feature = "edit")]
#[component]
pub fn EditableDefinition(props: EditableDefinitionProps) -> Element {
    rsx! {
        if props.editing {
            span {
                class: "definition definition-editing",
                input {
                    class: "definition-input",
                    r#type: "text",
                    value: "{props.value}",
                    oninput: move |e| props.on_input.call(e.value()),
                }
            }
        } else {
            span {
                class: "definition",
                "{props.value}"
            }
        }
    }
}

#[cfg(not(feature = "edit"))]
#[component]
pub fn EditableDefinition(props: EditableDefinitionProps) -> Element {
    rsx! {
        span {
            class: "definition",
            "{props.value}"
        }
    }
}
