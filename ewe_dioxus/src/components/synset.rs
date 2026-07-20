use crate::backend::api::get_synset;
use crate::backend::senses::get_sense_count;
use crate::components::{
    EditToggle, EditableDefinition, EditableExamples, EditableLemmas, EditableRelations,
    ExampleDraft, PendingRelation, Relation, RelationKey, Subcat,
};
use crate::Route;
use dioxus::prelude::*;
use oewn_lib::automaton::{Action, SynsetRef};
use oewn_lib::wordnet::{Example, MemberSynset, SenseRelation, SynsetId};
use std::collections::HashMap;

/// Diffs `drafts` (and `draft_definition`/`lemma_drafts`) against the synset's last-saved
/// state and returns the `automaton` actions needed to bring it up to date - empty if nothing
/// changed.
///
/// Members go first (`ChangeMembers` handles its own add/delete of entries internally), then
/// the definition, then examples: updates first (they only replace content in place), then
/// deletes in descending original-number order (so an earlier delete never shifts the position
/// a later one, or an update above it, expects), then adds last (which always append, so
/// ordering doesn't matter for them).
fn build_actions(
    synset_id: &SynsetId,
    original_members: &[String],
    lemma_drafts: &[String],
    original_definition: &str,
    draft_definition: &str,
    original_examples: &[Example],
    drafts: &[ExampleDraft],
    relation_deletes: &[RelationKey],
    relation_adds: &[PendingRelation],
) -> Vec<Action> {
    let mut actions = Vec::new();

    let members: Vec<String> = lemma_drafts
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if members != original_members {
        actions.push(Action::ChangeMembers {
            synset: SynsetRef::Id(synset_id.clone()),
            members,
        });
    }

    if draft_definition != original_definition {
        actions.push(Action::Definition {
            synset: SynsetRef::Id(synset_id.clone()),
            definition: draft_definition.to_string(),
        });
    }

    for draft in drafts {
        if draft.deleted {
            continue;
        }
        let Some(number) = draft.original_number else {
            continue;
        };
        let Some(original) = original_examples.get(number - 1) else {
            continue;
        };
        let source = normalize_source(&draft.source);
        if draft.text != original.text || source != original.source {
            actions.push(Action::UpdateExample {
                synset: SynsetRef::Id(synset_id.clone()),
                number,
                example: draft.text.clone(),
                source,
            });
        }
    }

    let mut delete_numbers: Vec<usize> = drafts
        .iter()
        .filter(|d| d.deleted)
        .filter_map(|d| d.original_number)
        .collect();
    delete_numbers.sort_unstable_by(|a, b| b.cmp(a));
    for number in delete_numbers {
        actions.push(Action::DeleteExample {
            synset: SynsetRef::Id(synset_id.clone()),
            number,
        });
    }

    for draft in drafts {
        if draft.original_number.is_none() && !draft.deleted && !draft.text.trim().is_empty() {
            actions.push(Action::AddExample {
                synset: SynsetRef::Id(synset_id.clone()),
                example: draft.text.clone(),
                source: normalize_source(&draft.source),
            });
        }
    }

    // `DeleteRelation` clears links between the pair in both directions regardless of type
    // (see change_manager::delete_rel/delete_sense_rel), so deletes never need the
    // forward/inverse swap that adds do.
    for delete in relation_deletes {
        actions.push(Action::DeleteRelation {
            source: SynsetRef::Id(synset_id.clone()),
            source_sense: None,
            target: SynsetRef::Id(delete.target.clone()),
            target_sense: None,
            source_lemma: delete.source_lemma.clone(),
            target_lemma: delete.target_lemma.clone(),
        });
    }

    for add in relation_adds {
        // About half of the relation types shown are computed by reverse lookup rather than
        // stored directly (e.g. `hyponym` is derived from the target's `hypernym`) - adding
        // one of those means inserting the inverse relation with source and target swapped.
        // See `components::relation_types` for why.
        let (source, source_lemma, target, target_lemma) = if add.info.swapped {
            (
                add.target.clone(),
                add.target_lemma.clone(),
                synset_id.clone(),
                add.source_lemma.clone(),
            )
        } else {
            (
                synset_id.clone(),
                add.source_lemma.clone(),
                add.target.clone(),
                add.target_lemma.clone(),
            )
        };
        actions.push(Action::AddRelation {
            source: SynsetRef::Id(source),
            source_sense: None,
            relation: add.info.store_as.to_string(),
            target: SynsetRef::Id(target),
            target_sense: None,
            source_lemma,
            target_lemma,
        });
    }

    actions
}

