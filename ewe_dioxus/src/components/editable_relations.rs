use dioxus::prelude::*;
use oewn_lib::wordnet::{MemberSynset, SenseRelation, SynsetId};

#[allow(unused_imports)]
use crate::components::relation_types::{RelationTypeInfo, SENSE_RELATION_TYPES, SYNSET_RELATION_TYPES};
#[allow(unused_imports)]
use crate::Route;

/// Identifies one existing relation row queued for deletion. `key` is the display field
/// (`"hypernym"`, `"antonym"`, ...), matching `RelationTypeInfo::key` - kept alongside the
/// target/lemmas since the same pair of synsets can be related more than one way at once.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RelationKey {
    pub key: &'static str,
    pub target: SynsetId,
    pub source_lemma: Option<String>,
    pub target_lemma: Option<String>,
}

/// A new relation queued for addition. `target_display` is a human-readable label (members +
/// definition) captured at pick time, since the target synset's full data isn't otherwise
/// available to this component once picked.
#[derive(Clone, PartialEq, Debug)]
pub struct PendingRelation {
    pub info: RelationTypeInfo,
    pub target: SynsetId,
    pub target_display: String,
    pub source_lemma: Option<String>,
    pub target_lemma: Option<String>,
}

#[derive(Clone, PartialEq, Props)]
pub struct EditableRelationsProps {
    pub synset: MemberSynset,
    pub pending_deletes: Vec<RelationKey>,
    pub pending_adds: Vec<PendingRelation>,
    pub on_pending_deletes_changed: EventHandler<Vec<RelationKey>>,
    pub on_pending_adds_changed: EventHandler<Vec<PendingRelation>>,
}

#[allow(dead_code)]
fn synset_rel_values<'a>(synset: &'a MemberSynset, key: &str) -> &'a [SynsetId] {
    match key {
        "hypernym" => &synset.hypernym,
        "hyponym" => &synset.hyponym,
        "instance_hypernym" => &synset.instance_hypernym,
        "instance_hyponym" => &synset.instance_hyponym,
        "attribute" => &synset.attribute,
        "causes" => &synset.causes,
        "is_caused_by" => &synset.is_caused_by,
        "domain_region" => &synset.domain_region,
        "has_domain_region" => &synset.has_domain_region,
        "domain_topic" => &synset.domain_topic,
        "has_domain_topic" => &synset.has_domain_topic,
        "exemplifies" => &synset.exemplifies,
        "is_exemplified_by" => &synset.is_exemplified_by,
        "entails" => &synset.entails,
        "is_entailed_by" => &synset.is_entailed_by,
        "mero_location" => &synset.mero_location,
        "holo_location" => &synset.holo_location,
        "holo_member" => &synset.holo_member,
        "mero_member" => &synset.mero_member,
        "holo_part" => &synset.holo_part,
        "mero_part" => &synset.mero_part,
        "holo_substance" => &synset.holo_substance,
        "mero_substance" => &synset.mero_substance,
        "meronym" => &synset.meronym,
        "holonym" => &synset.holonym,
        "similar" => &synset.similar,
        "feminine" => &synset.feminine,
        "masculine" => &synset.masculine,
        "also" => &synset.also,
        "other" => &synset.other,
        _ => &[],
    }
}

#[allow(dead_code)]
fn sense_rel_values<'a>(synset: &'a MemberSynset, key: &str) -> &'a [SenseRelation] {
    match key {
        "antonym" => &synset.antonym,
        "participle" => &synset.participle,
        "pertainym" => &synset.pertainym,
        "derivation" => &synset.derivation,
        "exemplifies_sense" => &synset.exemplifies_sense,
        "is_exemplified_by_sense" => &synset.is_exemplified_by_sense,
        "agent" => &synset.agent,
        "material" => &synset.material,
        "event" => &synset.event,
        "instrument" => &synset.instrument,
        "location" => &synset.location,
        "by_means_of" => &synset.by_means_of,
        "undergoer" => &synset.undergoer,
        "property" => &synset.property,
        "result" => &synset.result,
        "state" => &synset.state,
        "uses" => &synset.uses,
        "destination" => &synset.destination,
        "body_part" => &synset.body_part,
        "vehicle" => &synset.vehicle,
        _ => &[],
    }
}

