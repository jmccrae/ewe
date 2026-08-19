//! WN-LMF XML import.
//!
//! Streams the document with `quick_xml`'s pull parser rather than building a DOM or using
//! `serde` over the whole document - OEWN's real `wn.xml` is ~89MB/107k synsets/136k entries,
//! too large to comfortably hold as a tree.
//!
//! The parse happens in two stages, both driven by a single forward read of the file:
//! 1. Every `LexicalEntry`/`Synset` is built directly into the internal model as it's read.
//!    Forward-direction relations (the ones our own writer emits as the primary direction, e.g.
//!    `hypernym`, `antonym`, `domain_topic`) apply immediately, since they only ever need the
//!    *currently open* entry/synset. Relations that need their *canonical* direction to differ
//!    from how they're written (e.g. a `<SynsetRelation relType="hyponym" .../>` needs to be
//!    stored as `hypernym` on the *other* synset, not on the one it's written on) are deferred
//!    into an in-memory list, since the synset/sense they actually need to be stored on may not
//!    have been parsed yet. This is the common case, not an edge case - our own exporter emits
//!    relations from both ends (`MemberSynset` computes every inverse field), so importing our
//!    own output exercises it constantly.
//!    `Synset/@members` references `LexicalEntry/@id`s, not lemmas, so resolving it requires
//!    every `LexicalEntry` to have already been seen - this assumes (as OEWN's real `wn.xml`
//!    does) that every `LexicalEntry` in the document precedes every `Synset`. A member id not
//!    yet known is dropped with a warning rather than silently producing a wrong member list.
//! 2. Once the whole document is read, the deferred list from step 1 is applied against the
//!    now-complete in-memory entries/synsets, everything is bulk-inserted into `lexicon`, and
//!    `finalize_bulk_load` (shared with YAML's `Lexicon::load`) resolves the remaining
//!    `UnresolvedSenseOrSynsetId` targets (domain_topic/domain_region/exemplifies/other) and
//!    rebuilds the reverse-link indexes - exactly as it does for a freshly YAML-loaded lexicon.
//!
//! `Sense`/`LexicalEntry`/`Synset` ids are decoded via [`super::ids`]; see that module's doc
//! comment for why this is safe for WN-LMF documents that don't use OEWN's particular id scheme.

use super::ids;
use super::{LexiconMetadata, XmlImportError};
use crate::rels::{SenseRelType, SynsetRelType, YamlSynsetRelType};
use crate::wordnet::lexicon::finalize_bulk_load;
use crate::wordnet::{
    Entry, Example, Lexicon, PartOfSpeech, PosKey, Pronunciation, Sense, SenseId, SenseOrSynsetId,
    Synset, SynsetId, UnresolvedSenseOrSynsetId, ILIID,
};
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use std::collections::{HashMap, HashSet};
use std::io::{BufReader, Read};

type Result<T> = std::result::Result<T, XmlImportError>;

/// A synset relation collected while parsing whose canonical direction differs from how it was
/// written - not yet applied, since the synset it's stored on may not have been parsed yet.
struct PendingSynsetRel {
    /// The synset the canonical relation is stored *on*.
    apply_to: SynsetId,
    rel: YamlSynsetRelType,
    /// The synset the canonical relation points *at*.
    target: SynsetId,
}

/// A sense relation collected while parsing whose canonical direction differs from how it was
/// written (`has_domain_topic`/`has_domain_region`/`is_exemplified_by` - see
/// `SenseRelType::to_canonical`) - not yet applied, since the sense it's stored on may not have
/// been parsed yet.
struct PendingSenseRel {
    /// The sense the canonical relation is stored *on* (the written relation's *target*).
    apply_to: SenseId,
    rel: SenseRelType,
    /// The sense the canonical relation points *at* (the written relation's *source*).
    target: SenseId,
}

/// Everything accumulated during stage 1 that stage 2/3 needs. Bundled into one struct so the
/// parse functions below don't each take a dozen separate `&mut` parameters.
#[derive(Default)]
struct Accumulator {
    entries: Vec<(String, PosKey, Entry)>,
    entry_index: HashMap<(String, PosKey), usize>,
    sense_owner: HashMap<SenseId, (String, PosKey)>,
    entry_id_lookup: HashMap<String, (String, PosKey)>,
    homograph_counts: HashMap<(String, char), u32>,

    synsets: Vec<(String, SynsetId, Synset)>,
    synset_index: HashMap<SynsetId, usize>,

    pending_synset_rels: Vec<PendingSynsetRel>,
    pending_sense_rels: Vec<PendingSenseRel>,

    warned_rel_types: HashSet<String>,
}

