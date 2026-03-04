use thiserror::Error;
use serde::{Serialize,Deserialize,Serializer,Deserializer};
use std::collections::{HashMap, BTreeMap};
use std::fs;
use std::path::Path;
use std::fs::File;
use std::fmt;
use serde::de::{self, Visitor, MapAccess};
use serde::ser::SerializeMap;
use redb::{TableDefinition, TypeName, Database};
use speedy::{Readable, Writable};
use std::fmt::{Formatter, Display};


const TABLE: TableDefinition<&str, MemberSynset> = TableDefinition::new("lexicon");

/// The Lexicon contains the whole WordNet graph
pub struct Lexicon {
    //pub synsets : HashMap<SynsetId, MemberSynset>,
    db: redb::Database,
    entries : HashMap<String, Vec<SynsetId>>,
    synsets_by_ili : HashMap<String, SynsetId>,
    pub synset_ids : Vec<SynsetId>
}

impl Lexicon {
    /// Create a new empty lexicon
    #[allow(dead_code)]
    pub fn new() -> Lexicon {
        Lexicon {
            entries: HashMap::new(),
            //synsets: HashMap::new(),
            db: Database::create("wordnet.db").unwrap(),
            synsets_by_ili: HashMap::new(),
            synset_ids : Vec::new()
        }
    }

    pub fn from_disk() -> Lexicon {
        let db = Database::open("wordnet.db").unwrap();
        let file = File::open("wordnet.data").unwrap();
        let (entries, synsets_by_ili, synset_ids) = 
            <(HashMap::<String, Vec<SynsetId>>, HashMap::<String, SynsetId>, Vec::<SynsetId>)>::read_from_stream_buffered(&file).unwrap();
        Lexicon {
            db,
            entries,
            synsets_by_ili,
            synset_ids
        }
    }

    /// Load a lexicon from a folder of YAML files
    pub fn load<P: AsRef<Path>>(folder : P) -> Result<Lexicon, WordNetYAMLIOError> {
        let mut entries : HashMap<String, Entries> = HashMap::new();
        let mut synsets = HashMap::new();
        let mut synset_id_to_lexfile = HashMap::new();
        let mut sense_id_to_lemma_pos = HashMap::new();
        let mut synsets_by_ili = HashMap::new();
        let folder_files = fs::read_dir(folder)
            .map_err(|e| WordNetYAMLIOError::Io(format!("Could not list directory: {}", e)))?;
        println!("Loading WordNet");
        //let bar = ProgressBar::new(74);
        for file in folder_files {
            let file = file.map_err(|e|
                WordNetYAMLIOError::Io(format!("Could not list directory: {}", e)))?;
            let file_name = file.path().file_name().
                and_then(|x| x.to_str()).
                map(|x| x.to_string()).
                unwrap_or_else(|| "".to_string());
            if file_name.starts_with("entries-") && file_name.ends_with(".yaml") {
                let key = file_name[8..9].to_string();
                let entries2 : Entries =
                    serde_yaml::from_reader(File::open(file.path())
                        .map_err(|e| WordNetYAMLIOError::Io(format!("Error reading {} due to {}", file_name, e)))?)
                        .map_err(|e| WordNetYAMLIOError::Serde(format!("Error reading {} due to {}", file_name, e)))?;
                for (lemma, map) in entries2.0.iter() {
                    for (pos, entry) in map.iter() {
                        for sense in entry.sense.iter() {
                            sense_id_to_lemma_pos.insert(sense.id.clone(),
                                (lemma.to_string(), pos.clone()));
                        }
                    }
                }

                let entries2 = entries2.0.into_iter().map(|(lemma, map)| {
                    (lemma.clone(), map.into_iter().map(|(pos, entry)| {
                        (pos.clone(), Entry {
                            poskey: Some(pos.clone()),
                            ..entry
                        })
                    }).collect::<BTreeMap<_,_>>())
                }).collect::<BTreeMap<_,_>>();

                entries.insert(key, Entries(entries2));
            } else if file_name.ends_with(".yaml") && file_name != "frames.yaml" {
                let synsets2 : Synsets = serde_yaml::from_reader(
                    File::open(file.path())
                        .map_err(|e| WordNetYAMLIOError::Io(format!("Error reading {} due to {}", file_name, e)))?)
                        .map_err(|e| WordNetYAMLIOError::Serde(format!("Error reading {} due to {}", file_name, e)))?;
                let lexname = file_name[0..file_name.len()-5].to_string();
                for (id, value) in synsets2.0.iter() {
                    synset_id_to_lexfile.insert(id.clone(), lexname.clone());
                    synsets_by_ili.insert(value.ili.clone().unwrap_or_else(|| ILIID("".to_string())).0.clone(), id.clone());
                }
                let synsets2 = synsets2.0.into_iter().map(|(ssid, synset)| {
                    (ssid.clone(), Synset {
                        id: Some(ssid.clone()),
                        lexname: Some(lexname.clone()),
                        ..synset
                    })
                }).collect::<BTreeMap<_,_>>();
                synsets.insert(lexname, Synsets(synsets2));
            }
            //bar.inc(1);
        }
       //bar.finish();
       add_reverse_links(&mut synsets, &entries, &synset_id_to_lexfile);
       Ok(add_members(synsets, &mut entries))
    }

