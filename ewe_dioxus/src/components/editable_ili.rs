use dioxus::prelude::*;

#[derive(Clone, PartialEq, Props)]
pub struct EditableIliProps {
    pub value: String,
    /// Called on every keystroke with the current draft. Nothing is saved until the shared
    /// accept button (`EditToggle`) commits it - this component makes no server calls itself.
    pub on_input: EventHandler<String>,
}

/// The synset's Interlingual Index (ILI) identifier, editable as a single text field. The
/// caller only mounts this while the synset-wide edit toggle is on, regardless of whether the
/// "Show Synset Identifier" display option is enabled - editing the ILI shouldn't require
/// that switch. Saving (via `Action::ChangeILI`) happens all at once for the whole synset when
/// the shared accept button is clicked - see `EditToggle`.
#[cfg(feature = "edit")]
#[component]
pub fn EditableIli(props: EditableIliProps) -> Element {
    rsx! {
        span {
            class: "synset-id-editing",
            b {
                class: "synset-id-title",
                "Interlingual Index: "
            }
            input {
                class: "ili-input",
                r#type: "text",
                value: "{props.value}",
                oninput: move |e| props.on_input.call(e.value()),
            }
        }
    }
}

#[cfg(not(feature = "edit"))]
#[component]
#[allow(unused_variables)]
pub fn EditableIli(props: EditableIliProps) -> Element {
    rsx! {}
}
