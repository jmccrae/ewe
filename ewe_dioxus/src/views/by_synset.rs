use dioxus::prelude::*;
use crate::components::{WordNet, Synset, DisplayOptions, DownloadLinks};
use oewn_lib::wordnet::SynsetId;

#[component]
pub fn BySynset(synset: ReadSignal<String>) -> Element {
    let options = use_context::<Signal<DisplayOptions>>();

    rsx! {
        div {
            WordNet {},
            Synset {
                key: "{synset}",
                synset_id: SynsetId::new_owned(synset.cloned()),
                display_ids: options().show_ids,
                display_sensekeys: options().show_sensekeys,
                display_subcats: options().show_subcats,
                display_topics: options().show_topics,
                display_pronunciations: options().show_pronunciations,
                focus: String::new()
            }
            DownloadLinks { kind: "synset", id: synset.cloned() },
        }
    }
}