    ///// Get the lexicographer file name for a synset
    //pub fn lex_name_for(&self, synset_id : &SynsetId) -> Option<String> {
    //    self.synset_id_to_lexfile.get(synset_id).map(|x| x.clone())
    //}

    /// Get the entry data for a lemma
    pub fn entry_by_lemma(&self, lemma : &str) -> Vec<SynsetId> {
        if let Some(e) = self.entries.get(lemma) {
            e.clone()
        } else {
            Vec::new()
        }
    }

    /// Get synset data by ID
    pub fn synset_by_id(&self, synset_id : &SynsetId) -> Option<MemberSynset> {
        let read_txn = self.db.begin_read().unwrap();
        let table = read_txn.open_table(TABLE).unwrap();
        table.get(synset_id.0.as_str()).unwrap().map(|x| x.value())
    }

    /// Get synset by ILI
    pub fn synset_by_ili(&self, ili : &str) -> Option<(&SynsetId, MemberSynset)> {
        match self.synsets_by_ili.get(ili) {
            Some(ssid) => if let Some(synset) = self.synset_by_id(ssid) {
                Some((ssid, synset))
            } else {
                None
            },
            None => None
        }
    }

    /// Get the lemmas that start with a string
    pub fn lemma_by_prefix(&self, prefix: &str) -> Vec<String> {
        let prefix = prefix.to_lowercase();
        self.entries.iter().filter(|(k, _)| k.to_lowercase().starts_with(&prefix)).map(|(k, _)| k.clone()).collect()
    }

    /// Get the synsets that start with an ID
    pub fn ssid_by_prefix(&self, prefix: &str) -> Vec<String> {
        self.synset_ids.iter().filter(|k| k.0.starts_with(prefix)).map(|k| k.0.clone()).collect()
    }

    /// Get the ILIs that start with a string
    pub fn ili_by_prefix(&self, prefix: &str) -> Vec<String> {
        self.synsets_by_ili.iter().filter(|(k, _)| k.starts_with(prefix)).map(|(k, _)| k.clone()).collect()
    }

}

fn synset_by_id_mut<'a>(synsets : &'a mut HashMap<String, Synsets>, synset_id : &SynsetId,
    synset_id_to_lexfile : &HashMap<SynsetId, String>) -> Option<&'a mut Synset> {
    match synset_id_to_lexfile.get(synset_id) {
        Some(lex_name) => {
            match synsets.get_mut(lex_name) {
                Some(sss) => {
                    sss.0.get_mut(synset_id)
                },
                None => None
            }
        },
        None => None
    }
}