/// Parses a WN-LMF XML document and bulk-populates `lexicon` from it, the same way
/// `Lexicon::load` bulk-populates one from a YAML source tree. Returns the populated lexicon
/// together with the `Lexicon` element's own metadata (id/label/language/...), since a caller
/// re-exporting or otherwise inspecting the imported project needs that alongside the data.
pub fn read_lexicon_xml<L: Lexicon, R: Read>(mut lexicon: L, reader: R) -> Result<(L, LexiconMetadata)> {
    // Deliberately not `trim_text(true)`: that trims *every* text node uniformly, which would
    // silently corrupt genuinely meaningful leading/trailing whitespace inside e.g. a
    // `Pronunciation` value (confirmed present in real OEWN data). Insignificant whitespace-only
    // text nodes between sibling elements are already ignored on their own, since nothing below
    // extracts text except while positioned inside a specific text-bearing element
    // (`read_text_until_end`).
    let mut xml = Reader::from_reader(BufReader::new(reader));
    let mut buf = Vec::new();

    let mut metadata: Option<LexiconMetadata> = None;
    let mut prefix = String::new();
    let mut acc = Accumulator::default();

    loop {
        let event = xml.read_event_into(&mut buf)?;
        match event {
            Event::Eof => break,
            Event::Start(e) if e.name().as_ref() == b"Lexicon" => {
                let m = parse_lexicon_metadata(&e)?;
                prefix = m.id_prefix.clone();
                metadata = Some(m);
            }
            Event::Start(e) if e.name().as_ref() == b"LexicalEntry" => {
                let entry_xml_id = require_attr(&e, "id", "LexicalEntry")?;
                parse_lexical_entry(&mut xml, &mut buf, &prefix, entry_xml_id, &mut acc)?;
            }
            Event::Start(e) if e.name().as_ref() == b"Synset" => {
                let (lexname, id, mut synset) = build_synset(&e, &prefix, &acc.entry_id_lookup)?;
                parse_synset_children(&mut xml, &mut buf, &prefix, &id, &mut synset, &mut acc)?;
                acc.synset_index.insert(id.clone(), acc.synsets.len());
                acc.synsets.push((lexname, id, synset));
            }
            Event::Empty(e) if e.name().as_ref() == b"Synset" => {
                let (lexname, id, synset) = build_synset(&e, &prefix, &acc.entry_id_lookup)?;
                acc.synset_index.insert(id.clone(), acc.synsets.len());
                acc.synsets.push((lexname, id, synset));
            }
            _ => {}
        }
        buf.clear();
    }

    let metadata = metadata.ok_or_else(|| XmlImportError::Malformed("missing Lexicon element".to_string()))?;

    // Stage 2: apply relations whose canonical direction differs from how they were written,
    // now that every synset/sense exists to apply them to. A target this document never
    // describes is a dangling reference in the source file itself - nothing to apply to, same
    // as `validate()` would report for a YAML source with a broken reference.
    for pending in acc.pending_synset_rels {
        if let Some(&idx) = acc.synset_index.get(&pending.apply_to) {
            acc.synsets[idx].2.insert_rel(&pending.rel, &pending.target);
        }
    }
    for pending in acc.pending_sense_rels {
        if let Some((lemma, pos)) = acc.sense_owner.get(&pending.apply_to).cloned() {
            if let Some(&idx) = acc.entry_index.get(&(lemma, pos)) {
                if let Some(sense) = acc.entries[idx].2.sense.iter_mut().find(|s| s.id == pending.apply_to) {
                    sense.add_rel(pending.rel, SenseOrSynsetId::Sense(pending.target));
                }
            }
        }
    }

    // Stage 3: bulk-insert into the target lexicon (this also indexes sense_id -> (lemma, pos)
    // and the reverse-link maps incrementally; `finalize_bulk_load` below recomputes the
    // reverse-link maps wholesale regardless, so any partial state from these calls is
    // immaterial to the final result).
    for (lemma, pos, entry) in acc.entries {
        lexicon.insert_entry(lemma, pos, entry)?;
    }
    for (lexname, id, synset) in acc.synsets {
        lexicon.insert_synset(lexname, id, synset)?;
    }

    finalize_bulk_load(&mut lexicon)?;

    Ok((lexicon, metadata))
}

fn attr(e: &BytesStart, name: &str) -> Result<Option<String>> {
    match e.try_get_attribute(name)? {
        Some(a) => Ok(Some(a.unescape_value()?.into_owned())),
        None => Ok(None),
    }
}

fn require_attr(e: &BytesStart, name: &str, elem: &str) -> Result<String> {
    attr(e, name)?.ok_or_else(|| XmlImportError::Malformed(format!("<{elem}> missing required @{name}")))
}

fn parse_lexicon_metadata(e: &BytesStart) -> Result<LexiconMetadata> {
    Ok(LexiconMetadata {
        id_prefix: require_attr(e, "id", "Lexicon")?,
        label: attr(e, "label")?.unwrap_or_default(),
        language: attr(e, "language")?.unwrap_or_default(),
        email: attr(e, "email")?.filter(|s| !s.is_empty()),
        license: attr(e, "license")?.unwrap_or_default(),
        version: attr(e, "version")?.unwrap_or_default(),
        url: attr(e, "url")?.filter(|s| !s.is_empty()),
    })
}

fn part_of_speech_from_str(s: &str) -> Option<PartOfSpeech> {
    match s {
        "n" => Some(PartOfSpeech::n),
        "v" => Some(PartOfSpeech::v),
        "a" => Some(PartOfSpeech::a),
        "r" => Some(PartOfSpeech::r),
        "s" => Some(PartOfSpeech::s),
        _ => None,
    }
}

