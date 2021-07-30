use lazy_static::lazy_static;
use std::collections::HashMap;

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

    pub fn from(v : &str) -> Option<SynsetRelType> {
        match v {
            "agent" => Some(SynsetRelType::Agent),
            "also" => Some(SynsetRelType::Also),
            "attribute" => Some(SynsetRelType::Attribute),
            "be_in_state" => Some(SynsetRelType::BeInState),
            "causes" => Some(SynsetRelType::Causes),
            "classified_by" => Some(SynsetRelType::ClassifiedBy),
            "classifies" => Some(SynsetRelType::Classifies),
            "co_agent_instrument" => Some(SynsetRelType::CoAgentInstrument),
            "co_agent_patient" => Some(SynsetRelType::CoAgentPatient),
            "co_agent_result" => Some(SynsetRelType::CoAgentResult),
            "co_instrument_agent" => Some(SynsetRelType::CoInstrumentAgent),
            "co_instrument_patient" => Some(SynsetRelType::CoInstrumentPatient),
            "co_instrument_result" => Some(SynsetRelType::CoInstrumentResult),
            "co_patient_agent" => Some(SynsetRelType::CoPatientAgent),
            "co_patient_instrument" => Some(SynsetRelType::CoPatientInstrument),
            "co_result_agent" => Some(SynsetRelType::CoResultAgent),
            "co_result_instrument" => Some(SynsetRelType::CoResultInstrument),
            "co_role" => Some(SynsetRelType::CoRole),
            "direction" => Some(SynsetRelType::Direction),
            "domain_region" => Some(SynsetRelType::DomainRegion),
            "domain_topic" => Some(SynsetRelType::DomainTopic),
            "exemplifies" => Some(SynsetRelType::Exemplifies),
            "entails" => Some(SynsetRelType::Entails),
            "eq_synonym" => Some(SynsetRelType::EqSynonym),
            "has_domain_region" => Some(SynsetRelType::HasDomainRegion),
            "has_domain_topic" => Some(SynsetRelType::HasDomainTopic),
            "is_exemplified_by" => Some(SynsetRelType::IsExemplifiedBy),
            "holo_location" => Some(SynsetRelType::HoloLocation),
            "holo_member" => Some(SynsetRelType::HoloMember),
            "holo_part" => Some(SynsetRelType::HoloPart),
            "holo_portion" => Some(SynsetRelType::HoloPortion),
            "holo_substance" => Some(SynsetRelType::HoloSubstance),
            "holonym" => Some(SynsetRelType::Holonym),
            "hypernym" => Some(SynsetRelType::Hypernym),
            "hyponym" => Some(SynsetRelType::Hyponym),
            "in_manner" => Some(SynsetRelType::InManner),
            "instance_hypernym" => Some(SynsetRelType::InstanceHypernym),
            "instance_hyponym" => Some(SynsetRelType::InstanceHyponym),
            "instrument" => Some(SynsetRelType::Instrument),
            "involved" => Some(SynsetRelType::Involved),
            "involved_agent" => Some(SynsetRelType::InvolvedAgent),
            "involved_direction" => Some(SynsetRelType::InvolvedDirection),
            "involved_instrument" => Some(SynsetRelType::InvolvedInstrument),
            "involved_location" => Some(SynsetRelType::InvolvedLocation),
            "involved_patient" => Some(SynsetRelType::InvolvedPatient),
            "involved_result" => Some(SynsetRelType::InvolvedResult),
            "involved_source_direction" => Some(SynsetRelType::InvolvedSourceDirection),
            "involved_target_direction" => Some(SynsetRelType::InvolvedTargetDirection),
            "is_caused_by" => Some(SynsetRelType::IsCausedBy),
            "is_entailed_by" => Some(SynsetRelType::IsEntailedBy),
            "location" => Some(SynsetRelType::Location),
            "manner_of" => Some(SynsetRelType::MannerOf),
            "mero_location" => Some(SynsetRelType::MeroLocation),
            "mero_member" => Some(SynsetRelType::MeroMember),
            "mero_part" => Some(SynsetRelType::MeroPart),
            "mero_portion" => Some(SynsetRelType::MeroPortion),
            "mero_substance" => Some(SynsetRelType::MeroSubstance),
            "meronym" => Some(SynsetRelType::Meronym),
            "similar" => Some(SynsetRelType::Similar),
            "other" => Some(SynsetRelType::Other),
            "patient" => Some(SynsetRelType::Patient),
            "restricted_by" => Some(SynsetRelType::RestrictedBy),
            "restricts" => Some(SynsetRelType::Restricts),
            "result" => Some(SynsetRelType::Result),
            "role" => Some(SynsetRelType::Role),
            "source_direction" => Some(SynsetRelType::SourceDirection),
            "state_of" => Some(SynsetRelType::StateOf),
            "target_direction" => Some(SynsetRelType::TargetDirection),
            "subevent" => Some(SynsetRelType::Subevent),
            "is_subevent_of" => Some(SynsetRelType::IsSubeventOf),
            "antonym" => Some(SynsetRelType::Antonym),
            _ => None
        }
    }

    pub fn to_yaml(self) -> (bool, YamlSynsetRelType) {
        match self {
            SynsetRelType::Agent => (true, YamlSynsetRelType::Agent),
            SynsetRelType::Also => (true, YamlSynsetRelType::Also),
            SynsetRelType::Attribute => (true, YamlSynsetRelType::Attribute),
            SynsetRelType::BeInState => (true, YamlSynsetRelType::BeInState),
            SynsetRelType::Causes => (true, YamlSynsetRelType::Causes),
            SynsetRelType::ClassifiedBy => (false, YamlSynsetRelType::Classifies),
            SynsetRelType::Classifies => (true, YamlSynsetRelType::Classifies),
            SynsetRelType::CoAgentInstrument => (true, YamlSynsetRelType::CoAgentInstrument),
            SynsetRelType::CoAgentPatient => (true, YamlSynsetRelType::CoAgentPatient),
            SynsetRelType::CoAgentResult => (true, YamlSynsetRelType::CoAgentResult),
            SynsetRelType::CoInstrumentAgent => (false, YamlSynsetRelType::CoAgentInstrument),
            SynsetRelType::CoInstrumentPatient => (false, YamlSynsetRelType::CoPatientInstrument),
            SynsetRelType::CoInstrumentResult => (false, YamlSynsetRelType::CoResultInstrument),
            SynsetRelType::CoPatientAgent => (false, YamlSynsetRelType::CoAgentPatient),
            SynsetRelType::CoPatientInstrument => (true, YamlSynsetRelType::CoPatientInstrument),
            SynsetRelType::CoResultAgent => (false, YamlSynsetRelType::CoAgentResult),
            SynsetRelType::CoResultInstrument => (true, YamlSynsetRelType::CoResultInstrument),
            SynsetRelType::CoRole => (true, YamlSynsetRelType::CoRole),
            SynsetRelType::Direction => (true, YamlSynsetRelType::Direction),
            SynsetRelType::DomainRegion => (true, YamlSynsetRelType::DomainRegion),
            SynsetRelType::DomainTopic => (true, YamlSynsetRelType::DomainTopic),
            SynsetRelType::Exemplifies => (true, YamlSynsetRelType::Exemplifies),
            SynsetRelType::Entails => (true, YamlSynsetRelType::Entails),
            SynsetRelType::EqSynonym => (true, YamlSynsetRelType::EqSynonym),
            SynsetRelType::HasDomainRegion => (false, YamlSynsetRelType::DomainRegion),
            SynsetRelType::HasDomainTopic => (false, YamlSynsetRelType::DomainTopic),
            SynsetRelType::IsExemplifiedBy => (false, YamlSynsetRelType::Exemplifies),
            SynsetRelType::HoloLocation => (false, YamlSynsetRelType::MeroLocation),
            SynsetRelType::HoloMember => (false, YamlSynsetRelType::MeroMember),
            SynsetRelType::HoloPart => (false, YamlSynsetRelType::MeroPart),
            SynsetRelType::HoloPortion => (false, YamlSynsetRelType::MeroPortion),
            SynsetRelType::HoloSubstance => (false, YamlSynsetRelType::MeroSubstance),
            SynsetRelType::Holonym => (false, YamlSynsetRelType::Meronym),
            SynsetRelType::Hypernym => (true, YamlSynsetRelType::Hypernym),
            SynsetRelType::Hyponym => (false, YamlSynsetRelType::Hypernym),
            SynsetRelType::InManner => (false, YamlSynsetRelType::MannerOf),
            SynsetRelType::InstanceHypernym => (true, YamlSynsetRelType::InstanceHypernym),
            SynsetRelType::InstanceHyponym => (false, YamlSynsetRelType::InstanceHypernym),
            SynsetRelType::Instrument => (true, YamlSynsetRelType::Instrument),
            SynsetRelType::Involved => (false, YamlSynsetRelType::Role),
            SynsetRelType::InvolvedAgent => (false, YamlSynsetRelType::Agent),
            SynsetRelType::InvolvedDirection => (false, YamlSynsetRelType::Direction),
            SynsetRelType::InvolvedInstrument => (false, YamlSynsetRelType::Instrument),
            SynsetRelType::InvolvedLocation => (false, YamlSynsetRelType::Location),
            SynsetRelType::InvolvedPatient => (false, YamlSynsetRelType::Patient),
            SynsetRelType::InvolvedResult => (false, YamlSynsetRelType::Result),
            SynsetRelType::InvolvedSourceDirection => (false, YamlSynsetRelType::SourceDirection),
            SynsetRelType::InvolvedTargetDirection => (false, YamlSynsetRelType::TargetDirection),
            SynsetRelType::IsCausedBy => (false, YamlSynsetRelType::Causes),
            SynsetRelType::IsEntailedBy => (false, YamlSynsetRelType::Entails),
            SynsetRelType::Location => (true, YamlSynsetRelType::Location),
            SynsetRelType::MannerOf => (true, YamlSynsetRelType::MannerOf),
            SynsetRelType::MeroLocation => (true, YamlSynsetRelType::MeroLocation),
            SynsetRelType::MeroMember => (true, YamlSynsetRelType::MeroMember),
            SynsetRelType::MeroPart => (true, YamlSynsetRelType::MeroPart),
            SynsetRelType::MeroPortion => (true, YamlSynsetRelType::MeroPortion),
            SynsetRelType::MeroSubstance => (true, YamlSynsetRelType::MeroSubstance),
            SynsetRelType::Meronym => (true, YamlSynsetRelType::Meronym),
            SynsetRelType::Similar => (true, YamlSynsetRelType::Similar),
            SynsetRelType::Other => (true, YamlSynsetRelType::Other),
            SynsetRelType::Patient => (true, YamlSynsetRelType::Patient),
            SynsetRelType::RestrictedBy => (false, YamlSynsetRelType::Restricts),
            SynsetRelType::Restricts => (true, YamlSynsetRelType::Restricts),
            SynsetRelType::Result => (true, YamlSynsetRelType::Result),
            SynsetRelType::Role => (true, YamlSynsetRelType::Role),
            SynsetRelType::SourceDirection => (true, YamlSynsetRelType::SourceDirection),
            SynsetRelType::StateOf => panic!("TODO"),
            SynsetRelType::TargetDirection => (true, YamlSynsetRelType::TargetDirection),
            SynsetRelType::Subevent => (true, YamlSynsetRelType::Subevent),
            SynsetRelType::IsSubeventOf => (false, YamlSynsetRelType::Subevent),
            SynsetRelType::Antonym => (true, YamlSynsetRelType::Antonym)
        }
    }
}

