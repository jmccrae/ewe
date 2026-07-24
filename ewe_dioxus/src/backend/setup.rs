//! Detects whether the app has a working Wordnet database, and (on desktop) lets the user
//! configure one interactively instead of hand-editing `settings.toml`.
//!
//! Every `#[get]`/`#[post]` function in this file (and the rest of `backend/`) is written as
//! `#[cfg_attr(not(feature = "desktop"), get("..."))]` rather than a plain `#[get("...")]`. A
//! `desktop` build is a single self-contained binary with no separate server process at all -
//! `cfg_attr` skips the macro there entirely, leaving a plain `async fn` that view/component code
//! (already only ever calling these isomorphically) invokes directly, in-process. `web`/`server`
//! keep going through the macro as normal, and Dioxus's own internal `#[cfg(feature = "server")]`
//! split inside its expansion still correctly tells the WASM client's RPC stub apart from the
//! real server-side handler between those two - `desktop` is the only case this crate has to
//! handle itself.

use dioxus::prelude::*;
#[cfg(any(feature = "server", feature = "desktop"))]
use ewe_lib::wordnet::ReDBLexicon;
use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use thiserror::Error;

/// Whether the app has a usable lexicon, and enough detail about *why not* (if not) for the
/// frontend to show a specific message - see `components::setup_needed`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetupStatus {
    pub configured: bool,
    pub settings_toml_exists: bool,
    pub wordnet_source: Option<String>,
}

#[cfg_attr(not(feature = "desktop"), get("/api/setup_status"))]
pub async fn get_setup_status() -> Result<SetupStatus> {
    let settings = crate::db::read_settings();
    Ok(SetupStatus {
        configured: crate::db::read_lexicon().is_ok(),
        settings_toml_exists: std::path::Path::new("settings.toml").exists(),
        wordnet_source: settings.wordnet_source.clone(),
    })
}

#[derive(Error, Debug)]
#[allow(dead_code)]
enum EweSetupError {
    #[error("'{0}' is not a folder")]
    NotADirectory(String),
    #[error("No 'settings.toml' found in '{0}' - pick the folder containing your Wordnet project's settings.toml (the one with 'wordnet_source', 'database', etc.)")]
    NoSettingsToml(String),
    #[error("A folder is already being loaded - wait for it to finish before picking another")]
    AlreadyRunning,
    #[error("{0}")]
    Failed(String),
}

/// A snapshot of the in-progress load kicked off by `configure_wordnet_source`, polled by the
/// desktop setup screen's progress bar (`components::setup_needed`) via `get_setup_progress`.
/// `total == 0` means "running, but no file count yet" (e.g. the very first moment after
/// `configure_wordnet_source` returns, or a fast path - like re-opening an already-up-to-date
/// database - that never reports incremental progress at all).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetupProgress {
    pub current: u64,
    pub total: u64,
}

/// The currently running load's progress, or `None` if nothing is running - either because
/// nothing has been configured yet, or because the last configure attempt has already finished
/// (check `get_setup_error` to tell success from failure).
#[cfg(any(feature = "server", feature = "desktop"))]
static SETUP_PROGRESS: std::sync::RwLock<Option<SetupProgress>> = std::sync::RwLock::new(None);

/// The error from the most recently *finished* configure attempt, if it failed. Cleared at the
/// start of every new `configure_wordnet_source` call, so it only ever reflects the latest
/// attempt.
#[cfg(any(feature = "server", feature = "desktop"))]
static SETUP_ERROR: std::sync::RwLock<Option<String>> = std::sync::RwLock::new(None);

#[cfg(any(feature = "server", feature = "desktop"))]
struct SharedSetupProgress {
    current: u64,
}

#[cfg(any(feature = "server", feature = "desktop"))]
impl SharedSetupProgress {
    fn new() -> Self {
        Self { current: 0 }
    }
}

#[cfg(any(feature = "server", feature = "desktop"))]
impl ewe_lib::progress::Progress for SharedSetupProgress {
    fn start(&mut self, total: u64) {
        self.current = 0;
        *SETUP_PROGRESS.write().unwrap() = Some(SetupProgress { current: 0, total });
    }
    fn inc(&mut self, amount: u64) {
        self.current += amount;
        if let Some(status) = SETUP_PROGRESS.write().unwrap().as_mut() {
            status.current = self.current;
        }
    }
    // Deliberately does *not* clear `SETUP_PROGRESS` - `Lexicon::load` calls this the moment
    // *its own* file-reading loop is done, which is well before `run_configure` is actually
    // finished (it still has to build the corpus and hot-swap everything in afterward). Clearing
    // `SETUP_PROGRESS` here previously made the client conclude the whole configure was done and
    // reload the page early - while the corpus rebuild (and the swap that makes `LEXICON` itself
    // point at the freshly-loaded data) was still running in the background, so the reloaded page
    // still showed "not configured". `SETUP_PROGRESS` is only ever cleared by
    // `configure_wordnet_source`'s spawned task once `run_configure` as a whole has returned -
    // until then it just stays frozen at its last known `current`/`total` (accurate: there's
    // nothing more granular to report for the corpus/swap steps, but the operation is not done).
    fn finish(&mut self) {}
    fn set_percent_mode(&mut self, _percent_mode: bool) {}
}

