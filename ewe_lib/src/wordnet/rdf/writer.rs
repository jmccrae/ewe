use super::{RdfExportError, RdfExportOptions};
use crate::wordnet::{Lexicon, LexiconMetadata, MemberSynset, PosKey, SenseRelation, SynsetId, Synsets};
use oxrdf::vocab::rdf;
use oxrdf::*;
use oxrdfio::{RdfSerializer, WriterQuadSerializer};
use percent_encoding::{utf8_percent_encode, CONTROLS};
use std::collections::{BTreeMap, BTreeSet};

type Result<T> = std::result::Result<T, RdfExportError>;

/// Export every synset in `wn` as a single whole-lexicon RDF document, with a `lime:Lexicon`
/// header describing `options.site` (see the module doc comment for the entry-dedup guarantee
/// this - and [`write_lexicon_rdf_subset`] - provide).
pub fn write_lexicon_rdf<L: Lexicon>(wn: &L, options: &RdfExportOptions) -> Result<Vec<u8>> {
    let mut synsets = Vec::new();
    for lexfile in wn.synsets_iter()? {
        let (_, syns) = lexfile?;
        for entry in syns.iter()? {
            let (id, synset) = entry?;
            synsets.push(MemberSynset::from_synset(&id, synset.into_owned(), wn)?);
        }
    }
    let frames = wn.frames_get()?;
    write_rdf(&synsets, &frames, options, true)
}

/// Export exactly the given synsets, deduplicating `LexicalEntry` declarations across them the
/// same way [`write_lexicon_rdf`] does, but without the whole-lexicon `lime:Lexicon` header.
/// `frames` is the subcategorization-frame table (`Lexicon::frames_get`) that `synsem:synBehavior`
/// links are built from - pass `&[]` if the caller has no frame table (matches
/// `wordnet::xml::write_lexicon_xml_subset`'s equivalent parameter).
pub fn write_lexicon_rdf_subset(
    synsets: &[MemberSynset],
    frames: &[(String, String)],
    options: &RdfExportOptions,
) -> Result<Vec<u8>> {
    write_rdf(synsets, frames, options, false)
}

/// Per-(lemma, pos) accumulator for the entry-declaration pass: every synset the entry has a
/// sense in (for `ontolex:sense` links) and the union of subcat frame ids across those senses
/// (for `synsem:synBehavior` links).
#[derive(Default)]
struct EntryAcc<'a> {
    synset_ids: Vec<&'a str>,
    subcat: BTreeSet<&'a str>,
}

