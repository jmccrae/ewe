use thiserror::Error;
use std::collections::HashMap;
use std::hash::Hash;
use std::slice::Iter;

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
            //synset.to_xml(xml_file, self.comments)
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
            //form.to_xml(xml_file, part)?;
        }
        for sense in self.senses.iter() {
            //sense.to_xml(xml_file, comments)?;
        }
        for synbeh in self.syntactic_behaviours.iter() {
            //synbeh.to_xml(xml_file)?;
        }
        write!(xml_file, "    </LexicalEntry>
")
    }
}
//
//
pub struct Lemma {
    pub written_form : String,
    pub part_of_speech: PartOfSpeech
}

//class Lemma:
//    """The lemma gives the written form and part of speech of an entry"""
//    def __init__(self, written_form, part_of_speech):
//        self.written_form = written_form
//        self.part_of_speech = part_of_speech
//
pub struct Form {
    pub written_form : String
}
//class Form:
//    """The form gives an inflected form of the entry"""
//    def __init__(self, written_form):
//        self.written_form = written_form
//
//    def to_xml(self, xml_file):
//        xml_file.write("""      <Form writtenForm="%s"/>
//""" % escape_xml_lit(self.written_form))
//
//
pub struct Sense {
    pub id : String,
    pub synset : String,
    pub n : usize,
    pub sense_key : Option<String>
    
}
//class Sense:
//    """The sense links an entry to a synset"""
//    def __init__(self, id, synset, sense_key, n=-1, adjposition=None):
//        self.id = id
//        self.synset = synset
//        self.n = n
//        self.sense_key = sense_key
//        self.sense_relations = []
//        self.adjposition = adjposition
//
//    def add_sense_relation(self, relation):
//        self.sense_relations.append(relation)
//
//    def to_xml(self, xml_file, comments):
//        if self.adjposition:
//            n_str = " adjposition=\"%s\"" % self.adjposition
//        else:
//            n_str = ""
//        if self.n >= 0:
//            n_str = "%s n=\"%d\"" % (n_str, self.n)
//        if self.sense_key:
//            sk_str = " dc:identifier=\"%s\"" % escape_xml_lit(self.sense_key)
//        else:
//            sk_str = ""
//        if len(self.sense_relations) > 0:
//            xml_file.write("""      <Sense id="%s"%s synset="%s"%s>
//""" % (self.id, n_str, self.synset, sk_str))
//            for rel in self.sense_relations:
//                rel.to_xml(xml_file, comments)
//            xml_file.write("""        </Sense>
//""")
//        else:
//            xml_file.write("""      <Sense id="%s"%s synset="%s"%s/>
//""" % (self.id, n_str, self.synset, sk_str))
//
//
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
//class Synset:
//    """The synset is a collection of synonyms"""
//    def __init__(self, id, ili, part_of_speech, lex_name, source=None):
//        self.id = id
//        self.ili = ili
//        self.part_of_speech = part_of_speech
//        self.lex_name = lex_name
//        self.definitions = []
//        self.ili_definition = None
//        self.synset_relations = []
//        self.examples = []
//        self.source = source
//
//    def add_definition(self, definition, is_ili=False):
//        if is_ili:
//            if not definition in self.definitions:
//                self.definitions.append(definition)
//            self.ili_definition = definition
//        else:
//            self.definitions.append(definition)
//
//    def add_synset_relation(self, relation):
//        self.synset_relations.append(relation)
//
//    def add_example(self, example):
//        self.examples.append(example)
//
//    def to_xml(self, xml_file, comments):
//        if self.id in comments:
//            xml_file.write("""    <!-- %s -->
//""" % comments[self.id])
//        source_tag = ""
//        if self.source:
//            source_tag = " dc:source=\"%s\"" % (self.source)
//        xml_file.write("""    <Synset id="%s" ili="%s" partOfSpeech="%s" dc:subject="%s"%s>
//""" % (self.id, self.ili, self.part_of_speech.value, self.lex_name, source_tag))
//        for defn in self.definitions:
//            defn.to_xml(xml_file)
//        if self.ili_definition:
//            self.ili_definition.to_xml(xml_file, True)
//        for rel in self.synset_relations:
//            rel.to_xml(xml_file, comments)
//        for ex in self.examples:
//            ex.to_xml(xml_file)
//        xml_file.write("""    </Synset>
//""")
//
//
pub struct Definition {}
//class Definition:
//    def __init__(self, text):
//        self.text = text
//
//    def to_xml(self, xml_file, is_ili=False):
//        if is_ili:
//            xml_file.write("""      <ILIDefinition>%s</ILIDefinition>
//""" % escape_xml_lit(self.text))
//        else:
//            xml_file.write("""      <Definition>%s</Definition>
//""" % escape_xml_lit(self.text))
//
//    def __eq__(self, other):
//        return self.text == other.text
//
//
pub struct Example {}
//class Example:
//    def __init__(self, text, source=None):
//        self.text = text
//        self.source = source
//
//    def to_xml(self, xml_file):
//        if self.source:
//            xml_file.write("""      <Example dc:source=\"%s\">%s</Example>
//""" % (self.source, escape_xml_lit(self.text)))
//
//        else:
//            xml_file.write("""      <Example>%s</Example>
//""" % escape_xml_lit(self.text))
//
//
pub struct SynsetRelation {}
//class SynsetRelation:
//    def __init__(self, target, rel_type):
//        self.target = target
//        self.rel_type = rel_type
//
//    def to_xml(self, xml_file, comments):
//        xml_file.write("""      <SynsetRelation relType="%s" target="%s"/>""" % 
//                (self.rel_type.value, self.target))
//        if self.target in comments:
//            xml_file.write(""" <!-- %s -->
//""" % comments[self.target])
//        else:
//            xml_file.write("\n")
//
pub struct SenseRelation {}
//class SenseRelation:
//    def __init__(self, target, rel_type):
//        self.target = target
//        self.rel_type = rel_type
//
//    def to_xml(self, xml_file, comments):
//        xml_file.write("""        <SenseRelation relType="%s" target="%s"/>""" % 
//                (self.rel_type.value, self.target))
//        if self.target in comments:
//            xml_file.write(""" <!-- %s -->
//""" % comments[self.target])
//        else:
//            xml_file.write("\n")
//
//
pub struct SyntacticBehaviour {
    pub subcategorization_frame : String,
    pub senses : Vec<String>
}
//class SyntacticBehaviour:
//    def __init__(self, subcategorization_frame, senses):
//        if not isinstance(subcategorization_frame, str):
//            raise "Syntactic Behaviour is not string" + str(subcategorization_frame)
//        self.subcategorization_frame = subcategorization_frame
//        self.senses = senses
//
//    def to_xml(self, xml_file):
//        xml_file.write("""      <SyntacticBehaviour subcategorizationFrame="%s" senses="%s"/>
//""" % (escape_xml_lit(self.subcategorization_frame), " ".join(self.senses)))
//
//    def __repr__(self):
//        return "SyntacticBehaviour(%s, %s)" % (self.subcategorization_frame, " ".join(self.senses))
//
//
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
}
//
//def equal_pos(pos1, pos2):
//    return (pos1 == pos2 
//            or pos1 == PartOfSpeech.ADJECTIVE and pos2 == PartOfSpeech.ADJECTIVE_SATELLITE
//            or pos2 == PartOfSpeech.ADJECTIVE and pos1 == PartOfSpeech.ADJECTIVE_SATELLITE)
//
pub enum SynsetRelType { }
//class SynsetRelType(Enum):
//    AGENT = 'agent'
//    ALSO = 'also'
//    ATTRIBUTE = 'attribute'
//    BE_IN_STATE = 'be_in_state'
//    CAUSES = 'causes'
//    CLASSIFIED_BY = 'classified_by'
//    CLASSIFIES = 'classifies'
//    CO_AGENT_INSTRUMENT = 'co_agent_instrument'
//    CO_AGENT_PATIENT = 'co_agent_patient'
//    CO_AGENT_RESULT = 'co_agent_result'
//    CO_INSTRUMENT_AGENT = 'co_instrument_agent'
//    CO_INSTRUMENT_PATIENT = 'co_instrument_patient'
//    CO_INSTRUMENT_RESULT = 'co_instrument_result'
//    CO_PATIENT_AGENT = 'co_patient_agent'
//    CO_PATIENT_INSTRUMENT = 'co_patient_instrument'
//    CO_RESULT_AGENT = 'co_result_agent'
//    CO_RESULT_INSTRUMENT = 'co_result_instrument'
//    CO_ROLE = 'co_role'
//    DIRECTION = 'direction'
//    DOMAIN_REGION = 'domain_region'
//    DOMAIN_TOPIC = 'domain_topic'
//    EXEMPLIFIES = 'exemplifies'
//    ENTAILS = 'entails'
//    EQ_SYNONYM = 'eq_synonym'
//    HAS_DOMAIN_REGION = 'has_domain_region'
//    HAS_DOMAIN_TOPIC = 'has_domain_topic'
//    IS_EXEMPLIFIED_BY = 'is_exemplified_by'
//    HOLO_LOCATION = 'holo_location'
//    HOLO_MEMBER = 'holo_member'
//    HOLO_PART = 'holo_part'
//    HOLO_PORTION = 'holo_portion'
//    HOLO_SUBSTANCE = 'holo_substance'
//    HOLONYM = 'holonym'
//    HYPERNYM = 'hypernym'
//    HYPONYM = 'hyponym'
//    IN_MANNER = 'in_manner'
//    INSTANCE_HYPERNYM = 'instance_hypernym'
//    INSTANCE_HYPONYM = 'instance_hyponym'
//    INSTRUMENT = 'instrument'
//    INVOLVED = 'involved'
//    INVOLVED_AGENT = 'involved_agent'
//    INVOLVED_DIRECTION = 'involved_direction'
//    INVOLVED_INSTRUMENT = 'involved_instrument'
//    INVOLVED_LOCATION = 'involved_location'
//    INVOLVED_PATIENT = 'involved_patient'
//    INVOLVED_RESULT = 'involved_result'
//    INVOLVED_SOURCE_DIRECTION = 'involved_source_direction'
//    INVOLVED_TARGET_DIRECTION = 'involved_target_direction'
//    IS_CAUSED_BY = 'is_caused_by'
//    IS_ENTAILED_BY = 'is_entailed_by'
//    LOCATION = 'location'
//    MANNER_OF = 'manner_of'
//    MERO_LOCATION = 'mero_location'
//    MERO_MEMBER = 'mero_member'
//    MERO_PART = 'mero_part'
//    MERO_PORTION = 'mero_portion'
//    MERO_SUBSTANCE = 'mero_substance'
//    MERONYM = 'meronym'
//    SIMILAR = 'similar'
//    OTHER = 'other'
//    PATIENT = 'patient'
//    RESTRICTED_BY = 'restricted_by'
//    RESTRICTS = 'restricts'
//    RESULT = 'result'
//    ROLE = 'role'
//    SOURCE_DIRECTION = 'source_direction'
//    STATE_OF = 'state_of'
//    TARGET_DIRECTION = 'target_direction'
//    SUBEVENT = 'subevent'
//    IS_SUBEVENT_OF = 'is_subevent_of'
//    ANTONYM = 'antonym'
//
//inverse_synset_rels = {
//        SynsetRelType.HYPERNYM: SynsetRelType.HYPONYM,
//        SynsetRelType.HYPONYM: SynsetRelType.HYPERNYM,
//        SynsetRelType.INSTANCE_HYPERNYM: SynsetRelType.INSTANCE_HYPONYM,
//        SynsetRelType.INSTANCE_HYPONYM: SynsetRelType.INSTANCE_HYPERNYM,
//        SynsetRelType.MERONYM: SynsetRelType.HOLONYM,
//        SynsetRelType.HOLONYM: SynsetRelType.MERONYM,
//        SynsetRelType.MERO_LOCATION: SynsetRelType.HOLO_LOCATION,
//        SynsetRelType.HOLO_LOCATION: SynsetRelType.MERO_LOCATION,
//        SynsetRelType.MERO_MEMBER: SynsetRelType.HOLO_MEMBER,
//        SynsetRelType.HOLO_MEMBER: SynsetRelType.MERO_MEMBER,
//        SynsetRelType.MERO_PART: SynsetRelType.HOLO_PART,
//        SynsetRelType.HOLO_PART: SynsetRelType.MERO_PART,
//        SynsetRelType.MERO_PORTION: SynsetRelType.HOLO_PORTION,
//        SynsetRelType.HOLO_PORTION: SynsetRelType.MERO_PORTION,
//        SynsetRelType.MERO_SUBSTANCE: SynsetRelType.HOLO_SUBSTANCE,
//        SynsetRelType.HOLO_SUBSTANCE: SynsetRelType.MERO_SUBSTANCE,
//        SynsetRelType.BE_IN_STATE: SynsetRelType.STATE_OF,
//        SynsetRelType.STATE_OF: SynsetRelType.BE_IN_STATE,
//        SynsetRelType.CAUSES: SynsetRelType.IS_CAUSED_BY,
//        SynsetRelType.IS_CAUSED_BY: SynsetRelType.CAUSES,
//        SynsetRelType.SUBEVENT: SynsetRelType.IS_SUBEVENT_OF,
//        SynsetRelType.IS_SUBEVENT_OF: SynsetRelType.SUBEVENT,
//        SynsetRelType.MANNER_OF: SynsetRelType.IN_MANNER,
//        SynsetRelType.IN_MANNER: SynsetRelType.MANNER_OF,
//        SynsetRelType.RESTRICTS: SynsetRelType.RESTRICTED_BY,
//        SynsetRelType.RESTRICTED_BY: SynsetRelType.RESTRICTS,
//        SynsetRelType.CLASSIFIES: SynsetRelType.CLASSIFIED_BY,
//        SynsetRelType.CLASSIFIED_BY: SynsetRelType.CLASSIFIES,
//        SynsetRelType.ENTAILS: SynsetRelType.IS_ENTAILED_BY,
//        SynsetRelType.IS_ENTAILED_BY: SynsetRelType.ENTAILS,
//        SynsetRelType.DOMAIN_REGION: SynsetRelType.HAS_DOMAIN_REGION,
//        SynsetRelType.HAS_DOMAIN_REGION: SynsetRelType.DOMAIN_REGION,
//        SynsetRelType.DOMAIN_TOPIC: SynsetRelType.HAS_DOMAIN_TOPIC,
//        SynsetRelType.HAS_DOMAIN_TOPIC: SynsetRelType.DOMAIN_TOPIC,
//        SynsetRelType.EXEMPLIFIES: SynsetRelType.IS_EXEMPLIFIED_BY,
//        SynsetRelType.IS_EXEMPLIFIED_BY: SynsetRelType.EXEMPLIFIES,
//        SynsetRelType.ROLE: SynsetRelType.INVOLVED,
//        SynsetRelType.INVOLVED: SynsetRelType.ROLE,
//        SynsetRelType.AGENT: SynsetRelType.INVOLVED_AGENT,
//        SynsetRelType.INVOLVED_AGENT: SynsetRelType.AGENT,
//        SynsetRelType.PATIENT: SynsetRelType.INVOLVED_PATIENT,
//        SynsetRelType.INVOLVED_PATIENT: SynsetRelType.PATIENT,
//        SynsetRelType.RESULT: SynsetRelType.INVOLVED_RESULT,
//        SynsetRelType.INVOLVED_RESULT: SynsetRelType.RESULT,
//        SynsetRelType.INSTRUMENT: SynsetRelType.INVOLVED_INSTRUMENT,
//        SynsetRelType.INVOLVED_INSTRUMENT: SynsetRelType.INSTRUMENT,
//        SynsetRelType.LOCATION: SynsetRelType.INVOLVED_LOCATION,
//        SynsetRelType.INVOLVED_LOCATION: SynsetRelType.LOCATION,
//        SynsetRelType.DIRECTION: SynsetRelType.INVOLVED_DIRECTION,
//        SynsetRelType.INVOLVED_DIRECTION: SynsetRelType.DIRECTION,
//        SynsetRelType.TARGET_DIRECTION: SynsetRelType.INVOLVED_TARGET_DIRECTION,
//        SynsetRelType.INVOLVED_TARGET_DIRECTION: SynsetRelType.TARGET_DIRECTION,
//        SynsetRelType.SOURCE_DIRECTION: SynsetRelType.INVOLVED_SOURCE_DIRECTION,
//        SynsetRelType.INVOLVED_SOURCE_DIRECTION: SynsetRelType.SOURCE_DIRECTION,
//        SynsetRelType.CO_AGENT_PATIENT: SynsetRelType.CO_PATIENT_AGENT,
//        SynsetRelType.CO_PATIENT_AGENT: SynsetRelType.CO_AGENT_PATIENT,
//        SynsetRelType.CO_AGENT_INSTRUMENT: SynsetRelType.CO_INSTRUMENT_AGENT,
//        SynsetRelType.CO_INSTRUMENT_AGENT: SynsetRelType.CO_AGENT_INSTRUMENT,
//        SynsetRelType.CO_AGENT_RESULT: SynsetRelType.CO_RESULT_AGENT,
//        SynsetRelType.CO_RESULT_AGENT: SynsetRelType.CO_AGENT_RESULT,
//        SynsetRelType.CO_PATIENT_INSTRUMENT: SynsetRelType.CO_INSTRUMENT_PATIENT,
//        SynsetRelType.CO_INSTRUMENT_PATIENT: SynsetRelType.CO_PATIENT_INSTRUMENT,
//        SynsetRelType.CO_RESULT_INSTRUMENT: SynsetRelType.CO_INSTRUMENT_RESULT,
//        SynsetRelType.CO_INSTRUMENT_RESULT: SynsetRelType.CO_RESULT_INSTRUMENT,
//        SynsetRelType.ANTONYM: SynsetRelType.ANTONYM,
//        SynsetRelType.EQ_SYNONYM: SynsetRelType.EQ_SYNONYM,
//        SynsetRelType.SIMILAR: SynsetRelType.SIMILAR,
//#        SynsetRelType.ALSO: SynsetRelType.ALSO,
//        SynsetRelType.ATTRIBUTE: SynsetRelType.ATTRIBUTE,
//        SynsetRelType.CO_ROLE: SynsetRelType.CO_ROLE
//        }
//
//class SenseRelType(Enum):
//    ANTONYM = 'antonym'
//    ALSO = 'also'
//    PARTICIPLE = 'participle'
//    PERTAINYM = 'pertainym'
//    DERIVATION = 'derivation'
//    DOMAIN_TOPIC = 'domain_topic'
//    HAS_DOMAIN_TOPIC = 'has_domain_topic'
//    DOMAIN_REGION = 'domain_region'
//    HAS_DOMAIN_REGION = 'has_domain_region'
//    EXEMPLIFIES = 'exemplifies'
//    IS_EXEMPLIFIED_BY = 'is_exemplified_by'
//    SIMILAR = 'similar'
//    OTHER = 'other'
//    
//inverse_sense_rels = {
//        SenseRelType.DOMAIN_REGION: SenseRelType.HAS_DOMAIN_REGION,
//        SenseRelType.HAS_DOMAIN_REGION: SenseRelType.DOMAIN_REGION,
//        SenseRelType.DOMAIN_TOPIC: SenseRelType.HAS_DOMAIN_TOPIC,
//        SenseRelType.HAS_DOMAIN_TOPIC: SenseRelType.DOMAIN_TOPIC,
//        SenseRelType.EXEMPLIFIES: SenseRelType.IS_EXEMPLIFIED_BY,
//        SenseRelType.IS_EXEMPLIFIED_BY: SenseRelType.EXEMPLIFIES,
//        SenseRelType.ANTONYM: SenseRelType.ANTONYM,
//        SenseRelType.SIMILAR: SenseRelType.SIMILAR,
//        SenseRelType.ALSO: SenseRelType.ALSO,
//        SenseRelType.DERIVATION: SenseRelType.DERIVATION,
//        }
//
//class WordNetContentHandler(ContentHandler):
//    def __init__(self):
//        ContentHandler.__init__(self)
//        self.lexicon = None
//        self.entry = None
//        self.sense = None
//        self.defn = None
//        self.ili_defn = None
//        self.example = None
//        self.example_source = None
//        self.synset = None
//
//    def startElement(self, name, attrs):
//        if name == "Lexicon":
//            self.lexicon = Lexicon(attrs["id"], attrs["label"], attrs["language"],
//                    attrs["email"], attrs["license"], attrs["version"], attrs["url"])
//        elif name == "LexicalEntry":
//            self.entry = LexicalEntry(attrs["id"])
//        elif name == "Lemma":
//            self.entry.set_lemma(Lemma(attrs["writtenForm"], PartOfSpeech(attrs["partOfSpeech"])))
//        elif name == "Form":
//            self.entry.add_form(Form(attrs["writtenForm"]))
//        elif name == "Sense":
//            if "n" in attrs:
//                n = int(attrs["n"])
//            else:
//                n = -1
//            self.sense = Sense(attrs["id"], attrs["synset"], 
//                    attrs.get("dc:identifier") or "", n, attrs.get("adjposition"))
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
