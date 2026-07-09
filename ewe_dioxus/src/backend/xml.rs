//! Implements XML export in the Global Wordnet WN-LMF format.
//!
//! Schema: <http://globalwordnet.github.io/schemas/WN-LMF-relaxed-1.4.dtd>
//! Documentation: <https://globalwordnet.github.io/schemas/>
//!
//! Each export is a self-contained `LexicalResource`/`Lexicon` covering just
//! the requested synset (or all synsets a lemma has a sense in). Relation
//! targets outside that subset are still referenced (mirroring how the RDF
//! export links to synsets it doesn't itself describe), so `IDREF`/`IDREFS`
//! attributes are not guaranteed to resolve within a single partial export -
//! only a full-lexicon dump is fully self-contained per the DTD's strict
//! ID/IDREF typing.

use crate::backend::rdf::{lemma_id, resolve_lemma_synsets, resolve_synset};
use crate::dioxus_fullstack::{body::Body, http::Response};
use dioxus::prelude::*;
use oewn_lib::wordnet::synset_members::Member;
use oewn_lib::wordnet::{MemberSynset, PosKey, Pronunciation, SynsetId};
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use std::collections::BTreeMap;

const ID_PREFIX: &str = "oewn";
const DOCTYPE: &str =
    "LexicalResource SYSTEM \"http://globalwordnet.github.io/schemas/WN-LMF-relaxed-1.4.dtd\"";

#[get("/xml/synset/{id}")]
pub async fn synset_xml(id: String) -> Result<Response<Body>> {
    let id = SynsetId::new_owned(id);
    match resolve_synset(&id) {
        Ok(Some(ms)) => match gen_lexicon_xml(std::slice::from_ref(&ms)) {
            Ok(xml) => Ok(xml_response(xml)),
            Err(e) => Ok(server_error(e)),
        },
        Ok(None) => Ok(not_found("Synset not found")),
        Err(e) => Ok(server_error(e)),
    }
}

#[get("/xml/lemma/{lemma}")]
pub async fn lemma_xml(lemma: String) -> Result<Response<Body>> {
    match resolve_lemma_synsets(&lemma) {
        Ok(synsets) if synsets.is_empty() => Ok(not_found("Lemma not found")),
        Ok(synsets) => match gen_lexicon_xml(&synsets) {
            Ok(xml) => Ok(xml_response(xml)),
            Err(e) => Ok(server_error(e)),
        },
        Err(e) => Ok(server_error(e)),
    }
}

fn xml_response(xml: Vec<u8>) -> Response<Body> {
    Response::builder()
        .header("Content-Type", "application/xml")
        .body(Body::from(xml))
        .unwrap()
}

fn not_found(msg: &'static str) -> Response<Body> {
    Response::builder()
        .status(404)
        .body(Body::from(msg))
        .unwrap()
}

fn server_error(e: impl std::fmt::Display) -> Response<Body> {
    Response::builder()
        .status(500)
        .body(Body::from(format!("Internal server error: {}", e)))
        .unwrap()
}

fn entry_xml_id(lemma: &str, poskey: &PosKey) -> String {
    format!("{}-{}", ID_PREFIX, lemma_id(lemma, poskey))
}

fn synset_xml_id(id: &SynsetId) -> String {
    format!("{}-{}", ID_PREFIX, id.as_str())
}

fn sense_xml_id(lemma: &str, poskey: &PosKey, synset_id: &SynsetId) -> String {
    format!("{}-{}", entry_xml_id(lemma, poskey), synset_id.as_str())
}

struct EntryAcc<'a> {
    representative: &'a Member,
    senses: Vec<(&'a MemberSynset, &'a Member)>,
}

