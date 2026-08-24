use crate::backend::api::get_branding;
use crate::backend::setup::get_setup_status;
use crate::components::{
    provide_dirty_state, provide_display_options, provide_panel_visibility, provide_project_name,
    ProjectName, SetupNeeded, UnsavedChangesToast, ValidateButton,
};
use crate::Route;
use dioxus::prelude::*;

/// The Downloads page and JSON API docs are web-facing features that don't apply to the
/// single-user desktop app (which already has direct local access to its own data).
#[cfg(not(feature = "desktop"))]
#[component]
fn WebOnlyFooterLinks() -> Element {
    rsx! {
        Link { to: Route::Downloads {}, "Downloads" }
        " | "
        a { href: "/api/docs", "JSON API documentation" }
        " | "
    }
}

#[cfg(feature = "desktop")]
#[component]
fn WebOnlyFooterLinks() -> Element {
    rsx! {}
}

/// The change-history page (`views::history::History`) has nothing to show without the `edit`
/// feature (its own `#[cfg(not(feature = "edit"))]` variant renders a "not available" message
/// for anyone who navigates straight to `/history`) - so the footer link to it shouldn't be
/// offered as a nav option on a build without `edit` either.
#[cfg(feature = "edit")]
#[component]
fn HistoryLink() -> Element {
    rsx! {
        Link { to: Route::History {}, "History" }
        " | "
    }
}

#[cfg(not(feature = "edit"))]
#[component]
fn HistoryLink() -> Element {
    rsx! {}
}

/// The full set of theme override CSS custom properties, in application order. Field names on
/// `ThemeOverrides` map 1:1 onto `assets/styling/theme.css`'s "Roles"/font custom properties -
/// see that struct's doc comment.
fn theme_override_pairs(theme: &crate::backend::api::ThemeOverrides) -> Vec<(&'static str, Option<&str>)> {
    vec![
        ("--color-primary", theme.primary.as_deref()),
        ("--color-accent", theme.accent.as_deref()),
        ("--color-text", theme.text.as_deref()),
        ("--color-text-secondary", theme.text_secondary.as_deref()),
        ("--color-text-muted", theme.text_muted.as_deref()),
        ("--color-text-dim", theme.text_dim.as_deref()),
        ("--color-text-faint", theme.text_faint.as_deref()),
        ("--color-text-mute", theme.text_mute.as_deref()),
        ("--color-text-strong", theme.text_strong.as_deref()),
        ("--color-text-on-dark", theme.text_on_dark.as_deref()),
        ("--color-border", theme.border.as_deref()),
        ("--color-border-light", theme.border_light.as_deref()),
        ("--color-surface-light", theme.surface_light.as_deref()),
        ("--color-surface-hover", theme.surface_hover.as_deref()),
        ("--color-surface-dark", theme.surface_dark.as_deref()),
        ("--font-body", theme.font_body.as_deref()),
        ("--font-heading", theme.font_heading.as_deref()),
        ("--font-heading-weight", theme.font_heading_weight.as_deref()),
        ("--font-mono", theme.font_mono.as_deref()),
    ]
}

/// Builds the JS run against `document.documentElement`'s inline style to apply (`setProperty`)
/// or clear (`removeProperty`, for `None`) every theme override in one `document::eval` call.
/// `documentElement` (rather than some wrapping `<div>` in the render tree) is the target because
/// `assets/styling/main.css`'s own `body` rule reads `var(--font-body)`/`var(--color-accent)`/
/// `var(--color-text)` directly, and `body` is an *ancestor* of everything `WNLayout` renders - an
/// override set only on a descendant element would never reach `body`'s own rule, but
/// `documentElement` is an ancestor of `body` too, so it correctly cascades everywhere.
/// `removeProperty` on `None` matters on desktop: `SETTINGS` is hot-swapped in place on a project
/// switch (see `backend::setup::configure_wordnet_source`), so without it a stale override from a
/// previously themed project would leak into an unthemed one in the same running session.
///
/// Pure string-building, deliberately separated from the actual `document::eval` call so it's
/// unit-testable without a real DOM/webview.
fn build_theme_eval_js(pairs: &[(&str, Option<&str>)]) -> String {
    let mut js = String::new();
    for (var_name, value) in pairs {
        match value {
            Some(v) => js.push_str(&format!(
                "document.documentElement.style.setProperty('{var_name}', \"{}\");",
                escape_js_string(v)
            )),
            None => js.push_str(&format!(
                "document.documentElement.style.removeProperty('{var_name}');"
            )),
        }
    }
    js
}

/// A minimal JS string-literal escaper, matching the one `dioxus_document` uses internally for
/// its own `document::eval` calls (it isn't exposed publicly, so this is a small copy rather than
/// pulling in `serde_json` as a dependency just for this one call site). Kept as a defense-in-depth
/// layer around `build_theme_eval_js`'s interpolated values even though `EweSettings::load`
/// already validates them - validation and this call site live in different modules, and the
/// escape is cheap insurance against them drifting apart later (e.g. a new override field added
/// here without a matching validation arm there).
fn escape_js_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('"', "\\\"")
}

#[cfg(test)]
mod theme_eval_tests {
    use super::*;

    #[test]
    fn build_theme_eval_js_sets_present_and_clears_absent() {
        let js = build_theme_eval_js(&[
            ("--color-primary", Some("#002868")),
            ("--color-accent", None),
        ]);
        assert!(js.contains(
            "document.documentElement.style.setProperty('--color-primary', \"#002868\");"
        ));
        assert!(js.contains("document.documentElement.style.removeProperty('--color-accent');"));
    }

