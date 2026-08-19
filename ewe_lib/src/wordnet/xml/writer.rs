//! WN-LMF XML export.
//!
//! [`write_lexicon_xml`] produces a whole-lexicon, self-contained document: every relation
//! target is itself a synset/sense described in the same document, so it validates against
//! strict `IDREF` typing. [`write_lexicon_xml_subset`] is the lower-level building block it's
//! implemented on top of, and is also exposed directly for callers (e.g. `ewe_dioxus`'s
//! per-synset/per-lemma web routes) that intentionally export less than the whole lexicon; such
//! a subset is *not* guaranteed self-contained (a relation may point at a synset/sense outside
//! the subset), so `SenseRelation` targets outside the given synsets are silently omitted (there
//! is no way to construct a target's `Sense/@id` - it's derived from the target's real sense key,
//! not just its lemma/pos/synset - without the target's `MemberSynset` in hand), while
//! `SynsetRelation` targets are always emitted regardless (a `Synset/@id` is just `{prefix}-{id}`,
//! so it doesn't need the target to be present to be constructed - this mirrors how the RDF
//! export links to synsets it doesn't itself describe).

use super::ids;
use super::{LexiconMetadata, XmlExportError, WN_LMF_DOCTYPE};
use crate::wordnet::synset_members::Member;
use crate::wordnet::{Lexicon, MemberSynset, PosKey, Pronunciation, SenseId, SynsetId, Synsets};
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use std::collections::{BTreeMap, HashMap};

type Result<T> = std::result::Result<T, XmlExportError>;

/// Key identifying a sense within a synset by (lemma, pos, synset) - the information a
/// `SenseRelation`'s target carries - so an actual target sense id can be looked up.
type SenseKeyLookup = HashMap<(String, PosKey, SynsetId), SenseId>;

/// Export every synset in `wn` as a single self-contained WN-LMF document.
pub fn write_lexicon_xml<L: Lexicon>(wn: &L, metadata: &LexiconMetadata) -> Result<Vec<u8>> {
    let mut synsets = Vec::new();
    for lexfile in wn.synsets_iter()? {
        let (_, syns) = lexfile?;
        for entry in syns.iter()? {
            let (id, synset) = entry?;
            synsets.push(MemberSynset::from_synset(&id, synset.into_owned(), wn)?);
        }
    }
    write_lexicon_xml_subset(&synsets, metadata)
}

struct EntryAcc<'a> {
    representative: &'a Member,
    senses: Vec<(&'a MemberSynset, &'a Member)>,
}

/// Export exactly the given synsets (see the module doc comment for the self-containment
/// caveat this implies).
pub fn write_lexicon_xml_subset(synsets: &[MemberSynset], metadata: &LexiconMetadata) -> Result<Vec<u8>> {
    let prefix = metadata.id_prefix.as_str();

    let mut entries: BTreeMap<(String, PosKey), EntryAcc> = BTreeMap::new();
    let mut sense_id_lookup: SenseKeyLookup = HashMap::new();
    for synset in synsets {
        for member in &synset.members {
            entries
                .entry((member.lemma.clone(), member.poskey.clone()))
                .or_insert_with(|| EntryAcc {
                    representative: member,
                    senses: Vec::new(),
                })
                .senses
                .push((synset, member));
            sense_id_lookup.insert(
                (member.lemma.clone(), member.poskey.clone(), synset.id.clone()),
                member.sense.id.clone(),
            );
        }
    }

    let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);
    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;
    writer.write_event(Event::DocType(BytesText::from_escaped(WN_LMF_DOCTYPE)))?;

    let mut lexical_resource = BytesStart::new("LexicalResource");
    lexical_resource.push_attribute(("xmlns:dc", "https://globalwordnet.github.io/schemas/dc/"));
    writer.write_event(Event::Start(lexical_resource))?;

    let mut lexicon = BytesStart::new("Lexicon");
    lexicon.push_attribute(("id", prefix));
    lexicon.push_attribute(("label", metadata.label.as_str()));
    lexicon.push_attribute(("language", metadata.language.as_str()));
    lexicon.push_attribute(("email", metadata.email.as_deref().unwrap_or("")));
    lexicon.push_attribute(("license", metadata.license.as_str()));
    lexicon.push_attribute(("version", metadata.version.as_str()));
    lexicon.push_attribute(("url", metadata.url.as_deref().unwrap_or("")));
    writer.write_event(Event::Start(lexicon))?;

    for ((lemma, poskey), acc) in &entries {
        write_lexical_entry(&mut writer, prefix, lemma, poskey, acc, &sense_id_lookup)?;
    }

    for synset in synsets {
        write_synset(&mut writer, prefix, synset)?;
    }

    writer.write_event(Event::End(BytesEnd::new("Lexicon")))?;
    writer.write_event(Event::End(BytesEnd::new("LexicalResource")))?;

    Ok(writer.into_inner())
}

