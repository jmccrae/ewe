//! How the relations between names (part of speech `e`) and the common vocabulary are stored.
//!
//! In the `merged` layout a relation is stored where it was made, so the base files
//! (`noun.*.yaml`, `entries-a.yaml`, ...) refer to name synsets and senses that only exist in the
//! name files. In the `separate` layout every such relation is written in the name files, on the
//! name end, in its inverse form (`hyponym`, `holo_member`, `has_domain_region`, ... for
//! synsets; an `inverse:` block listing the senses that have the relation for senses), so the
//! base files are self-contained without the names. Loading understands both layouts, whatever
//! the project setting is. The data model in memory is the same in both.
use crate::rels::{SenseRelType, SynsetRelType};
use crate::wordnet::{
    Entries, Lexicon, LexiconError, PartOfSpeech, PosKey, Sense, SenseId, SenseOrSynsetId, Synset,
    SynsetId, Synsets, UnresolvedSenseOrSynsetId,
};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;

type Result<T> = std::result::Result<T, LexiconError>;

/// How relations between names and the common vocabulary are written to the YAML files
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NamesLayout {
    /// Each relation is stored where it was made (the base files may refer to names)
    #[default]
    Merged,
    /// Every relation between a name and something else is stored only in the name files
    Separate,
}

/// The storage options of a project: the top-level `names_layout` of its `settings.toml`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StorageOptions {
    pub names_layout: NamesLayout,
}

#[derive(Deserialize)]
struct SettingsFile {
    #[serde(default)]
    names_layout: NamesLayout,
}

impl StorageOptions {
    /// Reads the options from the contents of a `settings.toml`
    pub fn from_settings_str(contents: &str) -> std::result::Result<Self, String> {
        toml::from_str::<SettingsFile>(contents)
            .map(|s| StorageOptions { names_layout: s.names_layout })
            .map_err(|e| e.to_string())
    }

    /// Finds the project `settings.toml` for the YAML source folder `yaml_dir` (in the folder
    /// itself or up to two parents, like `ValidationOptions::for_wordnet`). No `settings.toml`
    /// gives the default.
    pub fn for_wordnet(yaml_dir: &Path) -> std::result::Result<Self, String> {
        let start = yaml_dir.canonicalize().unwrap_or_else(|_| yaml_dir.to_path_buf());
        for dir in start.ancestors().take(3) {
            let candidate = dir.join("settings.toml");
            if candidate.is_file() {
                let contents = std::fs::read_to_string(&candidate)
                    .map_err(|e| format!("Could not read {}: {}", candidate.display(), e))?;
                return Self::from_settings_str(&contents)
                    .map_err(|e| format!("Invalid {}: {}", candidate.display(), e));
            }
        }
        Ok(Self::default())
    }
}

fn is_name_pos(pos: &PosKey) -> bool {
    pos.to_part_of_speech() == Some(PartOfSpeech::e)
}

/// The ids of the name synsets and name senses of a lexicon
fn names_of<L: Lexicon>(wn: &L) -> Result<(HashSet<SynsetId>, HashSet<SenseId>)> {
    let mut synsets = HashSet::new();
    for group in wn.synsets_iter()? {
        let (_, group) = group?;
        for s in group.iter()? {
            let (id, synset) = s?;
            if synset.part_of_speech == PartOfSpeech::e {
                synsets.insert(id);
            }
        }
    }
    let mut senses = HashSet::new();
    for bucket in wn.entries_iter()? {
        let (_, bucket) = bucket?;
        for e in bucket.entries()? {
            let (_, pos, entry) = e?;
            if is_name_pos(&pos) {
                senses.extend(entry.sense.iter().map(|s| s.id.clone()));
            }
        }
    }
    Ok((synsets, senses))
}

