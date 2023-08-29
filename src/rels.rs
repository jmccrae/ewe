use crate::wordnet::PartOfSpeech;

#[derive(Clone,PartialEq,Debug,Eq,Hash)]
pub enum SynsetRelType { 
//  Agent,
    Also,
    Attribute,
//  BeInState,
    Causes,
//  ClassifiedBy,
//  Classifies,
//    CoAgentInstrument,
//    CoAgentPatient,
//    CoAgentResult,
//    CoInstrumentAgent,
//    CoInstrumentPatient,
//    CoInstrumentResult,
//    CoPatientAgent,
//    CoPatientInstrument,
//    CoResultAgent,
//    CoResultInstrument,
//    CoRole,
//    Direction,
    DomainRegion,
    DomainTopic,
    Exemplifies,
    Entails,
//    EqSynonym,
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
//    InManner,
    InstanceHypernym,
    InstanceHyponym,
//    Instrument,
//    Involved,
//    InvolvedAgent,
//    InvolvedDirection,
//    InvolvedInstrument,
//    InvolvedLocation,
//    InvolvedPatient,
//    InvolvedResult,
//    InvolvedSourceDirection,
//    InvolvedTargetDirection,
    IsCausedBy,
    IsEntailedBy,
//    Location,
//    MannerOf,
    MeroLocation,
    MeroMember,
    MeroPart,
    MeroPortion,
    MeroSubstance,
    Meronym,
    Similar,
    Feminine,
    Masculine,
    Other
//    Patient,
//    RestrictedBy,
//    Restricts,
//    Result,
//    Role,
//    SourceDirection,
// StateOf,
//    TargetDirection,
//    Subevent,
//    IsSubeventOf,
//    Antonym
}

