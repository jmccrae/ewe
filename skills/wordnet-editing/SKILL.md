---
name: wordnet-editing
description: Use when editing a Global Wordnet Association wordnet through the ewe_mcp MCP server's tools - adding/deleting/moving synsets and entries, changing members, definitions, examples, sources, ILI/Wikidata links or confidence scores, and adding, deleting, reversing or rewriting relations. Covers choosing between the single-action tools and apply_automaton, lexfile selection, the genus/differentia definition convention, when a synset needs a hypernym, sense vs. synset relations, safe entry moves, which edits are destructive, and when synset deletion/merging is actually appropriate - not the tool call syntax itself, which the tools' own schemas already cover.
---

# Editing a wordnet with ewe_mcp

This skill assumes an `ewe_mcp` MCP server is connected. Its tools fall into three
groups:

- **Reading and housekeeping:** `lookup_word`, `lookup_id`, `search_prefix`, `validate`,
  `save`, `reload`.
- **One tool per edit:** `add_synset`, `delete_synset`, `add_entry`, `delete_entry`,
  `move_entry`, `change_members`, `change_definition`, `add_example`, `update_example`,
  `delete_example`, `change_source`, `change_ili`, `change_wikidata`, `add_relation`,
  `delete_relation`, `reverse_relation`, `update_relations`, `set_confidence`,
  `fix_transitivity`.
- **Batches:** `apply_automaton`, which takes a list of the same edits (as
  `{"<tool name>": {...}}` items) and applies them in order.

Their parameters and return shapes are already fully described by the tools' own JSON
schemas - don't re-derive or guess at them here. What this skill covers is the
*judgment* a schema can't express: which tool, which lexfile, which relation, when a
hypernym is required, which edits are destructive, when deletion is actually the right
call.

## Single-action tools vs. `apply_automaton`

Every edit tool behaves like a one-item `apply_automaton` batch: it applies the edit,
validates, and auto-saves if the wordnet is clean. Otherwise it leaves the edit applied
in memory but unsaved and reports the validation errors. Every one of them also takes
`dry_run`.

- Use the **single-action tool** for one self-contained change - fixing a definition,
  adding an example, adding a relation between two existing synsets.
- Use **`apply_automaton`** when the change needs several steps that only make sense
  together, and in particular whenever a later step refers to something an earlier step
  created via `"last"`. The commonest case is a new synset plus its hypernym: on its own,
  `add_synset` leaves a synset with no hypernym, which fails validation and so is not
  saved.

## Structure of a wordnet

- A synset id is 8 digits plus a part-of-speech suffix (e.g. `00001740-n`). **Never
  invent one** - `add_synset` assigns it and returns it as `last_synset_id`. To reference a
  synset you just created later in the *same* `apply_automaton` call (e.g. to attach a
  hypernym right after creating it), use the synset-ref value `"last"` instead of
  guessing the id. `"last"` means nothing to a single-action tool.
- Every synset needs a definition: short (3-15 words), phrased as a genus and a
  differentia. E.g. "ewe" is a "sheep" (genus), differentiated by "female"
  (differentia).
- Every noun and verb synset needs at least one hypernym link. A synset with none will
  fail `validate` and the validation that runs after every edit (unless the project's
  `settings.toml` turns that check off).

## Adding a new synset

1. Settle on a definition and every lemma that belongs in the synset.
2. Find a hypernym target: `lookup_word`/`search_prefix` the closest existing concept
   and note its synset id (`lookup_word(..., sense_ids: true)` if you need a specific
   sense, not just any synset for that lemma).
3. Pick the lexfile it belongs in - see `references/lexfiles.md`.
4. One `apply_automaton` call: an `add_synset` action (definition, lexfile, pos,
   lemmas), followed by an `add_relation` action with `relation: "hypernym"`,
   `source: "last"`, and `target: <the id from step 2>`.
   Do this as one batch rather than two single-action calls, so the synset is never
   saved without its hypernym.
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

