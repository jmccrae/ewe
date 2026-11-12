//! Guards in-app navigation (and, on web, leaving the page) while a synset edit pane has
//! drafts that differ from the saved synset.
//!
//! This is distinct from the `dirty` context in `unsaved_changes`, which tracks edits that were
//! already *applied* but not yet saved to YAML (those survive navigation and reloads, since
//! they live in the database). Here we track drafts that live only in a `Synset` component's
//! local signals and would be lost.
use crate::Route;
use dioxus::prelude::*;
use std::collections::HashSet;

/// Record that the edit pane of `id` does (or no longer does) have unsaved drafts.
pub(crate) fn set_open(open: &mut HashSet<String>, id: &str, has_changes: bool) {
    if has_changes {
        open.insert(id.to_string());
    } else {
        open.remove(id);
    }
}

/// App-wide record of which synsets have an edit pane with unsaved drafts, plus a navigation
/// held back while the user confirms discarding them. Provided once by `WNLayout`.
#[derive(Clone, Copy)]
pub struct EditGuard {
    open: Signal<HashSet<String>>,
    pending: Signal<Option<Route>>,
}

pub fn provide_edit_guard() -> EditGuard {
    use_context_provider(|| EditGuard {
        open: Signal::new(HashSet::new()),
        pending: Signal::new(None),
    })
}

pub fn use_edit_guard() -> EditGuard {
    use_context::<EditGuard>()
}

impl EditGuard {
    /// Whether any edit pane currently has unsaved drafts
    pub fn has_open_edits(&self) -> bool {
        !(self.open)().is_empty()
    }

    /// Navigate to `to`, unless an edit pane has unsaved drafts, in which case the navigation
    /// is held until the user confirms in `DiscardEditsModal`.
    pub fn navigate(&mut self, to: Route) {
        if self.has_open_edits() {
            self.pending.set(Some(to));
        } else {
            navigator().push(to);
        }
    }
}

/// A `Link` that asks for confirmation first if any edit pane has unsaved drafts. Modifier and
/// non-primary clicks (open in a new tab) are left to the browser, as with a plain `Link`.
#[derive(Clone, PartialEq, Props)]
pub struct GuardedLinkProps {
    to: Route,
    #[props(default)]
    class: Option<String>,
    #[props(default)]
    id: Option<String>,
    children: Element,
}

#[component]
pub fn GuardedLink(props: GuardedLinkProps) -> Element {
    let mut guard = use_edit_guard();
    let to = props.to.clone();
    rsx! {
        Link {
            to: props.to.clone(),
            class: props.class.clone(),
            id: props.id.clone(),
            onclick_only: true,
            onclick: move |_| guard.navigate(to.clone()),
            {props.children}
        }
    }
}

/// Registers a synset's edit pane with the guard for as long as it is mounted. Rendered only
/// while the pane is open, so unmounting (accept, reject, navigation) unregisters it.
#[derive(Clone, PartialEq, Props)]
pub struct EditGuardRegistrarProps {
    synset_id: String,
    changed: bool,
}

#[component]
pub fn EditGuardRegistrar(props: EditGuardRegistrarProps) -> Element {
    let mut guard = use_edit_guard();
    let id = props.synset_id.clone();
    let changed = props.changed;
    use_effect(use_reactive!(|id, changed| {
        set_open(&mut guard.open.write(), &id, changed);
    }));
    let id = props.synset_id.clone();
    use_drop(move || {
        set_open(&mut guard.open.write(), &id, false);
    });
    rsx! {}
}

/// Confirmation shown when a navigation was held back by `EditGuard::navigate`. Mounted once
/// in `WNLayout`.
#[component]
pub fn DiscardEditsModal() -> Element {
    let mut guard = use_edit_guard();
    let Some(target) = (guard.pending)() else {
        return rsx! {};
    };
    rsx! {
        div {
            class: "modal-overlay",
            onclick: move |_| guard.pending.set(None),
            div {
                class: "modal-content",
                onclick: move |e| e.stop_propagation(),
                div {
                    class: "modal-header",
                    h3 { "Discard unsaved edits?" }
                    button {
                        class: "modal-close",
                        r#type: "button",
                        title: "Close",
                        onclick: move |_| guard.pending.set(None),
                        "×"
                    }
                }
                p {
                    "A synset editor is open with changes that haven't been accepted yet. "
                    "Leaving this page will discard them."
                }
                div {
                    class: "modal-actions",
                    button {
                        class: "edit-cancel",
                        r#type: "button",
                        onclick: move |_| guard.pending.set(None),
                        "Keep editing"
                    }
                    button {
                        class: "edit-delete",
                        r#type: "button",
                        onclick: move |_| {
                            guard.pending.set(None);
                            guard.open.write().clear();
                            navigator().push(target.clone());
                        },
                        "Discard and leave"
                    }
                }
            }
        }
    }
}

/// On web, ask the browser to show its native "Leave site?" prompt (reload, tab close,
/// external links) while any edit pane has unsaved drafts. A no-op on desktop, whose webview
/// has no equivalent and whose window close can't be intercepted.
#[component]
pub fn BeforeUnloadGuard() -> Element {
    let guard = use_edit_guard();
    let active = guard.has_open_edits();
    use_effect(use_reactive!(|active| {
        if cfg!(feature = "desktop") {
            return;
        }
        let js = if active {
            "window.onbeforeunload = function (e) { e.preventDefault(); e.returnValue = ''; return ''; };"
        } else {
            "window.onbeforeunload = null;"
        };
        document::eval(js);
    }));
    rsx! {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_open_tracks_changed_panes_independently() {
        let mut open = HashSet::new();
        set_open(&mut open, "a", true);
        set_open(&mut open, "b", true);
        assert_eq!(open.len(), 2);
        set_open(&mut open, "a", false);
        assert!(open.contains("b") && !open.contains("a"));
        // unregistering an id that was never registered is harmless
        set_open(&mut open, "zzz", false);
        set_open(&mut open, "b", false);
        assert!(open.is_empty());
    }
}