fn warn_once(warned: &mut HashSet<String>, message: String) {
    if warned.insert(message.clone()) {
        eprintln!("{message} (further occurrences of this message are suppressed)");
    }
}

fn parse_lexical_entry<R: Read>(
    xml: &mut Reader<BufReader<R>>,
    buf: &mut Vec<u8>,
    prefix: &str,
    entry_xml_id: String,
    acc: &mut Accumulator,
) -> Result<()> {
    let mut lemma: Option<String> = None;
    let mut pos_letter: Option<char> = None;
    let mut forms: Vec<String> = Vec::new();
    let mut pronunciations: Vec<Pronunciation> = Vec::new();
    let mut senses: Vec<Sense> = Vec::new();

    loop {
        match xml.read_event_into(buf)? {
            Event::End(e) if e.name().as_ref() == b"LexicalEntry" => break,
            Event::Eof => return Err(XmlImportError::Malformed("unexpected EOF inside <LexicalEntry>".to_string())),
            Event::Start(e) if e.name().as_ref() == b"Lemma" => {
                lemma = Some(require_attr(&e, "writtenForm", "Lemma")?);
                pos_letter = require_attr(&e, "partOfSpeech", "Lemma")?.chars().next();
                pronunciations = parse_pronunciations(xml, buf)?;
            }
            Event::Empty(e) if e.name().as_ref() == b"Lemma" => {
                lemma = Some(require_attr(&e, "writtenForm", "Lemma")?);
                pos_letter = require_attr(&e, "partOfSpeech", "Lemma")?.chars().next();
            }
            Event::Empty(e) if e.name().as_ref() == b"Form" => {
                forms.push(require_attr(&e, "writtenForm", "Form")?);
            }
            Event::Start(e) if e.name().as_ref() == b"Sense" => {
                let mut sense = build_sense(&e, prefix)?;
                parse_sense_relations(xml, buf, prefix, sense.id.clone(), &mut sense, acc)?;
                senses.push(sense);
            }
            Event::Empty(e) if e.name().as_ref() == b"Sense" => {
                senses.push(build_sense(&e, prefix)?);
            }
            _ => {}
        }
        buf.clear();
    }

    let lemma = lemma.ok_or_else(|| XmlImportError::Malformed("<LexicalEntry> missing <Lemma>".to_string()))?;
    let pos_letter = pos_letter.ok_or_else(|| XmlImportError::Malformed("<Lemma> missing @partOfSpeech".to_string()))?;

    // Successive LexicalEntry elements for the same (lemma, pos) are homographs - disambiguate
    // with a trailing counter ("n", "n2", "n3", ...), matching the convention YAML entries files
    // already use for this.
    let count = acc.homograph_counts.entry((lemma.clone(), pos_letter)).or_insert(0);
    let poskey = if *count == 0 {
        PosKey::new(pos_letter.to_string())
    } else {
        PosKey::new(format!("{}{}", pos_letter, *count + 1))
    };
    *count += 1;

    acc.entry_id_lookup.insert(entry_xml_id, (lemma.clone(), poskey.clone()));
    for sense in &senses {
        acc.sense_owner.insert(sense.id.clone(), (lemma.clone(), poskey.clone()));
    }

    let entry = Entry {
        sense: senses,
        form: forms,
        pronunciation: pronunciations,
    };
    acc.entry_index.insert((lemma.clone(), poskey.clone()), acc.entries.len());
    acc.entries.push((lemma, poskey, entry));

    Ok(())
}

fn parse_pronunciations<R: Read>(xml: &mut Reader<BufReader<R>>, buf: &mut Vec<u8>) -> Result<Vec<Pronunciation>> {
    let mut out = Vec::new();
    loop {
        match xml.read_event_into(buf)? {
            Event::End(e) if e.name().as_ref() == b"Lemma" => break,
            Event::Eof => return Err(XmlImportError::Malformed("unexpected EOF inside <Lemma>".to_string())),
            Event::Start(e) if e.name().as_ref() == b"Pronunciation" => {
                let variety = attr(&e, "variety")?;
                let text = read_text_until_end(xml, buf, b"Pronunciation")?;
                out.push(Pronunciation { value: text, variety });
            }
            Event::Empty(e) if e.name().as_ref() == b"Pronunciation" => {
                let variety = attr(&e, "variety")?;
                out.push(Pronunciation {
                    value: String::new(),
                    variety,
                });
            }
            _ => {}
        }
        buf.clear();
    }
    Ok(out)
}

fn build_sense(e: &BytesStart, prefix: &str) -> Result<Sense> {
    let sense_xml_id = require_attr(e, "id", "Sense")?;
    let synset_attr = require_attr(e, "synset", "Sense")?;
    let sense_id = SenseId::new(ids::unmap_sense_key(&sense_xml_id, prefix));
    let synset_id = SynsetId::new_owned(ids::strip_prefix_id(prefix, &synset_attr));
    Ok(Sense::new(sense_id, synset_id))
}

