use crate::backend::api::get_synset;
use crate::backend::senses::get_sense_count;
use crate::components::editable_relations::{sense_rel_values, synset_rel_values};
use crate::components::relation_types::{SENSE_RELATION_TYPES, SYNSET_RELATION_TYPES};
use crate::components::{
    confidence_draft, parse_confidence_draft, ConfidenceBadge, ConfidenceInput,
    DeleteSynsetButton, EditToggle, EditableDefinition, EditableExamples, EditableIli,
    EditableLemmas, EditableRelations, EditableWikidata, ExampleDraft, PendingRelation, Relation,
    RelationKey, Subcat,
};
use crate::Route;
use dioxus::prelude::*;
use ewe_lib::automaton::{Action, SenseRef, SynsetRef};
use ewe_lib::wordnet::{Example, MemberSynset, ScoredVec, SenseRelation, SynsetId};
use std::collections::HashMap;

/// Draft confidence scores (see `components::confidence`) for everything in the synset editor
/// except examples, whose drafts carry their own (`ExampleDraft::confidence`).
#[derive(Clone, PartialEq, Debug, Default)]
pub struct ConfidenceDrafts {
    pub synset: String,
    pub definition: String,
    /// (member lemma, draft) for every member present when editing started.
    pub senses: Vec<(String, String)>,
    /// Only the existing relations whose score the user has touched.
    pub relations: Vec<(RelationKey, String)>,
}

impl ConfidenceDrafts {
    fn from_synset(synset: &MemberSynset) -> ConfidenceDrafts {
        ConfidenceDrafts {
            synset: confidence_draft(synset.confidence),
            definition: confidence_draft(first_definition_confidence(synset)),
            senses: synset
                .members
                .iter()
                .map(|m| (m.lemma.clone(), confidence_draft(m.sense.confidence)))
                .collect(),
            relations: Vec::new(),
        }
    }
}

fn first_definition_confidence(synset: &MemberSynset) -> Option<f64> {
    synset.definition.first().and_then(|d| synset.definition.confidence(d))
}

/// A `SetConfidence` action - every selector `None` except those given.
fn set_confidence(
    synset: &SynsetId,
    confidence: Option<f64>,
    sense: Option<SenseRef>,
    definition: Option<usize>,
    example: Option<usize>,
    relation: Option<(&str, &SynsetId, Option<SenseRef>)>,
) -> Action {
    let (relation, target, target_sense) = match relation {
        Some((rel, target, target_sense)) => (Some(rel.to_string()), Some(SynsetRef::Id(target.clone())), target_sense),
        None => (None, None, None),
    };
    Action::SetConfidence {
        synset: SynsetRef::Id(synset.clone()),
        confidence,
        sense,
        entry: None,
        definition,
        example,
        relation,
        target,
        target_sense,
    }
}

/// The `SetConfidence` action for one existing relation row, addressed the same way
/// `build_actions` addresses an `AddRelation` for it (a reverse-computed row, e.g. `hyponym`, is
/// stored as the inverse relation on the other synset), and the score it had when editing
/// started. `None` if the row's key isn't a known relation type.
fn relation_confidence_action(
    synset: &MemberSynset,
    key: &RelationKey,
    confidence: Option<f64>,
) -> Option<(Action, Option<f64>)> {
    let lemma = |l: &Option<String>| l.clone().map(SenseRef::Lemma);
    match &key.source_lemma {
        None => {
            let info = SYNSET_RELATION_TYPES.iter().find(|i| i.key == key.key)?;
            let saved = synset_rel_values(synset, key.key).confidence(key.target.as_str());
            let action = if info.swapped {
                set_confidence(&key.target, confidence, None, None, None,
                    Some((info.store_as, &synset.id, None)))
            } else {
                set_confidence(&synset.id, confidence, None, None, None,
                    Some((info.store_as, &key.target, None)))
            };
            Some((action, saved))
        }
        Some(_) => {
            let info = SENSE_RELATION_TYPES.iter().find(|i| i.key == key.key)?;
            let saved = sense_rel_values(synset, key.key)
                .iter()
                .find(|r| {
                    r.target_synset == key.target
                        && Some(&r.source_lemma) == key.source_lemma.as_ref()
                        && r.target_lemma == key.target_lemma
                })
                .and_then(|r| r.confidence);
            let action = if info.swapped {
                set_confidence(&key.target, confidence, lemma(&key.target_lemma), None, None,
                    Some((info.store_as, &synset.id, lemma(&key.source_lemma))))
            } else {
                set_confidence(&synset.id, confidence, lemma(&key.source_lemma), None, None,
                    Some((info.store_as, &key.target, lemma(&key.target_lemma))))
            };
            Some((action, saved))
        }
    }
}

