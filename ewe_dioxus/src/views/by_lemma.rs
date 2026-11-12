use dioxus::prelude::*;
use crate::components::{WordNet, Synset, DisplayOptions, DownloadLinks, ProjectName};
use crate::backend::api::{get_lemma, get_lemma_description};
use ewe_lib::wordnet::{PartOfSpeech, SynsetId};

/// Group synset ids under the heading of their part of speech (satellites with the adjectives),
/// in the order of `PartOfSpeech::ALL`, leaving out empty groups. Ids with no recognisable part
/// of speech go under the heading of "other".
fn group_by_pos<'a>(synsets: impl IntoIterator<Item = &'a SynsetId>) -> Vec<(&'static str, Vec<&'a SynsetId>)> {
    let mut groups: Vec<(&'static str, Vec<&'a SynsetId>)> = Vec::new();
    for pos in PartOfSpeech::ALL {
        if !groups.iter().any(|(heading, _)| *heading == pos.heading()) {
            groups.push((pos.heading(), Vec::new()));
        }
    }
    for synset in synsets {
        let heading = synset.part_of_speech().unwrap_or(PartOfSpeech::x).heading();
        if let Some((_, group)) = groups.iter_mut().find(|(h, _)| *h == heading) {
            group.push(synset);
        }
    }
    groups.retain(|(_, group)| !group.is_empty());
    groups
}

#[component]
pub fn ByLemma(lemma: ReadSignal<String>) -> Element {
    let synsets = use_server_future(move ||  {
        let value = lemma.cloned();
        async move  {
            get_lemma(value).await
        }
    })?;
    let description = use_server_future(move || {
        let value = lemma.cloned();
        async move { get_lemma_description(value).await }
    })?;
    let options = use_context::<Signal<DisplayOptions>>();
    let project_name = use_context::<Signal<ProjectName>>();

    let x = if let Some(Ok(synsets)) = &*synsets.read() {
        rsx! {
            div {
                if !project_name().0.is_empty() {
                    document::Title { "{lemma} - {project_name().0}" }
                }
                if let Some(Ok(Some(desc))) = &*description.read() {
                    document::Meta { name: "description", content: "{desc}" }
                }
                WordNet {},
                for (heading, group) in group_by_pos(synsets) {
                    h3 {
                        class: "pos_label",
                        "{heading}"
                    },
                    for synset in group.into_iter() {
                        Synset {
                            key: "{synset}",
                            synset_id: synset.clone(),
                            display_ids: options().show_ids,
                            display_sensekeys: options().show_sensekeys,
                            display_subcats: options().show_subcats,
                            display_sources: options().show_sources,
                            display_topics: options().show_topics,
                            display_pronunciations: options().show_pronunciations,
                            focus: lemma
                        }
                    }
                }
                DownloadLinks { kind: "lemma", id: lemma.cloned() },
            }
        }
    } else {
        rsx! {
            div { "Failed to load synsets" }
        }
    };
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_by_pos() {
        let ids: Vec<SynsetId> = ["00000001-s", "00000002-n", "00000003-e", "00000004-a", "00000005-d", "00000006-n", "00000007-q"]
            .iter().map(|s| SynsetId::new(s)).collect();
        let groups = group_by_pos(&ids);
        let summary: Vec<(&str, Vec<&str>)> = groups.iter()
            .map(|(h, g)| (*h, g.iter().map(|s| s.as_str()).collect()))
            .collect();
        assert_eq!(summary, vec![
            ("Nouns", vec!["00000002-n", "00000006-n"]),
            ("Adjectives", vec!["00000001-s", "00000004-a"]),
            ("Other", vec!["00000007-q"]),
            ("Determiners", vec!["00000005-d"]),
            ("Names", vec!["00000003-e"]),
        ]);
    }
}
