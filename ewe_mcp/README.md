# ewe_mcp

An [MCP](https://modelcontextprotocol.io/) server exposing wordnet queries and
schema-validated automaton edits for wordnets built in the
[Global Wordnet Association](https://globalwordnet.github.io/) family of formats, built
on the [`ewe_lib`](../ewe_lib) data model, validation, and automaton engine.

It's the same automaton engine [`ewe_cli`](../ewe_cli) scripts drive, but linked
directly (no shelling out to a binary, no scraping text output) and speaking structured
JSON over the Model Context Protocol instead - so an agent gets a real JSON Schema for
every edit action, structured validation results, and a `dry_run` mode that exercises a
real apply against an in-memory copy before anything touches disk.

Installation
------------

Release builds of `ewe-mcp` can be obtained from the
[release section](https://github.com/jmccrae/ewe/releases), or built from source:

    cargo build --release -p ewe-mcp

Usage
-----

`ewe-mcp` speaks MCP over stdio. Point it at a wordnet directory (the same layout
`ewe_cli` uses - a folder containing `entries-*.yaml`, or a project root containing
`src/yaml/entries-*.yaml`) with `--wordnet`:

    ewe-mcp --wordnet /path/to/wn

If `--wordnet` is omitted, it looks for a wordnet in the current directory (or
`./src/yaml/`), matching `ewe_cli`'s behavior.

To point Claude Code at it, add an entry to your MCP server configuration:

```json
{
  "mcpServers": {
    "ewe": {
      "command": "/path/to/ewe-mcp",
      "args": ["--wordnet", "/path/to/wn"]
    }
  }
}
```

Tools
-----

- **`lookup_word(word, ignore_case?, sense_ids?)`** — every synset/sense a lemma
  belongs to, as JSON.
- **`lookup_id(id)`** — a synset or sense by its id, as JSON.
- **`search_prefix(prefix, max_results?)`** — lemmas starting with a prefix, for
  autocomplete-style lookups.
- **`validate()`** — runs full validation over the loaded wordnet and returns the
  errors found (empty if none).
- **`apply_automaton(actions, dry_run?)`** — applies a batch of automaton actions (add
  or delete synsets/entries/relations/examples, change definitions, etc. - see
  [`ewe_cli/README.md`](../ewe_cli/README.md) for the full action reference). Rejects
  batches containing a `validate` action - call `validate` instead. On a real
  (non-dry-run) apply, saves automatically if the result validates cleanly; otherwise
  the change stays applied in memory but unsaved, with the validation errors reported
  back so the caller can fix and reapply, or call `save` with `force`. With
  `dry_run: true`, applies against an in-memory clone and discards it - the real
  wordnet is never touched, but the report is otherwise identical (including any
  validation errors the change would introduce), since it runs through the exact same
  code path as a real apply.
- **`save(force?)`** — persists any pending in-memory changes to disk. Skipped
  (returning the current validation errors instead) unless the wordnet validates
  cleanly or `force` is set.

Every automaton action's shape (and the arguments to every other tool) is described by
a JSON Schema derived directly from `ewe_lib`'s `Action` type, so a malformed or
semantically invalid edit is rejected by the client or by `apply_automaton` itself
before it can reach the wordnet - the schema is always in sync with what the automaton
engine actually accepts.

Companion skill
----------------

The schema covers *what* each tool accepts; it doesn't cover *when* to use one action
over another - lexfile selection, the genus/differentia definition convention, when
`delete_synset` is appropriate vs. `move_entry`/merging, and similar judgment calls.
That's what the companion `wordnet-editing` Claude Code Skill is for. See the root
[`README.md`](../README.md#the-wordnet-editing-skill) for installation.
