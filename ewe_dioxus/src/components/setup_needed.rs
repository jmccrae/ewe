use dioxus::prelude::*;
use crate::backend::api::Branding;
use crate::backend::setup::SetupStatus;
use dioxus_fullstack::Loader;

static CSS: Asset = asset!("/assets/styling/setup.css");

/// Shown by `WNLayout` instead of the routed page content whenever `SetupStatus::configured`
/// is false - i.e. `db::open_lexicon` failed at startup and hasn't since been fixed. Web and
/// desktop builds show different remediation ([`SetupNeededDetail`]): web can only point at
/// `settings.toml`, since it has no interactive way to pick a folder; desktop offers a native
/// folder picker that fixes things for the current session without a restart, though it isn't
/// persisted - relaunching needs the folder picked again (see
/// `backend::setup::configure_wordnet_source`). `setup_status` and `branding` are `WNLayout`'s
/// own `use_loader` handles (for `get_setup_status` and `get_branding` respectively), passed
/// down so the desktop flow can `.restart()` both once configuring succeeds, reactively
/// re-rendering with the fresh status *and* the fresh project name/footer - `branding` is fetched
/// at the layout level (wrapping this very screen), so without an explicit restart it would stay
/// showing whatever was current before the project was configured.
#[component]
pub fn SetupNeeded(
    status: SetupStatus,
    setup_status: Loader<SetupStatus>,
    branding: Loader<Branding>,
) -> Element {
    rsx! {
        document::Style { href: CSS }
        div {
            class: "setup-needed",
            div {
                class: "setup-card",
                h2 { "Open Wordnet Folder" }
                SetupNeededDetail { status, setup_status, branding }
            }
        }
    }
}

#[cfg(not(feature = "desktop"))]
#[component]
#[allow(unused_variables)]
fn SetupNeededDetail(
    status: SetupStatus,
    setup_status: Loader<SetupStatus>,
    branding: Loader<Branding>,
) -> Element {
    rsx! {
        if !status.settings_toml_exists {
            p {
                "This deployment doesn't have a Wordnet set up yet - no " code { "settings.toml" }
                " was found in the application's working directory."
            }
            p {
                "Create one there with at least a " code { "wordnet_source" } " path pointing "
                "at your Wordnet's YAML source folder - see "
                code { "ewe_dioxus/english-wordnet-settings.toml" } " for a complete example."
            }
        } else if let Some(source) = &status.wordnet_source {
            p {
                "The configured " code { "wordnet_source" } " folder ('{source}') could not be "
                "loaded."
            }
            p { "Check the path in " code { "settings.toml" } " and restart the server." }
        } else {
            p {
                code { "settings.toml" } " doesn't set " code { "wordnet_source" } " yet, and no "
                "existing database was found."
            }
            p {
                "Add a " code { "wordnet_source" } " path pointing at your Wordnet's YAML "
                "source folder, then restart the server."
            }
        }
    }
}

#[cfg(feature = "desktop")]
#[component]
#[allow(unused_variables)]
fn SetupNeededDetail(
    status: SetupStatus,
    mut setup_status: Loader<SetupStatus>,
    mut branding: Loader<Branding>,
) -> Element {
    use crate::backend::setup::{
        configure_wordnet_source, get_setup_error, get_setup_progress, SetupProgress,
    };

    let mut error = use_signal(|| None::<String>);
    let mut picking = use_signal(|| false);
    let mut progress = use_signal(|| None::<SetupProgress>);

    rsx! {
        p {
            "Open a Wordnet project folder to get started - one containing its own "
            code { "settings.toml" } " that points at the data and assets to load."
        }
        p {
            "This only applies to the current session - it isn't saved, so you'll need to "
            "open the folder again next time you launch the app."
        }
        if let Some(err) = error() {
            p { class: "setup-error", "{err}" }
        }
        if let Some(p) = progress() {
            div {
                class: "edit-progress",
                div {
                    class: "edit-progress-label",
                    if p.total > 0 {
                        "Loading… {p.current}/{p.total}"
                    } else {
                        "Loading…"
                    }
                }
                div {
                    class: "edit-progress-track",
                    div {
                        class: "edit-progress-fill",
                        style: "width: {progress_percent(p.current, p.total)}%",
                    }
                }
            }
        }
        button {
            class: "list-add",
            r#type: "button",
            disabled: picking(),
            onclick: move |_| async move {
                picking.set(true);
                error.set(None);
                progress.set(None);
                let Some(folder) = rfd::AsyncFileDialog::new().pick_folder().await else {
                    picking.set(false);
                    return;
                };
                let path = folder.path().to_string_lossy().to_string();

                // `configure_wordnet_source` only validates the folder and kicks the actual
                // (possibly slow) load off in the background - see its doc comment for why this
                // polls for completion instead of awaiting one long-lived request.
                if let Err(e) = configure_wordnet_source(path).await {
                    error.set(Some(e.to_string()));
                    picking.set(false);
                    return;
                }

                loop {
                    let _ = document::eval("await new Promise(r => setTimeout(r, 300));").await;
                    if let Ok(Some(p)) = get_setup_progress().await {
                        progress.set(Some(p));
                        continue;
                    }
                    // No progress left to report - the background load has finished, either way.
                    progress.set(None);
                    picking.set(false);
                    match get_setup_error().await {
                        Ok(Some(msg)) => {
                            error.set(Some(msg));
                        }
                        _ => {
                            // Restarting `WNLayout`'s `get_setup_status` loader re-fetches it and
                            // reactively re-renders with the fresh (now `configured: true`)
                            // result - unlike a real `window.location.reload()`, this doesn't
                            // depend on the desktop webview supporting/behaving like a normal
                            // browser navigation, which turned out not to be reliable there.
                            // `branding` needs its own restart alongside it - it was already
                            // fetched (with the pre-configure project name/footer) by the time
                            // this screen ever showed, and nothing else would tell it to refetch.
                            setup_status.restart();
                            branding.restart();
                        }
                    }
                    break;
                }
            },
            if picking() { "Loading…" } else { "Open Folder" }
        }
    }
}

#[cfg(feature = "desktop")]
fn progress_percent(current: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        (current as f64 / total as f64 * 100.0).min(100.0)
    }
}
