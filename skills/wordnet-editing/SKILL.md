---
name: wordnet-editing
description: Use when editing a Global Wordnet Association wordnet (adding/deleting synsets, entries, relations, examples, or changing ILI/Wikidata links) through the ewe_mcp MCP server's tools. Covers lexfile selection, the genus/differentia definition convention, when a synset needs a hypernym, sense vs. synset relations, safe entry moves, and when synset deletion/merging is actually appropriate - not the tool call syntax itself, which the tools' own schemas already cover.
---

# Editing a wordnet with ewe_mcp

This skill assumes an `ewe_mcp` MCP server is connected, exposing `lookup_word`,
`lookup_id`, `search_prefix`, `validate`, `apply_automaton`, and `save`. Their
parameters and return shapes are already fully described by the tools' own JSON
schemas - don't re-derive or guess at them here. What this skill covers is the
*judgment* a schema can't express: which lexfile, which relation, when a hypernym is
required, when deletion is actually the right call.

## Structure of a wordnet

- A synset id is 8 digits plus a part-of-speech suffix (e.g. `00001740-n`). **Never
  invent one** - `apply_automaton`'s `add_synset` action assigns it. To reference a
  synset you just created later in the *same* `apply_automaton` call (e.g. to attach a
  hypernym right after creating it), use the synset-ref value `"last"` instead of
  guessing the id.
- Every synset needs a definition: short (3-15 words), phrased as a genus and a
  differentia. E.g. "ewe" is a "sheep" (genus), differentiated by "female"
  (differentia).
- Every noun and verb synset needs at least one hypernym link. A synset with none will
  fail `validate`/`apply_automaton`'s post-apply validation.

## Adding a new synset

1. Settle on a definition and every lemma that belongs in the synset.
2. Find a hypernym target: `lookup_word`/`search_prefix` the closest existing concept
   and note its synset id (`lookup_word(..., sense_ids: true)` if you need a specific
   sense, not just any synset for that lemma).
3. Pick the lexfile it belongs in - see `references/lexfiles.md`.
4. One `apply_automaton` call: an `add_synset` action (definition, lexfile, pos,
   lemmas), followed by an `add_relation` action with `relation: "hypernym"`,
   `source: "last"`, and `target: <the id from step 2>`.
5. Always `dry_run: true` first for anything beyond a single trivial action - see
   "Check before you commit" below.

## Updating relations

Relations are either between two **synsets** (just `source`/`target`) or two **senses**
(also give `source_sense`/`target_sense`, as a sense id or - more convenient when you
already know the lemma - `source_lemma`/`target_lemma`). Get a sense id via
`lookup_word(word, sense_ids: true)`.

The full relation name lists (which apply to synsets vs. senses) are in
`references/relations.md` rather than repeated here. Most have an inverse the tool
maintains automatically (`hypernym`/`hyponym`, `holo_*`/`mero_*`, etc.) - add whichever
direction reads naturally, not both.

### Entries

To relocate a lemma from one synset to another, use `move_entry` rather than a
`delete_entry` + `add_entry` pair - it carries the entry's existing sense relations,
forms, and pronunciations across instead of dropping them. Use plain `add_entry`/
`delete_entry` when there's nothing to carry over.

## Deleting synsets

Only appropriate in a few specific cases: merging/deduplicating two synsets that
describe the same sense, correcting a synset introduced by an error, or - only if
explicitly asked - a word that genuinely doesn't exist in English. It is not a general
cleanup tool.

- `reason` should name the issue driving the deletion, e.g. `"Duplicate (#123)"`. This
  convention isn't enforced by `apply_automaton` itself (unlike `ewe_cli`'s interactive
  menu, which rejects a reason without a `(#N)` suffix) - hold yourself to it anyway,
  since it's the only record of *why* a synset disappeared.
- Prefer giving `superseded_by`: it hands off the deleted synset's entries, relations,
  and examples to the target and leaves a deprecation record. Omit it only for a
  no-trail permanent removal - appropriate for e.g. a synset you created earlier in the
  *same* session and decided against, not for anything that might already be referenced
  elsewhere.

## Other edits

`change_definition`, `add_example`/`update_example`/`delete_example`, `change_ili`, and
`change_wikidata` actions cover the rest - their parameters are in the tool schema, not
repeated here. Two conventions worth knowing:

- Examples should use Unicode curly quotes (‘ ’), not straight quotes.
- An example's `source` is optional - omit it rather than inventing one.

## Check before you commit

For anything beyond a single trivial action, call `apply_automaton` with
`dry_run: true` first. It runs the exact same apply against an in-memory copy and
reports `would_succeed` plus any `validation_errors` the change would introduce,
without touching the real wordnet. Fix and re-check if needed, then call again with
`dry_run` omitted (or `false`) to actually apply.

A real apply already validates and auto-saves when the result is clean. If it comes
back with `saved: false` and validation errors, fix the underlying issue and reapply -
don't reach for `save(force: true)`. That's a deliberate, rare override for a change
you've decided to keep despite an existing validation error, not a routine step.
