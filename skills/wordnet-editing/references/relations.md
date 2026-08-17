# Relation names

The `relation` argument to `apply_automaton`'s `add_relation`/`delete_relation`/
`update_relations` actions. Which list applies depends on whether the relation is
between two synsets (no `source_sense`/`target_sense` given) or two senses (`source_sense`
and `target_sense`, or `source_lemma`/`target_lemma`, given) — see the "Updating
relations" section of `SKILL.md`.

## Synset relations

```
also
attribute
causes
domain_region
domain_topic
exemplifies
entails
has_domain_region
has_domain_topic
is_exemplified_by
holo_location
holo_member
holo_part
holo_portion
holo_substance
holonym
hypernym
hyponym
instance_hypernym
instance_hyponym
is_caused_by
is_entailed_by
mero_location
mero_member
mero_part
mero_portion
mero_substance
meronym
similar
other
feminine
masculine
```

Most of these have an inverse the tool maintains automatically (`hypernym`/`hyponym`,
`holo_*`/`mero_*`/`holonym`/`meronym`, `is_caused_by`/`causes`, `is_entailed_by`/
`entails`, `has_domain_*`/`domain_*`, `is_exemplified_by`/`exemplifies`) — add whichever
direction reads naturally; you don't need to add both.

## Sense relations

```
antonym
also
participle
pertainym
derivation
domain_topic
has_domain_topic
domain_region
has_domain_region
exemplifies
is_exemplified_by
is_pertainym_of
similar
agent
material
event
instrument
location
by_means_of
undergoer
property
result
state
uses
destination
body_part
vehicle
other
```