#[cfg(feature = "edit")]
fn find_relation_type(encoded: &str) -> Option<RelationTypeInfo> {
    if let Some(key) = encoded.strip_prefix("syn:") {
        SYNSET_RELATION_TYPES.iter().find(|i| i.key == key).copied()
    } else if let Some(key) = encoded.strip_prefix("sense:") {
        SENSE_RELATION_TYPES.iter().find(|i| i.key == key).copied()
    } else {
        None
    }
}

#[cfg(feature = "edit")]
fn render_synset_relation_group(
    info: &RelationTypeInfo,
    existing: &[SynsetId],
    pending_deletes: &[RelationKey],
    pending_adds: &[PendingRelation],
    on_pending_deletes_changed: EventHandler<Vec<RelationKey>>,
    on_pending_adds_changed: EventHandler<Vec<PendingRelation>>,
) -> Element {
    let visible_existing: Vec<SynsetId> = existing
        .iter()
        .filter(|id| {
            !pending_deletes
                .iter()
                .any(|d| d.key == info.key && d.target == **id)
        })
        .cloned()
        .collect();
    let adds: Vec<PendingRelation> = pending_adds
        .iter()
        .filter(|p| p.info.key == info.key)
        .cloned()
        .collect();
    if visible_existing.is_empty() && adds.is_empty() {
        return rsx! {};
    }

    rsx! {
        div {
            class: "relation-edit-group",
            span { class: "relation-edit-label", "{info.label}:" }
            for id in visible_existing {
                span {
                    key: "{id}",
                    class: "relation-edit-item",
                    Link {
                        to: Route::BySynset { synset: id.as_str().to_string() },
                        "{id.as_str()}"
                    }
                    button {
                        class: "edit-delete",
                        r#type: "button",
                        title: "Remove this relation",
                        onclick: {
                            let mut updated = pending_deletes.to_vec();
                            updated.push(RelationKey {
                                key: info.key,
                                target: id.clone(),
                                source_lemma: None,
                                target_lemma: None,
                            });
                            move |_| on_pending_deletes_changed.call(updated.clone())
                        },
                        "✗"
                    }
                }
            }
            for add in adds {
                span {
                    key: "{add.target}",
                    class: "relation-edit-item relation-edit-pending",
                    "{add.target_display}"
                    button {
                        class: "edit-delete",
                        r#type: "button",
                        title: "Remove this pending relation",
                        onclick: {
                            let remaining: Vec<PendingRelation> = pending_adds
                                .iter()
                                .filter(|p| *p != &add)
                                .cloned()
                                .collect();
                            move |_| on_pending_adds_changed.call(remaining.clone())
                        },
                        "✗"
                    }
                }
            }
        }
    }
}