fn write_lexical_entry<W: std::io::Write>(
    writer: &mut Writer<W>,
    prefix: &str,
    lemma: &str,
    poskey: &PosKey,
    acc: &EntryAcc,
    sense_id_lookup: &SenseKeyLookup,
) -> Result<()> {
    let mut entry = BytesStart::new("LexicalEntry");
    entry.push_attribute(("id", ids::entry_xml_id(prefix, lemma, poskey).as_str()));
    writer.write_event(Event::Start(entry))?;

    let pos = poskey.to_part_of_speech().map(|p| p.value()).unwrap_or("n");
    let mut lemma_el = BytesStart::new("Lemma");
    lemma_el.push_attribute(("writtenForm", lemma));
    lemma_el.push_attribute(("partOfSpeech", pos));
    if acc.representative.pronunciation.is_empty() {
        writer.write_event(Event::Empty(lemma_el))?;
    } else {
        writer.write_event(Event::Start(lemma_el))?;
        for pron in &acc.representative.pronunciation {
            write_pronunciation(writer, pron)?;
        }
        writer.write_event(Event::End(BytesEnd::new("Lemma")))?;
    }

    for form in &acc.representative.form {
        let mut form_el = BytesStart::new("Form");
        form_el.push_attribute(("writtenForm", form.as_str()));
        writer.write_event(Event::Empty(form_el))?;
    }

    for (synset, member) in &acc.senses {
        write_sense(writer, prefix, member, synset, sense_id_lookup)?;
    }

    writer.write_event(Event::End(BytesEnd::new("LexicalEntry")))?;
    Ok(())
}

fn write_pronunciation<W: std::io::Write>(writer: &mut Writer<W>, pron: &Pronunciation) -> Result<()> {
    let mut el = BytesStart::new("Pronunciation");
    if let Some(variety) = &pron.variety {
        el.push_attribute(("variety", variety.as_str()));
    }
    writer.write_event(Event::Start(el))?;
    writer.write_event(Event::Text(BytesText::new(&pron.value)))?;
    writer.write_event(Event::End(BytesEnd::new("Pronunciation")))?;
    Ok(())
}

fn write_sense<W: std::io::Write>(
    writer: &mut Writer<W>,
    prefix: &str,
    member: &Member,
    synset: &MemberSynset,
    sense_id_lookup: &SenseKeyLookup,
) -> Result<()> {
    let mut sense = BytesStart::new("Sense");
    sense.push_attribute(("id", ids::sense_xml_id(prefix, &member.sense.id).as_str()));
    sense.push_attribute(("synset", ids::synset_xml_id(prefix, &synset.id).as_str()));

    let relations = sense_relations_xml(prefix, synset, &member.lemma, sense_id_lookup);
    if relations.is_empty() {
        writer.write_event(Event::Empty(sense))?;
    } else {
        writer.write_event(Event::Start(sense))?;
        for (rel_type, target) in relations {
            let mut rel_el = BytesStart::new("SenseRelation");
            rel_el.push_attribute(("relType", rel_type));
            rel_el.push_attribute(("target", target.as_str()));
            writer.write_event(Event::Empty(rel_el))?;
        }
        writer.write_event(Event::End(BytesEnd::new("Sense")))?;
    }
    Ok(())
}

/// Only relation types that appear in the WN-LMF `SenseRelation` DTD enum are emitted here. The
/// inverse "is_X_of" semantic-role relations our internal model tracks (is_agent_of,
/// is_material_of, etc.) have no corresponding relType in the DTD - they're meant to be derived
/// by reversing the forward relation, not stored - so they're intentionally skipped.
fn sense_relations_xml(
    prefix: &str,
    synset: &MemberSynset,
    lemma: &str,
    sense_id_lookup: &SenseKeyLookup,
) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    macro_rules! rel {
        ($field:ident, $rel_type:expr) => {
            for rel in &synset.$field {
                if rel.source_lemma != lemma {
                    continue;
                }
                // A `None` target_lemma/target_poskey means the target is a bare synset
                // (domain_topic/domain_region/exemplifies/other can point at one instead of a
                // specific sense) - the WN-LMF SenseRelation/@target attribute must reference a
                // sense, so this can't be expressed here and is skipped, same as the is_X_of
                // relations above.
                let (Some(target_lemma), Some(target_poskey)) = (&rel.target_lemma, &rel.target_poskey) else {
                    continue;
                };
                let key = (target_lemma.clone(), target_poskey.clone(), rel.target_synset.clone());
                // A target outside this export's synsets has no known real sense key to build
                // its `Sense/@id` from (see the module doc comment) - skip it rather than emit
                // an unresolvable IDREF.
                if let Some(target_sense_id) = sense_id_lookup.get(&key) {
                    out.push(($rel_type, ids::sense_xml_id(prefix, target_sense_id)));
                }
            }
        };
    }

    rel!(antonym, "antonym");
    rel!(participle, "participle");
    rel!(pertainym, "pertainym");
    rel!(derivation, "derivation");
    rel!(exemplifies_sense, "exemplifies");
    rel!(is_exemplified_by_sense, "is_exemplified_by");
    rel!(agent, "agent");
    rel!(material, "material");
    rel!(event, "event");
    rel!(instrument, "instrument");
    rel!(location, "location");
    rel!(by_means_of, "by_means_of");
    rel!(undergoer, "undergoer");
    rel!(property, "property");
    rel!(result, "result");
    rel!(state, "state");
    rel!(uses, "uses");
    rel!(destination, "destination");
    rel!(body_part, "body_part");
    rel!(vehicle, "vehicle");

    out
}

