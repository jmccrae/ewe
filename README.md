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
- [`ewe_cli`](ewe_cli) — a menu-driven command-line editor. See
  [`ewe_cli/README.md`](ewe_cli/README.md) for installation, usage, and automaton-file
  scripting.
- [`ewe_dioxus`](ewe_dioxus) — a web/desktop UI built with [Dioxus](https://dioxuslabs.com/),
  offering both read-only browsing (search, lemma/synset pages, JSON/RDF/XML/Turtle
  export) and, behind an `edit` feature, a full in-browser editor. See
  [`ewe_dioxus/README.md`](ewe_dioxus/README.md) for details.

The command-line editor
------------------------

`ewe_cli` is a menu-driven CLI for interactively editing a Wordnet, and can also be
automated by scripting a batch of changes as a YAML "automaton" file:

```bash
cargo build --release
ewe automaton.yaml /path/to/wn
```

See [`ewe_cli/README.md`](ewe_cli/README.md) for installation, the interactive menu, and
the full automaton-file format.

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
