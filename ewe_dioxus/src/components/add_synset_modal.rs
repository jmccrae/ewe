use dioxus::prelude::*;

#[allow(unused_imports)]
use crate::Route;

#[derive(Clone, PartialEq, Props)]
pub struct AddSynsetTriggerProps {
    pub query: String,
}

/// The "+ Add a new synset for '...'" entry always shown at the end of the main search's
/// suggestions - regardless of whether the query also matched existing lemmas/synsets, since
/// the word not being in the dictionary yet is exactly the case this exists for. Owns its own
/// modal-open state so `WordNet` doesn't need any `edit`-feature awareness of its own.
#[cfg(feature = "edit")]
#[component]
pub fn AddSynsetTrigger(props: AddSynsetTriggerProps) -> Element {
    let mut show_modal = use_signal(|| false);

    rsx! {
        div {
            class: "add-synset-suggestion",
            onmousedown: move |_| show_modal.set(true),
            "+ Add a new synset for \"{props.query}\""
        }
        if show_modal() {
            AddSynsetModal {
                initial_lemma: props.query.clone(),
                on_close: move |_| show_modal.set(false),
            }
        }
    }
}

#[cfg(not(feature = "edit"))]
#[component]
#[allow(unused_variables)]
pub fn AddSynsetTrigger(props: AddSynsetTriggerProps) -> Element {
    rsx! {}
}

#[cfg(feature = "edit")]
#[derive(Clone, PartialEq, Debug, Default)]
struct LemmaDraft {
    lemma: String,
    /// Subcategorization frame keys (from `frames.yaml`) - only meaningful, and only shown in
    /// the UI, when the chosen part of speech is verb.
    frames: Vec<String>,
}

#[cfg(feature = "edit")]
#[derive(Clone, PartialEq, Props)]
struct AddSynsetModalProps {
    initial_lemma: String,
    on_close: EventHandler<()>,
}

/// Which part-of-speech values a lexicographer file allows, mirroring
/// `oewn_lib::wordnet::Lexicon::pos_for_lexfile`'s prefix rule exactly (kept in sync manually -
/// there's no server round trip for this since it's a pure function of the lexfile name).
#[cfg(feature = "edit")]
fn pos_options_for_lexfile(lexfile: &str) -> Vec<(&'static str, &'static str)> {
    if lexfile.starts_with("noun") {
        vec![("n", "noun")]
    } else if lexfile.starts_with("verb") {
        vec![("v", "verb")]
    } else if lexfile.starts_with("adv") {
        vec![("r", "adverb")]
    } else if lexfile.starts_with("adj") {
        vec![("a", "adjective"), ("s", "adjective satellite")]
    } else {
        vec![]
    }
}

#[cfg(feature = "edit")]
#[derive(Clone, PartialEq, Props)]
struct LemmaFramesPickerProps {
    /// All frames available to pick from (key, description).
    frames: Vec<(String, String)>,
    /// The frame keys already added for this lemma.
    selected: Vec<String>,
    on_change: EventHandler<Vec<String>>,
}