fn write_rdf(
    synsets: &[MemberSynset],
    frames: &[(String, String)],
    options: &RdfExportOptions,
    with_header: bool,
) -> Result<Vec<u8>> {
    let site = &options.site;
    let metadata = &options.metadata;

    let mut serializer = RdfSerializer::from_format(options.format)
        .with_prefix("rdf", "http://www.w3.org/1999/02/22-rdf-syntax-ns#")?
        .with_prefix("cc", "http://creativecommons.org/ns#")?
        .with_prefix("dc", "http://purl.org/dc/terms/")?
        .with_prefix("ili", "http://ili.globalwordnet.org/ili/")?
        .with_prefix("lime", "http://www.w3.org/ns/lemon/lime#")?
        .with_prefix("ontolex", "http://www.w3.org/ns/lemon/ontolex#")?
        .with_prefix("owl", "http://www.w3.org/2002/07/owl#")?
        .with_prefix("rdfs", "http://www.w3.org/2000/01/rdf-schema#")?
        .with_prefix("schema", "http://schema.org/")?
        .with_prefix("skos", "http://www.w3.org/2004/02/skos/core#")?
        .with_prefix("synsem", "http://www.w3.org/ns/lemon/synsem#")?
        .with_prefix("vartrans", "http://www.w3.org/ns/lemon/vartrans#")?
        .with_prefix("wn", "https://globalwordnet.github.io/schemas/wn#")?
        .with_prefix("wikidata", "http://www.wikidata.org/entity/")?
        .with_prefix("wordnet", site)?
        .with_prefix("wordnetlicense", &metadata.license)?
        .with_prefix("wordnetlemma", &section_uri(site, "lemma"))?
        .with_prefix("wordnetsynset", &section_uri(site, "synset"))?
        .with_prefix("wordnetframe", &section_uri(site, "frame"))?
        .for_writer(Vec::new());

    // Sort synsets by id so a whole-lexicon export (built from a `HashMap`-backed `Lexicon`
    // with no defined iteration order) is deterministic and diffable release-to-release -
    // matches the WN-LMF XML export's precedent (see `wordnet::xml::writer`).
    let mut synsets_sorted: Vec<&MemberSynset> = synsets.iter().collect();
    synsets_sorted.sort_by(|a, b| a.id.cmp(&b.id));

    // Pass 1: every unique (lemma, pos) across all given synsets gets its `LexicalEntry`
    // triples written exactly once, followed by one `ontolex:sense` link per synset it
    // actually has a sense in - this is what makes re-declaring an entry once per sense (the
    // bug the old `rapper` dedup pass papered over) impossible by construction.
    let mut entries: BTreeMap<(String, PosKey), EntryAcc> = BTreeMap::new();
    for synset in &synsets_sorted {
        for member in &synset.members {
            let acc = entries
                .entry((member.lemma.clone(), member.poskey.clone()))
                .or_default();
            acc.synset_ids.push(synset.id.as_str());
            acc.subcat.extend(member.sense.subcat.iter().map(|s| s.as_str()));
        }
    }
    for ((lemma, poskey), acc) in &entries {
        write_entry_triples(
            &mut serializer,
            site,
            &metadata.language,
            lemma,
            poskey,
            &acc.synset_ids,
            &acc.subcat,
        )?;
    }

    // Pass 2: per-sense relation triples and per-synset triples (definitions, examples,
    // relations, ...) - unchanged in shape from a single-synset export, since senses are
    // already unique to their own synset and can't be duplicated this way.
    for synset in &synsets_sorted {
        write_synset_triples(&mut serializer, site, synset)?;
    }

    // The frame table is shared/global, not per-entry - like `LexicalEntry` declarations,
    // write each frame's `rdfs:label` exactly once regardless of how many entries reference it.
    write_frame_triples(&mut serializer, site, frames)?;

    if with_header {
        let entry_uris: Vec<NamedNode> = entries
            .keys()
            .map(|(lemma, poskey)| build_url(site, "lemma", &lemma_id(lemma, poskey), None))
            .collect::<Result<_>>()?;
        write_lexicon_header(
            &mut serializer,
            site,
            metadata,
            entries.len(),
            synsets_sorted.len(),
            &entry_uris,
        )?;
    }

    Ok(serializer.finish()?)
}

/// The `lime:Lexicon`/`ontolex:ConceptSet` header, matching
/// <https://globalwordnet.github.io/schemas/#rdf>'s example Lexicon-level metadata block.
fn write_lexicon_header<W: std::io::Write>(
    serializer: &mut WriterQuadSerializer<W>,
    site: &str,
    metadata: &LexiconMetadata,
    entry_count: usize,
    concept_count: usize,
    entry_uris: &[NamedNode],
) -> Result<()> {
    let subject = NamedNodeRef::new(site)?;
    serializer.serialize_triple(TripleRef::new(subject, rdf::TYPE, &lime("Lexicon")?))?;
    serializer.serialize_triple(TripleRef::new(subject, rdf::TYPE, &ontolex("ConceptSet")?))?;
    if !metadata.label.is_empty() {
        serializer.serialize_triple(TripleRef::new(
            subject,
            &rdfs("label")?,
            LiteralRef::new_language_tagged_literal_unchecked(&metadata.label, &metadata.language),
        ))?;
    }
    serializer.serialize_triple(TripleRef::new(
        subject,
        &dc("language")?,
        LiteralRef::new_simple_literal(&metadata.language),
    ))?;
    serializer.serialize_triple(TripleRef::new(
        subject,
        &cc("license")?,
        NamedNodeRef::new(&metadata.license)?,
    ))?;
    if !metadata.version.is_empty() {
        serializer.serialize_triple(TripleRef::new(
            subject,
            &owl("versionInfo")?,
            LiteralRef::new_simple_literal(&metadata.version),
        ))?;
    }
    if let Some(email) = &metadata.email {
        serializer.serialize_triple(TripleRef::new(
            subject,
            &schema("email")?,
            LiteralRef::new_simple_literal(email),
        ))?;
    }
    if let Some(url) = &metadata.url {
        serializer.serialize_triple(TripleRef::new(subject, &schema("url")?, NamedNodeRef::new(url)?))?;
    }
    serializer.serialize_triple(TripleRef::new(
        subject,
        &lime("lexicalEntries")?,
        &Literal::from(entry_count as i64),
    ))?;
    serializer.serialize_triple(TripleRef::new(
        subject,
        &lime("concepts")?,
        &Literal::from(concept_count as i64),
    ))?;
    for entry in entry_uris {
        serializer.serialize_triple(TripleRef::new(subject, &lime("entry")?, entry))?;
    }
    Ok(())
}

