use dioxus::prelude::*;

#[derive(Clone, PartialEq, Props)]
pub struct EditableIliDefinitionProps {
    pub value: String,
    /// Called on every keystroke with the current draft. Nothing is saved until the shared
    /// accept button (`EditToggle`) commits it - this component makes no server calls itself.
    pub on_input: EventHandler<String>,
}

/// The synset's ILI definition: a standalone English-language gloss, exported as the WN-LMF
/// `ILIDefinition` of a synset with no ILI of its own. Needed in non-English lexicons, where the
/// synset's own definition isn't English; optional in English ones. An empty value clears it.
/// Saving (via `Action::ChangeILIDefinition`) happens all at once for the whole synset when the
/// shared accept button is clicked - see `EditToggle`.
#[cfg(feature = "edit")]
#[component]
pub fn EditableIliDefinition(props: EditableIliDefinitionProps) -> Element {
    rsx! {
        span {
            class: "synset-id-editing",
            b {
                class: "synset-id-title",
                "ILI Definition: "
            }
            input {
                class: "synset-ili-definition-input",
                r#type: "text",
                placeholder: "English gloss (optional)",
                value: "{props.value}",
                oninput: move |e| props.on_input.call(e.value()),
            }
        }
    }
}

#[cfg(not(feature = "edit"))]
#[component]
#[allow(unused_variables)]
pub fn EditableIliDefinition(props: EditableIliDefinitionProps) -> Element {
    rsx! {}
}