/// Augment the lexicon with reverse and sense links
fn add_reverse_links(synsets : &mut HashMap<String, Synsets>, entries : &HashMap<String, Entries>,
        synset_id_to_lexfile : &HashMap<SynsetId, String>) {
    macro_rules! add_reverse_links {
        ($rel:ident, $inv:ident) => {
            let mut elems = Vec::new();
            for synsets in synsets.values() {
                for synset in synsets.0.values() {
                    for hyp in synset.$rel.iter() {
                        elems.push((synset.id.clone().unwrap(), hyp.clone()));
                    }
                }
            }
            for (child, parent) in elems {
                if let Some(parent_synset) = synset_by_id_mut(synsets, &parent, synset_id_to_lexfile) {
                    parent_synset.$inv.push(child.clone());
                }
            }
        }
    }

    add_reverse_links!(hypernym, hyponym);
    add_reverse_links!(instance_hypernym, instance_hyponym);
    add_reverse_links!(mero_member, holo_member);
    add_reverse_links!(mero_part, holo_part);
    add_reverse_links!(mero_substance, holo_substance);
    add_reverse_links!(causes, is_caused_by);
    add_reverse_links!(exemplifies, is_exemplified_by);
    add_reverse_links!(entails, is_entailed_by);

    let mut sense_ids = HashMap::new();
    for entries in entries.values() {
        for (lemma, by_pos) in entries.0.iter() {
            for entry in by_pos.values() {
                for sense in entry.sense.iter() {
                    sense_ids.insert(sense.id.clone(), (lemma.clone(), sense.synset.clone()));
                }
            }
        }
    }


    macro_rules! add_sense_links {
        ($rel:ident, $inv:ident) => {
            let mut elems = Vec::new();
            for entries in entries.values() {
                for (lemma, by_pos) in entries.0.iter() {
                    for entry in by_pos.values() {
                        for sense in entry.sense.iter() {
                            for target in sense.$rel.iter() {
                                if let Some((target_lemma, synset)) = sense_ids.get(target) {
                                    elems.push((
                                        sense.synset.clone(),
                                        synset.clone(),
                                        lemma.clone(),
                                        target_lemma.clone(),
                                    ));
                                }
                            }
                        }
                    }
                }
            }

            for (source_synset, target_synset, source_lemma, target_lemma) in elems {
                if let Some(synset) = synset_by_id_mut(synsets, &source_synset, synset_id_to_lexfile) {
                    synset.$rel.push(SenseRelation {
                        target_synset: target_synset.clone(),
                        source_lemma: source_lemma.clone(),
                        target_lemma: target_lemma.clone()
                    });
                }
                if let Some(synset) = synset_by_id_mut(synsets, &target_synset, synset_id_to_lexfile) {
                    synset.$inv.push(SenseRelation {
                        target_synset: source_synset,
                        source_lemma: target_lemma,
                        target_lemma: source_lemma
                    });
                }
            }
        }
    }
    add_sense_links!(antonym, antonym);
    add_sense_links!(participle, is_participle_of);
    add_sense_links!(pertainym, is_pertainym_of);
    add_sense_links!(derivation, derivation);
    //add_sense_links!(exemplifies_sense, is_exemplified_by_sense);
    {
        let mut elems = Vec::new();
        for entries in entries.values() {
            for (lemma, by_pos) in entries.0.iter() {
                for entry in by_pos.values() {
                    for sense in entry.sense.iter() {
                        for target in sense.exemplifies.iter() {
                            if let Some((target_lemma, synset)) = sense_ids.get(target) {
                                elems.push((
                                        sense.synset.clone(),
                                        synset.clone(),
                                        lemma.clone(),
                                        target_lemma.clone(),
                                ));
                            }
                        }
                    }
                }
            }
        }

        for (source_synset, target_synset, source_lemma, target_lemma) in elems {
            if let Some(synset) = synset_by_id_mut(synsets, &source_synset, synset_id_to_lexfile) {
                synset.exemplifies_sense.push(SenseRelation {
                    target_synset: target_synset.clone(),
                    source_lemma: source_lemma.clone(),
                    target_lemma: target_lemma.clone()
                });
            }
            if let Some(synset) = synset_by_id_mut(synsets, &target_synset, synset_id_to_lexfile) {
                synset.is_exemplified_by_sense.push(SenseRelation {
                    target_synset: source_synset,
                    source_lemma: target_lemma,
                    target_lemma: source_lemma
                });
            }
        }
    } 
    add_sense_links!(agent, is_agent_of);
    add_sense_links!(material, is_material_of);
    add_sense_links!(event, is_event_of);
    add_sense_links!(instrument, is_instrument_of);
    add_sense_links!(location, is_location_of);
    add_sense_links!(by_means_of, is_by_means_of);
    add_sense_links!(undergoer, is_undergoer_of);
    add_sense_links!(property, is_property_of);
    add_sense_links!(result, is_result_of);
    add_sense_links!(state, is_state_of);
    add_sense_links!(uses, is_used_by);
    add_sense_links!(destination, is_destination_of);
    add_sense_links!(body_part, is_body_part_of);
    add_sense_links!(vehicle, is_vehicle_of);
}