pub enum YamlSynsetRelType {
    Agent,
    Also,
    Attribute,
    BeInState,
    Causes,
    Classifies,
    CoAgentInstrument,
    CoAgentPatient,
    CoAgentResult,
    CoPatientInstrument,
    CoResultInstrument,
    CoRole,
    Direction,
    DomainRegion,
    DomainTopic,
    Exemplifies,
    Entails,
    EqSynonym,
    Hypernym,
    InstanceHypernym,
    Instrument,
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
    Restricts,
    Result,
    Role,
    SourceDirection,
    TargetDirection,
    Subevent,
    Antonym
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

    pub fn from(v : &str) -> Option<SenseRelType> {
        match v {
            "antonym" => Some(SenseRelType::Antonym),
            "also" => Some(SenseRelType::Also),
            "participle" => Some(SenseRelType::Participle),
            "pertainym" => Some(SenseRelType::Pertainym),
            "derivation" => Some(SenseRelType::Derivation),
            "domain_topic" => Some(SenseRelType::DomainTopic),
            "has_domain_topic" => Some(SenseRelType::HasDomainTopic),
            "domain_region" => Some(SenseRelType::DomainRegion),
            "has_domain_region" => Some(SenseRelType::HasDomainRegion),
            "exemplifies" => Some(SenseRelType::Exemplifies),
            "is_exemplified_by" => Some(SenseRelType::IsExemplifiedBy),
            "similar" => Some(SenseRelType::Similar),
            "other" => Some(SenseRelType::Other),
            _ => None
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