fn write_entry_triples<W: std::io::Write>(
    serializer: &mut WriterQuadSerializer<W>,
    site: &str,
    language: &str,
    lemma: &str,
    poskey: &PosKey,
    synset_ids: &[&str],
    subcat: &BTreeSet<&str>,
) -> Result<()> {
    let entry = build_url(site, "lemma", &lemma_id(lemma, poskey), None)?;
    let pos = poskey
        .to_part_of_speech()
        .map(|pos| pos.long_pos())
        .unwrap_or("unknown");
    serializer.serialize_triple(TripleRef::new(&entry, rdf::TYPE, &ontolex("LexicalEntry")?))?;
    // `ontolex:canonicalForm`'s range is `ontolex:Form`, not a literal - the written
    // representation itself hangs off that `Form` resource as `ontolex:writtenRep`.
    let canonical_form = build_url(site, "lemma", &lemma_id(lemma, poskey), Some("canonicalForm"))?;
    serializer.serialize_triple(TripleRef::new(&entry, &ontolex("canonicalForm")?, &canonical_form))?;
    serializer.serialize_triple(TripleRef::new(&canonical_form, rdf::TYPE, &ontolex("Form")?))?;
    serializer.serialize_triple(TripleRef::new(
        &canonical_form,
        &ontolex("writtenRep")?,
        LiteralRef::new_language_tagged_literal_unchecked(lemma, language),
    ))?;
    serializer.serialize_triple(TripleRef::new(&entry, &wn("partOfSpeech")?, &wn(pos)?))?;
    for synset_id in synset_ids {
        let sense = build_url(site, "lemma", &lemma_id(lemma, poskey), Some(synset_id))?;
        serializer.serialize_triple(TripleRef::new(&entry, &ontolex("sense")?, &sense))?;
    }
    for code in subcat {
        let frame = build_url(site, "frame", code, None)?;
        serializer.serialize_triple(TripleRef::new(&entry, &synsem("synBehavior")?, &frame))?;
    }
    Ok(())
}

/// Writes each frame's `rdfs:label` exactly once - `frames` is the whole shared table
/// (`Lexicon::frames_get`), not filtered to only the codes actually referenced, matching how
/// `wordnet::xml::writer` emits every `SyntacticBehaviour` regardless of use.
fn write_frame_triples<W: std::io::Write>(
    serializer: &mut WriterQuadSerializer<W>,
    site: &str,
    frames: &[(String, String)],
) -> Result<()> {
    for (id, label) in frames {
        let frame = build_url(site, "frame", id, None)?;
        serializer.serialize_triple(TripleRef::new(
            &frame,
            &rdfs("label")?,
            LiteralRef::new_language_tagged_literal_unchecked(label, "en"),
        ))?;
    }
    Ok(())
}

/// Writes the direct `wn:{rel_name}` triple between a sense and its relation target, plus the
/// reified form the schema documents (<https://globalwordnet.github.io/schemas/#rdf>):
/// `[] a ontolex:SenseRelation ; vartrans:source ; vartrans:category wn:{rel_name} ;
/// vartrans:target .` - kept alongside the direct triple (not instead of it) since real GWA
/// RDF releases have historically used the direct form and consumers may depend on it.
/// `blank_id` must be unique per relation triple written (see call sites for how it's built).
fn write_sense_relation_triples<W: std::io::Write>(
    serializer: &mut WriterQuadSerializer<W>,
    source: &NamedNode,
    rel_name: &str,
    target: &NamedNode,
    blank_id: String,
) -> Result<()> {
    let category = wn(rel_name)?;
    serializer.serialize_triple(TripleRef::new(source, &category, target))?;
    let reified = BlankNode::new_unchecked(blank_id);
    serializer.serialize_triple(TripleRef::new(&reified, rdf::TYPE, &ontolex("SenseRelation")?))?;
    serializer.serialize_triple(TripleRef::new(&reified, &vartrans("source")?, source))?;
    serializer.serialize_triple(TripleRef::new(&reified, &vartrans("category")?, &category))?;
    serializer.serialize_triple(TripleRef::new(&reified, &vartrans("target")?, target))?;
    Ok(())
}

