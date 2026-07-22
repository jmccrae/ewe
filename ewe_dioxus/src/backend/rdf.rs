//! Implements the key RDF functionality of returning pages as RDF
//! using content negotiation

// TODO : URLs need to have a short prefix (oewn-00001740-n) instead of just ID 00001740-n
use crate::dioxus_fullstack::response::IntoResponse;
use crate::dioxus_fullstack::{body::Body, http::Response, HeaderMap, Redirect};
use dioxus::prelude::*;
use ewe_lib::wordnet::{Lexicon, MemberSynset, PosKey, SynsetId};
use oxrdf::vocab::rdf;
use oxrdf::*;
use oxrdfio::{RdfFormat, RdfSerializer, WriterQuadSerializer};
use percent_encoding::{utf8_percent_encode, CONTROLS};
use std::collections::BTreeSet;

/// Legacy WordNet ids come prefixed with the dataset name - the deployment's
/// configured `settings.id_prefix` (`oewn-` for the Open English Wordnet), or
/// `ewn-` as a hardcoded legacy fallback from when this project was called
/// just "English WordNet" - e.g. `oewn-00001740-n`, whereas every route in
/// this app (`/synset/{id}`, `/api/synset/{id}`, etc.) takes the bare id,
/// e.g. `00001740-n`. Returns `None` if `id` carries neither prefix.
fn strip_synset_id_prefix<'a>(id: &'a str, id_prefix: &str) -> Option<&'a str> {
    id.strip_prefix(&format!("{}-", id_prefix))
        .or_else(|| id.strip_prefix("ewn-"))
}

/// Legacy synset lookup paths: `/id/{id}` and `/synset/{id}` where `id`
/// still carries an `oewn-`/`ewn-` prefix (see [`strip_synset_id_prefix`]).
/// Both permanently redirect to the canonical, unprefixed `/synset/{id}`,
/// which then does the normal content negotiation.
#[get("/id/{id}")]
pub async fn synset_id_alias(id: String) -> Result<Response<Body>> {
    let id_prefix = &crate::SETTINGS.get().id_prefix;
    let bare_id = strip_synset_id_prefix(&id, id_prefix).unwrap_or(&id);
    Ok(Redirect::permanent(&format!("/synset/{}", bare_id)).into_response())
}

#[get("/synset/{id}", headers : HeaderMap)]
pub async fn synset_negotiated(id: String) -> Result<Response<Body>> {
    let id_prefix = &crate::SETTINGS.get().id_prefix;
    if let Some(bare_id) = strip_synset_id_prefix(&id, id_prefix) {
        return Ok(Redirect::permanent(&format!("/synset/{}", bare_id)).into_response());
    }
    let content_type = negotiate(headers);
    let response = match content_type {
        ContentType::HTML => Redirect::to(&format!("/view/synset/{}", id)).into_response(),
        ContentType::RDFXML => Redirect::to(&format!("/rdf/synset/{}", id)).into_response(),
        ContentType::Turtle => Redirect::to(&format!("/ttl/synset/{}", id)).into_response(),
        ContentType::XML => Redirect::to(&format!("/xml/synset/{}", id)).into_response(),
        ContentType::JSON => Redirect::to(&format!("/api/synset/{}", id)).into_response(),
    };
    Ok(response)
}

#[get("/rdf/synset/{id}")]
pub async fn synset_rdf(id: String) -> Result<Response<Body>> {
    synset_serialized(id, RdfFormat::RdfXml).await
}

#[get("/ttl/synset/{id}")]
pub async fn synset_ttl(id: String) -> Result<Response<Body>> {
    synset_serialized(id, RdfFormat::Turtle).await
}

async fn synset_serialized(id: String, format: RdfFormat) -> Result<Response<Body>> {
    let id = SynsetId::new_owned(id);
    match resolve_synset(&id) {
        Ok(Some(ms)) => {
            match gen_synset_rdf(
                format,
                "https://creativecommons.org/licenses/by/4.0/",
                "https://en-word.net/",
                "en",
                &ms,
            ) {
                Ok(rdf_data) => Ok(Response::builder()
                    .header("Content-Type", format.media_type())
                    .body(Body::from(rdf_data))
                    .unwrap()),
                Err(e) => Ok(Response::builder()
                    .status(500)
                    .body(Body::from(format!("Internal server error: {}", e)))
                    .unwrap()),
            }
        }
        Ok(None) => Ok(Response::builder()
            .status(404)
            .body(Body::from("Synset not found"))
            .unwrap()),
        Err(e) => Ok(Response::builder()
            .status(500)
            .body(Body::from(format!("Internal server error: {}", e)))
            .unwrap()),
    }
}