pub fn add_members(synsets : HashMap<String, Synsets>, entries : &HashMap<String, Entries>) -> Lexicon {
    let mut synset_members = HashMap::new();
    let mut entry_map = HashMap::new();
    let mut ili = HashMap::new();
    for (_, synsets) in synsets {
        for (_, synset) in synsets.0 {
            let id = synset.id.clone().unwrap();
            for member in synset.members.iter() {
                entry_map.entry(member.clone()).or_insert_with(Vec::new).push(id.clone());
            }
            if let Some(ili_id) = synset.ili.as_ref() {
                ili.insert(ili_id.0.clone(), id.clone());
            }
            let m = synset_with_members(synset, entries);
            synset_members.insert(id, m);
        }
    }
    std::fs::remove_file("wordnet.db").ok();
    let db = Database::create("wordnet.db").unwrap();
    let mut synset_ids = Vec::new();
    let write_txn = db.begin_write().unwrap();
    {
        let mut table = write_txn.open_table(TABLE).unwrap();
        for (id, synset) in synset_members {
            synset_ids.push(id.clone());
            table.insert(id.0.as_str(), synset).unwrap();
        }
    }
    write_txn.commit().unwrap();
    let mut data = File::create("wordnet.data").unwrap();
    (&entry_map, &ili, &synset_ids).write_to_stream(&mut data).unwrap();
    Lexicon {
        db,            
        entries: entry_map,
        synsets_by_ili: ili,
        synset_ids
    }
}

pub fn synset_with_members(synset : Synset, entries : &HashMap<String, Entries>) -> MemberSynset {
    let mut members = Vec::new();
    for m in synset.members.iter() {
        for entry in entries[&entry_key(m)].entry_by_lemma(m) {
            for sense in entry.sense.iter() {
                if Some(&sense.synset) != synset.id.as_ref() {
                    continue;
                }
                members.push(Member {
                    lemma: m.clone(),
                    sense: MemberSense {
                        id: sense.id.clone(),
                        subcat: sense.subcat.clone()
                    },
                    form: entry.form.clone(),
                    pronunciation: entry.pronunciation.clone(),
                    poskey: entry.poskey.clone(),
                    entry_no: entry.poskey.as_ref().and_then(|x| x.entry_no())
                });
            }
        }
    }
    MemberSynset {
        members,
        id: synset.id.unwrap(),
        lexname: synset.lexname.unwrap(),
        definition: synset.definition,
        example: synset.example,
        ili: synset.ili,
        wikidata: synset.wikidata,
        source: synset.source,
        part_of_speech: synset.part_of_speech,
        also: synset.also,
        attribute: synset.attribute,
        causes: synset.causes,
        domain_region: synset.domain_region,
        domain_topic: synset.domain_topic,
        exemplifies: synset.exemplifies,
        entails: synset.entails,
        hypernym: synset.hypernym,
        instance_hypernym: synset.instance_hypernym,
        mero_member: synset.mero_member,
        mero_part: synset.mero_part,
        mero_substance: synset.mero_substance,
        similar: synset.similar,
        hyponym: synset.hyponym,
        is_caused_by: synset.is_caused_by,
        has_domain_region: synset.has_domain_region,
        has_domain_topic: synset.has_domain_topic,
        is_exemplified_by: synset.is_exemplified_by,
        is_entailed_by: synset.is_entailed_by,
        instance_hyponym: synset.instance_hyponym,
        holo_member: synset.holo_member,
        holo_part: synset.holo_part,
        holo_substance: synset.holo_substance,
        antonym: synset.antonym,
        participle: synset.participle,
        is_participle_of: synset.is_participle_of,
        pertainym: synset.pertainym,
        is_pertainym_of: synset.is_pertainym_of,
        derivation: synset.derivation,
        exemplifies_sense: synset.exemplifies_sense,
        is_exemplified_by_sense: synset.is_exemplified_by_sense,
        agent: synset.agent,
        is_agent_of: synset.is_agent_of,
        material: synset.material,
        is_material_of: synset.is_material_of,
        event: synset.event,
        is_event_of: synset.is_event_of,
        instrument: synset.instrument,
        is_instrument_of: synset.is_instrument_of,
        location: synset.location,
        is_location_of: synset.is_location_of,
        by_means_of: synset.by_means_of,
        is_by_means_of: synset.is_by_means_of,
        undergoer: synset.undergoer,
        is_undergoer_of: synset.is_undergoer_of,
        property: synset.property,
        is_property_of: synset.is_property_of,
        result: synset.result,
        is_result_of: synset.is_result_of,
        state: synset.state,
        is_state_of: synset.is_state_of,
        uses: synset.uses,
        is_used_by: synset.is_used_by,
        destination: synset.destination,
        is_destination_of: synset.is_destination_of,
        body_part: synset.body_part,
        is_body_part_of: synset.is_body_part_of,
        vehicle: synset.vehicle,
        is_vehicle_of: synset.is_vehicle_of,
    }
}