impl SynsetRelType {
    pub fn value(&self) -> &'static str {
        match self {
//            SynsetRelType::Agent => "agent",
            SynsetRelType::Also => "also",
            SynsetRelType::Attribute => "attribute",
            //SynsetRelType::BeInState => "be_in_state",
            SynsetRelType::Causes => "causes",
            //SynsetRelType::ClassifiedBy => "classified_by",
            //SynsetRelType::Classifies => "classifies",
            //SynsetRelType::CoAgentInstrument => "co_agent_instrument",
            //SynsetRelType::CoAgentPatient => "co_agent_patient",
            //SynsetRelType::CoAgentResult => "co_agent_result",
            //SynsetRelType::CoInstrumentAgent => "co_instrument_agent",
            //SynsetRelType::CoInstrumentPatient => "co_instrument_patient",
            //SynsetRelType::CoInstrumentResult => "co_instrument_result",
            //SynsetRelType::CoPatientAgent => "co_patient_agent",
            //SynsetRelType::CoPatientInstrument => "co_patient_instrument",
            //SynsetRelType::CoResultAgent => "co_result_agent",
            //SynsetRelType::CoResultInstrument => "co_result_instrument",
            //SynsetRelType::CoRole => "co_role",
            //SynsetRelType::Direction => "direction",
            SynsetRelType::DomainRegion => "domain_region",
            SynsetRelType::DomainTopic => "domain_topic",
            SynsetRelType::Exemplifies => "exemplifies",
            SynsetRelType::Entails => "entails",
            //SynsetRelType::EqSynonym => "eq_synonym",
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
            //SynsetRelType::InManner => "in_manner",
            SynsetRelType::InstanceHypernym => "instance_hypernym",
            SynsetRelType::InstanceHyponym => "instance_hyponym",
            //SynsetRelType::Instrument => "instrument",
            //SynsetRelType::Involved => "involved",
            //SynsetRelType::InvolvedAgent => "involved_agent",
            //SynsetRelType::InvolvedDirection => "involved_direction",
            //SynsetRelType::InvolvedInstrument => "involved_instrument",
            //SynsetRelType::InvolvedLocation => "involved_location",
            //SynsetRelType::InvolvedPatient => "involved_patient",
            //SynsetRelType::InvolvedResult => "involved_result",
            //SynsetRelType::InvolvedSourceDirection => "involved_source_direction",
            //SynsetRelType::InvolvedTargetDirection => "involved_target_direction",
            SynsetRelType::IsCausedBy => "is_caused_by",
            SynsetRelType::IsEntailedBy => "is_entailed_by",
            //SynsetRelType::Location => "location",
            //SynsetRelType::MannerOf => "manner_of",
            SynsetRelType::MeroLocation => "mero_location",
            SynsetRelType::MeroMember => "mero_member",
            SynsetRelType::MeroPart => "mero_part",
            SynsetRelType::MeroPortion => "mero_portion",
            SynsetRelType::MeroSubstance => "mero_substance",
            SynsetRelType::Meronym => "meronym",
            SynsetRelType::Similar => "similar",
            SynsetRelType::Other => "other",
            SynsetRelType::Feminine => "feminine",
            SynsetRelType::Masculine => "masculine",
            //SynsetRelType::Patient => "patient",
            //SynsetRelType::RestrictedBy => "restricted_by",
            //SynsetRelType::Restricts => "restricts",
            //SynsetRelType::Result => "result",
            //SynsetRelType::Role => "role",
            //SynsetRelType::SourceDirection => "source_direction",
            //SynsetRelType::StateOf => "state_of",
            //SynsetRelType::TargetDirection => "target_direction",
            //SynsetRelType::Subevent => "subevent",
            //SynsetRelType::IsSubeventOf => "is_subevent_of",
            //SynsetRelType::Antonym => "antonym"
        }
    }

    pub fn from(v : &str) -> Option<SynsetRelType> {
        match v {
            //"agent" => Some(SynsetRelType::Agent),
            "also" => Some(SynsetRelType::Also),
            "attribute" => Some(SynsetRelType::Attribute),
            //"be_in_state" => Some(SynsetRelType::BeInState),
            "causes" => Some(SynsetRelType::Causes),
            //"classified_by" => Some(SynsetRelType::ClassifiedBy),
            //"classifies" => Some(SynsetRelType::Classifies),
            //"co_agent_instrument" => Some(SynsetRelType::CoAgentInstrument),
            //"co_agent_patient" => Some(SynsetRelType::CoAgentPatient),
            //"co_agent_result" => Some(SynsetRelType::CoAgentResult),
            //"co_instrument_agent" => Some(SynsetRelType::CoInstrumentAgent),
            //"co_instrument_patient" => Some(SynsetRelType::CoInstrumentPatient),
            //"co_instrument_result" => Some(SynsetRelType::CoInstrumentResult),
            //"co_patient_agent" => Some(SynsetRelType::CoPatientAgent),
            //"co_patient_instrument" => Some(SynsetRelType::CoPatientInstrument),
            //"co_result_agent" => Some(SynsetRelType::CoResultAgent),
            //"co_result_instrument" => Some(SynsetRelType::CoResultInstrument),
            //"co_role" => Some(SynsetRelType::CoRole),
            //"direction" => Some(SynsetRelType::Direction),
            "domain_region" => Some(SynsetRelType::DomainRegion),
            "domain_topic" => Some(SynsetRelType::DomainTopic),
            "exemplifies" => Some(SynsetRelType::Exemplifies),
            "entails" => Some(SynsetRelType::Entails),
            //"eq_synonym" => Some(SynsetRelType::EqSynonym),
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
            //"in_manner" => Some(SynsetRelType::InManner),
            "instance_hypernym" => Some(SynsetRelType::InstanceHypernym),
            "instance_hyponym" => Some(SynsetRelType::InstanceHyponym),
            //"instrument" => Some(SynsetRelType::Instrument),
            //"involved" => Some(SynsetRelType::Involved),
            //"involved_agent" => Some(SynsetRelType::InvolvedAgent),
            //"involved_direction" => Some(SynsetRelType::InvolvedDirection),
            //"involved_instrument" => Some(SynsetRelType::InvolvedInstrument),
            //"involved_location" => Some(SynsetRelType::InvolvedLocation),
            //"involved_patient" => Some(SynsetRelType::InvolvedPatient),
            //"involved_result" => Some(SynsetRelType::InvolvedResult),
            //"involved_source_direction" => Some(SynsetRelType::InvolvedSourceDirection),
            //"involved_target_direction" => Some(SynsetRelType::InvolvedTargetDirection),
            "is_caused_by" => Some(SynsetRelType::IsCausedBy),
            "is_entailed_by" => Some(SynsetRelType::IsEntailedBy),
            //"location" => Some(SynsetRelType::Location),
            //"manner_of" => Some(SynsetRelType::MannerOf),
            "mero_location" => Some(SynsetRelType::MeroLocation),
            "mero_member" => Some(SynsetRelType::MeroMember),
            "mero_part" => Some(SynsetRelType::MeroPart),
            "mero_portion" => Some(SynsetRelType::MeroPortion),
            "mero_substance" => Some(SynsetRelType::MeroSubstance),
            "meronym" => Some(SynsetRelType::Meronym),
            "similar" => Some(SynsetRelType::Similar),
            //"other" => Some(SynsetRelType::Other),
            //"patient" => Some(SynsetRelType::Patient),
            //"restricted_by" => Some(SynsetRelType::RestrictedBy),
            //"restricts" => Some(SynsetRelType::Restricts),
            //"result" => Some(SynsetRelType::Result),
            //"role" => Some(SynsetRelType::Role),
            //"source_direction" => Some(SynsetRelType::SourceDirection),
            //"state_of" => Some(SynsetRelType::StateOf),
            //"target_direction" => Some(SynsetRelType::TargetDirection),
            //"subevent" => Some(SynsetRelType::Subevent),
            //"is_subevent_of" => Some(SynsetRelType::IsSubeventOf),
            //"antonym" => Some(SynsetRelType::Antonym),
            "feminine" => Some(SynsetRelType::Feminine),
            "masculine" => Some(SynsetRelType::Masculine),
            _ => None
        }
    }

    pub fn to_yaml(self) -> (bool, YamlSynsetRelType) {
        match self {
            SynsetRelType::Also => (true, YamlSynsetRelType::Also),
            SynsetRelType::Attribute => (true, YamlSynsetRelType::Attribute),
            SynsetRelType::Causes => (true, YamlSynsetRelType::Causes),
            SynsetRelType::DomainRegion => (true, YamlSynsetRelType::DomainRegion),
            SynsetRelType::DomainTopic => (true, YamlSynsetRelType::DomainTopic),
            SynsetRelType::Exemplifies => (true, YamlSynsetRelType::Exemplifies),
            SynsetRelType::Entails => (true, YamlSynsetRelType::Entails),
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
            SynsetRelType::InstanceHypernym => (true, YamlSynsetRelType::InstanceHypernym),
            SynsetRelType::InstanceHyponym => (false, YamlSynsetRelType::InstanceHypernym),
            SynsetRelType::IsCausedBy => (false, YamlSynsetRelType::Causes),
            SynsetRelType::IsEntailedBy => (false, YamlSynsetRelType::Entails),
            SynsetRelType::MeroLocation => (true, YamlSynsetRelType::MeroLocation),
            SynsetRelType::MeroMember => (true, YamlSynsetRelType::MeroMember),
            SynsetRelType::MeroPart => (true, YamlSynsetRelType::MeroPart),
            SynsetRelType::MeroPortion => (true, YamlSynsetRelType::MeroPortion),
            SynsetRelType::MeroSubstance => (true, YamlSynsetRelType::MeroSubstance),
            SynsetRelType::Meronym => (true, YamlSynsetRelType::Meronym),
            SynsetRelType::Similar => (true, YamlSynsetRelType::Similar),
            SynsetRelType::Feminine => (true, YamlSynsetRelType::Feminine),
            SynsetRelType::Masculine => (true, YamlSynsetRelType::Masculine),
            SynsetRelType::Other => (true, YamlSynsetRelType::Other),
        }
    }

    pub fn is_symmetric(&self) -> bool {
        match self {
            SynsetRelType::Similar => true,
            _ => false
        }
    }

    pub fn pos(&self) -> Vec<&'static PartOfSpeech> {
        match self {
            SynsetRelType::Also => vec![&PartOfSpeech::n, &PartOfSpeech::v, &PartOfSpeech::a, &PartOfSpeech::r, &PartOfSpeech::s],
            SynsetRelType::Attribute => vec![&PartOfSpeech::n, &PartOfSpeech::a, &PartOfSpeech::s],
            SynsetRelType::Causes => vec![&PartOfSpeech::v], 
            SynsetRelType::DomainRegion => vec![&PartOfSpeech::n, &PartOfSpeech::v, &PartOfSpeech::a, &PartOfSpeech::r, &PartOfSpeech::s],
            SynsetRelType::DomainTopic => vec![&PartOfSpeech::n, &PartOfSpeech::v, &PartOfSpeech::a, &PartOfSpeech::r, &PartOfSpeech::s],
            SynsetRelType::Exemplifies => vec![&PartOfSpeech::n, &PartOfSpeech::v, &PartOfSpeech::a, &PartOfSpeech::r, &PartOfSpeech::s],
            SynsetRelType::Entails => vec![&PartOfSpeech::v],
            SynsetRelType::HasDomainRegion => vec![&PartOfSpeech::n, &PartOfSpeech::v, &PartOfSpeech::a, &PartOfSpeech::r, &PartOfSpeech::s],
            SynsetRelType::HasDomainTopic => vec![&PartOfSpeech::n, &PartOfSpeech::v, &PartOfSpeech::a, &PartOfSpeech::r, &PartOfSpeech::s],
            SynsetRelType::IsExemplifiedBy => vec![&PartOfSpeech::n, &PartOfSpeech::v, &PartOfSpeech::a, &PartOfSpeech::r, &PartOfSpeech::s],
            SynsetRelType::HoloLocation => vec![&PartOfSpeech::n],
            SynsetRelType::HoloMember => vec![&PartOfSpeech::n],
            SynsetRelType::HoloPart => vec![&PartOfSpeech::n],
            SynsetRelType::HoloPortion => vec![&PartOfSpeech::n],
            SynsetRelType::HoloSubstance => vec![&PartOfSpeech::n],
            SynsetRelType::Holonym => vec![&PartOfSpeech::n],
            SynsetRelType::Hypernym => vec![&PartOfSpeech::n, &PartOfSpeech::v],
            SynsetRelType::Hyponym => vec![&PartOfSpeech::n, &PartOfSpeech::v],
            SynsetRelType::InstanceHypernym => vec![&PartOfSpeech::n],
            SynsetRelType::InstanceHyponym => vec![&PartOfSpeech::n],
            SynsetRelType::IsCausedBy => vec![&PartOfSpeech::v],
            SynsetRelType::IsEntailedBy => vec![&PartOfSpeech::v],
            SynsetRelType::MeroLocation => vec![&PartOfSpeech::n],
            SynsetRelType::MeroMember => vec![&PartOfSpeech::n],
            SynsetRelType::MeroPart => vec![&PartOfSpeech::n],
            SynsetRelType::MeroPortion => vec![&PartOfSpeech::n],
            SynsetRelType::MeroSubstance => vec![&PartOfSpeech::n],
            SynsetRelType::Meronym => vec![&PartOfSpeech::n],
            SynsetRelType::Similar => vec![&PartOfSpeech::v, &PartOfSpeech::a, &PartOfSpeech::s],
            SynsetRelType::Feminine => vec![&PartOfSpeech::n],
            SynsetRelType::Masculine => vec![&PartOfSpeech::n],
            SynsetRelType::Other => vec![&PartOfSpeech::n, &PartOfSpeech::v, &PartOfSpeech::a, &PartOfSpeech::r, &PartOfSpeech::s]
        }
    }
}