fn write_synset<W: std::io::Write>(writer: &mut Writer<W>, prefix: &str, synset: &MemberSynset) -> Result<()> {
    let mut el = BytesStart::new("Synset");
    el.push_attribute(("id", ids::synset_xml_id(prefix, &synset.id).as_str()));
    let ili = synset
        .ili
        .as_ref()
        .map(|i| i.as_str().to_string())
        .unwrap_or_else(|| "in".to_string());
    el.push_attribute(("ili", ili.as_str()));
    el.push_attribute(("partOfSpeech", synset.part_of_speech.value()));
    let members = synset
        .members
        .iter()
        .map(|m| ids::entry_xml_id(prefix, &m.lemma, &m.poskey))
        .collect::<Vec<_>>()
        .join(" ");
    if !members.is_empty() {
        el.push_attribute(("members", members.as_str()));
    }
    el.push_attribute(("lexfile", synset.lexname.as_str()));
    writer.write_event(Event::Start(el))?;

    for defn in &synset.definition {
        let mut def_el = BytesStart::new("Definition");
        def_el.push_attribute(("language", "en"));
        writer.write_event(Event::Start(def_el))?;
        writer.write_event(Event::Text(BytesText::new(defn)))?;
        writer.write_event(Event::End(BytesEnd::new("Definition")))?;
    }

    for (rel_type, target) in synset_relations_xml(prefix, synset) {
        let mut rel_el = BytesStart::new("SynsetRelation");
        rel_el.push_attribute(("relType", rel_type));
        rel_el.push_attribute(("target", target.as_str()));
        writer.write_event(Event::Empty(rel_el))?;
    }

    for example in &synset.example {
        let mut ex_el = BytesStart::new("Example");
        ex_el.push_attribute(("language", "en"));
        if let Some(source) = &example.source {
            ex_el.push_attribute(("dc:source", source.as_str()));
        }
        writer.write_event(Event::Start(ex_el))?;
        writer.write_event(Event::Text(BytesText::new(&example.text)))?;
        writer.write_event(Event::End(BytesEnd::new("Example")))?;
    }

    writer.write_event(Event::End(BytesEnd::new("Synset")))?;
    Ok(())
}