/// Synset-relation counterpart of [`write_sense_relation_triples`] - the schema's synset-level
/// reification example has no `rdf:type` on the blank node (unlike the sense-level one), so
/// this doesn't emit one either.
fn write_synset_relation_triples<W: std::io::Write>(
    serializer: &mut WriterQuadSerializer<W>,
    source: &NamedNode,
    rel_name: &str,
    target: &NamedNode,
    blank_id: String,
) -> Result<()> {
    let category = wn(rel_name)?;
    serializer.serialize_triple(TripleRef::new(source, &category, target))?;
    let reified = BlankNode::new_unchecked(blank_id);
    serializer.serialize_triple(TripleRef::new(&reified, &vartrans("source")?, source))?;
    serializer.serialize_triple(TripleRef::new(&reified, &vartrans("category")?, &category))?;
    serializer.serialize_triple(TripleRef::new(&reified, &vartrans("target")?, target))?;
    Ok(())
}

fn write_synset_triples<W: std::io::Write>(
    serializer: &mut WriterQuadSerializer<W>,
    site: &str,
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

    let ss = build_url(site, "synset", synset.id.as_str(), None)?;

    for member in &synset.members {
        let sense = build_url(
            site,
            "lemma",
            &lemma_id(&member.lemma, &member.poskey),
            Some(synset.id.as_str()),
        )?;
        triple!(&sense, rdf::TYPE, &ontolex("LexicalSense")?);
        triple!(&sense, &ontolex("isLexicalizedSenseOf")?, &ss);
        macro_rules! sense_rel {
            ($rel_type:ident, $rel_name:expr) => {
                // `MemberSynset::from_synset` builds these lists (particularly the
                // reverse/`is_X_of` ones) from `HashMap` iteration, so their order isn't
                // stable run-to-run - sort by target identity here so a whole-lexicon
                // export is byte-reproducible (useful for diffing release-to-release).
                let mut rels: Vec<&SenseRelation> = synset
                    .$rel_type
                    .iter()
                    .filter(|rel| rel.source_lemma == member.lemma)
                    .collect();
                rels.sort_by(|a, b| {
                    (&a.target_synset, &a.target_lemma, &a.target_poskey)
                        .cmp(&(&b.target_synset, &b.target_lemma, &b.target_poskey))
                });
                for (i, rel) in rels.iter().enumerate() {
                    // domain_topic/domain_region/exemplifies/other can target
                    // either a sense or a bare synset; when there's no
                    // specific sense, link straight to the synset resource.
                    let target = match (&rel.target_lemma, &rel.target_poskey) {
                        (Some(target_lemma), Some(target_poskey)) => build_url(
                            site,
                            "lemma",
                            &lemma_id(target_lemma, target_poskey),
                            Some(rel.target_synset.as_str()),
                        )?,
                        _ => build_url(site, "synset", rel.target_synset.as_str(), None)?,
                    };
                    write_sense_relation_triples(
                        serializer,
                        &sense,
                        $rel_name,
                        &target,
                        format!(
                            "relsense-{}-{}-{}-{}",
                            lemma_id(&member.lemma, &member.poskey),
                            synset.id.as_str(),
                            $rel_name,
                            i
                        ),
                    )?;
                }
            };
        }

        sense_rel!(antonym, "antonym");
        sense_rel!(also_sense, "also");
        sense_rel!(similar_sense, "similar");
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

    for (i, defn) in synset.definition.iter().enumerate() {
        let bn = BlankNode::new_unchecked(format!("def-{}-{}", synset.id.as_str(), i));
        triple!(&ss, &wn("definition")?, &bn);
        triple!(&bn, rdf::VALUE, lang_lit!(defn, "en"));
    }

    for (i, example) in synset.example.iter().enumerate() {
        let bn = BlankNode::new_unchecked(format!("ex-{}-{}", synset.id.as_str(), i));
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
            let mut rels: Vec<&SynsetId> = synset.$rel_type.iter().collect();
            rels.sort();
            for (i, rel) in rels.iter().enumerate() {
                let target = build_url(site, "synset", rel.as_str(), None)?;
                write_synset_relation_triples(
                    serializer,
                    &ss,
                    $rel_name,
                    &target,
                    format!("relsynset-{}-{}-{}", synset.id.as_str(), $rel_name, i),
                )?;
            }
        };
    }
    synset_rel!(also, "also");
    synset_rel!(attribute, "attribute");
    synset_rel!(causes, "causes");
    synset_rel!(is_caused_by, "isCausedBy");
    synset_rel!(domain_region, "domainRegion");
    synset_rel!(has_domain_region, "hasDomainRegion");
    synset_rel!(domain_topic, "domainTopic");
    synset_rel!(has_domain_topic, "hasDomainTopic");
    synset_rel!(exemplifies, "exemplifies");
    synset_rel!(is_exemplified_by, "isExemplifiedBy");
    synset_rel!(entails, "entails");
    synset_rel!(is_entailed_by, "isEntailedBy");
    synset_rel!(hypernym, "hypernym");
    synset_rel!(hyponym, "hyponym");
    synset_rel!(instance_hypernym, "instanceHypernym");
    synset_rel!(instance_hyponym, "instanceHyponym");
    synset_rel!(mero_location, "meroLocation");
    synset_rel!(holo_location, "holoLocation");
    synset_rel!(mero_member, "meroMember");
    synset_rel!(holo_member, "holoMember");
    synset_rel!(mero_part, "meroPart");
    synset_rel!(holo_part, "holoPart");
    synset_rel!(mero_portion, "meroPortion");
    synset_rel!(holo_portion, "holoPortion");
    synset_rel!(mero_substance, "meroSubstance");
    synset_rel!(holo_substance, "holoSubstance");
    synset_rel!(meronym, "meronym");
    synset_rel!(holonym, "holonym");
    synset_rel!(similar, "similar");
    synset_rel!(feminine, "feminine");
    synset_rel!(masculine, "masculine");
    synset_rel!(other, "other");

    Ok(())
}

