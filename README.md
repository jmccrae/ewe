EWE Wordnet Editor
===================

EWE ("EWE Wordnet Editor") is a set of tools for working with wordnets built in the
[Global Wordnet Association](https://globalwordnet.github.io/) family of formats — it
started as an editor for the [Open English Wordnet](https://github.com/globalwordnet/english-wordnet),
but is designed to be configured for any wordnet project, in any language. It ensures
that changes are consistent and validates the resulting files.

The project is a Cargo workspace of three crates:

- [`ewe_lib`](ewe_lib) — the core Wordnet data model, storage, validation, and the
  "automaton" engine that applies batches of edits. Used by both tools below.
- [`ewe_cli`](ewe_cli) — a menu-driven command-line editor (documented in this file).
- [`ewe_dioxus`](ewe_dioxus) — a web/desktop UI built with [Dioxus](https://dioxuslabs.com/),
  offering both read-only browsing (search, lemma/synset pages, JSON/RDF/XML/Turtle
  export) and, behind an `edit` feature, a full in-browser editor. See
  [`ewe_dioxus/README.md`](ewe_dioxus/README.md) for details.

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
- validate
```

The web/desktop editor
-----------------------

`ewe_dioxus` provides a browser-based (or desktop) alternative to the CLI, built on
the same `ewe_lib` automaton engine. From the `ewe_dioxus` directory:

```bash
cargo install dioxus-cli
dx serve --features edit
```

This runs a read-only Wordnet browser plus, with `--features edit` enabled, a full
in-browser editor: editing lemmas, definitions, examples, relations, and ILI/Wikidata
identifiers; creating and deleting synsets; a change-log/history view; and saving
edits back out to the YAML source (with validation, and the option to revert to the
source instead). See [`ewe_dioxus/README.md`](ewe_dioxus/README.md) for full setup,
configuration, and route documentation.