#[cfg(feature = "edit")]
fn render_sense_relation_group(
    info: &RelationTypeInfo,
    existing: &[SenseRelation],
    pending_deletes: &[RelationKey],
    pending_adds: &[PendingRelation],
    on_pending_deletes_changed: EventHandler<Vec<RelationKey>>,
    on_pending_adds_changed: EventHandler<Vec<PendingRelation>>,
) -> Element {
    let visible_existing: Vec<SenseRelation> = existing
        .iter()
        .filter(|r| {
            !pending_deletes.iter().any(|d| {
                d.key == info.key
                    && d.target == r.target_synset
                    && d.source_lemma.as_deref() == Some(r.source_lemma.as_str())
                    && d.target_lemma.as_deref() == Some(r.target_lemma.as_str())
            })
        })
        .cloned()
        .collect();
    let adds: Vec<PendingRelation> = pending_adds
        .iter()
        .filter(|p| p.info.key == info.key)
        .cloned()
        .collect();
    if visible_existing.is_empty() && adds.is_empty() {
        return rsx! {};
    }

    rsx! {
        div {
            class: "relation-edit-group",
            span { class: "relation-edit-label", "{info.label}:" }
            for r in visible_existing {
                span {
                    key: "{r.source_lemma}-{r.target_lemma}-{r.target_synset}",
                    class: "relation-edit-item",
                    "{r.source_lemma} → {r.target_lemma} "
                    Link {
                        to: Route::BySynset { synset: r.target_synset.as_str().to_string() },
                        "({r.target_synset.as_str()})"
                    }
                    button {
                        class: "edit-delete",
                        r#type: "button",
                        title: "Remove this relation",
                        onclick: {
                            let mut updated = pending_deletes.to_vec();
                            updated.push(RelationKey {
                                key: info.key,
                                target: r.target_synset.clone(),
                                source_lemma: Some(r.source_lemma.clone()),
                                target_lemma: Some(r.target_lemma.clone()),
                            });
                            move |_| on_pending_deletes_changed.call(updated.clone())
                        },
                        "✗"
                    }
                }
            }
            for add in adds {
                span {
                    key: "{add.target}-{add.source_lemma:?}-{add.target_lemma:?}",
                    class: "relation-edit-item relation-edit-pending",
                    "{add.target_display}"
                    button {
                        class: "edit-delete",
                        r#type: "button",
                        title: "Remove this pending relation",
                        onclick: {
                            let remaining: Vec<PendingRelation> = pending_adds
                                .iter()
                                .filter(|p| *p != &add)
                                .cloned()
                                .collect();
                            move |_| on_pending_adds_changed.call(remaining.clone())
                        },
                        "✗"
                    }
                }
            }
        }
    }
}