/// Diffs `drafts` (and `draft_definition`/`lemma_drafts`) against the synset's last-saved
/// state and returns the `automaton` actions needed to bring it up to date - empty if nothing
/// changed.
///
/// Members go first (`ChangeMembers` handles its own add/delete of entries internally), then
/// the definition, then examples: updates first (they only replace content in place), then
/// deletes in descending original-number order (so an earlier delete never shifts the position
/// a later one, or an update above it, expects), then adds last (which always append, so
/// ordering doesn't matter for them). A new example's confidence goes inline on its
/// `AddExample`; an existing example's is set alongside its update (by original number, before
/// any delete shifts it); every other confidence is set last, once whatever it scores exists.
/// A malformed confidence draft is an error rather than being silently dropped.
fn build_actions(
    original: &MemberSynset,
    confidence: &ConfidenceDrafts,
    synset_id: &SynsetId,
    original_members: &[String],
    lemma_drafts: &[String],
    original_definition: &str,
    draft_definition: &str,
    original_examples: &[Example],
    drafts: &[ExampleDraft],
    relation_deletes: &[RelationKey],
    relation_adds: &[PendingRelation],
    original_ili: &str,
    draft_ili: &str,
    original_wikidata: &[String],
    draft_wikidata: &[String],
) -> Result<Vec<Action>, String> {
    let mut actions = Vec::new();

    let members: Vec<String> = lemma_drafts
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if members != original_members {
        actions.push(Action::ChangeMembers {
            synset: SynsetRef::Id(synset_id.clone()),
            members: members.clone(),
        });
    }

    if draft_definition != original_definition {
        actions.push(Action::Definition {
            confidence: None,
            synset: SynsetRef::Id(synset_id.clone()),
            definition: draft_definition.to_string(),
        });
    }

    if draft_ili != original_ili {
        actions.push(Action::ChangeILI {
            synset: SynsetRef::Id(synset_id.clone()),
            ili: draft_ili.to_string(),
        });
    }

    let wikidata: Vec<String> = draft_wikidata
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if wikidata != original_wikidata {
        actions.push(Action::ChangeWikidata {
            synset: SynsetRef::Id(synset_id.clone()),
            wikidata,
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
                confidence: None,
                synset: SynsetRef::Id(synset_id.clone()),
                number,
                example: draft.text.clone(),
                source,
            });
        }
        let c = parse_confidence_draft(&draft.confidence)?;
        if c != original.confidence {
            actions.push(set_confidence(synset_id, c, None, None, Some(number), None));
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
                confidence: parse_confidence_draft(&draft.confidence)?,
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
            confidence: None,
            source: SynsetRef::Id(source),
            source_sense: None,
            relation: add.info.store_as.to_string(),
            target: SynsetRef::Id(target),
            target_sense: None,
            source_lemma,
            target_lemma,
        });
    }

    let c = parse_confidence_draft(&confidence.synset)?;
    if c != original.confidence {
        actions.push(set_confidence(synset_id, c, None, None, None, None));
    }
    let c = parse_confidence_draft(&confidence.definition)?;
    if c != first_definition_confidence(original) && !draft_definition.is_empty() {
        actions.push(set_confidence(synset_id, c, None, Some(1), None, None));
    }
    for (lemma, draft) in &confidence.senses {
        // A member removed in this same edit has nothing left to score.
        if !members.contains(lemma) {
            continue;
        }
        let saved = original.members.iter().find(|m| m.lemma == *lemma).and_then(|m| m.sense.confidence);
        let c = parse_confidence_draft(draft)?;
        if c != saved {
            actions.push(set_confidence(synset_id, c, Some(SenseRef::Lemma(lemma.clone())), None, None, None));
        }
    }
    for (key, draft) in &confidence.relations {
        if relation_deletes.contains(key) {
            continue;
        }
        let c = parse_confidence_draft(draft)?;
        if let Some((action, saved)) = relation_confidence_action(original, key, c) {
            if c != saved {
                actions.push(action);
            }
        }
    }

    Ok(actions)
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
fn synset_rels(name: &'static str, rels: ScoredVec<SynsetId>, props: SynsetProps) -> Element {
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

fn map_ss_rels(rels: ScoredVec<SynsetId>) -> Vec<(SynsetId, Option<String>, Option<String>, Option<f64>)> {
    rels.iter_scored()
        .map(|(ss_id, confidence)| (ss_id.clone(), None, None, confidence))
        .collect()
}

fn map_se_rels(rels: Vec<SenseRelation>) -> Vec<(SynsetId, Option<String>, Option<String>, Option<f64>)> {
    rels.into_iter()
        .map(|se_rel| {
            (
                se_rel.target_synset,
                Some(se_rel.source_lemma),
                se_rel.target_lemma,
                se_rel.confidence,
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
    let mut ili_draft = use_signal(String::new);
    let mut wikidata_drafts = use_signal(Vec::<String>::new);
    let mut relation_deletes = use_signal(Vec::<RelationKey>::new);
    let mut relation_adds = use_signal(Vec::<PendingRelation>::new);
    let mut confidence_drafts = use_signal(ConfidenceDrafts::default);
    // Only mutated (`.set()`) inside the `edit` feature's accept handler below; harmless when
    // it isn't.
    #[allow(unused_mut)]
    let mut saving = use_signal(|| false);
    let mut edit_error = use_signal(|| None::<String>);
    // Only mutated inside the `edit` feature's accept handler below; harmless when it isn't.
    #[allow(unused_mut, unused_variables)]
    let mut dirty = use_context::<Signal<bool>>();

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
                        class: if editing() { "synset editing" } else { "synset" },
                        if props.display_ids || editing() {
                            div {
                                class: "synset-id",
                                // show: display.ids
                                if props.display_ids {
                                    span {
                                        class: "identifier",
                                        "{synset.id}"
                                    }
                                }
                                if !editing() {
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
                                            a {
                                                class: "identifier",
                                                href: "https://globalwordnet.org/cili/{ili}",
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
                                                class: "synset-id-title",
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
                                }
                                hr {}
                            }
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
                                    ConfidenceInput {
                                        value: confidence_drafts().synset,
                                        on_input: move |v| confidence_drafts.write().synset = v,
                                    }
                                } else {
                                    ConfidenceBadge { value: synset.confidence }
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
                                            ConfidenceBadge { value: member.sense.confidence.or(member.entry_confidence) },
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
                                // Editing the ILI/Wikidata identifiers doesn't depend on the
                                // "Show Synset Identifier" display option above - only the
                                // caller mounting this while the synset-wide edit toggle is on
                                // does. Sits between the lemma list and Definition/Examples/
                                // Relations, and (unlike `.side-icons`, a sibling of `.lemmas`
                                // below) only exists at all while editing, since there's nothing
                                // to show otherwise.
                                if editing() {
                                    div {
                                        class: "synset-identifiers-editing",
                                        EditableIli {
                                            value: ili_draft(),
                                            on_input: move |v| ili_draft.set(v),
                                        }
                                        EditableWikidata {
                                            drafts: wikidata_drafts(),
                                            on_drafts_changed: move |drafts| wikidata_drafts.set(drafts),
                                        }
                                    }
                                    div {
                                        class: "field-row sense-confidence-editing",
                                        b { class: "field-label", "Sense confidence: " }
                                        for (index, (lemma, value)) in confidence_drafts().senses.into_iter().enumerate() {
                                            span {
                                                key: "{lemma}",
                                                class: "sense-confidence-editing-item",
                                                "{lemma}"
                                                ConfidenceInput {
                                                    value,
                                                    on_input: move |v| {
                                                        if let Some(entry) = confidence_drafts.write().senses.get_mut(index) {
                                                            entry.1 = v;
                                                        }
                                                    },
                                                }
                                            }
                                        }
                                    }
                                }
                                if editing() {
                                    div {
                                        class: "field-row",
                                        b { class: "field-label", "Definition: " }
                                        EditableDefinition {
                                            editing: true,
                                            value: definition_draft(),
                                            on_input: move |v| definition_draft.set(v),
                                        }
                                        ConfidenceInput {
                                            value: confidence_drafts().definition,
                                            on_input: move |v| confidence_drafts.write().definition = v,
                                        }
                                    }
                                } else {
                                    EditableDefinition {
                                        editing: false,
                                        value: synset.definition.get(0).cloned().unwrap_or_default(),
                                        on_input: move |v| definition_draft.set(v),
                                    }
                                    ConfidenceBadge { value: first_definition_confidence(synset) }
                                }
                                if editing() {
                                    div {
                                        class: "field-row",
                                        b { class: "field-label", "Examples: " }
                                        EditableExamples {
                                            editing: true,
                                            examples: synset.example.clone(),
                                            drafts: example_drafts(),
                                            on_drafts_changed: move |drafts| example_drafts.set(drafts),
                                        }
                                    }
                                } else {
                                    EditableExamples {
                                        editing: false,
                                        examples: synset.example.clone(),
                                        drafts: example_drafts(),
                                        on_drafts_changed: move |drafts| example_drafts.set(drafts),
                                    }
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
                                        confidence_drafts: confidence_drafts().relations,
                                        on_confidence_drafts_changed: move |v| confidence_drafts.write().relations = v,
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
                                                rels: synset.antonym.to_vec(),
                                                props: props.clone()
                                            }
                                        },
                                        if !synset.participle.is_empty() {
                                             sense_rels {
                                                name: "Participles",
                                                rels: synset.participle.to_vec(),
                                                props: props.clone()
                                            }
                                        },
                                        if !synset.pertainym.is_empty() {
                                             sense_rels {
                                                name: "Of or Pertaining To",
                                                rels: synset.pertainym.to_vec(),
                                                props: props.clone()
                                            }
                                        },
                                        if !synset.derivation.is_empty() {
                                             sense_rels {
                                                name: "Derived From",
                                                rels: synset.derivation.to_vec(),
                                                props: props.clone()
                                            }
                                        },
                                        if !synset.exemplifies_sense.is_empty() {
                                             sense_rels {
                                                name: "Is an Example Of",
                                                rels: synset.exemplifies_sense.to_vec(),
                                                props: props.clone()
                                            }
                                        },
                                        if !synset.is_exemplified_by_sense.is_empty() {
                                             sense_rels {
                                                name: "Has Example",
                                                rels: synset.is_exemplified_by_sense.to_vec(),
                                                props: props.clone()
                                            }
                                        },
                                        if !synset.agent.is_empty() {
                                             sense_rels {
                                                name: "Agent",
                                                rels: synset.agent.to_vec(),
                                                props: props.clone()
                                            }
                                        },
                                        if !synset.material.is_empty() {
                                             sense_rels {
                                                name: "Material",
                                                rels: synset.material.to_vec(),
                                                props: props.clone()
                                            }
                                        },
                                        if !synset.event.is_empty() {
                                             sense_rels {
                                                name: "Event",
                                                rels: synset.event.to_vec(),
                                                props: props.clone()
                                            }
                                        },
                                        if !synset.instrument.is_empty() {
                                             sense_rels {
                                                name: "Instrument",
                                                rels: synset.instrument.to_vec(),
                                                props: props.clone()
                                            }
                                        },
                                        if !synset.location.is_empty() {
                                             sense_rels {
                                                name: "Location",
                                                rels: synset.location.to_vec(),
                                                props: props.clone()
                                            }
                                        },
                                        if !synset.by_means_of.is_empty() {
                                             sense_rels {
                                                name: "By Means Of",
                                                rels: synset.by_means_of.to_vec(),
                                                props: props.clone()
                                            }
                                        },
                                        if !synset.undergoer.is_empty() {
                                             sense_rels {
                                                name: "Undergoer",
                                                rels: synset.undergoer.to_vec(),
                                                props: props.clone()
                                            }
                                        },
                                        if !synset.property.is_empty() {
                                             sense_rels {
                                                name: "Property",
                                                rels: synset.property.to_vec(),
                                                props: props.clone()
                                            }
                                        },
                                        if !synset.result.is_empty() {
                                             sense_rels {
                                                name: "Result",
                                                rels: synset.result.to_vec(),
                                                props: props.clone()
                                            }
                                        },
                                        if !synset.state.is_empty() {
                                             sense_rels {
                                                name: "State",
                                                rels: synset.state.to_vec(),
                                                props: props.clone()
                                            }
                                        },
                                        if !synset.uses.is_empty() {
                                             sense_rels {
                                                name: "Uses",
                                                rels: synset.uses.to_vec(),
                                                props: props.clone()
                                            }
                                        },
                                        if !synset.destination.is_empty() {
                                             sense_rels {
                                                name: "Destination",
                                                rels: synset.destination.to_vec(),
                                                props: props.clone()
                                            }
                                        },
                                        if !synset.body_part.is_empty() {
                                             sense_rels {
                                                name: "Body Part",
                                                rels: synset.body_part.to_vec(),
                                                props: props.clone()
                                            }
                                        },
                                        if !synset.vehicle.is_empty() {
                                             sense_rels {
                                                name: "Vehicle",
                                                rels: synset.vehicle.to_vec(),
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
                                        let ili = synset.ili.as_ref().map(|i| i.to_string()).unwrap_or_default();
                                        let wikidata = synset.wikidata.clone();
                                        let confidence = ConfidenceDrafts::from_synset(synset);
                                        move |_| {
                                            confidence_drafts.set(confidence.clone());
                                            lemma_drafts.set(members.clone());
                                            definition_draft.set(definition.clone());
                                            example_drafts.set(ExampleDraft::from_examples(&examples));
                                            ili_draft.set(ili.clone());
                                            wikidata_drafts.set(wikidata.clone());
                                            relation_deletes.set(Vec::new());
                                            relation_adds.set(Vec::new());
                                            edit_error.set(None);
                                            editing.set(true);
                                        }
                                    },
                                    on_accept: {
                                        let original = synset.clone();
                                        let synset_id = synset.id.clone();
                                        let original_members: Vec<String> = synset.members.iter().map(|m| m.lemma.clone()).collect();
                                        let original_definition = synset.definition.get(0).cloned().unwrap_or_default();
                                        let original_examples = synset.example.clone();
                                        let original_ili = synset.ili.as_ref().map(|i| i.to_string()).unwrap_or_default();
                                        let original_wikidata = synset.wikidata.clone();
                                        move |_| {
                                            let actions = build_actions(
                                                &original,
                                                &confidence_drafts(),
                                                &synset_id,
                                                &original_members,
                                                &lemma_drafts(),
                                                &original_definition,
                                                &definition_draft(),
                                                &original_examples,
                                                &example_drafts(),
                                                &relation_deletes(),
                                                &relation_adds(),
                                                &original_ili,
                                                &ili_draft(),
                                                &original_wikidata,
                                                &wikidata_drafts(),
                                            );
                                            let actions = match actions {
                                                Ok(actions) => actions,
                                                Err(e) => {
                                                    edit_error.set(Some(e));
                                                    return;
                                                }
                                            };
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
                                                            dirty.set(true);
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
                                    if editing() {
                                        DeleteSynsetButton { synset_id: synset.id.clone() }
                                    }
                                }
                                if let Some(err) = edit_error() {
                                    span { class: "edit-error", "{err}" }
                                }
                                if !editing() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::ExampleDraft;
    use ewe_lib::automaton::apply_automaton;
    use ewe_lib::change_manager::{self, ChangeList};
    use ewe_lib::wordnet::{Lexicon, LexiconHashMapBackend, PosKey};

    /// dog -hypernym-> animal, dog's sense -antonym-> cat's sense, and one example on dog.
    fn fixture() -> (LexiconHashMapBackend, SynsetId, SynsetId, SynsetId) {
        let mut wn = LexiconHashMapBackend::new();
        let mut changes = ChangeList::new();
        // (`add_lexfile` is cfg(test)-only inside ewe_lib; `add_synset` registers the lexfile.)
        let mut ids = Vec::new();
        for (def, lemma) in [("a dog", "dog"), ("an animal", "animal"), ("a cat", "cat")] {
            let id = change_manager::add_synset(&mut wn, def.to_string(), "noun.animal".to_string(),
                PosKey::new("n".to_string()), None, &mut changes).unwrap();
            change_manager::add_entry(&mut wn, id.clone(), lemma.to_string(),
                PosKey::new("n".to_string()), Vec::new(), None, &mut changes).unwrap();
            ids.push(id);
        }
        let (dog, animal, cat) = (ids[0].clone(), ids[1].clone(), ids[2].clone());
        let actions = vec![
            Action::AddRelation {
                source: SynsetRef::Id(dog.clone()), source_sense: None,
                relation: "hypernym".to_string(), target: SynsetRef::Id(animal.clone()),
                target_sense: None, source_lemma: None, target_lemma: None, confidence: None,
            },
            Action::AddRelation {
                source: SynsetRef::Id(dog.clone()), source_sense: None,
                relation: "antonym".to_string(), target: SynsetRef::Id(cat.clone()),
                target_sense: None, source_lemma: Some("dog".to_string()),
                target_lemma: Some("cat".to_string()), confidence: None,
            },
            Action::AddExample { synset: SynsetRef::Id(dog.clone()), example: "woof".to_string(), source: None,
                confidence: None },
        ];
        apply_automaton(actions, &mut wn, &mut changes).unwrap();
        (wn, dog, animal, cat)
    }

    fn member_synset(wn: &LexiconHashMapBackend, id: &SynsetId) -> MemberSynset {
        let synset = wn.synset_by_id(id).unwrap().unwrap().into_owned();
        MemberSynset::from_synset(id, synset, wn).unwrap()
    }

    /// `build_actions` with every non-confidence draft left exactly as saved.
    fn confidence_only_actions(
        synset: &MemberSynset,
        confidence: &ConfidenceDrafts,
        examples: &[ExampleDraft],
    ) -> Result<Vec<Action>, String> {
        let members: Vec<String> = synset.members.iter().map(|m| m.lemma.clone()).collect();
        let definition = synset.definition.first().cloned().unwrap_or_default();
        build_actions(synset, confidence, &synset.id, &members, &members, &definition, &definition,
            &synset.example, examples, &[], &[], "", "", &synset.wikidata, &synset.wikidata)
    }

    #[test]
    fn test_build_actions_sets_every_kind_of_confidence() {
        let (mut wn, dog, animal, cat) = fixture();
        let dog_ss = member_synset(&wn, &dog);
        let mut confidence = ConfidenceDrafts::from_synset(&dog_ss);
        confidence.synset = "0.5".to_string();
        confidence.definition = "0.7".to_string();
        confidence.senses[0].1 = "0.8".to_string();
        confidence.relations.push((
            RelationKey { key: "antonym", target: cat.clone(),
                source_lemma: Some("dog".to_string()), target_lemma: Some("cat".to_string()) },
            "0.6".to_string(),
        ));
        let mut examples = ExampleDraft::from_examples(&dog_ss.example);
        examples[0].confidence = "0.3".to_string();
        examples.push(ExampleDraft { original_number: None, text: "arf".to_string(),
            source: String::new(), confidence: "0.2".to_string(), deleted: false });
        let actions = confidence_only_actions(&dog_ss, &confidence, &examples).unwrap();
        apply_automaton(actions, &mut wn, &mut ChangeList::new()).unwrap();

        let ss = wn.synset_by_id(&dog).unwrap().unwrap();
        assert_eq!(ss.confidence, Some(0.5));
        assert_eq!(ss.definition.confidence("a dog"), Some(0.7));
        assert_eq!(ss.example[0].confidence, Some(0.3));
        assert_eq!(ss.example[1].text, "arf");
        assert_eq!(ss.example[1].confidence, Some(0.2));
        let reloaded = member_synset(&wn, &dog);
        assert_eq!(reloaded.members[0].sense.confidence, Some(0.8));
        assert_eq!(reloaded.antonym[0].confidence, Some(0.6));
        // Untouched: nothing else got a score.
        assert_eq!(ss.hypernym.confidence(animal.as_str()), None);

        // Re-running with the now-saved values as drafts is a no-op.
        let confidence = ConfidenceDrafts::from_synset(&reloaded);
        let examples = ExampleDraft::from_examples(&reloaded.example);
        assert!(confidence_only_actions(&reloaded, &confidence, &examples).unwrap().is_empty());
    }

    #[test]
    fn test_build_actions_scores_inverse_relation_on_the_stored_side() {
        let (mut wn, dog, animal, _) = fixture();
        // Editing `animal`, whose `hyponym` row is really `dog`'s stored `hypernym`.
        let animal_ss = member_synset(&wn, &animal);
        let mut confidence = ConfidenceDrafts::from_synset(&animal_ss);
        confidence.relations.push((
            RelationKey { key: "hyponym", target: dog.clone(), source_lemma: None, target_lemma: None },
            "0.4".to_string(),
        ));
        let actions = confidence_only_actions(&animal_ss, &confidence, &[]).unwrap();
        apply_automaton(actions, &mut wn, &mut ChangeList::new()).unwrap();

        let dog_ss = wn.synset_by_id(&dog).unwrap().unwrap();
        assert_eq!(dog_ss.hypernym.confidence(animal.as_str()), Some(0.4));
        assert_eq!(member_synset(&wn, &animal).hyponym.confidence(dog.as_str()), Some(0.4));
    }

    #[test]
    fn test_build_actions_numbers_new_example_after_deletes() {
        let (mut wn, dog, _, _) = fixture();
        let dog_ss = member_synset(&wn, &dog);
        let confidence = ConfidenceDrafts::from_synset(&dog_ss);
        let mut examples = ExampleDraft::from_examples(&dog_ss.example);
        examples[0].deleted = true;
        examples.push(ExampleDraft { original_number: None, text: "arf".to_string(),
            source: String::new(), confidence: "0.2".to_string(), deleted: false });
        let actions = confidence_only_actions(&dog_ss, &confidence, &examples).unwrap();
        apply_automaton(actions, &mut wn, &mut ChangeList::new()).unwrap();

        let ss = wn.synset_by_id(&dog).unwrap().unwrap();
        assert_eq!(ss.example.len(), 1);
        assert_eq!(ss.example[0].text, "arf");
        assert_eq!(ss.example[0].confidence, Some(0.2));
    }

    #[test]
    fn test_build_actions_rejects_malformed_confidence() {
        let (wn, dog, _, _) = fixture();
        let dog_ss = member_synset(&wn, &dog);
        for bad in ["abc", "1.5", "-0.1"] {
            let mut confidence = ConfidenceDrafts::from_synset(&dog_ss);
            confidence.synset = bad.to_string();
            assert!(confidence_only_actions(&dog_ss, &confidence, &[]).is_err(), "{bad}");
        }
    }
}
