//! Faithful port of `WNDB.write` (and its `writeData`/`writeIndex`/`writeSenseIndex`/`writeExc`
//! helpers) from `gwn-scala-api`'s `wndb.scala` onto `ewe_lib`'s data model. Every field width,
//! sort order, and escaping rule below is intended to match that source exactly - see the plan
//! at issue #28 for why (acceptance is byte-exact equivalence with the Scala tool's output).
//! Comments reference the mirrored Scala function/lines where the logic is non-obvious.

use super::tables::PRINCETON_FRAMES;
use super::{WndbExportError, WndbExportOptions};
use crate::rels::{SenseRelType, SynsetRelType};
use crate::sense_keys::{extract_lex_id, LEX_FILENUMS};
use crate::wordnet::xml::ids::escape_lemma;
use crate::wordnet::{Lexicon, PartOfSpeech, PosKey, Sense, SenseId, Synset, SynsetId};
use std::collections::HashMap;
use std::path::Path;

type Result<T> = std::result::Result<T, WndbExportError>;

/// Order matters: it determines which synsets get placeholder forward-references vs. already-
/// resolved offsets when a later POS's data references an earlier one's synset (or vice versa) -
/// mirrors the exact `Seq((adjective,"adj"),(adverb,"adv"),(noun,"noun"),(verb,"verb"))` order in
/// `WNDB.write`.
const POS_BUCKETS: [(PartOfSpeech, &str); 4] = [
    (PartOfSpeech::a, "adj"),
    (PartOfSpeech::r, "adv"),
    (PartOfSpeech::n, "noun"),
    (PartOfSpeech::v, "verb"),
];

/// A `LexicalEntry`-equivalent: one (lemma, pos) pair with its senses in declaration order.
struct AllEntry {
    lemma: String,
    pos: PosKey,
    part_of_speech: PartOfSpeech,
    senses: Vec<Sense>,
    forms: Vec<String>,
}

/// `synsetLookup`/placeholder-patch state shared across all 4 POS buffers - a forward reference
/// from one buffer to a synset not yet written (in this buffer or any other) gets an 8-ASCII-char
/// placeholder that's patched in place, everywhere it was written, once the target is finally
/// assigned its real offset. Mirrors `WNDB.write`'s `synsetLookup`/`indexes`/`replaceAll`.
struct Offsets {
    /// synset id -> (8-char code - real offset or still-a-placeholder -, its part of speech)
    lookup: HashMap<SynsetId, (String, PartOfSpeech)>,
    /// placeholder code -> positions (buffer index, byte offset) that need patching
    patch_positions: HashMap<String, Vec<(usize, usize)>>,
}

impl Offsets {
    fn new() -> Offsets {
        Offsets {
            lookup: HashMap::new(),
            patch_positions: HashMap::new(),
        }
    }

    /// Resolve `target`'s 8-char code (allocating a placeholder if it hasn't been written yet)
    /// and part of speech - mirrors `wnSynsetIdFromGlobal`. Does not itself record any patch
    /// position; call [`Offsets::record_patch`] once the code is actually written into a buffer.
    fn resolve<L: Lexicon>(&mut self, wn: &L, target: &SynsetId) -> Result<(String, PartOfSpeech)> {
        if let Some((code, pos)) = self.lookup.get(target) {
            return Ok((code.clone(), pos.clone()));
        }
        let code = format!("Z{:06}Z", self.lookup.len());
        let pos = wn
            .synset_by_id(target)?
            .ok_or_else(|| WndbExportError::MissingSynset(target.clone()))?
            .part_of_speech
            .clone();
        self.lookup.insert(target.clone(), (code.clone(), pos.clone()));
        Ok((code, pos))
    }

    /// Record that `code` (if it's a placeholder, i.e. starts with `Z`) was just written at
    /// `(buf_idx, position)`, so it can be patched later once the real offset is known.
    fn record_patch(&mut self, code: &str, buf_idx: usize, position: usize) {
        if code.starts_with('Z') {
            self.patch_positions
                .entry(code.to_string())
                .or_default()
                .push((buf_idx, position));
        }
    }

    /// Assign `synset_id`'s real offset, patching every buffer position that was written with a
    /// placeholder for it - mirrors the `if(synsetLookup.contains(id)) { updateId(...) }` /
    /// `replaceAll` dance at the top of the per-synset loop in `writeData`.
    fn assign(&mut self, synset_id: &SynsetId, pos: &PartOfSpeech, code: String, bufs: &mut [Vec<u8>; 4]) {
        if let Some((old_code, _)) = self.lookup.get(synset_id) {
            if old_code.starts_with('Z') {
                if let Some(positions) = self.patch_positions.remove(old_code) {
                    for (buf_idx, position) in positions {
                        bufs[buf_idx][position..position + 8].copy_from_slice(code.as_bytes());
                    }
                }
            }
        }
        self.lookup.insert(synset_id.clone(), (code, pos.clone()));
    }
}

fn wndb_pos_char_no_satellite(pos: &PartOfSpeech) -> &'static str {
    if *pos == PartOfSpeech::s {
        "a"
    } else {
        pos.value()
    }
}

fn lex_name_number(lexname: &str, extra: &mut HashMap<String, usize>) -> usize {
    if let Some(n) = LEX_FILENUMS.get(lexname) {
        return *n;
    }
    if let Some(n) = extra.get(lexname) {
        return *n;
    }
    let n = 45 + extra.len();
    extra.insert(lexname.to_string(), n);
    eprintln!("LexName not found: {lexname}");
    n
}

/// The last 2 bytes of a sense key string (ASCII, always at least 2 bytes long: even the
/// shortest form `lemma%1:00:00::` ends in `::`) - mirrors `sense.id.takeRight(2)`, the
/// (usually-a-no-op, since most sense keys end `::`) stable-sort key `writeData` uses when
/// computing a synset's per-pointer `srcIdx`/`trgIdx` fields.
/// Strip a trailing `-{pos_char}` suffix from a synset id string - used to compare
/// `--sense-orders` CSV entries (which carry their own pos suffix) against synset ids regardless
/// of pos suffix.
fn no_pos(id: &str) -> &str {
    id.rfind('-').map(|p| &id[..p]).unwrap_or(id)
}

fn last2(s: &str) -> &str {
    let len = s.len();
    if len <= 2 {
        s
    } else {
        &s[len - 2..]
    }
}

