use dioxus::prelude::*;
use oewn_lib::wordnet::{SynsetId, MemberSynset, SenseRelation};
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

#[component]
fn sense_rels(name: &'static str, rels : Vec<SenseRelation>, props : SynsetProps) -> Element {
    rsx! {
        Relation {
            relation_name: name,
            targets: map_se_rels(rels),
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

fn map_se_rels(rels : Vec<SenseRelation>) -> Vec<(SynsetId, Option<String>, Option<String>)> {
    rels.into_iter().map(|se_rel| (se_rel.target_synset, Some(se_rel.source_lemma), Some(se_rel.target_lemma))).collect()
}

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
                                    if !synset.hypernym.is_empty() {
                                        synset_rels {
                                            name: "Hypernyms", 
                                            rels: synset.hypernym.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.hyponym.is_empty() {
                                        synset_rels {
                                            name: "Hyponyms",
                                            rels: synset.hyponym.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.instance_hypernym.is_empty() {
                                         synset_rels {
                                            name: "Instance Of",
                                            rels: synset.instance_hypernym.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.instance_hyponym.is_empty() {
                                         synset_rels {
                                            name: "Has Instance",
                                            rels: synset.instance_hyponym.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.attribute.is_empty() {
                                         synset_rels {
                                            name: "Attributes",
                                            rels: synset.attribute.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.causes.is_empty() {
                                         synset_rels {
                                            name: "Causes",
                                            rels: synset.causes.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.is_caused_by.is_empty() {
                                         synset_rels {
                                            name: "Is Caused By",
                                            rels: synset.is_caused_by.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.domain_region.is_empty() {
                                         synset_rels {
                                            name: "Used in Region",
                                            rels: synset.domain_region.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.has_domain_region.is_empty() {
                                         synset_rels {
                                            name: "Used in this Region",
                                            rels: synset.has_domain_region.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.domain_topic.is_empty() {
                                         synset_rels {
                                            name: "Subject",
                                            rels: synset.domain_topic.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.has_domain_topic.is_empty() {
                                         synset_rels {
                                            name: "Is a Subject of",
                                            rels: synset.has_domain_topic.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.exemplifies.is_empty() {
                                         synset_rels {
                                            name: "Is an Example Of",
                                            rels: synset.exemplifies.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.is_exemplified_by.is_empty() {
                                         synset_rels {
                                            name: "Has Example",
                                            rels: synset.is_exemplified_by.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.entails.is_empty() {
                                         synset_rels {
                                            name: "Entails",
                                            rels: synset.entails.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.is_entailed_by.is_empty() {
                                         synset_rels {
                                            name: "Is Entailed By",
                                            rels: synset.is_entailed_by.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.mero_location.is_empty() {
                                         synset_rels {
                                            name: "Is Located At",
                                            rels: synset.mero_location.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.holo_location.is_empty() {
                                         synset_rels {
                                            name: "Location Of",
                                            rels: synset.holo_location.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.holo_member.is_empty() {
                                         synset_rels {
                                            name: "Is Member Of",
                                            rels: synset.holo_member.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.mero_member.is_empty() {
                                         synset_rels {
                                            name: "Has Member",
                                            rels: synset.mero_member.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.holo_part.is_empty() {
                                         synset_rels {
                                            name: "Is Part Of",
                                            rels: synset.holo_part.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.mero_part.is_empty() {
                                         synset_rels {
                                            name: "Has Part",
                                            rels: synset.mero_part.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.holo_substance.is_empty() {
                                         synset_rels {
                                            name: "Is Made Of",
                                            rels: synset.holo_substance.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.mero_substance.is_empty() {
                                         synset_rels {
                                            name: "Makes",
                                            rels: synset.mero_substance.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.meronym.is_empty() {
                                         synset_rels {
                                            name: "Meronyms",
                                            rels: synset.meronym.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.holonym.is_empty() {
                                         synset_rels {
                                            name: "Holonyms",
                                            rels: synset.holonym.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.similar.is_empty() {
                                         synset_rels {
                                            name: "Similar To",
                                            rels: synset.similar.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.feminine.is_empty() {
                                         synset_rels {
                                            name: "Feminine Form",
                                            rels: synset.feminine.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.masculine.is_empty() {
                                         synset_rels {
                                            name: "Masculine Form",
                                            rels: synset.masculine.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.also.is_empty() {
                                        synset_rels {
                                            name: "See Also",
                                            rels: synset.also.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.other.is_empty() {
                                         synset_rels {
                                            name: "Other Related Synsets",
                                            rels: synset.other.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.antonym.is_empty() {
                                         sense_rels {
                                            name: "Antonyms",
                                            rels: synset.antonym.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.participle.is_empty() {
                                         sense_rels {
                                            name: "Participles",
                                            rels: synset.participle.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.pertainym.is_empty() {
                                         sense_rels {
                                            name: "Of or Pertaining To",
                                            rels: synset.pertainym.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.derivation.is_empty() {
                                         sense_rels {
                                            name: "Derived From",
                                            rels: synset.derivation.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.exemplifies_sense.is_empty() {
                                         sense_rels {
                                            name: "Is an Example Of",
                                            rels: synset.exemplifies_sense.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.is_exemplified_by_sense.is_empty() {
                                         sense_rels {
                                            name: "Has Example",
                                            rels: synset.is_exemplified_by_sense.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.agent.is_empty() {
                                         sense_rels {
                                            name: "Agent",
                                            rels: synset.agent.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.material.is_empty() {
                                         sense_rels {
                                            name: "Material",
                                            rels: synset.material.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.event.is_empty() {
                                         sense_rels {
                                            name: "Event",
                                            rels: synset.event.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.instrument.is_empty() {
                                         sense_rels {
                                            name: "Instrument",
                                            rels: synset.instrument.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.location.is_empty() {
                                         sense_rels {
                                            name: "Location",
                                            rels: synset.location.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.by_means_of.is_empty() {
                                         sense_rels {
                                            name: "By Means Of",
                                            rels: synset.by_means_of.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.undergoer.is_empty() {
                                         sense_rels {
                                            name: "Undergoer",
                                            rels: synset.undergoer.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.property.is_empty() {
                                         sense_rels {
                                            name: "Property",
                                            rels: synset.property.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.result.is_empty() {
                                         sense_rels {
                                            name: "Result",
                                            rels: synset.result.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.state.is_empty() {
                                         sense_rels {
                                            name: "State",
                                            rels: synset.state.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.uses.is_empty() {
                                         sense_rels {
                                            name: "Uses",
                                            rels: synset.uses.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.destination.is_empty() {
                                         sense_rels {
                                            name: "Destination",
                                            rels: synset.destination.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.body_part.is_empty() {
                                         sense_rels {
                                            name: "Body Part",
                                            rels: synset.body_part.clone(),
                                            props: props.clone()
                                        }
                                    },
                                    if !synset.vehicle.is_empty() {
                                         sense_rels {
                                            name: "Vehicle",
                                            rels: synset.vehicle.clone(),
                                            props: props.clone()
                                        }
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