fn ontolex(id: &str) -> Result<NamedNode> {
    Ok(NamedNode::new(format!(
        "http://www.w3.org/ns/lemon/ontolex#{}",
        id
    ))?)
}

fn wn(id: &str) -> Result<NamedNode> {
    Ok(NamedNode::new(format!(
        "https://globalwordnet.github.io/schemas/wn#{}",
        id
    ))?)
}

fn skos(id: &str) -> Result<NamedNode> {
    Ok(NamedNode::new(format!(
        "http://www.w3.org/2004/02/skos/core#{}",
        id
    ))?)
}

fn dc(id: &str) -> Result<NamedNode> {
    Ok(NamedNode::new(format!("http://purl.org/dc/terms/{}", id))?)
}

fn ili(id: &str) -> Result<NamedNode> {
    Ok(NamedNode::new(format!(
        "http://ili.globalwordnet.org/ili/{}",
        id
    ))?)
}

fn owl(id: &str) -> Result<NamedNode> {
    Ok(NamedNode::new(format!(
        "http://www.w3.org/2002/07/owl#{}",
        id
    ))?)
}

fn wikidata(id: &str) -> Result<NamedNode> {
    Ok(NamedNode::new(format!(
        "http://www.wikidata.org/entity/{}",
        id
    ))?)
}

fn lime(id: &str) -> Result<NamedNode> {
    Ok(NamedNode::new(format!(
        "http://www.w3.org/ns/lemon/lime#{}",
        id
    ))?)
}

fn rdfs(id: &str) -> Result<NamedNode> {
    Ok(NamedNode::new(format!(
        "http://www.w3.org/2000/01/rdf-schema#{}",
        id
    ))?)
}

fn schema(id: &str) -> Result<NamedNode> {
    Ok(NamedNode::new(format!("http://schema.org/{}", id))?)
}

