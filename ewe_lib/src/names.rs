//! Choosing which names (proper nouns, part of speech `e`) a release contains.
//!
//! The data keeps names apart from the common vocabulary: synsets with ids ending `-e` in
//! `name.*` lexfiles, and entries with part of speech `e`. A release can take them in several
//! ways, see [`NamesMode`]. [`restrict_names`] produces a self-consistent copy of a lexicon for
//! the chosen mode, which the XML, WNDB and RDF exporters (and `ewe stats`) then work on
//! unchanged.
use crate::wordnet::{
    Entries, Entry, Lexicon, LexiconError, LexiconHashMapBackend, PartOfSpeech, PosKey, Sense,
    SenseId, Synset, SynsetId, Synsets, UnresolvedSenseOrSynsetId,
};
use std::collections::{BTreeMap, HashSet};
use thiserror::Error;

/// Which names a release contains
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NamesMode {
    /// Everything as stored: names keep their `-e` ids and part of speech `e`
    #[default]
    Include,
    /// The core edition: no names, and nothing that refers to one
    Exclude,
    /// Only the names, and nothing that refers to the common vocabulary
    Only,
    /// The names as ordinary nouns, as in the releases before names had their own part of
    /// speech: `-n` ids, `noun.*` lexfiles, and entries merged with those of the same lemma
    Merged,
}