/// 0-based position of `sense_id` within `synset_id`'s members, ordered the same way `writeData`
/// orders them for `srcIdx`/`trgIdx` purposes (`entries.sortBy(_._2.id.takeRight(2))` - NOT the
/// synset's own `members` declaration order, which is only used for the synset line's own member
/// list).
fn member_index(
    entries_for_synset: &HashMap<SynsetId, Vec<(usize, SenseId)>>,
    synset_id: &SynsetId,
    sense_id: &SenseId,
) -> usize {
    let mut v = entries_for_synset.get(synset_id).cloned().unwrap_or_default();
    v.sort_by(|(_, a), (_, b)| last2(a.as_str()).cmp(last2(b.as_str())));
    v.iter().position(|(_, s)| s == sense_id).unwrap_or(0)
}

fn collect_all_entries<L: Lexicon>(wn: &L) -> Result<Vec<AllEntry>> {
    let mut entries = Vec::new();
    for entry in wn.entries()? {
        let (lemma, pos, entry) = entry?;
        if let Some(part_of_speech) = pos.to_part_of_speech() {
            entries.push(AllEntry {
                lemma,
                pos,
                part_of_speech,
                senses: entry.sense.clone(),
                forms: entry.form.clone(),
            });
        }
    }
    // Mirrors the real XML document's `LexicalEntry` order (see `xml/writer.rs`'s
    // `entries_sorted.sort_by(|a, b| a.0.cmp(&b.0))`), which is what `lexicon.entries` iterates
    // in once gwn-scala-api parses that document back in.
    entries.sort_by(|a, b| {
        let ka = format!("{}-{}", escape_lemma(&a.lemma), a.pos.as_str());
        let kb = format!("{}-{}", escape_lemma(&b.lemma), b.pos.as_str());
        ka.cmp(&kb)
    });
    Ok(entries)
}

/// `entries_for_synset[synset_id]` is the `(entry index into `all_entries`, sense id)` pairs
/// naming that synset, in `all_entries`' document order and (within one entry) each entry's own
/// sense declaration order - mirrors `entriesForSynset` in `WNDB.write`.
fn index_entries_by_synset(all_entries: &[AllEntry]) -> HashMap<SynsetId, Vec<(usize, SenseId)>> {
    let mut map: HashMap<SynsetId, Vec<(usize, SenseId)>> = HashMap::new();
    for (i, entry) in all_entries.iter().enumerate() {
        for sense in &entry.senses {
            map.entry(sense.synset.clone()).or_default().push((i, sense.id.clone()));
        }
    }
    map
}

/// Every relation type storable directly on `Synset`, excluding `hypernym` (handled separately,
/// first, by the caller) - mirrors reading `synset.synsetRelations` off the parsed
/// `LexicalResource`, restricted to the forward-declared half of it (the inverse half comes from
/// `Lexicon::links_to` in the caller, same split `MemberSynset::from_synset` makes).
fn forward_synset_relations(synset: &Synset) -> Vec<(SynsetRelType, SynsetId)> {
    let mut out = Vec::new();
    macro_rules! rel {
        ($field:ident, $rel_type:expr) => {
            for target in &synset.$field {
                out.push(($rel_type, target.clone()));
            }
        };
    }
    rel!(also, SynsetRelType::Also);
    rel!(attribute, SynsetRelType::Attribute);
    rel!(causes, SynsetRelType::Causes);
    rel!(domain_region, SynsetRelType::DomainRegion);
    rel!(domain_topic, SynsetRelType::DomainTopic);
    rel!(exemplifies, SynsetRelType::Exemplifies);
    rel!(entails, SynsetRelType::Entails);
    rel!(instance_hypernym, SynsetRelType::InstanceHypernym);
    rel!(mero_location, SynsetRelType::MeroLocation);
    rel!(mero_member, SynsetRelType::MeroMember);
    rel!(mero_part, SynsetRelType::MeroPart);
    rel!(mero_portion, SynsetRelType::MeroPortion);
    rel!(mero_substance, SynsetRelType::MeroSubstance);
    rel!(meronym, SynsetRelType::Meronym);
    rel!(similar, SynsetRelType::Similar);
    rel!(feminine, SynsetRelType::Feminine);
    rel!(masculine, SynsetRelType::Masculine);
    rel!(other, SynsetRelType::Other);
    out
}

/// Every sense-targeted relation on `sense` - both directly stored (forward) and inverse-
/// computed via `Lexicon::sense_links_to_get` (only `domain_topic`/`domain_region`/`exemplifies`
/// have a defined inverse - see `SenseRelType::inverse`), restricted to relation kinds that
/// target a specific sense (a synset-targeted `domain_topic`/`domain_region`/`exemplifies`/
/// `other` can't be expressed as a WNDB sense-level pointer, which always needs a source/target
/// sense-index pair).
fn sense_wndb_relations<L: Lexicon>(wn: &L, sense: &Sense) -> Result<Vec<(SenseRelType, SenseId)>> {
    let mut out = Vec::new();
    macro_rules! rel {
        ($field:ident, $rel_type:expr) => {
            for target in &sense.$field {
                out.push(($rel_type, target.clone()));
            }
        };
    }
    rel!(antonym, SenseRelType::Antonym);
    rel!(also, SenseRelType::Also);
    rel!(participle, SenseRelType::Participle);
    rel!(pertainym, SenseRelType::Pertainym);
    rel!(derivation, SenseRelType::Derivation);
    rel!(similar, SenseRelType::Similar);
    rel!(agent, SenseRelType::Agent);
    rel!(material, SenseRelType::Material);
    rel!(event, SenseRelType::Event);
    rel!(instrument, SenseRelType::Instrument);
    rel!(location, SenseRelType::Location);
    rel!(by_means_of, SenseRelType::ByMeansOf);
    rel!(undergoer, SenseRelType::Undergoer);
    rel!(property, SenseRelType::Property);
    rel!(result, SenseRelType::Result);
    rel!(state, SenseRelType::State);
    rel!(uses, SenseRelType::Uses);
    rel!(destination, SenseRelType::Destination);
    rel!(body_part, SenseRelType::BodyPart);
    rel!(vehicle, SenseRelType::Vehicle);
    for target in &sense.domain_topic {
        if let crate::wordnet::UnresolvedSenseOrSynsetId::Sense(id) = target {
            out.push((SenseRelType::DomainTopic, id.clone()));
        }
    }
    for target in &sense.domain_region {
        if let crate::wordnet::UnresolvedSenseOrSynsetId::Sense(id) = target {
            out.push((SenseRelType::DomainRegion, id.clone()));
        }
    }
    for target in &sense.exemplifies {
        if let crate::wordnet::UnresolvedSenseOrSynsetId::Sense(id) = target {
            out.push((SenseRelType::Exemplifies, id.clone()));
        }
    }

    // Self-inverse relation kinds (antonym/also/similar - `.inverse()` maps them to themselves)
    // are always authored explicitly on *both* sides directly in OEWN's YAML convention, unlike
    // the genuinely-canonical-direction-only kinds (domain_topic/domain_region/exemplifies).
    // Computing a backlink for a self-inverse kind would double up a relation the forward loop
    // above (`rel!(antonym, ...)`/etc.) already emitted once - confirmed empirically: without
    // this guard, "able"'s antonym pointer to "unable" appeared twice in `data.adj`.
    if let Some(backlinks) = wn.sense_links_to_get(&sense.id)? {
        for (rel_type, target) in backlinks.iter() {
            if let Some(inv) = rel_type.inverse() {
                if inv != *rel_type {
                    out.push((inv, target.clone()));
                }
            }
        }
    }
    Ok(out)
}

