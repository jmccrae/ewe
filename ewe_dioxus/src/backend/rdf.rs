//! Implements the key RDF functionality of returning pages as RDF
//! using content negotiation
use crate::dioxus_fullstack::{HeaderMap, Redirect, http::Response, body::Body};
use dioxus::prelude::*;
use crate::dioxus_fullstack::response::IntoResponse;
use oewn_lib::wordnet::{MemberSynset, Lexicon, SynsetId, Synset, PosKey};
use oxrdf::*;
use oxrdf::vocab::rdf;
use oxrdfio::{RdfFormat, RdfSerializer};
use percent_encoding::{utf8_percent_encode, CONTROLS};
use std::borrow::Cow;

#[get("/synset/{id}", headers : HeaderMap)]
pub async fn synset_negotiated(id : String) -> Result<Response<Body>> {
    let content_type = negotiate(headers);
    let response = match content_type {
        ContentType::HTML => Redirect::to(&format!("/view/synset/{}", id)).into_response(),
        ContentType::RDFXML => Redirect::to(&format!("/rdf/synset/{}", id)).into_response(),
        ContentType::Turtle => Redirect::to(&format!("/turtle/synset/{}", id)).into_response(),
        ContentType::JSON => Redirect::to(&format!("/api/synset/{}", id)).into_response(),
    };
    Ok(response)
}

#[get("/rdf/synset/{id}")]
pub async fn synset_rdf(id : String) -> Result<Response<Body>> {
    let id = SynsetId::new_owned(id);
    let result : Result<Option<Vec<u8>>> = crate::LEXICON.get().as_ref()
        .and_then(|lexicon| {
            let synset2 : Option<Result<Cow<Synset>>> = 
                lexicon.synset_by_id(&id).transpose().map(|s| Ok(s?));
            let synset : Option<Result<Synset>> = synset2.map(|res| res.and_then(|cow| Ok(cow.into_owned())));
            let member_synset : Option<Result<MemberSynset>> = synset
                .map(|synset| Ok(MemberSynset::from_synset(&id, synset?, lexicon)?));

            eprintln!("Synset lookup for {}: {:?}", id, member_synset);
            let x : Option<Result<Vec<u8>>> = member_synset.map(|res| {
                let z : Result<Vec<u8>> = res.and_then(|ms|
                    gen_synset_rdf("https://creativecommons.org/licenses/by/4.0/", "https://en-word.net/", "en", &ms));
                z
            });
                
            x
        }).transpose();

    if let Ok(Some(rdf_xml)) = result {
        Ok(Response::builder()
            .header("Content-Type", "application/rdf+xml")
            .body(Body::from(rdf_xml))
            .unwrap())
    } else if let Err(e) = result {
        Ok(Response::builder()
            .status(500)
            .body(Body::from(format!("Internal server error: {}", e)))
            .unwrap())
    } else {
        Ok(Response::builder()
            .status(404)
            .body(Body::from("Synset not found"))
            .unwrap())
    }
}

enum ContentType {
    HTML,
    RDFXML,
    Turtle,
    JSON
}

fn negotiate(headers : HeaderMap) -> ContentType {
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

    types.into_iter().find_map(|(_, mime)| {
        match mime {
            "text/html" => Some(ContentType::HTML),
            "application/rdf+xml" => Some(ContentType::RDFXML),
            "text/turtle" => Some(ContentType::Turtle),
            "application/json" => Some(ContentType::JSON),
            "*/*" => Some(ContentType::HTML), // Default to HTML if any type is accepted
            _ => None,
        }
    }).unwrap_or(ContentType::HTML) // Default to HTML if no acceptable type is found
}

fn ontolex(id : &str) -> Result<NamedNode> {
    Ok(NamedNode::new(&format!("http://www.w3.org/ns/lemon/ontolex#{}", id))?)
}

fn wn(id : &str) -> Result<NamedNode> {
    Ok(NamedNode::new(&format!("https://globalwordnet.github.io/schemas/wn#{}", id))?)
}