- **`add_relation`** adds one relation (and its inverse).
- **`delete_relation`** removes *every* relation between the two synsets (or the two
  senses) in both directions - it takes no relation type. Check `lookup_id` on the source
  first, so you know everything it will remove.
- **`reverse_relation`** flips the direction of the relation(s) from `source` to
  `target`. Use it to fix a link added the wrong way round (e.g. a hypernym that should
  have been a hyponym) rather than deleting and re-adding it.
- **`update_relations`** *replaces all* relations to and from a synset, incoming ones
  included, with the list you give. It is a rewrite tool, not a way to add one link -
  anything you leave out is deleted. Prefer `add_relation`/`delete_relation` for targeted
  changes, and always `dry_run` an `update_relations` first.
- **`fix_transitivity`** works on the whole wordnet, not one synset: it deletes every
  hypernym link A -> C that is already implied by A -> B -> C. Run it after a hierarchy
  restructure has left such shortcuts, `dry_run` first, and don't run it routinely.

### Entries

To relocate a lemma from one synset to another, use `move_entry` rather than a
`delete_entry` + `add_entry` pair - it carries the entry's existing sense relations,
forms, and pronunciations across instead of dropping them. Use plain `add_entry`/
`delete_entry` when there's nothing to carry over.

`change_members` sets the synset's *complete* member list, in order: lemmas you leave
out are deleted from the synset, and new ones are added as fresh senses. It's the right
tool for reordering members; for adding or
removing a single lemma, `add_entry`/`delete_entry` say more plainly what you mean. Never
use it to relocate a lemma - that drops the sense's relations, unlike `move_entry`.

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

`change_definition`, `add_example`/`update_example`/`delete_example`, `change_source`,
`change_ili`, and `change_wikidata` cover the rest - their parameters are in the tool
schema, not repeated here. Conventions worth knowing:

- A rewritten definition follows the same genus/differentia convention as a new one.
- Examples should use Unicode curly quotes (‘ ’), not straight quotes.
- Examples are numbered from 1 in the order `lookup_id` shows them. Deleting one shifts
  the numbers of the ones after it, so when deleting several, go from the highest number
  down (or re-check with `lookup_id` in between).
- An example's `source` is optional - omit it rather than inventing one.
- `change_source` records where the synset itself came from (its provenance). Set it
  only when you know the source; don't invent one.
- `change_ili` links the synset to the Interlingual Index; only set an ILI id you have
  confirmed matches this exact sense, since two synsets sharing one is a validation
  error.
- `change_wikidata` *replaces* the synset's Wikidata Q-ids with the list you give, so
  include any existing ones you want to keep; an empty list removes them all.

## Confidence scores

A confidence score records how sure the wordnet is of a synset, sense, entry, definition,
example or relation (WN-LMF `confidenceScore`, 0.0-1.0; no score means 1.0). Set one only
when you have an actual reason to doubt the item - e.g. an automatically generated or
unverified addition - not as routine annotation of your own edits.

- When creating something, give the score inline: `confidence` on `add_synset`, `add_entry`,
  `add_example` and `add_relation`; `definition_confidence` on `add_synset`; `entry_confidence`
  on `add_entry`.
- To re-score something that already exists, use `set_confidence`.
- Rewording a definition or example keeps its existing score. If your edit resolves the doubt,
  clear the score by sending `set_confidence` without `confidence`.

## Check before you commit

For anything beyond a single trivial action, call the tool (or `apply_automaton`) with
`dry_run: true` first. It runs the exact same apply against an in-memory copy and
reports `would_succeed` plus any `validation_errors` the change would introduce,
without touching the real wordnet. Fix and re-check if needed, then call again with
`dry_run` omitted (or `false`) to actually apply.

A real apply already validates and auto-saves when the result is clean. If it comes
back with `saved: false` and validation errors, fix the underlying issue and reapply -
don't reach for `save(force: true)`. That's a deliberate, rare override for a change
you've decided to keep despite an existing validation error, not a routine step.