pub fn write_wndb<L: Lexicon>(wn: &L, out_dir: &Path, options: &WndbExportOptions) -> Result<()> {
    std::fs::create_dir_all(out_dir)?;
    let header: Vec<u8> = match &options.license_file {
        Some(p) => std::fs::read(p)?,
        None => Vec::new(),
    };
    let sense_orders = super::sense_orders::load(options.sense_orders.as_deref())?;

    let all_entries = collect_all_entries(wn)?;
    let entries_for_synset = index_entries_by_synset(&all_entries);

    write_exc_files(&all_entries, out_dir)?;

    let mut bufs: [Vec<u8>; 4] = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut offsets = Offsets::new();
    let mut extra_lexnames: HashMap<String, usize> = HashMap::new();

    for (buf_idx, (pos, _)) in POS_BUCKETS.iter().enumerate() {
        bufs[buf_idx].extend_from_slice(&header);
        write_data(
            wn,
            pos,
            buf_idx,
            &mut bufs,
            &mut offsets,
            &mut extra_lexnames,
            &all_entries,
            &entries_for_synset,
        )?;
    }

    for (buf_idx, (_, name)) in POS_BUCKETS.iter().enumerate() {
        // One extra trailing newline beyond the last synset's own "  \n" terminator - mirrors
        // `out.println(stringBuilders(posShort)._1)`, which `println`s the *whole* accumulated
        // string (itself already ending in "\n") rather than `print`ing it.
        let mut content = bufs[buf_idx].clone();
        content.push(b'\n');
        std::fs::write(out_dir.join(format!("data.{name}")), &content)?;
    }

    for (pos, name) in POS_BUCKETS.iter() {
        let mut out: Vec<u8> = Vec::new();
        out.extend_from_slice(&header);
        write_index(wn, pos, &all_entries, &offsets, &sense_orders, &mut out)?;
        std::fs::write(out_dir.join(format!("index.{name}")), out)?;
    }

    write_sense_index(&all_entries, &offsets, out_dir)?;

    Ok(())
}