fn parse_sense_relations<R: Read>(
    xml: &mut Reader<BufReader<R>>,
    buf: &mut Vec<u8>,
    prefix: &str,
    own_sense_id: SenseId,
    sense: &mut Sense,
    acc: &mut Accumulator,
) -> Result<()> {
    loop {
        match xml.read_event_into(buf)? {
            Event::End(e) if e.name().as_ref() == b"Sense" => break,
            Event::Eof => return Err(XmlImportError::Malformed("unexpected EOF inside <Sense>".to_string())),
            Event::Empty(e) if e.name().as_ref() == b"SenseRelation" => {
                let rel_type_str = require_attr(&e, "relType", "SenseRelation")?;
                let target_attr = require_attr(&e, "target", "SenseRelation")?;
                let Some(rel) = SenseRelType::from(&rel_type_str) else {
                    warn_once(&mut acc.warned_rel_types, format!("Unrecognized SenseRelation relType {rel_type_str:?}"));
                    buf.clear();
                    continue;
                };
                let target_raw = ids::unmap_sense_key(&target_attr, prefix);
                let allows_synset_target = rel.allows_synset_target();
                let (canonical_direction, canonical_rel) = rel.to_canonical();
                if canonical_direction {
                    if allows_synset_target {
                        add_ambiguous_sense_rel(sense, canonical_rel, target_raw);
                    } else {
                        sense.add_rel(canonical_rel, SenseOrSynsetId::Sense(SenseId::new(target_raw)));
                    }
                } else {
                    // The written relation's target becomes the canonical relation's source -
                    // that source may not have been parsed yet, so defer.
                    acc.pending_sense_rels.push(PendingSenseRel {
                        apply_to: SenseId::new(target_raw),
                        rel: canonical_rel,
                        target: own_sense_id.clone(),
                    });
                }
            }
            _ => {}
        }
        buf.clear();
    }
    Ok(())
}

/// `domain_topic`/`domain_region`/`exemplifies`/`other` may target either a sense or a bare
/// synset - store as `Unresolved` exactly like a YAML-sourced sense does, so
/// `finalize_bulk_load`'s existing resolution pass classifies it against the fully-loaded
/// lexicon.
fn add_ambiguous_sense_rel(sense: &mut Sense, rel: SenseRelType, target_raw: String) {
    let target = UnresolvedSenseOrSynsetId::Unresolved(target_raw);
    match rel {
        SenseRelType::DomainTopic => push_unique(&mut sense.domain_topic, target),
        SenseRelType::DomainRegion => push_unique(&mut sense.domain_region, target),
        SenseRelType::Exemplifies => push_unique(&mut sense.exemplifies, target),
        _ => push_unique(&mut sense.other, target),
    }
}

fn push_unique<T: PartialEq>(v: &mut Vec<T>, item: T) {
    if !v.contains(&item) {
        v.push(item);
    }
}

fn build_synset(e: &BytesStart, prefix: &str, entry_id_lookup: &HashMap<String, (String, PosKey)>) -> Result<(String, SynsetId, Synset)> {
    let id_attr = require_attr(e, "id", "Synset")?;
    let id = SynsetId::new_owned(ids::strip_prefix_id(prefix, &id_attr));
    let pos_attr = require_attr(e, "partOfSpeech", "Synset")?;
    let pos = part_of_speech_from_str(&pos_attr)
        .ok_or_else(|| XmlImportError::Malformed(format!("Synset {id_attr}: unrecognized partOfSpeech {pos_attr:?}")))?;
    let lexname = require_attr(e, "lexfile", "Synset")?;

    let mut synset = Synset::new(pos);
    if let Some(ili) = attr(e, "ili")? {
        if ili != "in" {
            synset.ili = Some(ILIID::new(&ili));
        }
    }
    if let Some(members) = attr(e, "members")? {
        for member_id in members.split_whitespace() {
            match entry_id_lookup.get(member_id) {
                Some((lemma, _pos)) => synset.members.push(lemma.clone()),
                None => eprintln!(
                    "Synset {id_attr}: member {member_id:?} doesn't match any LexicalEntry seen so far - dropped \
                     (WN-LMF import assumes every LexicalEntry precedes every Synset in the document)"
                ),
            }
        }
    }
    Ok((lexname, id, synset))
}

fn parse_synset_children<R: Read>(
    xml: &mut Reader<BufReader<R>>,
    buf: &mut Vec<u8>,
    prefix: &str,
    own_id: &SynsetId,
    synset: &mut Synset,
    acc: &mut Accumulator,
) -> Result<()> {
    loop {
        match xml.read_event_into(buf)? {
            Event::End(e) if e.name().as_ref() == b"Synset" => break,
            Event::Eof => return Err(XmlImportError::Malformed("unexpected EOF inside <Synset>".to_string())),
            Event::Start(e) if e.name().as_ref() == b"Definition" => {
                let text = read_text_until_end(xml, buf, b"Definition")?;
                synset.definition.push(text);
            }
            Event::Start(e) if e.name().as_ref() == b"Example" => {
                let source = attr(&e, "dc:source")?;
                let text = read_text_until_end(xml, buf, b"Example")?;
                synset.example.push(Example::new(text, source));
            }
            Event::Empty(e) if e.name().as_ref() == b"SynsetRelation" => {
                let rel_type_str = require_attr(&e, "relType", "SynsetRelation")?;
                let target_attr = require_attr(&e, "target", "SynsetRelation")?;
                let Some(rel) = SynsetRelType::from(&rel_type_str) else {
                    warn_once(&mut acc.warned_rel_types, format!("Unrecognized SynsetRelation relType {rel_type_str:?}"));
                    buf.clear();
                    continue;
                };
                let target_id = SynsetId::new_owned(ids::strip_prefix_id(prefix, &target_attr));
                let (canonical_direction, canonical_rel) = rel.to_yaml();
                if canonical_direction {
                    synset.insert_rel(&canonical_rel, &target_id);
                } else {
                    acc.pending_synset_rels.push(PendingSynsetRel {
                        apply_to: target_id,
                        rel: canonical_rel,
                        target: own_id.clone(),
                    });
                }
            }
            _ => {}
        }
        buf.clear();
    }
    Ok(())
}