/// Loading: move the relations written in their inverse form on a name (the `separate`
/// layout) onto the synsets and senses they come from, and give the base end of a symmetric
/// relation that is written only on the name end (`similar`, `derivation`, ...) its copy. A
/// relation that is already there is left alone, so the `merged` layout, which has both ends,
/// loads unchanged. Must run before the reverse indexes are built.
pub(crate) fn apply_inverse_relations<L: Lexicon>(wn: &mut L) -> Result<()> {
    // Synsets
    let mut pending: Vec<(SynsetId, Vec<(SynsetRelType, SynsetId, Option<f64>)>)> = Vec::new();
    let mut symmetric: Vec<(SynsetId, SynsetRelType, SynsetId, Option<f64>)> = Vec::new();
    let mut names = HashSet::new();
    for group in wn.synsets_iter()? {
        let (_, group) = group?;
        for s in group.iter()? {
            let (id, synset) = s?;
            let is_name = synset.part_of_speech == PartOfSpeech::e;
            if is_name {
                names.insert(id.clone());
                // `attribute` is its own inverse and the data stores it at both ends, like `also`
                // and `similar`
                for rel in [SynsetRelType::Also, SynsetRelType::Similar, SynsetRelType::Attribute] {
                    for (target, conf) in synset.rel_targets(&rel).into_iter().flat_map(|t| t.iter_scored()) {
                        symmetric.push((id.clone(), rel.clone(), target.clone(), conf));
                    }
                }
            }
            if synset.has_inverse_rels() {
                let mut list = Vec::new();
                for rel in SynsetRelType::INVERSE_ONLY {
                    for (source, conf) in synset.inverse_rel_targets(&rel).into_iter().flat_map(|t| t.iter_scored()) {
                        list.push((rel.clone(), source.clone(), conf));
                    }
                }
                pending.push((id, list));
            }
        }
    }
    for (id, list) in pending {
        for (rel, source, conf) in list {
            if wn.synset_by_id(&source)?.is_none() {
                eprintln!("Warning: {} has {} {}, but that synset does not exist", id, rel.value(), source);
                continue;
            }
            // `id` has `source` as its `rel` (e.g. a hyponym), so the relation is stored the
            // way `add_rel(id, rel, source)` would store it
            let (stored_on_id, yaml_rel) = rel.clone().to_yaml();
            if stored_on_id {
                wn.update_synset(&id, |ss| ss.insert_rel_scored(&yaml_rel, &source, conf))?;
            } else {
                wn.update_synset(&source, |ss| ss.insert_rel_scored(&yaml_rel, &id, conf))?;
            }
        }
        wn.update_synset(&id, |ss| {
            for rel in SynsetRelType::INVERSE_ONLY {
                if let Some(targets) = ss.inverse_rel_targets_mut(&rel) {
                    targets.clear();
                }
            }
        })?;
    }
    for (name, rel, target, conf) in symmetric {
        if names.contains(&target) || wn.synset_by_id(&target)?.is_none() {
            continue;
        }
        let (_, yaml_rel) = rel.clone().to_yaml();
        wn.update_synset(&target, |ss| {
            if !ss.rel_targets(&rel).is_some_and(|t| t.iter().any(|x| *x == name)) {
                ss.insert_rel_scored(&yaml_rel, &name, conf);
            }
        })?;
    }

    // Senses
    type Pending = (String, PosKey, SenseId, Vec<(String, SenseId, Option<f64>)>);
    let mut pending: Vec<Pending> = Vec::new();
    let mut symmetric: Vec<(SenseId, SenseRelType, SenseId, Option<f64>)> = Vec::new();
    let mut name_senses: HashSet<SenseId> = HashSet::new();
    for bucket in wn.entries_iter()? {
        let (_, bucket) = bucket?;
        for e in bucket.entries()? {
            let (lemma, pos, entry) = e?;
            let is_name = is_name_pos(&pos);
            for sense in entry.sense.iter() {
                if is_name {
                    name_senses.insert(sense.id.clone());
                    for (rel, target) in sense.sense_links_from() {
                        if let (true, UnresolvedSenseOrSynsetId::Sense(target)) = (rel.is_symmetric(), target) {
                            let conf = sense.rel_confidence(&rel, target.as_str());
                            symmetric.push((sense.id.clone(), rel, target, conf));
                        }
                    }
                }
                if sense.inverse.values().any(|s| !s.is_empty()) {
                    let mut list = Vec::new();
                    for (rel, sources) in sense.inverse.iter() {
                        for (source, conf) in sources.iter_scored() {
                            list.push((rel.clone(), source.clone(), conf));
                        }
                    }
                    pending.push((lemma.clone(), pos.clone(), sense.id.clone(), list));
                }
            }
        }
    }
    for (lemma, pos, id, list) in pending {
        for (rel_name, source, conf) in list {
            let Some(rel) = SenseRelType::from(&rel_name) else {
                eprintln!("Warning: sense {} has an inverse relation {} that is not known", id, rel_name);
                continue;
            };
            add_sense_rel(wn, &source, rel, &id, conf)?;
        }
        wn.update_entry(&lemma, &pos, |entry| {
            for sense in entry.sense.iter_mut() {
                sense.inverse.clear();
            }
        })?;
    }
    for (name, rel, target, conf) in symmetric {
        if !name_senses.contains(&target) {
            add_sense_rel(wn, &target, rel, &name, conf)?;
        }
    }
    Ok(())
}