/// One `data.{pos}` file - mirrors `writeData` (`wndb.scala:745-913`).
fn write_data<L: Lexicon>(
    wn: &L,
    bucket_pos: &PartOfSpeech,
    buf_idx: usize,
    bufs: &mut [Vec<u8>; 4],
    offsets: &mut Offsets,
    extra_lexnames: &mut HashMap<String, usize>,
    all_entries: &[AllEntry],
    entries_for_synset: &HashMap<SynsetId, Vec<(usize, SenseId)>>,
) -> Result<()> {
    let mut synsets: Vec<(SynsetId, Synset)> = Vec::new();
    for entry in wn.synsets()? {
        let (id, synset) = entry?;
        if bucket_pos.equals_pos(&synset.part_of_speech) {
            synsets.push((id, synset.into_owned()));
        }
    }
    synsets.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));

    for (synset_id, synset) in &synsets {
        let code = format!("{:08}", bufs[buf_idx].len());
        offsets.assign(synset_id, &synset.part_of_speech, code.clone(), bufs);

        let lexname = wn.lex_name_for(synset_id)?.unwrap_or_default();
        let lexnum = lex_name_number(&lexname, extra_lexnames);

        {
            let buf = &mut bufs[buf_idx];
            buf.extend_from_slice(code.as_bytes());
            buf.extend_from_slice(format!(" {:02} ", lexnum).as_bytes());
            buf.extend_from_slice(synset.part_of_speech.value().as_bytes());
        }

        let members = entries_for_synset.get(synset_id).cloned().unwrap_or_default();
        let mut members_sorted = members.clone();
        if !synset.members.is_empty() {
            members_sorted.sort_by_key(|(i, _)| {
                synset
                    .members
                    .iter()
                    .position(|m| *m == all_entries[*i].lemma)
                    .unwrap_or(usize::MAX)
            });
        } else {
            members_sorted.sort_by(|(_, a), (_, b)| last2(a.as_str()).cmp(last2(b.as_str())));
        }

        {
            let buf = &mut bufs[buf_idx];
            buf.extend_from_slice(format!(" {:02x} ", members_sorted.len()).as_bytes());
            for (entry_idx, sense_id) in &members_sorted {
                let entry = &all_entries[*entry_idx];
                let sense = entry.senses.iter().find(|s| s.id == *sense_id).expect("sense in entry");
                buf.extend_from_slice(entry.lemma.replace(' ', "_").as_bytes());
                if let Some(adjposition) = &sense.adjposition {
                    buf.extend_from_slice(format!("({adjposition})").as_bytes());
                }
                let lex_id = extract_lex_id(&sense.id);
                buf.extend_from_slice(format!(" {:x} ", lex_id).as_bytes());
            }
        }

        // Relation pointers: hypernym first (synset-level), then sense-level relations in
        // document-entry order, then remaining synset-level relations sorted by target id -
        // exactly the 3-block order `writeData` emits them in. Collected first as
        // (symbol, target_synset_id, target_pos_char, idx_field) so the total count is known
        // before any pointer bytes are written (the `%03d` count field comes first on the line).
        let mut pointers: Vec<(&'static str, SynsetId, String, String)> = Vec::new();

        for target in &synset.hypernym {
            let (_, _) = offsets.resolve(wn, target)?; // ensure a code/placeholder exists
            let target_pos_char = wn
                .synset_by_id(target)?
                .map(|s| s.part_of_speech.value().to_string())
                .unwrap_or_else(|| "n".to_string());
            pointers.push(("@", target.clone(), target_pos_char, "0000".to_string()));
        }

        for (entry_idx, sense_id) in &members {
            let entry = &all_entries[*entry_idx];
            let sense = entry.senses.iter().find(|s| s.id == *sense_id).expect("sense in entry");
            for (rel_type, target_sense_id) in sense_wndb_relations(wn, sense)? {
                let Some(symbol) = rel_type.wndb_pointer(bucket_pos) else {
                    continue;
                };
                let Some((_, _, target_sense)) = wn.get_sense_by_id(&target_sense_id)? else {
                    continue;
                };
                let target_synset = target_sense.synset.clone();
                let target_pos_char = wn
                    .synset_by_id(&target_synset)?
                    .map(|s| wndb_pos_char_no_satellite(&s.part_of_speech).to_string())
                    .unwrap_or_else(|| "n".to_string());
                let src_idx = member_index(entries_for_synset, synset_id, sense_id);
                let trg_idx = member_index(entries_for_synset, &target_synset, &target_sense_id);
                let (_, _) = offsets.resolve(wn, &target_synset)?;
                pointers.push((
                    symbol,
                    target_synset,
                    target_pos_char,
                    format!("{:02x}{:02x}", src_idx + 1, trg_idx + 1),
                ));
            }
        }

        let mut remaining: Vec<(SynsetRelType, SynsetId)> = forward_synset_relations(synset);
        for (rel_type, target) in wn.links_to(synset_id)? {
            if let Some(inv) = rel_type.inverse() {
                // See the matching guard in `sense_wndb_relations` - self-inverse kinds
                // (also/similar/attribute) are already stored on both sides directly.
                if inv != rel_type {
                    remaining.push((inv, target));
                }
            }
        }
        remaining.sort_by(|a, b| a.1.as_str().cmp(b.1.as_str()));
        for (rel_type, target) in &remaining {
            let Some(symbol) = rel_type.wndb_pointer(bucket_pos) else {
                continue;
            };
            let target_pos_char = wn
                .synset_by_id(target)?
                .map(|s| wndb_pos_char_no_satellite(&s.part_of_speech).to_string())
                .unwrap_or_else(|| "n".to_string());
            let (_, _) = offsets.resolve(wn, target)?;
            pointers.push((symbol, target.clone(), target_pos_char, "0000".to_string()));
        }

        {
            let buf = &mut bufs[buf_idx];
            buf.extend_from_slice(format!("{:03} ", pointers.len()).as_bytes());
        }
        for (symbol, target, target_pos_char, idx_field) in &pointers {
            let (target_code, _) = offsets.resolve(wn, target)?;
            let buf = &mut bufs[buf_idx];
            buf.extend_from_slice(symbol.as_bytes());
            buf.push(b' ');
            let pos_in_buf = buf.len();
            buf.extend_from_slice(target_code.as_bytes());
            offsets.record_patch(&target_code, buf_idx, pos_in_buf);
            let buf = &mut bufs[buf_idx];
            buf.push(b' ');
            buf.extend_from_slice(target_pos_char.as_bytes());
            buf.push(b' ');
            buf.extend_from_slice(idx_field.as_bytes());
            buf.push(b' ');
        }

        write_frames(wn, bufs, buf_idx, &code, synset, &members, all_entries)?;

        {
            let buf = &mut bufs[buf_idx];
            if let Some(defn) = synset.definition.first() {
                buf.extend_from_slice(b"| ");
                buf.extend_from_slice(defn.replace('\u{a0}', " ").as_bytes());
            }
            for example in &synset.example {
                let text = example.text.replace('\u{a0}', " ");
                if text.starts_with('"') {
                    buf.extend_from_slice(format!("; {text}").as_bytes());
                } else {
                    buf.extend_from_slice(format!("; \"{text}\"").as_bytes());
                    if let Some(source) = &example.source {
                        if !source.starts_with("http") {
                            buf.extend_from_slice(format!(" - {source}").as_bytes());
                        }
                    }
                }
            }
            buf.extend_from_slice(b"  \n");
        }
    }

    Ok(())
}

/// Verb subcategorization-frame refs for one synset - mirrors the `frames2`/`frameRefs`
/// construction in `writeData` (`wndb.scala:848-896`), restricted to the subcat-id-reference
/// convention (`Sense::subcat` + `Lexicon::frames_get`) `ewe_lib` actually uses; the alternate
/// inline-per-entry `SyntacticBehaviour` encoding `gwn-scala-api`'s own model can also represent
/// has no equivalent here since nothing in `ewe_lib` ever produces it.
fn write_frames<L: Lexicon>(
    wn: &L,
    bufs: &mut [Vec<u8>; 4],
    buf_idx: usize,
    code: &str,
    synset: &Synset,
    members: &[(usize, SenseId)],
    all_entries: &[AllEntry],
) -> Result<()> {
    let frame_table = wn.frames_get()?;
    let frame_lookup: HashMap<&str, &str> =
        frame_table.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();

    // description -> sense ids using it, in document-entry order.
    let mut by_description: Vec<(String, Vec<SenseId>)> = Vec::new();
    for (entry_idx, sense_id) in members {
        let entry = &all_entries[*entry_idx];
        let sense = entry.senses.iter().find(|s| s.id == *sense_id).expect("sense in entry");
        for subcat in &sense.subcat {
            if let Some(description) = frame_lookup.get(subcat.as_str()) {
                match by_description.iter_mut().find(|(d, _)| d == description) {
                    Some((_, senses)) => senses.push(sense_id.clone()),
                    None => by_description.push((description.to_string(), vec![sense_id.clone()])),
                }
            }
        }
    }

    let mut frame_refs: Vec<(u32, usize)> = Vec::new();
    for (description, senses) in &by_description {
        let frame_id = *PRINCETON_FRAMES.get(description.as_str()).unwrap_or(&0);
        let applies_to_all = members.iter().all(|(_, s)| senses.contains(s));
        if applies_to_all {
            frame_refs.push((frame_id, 0));
        } else {
            for (entry_idx, sense_id) in members {
                if senses.contains(sense_id) {
                    // `sense.id.takeRight(2)` numeric-suffix special case in the Scala source is
                    // specific to gwn-scala-api's own XML-mapped `Sense/@id` form and never
                    // applies to a real WordNet sense key (which always ends `::` or `:NN`, never
                    // bare digits) - so this always takes the `synset.members.indexOf` fallback.
                    let entry = &all_entries[*entry_idx];
                    if let Some(idx) = synset.members.iter().position(|m| *m == entry.lemma) {
                        frame_refs.push((frame_id, idx));
                    }
                }
            }
        }
    }

    // Two-tier order confirmed against real Princeton `grind` output at /usr/share/wordnet:
    // w_num=0 entries first (ascending frame id), then w_num!=0 entries grouped by w_num
    // descending, ascending frame id within each group.
    frame_refs.sort_by_key(|(frame_id, w_num)| {
        if *w_num == 0 {
            (0u32, 0u32, *frame_id)
        } else {
            (1u32, u32::MAX - *w_num as u32, *frame_id)
        }
    });

    // HACK: 3 known Princeton-source-data inconsistencies patched verbatim by the Scala source
    // (`wndb.scala:880-890`) - reproduced here in case the same offsets recur, since offset
    // assignment is deterministic given the same synset set/order.
    if !frame_refs.is_empty() {
        match code {
            "02599707" => {
                frame_refs.push((2, 3));
                frame_refs.push((2, 4));
            }
            "02592711" => {
                frame_refs.retain(|r| *r != (2, 0));
                frame_refs.push((2, 1));
                frame_refs.push((2, 2));
            }
            "02741772" => {
                frame_refs.retain(|r| *r != (35, 0));
                frame_refs.push((35, 2));
                frame_refs.push((35, 1));
            }
            _ => {}
        }
    }

    // The count field is emitted whenever *any* subcat-derived frame ref existed before the
    // `< 36` filter - even if every single one gets filtered out, leaving a bare "00 " with no
    // "+" entries after it. Mirrors `if(!frameRefs.isEmpty) { ...; data ++= "%02d " format
    // (frameRefs2.size); ... }` - the emptiness check is on the *pre-filter* list, the count
    // printed is the *post-filter* size. Confirmed empirically against a "via-for" (frame 36,
    // always filtered) sense, whose real `data.verb` line still carries a trailing "00".
    let had_any_frame_ref = !frame_refs.is_empty();
    let frame_refs: Vec<(u32, usize)> = frame_refs.into_iter().filter(|(id, _)| *id < 36).collect();
    if had_any_frame_ref {
        let buf = &mut bufs[buf_idx];
        buf.extend_from_slice(format!("{:02} ", frame_refs.len()).as_bytes());
        for (frame_id, w_num) in &frame_refs {
            buf.extend_from_slice(format!("+ {:02} {:02x} ", frame_id, w_num).as_bytes());
        }
    }

    Ok(())
}