#[get("/lemma/{lemma}", headers : HeaderMap)]
pub async fn lemma_negotiated(lemma: String) -> Result<Response<Body>> {
    let content_type = negotiate(headers);
    let response = match content_type {
        ContentType::HTML => Redirect::to(&format!("/view/lemma/{}", lemma)).into_response(),
        ContentType::RDFXML => Redirect::to(&format!("/rdf/lemma/{}", lemma)).into_response(),
        ContentType::Turtle => Redirect::to(&format!("/ttl/lemma/{}", lemma)).into_response(),
        ContentType::XML => Redirect::to(&format!("/xml/lemma/{}", lemma)).into_response(),
        ContentType::JSON => Redirect::to(&format!("/api/lemma/{}", lemma)).into_response(),
    };
    Ok(response)
}

#[get("/rdf/lemma/{lemma}")]
pub async fn lemma_rdf(lemma: String) -> Result<Response<Body>> {
    lemma_serialized(lemma, RdfFormat::RdfXml).await
}

#[get("/ttl/lemma/{lemma}")]
pub async fn lemma_ttl(lemma: String) -> Result<Response<Body>> {
    lemma_serialized(lemma, RdfFormat::Turtle).await
}

async fn lemma_serialized(lemma: String, format: RdfFormat) -> Result<Response<Body>> {
    match lemma_rdf_bytes(&lemma, format) {
        Ok(Some(rdf_data)) => Ok(Response::builder()
            .header("Content-Type", format.media_type())
            .body(Body::from(rdf_data))
            .unwrap()),
        Ok(None) => Ok(Response::builder()
            .status(404)
            .body(Body::from("Lemma not found"))
            .unwrap()),
        Err(e) => Ok(Response::builder()
            .status(500)
            .body(Body::from(format!("Internal server error: {}", e)))
            .unwrap()),
    }
}

fn lemma_rdf_bytes(lemma: &str, format: RdfFormat) -> Result<Option<Vec<u8>>> {
    let member_synsets = resolve_lemma_synsets(lemma)?;
    if member_synsets.is_empty() {
        return Ok(None);
    }

    let rdf_data = gen_synsets_rdf(
        format,
        "https://creativecommons.org/licenses/by/4.0/",
        "https://en-word.net/",
        "en",
        &member_synsets,
    )?;

    Ok(Some(rdf_data))
}

/// Looks up a single synset by id and expands it into a [`MemberSynset`], the
/// enriched representation (with reverse relation links) shared by the RDF,
/// Turtle, XML, and JSON exports. Returns `Ok(None)` if the lexicon isn't
/// loaded or the id doesn't exist.
pub(crate) fn resolve_synset(id: &SynsetId) -> Result<Option<MemberSynset>> {
    let Some(lexicon) = crate::LEXICON.get().as_ref() else {
        return Ok(None);
    };
    let lexicon = lexicon.read().unwrap();
    let Some(synset) = lexicon.synset_by_id(id)? else {
        return Ok(None);
    };
    Ok(Some(MemberSynset::from_synset(
        id,
        synset.into_owned(),
        &*lexicon,
    )?))
}

/// Looks up every distinct synset that a lemma has a sense in. Returns an
/// empty vec (not an error) if the lexicon isn't loaded or the lemma is
/// unknown.
pub(crate) fn resolve_lemma_synsets(lemma: &str) -> Result<Vec<MemberSynset>> {
    let Some(lexicon) = crate::LEXICON.get().as_ref() else {
        return Ok(Vec::new());
    };
    let lexicon = lexicon.read().unwrap();

    let entries = lexicon.entry_by_lemma(lemma)?;
    let synset_ids: BTreeSet<SynsetId> = entries
        .iter()
        .flat_map(|entry| entry.sense.iter().map(|sense| sense.synset.clone()))
        .collect();

    let mut member_synsets = Vec::with_capacity(synset_ids.len());
    for id in &synset_ids {
        if let Some(synset) = resolve_synset(id)? {
            member_synsets.push(synset);
        }
    }
    Ok(member_synsets)
}

