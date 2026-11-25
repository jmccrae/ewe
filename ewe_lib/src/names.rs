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
use crate::wordnet::entry::BTEntries;
use crate::wordnet::section::entry_key;
use crate::wordnet::BTSynsets;
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
    #[error(transparent)]
    Load(#[from] crate::wordnet::WordNetYAMLIOError),
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
    restrict_names_into(wn, mode, LexiconHashMapBackend::new())
}

/// As [`restrict_names`], but writes the copy into `out` (e.g. an on-disk `ReDBLexicon`) and
/// streams `wn` instead of collecting it, so memory use is bounded by the id sets below rather
/// than by the size of the lexicon's contents.
pub fn restrict_names_into<L: Lexicon, O: Lexicon>(wn: &L, mode: NamesMode, mut out: O) -> Result<O> {
    // Pass 1: which synsets are kept
    let mut kept_synsets: HashSet<SynsetId> = HashSet::new();
    for group in wn.synsets_iter()? {
        let (_, group) = group?;
        for s in group.iter()? {
            let (id, synset) = s?;
            if synset_kept(&synset, mode) {
                kept_synsets.insert(id);
            }
        }
    }

    // Pass 2: which entries and senses are kept. Entries come out sorted by lemma, so all the
    // entries of one lemma are adjacent; a (synset, lemma) member is only ever dropped if none of
    // that lemma's entries keeps it, so it is decided one lemma at a time.
    let mut dropped_members: HashSet<(SynsetId, String)> = HashSet::new();
    let mut kept_senses: HashSet<SenseId> = HashSet::new();
    let mut group_lemma = String::new();
    let mut group_dropped: HashSet<(SynsetId, String)> = HashSet::new();
    let mut group_kept: HashSet<(SynsetId, String)> = HashSet::new();
    macro_rules! flush_group {
        () => {
            for key in group_dropped.drain() {
                if !group_kept.contains(&key) {
                    dropped_members.insert(key);
                }
            }
            group_kept.clear();
        };
    }
    for bucket in wn.entries_iter()? {
        let (_, bucket) = bucket?;
        for e in bucket.entries()? {
            let (lemma, pos, entry) = e?;
            if lemma != group_lemma {
                flush_group!();
                group_lemma = lemma.clone();
            }
            let name_entry = is_name_pos(&pos);
            let mut any_kept = false;
            for sense in entry.sense.iter() {
                if sense_kept(sense, name_entry, mode, &kept_synsets) {
                    any_kept = true;
                    group_kept.insert((sense.synset.clone(), lemma.clone()));
                } else {
                    group_dropped.insert((sense.synset.clone(), lemma.clone()));
                }
            }
            if entry_kept(&entry, any_kept, name_entry, mode) {
                for sense in entry.sense.iter() {
                    if sense_kept(sense, name_entry, mode, &kept_synsets) {
                        kept_senses.insert(sense.id.clone());
                    }
                }
            }
        }
    }
    flush_group!();

    // Pass 3: write the kept synsets, with everything that points at something gone removed.
    // Both this and the entries below are written in batches through the bulk-load paths, and
    // the reverse indexes are built once at the end, as `Lexicon::load` does: adding records one
    // by one maintains them incrementally, which is far too slow on a large lexicon.
    for group in wn.synsets_iter()? {
        let (lexname, group) = group?;
        let target_lexname = if mode == NamesMode::Merged { merged_lexfile(lexname) } else { lexname.to_string() };
        let mut batch = BTSynsets::new();
        for s in group.iter()? {
            let (id, synset) = s?;
            if !kept_synsets.contains(&id) {
                continue;
            }
            let mut synset = synset.into_owned();
            synset.for_each_rel_mut(|rels| rels.retain(|t| kept_synsets.contains(t)));
            synset.members.retain(|m| !dropped_members.contains(&(id.clone(), m.clone())));
            let new_id = if mode == NamesMode::Merged {
                let new_id = merged_id(&id);
                if new_id != id && kept_synsets.contains(&new_id) {
                    return Err(NamesError::IdCollision { name: id, noun: new_id });
                }
                if is_name_synset(&synset) {
                    synset.part_of_speech = PartOfSpeech::n;
                }
                synset.for_each_rel_mut(|rels| rename_synsets(rels));
                new_id
            } else {
                id
            };
            batch.insert(new_id, synset)?;
            if batch.0.len() >= WRITE_BATCH {
                out.synsets_insert(target_lexname.clone(), std::mem::replace(&mut batch, BTSynsets::new()))?;
            }
        }
        if !batch.0.is_empty() {
            out.synsets_insert(target_lexname, batch)?;
        }
    }

    // Pass 3b: write the kept entries, one lemma at a time
    let mut writer = EntryWriter::default();
    let mut pending: Vec<(PosKey, Entry)> = Vec::new();
    let mut pending_lemma = String::new();
    for bucket in wn.entries_iter()? {
        let (_, bucket) = bucket?;
        for e in bucket.entries()? {
            let (lemma, pos, entry) = e?;
            if lemma != pending_lemma {
                write_lemma(&mut out, &mut writer, mode, &pending_lemma, std::mem::take(&mut pending))?;
                pending_lemma = lemma.clone();
            }
            let name_entry = is_name_pos(&pos);
            let any_kept = entry.sense.iter().any(|s| sense_kept(s, name_entry, mode, &kept_synsets));
            if !entry_kept(&entry, any_kept, name_entry, mode) {
                continue;
            }
            let mut entry = entry.into_owned();
            entry.sense.retain(|s| sense_kept(s, name_entry, mode, &kept_synsets));
            for sense in entry.sense.iter_mut() {
                restrict_sense(sense, &kept_senses, &kept_synsets);
            }
            pending.push((pos, entry));
        }
    }
    write_lemma(&mut out, &mut writer, mode, &pending_lemma, pending)?;
    writer.flush(&mut out)?;
    out.frames_set(wn.frames_get()?.into_owned())?;
    crate::wordnet::lexicon::finalize_bulk_load_with(&mut out, false)?;
    Ok(out)
}