/// An empty source is not a valid value - treat it the same as no source at all.
fn normalize_source(source: &str) -> Option<String> {
    if source.is_empty() {
        None
    } else {
        Some(source.to_string())
    }
}

static CSS: Asset = asset!("/assets/styling/synset.css");
static WIKIDATA_ICON: Asset = asset!("/assets/wikidata.png");

#[derive(PartialEq, Clone, Props)]
pub struct SynsetProps {
    synset_id: ReadSignal<SynsetId>,
    display_ids: bool,
    display_sensekeys: bool,
    display_subcats: bool,
    display_topics: bool,
    display_pronunciations: bool,
    focus: String,
}

fn subcats(synset: &MemberSynset) -> HashMap<String, Vec<String>> {
    let mut subcat_map = HashMap::new();
    for member in &synset.members {
        for subcat in &member.sense.subcat {
            subcat_map
                .entry(subcat.clone())
                .or_insert(Vec::new())
                .push(member.lemma.clone());
        }
    }
    subcat_map
}

#[component]
fn synset_rels(name: &'static str, rels: Vec<SynsetId>, props: SynsetProps) -> Element {
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
fn sense_rels(name: &'static str, rels: Vec<SenseRelation>, props: SynsetProps) -> Element {
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

fn map_ss_rels(rels: Vec<SynsetId>) -> Vec<(SynsetId, Option<String>, Option<String>)> {
    rels.into_iter().map(|ss_id| (ss_id, None, None)).collect()
}

fn map_se_rels(rels: Vec<SenseRelation>) -> Vec<(SynsetId, Option<String>, Option<String>)> {
    rels.into_iter()
        .map(|se_rel| {
            (
                se_rel.target_synset,
                Some(se_rel.source_lemma),
                Some(se_rel.target_lemma),
            )
        })
        .collect()
}

#[component]
pub fn Synset(props: SynsetProps) -> Element {
    let synset = use_loader(move || {
        let synset_id = props.synset_id.cloned();
        async move { get_synset(synset_id).await }
    });

    // A non-zero count means the sense has corpus annotations worth linking to;
    // fetched separately (rather than baked into `get_synset`) so it stays a cheap,
    // index-backed lookup that doesn't slow down loading the synset itself.
    let sense_count = use_loader(move || {
        let synset_id = props.synset_id.cloned();
        async move { get_sense_count(synset_id).await }
    });

    let mut show_relations = use_signal(|| false);

    // Synset-wide edit toggle (the pencil next to the Wikidata icon, becomes an accept/reject
    // pair while on). Currently gates `EditableDefinition` and `EditableExamples`, but is
    // shared so lemmas/relations editors can hook into the same batch once they exist. Every
    // field's draft is committed (or discarded) together, in one call to
    // `backend::edit::apply_edits`, rather than each field saving itself independently.
    let mut editing = use_signal(|| false);
    let mut lemma_drafts = use_signal(Vec::<String>::new);
    let mut definition_draft = use_signal(String::new);
    let mut example_drafts = use_signal(Vec::<ExampleDraft>::new);
    let mut relation_deletes = use_signal(Vec::<RelationKey>::new);
    let mut relation_adds = use_signal(Vec::<PendingRelation>::new);
    // Only mutated (`.set()`) inside the `edit` feature's accept handler below; harmless when
    // it isn't.
    #[allow(unused_mut)]
    let mut saving = use_signal(|| false);
    let mut edit_error = use_signal(|| None::<String>);

    // `ss_load` only needs `.write()` (hence `mut`) when the `edit` feature's accept handler
    // below is reachable; harmless when it isn't.
    #[allow(unused_mut)]
    if let Ok(mut ss_load) = synset {
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
                                        " ("
                                    }
                                }
                                if let Some(ref ili) = synset.ili {
                                    span {
                                        b {
                                            class: "synset-id-title",
                                            "Interlingual Index: "
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
                                hr {}
                            },
                        },
                        div {
                            class: "lemmas-container",
                            div {
                                class: "lemmas",
                                span {
                                    class: "pos",
                                    "({synset.part_of_speech})"
                                },
                                if editing() {
                                    EditableLemmas {
                                        drafts: lemma_drafts(),
                                        on_drafts_changed: move |drafts| lemma_drafts.set(drafts),
                                    }
                                } else {
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
                                                    " (Pronunciation:",
                                                    for (i, pron) in member.pronunciation.iter().enumerate() {
                                                        if let Some(variety) = &pron.variety {
                                                            span {
                                                                class: "pronunciation_variety",
                                                                    " ({variety})"
                                                            }
                                                        }
                                                        " {pron.value}",
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
                                    }
                                },
                                EditableDefinition {
                                    editing: editing(),
                                    value: if editing() {
                                        definition_draft()
                                    } else {
                                        synset.definition.get(0).cloned().unwrap_or_default()
                                    },
                                    on_input: move |v| definition_draft.set(v),
                                }
                                EditableExamples {
                                    editing: editing(),
                                    examples: synset.example.clone(),
                                    drafts: example_drafts(),
                                    on_drafts_changed: move |drafts| example_drafts.set(drafts),
                                }
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
                                if editing() {
                                    EditableRelations {
                                        synset: synset.clone(),
                                        pending_deletes: relation_deletes(),
                                        pending_adds: relation_adds(),
                                        on_pending_deletes_changed: move |v| relation_deletes.set(v),
                                        on_pending_adds_changed: move |v| relation_adds.set(v),
                                    }
                                } else if show_relations() {
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
                                        },
                                        if let Ok(count_load) = &sense_count {
                                            if !count_load.loading() {
                                                if let Some(count) = Some(*count_load.read()).filter(|c| *c > 0) {
                                                    div {
                                                        class: "relation-title",
                                                        Link {
                                                            to: Route::BySenses { id: synset.id.as_str().to_string(), page: 0 },
                                                            "Occurrences ({count})"
                                                        }
                                                    }
                                                }
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
                            div {
                                class: "side-icons",
                                EditToggle {
                                    editing: editing(),
                                    saving: saving(),
                                    on_enter: {
                                        let members: Vec<String> = synset.members.iter().map(|m| m.lemma.clone()).collect();
                                        let definition = synset.definition.get(0).cloned().unwrap_or_default();
                                        let examples = synset.example.clone();
                                        move |_| {
                                            lemma_drafts.set(members.clone());
                                            definition_draft.set(definition.clone());
                                            example_drafts.set(ExampleDraft::from_examples(&examples));
                                            relation_deletes.set(Vec::new());
                                            relation_adds.set(Vec::new());
                                            edit_error.set(None);
                                            editing.set(true);
                                        }
                                    },
                                    on_accept: {
                                        let synset_id = synset.id.clone();
                                        let original_members: Vec<String> = synset.members.iter().map(|m| m.lemma.clone()).collect();
                                        let original_definition = synset.definition.get(0).cloned().unwrap_or_default();
                                        let original_examples = synset.example.clone();
                                        move |_| {
                                            let actions = build_actions(
                                                &synset_id,
                                                &original_members,
                                                &lemma_drafts(),
                                                &original_definition,
                                                &definition_draft(),
                                                &original_examples,
                                                &example_drafts(),
                                                &relation_deletes(),
                                                &relation_adds(),
                                            );
                                            if actions.is_empty() {
                                                editing.set(false);
                                                return;
                                            }
                                            #[cfg(feature = "edit")]
                                            {
                                                let synset_id = synset_id.clone();
                                                spawn(async move {
                                                    saving.set(true);
                                                    edit_error.set(None);
                                                    match crate::backend::edit::apply_edits(synset_id, actions).await {
                                                        Ok(updated) => {
                                                            if let Some(s) = ss_load.write().as_mut() {
                                                                *s = updated;
                                                            }
                                                            editing.set(false);
                                                        }
                                                        Err(e) => edit_error.set(Some(e.to_string())),
                                                    }
                                                    saving.set(false);
                                                });
                                            }
                                        }
                                    },
                                    on_reject: move |_| {
                                        editing.set(false);
                                        edit_error.set(None);
                                    },
                                }
                                if let Some(err) = edit_error() {
                                    span { class: "edit-error", "{err}" }
                                }
                                if let Some(wd) = synset.wikidata.first() {
                                    div {
                                        class: "wikidata",
                                        a {
                                            href: "https://www.wikidata.org/entity/{wd}",
                                            img {
                                                src: WIKIDATA_ICON,
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