    #[test]
    fn build_theme_eval_js_escapes_values() {
        let js = build_theme_eval_js(&[("--font-body", Some("weird\"value"))]);
        assert!(js.contains("weird\\\"value"));
        assert!(!js.contains("\"weird\"value\""));
    }

    #[test]
    fn build_theme_eval_js_all_none_yields_only_removes() {
        let js = build_theme_eval_js(&[("--color-primary", None), ("--color-accent", None)]);
        assert!(js.contains("removeProperty('--color-primary')"));
        assert!(js.contains("removeProperty('--color-accent')"));
        assert!(!js.contains("setProperty"));
    }

    #[test]
    fn build_theme_eval_js_empty_input_yields_empty_string() {
        assert_eq!(build_theme_eval_js(&[]), "");
    }
}

#[component]
pub fn WNLayout() -> Element {
    provide_display_options();
    provide_panel_visibility();
    provide_dirty_state();
    let mut project_name_ctx = provide_project_name();

    // Branding is fetched through a server function rather than reading
    // `crate::SETTINGS` here directly, since this component also runs in the
    // WASM client and `SETTINGS` is a server-only `Lazy`.
    let branding = use_loader(get_branding);
    let (project_name, footer, logo_svg) = match &branding {
        Ok(loaded) if !loaded.loading() => {
            let branding = loaded.read();
            (
                branding.project_name.clone(),
                branding.footer.clone(),
                branding.logo_svg.clone(),
            )
        }
        _ => (String::new(), String::new(), String::new()),
    };
    // `branding` is fetched here at the layout level - once, before it's known whether the app
    // is even configured - so it's already stuck showing stale (pre-configure) project_name/
    // footer by the time the desktop setup screen finishes. Unlike route content (which only
    // starts loading *after* `configured` flips true, so it's never stale), this needs its own
    // explicit restart alongside `setup_status`'s - see `SetupNeeded`'s doc comment.
    let branding_loader = match &branding {
        Ok(loaded) => Some(*loaded),
        Err(_) => None,
    };

    // Applies `branding.theme`'s overrides (colours/fonts) to `document.documentElement`'s
    // inline style - see `build_theme_eval_js`'s doc comment for why `documentElement` and not a
    // rendered element. `loader.read()` happens *inside* the closure (not hoisted out with
    // `project_name`/`footer`/`logo_svg` above) so this effect's reactive subscription is on the
    // loader's own signal - it reruns exactly when the resolved branding changes (the initial
    // "still loading" -> loaded transition, and again after a desktop project switch's
    // `.restart()`), with no manual before/after diffing needed.
    use_effect(move || {
        if let Some(loader) = branding_loader {
            if !loader.loading() {
                let branding = loader.read();
                let pairs = theme_override_pairs(&branding.theme);
                let js = build_theme_eval_js(&pairs);
                if !js.is_empty() {
                    document::eval(&js);
                }
            }
        }
    });

    // Shares `project_name` with route views via context so each can compose its own
    // `document::Title` (e.g. "{lemma} - {project_name}") without fetching branding itself.
    // Written directly here (not in a `use_effect`) so it's visible to `Outlet`'s children
    // within the same render pass - an effect would only run after the initial SSR render
    // completes, too late for those children's own `document::Title` to see it.
    if project_name_ctx().0 != project_name {
        project_name_ctx.set(ProjectName(project_name.clone()));
    }

    // The logo/title is centered on the home page (OED-style hero treatment)
    // but stays left-aligned everywhere else, like a normal site header.
    let is_home = matches!(use_route::<Route>(), Route::Home {});

    // Runs on every route (unlike the old SSR-only "Error loading lexicon" gate this replaced
    // in `main.rs`'s `App()`), so it's a consistent, isomorphic screen no matter which page the
    // user navigates to first. While `status` is still loading, optimistically render the
    // normal routed content rather than flash the setup screen - on this app's fullstack setup,
    // server functions like this are awaited during the initial render (same as `branding`
    // above), so "still loading" is not expected to be user-visible in practice.
    let setup_status = use_loader(get_setup_status);
    let not_configured = match &setup_status {
        Ok(loaded) if !loaded.loading() => Some(loaded.read().clone()).filter(|s| !s.configured),
        _ => None,
    };
    // Passed down to `SetupNeeded` so the desktop configure flow can call `.restart()` on this
    // exact loader once done - re-running `get_setup_status` and reactively re-rendering this
    // component with the fresh result, without needing a real page reload (which isn't guaranteed
    // to behave the same way in a desktop webview as it does in a browser).
    let setup_status_loader = match &setup_status {
        Ok(loaded) => Some(*loaded),
        Err(_) => None,
    };

    rsx! {
        div {
            class: "container",
            Link {
                id: "logo",
                class: if is_home { "home-logo" },
                to: Route::Home {},
                span {
                    id: "logo-img",
                    dangerous_inner_html: "{logo_svg}"
                }
                span {
                    id: "logo-title",
                    h1 {
                        class: "en-title",
                        "{project_name}"
                    }
                }
            }
            if let (Some(status), Some(loader), Some(branding_loader)) =
                (not_configured.clone(), setup_status_loader, branding_loader)
            {
                SetupNeeded { status, setup_status: loader, branding: branding_loader }
            } else {
                Outlet::<Route> {}
            }
            footer {
                class: "footer",
                div {
                    dangerous_inner_html: "{footer}"
                }
                p {
                    class: "api-docs-link",
                    WebOnlyFooterLinks {}
                    HistoryLink {}
                    ValidateButton {}
                }
            }
            UnsavedChangesToast {}
        }
    }
}