fn entry_key(lemma : &str) -> String {
    let key = lemma.to_lowercase().chars().next().expect("Empty lemma!");
    if key < 'a' || key > 'z' {
        '0'.to_string()
    } else {
        key.to_string()
    }
}


#[derive(Debug, PartialEq, Serialize, Deserialize, Clone,Eq,Hash,PartialOrd,Ord, Readable, Writable)]
pub struct PosKey(String);

impl PosKey {
    fn entry_no(&self) -> Option<u32> {
        if self.0.len() < 3 {
            None
        } else {
            self.0[2..].parse().ok()
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Entries(pub BTreeMap<String, BTreeMap<PosKey, Entry>>);

impl Entries {
    fn entry_by_lemma(&self, lemma : &str) -> Vec<&Entry> {
        self.0.get(lemma).iter().flat_map(|x| x.values()).collect()
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize,Clone,Default)]
pub struct Entry {
    pub sense : Vec<Sense>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub form : Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pronunciation : Vec<Pronunciation>,
    #[serde(default)]
    pub poskey : Option<PosKey>
}

#[derive(Debug, PartialEq, Serialize, Deserialize,Clone)]
pub struct Sense {
    pub id : SenseId,
    pub synset : SynsetId,
    #[serde(default)]
    pub adjposition : Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub subcat: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub antonym: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub also: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub participle: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pertainym: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub derivation: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub domain_topic: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub has_domain_topic: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub domain_region: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub has_domain_region: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exemplifies: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_exemplified_by: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub similar: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub other: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub agent: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub material: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub event: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub instrument: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub location: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub by_means_of: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub undergoer: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub property: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub result: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub state: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub uses: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub destination: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub body_part: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub vehicle: Vec<SenseId>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    sent : Vec<String>
}


#[derive(Debug, PartialEq, Serialize, Deserialize,Clone, Readable, Writable)]
pub struct Pronunciation {
    pub value : String,
    pub variety : Option<String>
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Synsets(pub BTreeMap<SynsetId, Synset>);


#[derive(Debug, PartialEq, Serialize, Deserialize,Clone)]
pub struct Synset {
    // not found in serialized data
    #[serde(default)]
    pub id : Option<SynsetId>,
    // not found in serialized data
    #[serde(default)]
    pub lexname: Option<String>,
    pub definition : Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub example : Vec<Example>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ili : Option<ILIID>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wikidata : Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source : Option<String>,
    #[serde(default)]
    pub members : Vec<String>,
    #[serde(rename="partOfSpeech")]
    pub part_of_speech : PartOfSpeech,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub also : Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub attribute : Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub causes : Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub domain_region : Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub domain_topic : Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exemplifies : Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub entails : Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub hypernym : Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub instance_hypernym : Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mero_member : Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mero_part : Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mero_substance : Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub similar : Vec<SynsetId>,
    /// Extra values that need to be inferred
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub hyponym: Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_caused_by: Vec<SynsetId>,
    #[serde(skip)]
    pub has_domain_region: Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub has_domain_topic: Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_exemplified_by: Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_entailed_by: Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub instance_hyponym: Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub holo_member: Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub holo_part: Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub holo_substance: Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub antonym: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub participle: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_participle_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pertainym: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_pertainym_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub derivation: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exemplifies_sense: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_exemplified_by_sense: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub agent: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_agent_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub material: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_material_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub event: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_event_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub instrument: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_instrument_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub location: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_location_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub by_means_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_by_means_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub undergoer: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_undergoer_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub property: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_property_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub result: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_result_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub state: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_state_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub uses: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_used_by: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub destination: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_destination_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub body_part: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_body_part_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub vehicle: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_vehicle_of: Vec<SenseRelation>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize,Clone, Readable, Writable)]
pub struct MemberSynset {
    // not found in serialized data
    pub id : SynsetId,
    // not found in serialized data
    pub lexname: String,
    pub definition : Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub example : Vec<Example>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ili : Option<ILIID>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wikidata : Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source : Option<String>,
    #[serde(default)]
    pub members : Vec<Member>,
    #[serde(rename="partOfSpeech")]
    pub part_of_speech : PartOfSpeech,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub also : Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub attribute : Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub causes : Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub domain_region : Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub domain_topic : Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exemplifies : Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub entails : Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub hypernym : Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub instance_hypernym : Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mero_member : Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mero_part : Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mero_substance : Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub similar : Vec<SynsetId>,
    /// Extra values that need to be inferred
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub hyponym: Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_caused_by: Vec<SynsetId>,
    #[serde(skip)]
    pub has_domain_region: Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub has_domain_topic: Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_exemplified_by: Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_entailed_by: Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub instance_hyponym: Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub holo_member: Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub holo_part: Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub holo_substance: Vec<SynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub antonym: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub participle: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_participle_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pertainym: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_pertainym_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub derivation: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exemplifies_sense: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_exemplified_by_sense: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub agent: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_agent_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub material: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_material_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub event: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_event_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub instrument: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_instrument_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub location: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_location_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub by_means_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_by_means_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub undergoer: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_undergoer_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub property: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_property_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub result: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_result_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub state: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_state_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub uses: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_used_by: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub destination: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_destination_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub body_part: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_body_part_of: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub vehicle: Vec<SenseRelation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub is_vehicle_of: Vec<SenseRelation>,
}

impl redb::Value for MemberSynset {
    type SelfType<'a> = MemberSynset;
    type AsBytes<'a> = Vec<u8>;
    fn fixed_width() -> Option<usize> {
        None
    }
    fn from_bytes<'a>(bytes: &'a [u8]) -> MemberSynset  where Self: 'a {
        MemberSynset::read_from_buffer(bytes).unwrap()
    }
    fn as_bytes<'a, 'b: 'a>(value : &MemberSynset) -> Vec<u8> {
        value.write_to_vec().unwrap()
    }
    fn type_name() -> TypeName {
        TypeName::new("MemberSynset")
    }
}


