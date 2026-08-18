//! `ewe-mcp` - an MCP server exposing wordnet queries and automaton edits over stdio.
//!
//! Speaks JSON-RPC on stdout exclusively (the MCP stdio transport's contract), so nothing in
//! this crate - or anything it calls into in `ewe_lib` - may write to stdout outside of the
//! `rmcp` framework itself. Diagnostics, if ever needed, must go to stderr.

use clap::Parser;
use ewe_lib::automaton::Action;
use ewe_lib::change_manager::ChangeList;
use ewe_lib::progress::NullProgress;
use ewe_lib::wordnet::{Lexicon, LexiconHashMapBackend, SenseId, SynsetId};
use rmcp::ServiceExt;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

#[derive(Parser, Debug)]
#[command(
    name = "ewe-mcp",
    version,
    about = "EWE MCP server - exposes wordnet queries and automaton edits over the Model Context Protocol",
    long_about = None
)]
struct Cli {
    /// Path to the WordNet database (a folder containing entries-*.yaml files, or its parent)
    #[arg(long, value_name = "PATH")]
    wordnet: Option<PathBuf>,
}

/// Resolves `path` (or the current directory, if not given) to a WordNet YAML folder and loads
/// it. Small and self-contained enough that it's duplicated in miniature from `ewe_cli`'s
/// `locate_wordnet` rather than shared via `ewe_lib` - the two callers want different
/// error-reporting shapes (CLI eprintln+exit vs. an MCP startup error).
fn locate_wordnet(path: Option<PathBuf>) -> Result<(String, LexiconHashMapBackend), String> {
    let path = if let Some(path) = path {
        if path.join("entries-a.yaml").exists() {
            path.to_string_lossy().to_string()
        } else if path.join("src/yaml/entries-a.yaml").exists() {
            path.join("src/yaml/").to_string_lossy().to_string()
        } else {
            return Err(format!(
                "Could not find WordNet at {}",
                path.to_string_lossy()
            ));
        }
    } else if Path::new("./src/yaml/entries-a.yaml").exists() {
        "./src/yaml/".to_owned()
    } else if Path::new("./entries-a.yaml").exists() {
        "./".to_owned()
    } else {
        return Err("Could not find a WordNet - pass --wordnet PATH".to_string());
    };
    let mut progress = NullProgress;
    let wn = LexiconHashMapBackend::new()
        .load(&path, &mut progress)
        .map_err(|e| format!("Could not load WordNet from {}: {}", path, e))?;
    Ok((path, wn))
}

struct ServerState {
    path: String,
    wn: LexiconHashMapBackend,
    changes: ChangeList,
    /// The most recent mtime seen under `path` as of the last successful `load`/`reload`/
    /// `save` - `save` compares this against the current on-disk state to detect a change
    /// made outside this process (see `perform_save`/the `reload` tool). `None` if it
    /// couldn't be determined (e.g. `path` didn't exist), in which case the staleness check
    /// is skipped rather than blocking every future save.
    loaded_mtime: Option<SystemTime>,
}

/// Best-effort snapshot of `path`'s current mtime - `None` rather than an error, since a
/// failure here (e.g. a transient read error) should degrade to "skip the staleness check"
/// rather than making every load/save in `ServerState` fallible.
fn source_mtime(path: &str) -> Option<SystemTime> {
    ewe_lib::source_mtime::latest_source_mtime(path).ok()
}

#[derive(Clone)]
struct EweMcpServer {
    state: Arc<Mutex<ServerState>>,
}