/// How many records are written to the output at a time
const WRITE_BATCH: usize = 2000;

/// Collects entries by section so they can be written to the output in batches
#[derive(Default)]
struct EntryWriter {
    by_section: BTreeMap<char, BTEntries>,
    count: usize,
}

impl EntryWriter {
    fn add<O: Lexicon>(&mut self, out: &mut O, lemma: &str, pos: PosKey, entry: Entry) -> Result<()> {
        for sense in entry.sense.iter() {
            out.sense_id_to_lemma_pos_insert(sense.id.clone(), (lemma.to_string(), pos.clone()))?;
        }
        self.by_section
            .entry(entry_key(lemma))
            .or_insert_with(BTEntries::new)
            .0
            .entry(lemma.to_string())
            .or_default()
            .insert(pos, entry);
        self.count += 1;
        if self.count >= WRITE_BATCH {
            self.flush(out)?;
        }
        Ok(())
    }

    fn flush<O: Lexicon>(&mut self, out: &mut O) -> Result<()> {
        for (key, entries) in std::mem::take(&mut self.by_section) {
            out.entries_insert(key, entries)?;
        }
        self.count = 0;
        Ok(())
    }
}

fn synset_kept(synset: &Synset, mode: NamesMode) -> bool {
    match mode {
        NamesMode::Include | NamesMode::Merged => true,
        NamesMode::Exclude => !is_name_synset(synset),
        NamesMode::Only => is_name_synset(synset),
    }
}

fn sense_kept(sense: &Sense, name_entry: bool, mode: NamesMode, kept_synsets: &HashSet<SynsetId>) -> bool {
    kept_synsets.contains(&sense.synset)
        && match mode {
            NamesMode::Include | NamesMode::Merged => true,
            NamesMode::Exclude => !name_entry,
            NamesMode::Only => name_entry,
        }
}

/// Whether an entry survives, given whether any of its senses did
fn entry_kept(entry: &Entry, any_sense_kept: bool, name_entry: bool, mode: NamesMode) -> bool {
    if !entry.sense.is_empty() {
        return any_sense_kept;
    }
    // An entry that never had a sense is only part of the common vocabulary or of the names,
    // never of both
    !match mode {
        NamesMode::Exclude => name_entry,
        NamesMode::Only => !name_entry,
        _ => false,
    }
}

/// Write the (already filtered) entries of one lemma, merging names into nouns in `Merged` mode
fn write_lemma<O: Lexicon>(out: &mut O, writer: &mut EntryWriter, mode: NamesMode, lemma: &str, entries: Vec<(PosKey, Entry)>) -> Result<()> {
    if entries.is_empty() {
        return Ok(());
    }
    if mode != NamesMode::Merged {
        for (pos, entry) in entries {
            writer.add(out, lemma, pos, entry)?;
        }
        return Ok(());
    }
    // The first name entry of a lemma joins the lemma's noun entry
    let mut by_pos: BTreeMap<PosKey, Entry> = entries.into_iter().collect();
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
        writer.add(out, lemma, pos, entry)?;
    }
    Ok(())
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

    /// The same export from a lexicon kept on disk (what `--low-memory` uses) is the same
    /// document, whichever mode and order it is written in
    #[cfg(feature = "redb")]
    #[test]
    fn test_on_disk_export_matches_in_memory() {
        use crate::wordnet::xml::{write_lexicon_xml_to, XmlOrder};
        use crate::wordnet::{LexiconMetadata, ReDBLexicon};
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
        let sorted_lines = |wn: &dyn Fn(&mut Vec<u8>)| {
            let mut out = Vec::new();
            wn(&mut out);
            let mut lines: Vec<String> =
                String::from_utf8(out).unwrap().lines().map(|l| l.to_string()).collect();
            lines.sort();
            lines
        };
        let dir = std::env::temp_dir().join(format!("ewe_test_disk_export_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for (n, mode) in [NamesMode::Include, NamesMode::Exclude, NamesMode::Only, NamesMode::Merged]
            .into_iter()
            .enumerate()
        {
            let memory = restrict_names(&fixture(), mode).unwrap();
            let expected = sorted_lines(&|out| {
                write_lexicon_xml_to(&memory, &metadata, out, XmlOrder::ById).unwrap();
            });
            let disk = restrict_names_into(
                &fixture(),
                mode,
                ReDBLexicon::create(dir.join(format!("{n}.redb")), 1024 * 1024).unwrap(),
            )
            .unwrap();
            for order in [XmlOrder::ById, XmlOrder::Storage] {
                let got = sorted_lines(&|out| {
                    write_lexicon_xml_to(&disk, &metadata, out, order).unwrap();
                });
                assert_eq!(got, expected, "{mode:?} written {order:?}");
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