pub enum YamlSynsetRelType {
    //Agent,
    Also,
    Attribute,
    //BeInState,
    Causes,
    //Classifies,
    //CoAgentInstrument,
    //CoAgentPatient,
    //CoAgentResult,
    //CoPatientInstrument,
    //CoResultInstrument,
    //CoRole,
    //Direction,
    DomainRegion,
    DomainTopic,
    Exemplifies,
    Entails,
    //EqSynonym,
    Hypernym,
    InstanceHypernym,
    //Instrument,
    //Location,
    //MannerOf,
    MeroLocation,
    MeroMember,
    MeroPart,
    MeroPortion,
    MeroSubstance,
    Meronym,
    Similar,
    Feminine,
    Masculine,
    Other
    //Patient,
    //Restricts,
    //Result,
    //Role,
    //SourceDirection,
    //TargetDirection,
    //Subevent,
    //Antonym
}

//lazy_static! {
//    static ref INVERSE_SYNSET_RELS : HashMap<SynsetRelType, SynsetRelType> = {
//        let mut map = HashMap::new();
//        //map.insert(SynsetRelType::Agent, SynsetRelType::InvolvedAgent);
//        map.insert(SynsetRelType::Also, SynsetRelType::Also);
//        //map.insert(SynsetRelType::Antonym, SynsetRelType::Antonym);
//        map.insert(SynsetRelType::Attribute, SynsetRelType::Attribute);
//        //map.insert(SynsetRelType::BeInState, SynsetRelType::StateOf);
//        map.insert(SynsetRelType::Causes, SynsetRelType::IsCausedBy);
//        //map.insert(SynsetRelType::ClassifiedBy, SynsetRelType::Classifies);
//        //map.insert(SynsetRelType::Classifies, SynsetRelType::ClassifiedBy);
//        //map.insert(SynsetRelType::CoAgentInstrument, SynsetRelType::CoInstrumentAgent);
//        //map.insert(SynsetRelType::CoAgentPatient, SynsetRelType::CoPatientAgent);
//        //map.insert(SynsetRelType::CoAgentResult, SynsetRelType::CoResultAgent);
//        //map.insert(SynsetRelType::CoInstrumentAgent, SynsetRelType::CoAgentInstrument);
//        //map.insert(SynsetRelType::CoInstrumentPatient, SynsetRelType::CoPatientInstrument);
//        //map.insert(SynsetRelType::CoInstrumentResult, SynsetRelType::CoResultInstrument);
//        //map.insert(SynsetRelType::CoPatientAgent, SynsetRelType::CoAgentPatient);
//        //map.insert(SynsetRelType::CoPatientInstrument, SynsetRelType::CoInstrumentPatient);
//        //map.insert(SynsetRelType::CoResultAgent, SynsetRelType::CoAgentResult);
//        //map.insert(SynsetRelType::CoResultInstrument, SynsetRelType::CoInstrumentResult);
//        //map.insert(SynsetRelType::CoRole, SynsetRelType::CoRole);
//        //map.insert(SynsetRelType::Direction, SynsetRelType::InvolvedDirection);
//        map.insert(SynsetRelType::DomainRegion, SynsetRelType::HasDomainRegion);
//        map.insert(SynsetRelType::DomainTopic, SynsetRelType::HasDomainTopic);
//        map.insert(SynsetRelType::Entails, SynsetRelType::IsEntailedBy);
//        //map.insert(SynsetRelType::EqSynonym, SynsetRelType::EqSynonym);
//        map.insert(SynsetRelType::Exemplifies, SynsetRelType::IsExemplifiedBy);
//        map.insert(SynsetRelType::HasDomainRegion, SynsetRelType::DomainRegion);
//        map.insert(SynsetRelType::HasDomainTopic, SynsetRelType::DomainTopic);
//        map.insert(SynsetRelType::HoloLocation, SynsetRelType::MeroLocation);
//        map.insert(SynsetRelType::HoloMember, SynsetRelType::MeroMember);
//        map.insert(SynsetRelType::HoloPart, SynsetRelType::MeroPart);
//        map.insert(SynsetRelType::HoloPortion, SynsetRelType::MeroPortion);
//        map.insert(SynsetRelType::HoloSubstance, SynsetRelType::MeroSubstance);
//        map.insert(SynsetRelType::Holonym, SynsetRelType::Meronym);
//        map.insert(SynsetRelType::Hypernym, SynsetRelType::Hyponym);
//        map.insert(SynsetRelType::Hyponym, SynsetRelType::Hypernym);
//        //map.insert(SynsetRelType::InManner, SynsetRelType::MannerOf);
//        map.insert(SynsetRelType::InstanceHypernym, SynsetRelType::InstanceHyponym);
//        map.insert(SynsetRelType::InstanceHyponym, SynsetRelType::InstanceHypernym);
//        //map.insert(SynsetRelType::Instrument, SynsetRelType::InvolvedInstrument);
//        //map.insert(SynsetRelType::Involved, SynsetRelType::Role);
//        //map.insert(SynsetRelType::InvolvedAgent, SynsetRelType::Agent);
//        //map.insert(SynsetRelType::InvolvedDirection, SynsetRelType::Direction);
//        //map.insert(SynsetRelType::InvolvedInstrument, SynsetRelType::Instrument);
//        //map.insert(SynsetRelType::InvolvedLocation, SynsetRelType::Location);
//        //map.insert(SynsetRelType::InvolvedPatient, SynsetRelType::Patient);
//        //map.insert(SynsetRelType::InvolvedResult, SynsetRelType::Result);
//        //map.insert(SynsetRelType::InvolvedSourceDirection, SynsetRelType::SourceDirection);
//        //map.insert(SynsetRelType::InvolvedTargetDirection, SynsetRelType::TargetDirection);
//        map.insert(SynsetRelType::IsCausedBy, SynsetRelType::Causes);
//        map.insert(SynsetRelType::IsEntailedBy, SynsetRelType::Entails);
//        map.insert(SynsetRelType::IsExemplifiedBy, SynsetRelType::Exemplifies);
//        //map.insert(SynsetRelType::IsSubeventOf, SynsetRelType::Subevent);
//        //map.insert(SynsetRelType::Location, SynsetRelType::InvolvedLocation);
//        //map.insert(SynsetRelType::MannerOf, SynsetRelType::InManner);
//        map.insert(SynsetRelType::MeroLocation, SynsetRelType::HoloLocation);
//        map.insert(SynsetRelType::MeroMember, SynsetRelType::HoloMember);
//        map.insert(SynsetRelType::MeroPart, SynsetRelType::HoloPart);
//        map.insert(SynsetRelType::MeroPortion, SynsetRelType::HoloPortion);
//        map.insert(SynsetRelType::MeroSubstance, SynsetRelType::HoloSubstance);
//        map.insert(SynsetRelType::Meronym, SynsetRelType::Holonym);
//        //map.insert(SynsetRelType::Patient, SynsetRelType::InvolvedPatient);
//        //map.insert(SynsetRelType::RestrictedBy, SynsetRelType::Restricts);
//        //map.insert(SynsetRelType::Restricts, SynsetRelType::RestrictedBy);
//        //map.insert(SynsetRelType::Result, SynsetRelType::InvolvedResult);
//        //map.insert(SynsetRelType::Role, SynsetRelType::Involved);
//        map.insert(SynsetRelType::Similar, SynsetRelType::Similar);
//        //map.insert(SynsetRelType::SourceDirection, SynsetRelType::InvolvedSourceDirection);
//        //map.insert(SynsetRelType::StateOf, SynsetRelType::BeInState);
//        //map.insert(SynsetRelType::Subevent, SynsetRelType::IsSubeventOf);
//        //map.insert(SynsetRelType::TargetDirection, SynsetRelType::InvolvedTargetDirection);
//        map
//    };
//}

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
    Agent,
    Material,
    Event,
    Instrument,
    Location,
    ByMeansOf,
    Undergoer,
    Property,
    Result,
    State,
    Uses,
    Destination,
    BodyPart,
    Vehicle,
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
            SenseRelType::Other => "other",
            SenseRelType::Agent => "agent",
            SenseRelType::Material => "material",
            SenseRelType::Event => "event",
            SenseRelType::Instrument => "instrument",
            SenseRelType::Location => "location",
            SenseRelType::ByMeansOf => "by_means_of",
            SenseRelType::Undergoer => "undergoer",
            SenseRelType::Property => "property",
            SenseRelType::Result => "result",
            SenseRelType::State => "state",
            SenseRelType::Uses => "uses",
            SenseRelType::Destination => "destination",
            SenseRelType::BodyPart => "body_part",
            SenseRelType::Vehicle => "vehicle"
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
            "agent" => Some(SenseRelType::Agent),
            "material" => Some(SenseRelType::Material),
            "event" => Some(SenseRelType::Event),
            "instrument" => Some(SenseRelType::Instrument),
            "location" => Some(SenseRelType::Location),
            "by_means_of" => Some(SenseRelType::ByMeansOf),
            "undergoer" => Some(SenseRelType::Undergoer),
            "property" => Some(SenseRelType::Property),
            "result" => Some(SenseRelType::Result),
            "state" => Some(SenseRelType::State),
            "uses" => Some(SenseRelType::Uses),
            "destination" => Some(SenseRelType::Destination),
            "body_part" => Some(SenseRelType::BodyPart),
            "vehicle" => Some(SenseRelType::Vehicle),
            _ => None
        }
    }
    
    pub fn is_symmetric(&self) -> bool {
        *self == SenseRelType::Antonym ||
            *self == SenseRelType::Similar ||
            *self == SenseRelType::Also ||
            *self == SenseRelType::Derivation
    }

    pub fn pos(&self) -> Vec<&'static PartOfSpeech> {
        match self {
            SenseRelType::Antonym => vec![&PartOfSpeech::n, &PartOfSpeech::v, &PartOfSpeech::a, &PartOfSpeech::r, &PartOfSpeech::s],
            SenseRelType::Also => vec![&PartOfSpeech::n, &PartOfSpeech::v, &PartOfSpeech::a, &PartOfSpeech::r, &PartOfSpeech::s],
            SenseRelType::Participle => vec![&PartOfSpeech::a, &PartOfSpeech::s],
            SenseRelType::Pertainym => vec![&PartOfSpeech::a, &PartOfSpeech::r, &PartOfSpeech::s],
            SenseRelType::Derivation => vec![&PartOfSpeech::n, &PartOfSpeech::v, &PartOfSpeech::a, &PartOfSpeech::r, &PartOfSpeech::s],
            SenseRelType::DomainTopic => vec![&PartOfSpeech::n, &PartOfSpeech::v, &PartOfSpeech::a, &PartOfSpeech::r, &PartOfSpeech::s],
            SenseRelType::HasDomainTopic => vec![&PartOfSpeech::n, &PartOfSpeech::v, &PartOfSpeech::a, &PartOfSpeech::r, &PartOfSpeech::s],
            SenseRelType::DomainRegion => vec![&PartOfSpeech::n, &PartOfSpeech::v, &PartOfSpeech::a, &PartOfSpeech::r, &PartOfSpeech::s],
            SenseRelType::HasDomainRegion => vec![&PartOfSpeech::n, &PartOfSpeech::v, &PartOfSpeech::a, &PartOfSpeech::r, &PartOfSpeech::s],
            SenseRelType::Exemplifies => vec![&PartOfSpeech::n, &PartOfSpeech::v, &PartOfSpeech::a, &PartOfSpeech::r, &PartOfSpeech::s],
            SenseRelType::IsExemplifiedBy => vec![&PartOfSpeech::n, &PartOfSpeech::v, &PartOfSpeech::a, &PartOfSpeech::r, &PartOfSpeech::s],
            SenseRelType::Similar => vec![&PartOfSpeech::v, &PartOfSpeech::a, &PartOfSpeech::s],
            SenseRelType::Other => vec![&PartOfSpeech::n, &PartOfSpeech::v, &PartOfSpeech::a, &PartOfSpeech::r, &PartOfSpeech::s],
            SenseRelType::Agent => vec![&PartOfSpeech::n, &PartOfSpeech::v],
            SenseRelType::Material => vec![&PartOfSpeech::n, &PartOfSpeech::v],
            SenseRelType::Event => vec![&PartOfSpeech::n, &PartOfSpeech::v],
            SenseRelType::Instrument => vec![&PartOfSpeech::n, &PartOfSpeech::v],
            SenseRelType::Location => vec![&PartOfSpeech::n, &PartOfSpeech::v],
            SenseRelType::ByMeansOf => vec![&PartOfSpeech::n, &PartOfSpeech::v],
            SenseRelType::Undergoer => vec![&PartOfSpeech::n, &PartOfSpeech::v],
            SenseRelType::Property => vec![&PartOfSpeech::n, &PartOfSpeech::v],
            SenseRelType::Result => vec![&PartOfSpeech::n, &PartOfSpeech::v],
            SenseRelType::State => vec![&PartOfSpeech::n, &PartOfSpeech::v],
            SenseRelType::Uses => vec![&PartOfSpeech::n, &PartOfSpeech::v],
            SenseRelType::Destination => vec![&PartOfSpeech::n, &PartOfSpeech::v],
            SenseRelType::BodyPart => vec![&PartOfSpeech::n, &PartOfSpeech::v],
            SenseRelType::Vehicle => vec![&PartOfSpeech::n, &PartOfSpeech::v],

        }
    }
}