fn synset_relations_xml(prefix: &str, synset: &MemberSynset) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    macro_rules! rel {
        ($field:ident, $rel_type:expr) => {
            for target in &synset.$field {
                out.push(($rel_type, ids::synset_xml_id(prefix, target)));
            }
        };
    }

    rel!(also, "also");
    rel!(attribute, "attribute");
    rel!(causes, "causes");
    rel!(domain_region, "domain_region");
    rel!(domain_topic, "domain_topic");
    rel!(exemplifies, "exemplifies");
    rel!(entails, "entails");
    rel!(hypernym, "hypernym");
    rel!(instance_hypernym, "instance_hypernym");
    rel!(mero_location, "mero_location");
    rel!(mero_member, "mero_member");
    rel!(mero_part, "mero_part");
    rel!(mero_portion, "mero_portion");
    rel!(mero_substance, "mero_substance");
    rel!(meronym, "meronym");
    rel!(similar, "similar");
    rel!(feminine, "feminine");
    rel!(masculine, "masculine");
    rel!(other, "other");
    rel!(hyponym, "hyponym");
    rel!(is_caused_by, "is_caused_by");
    rel!(has_domain_region, "has_domain_region");
    rel!(has_domain_topic, "has_domain_topic");
    rel!(is_exemplified_by, "is_exemplified_by");
    rel!(is_entailed_by, "is_entailed_by");
    rel!(instance_hyponym, "instance_hyponym");
    rel!(holo_location, "holo_location");
    rel!(holo_member, "holo_member");
    rel!(holo_part, "holo_part");
    rel!(holo_portion, "holo_portion");
    rel!(holo_substance, "holo_substance");
    rel!(holonym, "holonym");

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wordnet::{Entry, LexiconHashMapBackend, PartOfSpeech, Sense, SenseId as SId, Synset, SynsetId as SsId};

    fn simple_lexicon() -> LexiconHashMapBackend {
        let mut wn = LexiconHashMapBackend::new();
        let mut synset = Synset::new(PartOfSpeech::n);
        synset.definition.push("a domestic canine".to_string());
        synset.members.push("dog".to_string());
        wn.insert_synset("noun.animal".to_string(), SsId::new("00001740-n"), synset)
            .unwrap();
        let mut entry = Entry::new();
        entry
            .sense
            .push(Sense::new(SId::new("dog%1:05:00::"), SsId::new("00001740-n")));
        wn.insert_entry("dog".to_string(), PosKey::new("n"), entry).unwrap();
        wn
    }

    fn metadata() -> LexiconMetadata {
        LexiconMetadata {
            id_prefix: "oewn".to_string(),
            label: "Test Wordnet".to_string(),
            language: "en".to_string(),
            email: Some("test@example.com".to_string()),
            license: "https://creativecommons.org/licenses/by/4.0".to_string(),
            version: "2025".to_string(),
            url: Some("https://example.com".to_string()),
        }
    }

    #[test]
    fn test_write_lexicon_xml_is_valid_utf8_and_contains_expected_elements() {
        let wn = simple_lexicon();
        let xml = write_lexicon_xml(&wn, &metadata()).unwrap();
        let xml = String::from_utf8(xml).unwrap();
        assert!(xml.contains("WN-LMF-1.4.dtd"));
        assert!(xml.contains(r#"<LexicalEntry id="oewn-dog-n">"#));
        assert!(xml.contains(r#"<Lemma writtenForm="dog" partOfSpeech="n"/>"#));
        assert!(xml.contains(r#"<Sense id="oewn-dog__1.05.00.." synset="oewn-00001740-n"/>"#));
        assert!(xml.contains(r#"<Synset id="oewn-00001740-n" ili="in" partOfSpeech="n" members="oewn-dog-n" lexfile="noun.animal">"#));
        assert!(xml.contains("a domestic canine"));
    }

    #[test]
    fn test_write_lexicon_xml_subset_skips_unresolvable_sense_relation_target() {
        // hypernym/hyponym-style synset relations always resolve since they're pure id
        // formatting, but a sense relation pointing outside the given subset can't be
        // constructed and must be dropped rather than emit a broken target.
        let wn = simple_lexicon();
        let member_synset = wn.get_member_synset(&SsId::new("00001740-n")).unwrap();
        let xml = write_lexicon_xml_subset(std::slice::from_ref(&member_synset), &metadata()).unwrap();
        let xml = String::from_utf8(xml).unwrap();
        assert!(!xml.contains("SenseRelation"));
    }

    /// Exports the real, locally-checked-out OEWN YAML source and sanity-checks the result -
    /// well-formed, and element counts match what `Lexicon` itself reports. Needs
    /// `globalwordnet/english-wordnet` checked out as a sibling of this repo's parent
    /// (`/home/jmccrae/projects/globalwordnet/english-wordnet`); `#[ignore]`d since CI won't
    /// have that checkout.
    #[test]
    #[ignore]
    fn test_write_lexicon_xml_against_real_oewn_yaml() {
        use crate::progress::NullProgress;

        let path = "/home/jmccrae/projects/globalwordnet/english-wordnet/src/yaml/";
        let wn = LexiconHashMapBackend::new().load(path, &mut NullProgress).unwrap();
        let n_entries = wn.n_entries().unwrap();
        let n_synsets = wn.n_synsets().unwrap();
        assert!(n_entries > 100_000, "expected >100k entries, got {n_entries}");
        assert!(n_synsets > 90_000, "expected >90k synsets, got {n_synsets}");

        let xml = write_lexicon_xml(&wn, &metadata()).unwrap();
        let xml_str = String::from_utf8(xml.clone()).unwrap();
        assert!(xml_str.starts_with("<?xml"));
        assert!(xml_str.contains("WN-LMF-1.4.dtd"));

        // Well-formedness: a full quick_xml pull-parse over the whole document without error.
        let mut reader = quick_xml::Reader::from_reader(xml.as_slice());
        let mut buf = Vec::new();
        let mut lexical_entry_count = 0usize;
        let mut synset_count = 0usize;
        loop {
            match reader.read_event_into(&mut buf).unwrap() {
                quick_xml::events::Event::Start(e) | quick_xml::events::Event::Empty(e) => {
                    match e.name().as_ref() {
                        b"LexicalEntry" => lexical_entry_count += 1,
                        b"Synset" => synset_count += 1,
                        _ => {}
                    }
                }
                quick_xml::events::Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }
        assert_eq!(lexical_entry_count, n_entries);
        assert_eq!(synset_count, n_synsets);
    }
}