/// The 4 `{pos}.exc` irregular-inflection files - mirrors `writeExc` (`wndb.scala:574-592`). No
/// header is written to these (unlike `data.*`/`index.*`) - matches the Scala source, which never
/// prepends `PRINCETON_HEADER`/the license file to `.exc` output.
fn write_exc_files(all_entries: &[AllEntry], out_dir: &Path) -> Result<()> {
    for (pos, name) in POS_BUCKETS.iter() {
        let mut pairs: Vec<(String, String)> = Vec::new(); // (irregular_form, base_lemma)
        for entry in all_entries {
            if !pos.equals_pos(&entry.part_of_speech) {
                continue;
            }
            for form in &entry.forms {
                if form.contains(' ') {
                    return Err(WndbExportError::FormWithSpace(form.clone()));
                }
                pairs.push((form.clone(), entry.lemma.clone()));
            }
        }
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        let mut out = String::new();
        for (form, lemma) in &pairs {
            out.push_str(form);
            out.push(' ');
            out.push_str(lemma);
            out.push('\n');
        }
        std::fs::write(out_dir.join(format!("{name}.exc")), out)?;
    }
    Ok(())
}

fn collapse_pointer_symbol(symbol: &str) -> String {
    match symbol {
        ";u" => "; ".to_string(),
        "-u" => "- ".to_string(),
        ";c" => "; ".to_string(),
        "-c" => "- ".to_string(),
        ";r" => "; ".to_string(),
        // Faithful reproduction of an upstream quirk in `WNDB.writeIndex`'s collapse rules: every
        // other "has_*"-style inverse collapses to "- ", but `-r` (has_domain_region) collapses
        // to "; " instead - almost certainly a copy-paste bug in `wndb.scala`, but byte-exactness
        // means keeping it.
        "-r" => "; ".to_string(),
        "@i" => "@ ".to_string(),
        "~i" => "~ ".to_string(),
        other => format!("{other} "),
    }
}

/// Every WNDB-representable relation type touching `synset` (forward-declared plus inverse-
/// computed via `Lexicon::links_to`), including `hypernym` - used only for the pointer-symbol
/// *set* in `writeIndex` (unlike `write_data`, order doesn't matter here, so `hypernym` doesn't
/// need to be split out separately).
fn all_synset_rel_types<L: Lexicon>(wn: &L, synset_id: &SynsetId, synset: &Synset) -> Result<Vec<SynsetRelType>> {
    // Unlike `write_data`'s pointer *order* (which deliberately puts hypernym first, matching
    // `writeData`'s own explicit hypernym-first block), the pointer-*symbol-set* computation here
    // just needs `synset.synsetRelations`' natural field declaration order - putting hypernym
    // first here was wrong (confirmed empirically: caused "'hood"'s `@ ; ` vs the real `; @ `).
    let mut out: Vec<SynsetRelType> = forward_synset_relations(synset).into_iter().map(|(t, _)| t).collect();
    // `forward_synset_relations` omits `hypernym` (write_data's other caller needs it excluded) -
    // splice it back in at its natural struct-field position (between `entails` and
    // `instance_hypernym` - see `Synset`'s field order in `synset.rs`).
    let insert_at = out
        .iter()
        .position(|t| *t == SynsetRelType::InstanceHypernym)
        .unwrap_or(out.len());
    for _ in &synset.hypernym {
        out.insert(insert_at, SynsetRelType::Hypernym);
    }
    for (rel_type, _) in wn.links_to(synset_id)? {
        if let Some(inv) = rel_type.inverse() {
            if inv != rel_type {
                out.push(inv);
            }
        }
    }
    Ok(out)
}