/// The synset's relations, editable via a type dropdown and a search-based target picker
/// (rather than expecting the user to know synset ids) instead of making the read-only display
/// itself editable in place. Sense relations additionally need a source and target lemma,
/// since they connect specific senses, not whole synsets - both are picked from the relevant
/// synset's member list once a target is chosen. Nothing here calls the server; adding queues
/// a draft relation and deleting marks an existing one, both committed together with everything
/// else when `EditToggle`'s accept button runs the batch.
#[cfg(feature = "edit")]
#[component]
pub fn EditableRelations(props: EditableRelationsProps) -> Element {
    use crate::backend::edit::search_synsets;

    let synset = props.synset;
    let pending_deletes = props.pending_deletes;
    let pending_adds = props.pending_adds;
    let on_pending_deletes_changed = props.on_pending_deletes_changed;
    let on_pending_adds_changed = props.on_pending_adds_changed;

    let mut selected_key = use_signal(|| "syn:hypernym".to_string());
    let mut search_query = use_signal(String::new);
    let mut selected_target = use_signal(|| None::<crate::backend::edit::SynsetCandidate>);
    let mut source_lemma = use_signal(String::new);
    let mut target_lemma = use_signal(String::new);

    let mut suggestions = use_action(move |query: String| async move {
        search_synsets(query, None).await
    });

    let selected_info = find_relation_type(&selected_key());

    let can_queue = selected_target().is_some()
        && selected_info
            .map(|info| !info.is_sense || (!source_lemma().is_empty() && !target_lemma().is_empty()))
            .unwrap_or(false);

    rsx! {
        div {
            class: "relations relations-editing",
            for info in SYNSET_RELATION_TYPES.iter() {
                {render_synset_relation_group(
                    info,
                    synset_rel_values(&synset, info.key),
                    &pending_deletes,
                    &pending_adds,
                    on_pending_deletes_changed,
                    on_pending_adds_changed,
                )}
            }
            for info in SENSE_RELATION_TYPES.iter() {
                {render_sense_relation_group(
                    info,
                    sense_rel_values(&synset, info.key),
                    &pending_deletes,
                    &pending_adds,
                    on_pending_deletes_changed,
                    on_pending_adds_changed,
                )}
            }

            div {
                class: "relation-add-form",
                select {
                    class: "relation-type-select",
                    value: "{selected_key}",
                    onchange: move |e| {
                        selected_key.set(e.value());
                        selected_target.set(None);
                        search_query.set(String::new());
                        source_lemma.set(String::new());
                        target_lemma.set(String::new());
                    },
                    optgroup {
                        label: "Synset relations",
                        for info in SYNSET_RELATION_TYPES.iter() {
                            option { value: "syn:{info.key}", "{info.label}" }
                        }
                    }
                    optgroup {
                        label: "Sense relations",
                        for info in SENSE_RELATION_TYPES.iter() {
                            option { value: "sense:{info.key}", "{info.label}" }
                        }
                    }
                }
                if let Some(target) = selected_target() {
                    div {
                        class: "relation-selected-target",
                        "→ {target.members.join(\", \")} ({target.part_of_speech}) — {target.definition}"
                        button {
                            r#type: "button",
                            class: "edit-delete",
                            title: "Change target",
                            onclick: move |_| selected_target.set(None),
                            "✗"
                        }
                    }
                    if selected_info.map(|i| i.is_sense).unwrap_or(false) {
                        div {
                            class: "relation-lemma-pickers",
                            select {
                                value: "{source_lemma}",
                                onchange: move |e| source_lemma.set(e.value()),
                                option { value: "", "Source lemma…" }
                                for member in synset.members.iter() {
                                    option { value: "{member.lemma}", "{member.lemma}" }
                                }
                            }
                            "→"
                            select {
                                value: "{target_lemma}",
                                onchange: move |e| target_lemma.set(e.value()),
                                option { value: "", "Target lemma…" }
                                for lemma in target.members.iter() {
                                    option { value: "{lemma}", "{lemma}" }
                                }
                            }
                        }
                    }
                } else {
                    input {
                        class: "relation-search-input",
                        r#type: "text",
                        placeholder: "Search for a target word or synset…",
                        value: "{search_query}",
                        oninput: move |e| {
                            let value = e.value();
                            search_query.set(value.clone());
                            if !value.is_empty() {
                                suggestions.call(value);
                            }
                        },
                    }
                    if let Some(Ok(results)) = suggestions.value() {
                        {
                            let results = results.cloned();
                            rsx! {
                                if !results.is_empty() {
                                    ul {
                                        class: "relation-search-results",
                                        for candidate in results {
                                            li {
                                                key: "{candidate.id}",
                                                onclick: {
                                                    let candidate = candidate.clone();
                                                    move |_| {
                                                        selected_target.set(Some(candidate.clone()));
                                                        search_query.set(String::new());
                                                    }
                                                },
                                                "{candidate.members.join(\", \")} ({candidate.part_of_speech}) — {candidate.definition}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                button {
                    class: "list-add relation-queue",
                    r#type: "button",
                    disabled: !can_queue,
                    onclick: move |_| {
                        let Some(info) = selected_info else { return };
                        let Some(target) = selected_target() else { return };
                        let (source_lemma_val, target_lemma_val) = if info.is_sense {
                            (Some(source_lemma()), Some(target_lemma()))
                        } else {
                            (None, None)
                        };
                        let target_display = format!(
                            "{} ({}) — {}",
                            target.members.join(", "),
                            target.part_of_speech,
                            target.definition,
                        );
                        let mut adds = pending_adds.clone();
                        adds.push(PendingRelation {
                            info,
                            target: target.id.clone(),
                            target_display,
                            source_lemma: source_lemma_val,
                            target_lemma: target_lemma_val,
                        });
                        on_pending_adds_changed.call(adds);
                        selected_target.set(None);
                        search_query.set(String::new());
                        source_lemma.set(String::new());
                        target_lemma.set(String::new());
                    },
                    "+ Add relation"
                }
            }
        }
    }
}

#[cfg(not(feature = "edit"))]
#[component]
#[allow(unused_variables)]
pub fn EditableRelations(props: EditableRelationsProps) -> Element {
    rsx! {}
}