fn cc(id: &str) -> Result<NamedNode> {
    Ok(NamedNode::new(format!(
        "http://creativecommons.org/ns#{}",
        id
    ))?)
}

fn vartrans(id: &str) -> Result<NamedNode> {
    Ok(NamedNode::new(format!(
        "http://www.w3.org/ns/lemon/vartrans#{}",
        id
    ))?)
}

fn synsem(id: &str) -> Result<NamedNode> {
    Ok(NamedNode::new(format!(
        "http://www.w3.org/ns/lemon/synsem#{}",
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

pub(crate) fn lemma_id(lemma: &str, pos_key: &PosKey) -> String {
    format!("{}-{}", lemma.replace(" ", "_"), pos_key.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::RdfFormat;
    use crate::wordnet::synset_members::{Member, MemberSense};
    use crate::wordnet::{PartOfSpeech, Pronunciation, SenseId, SynsetId};

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
            also_sense: vec![],
            similar_sense: vec![],
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
            other_sense: vec![],
        }
    }

    fn test_options(format: RdfFormat) -> RdfExportOptions {
        RdfExportOptions {
            format,
            site: "https://example.com/".to_owned(),
            metadata: LexiconMetadata {
                id_prefix: String::new(),
                label: "Example Wordnet".to_owned(),
                language: "en".to_owned(),
                email: None,
                license: "https://creativecommons.org/licenses/by/4.0/".to_owned(),
                version: "1".to_owned(),
                url: None,
            },
        }
    }

    fn member(lemma: &str, poskey: &str, sense_id: &str) -> Member {
        Member {
            lemma: lemma.to_owned(),
            sense: MemberSense {
                id: SenseId::new(sense_id.to_owned()),
                subcat: vec![],
                adjposition: None,
            },
            form: vec![],
            pronunciation: Vec::<Pronunciation>::new(),
            poskey: PosKey::new(poskey),
            entry_no: None,
        }
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
    fn test_write_lexicon_rdf_subset_basic() {
        let synset_id = SynsetId::new_owned("12345-n".to_string());
        let synset = test_synset(
            synset_id,
            "noun.test".to_string(),
            vec!["test definition".to_string()],
            PartOfSpeech::n,
        );

        let result = write_lexicon_rdf_subset(&[synset], &[], &test_options(RdfFormat::Turtle));

        assert!(result.is_ok());
        let rdf_data = result.unwrap();
        let rdf_string = String::from_utf8_lossy(&rdf_data);
        assert!(rdf_string.contains("12345-n"));
        assert!(rdf_string.contains("test definition"));
    }

    #[test]
    fn test_canonical_form_is_a_resource_not_a_literal() {
        // `ontolex:canonicalForm`'s range is `ontolex:Form`, so its object must be a
        // resource (blank node or named node) carrying `ontolex:writtenRep`, never a
        // bare literal directly.
        let mut synset = test_synset(
            SynsetId::new_owned("12345-n".to_string()),
            "noun.test".to_string(),
            vec!["test definition".to_string()],
            PartOfSpeech::n,
        );
        synset.members.push(member("dog", "n", "dog%1:05:00::"));

        let rdf_data =
            write_lexicon_rdf_subset(&[synset], &[], &test_options(RdfFormat::Turtle)).unwrap();
        let rdf_string = String::from_utf8_lossy(&rdf_data);

        assert!(
            !rdf_string.contains("ontolex:canonicalForm \"dog\"@en"),
            "canonicalForm must not point directly at a literal, got:\n{}",
            rdf_string
        );
        assert!(rdf_string.contains("a ontolex:Form"));
        assert!(rdf_string.contains("ontolex:writtenRep \"dog\"@en"));
    }

    #[test]
    fn test_write_lexicon_rdf_subset_rdfxml() {
        let synset_id = SynsetId::new_owned("12345-n".to_string());
        let synset = test_synset(
            synset_id,
            "noun.test".to_string(),
            vec!["test definition".to_string()],
            PartOfSpeech::n,
        );

        let result = write_lexicon_rdf_subset(&[synset], &[], &test_options(RdfFormat::RdfXml));

        assert!(result.is_ok());
        let rdf_data = result.unwrap();
        let rdf_string = String::from_utf8_lossy(&rdf_data);
        assert!(rdf_string.contains("12345-n"));
        assert!(rdf_string.contains("test definition"));
    }

    #[test]
    fn test_multi_sense_entry_declared_once() {
        // A lemma with senses in two different synsets must get exactly one
        // `LexicalEntry` declaration (the bug the old `rapper` dedup pass
        // papered over - see the module doc comment).
        let mut synset_a = test_synset(
            SynsetId::new_owned("00000001-n".to_string()),
            "noun.test".to_string(),
            vec!["first sense".to_string()],
            PartOfSpeech::n,
        );
        synset_a.members.push(member("run", "n", "run%1:01:00::"));

        let mut synset_b = test_synset(
            SynsetId::new_owned("00000002-n".to_string()),
            "noun.test".to_string(),
            vec!["second sense".to_string()],
            PartOfSpeech::n,
        );
        synset_b.members.push(member("run", "n", "run%1:02:00::"));

        let rdf_data =
            write_lexicon_rdf_subset(&[synset_a, synset_b], &[], &test_options(RdfFormat::Turtle))
                .unwrap();
        let rdf_string = String::from_utf8_lossy(&rdf_data);

        assert_eq!(
            rdf_string.matches("ontolex:LexicalEntry").count(),
            1,
            "expected exactly one LexicalEntry declaration, got:\n{}",
            rdf_string
        );
        // ...but both senses are still linked from that one entry (Turtle abbreviates
        // repeated subject/predicate pairs as a comma-separated object list, so
        // "ontolex:sense" itself appears once even though both objects are present -
        // check the actual per-sense triples were still written twice instead).
        assert_eq!(rdf_string.matches("ontolex:LexicalSense").count(), 2);
        assert_eq!(rdf_string.matches("ontolex:isLexicalizedSenseOf").count(), 2);
    }

    #[test]
    fn test_inverse_synset_relations_are_emitted() {
        // `MemberSynset::from_synset` computes reverse relations (e.g. `hyponym` as the
        // inverse of `hypernym`) for display, but they're only asserted as real RDF
        // triples if the writer actually emits them - a plain triple store doesn't
        // infer `wn:hyponym` from `wn:hypernym` on its own.
        let mut synset = test_synset(
            SynsetId::new_owned("00000002-n".to_string()),
            "noun.test".to_string(),
            vec!["a hyponym".to_string()],
            PartOfSpeech::n,
        );
        synset.hyponym.push(SynsetId::new_owned("00000003-n".to_string()));
        synset.holonym.push(SynsetId::new_owned("00000004-n".to_string()));
        synset.has_domain_topic.push(SynsetId::new_owned("00000005-n".to_string()));

        let rdf_data = write_lexicon_rdf_subset(&[synset], &[], &test_options(RdfFormat::Turtle)).unwrap();
        let rdf_string = String::from_utf8_lossy(&rdf_data);

        assert!(rdf_string.contains("wn:hyponym"), "missing wn:hyponym, got:\n{}", rdf_string);
        assert!(rdf_string.contains("wn:holonym"), "missing wn:holonym, got:\n{}", rdf_string);
        assert!(
            rdf_string.contains("wn:hasDomainTopic"),
            "missing wn:hasDomainTopic, got:\n{}",
            rdf_string
        );
    }

    #[test]
    fn test_lexicon_header_present_on_whole_export_only() {
        let synset = test_synset(
            SynsetId::new_owned("12345-n".to_string()),
            "noun.test".to_string(),
            vec!["test definition".to_string()],
            PartOfSpeech::n,
        );

        let with_header = write_rdf(&[synset.clone()], &[], &test_options(RdfFormat::Turtle), true).unwrap();
        let without_header = write_rdf(&[synset], &[], &test_options(RdfFormat::Turtle), false).unwrap();

        assert!(String::from_utf8_lossy(&with_header).contains("lime:Lexicon"));
        assert!(!String::from_utf8_lossy(&without_header).contains("lime:Lexicon"));
    }

    #[test]
    fn test_lime_entry_links_every_lexical_entry() {
        // <https://globalwordnet.github.io/schemas/#rdf> links the Lexicon resource to
        // every LexicalEntry via `lime:entry` - only meaningful for the whole-lexicon
        // header, not a subset export (which has no Lexicon-level subject at all).
        let mut synset = test_synset(
            SynsetId::new_owned("12345-n".to_string()),
            "noun.test".to_string(),
            vec!["test definition".to_string()],
            PartOfSpeech::n,
        );
        synset.members.push(member("dog", "n", "dog%1:05:00::"));

        let rdf_data = write_rdf(&[synset], &[], &test_options(RdfFormat::Turtle), true).unwrap();
        let rdf_string = String::from_utf8_lossy(&rdf_data);

        assert!(
            rdf_string.contains("lime:entry wordnetlemma:dog-n"),
            "missing lime:entry link, got:\n{}",
            rdf_string
        );
    }

    #[test]
    fn test_metadata_uses_cc_license_and_dc_language() {
        // The schema's Lexicon-level example uses `cc:license`/`dc:language`, not
        // `dc:license`/`lime:language`.
        let synset = test_synset(
            SynsetId::new_owned("12345-n".to_string()),
            "noun.test".to_string(),
            vec!["test definition".to_string()],
            PartOfSpeech::n,
        );

        let rdf_data = write_rdf(&[synset], &[], &test_options(RdfFormat::Turtle), true).unwrap();
        let rdf_string = String::from_utf8_lossy(&rdf_data);

        assert!(rdf_string.contains("cc:license"), "got:\n{}", rdf_string);
        assert!(rdf_string.contains("dc:language"), "got:\n{}", rdf_string);
        assert!(!rdf_string.contains("dc:license"), "got:\n{}", rdf_string);
        assert!(!rdf_string.contains("lime:language"), "got:\n{}", rdf_string);
    }

    #[test]
    fn test_relations_are_also_reified_with_vartrans() {
        // The direct `wn:{rel}` triple is kept (real GWA RDF releases have historically
        // used it), but the schema also documents a reified `vartrans:source`/
        // `vartrans:category`/`vartrans:target` form alongside it - for both sense-level
        // and synset-level relations (only the sense-level blank node is typed).
        let mut synset_a = test_synset(
            SynsetId::new_owned("00000001-n".to_string()),
            "noun.test".to_string(),
            vec!["first".to_string()],
            PartOfSpeech::n,
        );
        synset_a.members.push(member("run", "n", "run%1:01:00::"));
        synset_a.hypernym.push(SynsetId::new_owned("00000002-n".to_string()));

        let synset_b = test_synset(
            SynsetId::new_owned("00000002-n".to_string()),
            "noun.test".to_string(),
            vec!["second".to_string()],
            PartOfSpeech::n,
        );

        let rdf_data =
            write_lexicon_rdf_subset(&[synset_a, synset_b], &[], &test_options(RdfFormat::Turtle))
                .unwrap();
        let rdf_string = String::from_utf8_lossy(&rdf_data);

        assert!(rdf_string.contains("vartrans:source"), "got:\n{}", rdf_string);
        assert!(rdf_string.contains("vartrans:category wn:hypernym"), "got:\n{}", rdf_string);
        assert!(rdf_string.contains("vartrans:target"), "got:\n{}", rdf_string);
        // still keeps the direct triple too
        assert!(rdf_string.contains("wn:hypernym wordnetsynset:00000002-n"), "got:\n{}", rdf_string);
    }

    #[test]
    fn test_syntactic_behavior_links_frame_and_declares_label() {
        let mut synset = test_synset(
            SynsetId::new_owned("00000010-v".to_string()),
            "verb.motion".to_string(),
            vec!["to pay".to_string()],
            PartOfSpeech::v,
        );
        let mut pay = member("pay", "v", "pay%2:40:00::");
        pay.sense.subcat = vec!["vtai".to_string()];
        synset.members.push(pay);

        let frames = vec![("vtai".to_string(), "Somebody ----s something".to_string())];

        let rdf_data =
            write_lexicon_rdf_subset(&[synset], &frames, &test_options(RdfFormat::Turtle)).unwrap();
        let rdf_string = String::from_utf8_lossy(&rdf_data);

        assert!(
            rdf_string.contains("synsem:synBehavior wordnetframe:vtai"),
            "got:\n{}",
            rdf_string
        );
        assert!(
            rdf_string.contains(r#"rdfs:label "Somebody ----s something"@en"#),
            "got:\n{}",
            rdf_string
        );
    }
}