enum ContentType {
    HTML,
    RDFXML,
    Turtle,
    XML,
    JSON,
}

fn negotiate(headers: HeaderMap) -> ContentType {
    let accept_str = headers
        .get("Accept")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("*/*");

    let mut types: Vec<(f32, &str)> = accept_str
        .split(',')
        .filter_map(|part| {
            let mut pieces = part.split(';');
            let mime = pieces.next()?.trim();

            // Default quality is 1.0 if not specified
            let mut q = 1.0;
            for piece in pieces {
                let piece = piece.trim();
                if piece.starts_with("q=") {
                    q = piece[2..].parse::<f32>().unwrap_or(0.0);
                }
            }
            Some((q, mime))
        })
        .collect();

    types.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    types
        .into_iter()
        .find_map(|(_, mime)| {
            match mime {
                "text/html" => Some(ContentType::HTML),
                "application/rdf+xml" => Some(ContentType::RDFXML),
                "text/turtle" => Some(ContentType::Turtle),
                "application/xml" => Some(ContentType::XML),
                "application/json" => Some(ContentType::JSON),
                "*/*" => Some(ContentType::HTML), // Default to HTML if any type is accepted
                _ => None,
            }
        })
        .unwrap_or(ContentType::HTML) // Default to HTML if no acceptable type is found
}

fn ontolex(id: &str) -> Result<NamedNode> {
    Ok(NamedNode::new(&format!(
        "http://www.w3.org/ns/lemon/ontolex#{}",
        id
    ))?)
}

fn wn(id: &str) -> Result<NamedNode> {
    Ok(NamedNode::new(&format!(
        "https://globalwordnet.github.io/schemas/wn#{}",
        id
    ))?)
}

fn skos(id: &str) -> Result<NamedNode> {
    Ok(NamedNode::new(&format!(
        "http://www.w3.org/2004/02/skos/core#{}",
        id
    ))?)
}

fn dc(id: &str) -> Result<NamedNode> {
    Ok(NamedNode::new(&format!("http://purl.org/dc/terms/{}", id))?)
}

fn ili(id: &str) -> Result<NamedNode> {
    Ok(NamedNode::new(&format!(
        "http://ili.globalwordnet.org/ili/{}",
        id
    ))?)
}

fn owl(id: &str) -> Result<NamedNode> {
    Ok(NamedNode::new(&format!(
        "http://www.w3.org/2002/07/owl#{}",
        id
    ))?)
}

fn wikidata(id: &str) -> Result<NamedNode> {
    Ok(NamedNode::new(&format!(
        "http://www.wikidata.org/entity/{}",
        id
    ))?)
}

const FRAGMENT_ENCODE_SET: &percent_encoding::AsciiSet = &CONTROLS.add(b'%').add(b'#');
const PATH_ENCODE_SET: &percent_encoding::AsciiSet = &CONTROLS
    .add(b'/')
    .add(b'#')
    .add(b'?')
    .add(b'&')
    .add(b'=')
    .add(b'+')
    .add(b'$')
    .add(b',')
    .add(b';')
    .add(b':')
    .add(b'@')
    .add(b' ');

fn section_uri(site: &str, section: &str) -> String {
    if site.ends_with("/") {
        format!("{}{}/", site, section)
    } else {
        format!("{}/{}/", site, section)
    }
}

fn build_url(site: &str, section: &str, id: &str, fragment: Option<&str>) -> Result<NamedNode> {
    let site = section_uri(site, section);
    let mut url = format!("{}{}", site, utf8_percent_encode(id, PATH_ENCODE_SET));
    if let Some(fragment) = fragment {
        url.push_str(&format!(
            "#{}",
            utf8_percent_encode(fragment, FRAGMENT_ENCODE_SET)
        ));
    }
    Ok(NamedNode::new(url)?)
}

fn gen_synset_rdf(
    format: RdfFormat,
    license: &str,
    site: &str,
    language: &str,
    synset: &MemberSynset,
) -> Result<Vec<u8>> {
    gen_synsets_rdf(format, license, site, language, std::slice::from_ref(synset))
}