#[derive(Error, Debug)]
pub enum NamesError {
    #[error(transparent)]
    Lexicon(#[from] LexiconError),
    #[error("Cannot merge name {name} into the nouns: {noun} already exists")]
    IdCollision { name: SynsetId, noun: SynsetId },
}

type Result<T> = std::result::Result<T, NamesError>;

fn is_name_synset(synset: &Synset) -> bool {
    synset.part_of_speech == PartOfSpeech::e
}

fn is_name_pos(pos: &PosKey) -> bool {
    pos.to_part_of_speech() == Some(PartOfSpeech::e)
}

/// The id a name synset gets in `Merged` mode (`08337853-e` becomes `08337853-n`)
fn merged_id(id: &SynsetId) -> SynsetId {
    match id.as_str().strip_suffix("-e") {
        Some(offset) => SynsetId::new_owned(format!("{offset}-n")),
        None => id.clone(),
    }
}

fn merged_lexfile(lexfile: &str) -> String {
    match lexfile.strip_prefix("name.") {
        Some(rest) => format!("noun.{rest}"),
        None => lexfile.to_string(),
    }
}

/// A copy of `wn` containing the names as described by `mode`, with no relation, sense or
/// member left pointing at something that is not in the copy. `Include` copies everything.
pub fn restrict_names<L: Lexicon>(wn: &L, mode: NamesMode) -> Result<LexiconHashMapBackend> {
    // Pass 1: which synsets are kept
    let mut synsets: Vec<(String, SynsetId, Synset)> = Vec::new();
    for group in wn.synsets_iter()? {
        let (lexname, group) = group?;
        for s in group.iter()? {
            let (id, synset) = s?;
            let keep = match mode {
                NamesMode::Include | NamesMode::Merged => true,
                NamesMode::Exclude => !is_name_synset(&synset),
                NamesMode::Only => is_name_synset(&synset),
            };
            if keep {
                synsets.push((lexname.clone(), id, synset.into_owned()));
            }
        }
    }
    let kept_synsets: HashSet<SynsetId> = synsets.iter().map(|(_, id, _)| id.clone()).collect();

    // Pass 2: which entries and senses are kept
    let mut entries: Vec<(String, PosKey, Entry)> = Vec::new();
    // senses that were dropped, per (synset, lemma), to tidy the synset members afterwards
    let mut dropped_members: HashSet<(SynsetId, String)> = HashSet::new();
    let mut kept_members: HashSet<(SynsetId, String)> = HashSet::new();
    let mut kept_senses: HashSet<SenseId> = HashSet::new();
    for bucket in wn.entries_iter()? {
        let (_, bucket) = bucket?;
        for e in bucket.entries()? {
            let (lemma, pos, entry) = e?;
            let mut entry = entry.into_owned();
            let had_senses = !entry.sense.is_empty();
            let name_entry = is_name_pos(&pos);
            entry.sense.retain(|sense| {
                let keep = kept_synsets.contains(&sense.synset)
                    && match mode {
                        NamesMode::Include | NamesMode::Merged => true,
                        NamesMode::Exclude => !name_entry,
                        NamesMode::Only => name_entry,
                    };
                if keep {
                    kept_members.insert((sense.synset.clone(), lemma.clone()));
                } else {
                    dropped_members.insert((sense.synset.clone(), lemma.clone()));
                }
                keep
            });
            if had_senses && entry.sense.is_empty() {
                continue;
            }
            // An entry that never had a sense is only part of the common vocabulary or of the
            // names, never of both
            if entry.sense.is_empty()
                && match mode {
                    NamesMode::Exclude => name_entry,
                    NamesMode::Only => !name_entry,
                    _ => false,
                }
            {
                continue;
            }
            for sense in &entry.sense {
                kept_senses.insert(sense.id.clone());
            }
            entries.push((lemma, pos, entry));
        }
    }

    // Pass 3: drop everything that points at something that is gone
    for (_, _, synset) in synsets.iter_mut() {
        synset.for_each_rel_mut(|rels| rels.retain(|t| kept_synsets.contains(t)));
    }
    for (_, id, synset) in synsets.iter_mut() {
        synset.members.retain(|m| {
            let key = (id.clone(), m.clone());
            !dropped_members.contains(&key) || kept_members.contains(&key)
        });
    }
    for (_, _, entry) in entries.iter_mut() {
        for sense in entry.sense.iter_mut() {
            restrict_sense(sense, &kept_senses, &kept_synsets);
        }
    }

    if mode == NamesMode::Merged {
        return merge_names(wn, synsets, entries);
    }

    let mut out = LexiconHashMapBackend::new();
    for (lexname, id, synset) in synsets {
        out.insert_synset(lexname, id, synset)?;
    }
    for (lemma, pos, entry) in entries {
        out.insert_entry(lemma, pos, entry)?;
    }
    out.frames_set(wn.frames_get()?.into_owned())?;
    Ok(out)
}

/// Remove the relations of a sense that point at a sense or synset that is not kept
fn restrict_sense(sense: &mut Sense, kept_senses: &HashSet<SenseId>, kept_synsets: &HashSet<SynsetId>) {
    sense.for_each_sense_rel_mut(|rels| rels.retain(|t| kept_senses.contains(t)));
    sense.for_each_mixed_rel_mut(|rels| {
        rels.retain(|t| match t {
            UnresolvedSenseOrSynsetId::Sense(id) => kept_senses.contains(id),
            UnresolvedSenseOrSynsetId::Synset(id) => kept_synsets.contains(id),
            UnresolvedSenseOrSynsetId::Unresolved(_) => true,
        })
    });
}

/// `Merged` mode: turn the names into nouns
fn merge_names<L: Lexicon>(
    wn: &L,
    synsets: Vec<(String, SynsetId, Synset)>,
    entries: Vec<(String, PosKey, Entry)>,
) -> Result<LexiconHashMapBackend> {
    let existing: HashSet<SynsetId> = synsets.iter().map(|(_, id, _)| id.clone()).collect();
    let mut out = LexiconHashMapBackend::new();
    for (lexname, id, mut synset) in synsets {
        let new_id = merged_id(&id);
        if new_id != id && existing.contains(&new_id) {
            return Err(NamesError::IdCollision { name: id, noun: new_id });
        }
        if is_name_synset(&synset) {
            synset.part_of_speech = PartOfSpeech::n;
        }
        synset.for_each_rel_mut(|rels| rename_synsets(rels));
        out.insert_synset(merged_lexfile(&lexname), new_id, synset)?;
    }

    // Entries: the first name entry of a lemma joins the lemma's noun entry
    let mut by_lemma: BTreeMap<String, BTreeMap<PosKey, Entry>> = BTreeMap::new();
    for (lemma, pos, entry) in entries {
        by_lemma.entry(lemma).or_default().insert(pos, entry);
    }
    for (lemma, mut by_pos) in by_lemma {
        let names: Vec<PosKey> = by_pos.keys().filter(|p| is_name_pos(p)).cloned().collect();
        let mut renamed: Vec<(PosKey, Entry)> = Vec::new();
        for (i, key) in names.into_iter().enumerate() {
            let mut entry = by_pos.remove(&key).expect("key just listed");
            for sense in entry.sense.iter_mut() {
                sense.synset = merged_id(&sense.synset);
                restrict_to_merged(sense);
            }
            if i == 0 {
                match by_pos.remove(&PosKey::new("n")) {
                    Some(noun) => by_pos.insert(PosKey::new("n"), merge_entries(noun, entry)),
                    None => by_pos.insert(PosKey::new("n"), entry),
                };
            } else {
                renamed.push((key, entry));
            }
        }
        // further name entries of the lemma (homographs) keep their own entry
        for (_, entry) in renamed {
            let mut k = 2;
            while by_pos.contains_key(&PosKey::new(format!("n{k}"))) {
                k += 1;
            }
            by_pos.insert(PosKey::new(format!("n{k}")), entry);
        }
        for (pos, mut entry) in by_pos {
            for sense in entry.sense.iter_mut() {
                sense.synset = merged_id(&sense.synset);
                restrict_to_merged(sense);
            }
            out.insert_entry(lemma.clone(), pos, entry)?;
        }
    }
    out.frames_set(wn.frames_get()?.into_owned())?;
    Ok(out)
}

fn rename_synsets(rels: &mut crate::wordnet::ScoredVec<SynsetId>) {
    let names: Vec<SynsetId> = rels.iter().filter(|t| t.as_str().ends_with("-e")).cloned().collect();
    for old in names {
        rels.rename(old.as_str(), || merged_id(&old));
    }
}

/// Point the synset targets of a sense's relations at the merged ids
fn restrict_to_merged(sense: &mut Sense) {
    sense.for_each_mixed_rel_mut(|rels| {
        let names: Vec<SynsetId> = rels
            .iter()
            .filter_map(|t| match t {
                UnresolvedSenseOrSynsetId::Synset(id) if id.as_str().ends_with("-e") => Some(id.clone()),
                _ => None,
            })
            .collect();
        for old in names {
            rels.rename(old.as_str(), || UnresolvedSenseOrSynsetId::Synset(merged_id(&old)));
        }
    });
}

/// Merge a name entry into the noun entry of the same lemma: the noun's senses come first
fn merge_entries(mut noun: Entry, name: Entry) -> Entry {
    noun.sense.extend(name.sense);
    for form in name.form {
        if !noun.form.contains(&form) {
            noun.form.push(form);
        }
    }
    for pronunciation in name.pronunciation {
        if !noun.pronunciation.contains(&pronunciation) {
            noun.pronunciation.push(pronunciation);
        }
    }
    noun.confidence = noun.confidence.or(name.confidence);
    noun.reconstructed |= name.reconstructed;
    noun
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::NullProgress;
    use crate::validate::{validate, ValidationError};
    use crate::wordnet::Entries;

    fn synset(pos: PartOfSpeech, definition: &str, members: &[&str]) -> Synset {
        let mut ss = Synset::new(pos);
        ss.definition.push(definition.to_string());
        ss.members = members.iter().map(|m| m.to_string()).collect();
        ss
    }

    fn sense(id: &str, synset: &str) -> Sense {
        Sense::new(SenseId::new(id), SynsetId::new(synset))
    }

    fn entry(senses: Vec<Sense>) -> Entry {
        let mut e = Entry::new();
        e.sense = senses;
        e
    }

    /// Common synsets, names, a lemma with a noun and a name entry, a name entry in a noun
    /// synset, and relations crossing the boundary in both directions.
    fn fixture() -> LexiconHashMapBackend {
        let mut wn = LexiconHashMapBackend::new();
        let id = SynsetId::new;
        wn.insert_synset("noun.Tops".into(), id("00000001-n"), synset(PartOfSpeech::n, "entity", &["entity"])).unwrap();
        let mut city = synset(PartOfSpeech::n, "a large town", &["city"]);
        city.hypernym.push(id("00000001-n"));
        wn.insert_synset("noun.location".into(), id("00000002-n"), city).unwrap();
        let mut boulevard = synset(PartOfSpeech::n, "a broad street", &["boulevard"]);
        boulevard.hypernym.push(id("00000001-n"));
        boulevard.domain_region.push(id("00000010-e"));
        boulevard.mero_member.push(id("00000010-e"));
        boulevard.wikidata.push("Q90".into());
        wn.insert_synset("noun.artifact".into(), id("00000003-n"), boulevard).unwrap();
        let mut prince = synset(PartOfSpeech::n, "a Trojan prince", &["Paris"]);
        prince.hypernym.push(id("00000001-n"));
        wn.insert_synset("noun.person".into(), id("00000004-n"), prince).unwrap();
        let mut pat = synset(PartOfSpeech::n, "a person called Pat", &["Pat"]);
        pat.hypernym.push(id("00000001-n"));
        wn.insert_synset("noun.person".into(), id("00000005-n"), pat).unwrap();

        let mut paris = synset(PartOfSpeech::e, "the capital of France", &["Paris"]);
        paris.instance_hypernym.push(id("00000002-n"));
        paris.wikidata.push("Q90".into());
        wn.insert_synset("name.location".into(), id("00000010-e"), paris).unwrap();
        let mut left_bank = synset(PartOfSpeech::e, "an area of Paris", &["Left Bank"]);
        left_bank.instance_hypernym.push(id("00000002-n"));
        wn.insert_synset("name.location".into(), id("00000011-e"), left_bank).unwrap();
        let mut quarter = synset(PartOfSpeech::e, "a district of the Left Bank", &["Latin Quarter"]);
        quarter.hypernym.push(id("00000011-e"));
        wn.insert_synset("name.location".into(), id("00000012-e"), quarter).unwrap();

        let mut e = |lemma: &str, pos: &str, senses: Vec<Sense>| {
            wn.insert_entry(lemma.to_string(), PosKey::new(pos), entry(senses)).unwrap();
        };
        e("entity", "n", vec![sense("entity%1:03:00::", "00000001-n")]);
        e("city", "n", vec![sense("city%1:15:00::", "00000002-n")]);
        let mut boulevard = sense("boulevard%1:06:00::", "00000003-n");
        boulevard.derivation.push(SenseId::new("paris%1:15:00::"));
        boulevard.domain_region.push(UnresolvedSenseOrSynsetId::Synset(id("00000010-e")));
        boulevard.other.push(UnresolvedSenseOrSynsetId::Synset(id("00000003-n")));
        e("boulevard", "n", vec![boulevard]);
        e("Paris", "n", vec![sense("paris%1:18:00::", "00000004-n")]);
        let mut paris_name = sense("paris%1:15:00::", "00000010-e");
        paris_name.derivation.push(SenseId::new("boulevard%1:06:00::"));
        e("Paris", "e", vec![paris_name]);
        e("Left Bank", "e", vec![sense("left_bank%1:15:00::", "00000011-e")]);
        e("Latin Quarter", "e", vec![sense("latin_quarter%1:15:00::", "00000012-e")]);
        e("Pat", "e", vec![sense("pat%1:18:00::", "00000005-n")]);
        wn
    }

    fn synset_ids(wn: &LexiconHashMapBackend) -> Vec<String> {
        let mut ids = Vec::new();
        for group in wn.synsets_iter().unwrap() {
            for s in group.unwrap().1.iter().unwrap() {
                ids.push(s.unwrap().0.as_str().to_string());
            }
        }
        ids.sort();
        ids
    }

    fn entry_keys(wn: &LexiconHashMapBackend) -> Vec<(String, String)> {
        let mut keys = Vec::new();
        for bucket in wn.entries_iter().unwrap() {
            for e in bucket.unwrap().1.entries().unwrap() {
                let (lemma, pos, _) = e.unwrap();
                keys.push((lemma, pos.as_str().to_string()));
            }
        }
        keys.sort();
        keys
    }

    fn dangling(wn: &LexiconHashMapBackend) -> Vec<String> {
        validate(wn, &mut NullProgress)
            .unwrap()
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    ValidationError::SynsetRelTargetMissing { .. }
                        | ValidationError::SenseRelTargetMissing { .. }
                        | ValidationError::SenseSynsetNotExists { .. }
                )
            })
            .map(|e| e.to_string())
            .collect()
    }

    #[test]
    fn test_the_fixture_is_consistent() {
        assert!(dangling(&fixture()).is_empty());
    }

    #[test]
    fn test_include_keeps_everything() {
        let wn = fixture();
        let all = restrict_names(&wn, NamesMode::Include).unwrap();
        assert_eq!(synset_ids(&all), synset_ids(&wn));
        assert_eq!(entry_keys(&all), entry_keys(&wn));
    }

    #[test]
    fn test_exclude_drops_names_and_everything_pointing_at_them() {
        let core = restrict_names(&fixture(), NamesMode::Exclude).unwrap();
        assert_eq!(
            synset_ids(&core),
            vec!["00000001-n", "00000002-n", "00000003-n", "00000004-n", "00000005-n"]
        );
        // the lemma with both kinds of entry keeps its noun, the names are gone
        assert_eq!(
            entry_keys(&core),
            vec![
                ("Paris".to_string(), "n".to_string()),
                ("boulevard".to_string(), "n".to_string()),
                ("city".to_string(), "n".to_string()),
                ("entity".to_string(), "n".to_string()),
            ]
        );
        let boulevard = core.synset_by_id(&SynsetId::new("00000003-n")).unwrap().unwrap();
        assert!(boulevard.domain_region.is_empty() && boulevard.mero_member.is_empty());
        assert_eq!(boulevard.hypernym.len(), 1);
        // the QID shared with the name no longer clashes
        assert!(validate(&core, &mut NullProgress).unwrap().iter().all(
            |e| !matches!(e, ValidationError::DuplicateWikidataId { .. })
        ));
        let sense = core.get_sense_by_id(&SenseId::new("boulevard%1:06:00::")).unwrap().unwrap().2;
        assert!(sense.derivation.is_empty() && sense.domain_region.is_empty());
        // a relation to something that is kept stays
        assert_eq!(sense.other.len(), 1);
        // a name that was a member of a noun synset is not left behind as a member
        let pat = core.synset_by_id(&SynsetId::new("00000005-n")).unwrap().unwrap();
        assert!(pat.members.is_empty());
        assert!(dangling(&core).is_empty());
    }

    #[test]
    fn test_only_keeps_the_names() {
        let names = restrict_names(&fixture(), NamesMode::Only).unwrap();
        assert_eq!(synset_ids(&names), vec!["00000010-e", "00000011-e", "00000012-e"]);
        assert_eq!(
            entry_keys(&names),
            vec![
                ("Latin Quarter".to_string(), "e".to_string()),
                ("Left Bank".to_string(), "e".to_string()),
                ("Paris".to_string(), "e".to_string()),
            ]
        );
        let paris = names.synset_by_id(&SynsetId::new("00000010-e")).unwrap().unwrap();
        assert!(paris.instance_hypernym.is_empty());
        let quarter = names.synset_by_id(&SynsetId::new("00000012-e")).unwrap().unwrap();
        assert_eq!(quarter.hypernym.len(), 1);
        assert!(dangling(&names).is_empty());
    }

    #[test]
    fn test_merged_turns_names_into_nouns() {
        let merged = restrict_names(&fixture(), NamesMode::Merged).unwrap();
        let ids = synset_ids(&merged);
        assert!(ids.iter().all(|id| id.ends_with("-n")), "{ids:?}");
        assert_eq!(ids.len(), 8);
        let paris = merged.synset_by_id(&SynsetId::new("00000010-n")).unwrap().unwrap();
        assert_eq!(paris.part_of_speech, PartOfSpeech::n);
        assert_eq!(merged.lex_name_for(&SynsetId::new("00000010-n")).unwrap().as_deref(), Some("noun.location"));
        // references follow the new ids
        let boulevard = merged.synset_by_id(&SynsetId::new("00000003-n")).unwrap().unwrap();
        assert_eq!(boulevard.domain_region.as_slice(), &[SynsetId::new("00000010-n")]);
        let quarter = merged.synset_by_id(&SynsetId::new("00000012-n")).unwrap().unwrap();
        assert_eq!(quarter.hypernym.as_slice(), &[SynsetId::new("00000011-n")]);
        // the lemma with a noun and a name entry has one noun entry, common sense first
        let entries = merged.entry_by_lemma_with_pos("Paris").unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0.as_str(), "n");
        let senses: Vec<_> = entries[0].1.sense.iter().map(|s| (s.id.as_str(), s.synset.as_str())).collect();
        assert_eq!(
            senses,
            vec![("paris%1:18:00::", "00000004-n"), ("paris%1:15:00::", "00000010-n")]
        );
        assert!(entry_keys(&merged).iter().all(|(_, pos)| pos == "n"));
        let (_, _, boulevard_sense) = merged.get_sense_by_id(&SenseId::new("boulevard%1:06:00::")).unwrap().unwrap();
        assert_eq!(
            boulevard_sense.domain_region.as_slice(),
            &[UnresolvedSenseOrSynsetId::Synset(SynsetId::new("00000010-n"))]
        );
        assert!(dangling(&merged).is_empty());
    }

    #[test]
    fn test_merged_refuses_to_overwrite_a_noun() {
        let mut wn = fixture();
        wn.insert_synset(
            "noun.location".into(),
            SynsetId::new("00000010-n"),
            synset(PartOfSpeech::n, "something else", &["something"]),
        )
        .unwrap();
        match restrict_names(&wn, NamesMode::Merged) {
            Err(NamesError::IdCollision { name, noun }) => {
                assert_eq!((name.as_str(), noun.as_str()), ("00000010-e", "00000010-n"));
            }
            other => panic!("expected a collision, got {:?}", other.map(|_| ())),
        }
    }

    #[test]
    fn test_exports_follow_the_mode() {
        use crate::wordnet::xml::write_lexicon_xml;
        use crate::wordnet::LexiconMetadata;
        let metadata = LexiconMetadata {
            lmf_version: crate::wordnet::LmfVersion::V1_5,
            confidence: None,
            id_prefix: "test".to_string(),
            label: "Test".to_string(),
            language: "en".to_string(),
            email: None,
            license: "https://creativecommons.org/licenses/by/4.0".to_string(),
            version: "1".to_string(),
            url: None,
        };
        let xml = |mode| {
            let wn = restrict_names(&fixture(), mode).unwrap();
            String::from_utf8(write_lexicon_xml(&wn, &metadata).unwrap()).unwrap()
        };
        let core = xml(NamesMode::Exclude);
        assert!(!core.contains("-e\"") && !core.contains("partOfSpeech=\"e\""), "{core}");
        let full = xml(NamesMode::Include);
        assert!(full.contains("partOfSpeech=\"e\""));
        let merged = xml(NamesMode::Merged);
        assert!(!merged.contains("-e\"") && !merged.contains("partOfSpeech=\"e\""));
        assert!(merged.contains("00000010-n"));
    }
}
