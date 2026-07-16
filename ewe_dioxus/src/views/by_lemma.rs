use dioxus::prelude::*;
use crate::components::{WordNet, Synset, DisplayOptions, DownloadLinks};
use crate::backend::api::get_lemma;

#[component]
pub fn ByLemma(lemma: ReadSignal<String>) -> Element {
    let synsets = use_server_future(move ||  {
        let value = lemma.cloned();
        async move  {
            get_lemma(value).await
        }
    })?;
    let options = use_context::<Signal<DisplayOptions>>();

    let x = if let Some(Ok(synsets)) = &*synsets.read() {
        rsx! {
            div {
                WordNet {},
                {
                    let (nouns, rest) = synsets.into_iter().partition::<Vec<_>, _>(|s| s.as_str().ends_with('n'));
                    let (verbs, rest) = rest.into_iter().partition::<Vec<_>, _>(|s| s.as_str().ends_with('v'));
                    let (adjectives, adverbs) = rest.into_iter().partition::<Vec<_>, _>(|s| s.as_str().ends_with('a') || s.as_str().ends_with('s'));
                    rsx!{
                        if nouns.len() > 0 {
                            h3 { 
                                class: "pos_label",
                                "Nouns" 
                            },
                            for synset in nouns.into_iter() {
                                Synset {
                                    key: "{synset}",
                                    synset_id: synset.clone(),
                                    display_ids: options().show_ids,
                                    display_sensekeys: options().show_sensekeys,
                                    display_subcats: options().show_subcats,
                                    display_topics: options().show_topics,
                                    display_pronunciations: options().show_pronunciations,
                                    focus: lemma
                                }
                            }
                        }
                        if verbs.len() > 0 {
                            h3 { 
                                class: "pos_label",
                                "Verbs" 
                            },
                            for synset in verbs.into_iter() {
                                Synset {
                                    key: "{synset}",
                                    synset_id: synset.clone(),
                                    display_ids: options().show_ids,
                                    display_sensekeys: options().show_sensekeys,
                                    display_subcats: options().show_subcats,
                                    display_topics: options().show_topics,
                                    display_pronunciations: options().show_pronunciations,
                                    focus: lemma
                                }
                            }
                        }
                        if adjectives.len() > 0 {
                            h3 { 
                                class: "pos_label",
                                "Adjectives" 
                            },
                            for synset in adjectives.into_iter() {
                                Synset {
                                    key: "{synset}",
                                    synset_id: synset.clone(),
                                    display_ids: options().show_ids,
                                    display_sensekeys: options().show_sensekeys,
                                    display_subcats: options().show_subcats,
                                    display_topics: options().show_topics,
                                    display_pronunciations: options().show_pronunciations,
                                    focus: lemma
                                }
                            }
                        }
                        if adverbs.len() > 0 {
                            h3 { 
                                class: "pos_label",
                                "Adverbs" 
                            },
                            for synset in adverbs.into_iter() {
                                Synset {
                                    key: "{synset}",
                                    synset_id: synset.clone(),
                                    display_ids: options().show_ids,
                                    display_sensekeys: options().show_sensekeys,
                                    display_subcats: options().show_subcats,
                                    display_topics: options().show_topics,
                                    display_pronunciations: options().show_pronunciations,
                                    focus: lemma
                                }
                            }
                        }
                    }
                }
                DownloadLinks { kind: "lemma", id: lemma.cloned() },
            }
        }
    } else {
        rsx! {
            div { "Failed to load synsets" }
        }
    };
    x
}