/// Give the sense `source` the relation `rel` to the sense `target`, unless it has it already
fn add_sense_rel<L: Lexicon>(
    wn: &mut L,
    source: &SenseId,
    rel: SenseRelType,
    target: &SenseId,
    conf: Option<f64>,
) -> Result<()> {
    let Some((lemma, pos)) = wn.sense_id_to_lemma_pos_get(source)? else {
        eprintln!("Warning: a relation {} to {} comes from sense {}, which does not exist", rel.value(), target, source);
        return Ok(());
    };
    wn.update_entry(&lemma, &pos, |entry| {
        if let Some(sense) = entry.sense.iter_mut().find(|s| s.id == *source) {
            if !sense
                .sense_links_from()
                .iter()
                .any(|(r, t)| *r == rel && t.as_str() == target.as_str())
            {
                sense.add_rel(rel.clone(), SenseOrSynsetId::Sense(target.clone()));
                if conf.is_some() {
                    sense.set_rel_confidence(&rel, target.as_str(), conf);
                }
            }
        }
    })?;
    Ok(())
}

/// What saving in the `separate` layout changes: modified copies of the synsets and senses
/// involved, to be written instead of the stored ones.
#[derive(Default)]
pub(crate) struct SeparateEdits {
    pub synsets: HashMap<SynsetId, Synset>,
    pub senses: HashMap<SenseId, Sense>,
    /// Relations from the common vocabulary to a name that cannot be stored on the name end
    /// (they have no inverse form), so they stay in the base files
    pub leftovers: Vec<String>,
}

fn edit_synset<'a, L: Lexicon>(
    wn: &L,
    edits: &'a mut HashMap<SynsetId, Synset>,
    id: &SynsetId,
) -> Result<&'a mut Synset> {
    if !edits.contains_key(id) {
        let synset = wn
            .synset_by_id(id)?
            .ok_or_else(|| LexiconError::SynsetIdNotFound(id.clone()))?
            .into_owned();
        edits.insert(id.clone(), synset);
    }
    Ok(edits.get_mut(id).expect("just inserted"))
}

fn edit_sense<'a, L: Lexicon>(
    wn: &L,
    edits: &'a mut HashMap<SenseId, Sense>,
    id: &SenseId,
) -> Result<&'a mut Sense> {
    if !edits.contains_key(id) {
        let (_, _, sense) = wn
            .get_sense_by_id(id)?
            .ok_or_else(|| LexiconError::SenseOrSynsetIdNotFound(id.as_str().to_string()))?;
        edits.insert(id.clone(), sense.into_owned());
    }
    Ok(edits.get_mut(id).expect("just inserted"))
}