/// Polled by the desktop setup screen while a `configure_wordnet_source` load is running.
/// `None` means nothing is currently running.
#[cfg(feature = "edit")]
#[cfg_attr(not(feature = "desktop"), get("/api/setup/progress"))]
pub async fn get_setup_progress() -> Result<Option<SetupProgress>> {
    Ok(SETUP_PROGRESS.read().unwrap().clone())
}

/// The error from the most recently finished `configure_wordnet_source` call, if it failed.
/// Meant to be polled once `get_setup_progress` returns `None` (i.e. the load has finished) to
/// tell success from failure, since that request itself returns almost immediately - see
/// `configure_wordnet_source`'s doc comment for why.
#[cfg(feature = "edit")]
#[cfg_attr(not(feature = "desktop"), get("/api/setup/error"))]
pub async fn get_setup_error() -> Result<Option<String>> {
    Ok(SETUP_ERROR.read().unwrap().clone())
}

/// Validates that `path` is a Wordnet project folder (contains a `settings.toml`) and, if so,
/// kicks off rebuilding the lexicon (and, best-effort, the corpus) from it in the background,
/// returning almost immediately rather than waiting for that load to finish.
///
/// Loading a large Wordnet can take upwards of a minute, and a single HTTP request surviving
/// that whole time - across a native webview's HTTP client, any keep-alive/proxy timeouts, and
/// so on - is fragile in a way that's hard to fully rule out. Backgrounding the actual load and
/// having the client poll [`get_setup_progress`]/[`get_setup_error`] instead (see
/// `components::setup_needed`'s desktop button) means the worst case if something *does* go
/// wrong with a long-lived connection is a poll failing and getting retried, not the whole load
/// silently going unobserved by the client even though the server finished it - which is what
/// was happening before this was backgrounded: the load completed and hot-swapped in
/// successfully, but the awaited request never resolved on the client to trigger the page
/// reload, so the new state stayed invisible until the app was restarted.
///
/// `path` must be a folder containing a `settings.toml` (see e.g.
/// `~/p/globalwordnet/english-wordnet/settings.toml` for a real example) - unlike the app's own
/// bootstrap `settings.toml`, that file's path-valued fields (`wordnet_source`, `database`,
/// `logo`, etc.) are resolved relative to *its own* folder rather than the server's working
/// directory (see `EweSettings::load`), so the same project folder works no matter where it's
/// picked from or where the server process happens to be running.
///
/// Deliberately does *not* write anything to the server's own working directory (in particular,
/// it never touches its local `settings.toml`) - only the in-memory `SETTINGS`/`LEXICON`/`CORPUS`
/// are hot-swapped, so relaunching the app starts unconfigured again and needs the folder picked
/// again. Two reasons: writing into the project's own working directory while `dx serve` is
/// watching it for changes (as it does during desktop development) risks triggering a hot-reload
/// rebuild/restart mid-load, which can race the still-running background load against the freshly
/// restarted process's own eager corpus/lexicon initialization and corrupt the on-disk database
/// out from under it; and more generally, a project folder's own `settings.toml` is that project's
/// configuration, not the app's - overwriting the app's local copy with it on every pick isn't
/// something this flow should be doing on the user's behalf.
///
/// Gated on `edit` (not `desktop`): letting arbitrary HTTP clients repoint the server at any
/// local filesystem path would be a real vulnerability on a public deployment - `edit` is what
/// actually captures "trusted, single-user" here (`desktop` already implies it, and the public,
/// read-only en-word.net web build never compiles with `edit` on).
///
/// A `desktop` build never actually sends this over HTTP at all: `#[cfg_attr]` below skips the
/// `#[post(...)]` macro there entirely, so `components::setup_needed`'s desktop button calls this
/// as a plain in-process async fn, same as every other endpoint in `backend/` - see this file's
/// module doc comment.
#[cfg(feature = "edit")]
#[cfg_attr(not(feature = "desktop"), post("/api/setup/configure_wordnet_source"))]
pub async fn configure_wordnet_source(path: String) -> Result<()> {
    // Belt-and-braces: force `LEXICON`/`CORPUS`/`SETTINGS` (all `Lazy`s) to have already run
    // their initializer, if they haven't already, *before* `run_configure` reaches its final
    // swap. `main.rs`'s `App()` already does this eagerly at startup on both `server` and
    // `desktop` builds, so in practice this is always a no-op by the time a user can reach this
    // button - but if it somehow weren't, the alternative is bad: the *first* ever call to
    // `CORPUS.get()` could end up being `run_configure`'s own
    // `*crate::CORPUS.get().write().unwrap() = corpus;` line - which would trigger `CORPUS`'s
    // initializer right then, rebuilding a corpus from whatever *old* settings were current at
    // that moment (not the ones just configured, since `SETTINGS` itself is swapped after
    // `CORPUS`), immediately followed by the assignment silently discarding that wasted result.
    // Confirmed live (before `desktop` embedded this state in-process, back when the desktop
    // webview's shell never triggered a real `App()` render at all): this produced a spurious
    // "Failed to open corpus: Layer ... does not exist" from a completely unrelated, empty
    // corpus - identical wording to a real corpus failure, just from a different, throwaway
    // `open_corpus` call a moment after the real one succeeded.
    let _ = crate::LEXICON.get();
    let _ = crate::CORPUS.get();
    let _ = crate::SETTINGS.get();

    let root = std::path::Path::new(&path);
    if !root.is_dir() {
        return Err(EweSetupError::NotADirectory(path).into());
    }
    let settings_path = root.join("settings.toml");
    if !settings_path.is_file() {
        return Err(EweSetupError::NoSettingsToml(path).into());
    }

    let new_settings = crate::settings::EweSettings::load(&settings_path.to_string_lossy())
        .map_err(|e| EweSetupError::Failed(e.to_string()))?;

    {
        // Checked and set under a single write-lock acquisition (one critical section, not a
        // separate check then a separate set) so two overlapping calls - e.g. a double-click
        // before the button's `disabled` state visually takes effect - can't both see "nothing
        // running" and both proceed. Without this, two concurrent `run_configure`s would each
        // independently rebuild the *same* corpus/lexicon database files, which is exactly the
        // kind of file-level race ("Layer ... does not exist" from a corpus rebuild reading a
        // file another rebuild is concurrently deleting/rewriting) this project has been hitting.
        let mut progress = SETUP_PROGRESS.write().unwrap();
        if progress.is_some() {
            return Err(EweSetupError::AlreadyRunning.into());
        }
        // Set synchronously (before the request returns) rather than as the first thing the
        // spawned task below does, so a client that starts polling `get_setup_progress` the
        // moment this request resolves can never observe `None` and mistake "hasn't started yet"
        // for "already finished" - see `SetupProgress`'s doc comment on what `total == 0` means.
        *progress = Some(SetupProgress { current: 0, total: 0 });
    }
    *SETUP_ERROR.write().unwrap() = None;

    tokio::spawn(async move {
        if let Err(e) = run_configure(new_settings).await {
            *SETUP_ERROR.write().unwrap() = Some(e);
        }
        *SETUP_PROGRESS.write().unwrap() = None;
    });

    Ok(())
}

