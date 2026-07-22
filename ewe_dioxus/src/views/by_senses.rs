use dioxus::prelude::*;
use crate::backend::api::get_synset;
use crate::backend::senses::get_sense_concordance;
use crate::components::{ProjectName, WordNet};
use crate::Route;
use ewe_lib::wordnet::{MemberSynset, SynsetId};

static CSS: Asset = asset!("/assets/styling/senses.css");

/// "Occurrences of 'cat', 'true cat' (feline mammal usually having thick
/// soft fur and no ability to roar)" - falls back to the bare id if the
/// synset hasn't loaded (or doesn't exist).
fn heading_text(synset: Option<MemberSynset>, id: &str) -> String {
    let Some(synset) = synset else {
        return format!("Occurrences of {id}");
    };
    let lemmas: Vec<String> = synset.members.iter().map(|m| format!("'{}'", m.lemma)).collect();
    let lemma_text = if lemmas.is_empty() { id.to_string() } else { lemmas.join(", ") };
    match synset.definition.first() {
        Some(definition) => format!("Occurrences of {lemma_text} ({definition})"),
        None => format!("Occurrences of {lemma_text}"),
    }
}

#[component]
pub fn BySenses(id: ReadSignal<String>, page: ReadSignal<usize>) -> Element {
    let synset = use_loader(move || {
        let value = id.cloned();
        async move { get_synset(SynsetId::new_owned(value)).await }
    });
    let heading = match &synset {
        Ok(loaded) if !loaded.loading() => heading_text(loaded.read().clone(), &id.cloned()),
        _ => format!("Occurrences of {}", id.cloned()),
    };

    let concordance = use_loader(move || {
        let value = id.cloned();
        let page = page.cloned();
        async move { get_sense_concordance(SynsetId::new_owned(value), page).await }
    });
    let project_name = use_context::<Signal<ProjectName>>();

    rsx! {
        if !project_name().0.is_empty() {
            document::Title { "{id} - {project_name().0}" }
        }
        document::Style { href: CSS },
        div {
            WordNet {},
            div {
                class: "senses",
                h3 { "{heading}" }
                if let Ok(conc_loaded) = &concordance {
                    if conc_loaded.loading() {
                        p { "Loading..." }
                    } else {
                        {
                            let concordance = &*conc_loaded.read();
                            if concordance.total == 0 {
                                rsx! {
                                    p { "No occurrences found." }
                                }
                            } else {
                                rsx! {
                                    table {
                                        class: "concordance",
                                        tbody {
                                            for (index, line) in concordance.lines.iter().enumerate() {
                                                tr {
                                                    key: "{index}",
                                                    title: "{line.doc_id}",
                                                    td { class: "concordance-left", "{line.left}" }
                                                    td { class: "concordance-target", "{line.target}" }
                                                    td { class: "concordance-right", "{line.right}" }
                                                }
                                            }
                                        }
                                    }
                                    div {
                                        class: "concordance-pagination",
                                        if concordance.page > 0 {
                                            Link {
                                                to: Route::BySenses { id: id.cloned(), page: concordance.page - 1 },
                                                "◀ Previous"
                                            }
                                        } else {
                                            span { class: "disabled", "◀ Previous" }
                                        }
                                        span { class: "concordance-page-label", "Page {concordance.page + 1} of {concordance.total_pages}" }
                                        if concordance.page + 1 < concordance.total_pages {
                                            Link {
                                                to: Route::BySenses { id: id.cloned(), page: concordance.page + 1 },
                                                "Next ▶"
                                            }
                                        } else {
                                            span { class: "disabled", "Next ▶" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    p { "Failed to load occurrences" }
                }
            }
        }
    }
}