#[derive(Debug, PartialEq, Serialize, Deserialize,Clone, Readable, Writable)]
pub struct Member {
    pub lemma : String,
    pub sense : MemberSense,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub form : Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pronunciation : Vec<Pronunciation>,
    #[serde(default)]
    pub poskey : Option<PosKey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_no : Option<u32>
}

#[derive(Debug, PartialEq, Serialize, Deserialize,Clone, Readable, Writable)]
pub struct MemberSense {
    pub id : SenseId,
    pub subcat: Vec<String>,
}
 
#[derive(Debug, PartialEq, Serialize, Deserialize,Clone, Readable, Writable)]
pub struct SenseRelation {
    pub target_synset: SynsetId,
    pub source_lemma: String,
    pub target_lemma: String
}

#[derive(Debug, PartialEq,Clone, Readable, Writable)]
pub struct Example {
    pub text : String,
    pub source : Option<String>
}

impl Serialize for Example {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.source {
            Some(ref s) => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("source", s)?;
                map.serialize_entry("text", &self.text)?;
                map.end()
            },
            None => {
                serializer.serialize_str(&self.text)
            }
        }
    }
}


impl<'de> Deserialize<'de> for Example {
    fn deserialize<D>(deserializer: D) -> Result<Example, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ExampleVisitor)
    }
}

