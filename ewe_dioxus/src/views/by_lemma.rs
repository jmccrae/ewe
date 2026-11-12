use dioxus::prelude::*;
use crate::components::{WordNet, Synset, DisplayOptions, DownloadLinks, ProjectName};
use crate::backend::api::{get_lemma, get_lemma_description};
use ewe_lib::wordnet::{PartOfSpeech, SynsetId};

/// The headings synsets are grouped under, with the parts of speech each one holds, in
/// display order. Synsets of any other part of speech are shown under "Other".
const POS_GROUPS: [(&str, &[PartOfSpeech]); 5] = [
    ("Nouns", &[PartOfSpeech::n]),
    ("Verbs", &[PartOfSpeech::v]),
    ("Adjectives", &[PartOfSpeech::a, PartOfSpeech::s]),
    ("Adverbs", &[PartOfSpeech::r]),
    ("Names", &[PartOfSpeech::e]),
];

/// Group synset ids under their part of speech heading, leaving out empty groups
fn group_by_pos<'a>(synsets: impl IntoIterator<Item = &'a SynsetId>) -> Vec<(&'static str, Vec<&'a SynsetId>)> {
    let mut groups: Vec<(&'static str, Vec<&'a SynsetId>)> =
        POS_GROUPS.iter().map(|(heading, _)| (*heading, Vec::new())).collect();
    groups.push(("Other", Vec::new()));
    for synset in synsets {
        let pos = synset.part_of_speech();
        let i = POS_GROUPS
            .iter()
            .position(|(_, poses)| pos.is_some_and(|pos| poses.contains(&pos)))
            .unwrap_or(POS_GROUPS.len());
        groups[i].1.push(synset);
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
        let ids: Vec<SynsetId> = ["00000001-s", "00000002-n", "00000003-e", "00000004-a", "00000005-d", "00000006-n"]
            .iter().map(|s| SynsetId::new(s)).collect();
        let groups = group_by_pos(&ids);
        let summary: Vec<(&str, Vec<&str>)> = groups.iter()
            .map(|(h, g)| (*h, g.iter().map(|s| s.as_str()).collect()))
            .collect();
        assert_eq!(summary, vec![
            ("Nouns", vec!["00000002-n", "00000006-n"]),
            ("Adjectives", vec!["00000001-s", "00000004-a"]),
            ("Names", vec!["00000003-e"]),
            ("Other", vec!["00000005-d"]),
        ]);
    }
}
