use dioxus::prelude::*;

/// The site's configured `project_name` (from `settings.toml`), shared via context so
/// route views can incorporate it into their `document::Title` without each having to
/// fetch branding themselves. Populated by `WNLayout` once `get_branding` resolves;
/// empty until then.
#[derive(Clone, PartialEq, Default)]
pub struct ProjectName(pub String);

/// Provide a [`ProjectName`] signal to descendant components. Call once from a layout or
/// root component; descendants read it with `use_context::<Signal<ProjectName>>()`.
pub fn provide_project_name() -> Signal<ProjectName> {
    use_context_provider(|| Signal::new(ProjectName::default()))
}
