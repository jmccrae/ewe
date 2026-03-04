use dioxus::prelude::*;
use oewn_lib::wordnet::{SynsetId, MemberSynset/*, SenseRelation*/};
use crate::backend::api::get_synset;
use crate::components::{Subcat,Relation};
use crate::Route;
use std::collections::HashMap;

static CSS: Asset = asset!("/assets/styling/synset.css");

#[derive(PartialEq, Clone, Props)]
pub struct SynsetProps {
    synset_id: ReadSignal<SynsetId>,
    display_ids: bool,
    display_sensekeys: bool,
    display_subcats: bool,
    display_topics: bool,
    display_pronunciations: bool,
    focus: String
}
    
fn subcats(synset: &MemberSynset) -> HashMap<String, Vec<String>> {
    let mut subcat_map = HashMap::new();
    for member in &synset.members {
        for subcat in &member.sense.subcat {
            subcat_map.entry(subcat.clone()).or_insert(Vec::new()).push(member.lemma.clone());
        }
    }
    subcat_map
}

#[component]
fn synset_rels(name: &'static str, rels : Vec<SynsetId>, props : SynsetProps) -> Element {
    rsx! {
        Relation {
            relation_name: name,
            targets: map_ss_rels(rels),
            display_ids: props.display_ids,
            display_sensekeys: props.display_sensekeys,
            display_subcats: props.display_subcats,
            display_topics: props.display_topics,
            display_pronunciations: props.display_pronunciations
        }
    }
}

fn map_ss_rels(rels : Vec<SynsetId>) -> Vec<(SynsetId, Option<String>, Option<String>)> {
    rels.into_iter().map(|ss_id| (ss_id, None, None)).collect()
}

//fn map_se_rels(rels : Vec<SenseRelation>) -> Vec<(SynsetId, Option<String>, Option<String>)> {
//    rels.into_iter().map(|se_rel| (se_rel.target_synset, Some(se_rel.source_lemma), Some(se_rel.target_lemma))).collect()
//}

#[component]
pub fn Synset(props : SynsetProps) -> Element {
    let synset = use_loader(move || async move {
        get_synset(props.synset_id.cloned()).await
    });

    let mut show_relations = use_signal(|| false);

    if let Ok(ss_load) = synset {
        if ss_load.loading() {
            rsx! {
                div {
                    "Loading..."
                }
            }
        } else {
            if let Some(synset) = &*ss_load.read() {
                rsx! {
                    document::Style { href: CSS },
                    div { 
                        class: "synset", 
                        if props.display_ids {
                            div {
                                class: "synset-id",
                                // show: display.ids
                                span {
                                    class: "identifier",
                                    "{synset.id}"
                                }
                                if synset.ili.is_some() || !synset.wikidata.is_empty() {
                                    span {
                                        "("
                                    }
                                }
                                if let Some(ref ili) = synset.ili {
                                    span {
                                        b {
                                            class: "synset-id-title",
                                            "Interlingual Index:"
                                        },
                                        span {
                                            class: "identifier",
                                            "{ili}"
                                        }
                                    }
                                }
                                if synset.ili.is_some() && !synset.wikidata.is_empty() {
                                    span {
                                        ", "
                                    }
                                }
                                for (idx, wikidata) in synset.wikidata.iter().enumerate() {
                                    span {
                                        b {
                                            "Wikidata:"
                                        },
                                        a {
                                            class: "identifier",
                                            href: "https://www.wikidata.org/wiki/{wikidata}",
                                            "{wikidata}"
                                        },
                                        if idx < synset.wikidata.len() - 1 {
                                            ", "
                                        }
                                    }
                                }
                                if synset.ili.is_some() || !synset.wikidata.is_empty() {
                                    span {
                                        ")"
                                    }
                                }

                            },
                        },
                        div {
                            class: "lemmas",
                            span {
                                class: "pos",
                                "({synset.part_of_speech})"
                            },
                            for (index, member) in synset.members.iter().enumerate() {
                                span {
                                    class: "lemma",
                                    Link {
                                        to: Route::ByLemma { lemma: member.lemma.clone() },
                                        class: if member.lemma == props.focus {
                                            "focus"
                                        } else {
                                            "unfocused"
                                        },
                                        "{member.lemma}"
                                    },
                                    if let Some(entry_no) = member.entry_no {
                                        sup {
                                            "{entry_no}"
                                        }
                                    }
                                    if props.display_sensekeys {
                                        span {
                                            class: "sense_key",
                                            "{member.sense.id}"
                                        }
                                    }
                                    if props.display_pronunciations && member.pronunciation.len() > 0 {
                                        span {
                                            class: "pronunciations",
                                            "(Pronunciation: ",
                                            for (i, pron) in member.pronunciation.iter().enumerate() {
                                                if let Some(variety) = &pron.variety {
                                                    span {
                                                        class: "pronunciation_variety",
                                                        "{variety}"
                                                    }
                                                }
                                                "{pron.value}",
                                                if i < member.pronunciation.len() - 1 {
                                                    ", " 
                                                }
                                            },
                                            ")"
                                        }
                                    }
                                    if index < synset.members.len() - 1 {
                                        ", "
                                    }
                                }
                            },
                            span {
                                class: "definition",
                                "{synset.definition.get(0).unwrap_or(&String::from(\"\"))}"
                            }
                            for (index, example) in synset.example.iter().enumerate() {
                                if let Some(source) = &example.source {
                                    if source.starts_with("http") {
                                        a {
                                            class: "example",
                                            href: "{source}",
                                            "“{example.text}”"
                                        }
                                    } else {
                                        span {
                                            class: "example",
                                            "“{example.text}” ({source})"
                                        }
                                    }
                                } else {
                                    span {
                                        class: "example",
                                        "“{example.text}”"
                                    }
                                },
                                if index < synset.example.len() - 1 {
                                    ", "
                                }
                            },
                            if props.display_topics {
                                div {
                                    class: "topics",
                                    b { "Topics: " },
                                    "{synset.lexname}"
                                }
                            },
                            if props.display_subcats {
                                Subcat {
                                    subcats: subcats(synset)
                                }
                            },
                            if show_relations() {
                                div {
                                    class: "relations",
                                    synset_rels {
                                        name: "Hypernyms", 
                                        rels: synset.hypernym.clone(),
                                        props: props.clone()
                                    },
                                    synset_rels {
                                        name: "Hyponyms",
                                        rels: synset.hyponym.clone(),
                                        props: props.clone()
                                    }
                                }
                            } else {
                                div {
                                    class: "more",
                                    a {
                                       onclick: move |_| show_relations.toggle(),
                                       "MORE ▶"
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                rsx! {
                    div {
                        class: "synset",
                        "No synset found"
                    }
                }
            }
        }
    } else {
        eprintln!("Error loading synset {:?}", synset);
        rsx! {
            div {
                class: "synset",
                "Error loading synset"
            }
        }
    }
}