pub struct ExampleVisitor;

impl<'de> Visitor<'de> for ExampleVisitor
{
    type Value = Example;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("string or map")
    }

    fn visit_str<E>(self, value: &str) -> Result<Example, E>
    where
        E: de::Error,
    {
        Ok(Example { text: value.to_string(), source: None })
    }

    fn visit_map<M>(self, mut map: M) -> Result<Example, M::Error>
    where
        M: MapAccess<'de>,
    {
        let key1 = map.next_key::<String>()?;
        let val1 = map.next_value::<String>()?;
        let key2 = map.next_key::<String>()?;
        let val2 = map.next_value::<String>()?;
        if key1 == Some("text".to_string()) && key2 == Some("source".to_string()) {
            Ok(Example { text: val1, source: Some(val2) })
        } else if key2 == Some("text".to_string()) && key1 == Some("source".to_string()) {
            Ok(Example { text: val2, source: Some(val1) })
        } else {
            panic!("Unexpected keys in example")
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize,Clone, Readable, Writable)]
pub struct ILIID(String);

impl ILIID {
    #[allow(dead_code)]
    pub fn new(s : &str) -> ILIID { ILIID(s.to_string()) }
}

impl Display for ILIID {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, Serialize, Deserialize,Clone, Readable, Writable)]
pub enum PartOfSpeech { n, v, a, r, s }

impl PartOfSpeech {
    pub fn str(&self) -> &'static str {
        match *self {
            PartOfSpeech::n => "n",
            PartOfSpeech::v => "v",
            PartOfSpeech::a => "a",
            PartOfSpeech::r => "r",
            PartOfSpeech::s => "s"
        }
    }
    pub fn from_str(s : &str) -> Result<PartOfSpeech, String> {
        match s {
            "n" => Ok(PartOfSpeech::n),
            "v" => Ok(PartOfSpeech::v),
            "a" => Ok(PartOfSpeech::a),
            "r" => Ok(PartOfSpeech::r),
            "s" => Ok(PartOfSpeech::s),
            _ => Err(format!("Unknown part of speech: {}", s))
        }
    }
    pub fn as_long_string(&self) -> &'static str {
        match *self {
            PartOfSpeech::n => "noun",
            PartOfSpeech::v => "verb",
            PartOfSpeech::a => "adjective",
            PartOfSpeech::s => "adjective_satellite",
            PartOfSpeech::r => "adverb",
        }
    }
}

impl Display for PartOfSpeech {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.str())
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone,Eq,Hash, Readable, Writable)]
pub struct SenseId(String);

impl SenseId {
    pub fn to_string(&self) -> String {
        self.0.clone()
    }
}

impl Display for SenseId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone,Eq,Hash,PartialOrd,Ord, Readable, Writable)]
pub struct SynsetId(String);

impl SynsetId {
    pub fn new(s : &str) -> SynsetId { SynsetId(s.to_string()) }
    pub fn to_string(&self) -> String { self.0.clone() }
    pub fn as_str(&self) -> &str { &self.0 }
}

impl Display for SynsetId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Error,Debug)]
pub enum WordNetYAMLIOError {
    #[error("Could not load WordNet: {0}")]
    Io(String),
    #[error("Could not load WordNet: {0}")]
    Serde(String),
}