/// One `index.{pos}` file - mirrors `writeIndex` (`wndb.scala:939-1011`).
fn write_index<L: Lexicon>(
    wn: &L,
    bucket_pos: &PartOfSpeech,
    all_entries: &[AllEntry],
    offsets: &Offsets,
    sense_orders: &super::sense_orders::SenseOrders,
    out: &mut Vec<u8>,
) -> Result<()> {
    // Group by lowercased-and-underscored lemma, sorted by that key - matches
    // `.groupBy(...).toSeq.sortBy(_._1)`. Each group's entries preserve `all_entries`' (document)
    // order, since we're scanning it in order.
    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
    for (i, entry) in all_entries.iter().enumerate() {
        if !bucket_pos.equals_pos(&entry.part_of_speech) {
            continue;
        }
        let key = entry.lemma.replace(' ', "_").to_lowercase();
        match groups.iter_mut().find(|(k, _)| *k == key) {
            Some((_, v)) => v.push(i),
            None => groups.push((key, vec![i])),
        }
    }
    groups.sort_by(|a, b| a.0.cmp(&b.0));

    for (lemma_lower, entry_idxs) in &groups {
        let mut synset_ids: Vec<SynsetId> = Vec::new();
        let mut ptrs: Vec<String> = Vec::new();
        for &i in entry_idxs {
            let entry = &all_entries[i];
            for sense in &entry.senses {
                synset_ids.push(sense.synset.clone());
            }
        }

        // Sense-order override, when `--sense-orders` covers this lemma+pos - mirrors the
        // `senseOrders.get(...)` block in `writeIndex`.
        if let Some(csv_order) = sense_orders.get(lemma_lower) {
            if csv_order.first().is_some_and(|id| id.as_str().ends_with(bucket_pos.value())) {
                let csv_no_pos: Vec<&str> = csv_order.iter().map(|id| no_pos(id.as_str())).collect();
                let cur_no_pos: Vec<&str> = synset_ids.iter().map(|id| no_pos(id.as_str())).collect();
                let mut csv_set: Vec<&str> = csv_no_pos.clone();
                csv_set.sort();
                csv_set.dedup();
                let mut cur_set: Vec<&str> = cur_no_pos.clone();
                cur_set.sort();
                cur_set.dedup();
                if csv_set == cur_set {
                    let mut with_key: Vec<(usize, SynsetId)> = synset_ids
                        .into_iter()
                        .map(|id| {
                            let key = no_pos(id.as_str());
                            let pos = csv_no_pos.iter().position(|k| *k == key).unwrap_or(usize::MAX);
                            (pos, id)
                        })
                        .collect();
                    with_key.sort_by_key(|(pos, _)| *pos);
                    synset_ids = with_key.into_iter().map(|(_, id)| id).collect();
                } else {
                    eprintln!(
                        "{lemma_lower} has different set of keys to ordering ([{:?}] vs. [{:?}])",
                        csv_order, cur_no_pos
                    );
                }
            }
        }

        // The pointer-symbol set for this group is the union, over every sense of every entry
        // in it, of that sense's own relation types AND its synset's relation types - mirrors
        // `sense.senseRelations.map(_.relType) ++ lexicon.synsetsById(sense.synsetRef)
        // .synsetRelations.map(_.relType)`.
        let mut seen_synsets: Vec<SynsetId> = Vec::new();
        for &i in entry_idxs {
            let entry = &all_entries[i];
            for sense in &entry.senses {
                for rel_type in sense_wndb_relation_types(sense) {
                    if let Some(symbol) = rel_type.wndb_pointer(bucket_pos) {
                        let collapsed = collapse_pointer_symbol(symbol);
                        if !ptrs.contains(&collapsed) {
                            ptrs.push(collapsed);
                        }
                    }
                }
                if !seen_synsets.contains(&sense.synset) {
                    seen_synsets.push(sense.synset.clone());
                }
            }
        }
        for synset_id in &seen_synsets {
            if let Some(synset) = wn.synset_by_id(synset_id)? {
                for rel_type in all_synset_rel_types(wn, synset_id, &synset)? {
                    if let Some(symbol) = rel_type.wndb_pointer(bucket_pos) {
                        let collapsed = collapse_pointer_symbol(symbol);
                        if !ptrs.contains(&collapsed) {
                            ptrs.push(collapsed);
                        }
                    }
                }
            }
        }

        let synset_cnt = synset_ids.len();
        let synset_codes: Vec<String> = synset_ids
            .iter()
            .map(|id| {
                offsets
                    .lookup
                    .get(id)
                    .map(|(code, _)| code.clone())
                    .unwrap_or_else(|| "00000000".to_string())
            })
            .collect();

        let mut line = String::new();
        line.push_str(lemma_lower);
        line.push(' ');
        line.push_str(bucket_pos.value());
        line.push(' ');
        line.push_str(&synset_cnt.to_string());
        line.push(' ');
        line.push_str(&ptrs.len().to_string());
        line.push(' ');
        for p in &ptrs {
            line.push_str(p);
        }
        line.push_str(&synset_cnt.to_string());
        line.push_str(" 0 ");
        line.push_str(&synset_codes.join(" "));
        line.push_str("  \n");
        out.extend_from_slice(line.as_bytes());
    }

    Ok(())
}

fn sense_wndb_relation_types(sense: &Sense) -> Vec<SenseRelType> {
    let mut out = Vec::new();
    macro_rules! rel {
        ($field:ident, $rel_type:expr) => {
            for _ in &sense.$field {
                out.push($rel_type);
            }
        };
    }
    rel!(antonym, SenseRelType::Antonym);
    rel!(also, SenseRelType::Also);
    rel!(participle, SenseRelType::Participle);
    rel!(pertainym, SenseRelType::Pertainym);
    rel!(derivation, SenseRelType::Derivation);
    rel!(similar, SenseRelType::Similar);
    rel!(agent, SenseRelType::Agent);
    rel!(material, SenseRelType::Material);
    rel!(event, SenseRelType::Event);
    rel!(instrument, SenseRelType::Instrument);
    rel!(location, SenseRelType::Location);
    rel!(by_means_of, SenseRelType::ByMeansOf);
    rel!(undergoer, SenseRelType::Undergoer);
    rel!(property, SenseRelType::Property);
    rel!(result, SenseRelType::Result);
    rel!(state, SenseRelType::State);
    rel!(uses, SenseRelType::Uses);
    rel!(destination, SenseRelType::Destination);
    rel!(body_part, SenseRelType::BodyPart);
    rel!(vehicle, SenseRelType::Vehicle);
    for _ in &sense.domain_topic {
        out.push(SenseRelType::DomainTopic);
    }
    for _ in &sense.domain_region {
        out.push(SenseRelType::DomainRegion);
    }
    for _ in &sense.exemplifies {
        out.push(SenseRelType::Exemplifies);
    }
    out
}