fn gen_synsets_rdf(
    format: RdfFormat,
    license: &str,
    site: &str,
    language: &str,
    synsets: &[MemberSynset],
) -> Result<Vec<u8>> {
    let mut serializer = RdfSerializer::from_format(format)
        .with_prefix("rdf", "http://www.w3.org/1999/02/22-rdf-syntax-ns#")?
        .with_prefix("dc", "http://purl.org/dc/terms/")?
        .with_prefix("ili", "http://ili.globalwordnet.org/ili/")?
        .with_prefix("lime", "http://www.w3.org/ns/lemon/lime#")?
        .with_prefix("ontolex", "http://www.w3.org/ns/lemon/ontolex#")?
        .with_prefix("owl", "http://www.w3.org/2002/07/owl#")?
        .with_prefix("rdfs", "http://www.w3.org/2000/01/rdf-schema#")?
        .with_prefix("schema", "http://schema.org/")?
        .with_prefix("skos", "http://www.w3.org/2004/02/skos/core#")?
        .with_prefix("synsem", "http://www.w3.org/ns/lemon/synsem#")?
        .with_prefix("wn", "https://globalwordnet.github.io/schemas/wn#")?
        .with_prefix("wikidata", "http://www.wikidata.org/entity/")?
        .with_prefix("wordnet", site)?
        .with_prefix("wordnetlicense", license)?
        .with_prefix("wordnetlemma", &section_uri(site, "lemma"))?
        .with_prefix("wordnetsynset", &section_uri(site, "synset"))?
        .for_writer(Vec::new());

    for synset in synsets {
        write_synset_triples(&mut serializer, site, language, synset)?;
    }

    Ok(serializer.finish()?)
}