#[derive(Serialize)]
struct LookupResult {
    synset_id: String,
    synset: ewe_lib::wordnet::Synset,
    sense_id: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct LookupWordParams {
    /// The word (lemma) to look up.
    word: String,
    /// Case-insensitive lookup. Defaults to false.
    #[serde(default)]
    ignore_case: bool,
    /// Include each match's sense id in the result. Defaults to false.
    #[serde(default)]
    sense_ids: bool,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct LookupIdParams {
    /// A synset id (e.g. "00001740-n") or sense id (e.g. "example%1:09:00::").
    id: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct SearchPrefixParams {
    /// The lemma prefix to search for.
    prefix: String,
    /// Maximum number of matches to return. Unbounded if omitted.
    #[serde(default)]
    max_results: Option<usize>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct ApplyAutomatonParams {
    /// The batch of automaton actions to apply, in order.
    actions: Vec<Action>,
    /// If true, apply against an in-memory clone and discard it instead of mutating (or
    /// saving) the real wordnet - reports whether the batch would succeed and any
    /// validation errors it would introduce. Defaults to false.
    #[serde(default)]
    dry_run: bool,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct SaveParams {
    /// Save even if the wordnet currently has validation errors. Defaults to false.
    #[serde(default)]
    force: bool,
}

#[derive(Serialize)]
struct ValidateReport {
    count: usize,
    errors: Vec<String>,
}

#[derive(Serialize)]
struct DryRunReport {
    would_succeed: bool,
    apply_error: Option<String>,
    last_synset_id: Option<String>,
    validation_errors: Vec<String>,
}

#[derive(Serialize)]
struct ApplyReport {
    applied: bool,
    saved: bool,
    /// True if `saved` is false only because the on-disk wordnet has changed since this
    /// server last loaded/saved it - call `reload` to pick up that change, then reapply.
    stale: bool,
    last_synset_id: Option<String>,
    validation_errors: Vec<String>,
    change_summaries: Vec<String>,
}

#[derive(Serialize)]
struct SaveReport {
    saved: bool,
    /// True if `saved` is false only because the on-disk wordnet has changed since this
    /// server last loaded/saved it - call `reload` to pick up that change, then retry (or
    /// pass `force: true` to save over it anyway).
    stale: bool,
    validation_errors: Vec<String>,
}

#[derive(Serialize)]
struct ReloadReport {
    path: String,
    /// True if this session had unsaved in-memory changes that `reload` just discarded in
    /// favor of what was on disk.
    discarded_unsaved_changes: bool,
}

fn validation_errors_of(wn: &LexiconHashMapBackend) -> Result<Vec<String>, String> {
    let mut progress = NullProgress;
    Ok(ewe_lib::validate::validate(wn, &mut progress)
        .map_err(|e| e.to_string())?
        .iter()
        .map(|e| e.to_string())
        .collect())
}

/// Whether the wordnet on disk at `state.path` has changed since this server last
/// loaded/saved it - `false` if that can't be determined (see `source_mtime`), so an
/// unreadable path degrades to "trust the in-memory copy" rather than locking out every
/// future save.
fn is_stale(state: &ServerState) -> bool {
    match state.loaded_mtime {
        Some(loaded) => source_mtime(&state.path).is_some_and(|current| current > loaded),
        None => false,
    }
}

/// Save `state.wn` to `state.path` unless validation errors or on-disk staleness block it
/// (in which case `force` overrides both). Shared by the `save` tool and `apply_automaton`'s
/// auto-save so the two can't drift out of sync on what "safe to save" means. Returns
/// `(saved, validation_errors, stale)`.
fn perform_save(state: &mut ServerState, force: bool) -> Result<(bool, Vec<String>, bool), String> {
    let validation_errors = validation_errors_of(&state.wn)?;
    let stale = is_stale(state);
    let saved = if (validation_errors.is_empty() && !stale) || force {
        let mut save_progress = NullProgress;
        state
            .wn
            .save(&state.path, &mut save_progress)
            .map_err(|e| e.to_string())?;
        state.loaded_mtime = source_mtime(&state.path);
        true
    } else {
        false
    };
    Ok((saved, validation_errors, stale && !saved))
}

#[tool_router(server_handler)]
impl EweMcpServer {
    #[tool(
        description = "Look up a word (lemma) and return every synset/sense it belongs to as JSON."
    )]
    fn lookup_word(
        &self,
        Parameters(LookupWordParams {
            word,
            ignore_case,
            sense_ids,
        }): Parameters<LookupWordParams>,
    ) -> Result<String, String> {
        let state = self.state.lock().unwrap();
        let entries = if ignore_case {
            state.wn.entry_by_lemma_ignore_case(&word)
        } else {
            state.wn.entry_by_lemma(&word)
        }
        .map_err(|e| e.to_string())?;

        let mut results = Vec::new();
        for entry in entries {
            for sense in entry.sense.iter() {
                let synset = state
                    .wn
                    .synset_by_id(&sense.synset)
                    .map_err(|e| e.to_string())?
                    .ok_or_else(|| format!("Synset {} not found", sense.synset.as_str()))?;
                results.push(LookupResult {
                    synset_id: sense.synset.as_str().to_string(),
                    synset: synset.into_owned(),
                    sense_id: if sense_ids {
                        Some(sense.id.as_str().to_string())
                    } else {
                        None
                    },
                });
            }
        }
        serde_json::to_string(&results).map_err(|e| e.to_string())
    }

    #[tool(description = "Look up a synset or sense by its id and return it as JSON.")]
    fn lookup_id(
        &self,
        Parameters(LookupIdParams { id }): Parameters<LookupIdParams>,
    ) -> Result<String, String> {
        let state = self.state.lock().unwrap();

        if let Some(synset) = state
            .wn
            .synset_by_id(&SynsetId::new(&id))
            .map_err(|e| e.to_string())?
        {
            let result = LookupResult {
                synset_id: id,
                synset: synset.into_owned(),
                sense_id: None,
            };
            return serde_json::to_string(&result).map_err(|e| e.to_string());
        }

        if let Some((_, _, sense)) = state
            .wn
            .get_sense_by_id(&SenseId::new(id.clone()))
            .map_err(|e| e.to_string())?
        {
            let synset = state
                .wn
                .synset_by_id(&sense.synset)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Synset {} not found", sense.synset.as_str()))?;
            let result = LookupResult {
                synset_id: sense.synset.as_str().to_string(),
                synset: synset.into_owned(),
                sense_id: Some(sense.id.as_str().to_string()),
            };
            return serde_json::to_string(&result).map_err(|e| e.to_string());
        }

        Err(format!("No synset or sense found for '{}'", id))
    }

    #[tool(
        description = "Find lemmas starting with the given prefix (for autocomplete-style lookups)."
    )]
    fn search_prefix(
        &self,
        Parameters(SearchPrefixParams {
            prefix,
            max_results,
        }): Parameters<SearchPrefixParams>,
    ) -> Result<String, String> {
        let state = self.state.lock().unwrap();
        let matches = state
            .wn
            .lemma_by_prefix(&prefix, max_results)
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&matches).map_err(|e| e.to_string())
    }

