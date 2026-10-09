# ewe_cli

A menu-driven command-line editor for wordnets built in the [Global Wordnet Association](https://globalwordnet.github.io/) family of formats, built on the [`ewe_lib`](../ewe_lib) data model and automaton engine.

Installation
------------

Release builds of `ewe_cli` can be obtained from the [release section](https://github.com/jmccrae/ewe/releases).
These are executables and can be run directly. We recommend saving these to the same
folder that contains the Git repository for the wordnet you're editing. EWE can be
started by executing this file.

Usage
-----

EWE is menu-driven, please choose the appropriate option when it has started
you should see something like this:

```

         ,ww                             
   wWWWWWWW_)  Welcome to EWE            
   `WWWWWW'    - EWE Wordnet Editor       
    II  II                               

Loading WordNet
████████████████████████████████████████████████████████████████████████ 73/73

Please choose an option:
1. Add/delete/move entry
2. Add/delete a synset
3. Change a definition
4. Change an example
5. Change a relation
6. Validate
X. Exit EWE
Option> 
```

Building EWE
------------

EWE can be built with Cargo as follows

    cargo build --release

Automating EWE
--------------

EWE can be automated with an automaton file as follows

    ewe automaton.yaml /path/to/wn

An example of the usage of the automaton file is given below

```yaml
---
- add_entry:
    synset: 00001740-n
    lemma: bar
    pos: n
- delete_entry:
    synset: 00001740-n
    lemma: bar
- move_entry:
    synset: 00001740-n
    lemma: bar
    target_synset: 00001741-n
- change_members:
    synset: 00001740-n
    members: ["entity", "thing"]
- add_synset:
    definition: something or someone
    lexfile: noun.animal
    pos: n
    lemmas:
      - bar
- delete_synset:
    synset: 00001740-n
    reason: "Duplicate (#123)"
    superseded_by: 00001741-n
- change_definition:
    synset: 00001740-n
    definition: This is a definition
- add_example:
    synset: 00001740-n
    example: This is an example
    source: This is a source
- update_example:
    synset: 00001740-n
    number: 1
    example: This is an updated example
- delete_example:
    synset: 00001740-n
    number: 1
- add_relation:
    source: 00001740-n
    relation: hypernym
    target: 00001741-n
- delete_relation:
    source: 00001740-n
    source_sense: "example%1:09:00::"
    target: 00001741-n
    target_sense: "target%1:10:00::'"
- reverse_relation:
    source: 00001740-n
    target: 00001741-n
- update_relations:
    source: 00001740-n
    relations:
        - relation: hypernym
          target: 00001741-n
        - relation: hyponym
          target: 00001742-n
          source_lemma: test
          target_lemma: test
- set_confidence:            # WN-LMF confidenceScore, 0.0-1.0, of the synset itself
    synset: 00001740-n
    confidence: 0.8
- set_confidence:            # ... or one thing inside it: `sense`, `entry`, `definition`,
    synset: 00001740-n       # `example` (1-indexed), or a `relation` to a `target`
    relation: hypernym
    target: 00001741-n
    confidence: 0.6
- set_confidence:            # a sense relation: `sense` is its source (`target_sense` can
    synset: 00001740-n       # be omitted for domain_topic/domain_region/exemplifies/other
    sense: "lemma=bar"       # targeting the synset itself)
    relation: antonym
    target: 00001742-n
    target_sense: "lemma=baz"
    confidence: 0.5
- set_confidence:            # omitting `confidence` clears it (i.e. back to 1.0)
    synset: 00001740-n
    example: 1
- validate
```

## Non-interactive validation (CI)

`ewe validate` runs validation without any prompting and exits with status `0` if there are no
errors, `1` if there are validation errors, and `2` if the wordnet could not be loaded.

```
ewe validate --wordnet path/to/wordnet
```

Individual checks can be turned off:

- `--skip-symmetric`: symmetric relation checks
- `--skip-duplicate-ili`: duplicate ILIs
- `--skip-duplicate-definitions`: duplicate definitions
- `--skip-similar`: `similar` links must join an `a` and an `s` synset
- `--skip-hypernym`: hypernym/instance_hypernym checks (cross-POS, instance targets, missing hypernym, hypernym/instance conflict, transitivity)

A project can also turn checks off permanently in the `[validation]` table of its
`settings.toml`, using the flag names with underscores. The file is looked for in the wordnet's YAML folder
and up to two folders above it, so `<project>/settings.toml` is found for `<project>/src/yaml/`.

```toml
[validation]
skip_duplicate_ili = true
skip_hypernym = true
```

These settings apply wherever EWE validates (`ewe validate`, the interactive editor, `save` and
the automaton `validate` action), and also in `ewe-mcp` and the Dioxus app. On the command line,
a check is skipped if either a flag or `settings.toml` skips it. An unknown key in
`[validation]` is an error (exit status `2`).
### Confidence scores

Actions that create or change something take an optional WN-LMF confidence score (0.0-1.0),
so it can be set in the same step. Omitting it means no score, which WN-LMF treats as 1.0.

```yaml
- add_synset:
    definition: a feathered animal
    lexfile: noun.animal
    lemmas: [bird]
    confidence: 0.7              # the new synset
    definition_confidence: 0.6   # its definition
- add_entry:
    synset: last
    lemma: fowl
    pos: n
    confidence: 0.8              # the new sense
    entry_confidence: 0.9        # the lexical entry (re-scores it if "fowl" already had one)
- add_example:
    synset: last
    example: the bird sang
    confidence: 0.5
- add_relation:                  # a symmetric relation (also, similar, antonym, ...) is
    source: last                 # scored in both directions
    relation: hypernym
    target: 00001741-n
    confidence: 0.4
```

`change_definition`, `update_example` and each `update_relations` item take `confidence` too;
there, omitting it keeps the item's current score. To change the score of something that
already exists without otherwise editing it, use `set_confidence` (examples above).

Confidence scores appear in the YAML source as a `confidence:` key on a synset, sense,
entry or example, and on definitions and relation targets by writing the list item as a map,
e.g. `hypernym: [{target: 00001741-n, confidence: 0.6}]`. Items without a score keep the plain
form, so a wordnet with no scores is unchanged.