fn write_synset_triples<W: std::io::Write>(
    serializer: &mut WriterQuadSerializer<W>,
    site: &str,
    language: &str,
    synset: &MemberSynset,
) -> Result<()> {
    macro_rules! triple {
        ($s:expr, $p:expr, $o:expr) => {
            serializer.serialize_triple(TripleRef::new($s, $p, $o))?
        };
    }

    macro_rules! lang_lit {
        ($value:expr, $lang:expr) => {
            LiteralRef::new_language_tagged_literal_unchecked($value, $lang)
        };
    }

    macro_rules! lit {
        ($value:expr) => {
            LiteralRef::new_simple_literal($value)
        };
    }

    let ss = build_url(site, "synset", &synset.id.as_str(), None)?;

    for member in &synset.members {
        let entry = build_url(
            site,
            "lemma",
            &lemma_id(&member.lemma, &member.poskey),
            None,
        )?;
        let sense = build_url(
            site,
            "lemma",
            &lemma_id(&member.lemma, &member.poskey),
            Some(synset.id.as_str()),
        )?;
        let pos = member
            .poskey
            .to_part_of_speech()
            .map(|pos| pos.long_pos())
            .unwrap_or("unknown");
        triple!(&entry, rdf::TYPE, &ontolex("LexicalEntry")?);
        triple!(
            &entry,
            &ontolex("canonicalForm")?,
            lang_lit!(&member.lemma, language)
        );
        triple!(&entry, &ontolex("sense")?, &sense);
        triple!(&entry, &wn("partOfSpeech")?, &wn(pos)?);
        triple!(&sense, rdf::TYPE, &ontolex("LexicalSense")?);
        triple!(&sense, &ontolex("isLexicalizedSenseOf")?, &ss);
        macro_rules! sense_rel {
            ($rel_type:ident, $rel_name:expr) => {
                for rel in &synset.$rel_type {
                    if rel.source_lemma == member.lemma {
                        let target = build_url(
                            site,
                            "lemma",
                            &lemma_id(&rel.target_lemma, &rel.target_poskey),
                            Some(rel.target_synset.as_str()),
                        )?;
                        triple!(&sense, &wn($rel_name)?, &target);
                    }
                }
            };
        }

        sense_rel!(antonym, "antonym");
        sense_rel!(participle, "participle");
        sense_rel!(is_participle_of, "isParticipleOf");
        sense_rel!(pertainym, "pertainym");
        sense_rel!(derivation, "derivation");
        sense_rel!(domain_topic_sense, "domainTopic");
        sense_rel!(has_domain_topic_sense, "hasDomainTopic");
        sense_rel!(domain_region_sense, "domainRegion");
        sense_rel!(has_domain_region_sense, "hasDomainRegion");
        sense_rel!(exemplifies_sense, "exemplifies");
        sense_rel!(is_exemplified_by_sense, "isExemplifiedBy");
        sense_rel!(agent, "agent");
        sense_rel!(is_agent_of, "isAgentOf");
        sense_rel!(material, "material");
        sense_rel!(is_material_of, "isMaterialOf");
        sense_rel!(event, "event");
        sense_rel!(is_event_of, "isEventOf");
        sense_rel!(instrument, "instrument");
        sense_rel!(is_instrument_of, "isInstrumentOf");
        sense_rel!(location, "location");
        sense_rel!(is_location_of, "isLocationOf");
        sense_rel!(by_means_of, "byMeansOf");
        sense_rel!(is_by_means_of, "isByMeansOf");
        sense_rel!(undergoer, "undergoer");
        sense_rel!(is_undergoer_of, "isUndergoerOf");
        sense_rel!(property, "property");
        sense_rel!(is_property_of, "isPropertyOf");
        sense_rel!(result, "result");
        sense_rel!(is_result_of, "isResultOf");
        sense_rel!(state, "state");
        sense_rel!(is_state_of, "isStateOf");
        sense_rel!(uses, "uses");
        sense_rel!(is_used_by, "isUsedBy");
        sense_rel!(destination, "destination");
        sense_rel!(is_destination_of, "isDestinationOf");
        sense_rel!(body_part, "bodyPart");
        sense_rel!(is_body_part_of, "isBodyPartOf");
        sense_rel!(vehicle, "vehicle");
        sense_rel!(is_vehicle_of, "isVehicleOf");
    }

    triple!(&ss, rdf::TYPE, &ontolex("LexicalConcept")?);
    triple!(&ss, &skos("inScheme")?, NamedNodeRef::new(site)?);

    for defn in &synset.definition {
        let bn = BlankNode::default();
        triple!(&ss, &wn("definition")?, &bn);
        triple!(&bn, rdf::VALUE, lang_lit!(&defn, "en"));
    }

    for example in &synset.example {
        let bn = BlankNode::default();
        triple!(&ss, &wn("example")?, &bn);
        triple!(&bn, rdf::VALUE, lang_lit!(&example.text, "en"));
        if let Some(source) = &example.source {
            if source.starts_with("http://") || source.starts_with("https://") {
                triple!(&bn, &dc("source")?, NamedNodeRef::new(source)?);
            } else {
                triple!(&bn, &dc("source")?, lang_lit!(source, "en"));
            }
        }
    }

    if let Some(ili_id) = synset.ili.as_ref() {
        triple!(&ss, &wn("ili")?, &ili(ili_id.as_str())?);
    }

    triple!(&ss, &wn("lexfile")?, lit!(&synset.lexname));

    triple!(
        &ss,
        &wn("partOfSpeech")?,
        &wn(synset.part_of_speech.long_pos())?
    );

    for wd in &synset.wikidata {
        triple!(&ss, &owl("sameAs")?, &wikidata(wd)?);
    }

    if let Some(source) = &synset.source {
        if source.starts_with("http://") || source.starts_with("https://") {
            triple!(&ss, &dc("source")?, NamedNodeRef::new(source)?);
        } else {
            triple!(&ss, &dc("source")?, lang_lit!(source, "en"));
        }
    }
    macro_rules! synset_rel {
        ($rel_type:ident, $rel_name:expr) => {
            for rel in &synset.$rel_type {
                let target = build_url(site, "synset", &rel.as_str(), None)?;
                triple!(&ss, &wn($rel_name)?, &target);
            }
        };
    }
    synset_rel!(also, "also");
    synset_rel!(attribute, "attribute");
    synset_rel!(causes, "causes");
    synset_rel!(domain_region, "domainRegion");
    synset_rel!(domain_topic, "domainTopic");
    synset_rel!(exemplifies, "exemplifies");
    synset_rel!(entails, "entails");
    synset_rel!(hypernym, "hypernym");
    synset_rel!(instance_hypernym, "instanceHypernym");
    synset_rel!(mero_location, "meroLocation");
    synset_rel!(mero_member, "meroMember");
    synset_rel!(mero_part, "meroPart");
    synset_rel!(mero_portion, "meroPortion");
    synset_rel!(mero_substance, "meroSubstance");
    synset_rel!(meronym, "meronym");
    synset_rel!(similar, "similar");
    synset_rel!(feminine, "feminine");
    synset_rel!(masculine, "masculine");
    synset_rel!(other, "other");

    Ok(())
}