/// `index.sense` - mirrors `writeSenseIndex` (`wndb.scala:1013-1041`). No header. Runs once,
/// globally, after every `data.*` buffer is complete (needs every synset's final offset).
/// `index.sense`'s sense-key column is *not* simply `sense.id` verbatim: real `english-wordnet-
/// 2025.xml` never sets `Sense/@dc:identifier` (confirmed: zero occurrences in the file), so
/// `WNDB.write` always falls back to its own `unmapSenseKey` to reconstruct a sense key from the
/// escaped `Sense/@id` - and that function is *not* a full inverse of the escaping scheme
/// `ids::map_sense_key` (ported from OEWN's own `sense_keys.py`) implements: it only undoes
/// `-ap-`/`-sl-`/`-ex-`/`-cm-`/`-cl-`, never the hyphen-doubling or any of the other escapes. So
/// a lemma with a literal hyphen (e.g. "10-membered") round-trips through the real tool as
/// "10--membered" in `index.sense` - confirmed against `oewn2025/index.sense` directly. Faithfully
/// reproducing that (rather than "fixing" it) is what byte-exactness requires: re-escape via
/// `ids::map_sense_key` the same way a WN-LMF document would encode this sense key, then run
/// `WNDB.unmapSenseKey`'s exact (incomplete) logic on that - mirrors `wndb.scala:1043-1057`.
fn scala_sense_key(sense_key: &str) -> String {
    let escaped = crate::wordnet::xml::ids::map_sense_key(sense_key, "x");
    if let Some(dunder) = escaped.find("__") {
        let e0 = &escaped[..dunder];
        let first_dash = escaped.find('-').unwrap_or(0);
        let l = if first_dash + 1 <= e0.len() {
            &e0[first_dash + 1..]
        } else {
            e0
        };
        let mut r = escaped[dunder + 2..].to_string();
        if escaped.ends_with("__") {
            r.push_str("__");
        }
        let l = l.replace("-ap-", "'").replace("-sl-", "/").replace("-ex-", "!").replace("-cm-", ",").replace("-cl-", ":");
        let r = r.replace('_', ":").replace('.', ":").replace("-sp-", "_");
        format!("{l}%{r}")
    } else {
        let rest = escaped.get(4..).unwrap_or("");
        rest.replace("__", "%").replace("-ap-", "'").replace("-sl-", "/").replace("-ex-", "!").replace("-cm-", ",").replace("-cl-", ":")
    }
}

