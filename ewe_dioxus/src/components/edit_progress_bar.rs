#![cfg(feature = "edit")]
use dioxus::prelude::*;

#[derive(Clone, PartialEq, Props)]
pub struct EditProgressBarProps {
    /// Poll `GET /api/edit/progress` and show a bar while true; stops polling (and clears
    /// whatever was shown) as soon as it goes false.
    pub active: bool,
}

/// A live progress bar for whichever save/validate call is currently running server-side (see
/// `backend::edit::{save_lexicon, validate_lexicon, get_progress}`), shown by `ValidateModal` and
/// `UnsavedChangesToast` while their respective request is in flight.
///
/// Polls on a plain timer rather than anything push-based, since a single request/response call
/// (however long it takes to resolve) has no channel back to the client to push updates over -
/// the delay between polls is done via `document::eval` (a JS `setTimeout`) rather than
/// `tokio::time::sleep`/`gloo-timers`, since that works identically on both the web (wasm) and
/// desktop (native, but rendered through the same kind of JS-capable webview) targets without
/// needing a target-specific implementation of either.
#[component]
pub fn EditProgressBar(props: EditProgressBarProps) -> Element {
    use crate::backend::edit::{get_progress, ProgressStatus};

    let mut status = use_signal(|| None::<ProgressStatus>);
    let mut poll_task = use_signal(|| None::<dioxus_core::Task>);

    use_effect(move || {
        if props.active {
            if poll_task.peek().is_none() {
                let handle = spawn(async move {
                    loop {
                        match get_progress().await {
                            Ok(s) => status.set(s),
                            Err(_) => break,
                        }
                        let _ = document::eval("await new Promise(r => setTimeout(r, 300));").await;
                    }
                });
                poll_task.set(Some(handle));
            }
        } else {
            if let Some(task) = poll_task.take() {
                task.cancel();
            }
            status.set(None);
        }
    });

    rsx! {
        if let Some(s) = status() {
            div {
                class: "edit-progress",
                div {
                    class: "edit-progress-label",
                    if s.total > 0 {
                        "{s.operation}… {s.current}/{s.total}"
                    } else {
                        "{s.operation}…"
                    }
                }
                div {
                    class: "edit-progress-track",
                    div {
                        class: "edit-progress-fill",
                        style: "width: {percent(s.current, s.total)}%",
                    }
                }
            }
        }
    }
}

fn percent(current: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        (current as f64 / total as f64 * 100.0).min(100.0)
    }
}