pub(crate) fn lemma_id(lemma: &str, pos_key: &PosKey) -> String {
    format!("{}-{}", lemma.replace(" ", "_"), pos_key.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ewe_lib::wordnet::{Example, MemberSynset, PartOfSpeech, PosKey, SenseRelation, SynsetId};
    use oxrdf::NamedNode;

    /// `MemberSynset` has no public constructor besides `from_synset` (which needs a
    /// `Lexicon`), so tests build one directly via its (all-public) fields, leaving
    /// every relation/member list empty except what's under test.
    fn test_synset(
        id: SynsetId,
        lexname: String,
        definition: Vec<String>,
        part_of_speech: PartOfSpeech,
    ) -> MemberSynset {
        MemberSynset {
            id,
            lexname,
            members: vec![],
            definition,
            example: vec![],
            ili: None,
            wikidata: vec![],
            source: None,
            part_of_speech,
            also: vec![],
            attribute: vec![],
            causes: vec![],
            domain_region: vec![],
            domain_topic: vec![],
            exemplifies: vec![],
            entails: vec![],
            hypernym: vec![],
            instance_hypernym: vec![],
            mero_location: vec![],
            mero_member: vec![],
            mero_part: vec![],
            mero_portion: vec![],
            mero_substance: vec![],
            meronym: vec![],
            similar: vec![],
            feminine: vec![],
            masculine: vec![],
            other: vec![],
            hyponym: vec![],
            is_caused_by: vec![],
            has_domain_region: vec![],
            has_domain_topic: vec![],
            is_exemplified_by: vec![],
            is_entailed_by: vec![],
            instance_hyponym: vec![],
            holo_location: vec![],
            holo_member: vec![],
            holo_part: vec![],
            holo_portion: vec![],
            holo_substance: vec![],
            holonym: vec![],
            antonym: vec![],
            participle: vec![],
            is_participle_of: vec![],
            pertainym: vec![],
            derivation: vec![],
            domain_topic_sense: vec![],
            has_domain_topic_sense: vec![],
            domain_region_sense: vec![],
            has_domain_region_sense: vec![],
            exemplifies_sense: vec![],
            is_exemplified_by_sense: vec![],
            agent: vec![],
            is_agent_of: vec![],
            material: vec![],
            is_material_of: vec![],
            event: vec![],
            is_event_of: vec![],
            instrument: vec![],
            is_instrument_of: vec![],
            location: vec![],
            is_location_of: vec![],
            by_means_of: vec![],
            is_by_means_of: vec![],
            undergoer: vec![],
            is_undergoer_of: vec![],
            property: vec![],
            is_property_of: vec![],
            result: vec![],
            is_result_of: vec![],
            state: vec![],
            is_state_of: vec![],
            uses: vec![],
            is_used_by: vec![],
            destination: vec![],
            is_destination_of: vec![],
            body_part: vec![],
            is_body_part_of: vec![],
            vehicle: vec![],
            is_vehicle_of: vec![],
        }
    }

    #[test]
    fn test_strip_synset_id_prefix_oewn() {
        assert_eq!(
            strip_synset_id_prefix("oewn-00001740-n", "oewn"),
            Some("00001740-n")
        );
    }

    #[test]
    fn test_strip_synset_id_prefix_ewn() {
        assert_eq!(
            strip_synset_id_prefix("ewn-00001740-n", "oewn"),
            Some("00001740-n")
        );
    }

    #[test]
    fn test_strip_synset_id_prefix_none() {
        assert_eq!(strip_synset_id_prefix("00001740-n", "oewn"), None);
    }

    #[test]
    fn test_negotiate_html() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"
                .parse()
                .unwrap(),
        );
        assert!(matches!(negotiate(headers), ContentType::HTML));
    }

    #[test]
    fn test_negotiate_rdf_xml() {
        let mut headers = HeaderMap::new();
        headers.insert("Accept", "application/rdf+xml".parse().unwrap());
        assert!(matches!(negotiate(headers), ContentType::RDFXML));
    }

    #[test]
    fn test_negotiate_turtle() {
        let mut headers = HeaderMap::new();
        headers.insert("Accept", "text/turtle".parse().unwrap());
        assert!(matches!(negotiate(headers), ContentType::Turtle));
    }

    #[test]
    fn test_negotiate_json() {
        let mut headers = HeaderMap::new();
        headers.insert("Accept", "application/json".parse().unwrap());
        assert!(matches!(negotiate(headers), ContentType::JSON));
    }

    #[test]
    fn test_negotiate_xml() {
        let mut headers = HeaderMap::new();
        headers.insert("Accept", "application/xml".parse().unwrap());
        assert!(matches!(negotiate(headers), ContentType::XML));
    }

    #[test]
    fn test_negotiate_quality_preference() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Accept",
            "text/html;q=0.5,application/rdf+xml;q=0.9".parse().unwrap(),
        );
        assert!(matches!(negotiate(headers), ContentType::RDFXML));
    }

    #[test]
    fn test_negotiate_default() {
        let headers = HeaderMap::new();
        assert!(matches!(negotiate(headers), ContentType::HTML));
    }

    #[test]
    fn test_ontolex_namespace() {
        let result = ontolex("LexicalEntry").unwrap();
        assert_eq!(
            result.as_str(),
            "http://www.w3.org/ns/lemon/ontolex#LexicalEntry"
        );
    }

    #[test]
    fn test_wn_namespace() {
        let result = wn("hypernym").unwrap();
        assert_eq!(
            result.as_str(),
            "https://globalwordnet.github.io/schemas/wn#hypernym"
        );
    }

    #[test]
    fn test_build_url_without_fragment() {
        let result = build_url("https://example.com", "synset", "12345-n", None).unwrap();
        assert_eq!(result.as_str(), "https://example.com/synset/12345-n");
    }

    #[test]
    fn test_build_url_with_fragment() {
        let result = build_url("https://example.com", "lemma", "dog-n", Some("12345-n")).unwrap();
        assert_eq!(result.as_str(), "https://example.com/lemma/dog-n#12345-n");
    }

    #[test]
    fn test_build_url_encoding() {
        let result = build_url(
            "https://example.com",
            "synset",
            "test id",
            Some("test#fragment"),
        )
        .unwrap();
        assert_eq!(
            result.as_str(),
            "https://example.com/synset/test%20id#test%23fragment"
        );
    }

    #[test]
    fn test_lemma_id_simple() {
        let pos_key = PosKey::new("n");
        let result = lemma_id("dog", &pos_key);
        assert_eq!(result, "dog-n");
    }

    #[test]
    fn test_lemma_id_with_spaces() {
        let pos_key = PosKey::new("n");
        let result = lemma_id("hot dog", &pos_key);
        assert_eq!(result, "hot_dog-n");
    }

    #[test]
    fn test_gen_synset_rdf_basic() {
        let synset_id = SynsetId::new_owned("12345-n".to_string());
        let synset = test_synset(
            synset_id,
            "noun.test".to_string(),
            vec!["test definition".to_string()],
            PartOfSpeech::n,
        );

        let result = gen_synset_rdf(
            RdfFormat::RdfXml,
            "https://creativecommons.org/licenses/by/4.0/",
            "https://en-word.net/",
            "en",
            &synset,
        );

        assert!(result.is_ok());
        let rdf_data = result.unwrap();
        let rdf_string = String::from_utf8_lossy(&rdf_data);
        assert!(rdf_string.contains("12345-n"));
        assert!(rdf_string.contains("test definition"));
    }

    #[test]
    fn test_gen_synset_turtle_basic() {
        let synset_id = SynsetId::new_owned("12345-n".to_string());
        let synset = test_synset(
            synset_id,
            "noun.test".to_string(),
            vec!["test definition".to_string()],
            PartOfSpeech::n,
        );

        let result = gen_synset_rdf(
            RdfFormat::Turtle,
            "https://creativecommons.org/licenses/by/4.0/",
            "https://en-word.net/",
            "en",
            &synset,
        );

        assert!(result.is_ok());
        let rdf_data = result.unwrap();
        let rdf_string = String::from_utf8_lossy(&rdf_data);
        assert!(rdf_string.contains("12345-n"));
        assert!(rdf_string.contains("test definition"));
    }
}
