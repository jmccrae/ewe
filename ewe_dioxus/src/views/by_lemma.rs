use dioxus::prelude::*;
use crate::components::{WordNet, Synset};
use crate::backend::api::get_lemma;

#[component]
pub fn ByLemma(lemma: ReadSignal<String>) -> Element {
    let synsets = use_loader(move || async move  {
        get_lemma(lemma.cloned()).await
    });
    
    if let Ok(synsets) = synsets {
        rsx! {
            div {
                WordNet {},
                match &*synsets.read() {
                    synsets => {
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
                                        synset_id: synset.clone(),
                                        display_ids: false,
                                        display_sensekeys: false,
                                        display_subcats: false,
                                        display_topics: false,
                                        display_pronunciations: false,
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
                                        synset_id: synset.clone(),
                                        display_ids: false,
                                        display_sensekeys: false,
                                        display_subcats: false,
                                        display_topics: false,
                                        display_pronunciations: false,
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
                                        synset_id: synset.clone(),
                                        display_ids: false,
                                        display_sensekeys: false,
                                        display_subcats: false,
                                        display_topics: false,
                                        display_pronunciations: false,
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
                                        synset_id: synset.clone(),
                                        display_ids: false,
                                        display_sensekeys: false,
                                        display_subcats: false,
                                        display_topics: false,
                                        display_pronunciations: false,
                                        focus: lemma
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    } else {
        rsx! {
            div { "Failed to load synsets" }
        }
    }
}
