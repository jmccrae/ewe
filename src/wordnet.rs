use thiserror::Error;
use std::collections::HashMap;
use std::hash::Hash;
use std::slice::Iter;
use xml::reader::XmlEvent;
use xml::name::OwnedName;
use xml::attribute::OwnedAttribute;

/// A list which can be rapidly accessed by key
pub struct KeyList<K,V> {
    vec: Vec<V>,
    map: HashMap<K,usize>
}

impl<K: Eq+Hash+std::fmt::Debug,V> KeyList<K,V> {
    pub fn new() -> KeyList<K,V> {
        KeyList { vec: Vec::new(), map: HashMap::new() }
    }

    pub fn push(&mut self, k : K, v : V) {
        self.map.insert(k, self.vec.len());
        self.vec.push(v);
    }
    pub fn get(&self, k: &K) -> Option<&V> {
        return self.map.get(k).map(|idx| &self.vec[*idx])
    }
    pub fn iter(&self) -> Iter<'_, V> {
        return self.vec.iter()
    }
    pub fn contains_key(&self, k: &K) -> bool {
        return self.map.contains_key(k)
    }
    pub fn len(&self) -> usize {
        self.vec.len()
    }
//    pub fn move_key(&mut self, old: &K, new : K) -> Result<(),WordNetError> {
//        let old_val = self.map.remove(old).ok_or(
//            WordNetError::SenseKeyNotFound(format!("{:?}", old)))?;
//        self.map.insert(new, old_val);
//        Ok(())
//    }
}

/// The lexicon contains all the synset and entries
pub struct Lexicon {
    pub id: String,
    pub label: String,
    pub language: String,
    pub email: String,
    pub license: String,
    pub version: String,
    pub url: String,
    pub entries: KeyList<String,LexicalEntry>,
    pub synsets: KeyList<String,Synset>,
    pub comments: HashMap<String,String>,
    member2entry: HashMap<String, Vec<String>>,
    members: HashMap<String, Vec<String>>,
    sense2synset: HashMap<String, String>,
    sense2entry: HashMap<String, String>
}

impl Lexicon {
    pub fn new(id : String, label : String, language : String, email : String,
        license : String, version : String, url : String) -> Lexicon {
        Lexicon {
            id, label, language, email, license, version, url,
            entries: KeyList::new(), synsets: KeyList::new(), comments: HashMap::new(),
            member2entry: HashMap::new(), members: HashMap::new(),
            sense2synset: HashMap::new(),
            sense2entry: HashMap::new() }
    }

    pub fn add_entry(&mut self, entry : LexicalEntry) -> Result<(), WordNetError> {
        if self.entries.contains_key(&entry.id) {
            return Err(WordNetError::DuplicateEntryKey(entry.id.clone()))
        }
        for sense in entry.senses.iter() {
            self.members.entry(sense.synset.clone()).or_insert_with(|| Vec::new())
                .push(entry.lemma.written_form.clone());
            self.sense2synset.insert(sense.id.clone(), sense.synset.clone());
            self.sense2entry.insert(sense.id.clone(), entry.id.clone());
        }
        self.member2entry.entry(entry.lemma.written_form.clone())
            .or_insert_with(|| Vec::new()).push(entry.id.clone());
        self.entries.push(entry.id.clone(), entry);
        Ok(())
    }

    pub fn add_synset(&mut self, synset : Synset) {
        self.synsets.push(synset.id.clone(), synset);
    }

    pub fn entry_by_id(&self, id : &String) -> Option<&LexicalEntry> {
        self.entries.get(id)
    }

    pub fn synset_by_id(&self, id : &String) -> Option<&Synset> {
        self.synsets.get(id)
    }

    pub fn sense_by_id(&self, id : &String) -> Option<&Sense> {
        self.sense2entry.get(id).and_then(|ss|
            self.entries.get(ss)).and_then(|ss| {
            ss.senses.iter().find(|s| s.id == *id)
        })
    }

    pub fn entry_by_lemma(&self, lemma : &String) -> Option<Vec<&LexicalEntry>> {
        match self.member2entry.get(lemma) {
            Some(v) => Some(v.iter().flat_map(|id| self.entries.get(id)).collect()),
            None => None
        }
    }

    pub fn members_by_id(&self, synset_id : &String) -> Vec<String> {
        return self.members.get(synset_id)
            .map(|x| x.clone())
            .unwrap_or_else(|| Vec::new())
    }

    pub fn sense_to_synset(&self, sense_id : &String) -> Option<&String> {
        return self.sense2synset.get(sense_id)
    }

    pub fn change_sense_id(&mut self, sense : &mut Sense, new_id : String) 
        -> Result<(), WordNetError>  {
        self.sense2synset.remove(&sense.id).ok_or(
            WordNetError::SenseKeyNotFound(sense.id.to_string()))?;
        let entry_id = self.sense2entry.remove(&sense.id)
            .expect("Inconsistent state");
        sense.id = new_id.clone();
        self.sense2synset.insert(sense.id.clone(), sense.synset.clone());
        self.sense2entry.insert(sense.id.clone(), entry_id);
        Ok(())
    }

    fn to_xml<W : std::io::Write>(&self, xml_file : &mut W, part : bool) -> std::io::Result<()> {
        write!(xml_file, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n")?;
        if part {
            write!(xml_file, "<!DOCTYPE LexicalResource SYSTEM \"http://globalwordnet.github.io/schemas/WN-LMF-relaxed-1.0.dtd\">\n")?;
        } else {
            write!(xml_file, "<!DOCTYPE LexicalResource SYSTEM \"http://globalwordnet.github.io/schemas/WN-LMF-1.0.dtd\">\n")?;
        }
        write!(xml_file, "<LexicalResource xmlns:dc=\"http://purl.org/dc/elements/1.1/\">
  <Lexicon id=\"{}\" 
           label=\"{}\" 
           language=\"{}\"
           email=\"{}\"
           license=\"{}\"
           version=\"{}\"
           url=\"{}\">
", self.id, self.label, self.language, self.email,
               self.license, self.version, self.url)?;

        for entry in self.entries.iter() {
            entry.to_xml(xml_file, &self.comments)?;
        }
        for synset in self.synsets.iter() {
            synset.to_xml(xml_file, &self.comments)?;
        }
        write!(xml_file, "  </Lexicon>
</LexicalResource>\n")?;
        Ok(())
    }
}

impl std::fmt::Display for Lexicon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Lexicon with ID {} and {} entries and {} synsets", self.id, 
                self.entries.len(), self.synsets.len())
    }
}

