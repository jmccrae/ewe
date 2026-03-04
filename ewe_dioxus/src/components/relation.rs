use dioxus::prelude::*;
use crate::components::Synset;
use oewn_lib::wordnet::SynsetId;

#[derive(PartialEq, Clone, Props)]
pub struct RelationProps {
    relation_name: &'static str,
    targets: Vec<(SynsetId, Option<String>, Option<String>)>,
    display_ids: bool,
    display_sensekeys: bool,
    display_subcats: bool,
    display_topics: bool,
    display_pronunciations: bool,
}
    


#[component]
pub fn Relation(props : RelationProps) -> Element {
    let mut show_relation = use_signal(|| false);

    rsx! {
        if props.targets.len() > 0 {
            div {
                "class": "relation-title",
                a {
                    onclick: move |_| show_relation.toggle(),
                    "{props.relation_name} ({props.targets.len()})",
                },
            },
            div {
                span {
                    if show_relation() {
                        for p in props.targets.iter() {
                            if let Some(sl) = &p.1 {
                                if let Some(tl) = &p.2 {
                                    span {
                                        "{sl} → {tl}: "
                                    }
                                }
                            },
                            Synset {
                                synset_id: p.0.clone(),
                                display_ids: props.display_ids,
                                display_sensekeys: props.display_sensekeys,
                                display_subcats: props.display_subcats,
                                display_topics: props.display_topics,
                                display_pronunciations: props.display_pronunciations,
                                focus: String::new()
                            }
                        }
                    } 
                }
            }
        } 
    }
}