fn skos(id : &str) -> Result<NamedNode> {
    Ok(NamedNode::new(&format!("http://www.w3.org/2004/02/skos/core#{}", id))?)
}

fn dc(id : &str) -> Result<NamedNode> {
    Ok(NamedNode::new(&format!("http://purl.org/dc/terms/{}", id))?)
}

fn ili(id : &str) -> Result<NamedNode> {
    Ok(NamedNode::new(&format!("http://ili.globalwordnet.org/ili/{}", id))?)
}

fn owl(id : &str) -> Result<NamedNode> {
    Ok(NamedNode::new(&format!("http://www.w3.org/2002/07/owl#{}", id))?)
}

fn wikidata(id : &str) -> Result<NamedNode> {
    Ok(NamedNode::new(&format!("http://www.wikidata.org/entity/{}", id))?)
}

const FRAGMENT_ENCODE_SET: &percent_encoding::AsciiSet = &CONTROLS.add(b'%');
const PATH_ENCODE_SET: &percent_encoding::AsciiSet = &CONTROLS.add(b'/').add(b'#').add(b'?').add(b'&').add(b'=').add(b'+').add(b'$').add(b',').add(b';').add(b':').add(b'@');

fn build_url(site : &str, section : &str, id : &str, fragment : Option<&str>) -> Result<NamedNode> {
    let site = if site.ends_with("/") {
        format!("{}{}/", site, section)
    } else {
        format!("{}/{}/", site, section)
    };
    let mut url = format!("{}{}", site, utf8_percent_encode(id, PATH_ENCODE_SET));
    if let Some(fragment) = fragment {
        url.push_str(&format!("#{}", utf8_percent_encode(fragment, FRAGMENT_ENCODE_SET)));
    }
    Ok(NamedNode::new(url)?)
}

fn gen_synset_rdf(license : &str, site : &str, language : &str, synset : &MemberSynset) -> Result<Vec<u8>> {
    let mut serializer = RdfSerializer::from_format(RdfFormat::RdfXml)
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
        .for_writer(Vec::new());

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
        let entry = build_url(site, "lemma", &lemma_id(&member.lemma, &member.poskey), None)?;
        let sense = build_url(site, "lemma", &lemma_id(&member.lemma, &member.poskey), Some(synset.id.as_str()))?;
        let pos = member.poskey.to_part_of_speech().map(|pos| pos.long_pos()).unwrap_or("unknown");
        triple!(&entry, rdf::TYPE, &ontolex("LexicalEntry")?);
        triple!(&entry, &ontolex("canonicalForm")?, lang_lit!(&member.lemma, language));
        triple!(&entry, &ontolex("sense")?, &sense);
        triple!(&entry, &wn("partOfSpeech")?, &wn(pos)?);
        triple!(&sense, rdf::TYPE, &ontolex("LexicalSense")?);
        triple!(&sense, &ontolex("isLexicalizedSenseOf")?, &ss);
        macro_rules! sense_rel {
            ($rel_type:ident, $rel_name:expr) => {
                for rel in &synset.$rel_type {
                    if rel.source_lemma == member.lemma {
                        let target = build_url(site, "lemma", &lemma_id(&rel.target_lemma, &rel.target_poskey), Some(rel.target_synset.as_str()))?;
                        triple!(&sense, &wn($rel_name)?, &target);
                    }
                }
            }
        }


        sense_rel!(antonym, "antonym");
        sense_rel!(participle, "participle");
        sense_rel!(is_participle_of, "isParticipleOf");
        sense_rel!(pertainym, "pertainym");
        sense_rel!(is_pertainym_of, "isPertainymOf");
        sense_rel!(derivation, "derivation");
        sense_rel!(exemplifies_sense,"exemplifies");
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

    triple!(&ss, &wn("partOfSpeech")?, &wn(synset.part_of_speech.long_pos())?);

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
        }
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

    Ok(serializer.finish()?)
}

fn lemma_id(lemma : &str, pos_key : &PosKey) -> String {
    format!("{}-{}", lemma.replace(" ", "_"), pos_key.as_str())
}



