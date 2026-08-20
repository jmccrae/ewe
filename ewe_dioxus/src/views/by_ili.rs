use dioxus::prelude::*;
use crate::backend::api::get_synset_by_ili;
use crate::Route;

/// Resolves the old `en-word.net/ili/iXXX`-style URL (see issue #20 - that namespace 404s on
/// the live site now) to whichever synset currently carries that ILI, then redirects to its
/// canonical `/view/synset/:id` page - the same place picking an ILI match from the search bar
/// (`WordNet`/`autocomplete`) already sends you.
#[component]
pub fn ByIli(id: ReadSignal<String>) -> Element {
    let navigator = use_navigator();
    let synset = use_server_future(move || {
        let value = id.cloned();
        async move { get_synset_by_ili(value).await }
    })?;

    use_effect(move || {
        if let Some(Ok(Some(synset_id))) = &*synset.read() {
            navigator.replace(Route::BySynset { synset: synset_id.as_str().to_string() });
        }
    });

    let value = synset.read().clone();
    match value {
        Some(Ok(Some(_))) => rsx! {},
        Some(Ok(None)) => rsx! {
            div { "No synset found for ILI \"{id}\"." }
        },
        _ => rsx! {
            div { "Failed to look up ILI \"{id}\"." }
        },
    }
}