fn read_text_until_end<R: Read>(xml: &mut Reader<BufReader<R>>, buf: &mut Vec<u8>, end_tag: &[u8]) -> Result<String> {
    let mut text = String::new();
    loop {
        match xml.read_event_into(buf)? {
            Event::End(e) if e.name().as_ref() == end_tag => break,
            Event::Eof => return Err(XmlImportError::Malformed("unexpected EOF reading element text".to_string())),
            Event::Text(t) => text.push_str(&t.unescape()?),
            _ => {}
        }
        buf.clear();
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::writer::write_lexicon_xml;
    use crate::wordnet::entry::Entries;
    use crate::wordnet::synset::Synsets;
    use crate::wordnet::{LexiconHashMapBackend, UnresolvedSenseOrSynsetId};

    fn entry_for<L: Lexicon>(wn: &L, lemma: &str, pos: &PosKey) -> Entry {
        wn.entry_by_lemma_with_pos(lemma)
            .unwrap()
            .into_iter()
            .find(|(p, _)| p == pos)
            .unwrap_or_else(|| panic!("no entry for {lemma}/{pos}"))
            .1
            .into_owned()
    }

    /// WN-LMF has no equivalent of EWE's homograph `PosKey` suffix (`n`, `n-1`, `n2`, ...) - a
    /// `LexicalEntry` is just "this lemma, this part of speech", repeated as many times as there
    /// are homographs. So a re-imported document renumbers homograph entries from scratch in
    /// whatever order it encounters them, rather than preserving the original suffix strings
    /// (confirmed against the real source: e.g. "Re" is filed under `n-1`, not `n`/`n2`). The
    /// *grouping* of which senses belong to the same homograph entry is preserved exactly
    /// though, so match entries up by their sense id set instead of by the literal `PosKey`.
    fn entry_matching_senses<L: Lexicon>(wn: &L, lemma: &str, expected: &Entry) -> Entry {
        let expected_ids: std::collections::HashSet<&SenseId> =
            expected.sense.iter().map(|s| &s.id).collect();
        wn.entry_by_lemma_with_pos(lemma)
            .unwrap()
            .into_iter()
            .map(|(_, e)| e.into_owned())
            .find(|e| e.sense.iter().map(|s| &s.id).collect::<std::collections::HashSet<_>>() == expected_ids)
            .unwrap_or_else(|| panic!("no entry for {lemma} with matching senses {expected_ids:?}"))
    }

    /// A small hand-written WN-LMF document exercising: entries before synsets, a forward sense
    /// relation (antonym), an inverse synset relation (`hyponym`, written on the synset that
    /// comes *first* in the document, targeting one that's only defined *later* - so applying it
    /// genuinely needs the stage-2 deferred pass, not just in-place mutation), and a
    /// sense-or-synset-ambiguous relation (`domain_topic`) whose target is a bare synset id.
    const FIXTURE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE LexicalResource SYSTEM "http://globalwordnet.github.io/schemas/WN-LMF-1.4.dtd">
<LexicalResource xmlns:dc="https://globalwordnet.github.io/schemas/dc/">
  <Lexicon id="test" label="Test Wordnet" language="en" email="test@example.com" license="https://creativecommons.org/licenses/by/4.0" version="1" url="https://example.com">
    <LexicalEntry id="test-dog-n">
      <Lemma writtenForm="dog" partOfSpeech="n"/>
      <Sense id="test-dog__1.05.00.." synset="test-00001740-n">
        <SenseRelation relType="antonym" target="test-cat__1.05.00.."/>
        <SenseRelation relType="domain_topic" target="test-00001741-n"/>
      </Sense>
    </LexicalEntry>
    <LexicalEntry id="test-cat-n">
      <Lemma writtenForm="cat" partOfSpeech="n"/>
      <Sense id="test-cat__1.05.00.." synset="test-00001741-n"/>
    </LexicalEntry>
    <Synset id="test-00001741-n" ili="in" partOfSpeech="n" members="test-cat-n" lexfile="noun.animal">
      <Definition language="en">a domestic feline</Definition>
      <SynsetRelation relType="hyponym" target="test-00001740-n"/>
    </Synset>
    <Synset id="test-00001740-n" ili="in" partOfSpeech="n" members="test-dog-n" lexfile="noun.animal">
      <Definition language="en">a domestic canine</Definition>
    </Synset>
  </Lexicon>
</LexicalResource>
"#;

    #[test]
    fn test_read_lexicon_xml_fixture() {
        let (wn, metadata) = read_lexicon_xml(LexiconHashMapBackend::new(), FIXTURE.as_bytes()).unwrap();

        assert_eq!(metadata.id_prefix, "test");
        assert_eq!(metadata.label, "Test Wordnet");
        assert_eq!(metadata.email.as_deref(), Some("test@example.com"));

        let dog_ss = wn.synset_by_id(&SynsetId::new("00001740-n")).unwrap().unwrap();
        assert_eq!(dog_ss.definition, vec!["a domestic canine".to_string()]);
        assert_eq!(dog_ss.members, vec!["dog".to_string()]);
        // The hyponym relation was written on 00001741 (parsed first) targeting 00001740
        // (parsed second) - its canonical form (hypernym, stored on the target) only lands here
        // via the stage-2 deferred pass.
        assert_eq!(dog_ss.hypernym, vec![SynsetId::new("00001741-n")]);

        let cat_ss = wn.synset_by_id(&SynsetId::new("00001741-n")).unwrap().unwrap();
        assert!(cat_ss.hypernym.is_empty());

        let dog_entry = entry_for(&wn, "dog", &PosKey::new("n"));
        let dog_sense = &dog_entry.sense[0];
        assert_eq!(dog_sense.id, SenseId::new("dog%1:05:00::"));
        assert_eq!(dog_sense.antonym, vec![SenseId::new("cat%1:05:00::")]);
        // Resolved by `finalize_bulk_load` from the raw "00001741-n" string into a definite
        // Synset variant, since that string names a real synset (not a sense) in this document.
        assert_eq!(
            dog_sense.domain_topic,
            vec![UnresolvedSenseOrSynsetId::Synset(SynsetId::new("00001741-n"))]
        );
    }

    #[test]
    fn test_write_then_read_round_trips_relations() {
        use crate::wordnet::{Entry, LexiconMetadata as Meta, PartOfSpeech, Sense};

        let mut wn = LexiconHashMapBackend::new();
        let mut dog = Synset::new(PartOfSpeech::n);
        dog.definition.push("a domestic canine".to_string());
        dog.members.push("dog".to_string());
        dog.hypernym.push(SynsetId::new("00001741-n"));
        wn.insert_synset("noun.animal".to_string(), SynsetId::new("00001740-n"), dog).unwrap();
        let mut animal = Synset::new(PartOfSpeech::n);
        animal.definition.push("a living creature".to_string());
        animal.members.push("animal".to_string());
        wn.insert_synset("noun.animal".to_string(), SynsetId::new("00001741-n"), animal).unwrap();
        let mut cat_ss = Synset::new(PartOfSpeech::n);
        cat_ss.definition.push("a domestic feline".to_string());
        cat_ss.members.push("cat".to_string());
        wn.insert_synset("noun.animal".to_string(), SynsetId::new("00001742-n"), cat_ss).unwrap();

        let mut dog_entry = Entry::new();
        let mut dog_sense = Sense::new(SenseId::new("dog%1:05:00::"), SynsetId::new("00001740-n"));
        dog_sense.antonym.push(SenseId::new("cat%1:05:00::"));
        dog_entry.sense.push(dog_sense);
        wn.insert_entry("dog".to_string(), PosKey::new("n"), dog_entry).unwrap();
        let mut cat_entry = Entry::new();
        cat_entry.sense.push(Sense::new(SenseId::new("cat%1:05:00::"), SynsetId::new("00001742-n")));
        wn.insert_entry("cat".to_string(), PosKey::new("n"), cat_entry).unwrap();
        let mut animal_entry = Entry::new();
        animal_entry.sense.push(Sense::new(SenseId::new("animal%1:03:00::"), SynsetId::new("00001741-n")));
        wn.insert_entry("animal".to_string(), PosKey::new("n"), animal_entry).unwrap();

        let metadata = Meta {
            id_prefix: "test".to_string(),
            label: "Test".to_string(),
            language: "en".to_string(),
            email: None,
            license: "https://creativecommons.org/licenses/by/4.0".to_string(),
            version: "1".to_string(),
            url: None,
        };
        let xml = write_lexicon_xml(&wn, &metadata).unwrap();

        let (reimported, _) = read_lexicon_xml(LexiconHashMapBackend::new(), xml.as_slice()).unwrap();

        let dog_ss = reimported.synset_by_id(&SynsetId::new("00001740-n")).unwrap().unwrap();
        assert_eq!(dog_ss.hypernym, vec![SynsetId::new("00001741-n")]);
        let animal_ss = reimported.synset_by_id(&SynsetId::new("00001741-n")).unwrap().unwrap();
        // hyponym is the inverse of hypernym - our own writer emits it (computed via the
        // reverse-index), and re-importing it must land back on the *original* (00001740's
        // hypernym), not create a spurious extra one on 00001741 itself.
        assert!(animal_ss.hypernym.is_empty());

        let dog_entry = entry_for(&reimported, "dog", &PosKey::new("n"));
        assert_eq!(dog_entry.sense[0].antonym, vec![SenseId::new("cat%1:05:00::")]);
        // Unlike hypernym/hyponym, `MemberSynset` doesn't compute a reverse-index view for
        // antonym - the writer only ever emits what's literally stored on `Sense.antonym`, and
        // this fixture only set it on dog's sense. Cat legitimately has none, both before and
        // after the round trip.
        let cat_entry = entry_for(&reimported, "cat", &PosKey::new("n"));
        assert!(cat_entry.sense[0].antonym.is_empty());
    }

    /// Imports the real, locally-decompressed OEWN release XML and sanity-checks the result -
    /// well-formed data, counts in the right ballpark, and a spot-checked known entry. Needs
    /// `globalwordnet/english-wordnet` checked out with `english-wordnet-2025.xml.gz` gunzipped
    /// to a sibling `english-wordnet-2025.xml` (`gunzip -k english-wordnet-2025.xml.gz`, per
    /// that repo's own `RELEASING.md`); `#[ignore]`d since CI won't have that checkout.
    #[test]
    #[ignore]
    fn test_read_lexicon_xml_against_real_oewn_release() {
        use std::fs::File;

        let path = "/home/jmccrae/projects/globalwordnet/english-wordnet/english-wordnet-2025.xml";
        let file = File::open(path).unwrap();
        let (wn, metadata) = read_lexicon_xml(LexiconHashMapBackend::new(), file).unwrap();

        assert_eq!(metadata.id_prefix, "oewn");
        assert_eq!(metadata.version, "2025");

        let n_entries = wn.n_entries().unwrap();
        let n_synsets = wn.n_synsets().unwrap();
        assert!(n_entries > 100_000, "expected >100k entries, got {n_entries}");
        assert!(n_synsets > 90_000, "expected >90k synsets, got {n_synsets}");

        // Spot-check a known entry: exact sense key recovery proves `ids::unmap_sense_key`
        // correctly inverts the real OEWN escaping, not just the synthetic cases above.
        let entry = entry_for(&wn, ".22-caliber", &PosKey::new("a"));
        assert_eq!(entry.sense[0].id, SenseId::new(".22-caliber%3:01:00::"));

        let dog_ss = wn.synset_by_id(&SynsetId::new("02086723-n")).unwrap();
        assert!(dog_ss.is_some(), "expected the canonical dog synset to be present");
    }

    /// Exports the real, locally-decompressed OEWN release XML via YAML load, then round-trips
    /// it: import -> export -> re-import, asserting the twice-imported model matches the
    /// once-imported one. This validates `ids.rs`'s encode/decode pair is a true inverse and
    /// that the writer/reader's canonical-direction handling agree with each other, without
    /// needing to match OEWN's own writer byte-for-byte.
    #[test]
    #[ignore]
    fn test_export_then_reimport_matches_original_import() {
        use std::fs::File;

        let path = "/home/jmccrae/projects/globalwordnet/english-wordnet/english-wordnet-2025.xml";
        let file = File::open(path).unwrap();
        let (wn, metadata) = read_lexicon_xml(LexiconHashMapBackend::new(), file).unwrap();

        let xml = write_lexicon_xml(&wn, &metadata).unwrap();
        let (reimported, _) = read_lexicon_xml(LexiconHashMapBackend::new(), xml.as_slice()).unwrap();

        assert_eq!(wn.n_entries().unwrap(), reimported.n_entries().unwrap());
        assert_eq!(wn.n_synsets().unwrap(), reimported.n_synsets().unwrap());
        assert_eq!(wn.n_senses().unwrap(), reimported.n_senses().unwrap());

        let dog_ss_before = wn.synset_by_id(&SynsetId::new("02086723-n")).unwrap().unwrap();
        let dog_ss_after = reimported.synset_by_id(&SynsetId::new("02086723-n")).unwrap().unwrap();
        assert_eq!(dog_ss_before.definition, dog_ss_after.definition);
        assert_eq!(dog_ss_before.members, dog_ss_after.members);
        assert_eq!(dog_ss_before.hypernym, dog_ss_after.hypernym);
        assert_eq!(dog_ss_before.meronym, dog_ss_after.meronym);
        // hyponym is inverse-only (computed, not a `Synset` field) - compare it via the
        // resolved `MemberSynset` view instead, which is where the writer itself reads it from.
        let dog_ms_before = wn.get_member_synset(&SynsetId::new("02086723-n")).unwrap();
        let dog_ms_after = reimported.get_member_synset(&SynsetId::new("02086723-n")).unwrap();
        assert_eq!(dog_ms_before.hyponym.len(), dog_ms_after.hyponym.len());
    }

    /// Compares every field the XML writer actually emits for one synset. `wikidata`/`source`
    /// aren't part of WN-LMF's core schema and were never emitted by the exporter this was moved
    /// from (a pre-existing gap, not something introduced by this module) - blanked out here
    /// rather than compared.
    fn assert_synset_round_trips(id: &SynsetId, ground_truth: &Synset, reimported: &Synset) {
        let mut normalized_truth = ground_truth.clone();
        normalized_truth.wikidata = Vec::new();
        normalized_truth.source = None;
        assert_eq!(&normalized_truth, reimported, "synset {id} mismatch after XML round trip");
    }

    /// Compares every `Sense` field the XML writer actually emits a `SenseRelation` for.
    /// `also`/`similar`/`domain_topic`/`domain_region`/`other` are real fields on `Sense` but
    /// `writer.rs`'s `sense_relations_xml` never emits them at the sense level (only at the
    /// synset level, under different field names) - another pre-existing gap carried over
    /// unchanged from the original exporter. `subcat`/`adjposition`/`sent` are YAML-only
    /// extensions with no WN-LMF equivalent at all.
    fn assert_sense_round_trips(entry_lemma: &str, ground_truth: &Sense, reimported: &Sense) {
        macro_rules! field {
            ($f:ident) => {
                assert_eq!(
                    ground_truth.$f,
                    reimported.$f,
                    "{entry_lemma}: sense {} field `{}` mismatch after XML round trip",
                    ground_truth.id,
                    stringify!($f)
                );
            };
        }
        field!(id);
        field!(synset);
        field!(antonym);
        field!(participle);
        field!(pertainym);
        field!(derivation);
        field!(exemplifies);
        field!(agent);
        field!(material);
        field!(event);
        field!(instrument);
        field!(location);
        field!(by_means_of);
        field!(undergoer);
        field!(property);
        field!(result);
        field!(state);
        field!(uses);
        field!(destination);
        field!(body_part);
        field!(vehicle);
    }

    /// The strongest validation available: load the real OEWN YAML source directly (the ground
    /// truth `Lexicon::load` already has its own extensive test coverage against), export it to
    /// XML, and re-import that XML - then compare *every* synset and *every* sense against the
    /// YAML-loaded original, not just against another XML import. This is what actually proves
    /// XML import/export preserves the source data, rather than just proving the two directions
    /// are self-consistent with each other (which a shared bug in both could still pass).
    /// Needs the same local OEWN checkout as the other real-data tests in this module.
    #[test]
    #[ignore]
    fn test_yaml_ground_truth_matches_xml_round_trip() {
        use crate::progress::NullProgress;

        let yaml_path = "/home/jmccrae/projects/globalwordnet/english-wordnet/src/yaml/";
        let ground_truth = LexiconHashMapBackend::new().load(yaml_path, &mut NullProgress).unwrap();

        let metadata = LexiconMetadata {
            id_prefix: "oewn".to_string(),
            label: "Open English Wordnet".to_string(),
            language: "en".to_string(),
            email: None,
            license: "https://creativecommons.org/licenses/by/4.0".to_string(),
            version: "2025".to_string(),
            url: None,
        };
        let xml = write_lexicon_xml(&ground_truth, &metadata).unwrap();
        let (reimported, _) = read_lexicon_xml(LexiconHashMapBackend::new(), xml.as_slice()).unwrap();

        assert_eq!(ground_truth.n_entries().unwrap(), reimported.n_entries().unwrap());
        assert_eq!(ground_truth.n_synsets().unwrap(), reimported.n_synsets().unwrap());
        assert_eq!(ground_truth.n_senses().unwrap(), reimported.n_senses().unwrap());

        let mut synsets_compared = 0usize;
        for lexfile in ground_truth.synsets_iter().unwrap() {
            let (lexname, synsets) = lexfile.unwrap();
            for entry in synsets.iter().unwrap() {
                let (id, synset) = entry.unwrap();
                let reimported_synset = reimported
                    .synset_by_id(&id)
                    .unwrap()
                    .unwrap_or_else(|| panic!("synset {id} missing after XML round trip"));
                assert_synset_round_trips(&id, &synset, &reimported_synset);
                assert_eq!(
                    reimported.lex_name_for(&id).unwrap().as_deref(),
                    Some(lexname.as_str()),
                    "lexfile mismatch for synset {id}"
                );
                synsets_compared += 1;
            }
        }
        assert_eq!(synsets_compared, ground_truth.n_synsets().unwrap());

        let mut entries_compared = 0usize;
        for bucket in ground_truth.entries_iter().unwrap() {
            let (_, entries) = bucket.unwrap();
            for entry in entries.entries().unwrap() {
                let (lemma, pos, ground_entry) = entry.unwrap();
                let reimported_entry = entry_matching_senses(&reimported, &lemma, &ground_entry);
                assert_eq!(
                    ground_entry.form, reimported_entry.form,
                    "{lemma}/{pos}: form mismatch after XML round trip"
                );
                assert_eq!(
                    ground_entry.pronunciation, reimported_entry.pronunciation,
                    "{lemma}/{pos}: pronunciation mismatch after XML round trip"
                );
                assert_eq!(
                    ground_entry.sense.len(), reimported_entry.sense.len(),
                    "{lemma}/{pos}: sense count mismatch after XML round trip"
                );
                let mut reimported_by_id: HashMap<SenseId, Sense> = reimported_entry
                    .sense
                    .into_iter()
                    .map(|s| (s.id.clone(), s))
                    .collect();
                for ground_sense in &ground_entry.sense {
                    let reimported_sense = reimported_by_id.remove(&ground_sense.id).unwrap_or_else(|| {
                        panic!("{lemma}/{pos}: sense {} missing after XML round trip", ground_sense.id)
                    });
                    assert_sense_round_trips(&lemma, ground_sense, &reimported_sense);
                }
                entries_compared += 1;
            }
        }
        assert_eq!(entries_compared, ground_truth.n_entries().unwrap());
    }
}