/// The actual (potentially slow) work `configure_wordnet_source` backgrounds: rebuild the
/// lexicon, best-effort rebuild the corpus, then hot-swap all three into the running server -
/// see `configure_wordnet_source`'s doc comment for why this deliberately doesn't persist
/// anything to disk itself.
#[cfg(any(feature = "server", feature = "desktop"))]
async fn run_configure(new_settings: crate::settings::EweSettings) -> Result<(), String> {
    let lexicon = {
        let settings_for_load = new_settings.clone();
        // The blocking closure converts its error to a `String` before crossing back over the
        // `spawn_blocking` boundary - `open_lexicon_with_progress`'s own `Box<dyn Error>` isn't
        // `Send`, which `spawn_blocking`'s return type must be.
        tokio::task::spawn_blocking(move || -> Result<ReDBLexicon, String> {
            let mut progress = SharedSetupProgress::new();
            crate::db::open_lexicon_with_progress(&settings_for_load, &mut progress)
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| format!("Load task panicked: {e}"))??
    };

    // The corpus is supplementary (only used to show usage examples), so - matching how it's
    // treated at startup in `main.rs` - a failure to open it is logged but doesn't block
    // configuring the rest of the project. Runs on a blocking thread for the same reason the
    // lexicon load above does: rebuilding from a large corpus source (e.g. all of SemCor) is a
    // synchronous, CPU/IO-bound scan that would otherwise tie up an async worker thread - and,
    // worse on a single-threaded runtime, stall every other request (including this same load's
    // own progress polls) until it finished.
    let corpus = if crate::db::is_unconfigured_corpus(&new_settings) {
        // This project just doesn't set up a corpus (a valid choice - it's entirely optional) -
        // not a failure, so nothing to attempt or log here.
        None
    } else {
        let settings_for_corpus = new_settings.clone();
        let result = tokio::task::spawn_blocking(move || {
            crate::db::open_corpus(&settings_for_corpus).map_err(|e| e.to_string())
        })
        .await;
        match result {
            Ok(Ok(corpus)) => Some(corpus),
            Ok(Err(e)) => {
                eprintln!("Failed to open corpus: {}", e);
                None
            }
            Err(e) => {
                eprintln!("Corpus load task panicked: {e}");
                None
            }
        }
    };

    *crate::LEXICON.get().write().unwrap() = Some(lexicon);
    *crate::CORPUS.get().write().unwrap() = corpus;
    *crate::db::write_settings() = new_settings;

    Ok(())
}