/// Work out what saving in the `separate` layout writes differently: the relations that a
/// common synset or sense has to a name move to the name, where they are written in their
/// inverse form (a symmetric relation just gets its copy on the name end).
pub(crate) fn separate_edits<L: Lexicon>(wn: &L) -> Result<SeparateEdits> {
    let (name_synsets, name_senses) = names_of(wn)?;
    let mut edits = SeparateEdits::default();
    if name_synsets.is_empty() && name_senses.is_empty() {
        return Ok(edits);
    }

    // Synset relations
    for group in wn.synsets_iter()? {
        let (_, group) = group?;
        for s in group.iter()? {
            let (id, synset) = s?;
            if name_synsets.contains(&id) {
                continue;
            }
            for (rel, target) in synset.links_from() {
                if !name_synsets.contains(&target) {
                    continue;
                }
                let confidence = synset.rel_confidence(&rel, &target);
                let Some(inverse) = rel.inverse() else {
                    edits.leftovers.push(format!("{} {} {}", id, rel.value(), target));
                    continue;
                };
                let source = edit_synset(wn, &mut edits.synsets, &id)?;
                if let Some(targets) = source.rel_targets_mut(&rel) {
                    targets.retain(|t| *t != target);
                }
                let name = edit_synset(wn, &mut edits.synsets, &target)?;
                let field = if SynsetRelType::INVERSE_ONLY.contains(&inverse) {
                    name.inverse_rel_targets_mut(&inverse)
                } else {
                    name.rel_targets_mut(&inverse)
                };
                if let Some(field) = field {
                    if !field.iter().any(|x| *x == id) {
                        field.push_scored(id.clone(), confidence);
                    }
                }
            }
        }
    }
    // The same order whatever the order the base synsets were visited in
    for synset in edits.synsets.values_mut() {
        for rel in SynsetRelType::INVERSE_ONLY {
            if let Some(field) = synset.inverse_rel_targets_mut(&rel) {
                field.sort_by(|a, b| a.as_str().cmp(b.as_str()));
            }
        }
    }

    // Sense relations
    let mut found: Vec<(SenseId, SenseRelType, SenseId, Option<f64>)> = Vec::new();
    for bucket in wn.entries_iter()? {
        let (_, bucket) = bucket?;
        for e in bucket.entries()? {
            let (_, pos, entry) = e?;
            if is_name_pos(&pos) {
                continue;
            }
            for sense in entry.sense.iter() {
                for (rel, target) in sense.sense_links_from() {
                    let target_name = match &target {
                        UnresolvedSenseOrSynsetId::Sense(t) if name_senses.contains(t) => Some(t.clone()),
                        UnresolvedSenseOrSynsetId::Unresolved(t) if name_senses.contains(&SenseId::new(t.as_str())) => {
                            Some(SenseId::new(t.as_str()))
                        }
                        UnresolvedSenseOrSynsetId::Synset(t) if name_synsets.contains(t) => {
                            edits.leftovers.push(format!("{} {} {}", sense.id, rel.value(), t));
                            None
                        }
                        _ => None,
                    };
                    if let Some(target) = target_name {
                        let conf = sense.rel_confidence(&rel, target.as_str());
                        found.push((sense.id.clone(), rel, target, conf));
                    }
                }
            }
        }
    }
    for (source, rel, target, conf) in found {
        edit_sense(wn, &mut edits.senses, &source)?.remove_rel(&SenseOrSynsetId::Sense(target.clone()));
        let name = edit_sense(wn, &mut edits.senses, &target)?;
        if rel.is_symmetric() {
            // Its inverse is itself: the name end just has it too
            if !name.sense_links_from().iter().any(|(r, t)| *r == rel && t.as_str() == source.as_str()) {
                name.add_rel(rel.clone(), SenseOrSynsetId::Sense(source.clone()));
                if conf.is_some() {
                    name.set_rel_confidence(&rel, source.as_str(), conf);
                }
            }
        } else {
            let sources = name.inverse.entry(rel.value().to_string()).or_default();
            if !sources.iter().any(|x| *x == source) {
                sources.push_scored(source.clone(), conf);
            }
        }
    }
    for sense in edits.senses.values_mut() {
        for sources in sense.inverse.values_mut() {
            sources.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        }
    }
    Ok(edits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::names::{restrict_names, NamesMode};
    use crate::progress::NullProgress;
    use crate::validate::{validate, ValidationError};
    use crate::wordnet::{Entry, LexiconHashMapBackend};
    use std::collections::BTreeMap;

    fn synset(pos: PartOfSpeech, definition: &str, members: &[&str]) -> Synset {
        let mut ss = Synset::new(pos);
        ss.definition.push(definition.to_string());
        ss.members = members.iter().map(|m| m.to_string()).collect();
        ss
    }

    fn entry(senses: Vec<Sense>) -> Entry {
        let mut e = Entry::new();
        e.sense = senses;
        e
    }

    fn id(s: &str) -> SynsetId {
        SynsetId::new(s)
    }

    /// Common synsets and names with relations in both directions: the base synset
    /// `boulevard` has `domain_region`, `mero_member` and `also` to the name `Paris`, and its
    /// sense has `pertainym` (with a confidence), `derivation` (also on the name sense) and
    /// `agent` to the sense of `Paris`.
    fn fixture() -> LexiconHashMapBackend {
        let mut wn = LexiconHashMapBackend::new();
        wn.insert_synset("noun.Tops".into(), id("00000001-n"), synset(PartOfSpeech::n, "entity", &["entity"])).unwrap();
        let mut city = synset(PartOfSpeech::n, "a large town", &["city"]);
        city.hypernym.push(id("00000001-n"));
        wn.insert_synset("noun.location".into(), id("00000002-n"), city).unwrap();
        let mut boulevard = synset(PartOfSpeech::n, "a broad street", &["boulevard"]);
        boulevard.hypernym.push(id("00000001-n"));
        boulevard.domain_region.push(id("00000010-e"));
        boulevard.mero_member.push(id("00000010-e"));
        boulevard.also.push(id("00000010-e"));
        boulevard.attribute.push(id("00000010-e"));
        wn.insert_synset("noun.artifact".into(), id("00000003-n"), boulevard).unwrap();
        let mut paris = synset(PartOfSpeech::e, "the capital of France", &["Paris"]);
        paris.instance_hypernym.push(id("00000002-n"));
        // symmetric relations are stored at both ends
        paris.also.push(id("00000003-n"));
        paris.attribute.push(id("00000003-n"));
        wn.insert_synset("name.location".into(), id("00000010-e"), paris).unwrap();
        let mut left_bank = synset(PartOfSpeech::e, "an area of Paris", &["Left Bank"]);
        left_bank.instance_hypernym.push(id("00000002-n"));
        left_bank.mero_part.push(id("00000010-e"));
        wn.insert_synset("name.location".into(), id("00000011-e"), left_bank).unwrap();

        let sense = |key: &str, synset: &str| Sense::new(SenseId::new(key), id(synset));
        let mut add = |lemma: &str, pos: &str, senses: Vec<Sense>| {
            wn.insert_entry(lemma.to_string(), PosKey::new(pos), entry(senses)).unwrap();
        };
        add("entity", "n", vec![sense("entity%1:03:00::", "00000001-n")]);
        add("city", "n", vec![sense("city%1:15:00::", "00000002-n")]);
        let mut boulevard = sense("boulevard%1:06:00::", "00000003-n");
        boulevard.pertainym.push_scored(SenseId::new("paris%1:15:00::"), Some(0.8));
        boulevard.derivation.push(SenseId::new("paris%1:15:00::"));
        boulevard.agent.push(SenseId::new("paris%1:15:00::"));
        add("boulevard", "n", vec![boulevard]);
        let mut paris = sense("paris%1:15:00::", "00000010-e");
        paris.derivation.push(SenseId::new("boulevard%1:06:00::"));
        add("Paris", "e", vec![paris]);
        add("Left Bank", "e", vec![sense("left_bank%1:15:00::", "00000011-e")]);
        wn
    }

    fn tmp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("ewe_test_layout_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn save(wn: &LexiconHashMapBackend, dir: &Path, layout: NamesLayout) {
        wn.save_with(dir, &mut NullProgress, &StorageOptions { names_layout: layout }).unwrap();
    }

    fn load(dir: &Path) -> LexiconHashMapBackend {
        LexiconHashMapBackend::new().load(dir, &mut NullProgress).unwrap()
    }

    fn synsets(wn: &LexiconHashMapBackend) -> BTreeMap<SynsetId, Synset> {
        let mut out = BTreeMap::new();
        for group in wn.synsets_iter().unwrap() {
            for s in group.unwrap().1.iter().unwrap() {
                let (id, synset) = s.unwrap();
                out.insert(id, synset.into_owned());
            }
        }
        out
    }

    fn entries(wn: &LexiconHashMapBackend) -> BTreeMap<(String, PosKey), Entry> {
        let mut out = BTreeMap::new();
        for bucket in wn.entries_iter().unwrap() {
            for e in bucket.unwrap().1.entries().unwrap() {
                let (lemma, pos, entry) = e.unwrap();
                out.insert((lemma, pos), entry.into_owned());
            }
        }
        out
    }

    fn read_all(dir: &Path, prefix: &str, suffix: &str) -> String {
        let mut names: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .flatten()
            .map(|f| f.file_name().to_string_lossy().to_string())
            .filter(|n| n.starts_with(prefix) && n.ends_with(suffix))
            .collect();
        names.sort();
        names.iter().map(|n| std::fs::read_to_string(dir.join(n)).unwrap()).collect()
    }

    fn base_text(dir: &Path) -> String {
        // everything except the name files
        let mut text = String::new();
        for f in std::fs::read_dir(dir).unwrap().flatten() {
            let n = f.file_name().to_string_lossy().to_string();
            if n.ends_with(".yaml") && !n.starts_with("name.") && !n.ends_with("-name.yaml") {
                text += &std::fs::read_to_string(f.path()).unwrap();
            }
        }
        text
    }

    #[test]
    fn test_separate_layout_keeps_names_out_of_the_base_files() {
        let dir = tmp("base");
        save(&fixture(), &dir, NamesLayout::Separate);
        let base = base_text(&dir);
        assert!(!base.contains("-e\n") && !base.contains("-e:") && !base.contains("00000010-e"), "{base}");
        assert!(!base.contains("paris%1:15:00::"), "{base}");
        // the name side has the relations, in their inverse form
        let names = std::fs::read_to_string(dir.join("name.location.yaml")).unwrap();
        for key in ["has_domain_region:", "holo_member:", "also:"] {
            assert!(names.contains(key), "{key} in {names}");
        }
        let name_entries = read_all(&dir, "entries-", "-name.yaml");
        assert!(name_entries.contains("inverse:"), "{name_entries}");
        assert!(name_entries.contains("pertainym:") && name_entries.contains("agent:"), "{name_entries}");
        assert!(name_entries.contains("confidence: 0.8"), "{name_entries}");
        // symmetric relations just have their copy on the name end
        assert!(name_entries.contains("derivation:"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_separate_layout_loads_back_to_the_same_lexicon() {
        let original = fixture();
        let dir = tmp("roundtrip");
        save(&original, &dir, NamesLayout::Separate);
        let reloaded = load(&dir);
        for (id, ss) in synsets(&original) {
            let other = synsets(&reloaded).remove(&id).unwrap();
            assert_eq!(ss.links_from(), other.links_from(), "{id}");
        }
        assert_eq!(synsets(&reloaded), synsets(&original));
        for (k, e) in entries(&original) {
            let other = entries(&reloaded).remove(&k).unwrap();
            for (a, b) in e.sense.iter().zip(other.sense.iter()) {
                assert_eq!(a.sense_links_from(), b.sense_links_from(), "{:?}", a.id);
            }
        }
        assert_eq!(entries(&reloaded), entries(&original));
        // the inverse forms are consumed by loading
        assert!(synsets(&reloaded).values().all(|s| !s.has_inverse_rels()));
        assert!(entries(&reloaded).values().all(|e| e.sense.iter().all(|s| s.inverse.is_empty())));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_saving_again_is_stable_and_merged_layout_still_loads() {
        let original = fixture();
        let first = tmp("stable1");
        save(&original, &first, NamesLayout::Separate);
        let second = tmp("stable2");
        save(&load(&first), &second, NamesLayout::Separate);
        for f in std::fs::read_dir(&first).unwrap().flatten() {
            let name = f.file_name();
            if name.to_string_lossy().ends_with(".yaml") {
                assert_eq!(
                    std::fs::read_to_string(f.path()).unwrap(),
                    std::fs::read_to_string(second.join(&name)).unwrap(),
                    "{name:?}"
                );
            }
        }
        // the legacy layout (relations where they were made, both ends) loads unchanged,
        // and converts to the separate layout and back
        let merged = tmp("merged");
        save(&original, &merged, NamesLayout::Merged);
        assert!(base_text(&merged).contains("00000010-e"));
        let from_merged = load(&merged);
        assert_eq!(synsets(&from_merged), synsets(&original));
        assert_eq!(entries(&from_merged), entries(&original));
        for d in [first, second, merged] {
            let _ = std::fs::remove_dir_all(&d);
        }
    }

    #[test]
    fn test_the_base_files_alone_are_the_core_edition() {
        let original = fixture();
        let dir = tmp("core");
        save(&original, &dir, NamesLayout::Separate);
        for f in std::fs::read_dir(&dir).unwrap().flatten() {
            let n = f.file_name().to_string_lossy().to_string();
            if n.starts_with("name.") || n.ends_with("-name.yaml") {
                std::fs::remove_file(f.path()).unwrap();
            }
        }
        let base_only = load(&dir);
        let core = restrict_names(&original, NamesMode::Exclude).unwrap();
        assert_eq!(synsets(&base_only), synsets(&core));
        assert_eq!(entries(&base_only), entries(&core));
        let dangling = validate(&base_only, &mut NullProgress).unwrap().into_iter().any(|e| {
            matches!(
                e,
                ValidationError::SynsetRelTargetMissing { .. }
                    | ValidationError::SenseRelTargetMissing { .. }
                    | ValidationError::SenseSynsetNotExists { .. }
            )
        });
        assert!(!dangling);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_relations_without_an_inverse_form_stay_in_the_base_files() {
        let mut wn = fixture();
        // `feminine` has no inverse form, so it cannot move to the name
        wn.update_synset(&id("00000003-n"), |ss| ss.feminine.push(id("00000010-e"))).unwrap();
        let edits = separate_edits(&wn).unwrap();
        assert_eq!(edits.leftovers, vec!["00000003-n feminine 00000010-e".to_string()]);
        let dir = tmp("leftover");
        save(&wn, &dir, NamesLayout::Separate);
        assert!(base_text(&dir).contains("feminine:"));
        assert_eq!(synsets(&load(&dir)), synsets(&wn));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_storage_options_from_settings() {
        assert_eq!(StorageOptions::from_settings_str("").unwrap().names_layout, NamesLayout::Merged);
        assert_eq!(
            StorageOptions::from_settings_str("names_layout = \"separate\"\ndatabase = \"x.db\"")
                .unwrap()
                .names_layout,
            NamesLayout::Separate
        );
        assert!(StorageOptions::from_settings_str("names_layout = \"sideways\"").is_err());
    }
}
