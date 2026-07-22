//! The relation types the editor offers, matching exactly the fields `components::synset`
//! already renders read-only (see its `synset_rels`/`sense_rels` calls) - not the full set
//! `MemberSynset` exposes, some of which (e.g. `is_agent_of`) aren't shown anywhere yet.
//!
//! About half of these fields are computed by reverse lookup rather than stored directly (for
//! example `hyponym` is derived from *other* synsets' `hypernym`, not stored on this one) - see
//! `MemberSynset::from_synset` in oewn_lib. Deleting is unaffected (`Action::DeleteRelation`
//! clears links between the pair in both directions regardless of type), but *adding* one of
//! these needs the inverse relation inserted with source and target swapped. `store_as` and
//! `swapped` capture that so the rest of the editor can stay agnostic to it.
//!
//! Only referenced from the `edit` feature's editor UI, so everything here is otherwise dead
//! code on the default (non-edit) web/mobile build.
#![allow(dead_code)]

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct RelationTypeInfo {
    /// The field name as shown in the read-only view and used to key pending changes.
    pub key: &'static str,
    pub label: &'static str,
    pub is_sense: bool,
    /// The relation string to actually submit in `Action::AddRelation`/`SynsetRelType`/
    /// `SenseRelType::from` - differs from `key` for reverse-computed fields.
    pub store_as: &'static str,
    /// If true, adding this relation must swap source/target (and, for sense relations,
    /// source_lemma/target_lemma) because `store_as` is the *inverse* relation type.
    pub swapped: bool,
}

const fn forward(key: &'static str, label: &'static str) -> RelationTypeInfo {
    RelationTypeInfo { key, label, is_sense: false, store_as: key, swapped: false }
}

const fn inverse(key: &'static str, label: &'static str, store_as: &'static str) -> RelationTypeInfo {
    RelationTypeInfo { key, label, is_sense: false, store_as, swapped: true }
}

const fn sense_forward(key: &'static str, label: &'static str) -> RelationTypeInfo {
    RelationTypeInfo { key, label, is_sense: true, store_as: key, swapped: false }
}

const fn sense_inverse(key: &'static str, label: &'static str, store_as: &'static str) -> RelationTypeInfo {
    RelationTypeInfo { key, label, is_sense: true, store_as, swapped: true }
}

pub const SYNSET_RELATION_TYPES: &[RelationTypeInfo] = &[
    forward("hypernym", "Hypernym"),
    inverse("hyponym", "Hyponym", "hypernym"),
    forward("instance_hypernym", "Instance Of"),
    inverse("instance_hyponym", "Has Instance", "instance_hypernym"),
    forward("attribute", "Attribute"),
    forward("causes", "Causes"),
    inverse("is_caused_by", "Is Caused By", "causes"),
    forward("domain_region", "Used in Region"),
    inverse("has_domain_region", "Used in this Region", "domain_region"),
    forward("domain_topic", "Subject"),
    inverse("has_domain_topic", "Is a Subject of", "domain_topic"),
    forward("exemplifies", "Is an Example Of"),
    inverse("is_exemplified_by", "Has Example", "exemplifies"),
    forward("entails", "Entails"),
    inverse("is_entailed_by", "Is Entailed By", "entails"),
    forward("mero_location", "Is Located At"),
    inverse("holo_location", "Location Of", "mero_location"),
    inverse("holo_member", "Is Member Of", "mero_member"),
    forward("mero_member", "Has Member"),
    inverse("holo_part", "Is Part Of", "mero_part"),
    forward("mero_part", "Has Part"),
    inverse("holo_substance", "Is Made Of", "mero_substance"),
    forward("mero_substance", "Makes"),
    forward("meronym", "Meronym"),
    inverse("holonym", "Holonym", "meronym"),
    forward("similar", "Similar To"),
    forward("feminine", "Feminine Form"),
    forward("masculine", "Masculine Form"),
    forward("also", "See Also"),
    forward("other", "Other Related Synsets"),
];

pub const SENSE_RELATION_TYPES: &[RelationTypeInfo] = &[
    sense_forward("antonym", "Antonym"),
    sense_forward("participle", "Participle"),
    sense_forward("pertainym", "Of or Pertaining To"),
    sense_forward("derivation", "Derived From"),
    sense_forward("exemplifies_sense", "Is an Example Of"),
    sense_inverse("is_exemplified_by_sense", "Has Example", "exemplifies_sense"),
    sense_forward("agent", "Agent"),
    sense_forward("material", "Material"),
    sense_forward("event", "Event"),
    sense_forward("instrument", "Instrument"),
    sense_forward("location", "Location"),
    sense_forward("by_means_of", "By Means Of"),
    sense_forward("undergoer", "Undergoer"),
    sense_forward("property", "Property"),
    sense_forward("result", "Result"),
    sense_forward("state", "State"),
    sense_forward("uses", "Uses"),
    sense_forward("destination", "Destination"),
    sense_forward("body_part", "Body Part"),
    sense_forward("vehicle", "Vehicle"),
];