fn gen_lexicon_xml(synsets: &[MemberSynset]) -> Result<Vec<u8>> {
    let mut entries: BTreeMap<(String, PosKey), EntryAcc> = BTreeMap::new();
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
        }
    }

    let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);
    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;
    writer.write_event(Event::DocType(BytesText::from_escaped(DOCTYPE)))?;

    let mut lexical_resource = BytesStart::new("LexicalResource");
    lexical_resource.push_attribute(("xmlns:dc", "https://globalwordnet.github.io/schemas/dc/"));
    writer.write_event(Event::Start(lexical_resource))?;

    let mut lexicon = BytesStart::new("Lexicon");
    lexicon.push_attribute(("id", ID_PREFIX));
    lexicon.push_attribute(("label", "Open English Wordnet"));
    lexicon.push_attribute(("language", "en"));
    lexicon.push_attribute(("email", "english-wordnet@googlegroups.com"));
    lexicon.push_attribute(("license", "https://creativecommons.org/licenses/by/4.0"));
    lexicon.push_attribute(("version", "2024"));
    lexicon.push_attribute(("url", "https://github.com/globalwordnet/english-wordnet"));
    writer.write_event(Event::Start(lexicon))?;

    for ((lemma, poskey), acc) in &entries {
        write_lexical_entry(&mut writer, lemma, poskey, acc)?;
    }

    for synset in synsets {
        write_synset(&mut writer, synset)?;
    }

    writer.write_event(Event::End(BytesEnd::new("Lexicon")))?;
    writer.write_event(Event::End(BytesEnd::new("LexicalResource")))?;

    Ok(writer.into_inner())
}

fn write_lexical_entry<W: std::io::Write>(
    writer: &mut Writer<W>,
    lemma: &str,
    poskey: &PosKey,
    acc: &EntryAcc,
) -> Result<()> {
    let mut entry = BytesStart::new("LexicalEntry");
    entry.push_attribute(("id", entry_xml_id(lemma, poskey).as_str()));
    writer.write_event(Event::Start(entry))?;

    let pos = poskey
        .to_part_of_speech()
        .map(|p| p.value())
        .unwrap_or("n");
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
        write_sense(writer, lemma, poskey, synset, member)?;
    }

    writer.write_event(Event::End(BytesEnd::new("LexicalEntry")))?;
    Ok(())
}

fn write_pronunciation<W: std::io::Write>(
    writer: &mut Writer<W>,
    pron: &Pronunciation,
) -> Result<()> {
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
    lemma: &str,
    poskey: &PosKey,
    synset: &MemberSynset,
    member: &Member,
) -> Result<()> {
    let mut sense = BytesStart::new("Sense");
    sense.push_attribute(("id", sense_xml_id(lemma, poskey, &synset.id).as_str()));
    sense.push_attribute(("synset", synset_xml_id(&synset.id).as_str()));
    sense.push_attribute(("dc:identifier", member.sense.id.as_str()));

    let relations = sense_relations_xml(synset, lemma);
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

/// Only relation types that appear in the WN-LMF `SenseRelation` DTD enum are
/// emitted here. The inverse "is_X_of" semantic-role relations our internal
/// model tracks (is_agent_of, is_material_of, etc.) have no corresponding
/// relType in the DTD - they're meant to be derived by reversing the forward
/// relation, not stored - so they're intentionally skipped.
fn sense_relations_xml(synset: &MemberSynset, lemma: &str) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    macro_rules! rel {
        ($field:ident, $rel_type:expr) => {
            for rel in &synset.$field {
                if rel.source_lemma == lemma {
                    out.push((
                        $rel_type,
                        sense_xml_id(&rel.target_lemma, &rel.target_poskey, &rel.target_synset),
                    ));
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

fn write_synset<W: std::io::Write>(writer: &mut Writer<W>, synset: &MemberSynset) -> Result<()> {
    let mut el = BytesStart::new("Synset");
    el.push_attribute(("id", synset_xml_id(&synset.id).as_str()));
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
        .map(|m| entry_xml_id(&m.lemma, &m.poskey))
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

    for (rel_type, target) in synset_relations_xml(synset) {
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

fn synset_relations_xml(synset: &MemberSynset) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    macro_rules! rel {
        ($field:ident, $rel_type:expr) => {
            for target in &synset.$field {
                out.push(($rel_type, synset_xml_id(target)));
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

    #[test]
    fn test_entry_xml_id() {
        let pos_key = PosKey::new("n");
        assert_eq!(entry_xml_id("dog", &pos_key), "oewn-dog-n");
    }

    #[test]
    fn test_synset_xml_id() {
        let id = SynsetId::new_owned("00001740-n".to_string());
        assert_eq!(synset_xml_id(&id), "oewn-00001740-n");
    }
}