/// A single lemma's subcategorization frames: an add/remove list matching the "select + Add
/// button + removable chip" pattern used everywhere else in the editor (relations, lemmas,
/// examples), rather than a checkbox per frame - a plain child component (not inlined into the
/// lemma-row loop in `AddSynsetModal`) purely so it can hold its own "pending frame" signal,
/// which a loop body can't do (hooks need a stable per-component call site, not one keyed by
/// loop index).
#[cfg(feature = "edit")]
#[component]
fn LemmaFramesPicker(props: LemmaFramesPickerProps) -> Element {
    let mut pending = use_signal(String::new);
    let available: Vec<(String, String)> = props
        .frames
        .iter()
        .filter(|(key, _)| !props.selected.contains(key))
        .cloned()
        .collect();

    rsx! {
        div {
            class: "add-synset-frames",
            for frame_key in props.selected.iter().cloned() {
                span {
                    key: "{frame_key}",
                    class: "add-synset-frame-chip",
                    "{frame_key}"
                    button {
                        class: "edit-delete",
                        r#type: "button",
                        title: "Remove this frame",
                        onclick: {
                            let frame_key = frame_key.clone();
                            let selected = props.selected.clone();
                            let on_change = props.on_change;
                            move |_| {
                                let updated: Vec<String> = selected
                                    .iter()
                                    .filter(|k| **k != frame_key)
                                    .cloned()
                                    .collect();
                                on_change.call(updated);
                            }
                        },
                        "×"
                    }
                }
            }
            div {
                class: "add-synset-frame-picker",
                select {
                    value: "{pending}",
                    onchange: move |e| pending.set(e.value()),
                    option { value: "", "Choose a frame…" }
                    for (key, desc) in available.iter() {
                        option { value: "{key}", "{key} — {desc}" }
                    }
                }
                button {
                    class: "list-add",
                    r#type: "button",
                    disabled: pending().is_empty(),
                    onclick: {
                        let selected = props.selected.clone();
                        let on_change = props.on_change;
                        move |_| {
                            let key = pending();
                            if !key.is_empty() {
                                let mut updated = selected.clone();
                                updated.push(key);
                                on_change.call(updated);
                                pending.set(String::new());
                            }
                        }
                    },
                    "+ Add subcat"
                }
            }
        }
    }
}

