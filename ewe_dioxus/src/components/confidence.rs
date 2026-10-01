//! WN-LMF confidence scores (`confidenceScore`): a read-only badge, and the input used for
//! them while the synset-wide edit toggle is on.
//!
//! No score at all means the WN-LMF default of 1.0, and a score of exactly 1.0 says the same
//! thing - neither is worth any visual noise, so the badge only appears below 1.0.

use dioxus::prelude::*;

const TITLE: &str = "Confidence score (WN-LMF confidenceScore)";

#[component]
pub fn ConfidenceBadge(value: Option<f64>) -> Element {
    match value.filter(|c| *c < 1.0) {
        Some(c) => rsx! {
            span {
                class: "confidence",
                title: TITLE,
                "conf. {c}"
            }
        },
        None => rsx! {},
    }
}

/// A small number field for a draft score. Empty means no score.
#[component]
pub fn ConfidenceInput(value: String, on_input: EventHandler<String>) -> Element {
    rsx! {
        input {
            class: "confidence-input",
            r#type: "number",
            min: "0",
            max: "1",
            step: "0.05",
            placeholder: "conf.",
            title: "{TITLE} - between 0 and 1, or empty for none",
            value: "{value}",
            oninput: move |e| on_input.call(e.value()),
        }
    }
}

/// The draft text for a saved score.
pub fn confidence_draft(value: Option<f64>) -> String {
    value.map(|c| c.to_string()).unwrap_or_default()
}

/// Parses a draft back into a score - `None` for an empty draft, an error (to show the user,
/// rather than silently dropping their edit) for anything that isn't a number in 0-1.
pub fn parse_confidence_draft(draft: &str) -> Result<Option<f64>, String> {
    let draft = draft.trim();
    if draft.is_empty() {
        return Ok(None);
    }
    let c: f64 = draft
        .parse()
        .map_err(|_| format!("Confidence {draft:?} is not a number"))?;
    if ewe_lib::validate::is_valid_confidence(c) {
        Ok(Some(c))
    } else {
        Err(format!("Confidence {c} is not between 0 and 1"))
    }
}
