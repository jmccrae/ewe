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
- add_synset:                # `id` and `ili` are optional: without `id` one is derived
    id: 02084071-n             # from the definition
    ili: i46360
    definition: a member of the genus Canis
    lexfile: noun.animal
    lemmas: [dog]
- change_ili_definition:      # English gloss written as the XML `ILIDefinition`; needed in
    synset: 00001740-n         # non-English lexicons for synsets with no ILI
    ili_definition: a member of the genus Canis
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
- change_reconstructed:      # mark a lemma as a reconstructed (unattested) form, shown as `*lemma`
    synset: 00001740-n       # (`reconstructed: false` clears it; add_entry takes the flag too)
    lemma: dog
    reconstructed: true
- validate
```

Reconstructed forms are stored as a flag on the entry (`reconstructed: true` in the YAML), never
as an asterisk in the lemma. `ewe export xml --lmf-version 1.5` writes it as WN-LMF 1.5's
`Lemma/@reconstructed`; the default, `--lmf-version 1.4`, has no such attribute, so it is
omitted and a warning is printed with the number of lemmas affected.

## Names (proper nouns)

Names are synsets with part of speech `e` (ids ending `-e`, `name.*` lexfiles, entries in
`entries-<LETTER>-name.yaml`). `ewe export xml|wndb|rdf` and `ewe stats` take `--names` to choose
which of them a release or report covers:

| `--names` | Result |
| --- | --- |
| `include` (default) | Everything as stored; names keep their `-e` ids and part of speech `e`. |
| `exclude` | The core edition: no names, and no relation, sense or member that refers to one. |
| `only` | Just the names, without anything that refers to the common vocabulary. |
| `merged` | The names as ordinary nouns (`-n` ids, `noun.*` lexfiles, a name entry joined to the noun entry of the same lemma), as in releases before names had their own part of speech. |

WNDB has no part of speech for names: with `include` they are left out (a note is printed),
`only` is an error and `merged` writes them into `data.noun`/`index.noun`. A
`src/sense-orders.csv` row may list synsets that are not exported; the others are still ordered
as the row says.

```
ewe export xml core.xml --names exclude
ewe export wndb wndb/ --names merged
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
- `--skip-ili-definition`: non-English lexicons (`language` in `settings.toml`) must give every synset with no ILI an `ili_definition` (or set its `ili` to an empty string)
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
