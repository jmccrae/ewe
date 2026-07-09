use dioxus::prelude::*;

static CSS: Asset = asset!("/assets/styling/display_options.css");

/// Which optional fields to show alongside each synset entry. Shared via context so the
/// choice persists as the user searches different lemmas.
#[derive(Clone, Copy, PartialEq, Default)]
pub struct DisplayOptions {
    pub show_ids: bool,
    pub show_sensekeys: bool,
    pub show_subcats: bool,
    pub show_topics: bool,
    pub show_pronunciations: bool,
}

/// Whether the [`DisplayOptionsPanel`] is expanded. Kept separate from [`DisplayOptions`]
/// since it's UI state, not a display choice, but shared via context for the same reason:
/// the button and the panel it controls live in different parts of the page.
#[derive(Clone, Copy, PartialEq)]
pub struct ShowOptionsPanel(pub bool);

/// Provide a [`DisplayOptions`] signal to descendant components. Call once from a layout or
/// root component; descendants read/write it with `use_context::<Signal<DisplayOptions>>()`.
pub fn provide_display_options() -> Signal<DisplayOptions> {
    use_context_provider(|| Signal::new(DisplayOptions::default()))
}

/// Provide a [`ShowOptionsPanel`] signal to descendant components. Call once from a layout or
/// root component, alongside [`provide_display_options`].
pub fn provide_panel_visibility() -> Signal<ShowOptionsPanel> {
    use_context_provider(|| Signal::new(ShowOptionsPanel(false)))
}

/// The "Options ▼" toggle button. Meant to sit next to the search bar; toggles the
/// visibility of [`DisplayOptionsPanel`], which floats directly beneath it.
#[component]
pub fn DisplayOptionsButton() -> Element {
    let mut show_panel = use_context::<Signal<ShowOptionsPanel>>();

    rsx! {
        document::Style { href: CSS },
        div {
            class: "wordnet-options",
            a {
                class: if show_panel().0 { "option_button option_button_selected" } else { "option_button" },
                onclick: move |_| show_panel.write().0 = !show_panel().0,
                if show_panel().0 { "Options ▲" } else { "Options ▼" }
            }
            DisplayOptionsPanel {}
        }
    }
}

/// The collapsible panel of checkboxes toggling each field in [`DisplayOptions`]. Shown when
/// [`DisplayOptionsButton`] is toggled open. Mirrors the options panel on <https://en-word.net/>.
#[component]
pub fn DisplayOptionsPanel() -> Element {
    let mut options = use_context::<Signal<DisplayOptions>>();
    let show_panel = use_context::<Signal<ShowOptionsPanel>>();

    rsx! {
        if show_panel().0 {
            div {
                class: "option_panel",
                div {
                    class: "option_panel_internal",
                    label {
                        input {
                            r#type: "checkbox",
                            checked: options().show_ids,
                            onchange: move |e| options.write().show_ids = e.checked(),
                        }
                        "Show Synset Identifier"
                    }
                    label {
                        input {
                            r#type: "checkbox",
                            checked: options().show_sensekeys,
                            onchange: move |e| options.write().show_sensekeys = e.checked(),
                        }
                        "Show Sense Keys"
                    }
                    label {
                        input {
                            r#type: "checkbox",
                            checked: options().show_subcats,
                            onchange: move |e| options.write().show_subcats = e.checked(),
                        }
                        "Show Subcategorization Frames"
                    }
                    label {
                        input {
                            r#type: "checkbox",
                            checked: options().show_topics,
                            onchange: move |e| options.write().show_topics = e.checked(),
                        }
                        "Show Topics"
                    }
                    label {
                        input {
                            r#type: "checkbox",
                            checked: options().show_pronunciations,
                            onchange: move |e| options.write().show_pronunciations = e.checked(),
                        }
                        "Show Pronunciation"
                    }
                }
            }
        }
    }
}