#[derive(Clone,PartialEq,Debug)]
pub struct LexicalEntry { 
    pub id : String,
    pub lemma : Lemma,
    pub forms : Vec<Form>,
    pub senses : Vec<Sense>,
    pub syntactic_behaviours: Vec<SyntacticBehaviour>
}

impl LexicalEntry {
    fn new(id : String, lemma : Lemma) -> LexicalEntry {
        LexicalEntry {
            id, lemma, forms: Vec::new(),
            senses: Vec::new(), syntactic_behaviours: Vec::new()
        }
    }

    fn set_lemma(&mut self, lemma : Lemma) {
        self.lemma = lemma;
    }

    fn add_form(&mut self, form : Form) {
        self.forms.push(form)
    }

    fn add_sense(&mut self, sense : Sense) {
        self.senses.push(sense)
    }

    fn add_syntactic_behaviour(&mut self, synbeh : SyntacticBehaviour) {
        self.syntactic_behaviours.push(synbeh)
    }

    fn to_xml<W : std::io::Write>(&self, xml_file : &mut W, comments : &HashMap<String, String>) -> std::io::Result<()> {
        write!(xml_file, "    <LexicalEntry id=\"{}\">
      <Lemma writtenForm=\"{}\" partOfSpeech=\"{}\"/>
", self.id, escape_xml_lit(&self.lemma.written_form), self.lemma.part_of_speech.value())?;
        for form in self.forms.iter() {
            form.to_xml(xml_file)?;
        }
        for sense in self.senses.iter() {
            sense.to_xml(xml_file, comments)?;
        }
        for synbeh in self.syntactic_behaviours.iter() {
            synbeh.to_xml(xml_file)?;
        }
        write!(xml_file, "    </LexicalEntry>
")
    }
}

#[derive(Clone,PartialEq,Debug)]
pub struct Lemma {
    pub written_form : String,
    pub part_of_speech: PartOfSpeech
}

impl Lemma {
    fn new(written_form : String, part_of_speech : PartOfSpeech) -> Lemma {
        Lemma { written_form, part_of_speech }
    }
}

#[derive(Clone,PartialEq,Debug)]
pub struct Form {
    pub written_form : String
}

impl Form {
    fn new(written_form : String) -> Form {
        Form { written_form }
    }

    fn to_xml<W : std::io::Write>(&self, xml_file : &mut W) -> std::io::Result<()> {
        write!(xml_file, "      <Form writtenForm=\"{}\"/>
", self.written_form)
    }
}

#[derive(Clone,PartialEq,Debug)]
pub struct Sense {
    pub id : String,
    pub synset : String,
    pub n : i32,
    pub sense_key : Option<String>,
    pub sense_relations : Vec<SenseRelation>,
    pub adjposition : Option<String>
    
}

impl Sense {
    fn new(id : String, synset : String, sense_key : Option<String>,
        n : i32, adjposition : Option<String>) -> Sense {
        Sense { id, synset, sense_key, n, sense_relations: Vec::new(), adjposition }
    }

    fn add_sense_relation(&mut self, relation : SenseRelation) {
        self.sense_relations.push(relation);
    }

    fn to_xml<W : std::io::Write>(&self, xml_file : &mut W, comments : 
        &HashMap<String, String>) -> std::io::Result<()> {
        let n_str = match self.adjposition {
            Some(ref ap) => format!(" adjposition=\"{}\"", ap),
            None => "".to_owned()
        };
        let n_str = if self.n >= 0 {
            format!("{} n=\"{}\"", n_str, self.n)
        } else {
            n_str
        };
        let sk_str = match self.sense_key {
            Some(ref sk) => format!(" dc:identifier=\"{}\"", escape_xml_lit(sk)),
            None => "".to_owned()
        };
        if self.sense_relations.len() > 0 {
            write!(xml_file, "      <Sense id=\"{}\"{} synset=\"{}\"{}>
", self.id, n_str, self.synset, sk_str)?;
            for rel in self.sense_relations.iter() {
                //rel.to_xml(xml_file, comments)?;
            }
            write!(xml_file, "        </Sense>
")
        } else {
            write!(xml_file, "      <Sense id=\"{}\"{} synset=\"{}\"{}/>
", self.id, n_str, self.synset, sk_str)
        }
    }
}

#[derive(Clone,PartialEq,Debug)]
pub struct Synset {
    pub id : String,
    pub ili : String,
    pub part_of_speech : PartOfSpeech,
    pub lex_name : Option<String>,
    pub definitions : Vec<Definition>,
    pub ili_definition : Option<Definition>,
    pub synset_relations : Vec<SynsetRelation>,
    pub examples : Vec<Example>,
    pub source : Option<String>
}

impl Synset {
    pub fn new(id : String, ili : String, part_of_speech : PartOfSpeech, 
        lex_name : Option<String>, source : Option<String>) -> Synset {
        Synset {
            id, ili, part_of_speech, lex_name,
            definitions: Vec::new(),
            ili_definition: None,
            synset_relations: Vec::new(),
            examples: Vec::new(),
            source
        }
    }

    pub fn add_definition(&mut self, definition : Definition) {
        self.definitions.push(definition);
    }

    pub fn add_ili_definition(&mut self, definition : Definition) {
        if !self.definitions.contains(&definition) {
            self.definitions.push(definition.clone());
        }
        self.ili_definition = Some(definition);
    }

    pub fn add_synset_relation(&mut self, relation : SynsetRelation) {
        self.synset_relations.push(relation);
    }

    pub fn add_example(&mut self, example : Example) {
        self.examples.push(example);
    }

    fn to_xml<W : std::io::Write>(&self, xml_file : &mut W, comments : 
        &HashMap<String, String>) -> std::io::Result<()> {
        if comments.contains_key(&self.id) {
            write!(xml_file, "    <!-- {} -->
", comments[&self.id])?;
        }
        let source_tag = match self.source {
            Some(ref s) => format!(" dc:source=\"{}\"", s),
            None => "".to_owned()
        };
        let lex_name = match self.lex_name {
            Some(ref s) => s,
            None => ""
        };
        write!(xml_file, "    <Synset id=\"{}\" ili=\"{}\" partOfSpeech=\"{}\" dc:subject=\"{}\"{}>
", self.id, self.ili, self.part_of_speech.value(), 
            lex_name, source_tag)?;
        for defn in self.definitions.iter() {
            defn.to_xml(xml_file, false)?;
        }
        match self.ili_definition {
            Some(ref d) => d.to_xml(xml_file, true)?,
            None => {}
        }
        for rel in self.synset_relations.iter() {
            rel.to_xml(xml_file, comments)?;
        }
        for ex in self.examples.iter() {
            ex.to_xml(xml_file)?;
        }
        write!(xml_file, "    </Synset>
")
    }
}

#[derive(Clone,PartialEq,Debug)]
pub struct Definition {
    pub text : String
}

impl Definition {
    pub fn new(text : String) -> Definition {
        Definition { text } 
    }

    fn to_xml<W : std::io::Write>(&self, xml_file : &mut W, is_ili : bool) -> std::io::Result<()> {
        if is_ili {
            write!(xml_file, "      <ILIDefinition>{}</ILIDefinition>
", escape_xml_lit(&self.text))
        } else {
            write!(xml_file, "      <Definition>{}</Definition>
", escape_xml_lit(&self.text))
        }
    }
}


#[derive(Clone,PartialEq,Debug)]
pub struct Example {
    pub text : String,
    pub source : Option<String>
}

impl Example {
    pub fn new(text : String, source : Option<String>) -> Example {
        Example { text, source }
    }

    fn to_xml<W : std::io::Write>(&self, xml_file : &mut W) -> std::io::Result<()> {
        match self.source {
            Some(ref src) => write!(xml_file, "      <Example dc:source=\"{}\">{}</Example>
", src, escape_xml_lit(&self.text)),
            None => write!(xml_file, "      <Example>{}</Example>
", escape_xml_lit(&self.text))
        }
    }
}


#[derive(Clone,PartialEq,Debug)]
pub struct SynsetRelation {
    target : String,
    rel_type : SynsetRelType
}

impl SynsetRelation {
    pub fn new(target : String, rel_type : SynsetRelType) -> SynsetRelation {
        SynsetRelation { target, rel_type }
    }

    fn to_xml<W : std::io::Write>(&self, xml_file : &mut W, comments : &HashMap<String, String>) -> std::io::Result<()> {
        write!(xml_file, "      <SynsetRelation relType=\"{}\" target=\"{}\"/>",
                self.rel_type.value(), self.target)?;
        if comments.contains_key(&self.target) {
            write!(xml_file, " <!-- {} -->
", comments[&self.target])
        } else {
            write!(xml_file, "\n")
        }
    }
}


#[derive(Clone,PartialEq,Debug)]
pub struct SenseRelation {
    target : String,
    rel_type : SenseRelType
}

impl SenseRelation {
    pub fn new(target : String, rel_type : SenseRelType) -> SenseRelation {
        SenseRelation { target, rel_type }
    }

    fn to_xml<W : std::io::Write>(&self, xml_file : &mut W, comments : &HashMap<String, String>) -> std::io::Result<()> {
        write!(xml_file, "        <SenseRelation relType=\"{}\" target=\"{}\"/>",
                self.rel_type.value(), self.target)?;
        match comments.get(&self.target) {
            Some(c) => write!(xml_file, " <!-- {} -->
", c),
            None => write!(xml_file, "\n")
        }
    }
}


#[derive(Clone,PartialEq,Debug)]
pub struct SyntacticBehaviour {
    pub subcategorization_frame : String,
    pub senses : Vec<String>
}

impl SyntacticBehaviour {
    pub fn new(subcategorization_frame : String, senses : Vec<String>) -> SyntacticBehaviour {
        SyntacticBehaviour { subcategorization_frame, senses }
    }
    
    fn to_xml<W : std::io::Write>(&self, xml_file : &mut W) -> std::io::Result<()> {
        write!(xml_file, "      <SyntacticBehaviour subcategorizationFrame=\"{}\" senses=\"{}\"/>
", escape_xml_lit(&self.subcategorization_frame), self.senses.join(" "))
    }
}

#[derive(Clone,PartialEq,Debug)]
pub enum PartOfSpeech {
    Noun,
    Verb,
    Adjective,
    Adverb,
    AdjectiveSatellite,
    NamedEntity,
    Conjunction,
    Adposition,
    Other,
    Unknown
}

impl PartOfSpeech {
    fn value(&self) -> &'static str {
        match self {
            PartOfSpeech::Noun => "n",
            PartOfSpeech::Verb => "v",
            PartOfSpeech::Adjective => "a",
            PartOfSpeech::Adverb => "r",
            PartOfSpeech::AdjectiveSatellite => "s",
            PartOfSpeech::NamedEntity => "t",
            PartOfSpeech::Conjunction => "c",
            PartOfSpeech::Adposition => "p",
            PartOfSpeech::Other => "x",
            PartOfSpeech::Unknown => "u"
        }
    }

    fn from(v : &str) -> Result<PartOfSpeech,WordNetXMLParseError> {
        match v {
            "n" => Ok(PartOfSpeech::Noun),
            "v" => Ok(PartOfSpeech::Verb),
            "a" => Ok(PartOfSpeech::Adjective),
            "r" => Ok(PartOfSpeech::Adverb),
            "s" => Ok(PartOfSpeech::AdjectiveSatellite),
            "t" => Ok(PartOfSpeech::NamedEntity),
            "c" => Ok(PartOfSpeech::Conjunction),
            "p" => Ok(PartOfSpeech::Adposition),
            "x" => Ok(PartOfSpeech::Other),
            "u" => Ok(PartOfSpeech::Unknown),
             _ => Err(WordNetXMLParseError::BadPartOfSpeech(v.to_string()))
        }
    }
}

pub fn equal_pos(pos1 : PartOfSpeech, pos2 : PartOfSpeech) -> bool {
    return pos1 == pos2 
            || pos1 == PartOfSpeech::Adjective && pos2 == PartOfSpeech::AdjectiveSatellite
            || pos2 == PartOfSpeech::Adjective && pos1 == PartOfSpeech::AdjectiveSatellite;
}

#[derive(Clone,PartialEq,Debug,Eq,Hash)]
pub enum SynsetRelType { 
    Agent,
    Also,
    Attribute,
    BeInState,
    Causes,
    ClassifiedBy,
    Classifies,
    CoAgentInstrument,
    CoAgentPatient,
    CoAgentResult,
    CoInstrumentAgent,
    CoInstrumentPatient,
    CoInstrumentResult,
    CoPatientAgent,
    CoPatientInstrument,
    CoResultAgent,
    CoResultInstrument,
    CoRole,
    Direction,
    DomainRegion,
    DomainTopic,
    Exemplifies,
    Entails,
    EqSynonym,
    HasDomainRegion,
    HasDomainTopic,
    IsExemplifiedBy,
    HoloLocation,
    HoloMember,
    HoloPart,
    HoloPortion,
    HoloSubstance,
    Holonym,
    Hypernym,
    Hyponym,
    InManner,
    InstanceHypernym,
    InstanceHyponym,
    Instrument,
    Involved,
    InvolvedAgent,
    InvolvedDirection,
    InvolvedInstrument,
    InvolvedLocation,
    InvolvedPatient,
    InvolvedResult,
    InvolvedSourceDirection,
    InvolvedTargetDirection,
    IsCausedBy,
    IsEntailedBy,
    Location,
    MannerOf,
    MeroLocation,
    MeroMember,
    MeroPart,
    MeroPortion,
    MeroSubstance,
    Meronym,
    Similar,
    Other,
    Patient,
    RestrictedBy,
    Restricts,
    Result,
    Role,
    SourceDirection,
    StateOf,
    TargetDirection,
    Subevent,
    IsSubeventOf,
    Antonym
}

impl SynsetRelType {
    pub fn value(&self) -> &'static str {
        match self {
            SynsetRelType::Agent => "agent",
            SynsetRelType::Also => "also",
            SynsetRelType::Attribute => "attribute",
            SynsetRelType::BeInState => "be_in_state",
            SynsetRelType::Causes => "causes",
            SynsetRelType::ClassifiedBy => "classified_by",
            SynsetRelType::Classifies => "classifies",
            SynsetRelType::CoAgentInstrument => "co_agent_instrument",
            SynsetRelType::CoAgentPatient => "co_agent_patient",
            SynsetRelType::CoAgentResult => "co_agent_result",
            SynsetRelType::CoInstrumentAgent => "co_instrument_agent",
            SynsetRelType::CoInstrumentPatient => "co_instrument_patient",
            SynsetRelType::CoInstrumentResult => "co_instrument_result",
            SynsetRelType::CoPatientAgent => "co_patient_agent",
            SynsetRelType::CoPatientInstrument => "co_patient_instrument",
            SynsetRelType::CoResultAgent => "co_result_agent",
            SynsetRelType::CoResultInstrument => "co_result_instrument",
            SynsetRelType::CoRole => "co_role",
            SynsetRelType::Direction => "direction",
            SynsetRelType::DomainRegion => "domain_region",
            SynsetRelType::DomainTopic => "domain_topic",
            SynsetRelType::Exemplifies => "exemplifies",
            SynsetRelType::Entails => "entails",
            SynsetRelType::EqSynonym => "eq_synonym",
            SynsetRelType::HasDomainRegion => "has_domain_region",
            SynsetRelType::HasDomainTopic => "has_domain_topic",
            SynsetRelType::IsExemplifiedBy => "is_exemplified_by",
            SynsetRelType::HoloLocation => "holo_location",
            SynsetRelType::HoloMember => "holo_member",
            SynsetRelType::HoloPart => "holo_part",
            SynsetRelType::HoloPortion => "holo_portion",
            SynsetRelType::HoloSubstance => "holo_substance",
            SynsetRelType::Holonym => "holonym",
            SynsetRelType::Hypernym => "hypernym",
            SynsetRelType::Hyponym => "hyponym",
            SynsetRelType::InManner => "in_manner",
            SynsetRelType::InstanceHypernym => "instance_hypernym",
            SynsetRelType::InstanceHyponym => "instance_hyponym",
            SynsetRelType::Instrument => "instrument",
            SynsetRelType::Involved => "involved",
            SynsetRelType::InvolvedAgent => "involved_agent",
            SynsetRelType::InvolvedDirection => "involved_direction",
            SynsetRelType::InvolvedInstrument => "involved_instrument",
            SynsetRelType::InvolvedLocation => "involved_location",
            SynsetRelType::InvolvedPatient => "involved_patient",
            SynsetRelType::InvolvedResult => "involved_result",
            SynsetRelType::InvolvedSourceDirection => "involved_source_direction",
            SynsetRelType::InvolvedTargetDirection => "involved_target_direction",
            SynsetRelType::IsCausedBy => "is_caused_by",
            SynsetRelType::IsEntailedBy => "is_entailed_by",
            SynsetRelType::Location => "location",
            SynsetRelType::MannerOf => "manner_of",
            SynsetRelType::MeroLocation => "mero_location",
            SynsetRelType::MeroMember => "mero_member",
            SynsetRelType::MeroPart => "mero_part",
            SynsetRelType::MeroPortion => "mero_portion",
            SynsetRelType::MeroSubstance => "mero_substance",
            SynsetRelType::Meronym => "meronym",
            SynsetRelType::Similar => "similar",
            SynsetRelType::Other => "other",
            SynsetRelType::Patient => "patient",
            SynsetRelType::RestrictedBy => "restricted_by",
            SynsetRelType::Restricts => "restricts",
            SynsetRelType::Result => "result",
            SynsetRelType::Role => "role",
            SynsetRelType::SourceDirection => "source_direction",
            SynsetRelType::StateOf => "state_of",
            SynsetRelType::TargetDirection => "target_direction",
            SynsetRelType::Subevent => "subevent",
            SynsetRelType::IsSubeventOf => "is_subevent_of",
            SynsetRelType::Antonym => "antonym"
        }
    }

    pub fn from(v : &str) -> Result<SynsetRelType, WordNetXMLParseError> {
        match v {
            "agent" => Ok(SynsetRelType::Agent),
            "also" => Ok(SynsetRelType::Also),
            "attribute" => Ok(SynsetRelType::Attribute),
            "be_in_state" => Ok(SynsetRelType::BeInState),
            "causes" => Ok(SynsetRelType::Causes),
            "classified_by" => Ok(SynsetRelType::ClassifiedBy),
            "classifies" => Ok(SynsetRelType::Classifies),
            "co_agent_instrument" => Ok(SynsetRelType::CoAgentInstrument),
            "co_agent_patient" => Ok(SynsetRelType::CoAgentPatient),
            "co_agent_result" => Ok(SynsetRelType::CoAgentResult),
            "co_instrument_agent" => Ok(SynsetRelType::CoInstrumentAgent),
            "co_instrument_patient" => Ok(SynsetRelType::CoInstrumentPatient),
            "co_instrument_result" => Ok(SynsetRelType::CoInstrumentResult),
            "co_patient_agent" => Ok(SynsetRelType::CoPatientAgent),
            "co_patient_instrument" => Ok(SynsetRelType::CoPatientInstrument),
            "co_result_agent" => Ok(SynsetRelType::CoResultAgent),
            "co_result_instrument" => Ok(SynsetRelType::CoResultInstrument),
            "co_role" => Ok(SynsetRelType::CoRole),
            "direction" => Ok(SynsetRelType::Direction),
            "domain_region" => Ok(SynsetRelType::DomainRegion),
            "domain_topic" => Ok(SynsetRelType::DomainTopic),
            "exemplifies" => Ok(SynsetRelType::Exemplifies),
            "entails" => Ok(SynsetRelType::Entails),
            "eq_synonym" => Ok(SynsetRelType::EqSynonym),
            "has_domain_region" => Ok(SynsetRelType::HasDomainRegion),
            "has_domain_topic" => Ok(SynsetRelType::HasDomainTopic),
            "is_exemplified_by" => Ok(SynsetRelType::IsExemplifiedBy),
            "holo_location" => Ok(SynsetRelType::HoloLocation),
            "holo_member" => Ok(SynsetRelType::HoloMember),
            "holo_part" => Ok(SynsetRelType::HoloPart),
            "holo_portion" => Ok(SynsetRelType::HoloPortion),
            "holo_substance" => Ok(SynsetRelType::HoloSubstance),
            "holonym" => Ok(SynsetRelType::Holonym),
            "hypernym" => Ok(SynsetRelType::Hypernym),
            "hyponym" => Ok(SynsetRelType::Hyponym),
            "in_manner" => Ok(SynsetRelType::InManner),
            "instance_hypernym" => Ok(SynsetRelType::InstanceHypernym),
            "instance_hyponym" => Ok(SynsetRelType::InstanceHyponym),
            "instrument" => Ok(SynsetRelType::Instrument),
            "involved" => Ok(SynsetRelType::Involved),
            "involved_agent" => Ok(SynsetRelType::InvolvedAgent),
            "involved_direction" => Ok(SynsetRelType::InvolvedDirection),
            "involved_instrument" => Ok(SynsetRelType::InvolvedInstrument),
            "involved_location" => Ok(SynsetRelType::InvolvedLocation),
            "involved_patient" => Ok(SynsetRelType::InvolvedPatient),
            "involved_result" => Ok(SynsetRelType::InvolvedResult),
            "involved_source_direction" => Ok(SynsetRelType::InvolvedSourceDirection),
            "involved_target_direction" => Ok(SynsetRelType::InvolvedTargetDirection),
            "is_caused_by" => Ok(SynsetRelType::IsCausedBy),
            "is_entailed_by" => Ok(SynsetRelType::IsEntailedBy),
            "location" => Ok(SynsetRelType::Location),
            "manner_of" => Ok(SynsetRelType::MannerOf),
            "mero_location" => Ok(SynsetRelType::MeroLocation),
            "mero_member" => Ok(SynsetRelType::MeroMember),
            "mero_part" => Ok(SynsetRelType::MeroPart),
            "mero_portion" => Ok(SynsetRelType::MeroPortion),
            "mero_substance" => Ok(SynsetRelType::MeroSubstance),
            "meronym" => Ok(SynsetRelType::Meronym),
            "similar" => Ok(SynsetRelType::Similar),
            "other" => Ok(SynsetRelType::Other),
            "patient" => Ok(SynsetRelType::Patient),
            "restricted_by" => Ok(SynsetRelType::RestrictedBy),
            "restricts" => Ok(SynsetRelType::Restricts),
            "result" => Ok(SynsetRelType::Result),
            "role" => Ok(SynsetRelType::Role),
            "source_direction" => Ok(SynsetRelType::SourceDirection),
            "state_of" => Ok(SynsetRelType::StateOf),
            "target_direction" => Ok(SynsetRelType::TargetDirection),
            "subevent" => Ok(SynsetRelType::Subevent),
            "is_subevent_of" => Ok(SynsetRelType::IsSubeventOf),
            "antonym" => Ok(SynsetRelType::Antonym),
            _ => Err(WordNetXMLParseError::BadRelType(v.to_string()))
        }
    }
}

lazy_static! {
    static ref INVERSE_SYNSET_RELS : HashMap<SynsetRelType, SynsetRelType> = {
        let mut map = HashMap::new();
        map.insert(SynsetRelType::Hypernym, SynsetRelType::Hyponym);
        map.insert(SynsetRelType::Hyponym, SynsetRelType::Hypernym);
        map.insert(SynsetRelType::InstanceHypernym, SynsetRelType::InstanceHyponym);
        map.insert(SynsetRelType::InstanceHyponym, SynsetRelType::InstanceHypernym);
        map.insert(SynsetRelType::Meronym, SynsetRelType::Holonym);
        map.insert(SynsetRelType::Holonym, SynsetRelType::Meronym);
        map.insert(SynsetRelType::MeroLocation, SynsetRelType::HoloLocation);
        map.insert(SynsetRelType::HoloLocation, SynsetRelType::MeroLocation);
        map.insert(SynsetRelType::MeroMember, SynsetRelType::HoloMember);
        map.insert(SynsetRelType::HoloMember, SynsetRelType::MeroMember);
        map.insert(SynsetRelType::MeroPart, SynsetRelType::HoloPart);
        map.insert(SynsetRelType::HoloPart, SynsetRelType::MeroPart);
        map.insert(SynsetRelType::MeroPortion, SynsetRelType::HoloPortion);
        map.insert(SynsetRelType::HoloPortion, SynsetRelType::MeroPortion);
        map.insert(SynsetRelType::MeroSubstance, SynsetRelType::HoloSubstance);
        map.insert(SynsetRelType::HoloSubstance, SynsetRelType::MeroSubstance);
        map.insert(SynsetRelType::BeInState, SynsetRelType::StateOf);
        map.insert(SynsetRelType::StateOf, SynsetRelType::BeInState);
        map.insert(SynsetRelType::Causes, SynsetRelType::IsCausedBy);
        map.insert(SynsetRelType::IsCausedBy, SynsetRelType::Causes);
        map.insert(SynsetRelType::Subevent, SynsetRelType::IsSubeventOf);
        map.insert(SynsetRelType::IsSubeventOf, SynsetRelType::Subevent);
        map.insert(SynsetRelType::MannerOf, SynsetRelType::InManner);
        map.insert(SynsetRelType::InManner, SynsetRelType::MannerOf);
        map.insert(SynsetRelType::Restricts, SynsetRelType::RestrictedBy);
        map.insert(SynsetRelType::RestrictedBy, SynsetRelType::Restricts);
        map.insert(SynsetRelType::Classifies, SynsetRelType::ClassifiedBy);
        map.insert(SynsetRelType::ClassifiedBy, SynsetRelType::Classifies);
        map.insert(SynsetRelType::Entails, SynsetRelType::IsEntailedBy);
        map.insert(SynsetRelType::IsEntailedBy, SynsetRelType::Entails);
        map.insert(SynsetRelType::DomainRegion, SynsetRelType::HasDomainRegion);
        map.insert(SynsetRelType::HasDomainRegion, SynsetRelType::DomainRegion);
        map.insert(SynsetRelType::DomainTopic, SynsetRelType::HasDomainTopic);
        map.insert(SynsetRelType::HasDomainTopic, SynsetRelType::DomainTopic);
        map.insert(SynsetRelType::Exemplifies, SynsetRelType::IsExemplifiedBy);
        map.insert(SynsetRelType::IsExemplifiedBy, SynsetRelType::Exemplifies);
        map.insert(SynsetRelType::Role, SynsetRelType::Involved);
        map.insert(SynsetRelType::Involved, SynsetRelType::Role);
        map.insert(SynsetRelType::Agent, SynsetRelType::InvolvedAgent);
        map.insert(SynsetRelType::InvolvedAgent, SynsetRelType::Agent);
        map.insert(SynsetRelType::Patient, SynsetRelType::InvolvedPatient);
        map.insert(SynsetRelType::InvolvedPatient, SynsetRelType::Patient);
        map.insert(SynsetRelType::Result, SynsetRelType::InvolvedResult);
        map.insert(SynsetRelType::InvolvedResult, SynsetRelType::Result);
        map.insert(SynsetRelType::Instrument, SynsetRelType::InvolvedInstrument);
        map.insert(SynsetRelType::InvolvedInstrument, SynsetRelType::Instrument);
        map.insert(SynsetRelType::Location, SynsetRelType::InvolvedLocation);
        map.insert(SynsetRelType::InvolvedLocation, SynsetRelType::Location);
        map.insert(SynsetRelType::Direction, SynsetRelType::InvolvedDirection);
        map.insert(SynsetRelType::InvolvedDirection, SynsetRelType::Direction);
        map.insert(SynsetRelType::TargetDirection, SynsetRelType::InvolvedTargetDirection);
        map.insert(SynsetRelType::InvolvedTargetDirection, SynsetRelType::TargetDirection);
        map.insert(SynsetRelType::SourceDirection, SynsetRelType::InvolvedSourceDirection);
        map.insert(SynsetRelType::InvolvedSourceDirection, SynsetRelType::SourceDirection);
        map.insert(SynsetRelType::CoAgentPatient, SynsetRelType::CoPatientAgent);
        map.insert(SynsetRelType::CoPatientAgent, SynsetRelType::CoAgentPatient);
        map.insert(SynsetRelType::CoAgentInstrument, SynsetRelType::CoInstrumentAgent);
        map.insert(SynsetRelType::CoInstrumentAgent, SynsetRelType::CoAgentInstrument);
        map.insert(SynsetRelType::CoAgentResult, SynsetRelType::CoResultAgent);
        map.insert(SynsetRelType::CoResultAgent, SynsetRelType::CoAgentResult);
        map.insert(SynsetRelType::CoPatientInstrument, SynsetRelType::CoInstrumentPatient);
        map.insert(SynsetRelType::CoInstrumentPatient, SynsetRelType::CoPatientInstrument);
        map.insert(SynsetRelType::CoResultInstrument, SynsetRelType::CoInstrumentResult);
        map.insert(SynsetRelType::CoInstrumentResult, SynsetRelType::CoResultInstrument);
        map.insert(SynsetRelType::Antonym, SynsetRelType::Antonym);
        map.insert(SynsetRelType::EqSynonym, SynsetRelType::EqSynonym);
        map.insert(SynsetRelType::Similar, SynsetRelType::Similar);
        map.insert(SynsetRelType::Also, SynsetRelType::Also);
        map.insert(SynsetRelType::Attribute, SynsetRelType::Attribute);
        map.insert(SynsetRelType::CoRole, SynsetRelType::CoRole);
        map
    };
}

#[derive(Clone,PartialEq,Debug,Eq,Hash)]
pub enum SenseRelType {
    Antonym,
    Also,
    Participle,
    Pertainym,
    Derivation,
    DomainTopic,
    HasDomainTopic,
    DomainRegion,
    HasDomainRegion,
    Exemplifies,
    IsExemplifiedBy,
    Similar,
    Other
}

impl SenseRelType {
    pub fn value(&self) -> &'static str {
        match self { 
            SenseRelType::Antonym => "antonym",
            SenseRelType::Also => "also",
            SenseRelType::Participle => "participle",
            SenseRelType::Pertainym => "pertainym",
            SenseRelType::Derivation => "derivation",
            SenseRelType::DomainTopic => "domain_topic",
            SenseRelType::HasDomainTopic => "has_domain_topic",
            SenseRelType::DomainRegion => "domain_region",
            SenseRelType::HasDomainRegion => "has_domain_region",
            SenseRelType::Exemplifies => "exemplifies",
            SenseRelType::IsExemplifiedBy => "is_exemplified_by",
            SenseRelType::Similar => "similar",
            SenseRelType::Other => "other"
        }
    }

    pub fn from(v : &str) -> Result<SenseRelType,WordNetXMLParseError> {
        match v {
            "antonym" => Ok(SenseRelType::Antonym),
            "also" => Ok(SenseRelType::Also),
            "participle" => Ok(SenseRelType::Participle),
            "pertainym" => Ok(SenseRelType::Pertainym),
            "derivation" => Ok(SenseRelType::Derivation),
            "domain_topic" => Ok(SenseRelType::DomainTopic),
            "has_domain_topic" => Ok(SenseRelType::HasDomainTopic),
            "domain_region" => Ok(SenseRelType::DomainRegion),
            "has_domain_region" => Ok(SenseRelType::HasDomainRegion),
            "exemplifies" => Ok(SenseRelType::Exemplifies),
            "is_exemplified_by" => Ok(SenseRelType::IsExemplifiedBy),
            "similar" => Ok(SenseRelType::Similar),
            "other" => Ok(SenseRelType::Other),
            _ => Err(WordNetXMLParseError::BadRelType(v.to_string())) 
        }
    }
}

lazy_static! {
    static ref INVERSE_SENSE_RELS : HashMap<SenseRelType, SenseRelType> = {
        let mut map = HashMap::new();
        map.insert(SenseRelType::DomainRegion, SenseRelType::HasDomainRegion);
        map.insert(SenseRelType::HasDomainRegion, SenseRelType::DomainRegion);
        map.insert(SenseRelType::DomainTopic, SenseRelType::HasDomainTopic);
        map.insert(SenseRelType::HasDomainTopic, SenseRelType::DomainTopic);
        map.insert(SenseRelType::Exemplifies, SenseRelType::IsExemplifiedBy);
        map.insert(SenseRelType::IsExemplifiedBy, SenseRelType::Exemplifies);
        map.insert(SenseRelType::Antonym, SenseRelType::Antonym);
        map.insert(SenseRelType::Similar, SenseRelType::Similar);
        map.insert(SenseRelType::Also, SenseRelType::Also);
        map.insert(SenseRelType::Derivation, SenseRelType::Derivation);
        map
    };
}


struct WordNetContentHandler {
    lexicon : Option<Lexicon>,
    entry_id : Result<String,WordNetXMLParseError>,
    entry : Option<LexicalEntry>,
    sense : Option<Sense>,
    defn : Option<Definition>,
    ili_defn : Option<Definition>,
    example : Option<Example>,
    example_source : Option<String>,
    synset : Option<Synset>
}

fn attr<'a>(attrs : &'a Vec<OwnedAttribute>, name : &str) -> Result<String,WordNetXMLParseError> {
    attrs.iter().find(|x| x.name.local_name == name).ok_or_else(|| WordNetXMLParseError::AttributeNotFound(name.to_string())).map(|x| x.value.clone())
}

impl WordNetContentHandler {

    pub fn wordnet_content_handler(&mut self, event : XmlEvent) -> Result<(),WordNetXMLParseError> {
        match event {
            XmlEvent::StartElement { name, attributes: attrs, namespace:_ } => {
                self.start_element(name, attrs)
            },
            _ => {
                Ok(())
            }
        }
    }

    fn start_element(&mut self, name : OwnedName, attrs : Vec<OwnedAttribute>) -> Result<(), WordNetXMLParseError> {
        if name.local_name == "Lexicon" {
            self.lexicon = Some(Lexicon::new(attr(&attrs, "id")?,
                attr(&attrs, "label")?, attr(&attrs, "language")?,
                attr(&attrs, "email")?, attr(&attrs, "license")?,
                attr(&attrs, "version")?, attr(&attrs, "url")?));
        } else if name.local_name == "LexicalEntry" {
            self.entry_id = attr(&attrs, "id");
        } else if name.local_name == "Lemma" {
            let lemma = Lemma::new(attr(&attrs, "writtenForm")?,
                    PartOfSpeech::from(&attr(&attrs, "partOfSpeech")?)?);
            self.entry = Some(LexicalEntry::new(
                self.entry_id.clone()?,
                lemma));
        } else if name.local_name == "Form" {
            self.entry.as_mut().
                ok_or(WordNetXMLParseError::InvalidChild("Form","LexicalEntry"))?.
                    add_form(Form::new(attr(&attrs, "writtenForm")?));
        } else if name.local_name == "Sense" {
            let n = match attr(&attrs, "n") {
                Ok(s) => s.parse::<i32>().map_err(|_| WordNetXMLParseError::NonNumericN(s.to_string()))?,
                Err(_) => -1
            };
            self.sense = Some(Sense::new(
                    attr(&attrs, "id")?,
                    attr(&attrs, "synset")?,
                    attr(&attrs, "identifier").ok(),
                    n, attr(&attrs, "adjposition").ok()));
        }
        Ok(())
    }
}
//        elif name == "Synset":
//            self.synset = Synset(attrs["id"], attrs["ili"], 
//                PartOfSpeech(attrs["partOfSpeech"]),
//                attrs.get("dc:subject",""),
//                attrs.get("dc:source",""))
//        elif name == "Definition":
//            self.defn = ""
//        elif name == "ILIDefinition":
//            self.ili_defn = ""
//        elif name == "Example":
//            self.example = ""
//            self.example_source = attrs.get("dc:source")
//        elif name == "SynsetRelation":
//            self.synset.add_synset_relation(
//                    SynsetRelation(attrs["target"],
//                    SynsetRelType(attrs["relType"])))
//        elif name == "SenseRelation":
//            self.sense.add_sense_relation(
//                    SenseRelation(attrs["target"],
//                    SenseRelType(attrs["relType"])))
//        elif name == "SyntacticBehaviour":
//            self.entry.add_syntactic_behaviour(
//                    SyntacticBehaviour(
//                        attrs["subcategorizationFrame"],
//                        attrs["senses"].split(" ")))
//        elif name == "LexicalResource":
//            pass
//        else:
//            raise ValueError("Unexpected Tag: " + name)
//
//    def endElement(self, name):
//        if name == "LexicalEntry":
//            self.lexicon.add_entry(self.entry)
//            self.entry = None
//        elif name == "Sense":
//            self.entry.add_sense(self.sense)
//            self.sense = None
//        elif name == "Synset":
//            self.lexicon.add_synset(self.synset)
//            self.synset = None
//        elif name == "Definition":
//            self.synset.add_definition(Definition(self.defn))
//            self.defn = None
//        elif name == "ILIDefinition":
//            self.synset.add_definition(Definition(self.ili_defn), True)
//            self.ili_defn = None
//        elif name == "Example":
//            self.synset.add_example(Example(self.example, self.example_source))
//            self.example = None
//
//
//    def characters(self, content):
//        if self.defn != None:
//            self.defn += content
//        elif self.ili_defn != None:
//            self.ili_defn += content
//        elif self.example != None:
//            self.example += content
//        elif content.strip() == '':
//            pass
//        else:
//            print(content)
//            raise ValueError("Text content not expected")
//
fn escape_xml_lit(lit : &str) -> String {
    lit.replace("&", "&amp;").replace("'", "&apos;").
        replace("\"", "&quot;").replace("<", "&lt;").replace(">", "&gt;")
}
//
//def extract_comments(wordnet_file,lexicon):
//    with codecs.open(wordnet_file,"r",encoding="utf-8") as source:
//        sen_rel_comment = re.compile(".*<SenseRelation .* target=\"(.*)\".*/> <!-- (.*) -->")
//        syn_rel_comment = re.compile(".*<SynsetRelation .* target=\"(.*)\".*/> <!-- (.*) -->")
//        comment = re.compile(".*<!-- (.*) -->.*")
//        synset = re.compile(".*<Synset id=\"(\\S*)\".*")
//        c = None
//        for line in source.readlines():
//            m = sen_rel_comment.match(line)
//            if m:
//                lexicon.comments[m.group(1)] = m.group(2)
//            else:
//                m = syn_rel_comment.match(line)
//                if m:
//                    lexicon.comments[m.group(1)] = m.group(2)
//                else:
//                    m = comment.match(line)
//                    if m:
//                        c = m.group(1)
//                    else:
//                        m = synset.match(line)
//                        if m and c:
//                            lexicon.comments[m.group(1)] = c
//                            c = None
//
//
//def escape_lemma(lemma):
//    """Format the lemma so it is valid XML id"""
//    def elc(c):
//        if (c >= 'A' and c <= 'Z') or (c >= 'a' and c <= 'z') or (c >= '0' and c <= '9') or c == '.':
//            return c
//        elif c == ' ':
//            return '_'
//        elif c == '(':
//            return '-lb-'
//        elif c == ')':
//            return '-rb-'
//        elif c == '\'':
//            return '-ap-'
//        elif c == '/':
//            return '-sl-'
//        elif c == '-':
//            return '-'
//        elif c == ',':
//            return '-cm-'
//        elif c == '!':
//            return '-ex-'
//        else:
//            return '-%04x-' % ord(c)
//
//    return "".join(elc(c) for c in lemma)
//
//def parse_wordnet(wordnet_file):
//    with codecs.open(wordnet_file,"r",encoding="utf-8") as source:
//        handler = WordNetContentHandler()
//        parse(source, handler)
//    extract_comments(wordnet_file, handler.lexicon)
//    return handler.lexicon
//
//if __name__ == "__main__":
//    wordnet = parse_wordnet(sys.argv[1])
//    xml_file = open("wn31-test.xml","w")
//    wordnet.to_xml(xml_file, True)

#[derive(Error,Debug)]
pub enum WordNetError {
    #[error("duplicate entry key ${0}")]
    DuplicateEntryKey(String),
    #[error("sense key not found ${0}")]
    SenseKeyNotFound(String)
}

#[derive(Error,Debug,Clone)]
pub enum WordNetXMLParseError {
    #[error("expected an attribute ${0}")]
    AttributeNotFound(String),
    #[error("invalid value for partOfSpeech: ${0}")]
    BadPartOfSpeech(String),
    #[error("invalid value for relType: ${0}")]
    BadRelType(String),
    #[error("encountered ${0} but not as a child of ${1}")]
    InvalidChild(&'static str,&'static str),
    #[error("the value of n must be numeric but was ${0}")]
    NonNumericN(String)
}