fn write_sense_index(all_entries: &[AllEntry], offsets: &Offsets, out_dir: &Path) -> Result<()> {
    // Group by (lowercase lemma, part-of-speech char) preserving document order within a group -
    // mirrors `entriesByLowercaseLemma`. Iteration order across groups doesn't matter, since the
    // output is fully re-sorted as plain strings below.
    let mut groups: HashMap<(String, &'static str), Vec<usize>> = HashMap::new();
    for (i, entry) in all_entries.iter().enumerate() {
        let key = (entry.lemma.to_lowercase(), entry.part_of_speech.value());
        groups.entry(key).or_default().push(i);
    }

    let mut lines: Vec<String> = Vec::new();
    for entry_idxs in groups.values() {
        let mut i = 1usize;
        for &idx in entry_idxs {
            let entry = &all_entries[idx];
            for sense in &entry.senses {
                let code = offsets
                    .lookup
                    .get(&sense.synset)
                    .map(|(code, _)| code.clone())
                    .unwrap_or_else(|| "00000000".to_string());
                lines.push(format!("{} {} {} 0", scala_sense_key(sense.id.as_str()), code, i));
                i += 1;
            }
        }
    }
    lines.sort();

    let mut out = String::new();
    for line in &lines {
        out.push_str(line);
        out.push('\n');
    }
    std::fs::write(out_dir.join("index.sense"), out)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wordnet::{Entry, LexiconHashMapBackend, PartOfSpeech as Pos, Sense, SenseId as SId, Synset, SynsetId as SsId};

    /// Transcribed verbatim from `WNDB.PRINCETON_HEADER` in `gwn-scala-api`'s `wndb.scala` - used
    /// only so this test can reuse `WNDBSpec`'s literal expected output strings (see
    /// `test_wndb.scala`) unmodified, by supplying the exact same header via `license_file`.
    /// `ewe`'s own exporter never hardcodes this text (see `WndbExportOptions::license_file`'s
    /// doc comment) - it's test fixture data here, nothing more.
    const PRINCETON_HEADER: &str = "  1 This software and database is being provided to you, the LICENSEE, by  \n  2 Princeton University under the following license.  By obtaining, using  \n  3 and/or copying this software and database, you agree that you have  \n  4 read, understood, and will comply with these terms and conditions.:  \n  5   \n  6 Permission to use, copy, modify and distribute this software and  \n  7 database and its documentation for any purpose and without fee or  \n  8 royalty is hereby granted, provided that you agree to comply with  \n  9 the following copyright notice and statements, including the disclaimer,  \n  10 and that the same appear on ALL copies of the software, database and  \n  11 documentation, including modifications that you make for internal  \n  12 use or for distribution.  \n  13   \n  14 WordNet 3.1 Copyright 2011 by Princeton University.  All rights reserved.  \n  15   \n  16 THIS SOFTWARE AND DATABASE IS PROVIDED \"AS IS\" AND PRINCETON  \n  17 UNIVERSITY MAKES NO REPRESENTATIONS OR WARRANTIES, EXPRESS OR  \n  18 IMPLIED.  BY WAY OF EXAMPLE, BUT NOT LIMITATION, PRINCETON  \n  19 UNIVERSITY MAKES NO REPRESENTATIONS OR WARRANTIES OF MERCHANT-  \n  20 ABILITY OR FITNESS FOR ANY PARTICULAR PURPOSE OR THAT THE USE  \n  21 OF THE LICENSED SOFTWARE, DATABASE OR DOCUMENTATION WILL NOT  \n  22 INFRINGE ANY THIRD PARTY PATENTS, COPYRIGHTS, TRADEMARKS OR  \n  23 OTHER RIGHTS.  \n  24   \n  25 The name of Princeton University or Princeton may not be used in  \n  26 advertising or publicity pertaining to distribution of the software  \n  27 and/or database.  Title to copyright in this software, database and  \n  28 any associated documentation shall at all times remain with  \n  29 Princeton University and LICENSEE agrees to preserve same.  \n";

    /// The 3-synset/2-entry fixture `WNDBSpec` in `gwn-scala-api`'s `test_wndb.scala` uses
    /// (transcribed from `example3.xml`): "paternal grandfather" derives from "grandfather",
    /// whose synset has a hypernym. Synset ids are chosen only to preserve the same relative sort
    /// order as the original `example-en-*` ids (see this test module's doc comment for why that
    /// order matters) - the literal id strings themselves don't affect output content.
    fn small_fixture() -> LexiconHashMapBackend {
        let mut wn = LexiconHashMapBackend::new();

        let mut grandfather_ss = Synset::new(Pos::n);
        grandfather_ss.definition.push("the father of your father or mother".to_string());
        grandfather_ss.members.push("grandfather".to_string());
        grandfather_ss.hypernym.push(SsId::new("10162692-n"));
        wn.insert_synset("none".to_string(), SsId::new("10161911-n"), grandfather_ss).unwrap();

        let mut paternal_ss = Synset::new(Pos::n);
        paternal_ss
            .definition
            .push("A father's father; a paternal grandfather".to_string());
        paternal_ss.members.push("paternal grandfather".to_string());
        wn.insert_synset("none".to_string(), SsId::new("00000001-n"), paternal_ss).unwrap();

        let target_ss = Synset::new(Pos::n);
        wn.insert_synset("none".to_string(), SsId::new("10162692-n"), target_ss).unwrap();

        let mut grandfather_entry = Entry::new();
        grandfather_entry
            .sense
            .push(Sense::new(SId::new("grandfather%1:01:00::"), SsId::new("10161911-n")));
        wn.insert_entry("grandfather".to_string(), PosKey::new("n"), grandfather_entry).unwrap();

        let mut paternal_entry = Entry::new();
        let mut paternal_sense = Sense::new(SId::new("paternal_grandfather%1:01:00::"), SsId::new("00000001-n"));
        paternal_sense.derivation.push(SId::new("grandfather%1:01:00::"));
        paternal_entry.sense.push(paternal_sense);
        wn.insert_entry("paternal grandfather".to_string(), PosKey::new("n"), paternal_entry)
            .unwrap();

        wn
    }

    fn write_fixture(wn: &LexiconHashMapBackend, license_file: Option<std::path::PathBuf>) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let out_dir = std::env::temp_dir().join(format!("ewe_wndb_test_{}_{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&out_dir);
        write_wndb(wn, &out_dir, &WndbExportOptions { license_file, sense_orders: None }).unwrap();
        out_dir
    }

    fn header_file() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("ewe_wndb_header_{}_{n}.txt", std::process::id()));
        std::fs::write(&path, PRINCETON_HEADER).unwrap();
        path
    }

    /// Unlike `gwn-scala-api`'s `WNDB.write` (which only pointer-formats whatever relations are
    /// already literally present on the parsed `LexicalResource`), `ewe_lib`'s own model only
    /// ever stores the canonical/forward direction of a relation (see `SenseRelType`/
    /// `SynsetRelType`'s doc comments) - `hyponym` is never stored, only derived from `hypernym`.
    /// So unlike the hand-written `example3.xml` fixture `WNDBSpec` in `test_wndb.scala` uses
    /// (which has no reciprocal `hyponym` element on its target synset, since nothing in that
    /// pipeline computes one), a real WN-LMF document generated by an actual toolchain (e.g.
    /// `english-wordnet-2025.xml`, produced by `from_yaml.py`) does carry the computed reciprocal
    /// explicitly (confirmed empirically: `oewn2025/data.noun`'s "grandparent" synset carries a
    /// literal `~` pointer back to "grandfather") - so `write_data` computing that reciprocal
    /// itself (via `Lexicon::links_to`) is the correct behavior for matching real output, and
    /// this test's expectation (unlike the Scala one) includes it on the target synset's line.
    #[test]
    fn test_write_data_noun_matches_gwn_scala_api_fixture() {
        let wn = small_fixture();
        let out_dir = write_fixture(&wn, Some(header_file()));
        let data_noun = std::fs::read_to_string(out_dir.join("data.noun")).unwrap();

        let expected = format!(
            "{PRINCETON_HEADER}00001740 45 n 01 paternal_grandfather 0 001 + 00001848 n 0101 | A father's father; a paternal grandfather  \n00001848 45 n 01 grandfather 0 001 @ 00001941 n 0000 | the father of your father or mother  \n00001941 45 n 00 001 ~ 00001848 n 0000   \n\n"
        );
        assert_eq!(data_noun, expected);
    }

    #[test]
    fn test_write_index_noun_matches_gwn_scala_api_fixture() {
        let wn = small_fixture();
        let out_dir = write_fixture(&wn, Some(header_file()));
        let index_noun = std::fs::read_to_string(out_dir.join("index.noun")).unwrap();
        assert_eq!(
            index_noun,
            format!("{PRINCETON_HEADER}grandfather n 1 1 @ 1 0 00001848  \npaternal_grandfather n 1 1 + 1 0 00001740  \n")
        );
    }

    #[test]
    fn test_write_sense_index_matches_gwn_scala_api_fixture() {
        let wn = small_fixture();
        let out_dir = write_fixture(&wn, Some(header_file()));
        let index_sense = std::fs::read_to_string(out_dir.join("index.sense")).unwrap();
        assert_eq!(
            index_sense,
            "grandfather%1:01:00:: 00001848 1 0\npaternal_grandfather%1:01:00:: 00001740 1 0\n"
        );
    }

    #[test]
    fn test_scala_sense_key_round_trips_ordinary_keys() {
        assert_eq!(scala_sense_key("grandfather%1:01:00::"), "grandfather%1:01:00::");
    }

    #[test]
    fn test_scala_sense_key_reproduces_hyphen_doubling_quirk() {
        // Transcribed from real `oewn2025/index.sense` - confirms the exact upstream quirk this
        // is meant to reproduce (see this function's doc comment).
        assert_eq!(
            scala_sense_key("10-membered%5:00:00:membered:00"),
            "10--membered%5:00:00:membered:00"
        );
    }

    #[test]
    fn test_no_header_when_license_file_absent() {
        let wn = small_fixture();
        let out_dir = write_fixture(&wn, None);
        let data_noun = std::fs::read_to_string(out_dir.join("data.noun")).unwrap();
        assert!(!data_noun.contains("Princeton"), "no header should be written when license_file is None");
        assert!(data_noun.starts_with("00000000 "), "first synset should start at byte 0 with no header");
    }

    #[test]
    fn test_exc_files_have_no_header() {
        let mut wn = small_fixture();
        let mut entry = Entry::new();
        entry.sense.push(Sense::new(SId::new("goose%1:05:00::"), SsId::new("10161911-n")));
        entry.form.push("geese".to_string());
        wn.insert_entry("goose".to_string(), PosKey::new("n"), entry).unwrap();

        let out_dir = write_fixture(&wn, None);
        let noun_exc = std::fs::read_to_string(out_dir.join("noun.exc")).unwrap();
        assert_eq!(noun_exc, "geese goose\n");
    }
}
