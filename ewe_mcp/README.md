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

Reloading after a rebuild
--------------------------

Claude Code (and other MCP clients) launch `ewe-mcp` once per session and keep that
process running for the session's whole lifetime - rebuilding the binary (e.g. after
pulling a new `ewe` commit) does nothing to a client that's already connected to the
old process. If a fix you expect isn't showing up, check that the running server was
actually built after your change first, before assuming the bug is still live:

- In Claude Code, run `/mcp` to see each configured server's status; use it to restart
  the `ewe` server, or exit and restart the session so it relaunches `ewe-mcp` from the
  (now up to date) binary on `$PATH`/in your MCP config.
- In Claude Desktop, quit and reopen the app, or use the MCP server toggle in Settings
  to disable and re-enable `ewe`.

This is a different problem from the wordnet *files* changing underneath an
already-running server (e.g. a `git checkout` in the same working directory, made
outside the MCP session) - restarting the client fixes that too (it relaunches
`ewe-mcp`, which reloads from disk), but for that case the lighter-weight `reload` tool
below is usually more convenient than tearing down the whole session.

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
  (non-dry-run) apply, saves automatically if the result validates cleanly and the
  on-disk wordnet hasn't changed since this server last loaded/saved it; otherwise the
  change stays applied in memory but unsaved, with the validation errors (and a `stale`
  flag, if that's why) reported back so the caller can fix-and-reapply, call `reload`,
  or call `save` with `force`. With `dry_run: true`, applies against an in-memory clone
  and discards it - the real wordnet is never touched, but the report is otherwise
  identical (including any validation errors the change would introduce), since it runs
  through the exact same code path as a real apply.
- **`save(force?)`** — persists any pending in-memory changes to disk. Skipped unless
  the wordnet validates cleanly and the on-disk files haven't changed since this server
  last loaded/saved them; the report says which (`validation_errors`, `stale`) and
  `force: true` overrides both.
- **`reload()`** — reloads the wordnet from disk, replacing the in-memory copy. Use
  this after an out-of-band change to the wordnet files made outside the MCP session
  (a `git checkout`, branch switch, or hand edit) - `save`/`apply_automaton` refuse to
  write over such a change (reporting `stale: true`) until this is called. Discards any
  pending in-memory changes this session made but never saved; the report's
  `discarded_unsaved_changes` says whether that happened, so it's worth checking before
  relying on `reload` mid-session.

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
