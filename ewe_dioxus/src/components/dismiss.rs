use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::*;

/// Calls `on_dismiss` as soon as the user mouses down outside the DOM element identified by
/// `container_id`. Meant for dropdown-style popups (search suggestions, the display options
/// panel) that live for the lifetime of the layout they're mounted in - so, unlike
/// `EditProgressBar`'s poll loop (which starts/stops with a prop), the listener here is set up
/// once and left running for the app's lifetime, and simply no-ops on every mousedown while the
/// element it's watching for isn't in the DOM (i.e. while the dropdown is already closed). That
/// sidesteps having to tear the `document` listener back down on close/unmount, which `document::
/// eval` has no supported way to do short of leaking it anyway.
///
/// Escape-to-dismiss isn't handled here - it's simpler to just add a `Key::Escape` arm to the
/// component's own `onkeydown` (see `WordNet`, `DisplayOptionsButton`), since keydown bubbles up
/// from whichever descendant actually has focus and needs no DOM-outside detection at all.
pub fn use_dismiss_on_outside_click(container_id: &'static str, on_dismiss: impl FnMut() + 'static) {
    // Boxed in a `Rc<RefCell<_>>` (via `use_hook`, so it's wrapped exactly once) rather than
    // captured directly by the `use_effect` closure below: `use_effect`'s callback is `FnMut`
    // (it may run again on later renders even though the `started` guard makes every run but the
    // first a no-op), so the borrow checker requires anything it captures to survive being
    // "used" more than once - which a plain `impl FnMut` moved into the inner `spawn` can't
    // satisfy, but cloning an `Rc` on each run can.
    let on_dismiss = use_hook(|| Rc::new(RefCell::new(on_dismiss)));
    let mut started = use_signal(|| false);
    use_effect(move || {
        if started() {
            return;
        }
        started.set(true);
        let on_dismiss = on_dismiss.clone();
        spawn(async move {
            let mut eval = document::eval(&format!(
                r#"
                document.addEventListener('mousedown', (e) => {{
                    const el = document.getElementById('{container_id}');
                    if (el && !el.contains(e.target)) {{
                        dioxus.send(true);
                    }}
                }}, true);
                "#
            ));
            loop {
                match eval.recv::<bool>().await {
                    Ok(_) => (on_dismiss.borrow_mut())(),
                    Err(_) => break,
                }
            }
        });
    });
}