    #[tool(description = "Run full validation over the loaded wordnet and return any errors found.")]
    fn validate(&self) -> Result<String, String> {
        let state = self.state.lock().unwrap();
        let errors = validation_errors_of(&state.wn)?;
        let report = ValidateReport {
            count: errors.len(),
            errors,
        };
        serde_json::to_string(&report).map_err(|e| e.to_string())
    }

    #[tool(
        description = "Apply a batch of automaton actions (add/delete/change synsets, entries, \
        relations, examples, ...) to the loaded wordnet. Rejects batches containing a `validate` \
        action - call the `validate` tool instead. On success (with dry_run false), saves \
        automatically if the result validates cleanly; otherwise leaves the change applied in \
        memory but unsaved and reports the validation errors, letting the caller fix-and-reapply \
        or call `save` with force. With dry_run true, applies against an in-memory clone and \
        discards it, reporting whether it would succeed and any validation errors it would \
        introduce, without touching the real wordnet."
    )]
    fn apply_automaton(
        &self,
        Parameters(ApplyAutomatonParams { actions, dry_run }): Parameters<ApplyAutomatonParams>,
    ) -> Result<String, String> {
        if actions.iter().any(|a| matches!(a, Action::Validate)) {
            return Err(
                "Action batches may not contain a `validate` action - call the `validate` tool instead."
                    .to_string(),
            );
        }

        let mut guard = self.state.lock().unwrap();
        // Deref once up front: `&mut guard.wn`/`&mut guard.changes` below borrow disjoint
        // fields of `*guard` directly, which the borrow checker can split - going through
        // `MutexGuard`'s `DerefMut` on each access instead would look like one call
        // borrowing the whole guard twice.
        let state = &mut *guard;

        if dry_run {
            let mut scratch_wn = state.wn.clone();
            let mut scratch_changes = ChangeList::new();
            let (would_succeed, apply_error, last_synset_id) =
                match ewe_lib::automaton::apply_automaton(actions, &mut scratch_wn, &mut scratch_changes)
                {
                    Ok((last_synset_id, _)) => (true, None, last_synset_id),
                    Err(e) => (false, Some(e), None),
                };
            let validation_errors = validation_errors_of(&scratch_wn).unwrap_or_default();
            let report = DryRunReport {
                would_succeed,
                apply_error,
                last_synset_id: last_synset_id.map(|id| id.as_str().to_string()),
                validation_errors,
            };
            return serde_json::to_string(&report).map_err(|e| e.to_string());
        }

        let change_summaries: Vec<String> = actions.iter().map(|a| a.summary()).collect();
        let (last_synset_id, _) = ewe_lib::automaton::apply_automaton(
            actions,
            &mut state.wn,
            &mut state.changes,
        )?;
        let last_synset_id = last_synset_id.map(|id| id.as_str().to_string());

        let (saved, validation_errors, stale) = perform_save(state, false)?;

        let report = ApplyReport {
            applied: true,
            saved,
            stale,
            last_synset_id,
            validation_errors,
            change_summaries,
        };
        serde_json::to_string(&report).map_err(|e| e.to_string())
    }

    #[tool(
        description = "Persist any pending in-memory changes to disk. Skips saving (returning the \
        validation errors instead) unless the wordnet currently validates cleanly or `force` is \
        set. Also skips saving - reporting `stale: true` - if the wordnet files on disk have \
        changed since this server last loaded/saved them (e.g. a `git checkout`/branch switch/ \
        hand edit made outside this MCP session); call `reload` first to pick up that change, or \
        pass `force: true` to save over it anyway."
    )]
    fn save(&self, Parameters(SaveParams { force }): Parameters<SaveParams>) -> Result<String, String> {
        let mut guard = self.state.lock().unwrap();
        let (saved, validation_errors, stale) = perform_save(&mut guard, force)?;
        let report = SaveReport {
            saved,
            stale,
            validation_errors,
        };
        serde_json::to_string(&report).map_err(|e| e.to_string())
    }

    #[tool(
        description = "Reload the wordnet from disk, replacing the in-memory copy with whatever \
        is currently at the server's wordnet path. Use this after an out-of-band change to the \
        wordnet files made outside this MCP session (a `git checkout`, branch switch, or hand \
        edit) - `save`/`apply_automaton` refuse to write over such a change (reporting \
        `stale: true`) until this is called. Discards any pending in-memory changes this \
        session made but never saved; `discarded_unsaved_changes` in the result says whether \
        that happened."
    )]
    fn reload(&self) -> Result<String, String> {
        let mut guard = self.state.lock().unwrap();
        let mut progress = NullProgress;
        let wn = LexiconHashMapBackend::new()
            .load(&guard.path, &mut progress)
            .map_err(|e| e.to_string())?;
        let discarded_unsaved_changes = guard.changes.changed();
        guard.wn = wn;
        guard.changes = ChangeList::new();
        guard.loaded_mtime = source_mtime(&guard.path);
        let report = ReloadReport {
            path: guard.path.clone(),
            discarded_unsaved_changes,
        };
        serde_json::to_string(&report).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ewe_lib::wordnet::PosKey;

    fn test_server(path: String) -> EweMcpServer {
        let mut wn = LexiconHashMapBackend::new();
        // `add_lexfile` is cfg(test)-only inside `ewe_lib` itself, so it's not visible here -
        // registering "noun.animal" as a real lexfile (as a side effect of adding a seed
        // synset to it) is the only way to do this through `ewe_lib`'s public API.
        ewe_lib::change_manager::add_synset(
            &mut wn,
            "seed synset".to_string(),
            "noun.animal".to_string(),
            PosKey::new("n".to_string()),
            None,
            &mut ChangeList::new(),
        )
        .unwrap();
        let loaded_mtime = source_mtime(&path);
        EweMcpServer {
            state: Arc::new(Mutex::new(ServerState {
                path,
                wn,
                changes: ChangeList::new(),
                loaded_mtime,
            })),
        }
    }

    fn add_synset_action(lemma: &str) -> Vec<Action> {
        vec![Action::AddSynset {
            definition: "a test synset".to_string(),
            lexfile: "noun.animal".to_string(),
            pos: Some(PosKey::new("n".to_string())),
            lemmas: vec![lemma.to_string()],
            subcats: Vec::new(),
        }]
    }

    #[test]
    fn dry_run_leaves_state_untouched() {
        // Path is never touched by a dry run - only real applies save.
        let server = test_server("/nonexistent/does-not-matter".to_string());

        server
            .apply_automaton(Parameters(ApplyAutomatonParams {
                actions: add_synset_action("dryruncat"),
                dry_run: true,
            }))
            .unwrap();

        let looked_up = server
            .lookup_word(Parameters(LookupWordParams {
                word: "dryruncat".to_string(),
                ignore_case: false,
                sense_ids: false,
            }))
            .unwrap();
        assert_eq!(looked_up, "[]", "a dry run must not mutate the real lexicon");
    }

    #[test]
    fn validation_errors_block_save_but_force_overrides() {
        let dir = std::env::temp_dir().join(format!(
            "ewe_mcp_test_{}_{}",
            std::process::id(),
            "validation_errors_block_save"
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.to_string_lossy().to_string();
        let server = test_server(path);

        // A noun synset with no hypernym fails validate(), so this apply should not save.
        let apply_result = server
            .apply_automaton(Parameters(ApplyAutomatonParams {
                actions: add_synset_action("nohypernymcat"),
                dry_run: false,
            }))
            .unwrap();
        assert!(apply_result.contains("\"saved\":false"), "{}", apply_result);
        assert!(
            std::fs::read_dir(&dir).unwrap().next().is_none(),
            "a validation-failing apply must not write anything to disk"
        );

        let save_result = server
            .save(Parameters(SaveParams { force: true }))
            .unwrap();
        assert!(save_result.contains("\"saved\":true"), "{}", save_result);
        assert!(
            std::fs::read_dir(&dir).unwrap().next().is_some(),
            "save(force: true) should write the pending change to disk despite validation errors"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// A fresh, uniquely-named `<temp>/<name>/src/yaml` directory to load/save a test
    /// wordnet against - nested under its own unique parent (rather than a bare
    /// `<temp>/<name>`) so that `Lexicon::save`'s sibling `../deprecations.csv` write
    /// lands in a directory private to this test, not the shared `std::env::temp_dir()`
    /// every other test (and concurrent test run) also writes into. Staleness detection
    /// stats that sibling file too, so without this isolation an unrelated concurrent
    /// test's save could make this test's directory look stale (or vice versa).
    fn isolated_test_wn_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir()
            .join(format!("ewe_mcp_test_{}_{}", std::process::id(), name))
            .join("src/yaml");
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn stale_source_blocks_save_unless_forced() {
        let dir = isolated_test_wn_dir("stale_source_blocks_save");
        let path = dir.to_string_lossy().to_string();

        // A fresh `LexiconHashMapBackend` (no lexfiles at all) validates cleanly, so any
        // block on save below is attributable to staleness alone, not validation errors.
        let mut state = ServerState {
            path: path.clone(),
            wn: LexiconHashMapBackend::new(),
            changes: ChangeList::new(),
            loaded_mtime: source_mtime(&path),
        };
        assert!(
            !is_stale(&state),
            "nothing has touched the directory since loaded_mtime was captured"
        );

        // Simulate an out-of-band change made outside this session (e.g. `git checkout`).
        std::fs::write(dir.join("entries-z.yaml"), "{}\n").unwrap();
        assert!(
            is_stale(&state),
            "a file written after loaded_mtime must be detected as stale"
        );

        let (saved, validation_errors, stale) = perform_save(&mut state, false).unwrap();
        assert!(!saved, "save must refuse to write over an unseen on-disk change");
        assert!(validation_errors.is_empty());
        assert!(stale, "the report must say staleness is why save was skipped");

        let (saved, _validation_errors, stale) = perform_save(&mut state, true).unwrap();
        assert!(saved, "force must override staleness (same escape hatch as validation errors)");
        assert!(!stale, "a successful save is never reported as stale");

        std::fs::remove_dir_all(dir.parent().unwrap().parent().unwrap()).ok();
    }

    #[test]
    fn reload_picks_up_disk_and_reports_discarded_changes() {
        let dir = isolated_test_wn_dir("reload_picks_up_disk");
        let path = dir.to_string_lossy().to_string();
        let server = test_server(path.clone());

        // A real apply (even one left unsaved by a validation error, as this one is - see
        // `validation_errors_block_save_but_force_overrides`) marks the session's `changes`
        // dirty, unlike the synset seeded directly into `wn` by `test_server`.
        server
            .apply_automaton(Parameters(ApplyAutomatonParams {
                actions: add_synset_action("reloadtestcat"),
                dry_run: false,
            }))
            .unwrap();

        // Simulate an out-of-band change made outside this session.
        std::fs::write(dir.join("entries-z.yaml"), "{}\n").unwrap();
        let save_result = server
            .save(Parameters(SaveParams { force: false }))
            .unwrap();
        assert!(save_result.contains("\"stale\":true"), "{}", save_result);
        assert!(save_result.contains("\"saved\":false"), "{}", save_result);

        let reload_result = server.reload().unwrap();
        assert!(
            reload_result.contains("\"discarded_unsaved_changes\":true"),
            "{}",
            reload_result
        );

        // Staleness is cleared by reload - a clean empty lexicon (which is what `dir` now
        // holds on disk) now saves without needing `force`.
        let save_result = server
            .save(Parameters(SaveParams { force: false }))
            .unwrap();
        assert!(save_result.contains("\"stale\":false"), "{}", save_result);
        assert!(save_result.contains("\"saved\":true"), "{}", save_result);

        std::fs::remove_dir_all(dir.parent().unwrap().parent().unwrap()).ok();
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let (path, wn) = locate_wordnet(cli.wordnet).map_err(|e| anyhow::anyhow!(e))?;
    let loaded_mtime = source_mtime(&path);

    let server = EweMcpServer {
        state: Arc::new(Mutex::new(ServerState {
            path,
            wn,
            changes: ChangeList::new(),
            loaded_mtime,
        })),
    };

    let service = server.serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;
    Ok(())
}