#[cfg(feature = "edit")]
#[component]
fn AddSynsetModal(props: AddSynsetModalProps) -> Element {
    use crate::backend::edit::{add_synset, add_synset_metadata};

    // `Result::Err(Loading)` here means the loader itself couldn't be created (not "still
    // loading" - that's `Loader::loading()` below) - vanishingly rare, and there's nothing
    // useful to show beyond a generic message, so bail out before dealing with anything else.
    let Ok(metadata) = use_loader(add_synset_metadata) else {
        return rsx! {
            div { class: "modal-overlay",
                div { class: "modal-content", "Could not load the form. Please try again." }
            }
        };
    };

    let mut definition = use_signal(String::new);
    let mut lexfile = use_signal(String::new);
    let mut pos = use_signal(String::new);
    let mut lemma_drafts = use_signal({
        let initial_lemma = props.initial_lemma.clone();
        move || vec![LemmaDraft { lemma: initial_lemma.clone(), frames: Vec::new() }]
    });
    let mut submitting = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    // Pick a sensible default lexfile once the list loads, and whenever the lexfile changes,
    // reset `pos` to the (first) valid option for it rather than leaving a stale, possibly
    // invalid value in place (e.g. switching from an adjective file with "s" chosen to a noun
    // file, which only accepts "n").
    {
        let metadata = metadata.clone();
        use_effect(move || {
            if lexfile().is_empty() && !metadata.loading() {
                if let Some(first) = metadata.read().lexfiles.first() {
                    lexfile.set(first.clone());
                }
            }
        });
    }
    use_effect(move || {
        let options = pos_options_for_lexfile(&lexfile());
        if !options.iter().any(|(key, _)| *key == pos()) {
            if let Some((key, _)) = options.first() {
                pos.set(key.to_string());
            }
        }
    });

    let can_submit = !definition().trim().is_empty()
        && !lexfile().is_empty()
        && !pos().is_empty()
        && lemma_drafts().iter().any(|d| !d.lemma.trim().is_empty());

    rsx! {
        div {
            class: "modal-overlay",
            onclick: move |_| props.on_close.call(()),
            div {
                class: "modal-content",
                onclick: move |e| e.stop_propagation(),
                div {
                    class: "modal-header",
                    h3 { "Add a new synset" }
                    button {
                        class: "modal-close",
                        r#type: "button",
                        title: "Close",
                        onclick: move |_| props.on_close.call(()),
                        "×"
                    }
                }
                if metadata.loading() {
                    div { "Loading…" }
                } else {
                    {
                        let loaded = metadata.read();
                        let lexfiles = loaded.lexfiles.clone();
                        let frames = loaded.frames.clone();
                        let pos_options = pos_options_for_lexfile(&lexfile());
                        rsx! {
                            div {
                                class: "add-synset-field",
                                label { "Definition" }
                                textarea {
                                    value: "{definition}",
                                    oninput: move |e| definition.set(e.value()),
                                }
                            }
                            div {
                                class: "add-synset-field",
                                label { "Lexicographer file" }
                                select {
                                    value: "{lexfile}",
                                    onchange: move |e| lexfile.set(e.value()),
                                    for name in lexfiles.iter() {
                                        option { value: "{name}", "{name}" }
                                    }
                                }
                            }
                            if pos_options.len() > 1 {
                                div {
                                    class: "add-synset-field",
                                    label { "Part of speech" }
                                    select {
                                        value: "{pos}",
                                        onchange: move |e| pos.set(e.value()),
                                        for (key, label) in pos_options.iter() {
                                            option { value: "{key}", "{label}" }
                                        }
                                    }
                                }
                            }
                            div {
                                class: "add-synset-field",
                                label { "Lemmas" }
                                for (index, draft) in lemma_drafts().iter().cloned().enumerate() {
                                    div {
                                        key: "{index}",
                                        class: "add-synset-lemma-row",
                                        input {
                                            r#type: "text",
                                            value: "{draft.lemma}",
                                            oninput: move |e| {
                                                let mut drafts = lemma_drafts();
                                                if let Some(d) = drafts.get_mut(index) {
                                                    d.lemma = e.value();
                                                }
                                                lemma_drafts.set(drafts);
                                            },
                                        }
                                        button {
                                            class: "edit-delete",
                                            r#type: "button",
                                            title: "Remove this lemma",
                                            disabled: lemma_drafts().len() <= 1,
                                            onclick: move |_| {
                                                let mut drafts = lemma_drafts();
                                                if drafts.len() > 1 {
                                                    drafts.remove(index);
                                                }
                                                lemma_drafts.set(drafts);
                                            },
                                            "🗑"
                                        }
                                        if pos() == "v" && !frames.is_empty() {
                                            LemmaFramesPicker {
                                                frames: frames.clone(),
                                                selected: draft.frames.clone(),
                                                on_change: move |updated: Vec<String>| {
                                                    let mut drafts = lemma_drafts();
                                                    if let Some(d) = drafts.get_mut(index) {
                                                        d.frames = updated;
                                                    }
                                                    lemma_drafts.set(drafts);
                                                },
                                            }
                                        }
                                    }
                                }
                                button {
                                    class: "list-add",
                                    r#type: "button",
                                    onclick: move |_| {
                                        let mut drafts = lemma_drafts();
                                        drafts.push(LemmaDraft::default());
                                        lemma_drafts.set(drafts);
                                    },
                                    "+ Add lemma"
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
                                    onclick: move |_| props.on_close.call(()),
                                    "Cancel"
                                }
                                button {
                                    class: "edit-save",
                                    r#type: "button",
                                    disabled: !can_submit || submitting(),
                                    onclick: move |_| {
                                        let definition_val = definition();
                                        let lexfile_val = lexfile();
                                        let pos_val = pos();
                                        let drafts = lemma_drafts();
                                        let lemmas: Vec<String> = drafts.iter()
                                            .map(|d| d.lemma.trim().to_string())
                                            .filter(|l| !l.is_empty())
                                            .collect();
                                        let subcats: Vec<Vec<String>> = if pos_val == "v" {
                                            drafts.iter().map(|d| d.frames.clone()).collect()
                                        } else {
                                            Vec::new()
                                        };
                                        spawn(async move {
                                            submitting.set(true);
                                            error.set(None);
                                            match add_synset(
                                                definition_val,
                                                lexfile_val,
                                                Some(oewn_lib::wordnet::PosKey::new(pos_val)),
                                                lemmas,
                                                subcats,
                                            ).await {
                                                Ok(created) => {
                                                    navigator().push(Route::BySynset { synset: created.id.as_str().to_string() });
                                                    props.on_close.call(());
                                                }
                                                Err(e) => error.set(Some(e.to_string())),
                                            }
                                            submitting.set(false);
                                        });
                                    },
                                    if submitting() { "Creating…" } else { "Create synset" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
