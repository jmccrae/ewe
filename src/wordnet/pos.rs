use serde::{Serialize,Deserialize};
use std::fmt;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone,Eq,Hash,PartialOrd,Ord)]
pub struct PosKey(String);

impl PosKey {
    pub fn new(s : String) -> PosKey { PosKey(s) }
    pub fn as_str(&self) -> &str { &self.0 }
    pub fn to_part_of_speech(&self) -> Option<PartOfSpeech> {
        if self.0.starts_with("n") {
            Some(PartOfSpeech::n)
        } else if self.0.starts_with("v") {
            Some(PartOfSpeech::v)
        } else if self.0.starts_with("a") {
            Some(PartOfSpeech::a)
        } else if self.0.starts_with("r") {
            Some(PartOfSpeech::r)
        } else if self.0.starts_with("s") {
            Some(PartOfSpeech::s)
        } else {
            None
        }
    }
    pub fn entry_no(&self) -> Option<u32> {
        if self.0.len() < 3 {
            None
        } else {
            self.0[2..].parse().ok()
        }
    }
    
}

impl fmt::Display for PosKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, Serialize, Deserialize,Clone)]
pub enum PartOfSpeech { n, v, a, r, s }

impl PartOfSpeech {
    pub fn value(&self) -> &'static str {
        match self {
            PartOfSpeech::n => "n",
            PartOfSpeech::v => "v",
            PartOfSpeech::a => "a",
            PartOfSpeech::r => "r",
            PartOfSpeech::s => "s"
        }
    }

    pub fn to_pos_key(&self) -> PosKey {
        PosKey::new(self.value().to_string())
    }

    //pub fn equals_pos(&self, s : &str) -> bool {
    //    match self {
    //        PartOfSpeech::n => s.starts_with("n"),
    //        PartOfSpeech::v => s.starts_with("v"),
    //        PartOfSpeech::a => s.starts_with("a") || s.starts_with("s"),
    //        PartOfSpeech::r => s.starts_with("r"),
    //        PartOfSpeech::s => s.starts_with("a") || s.starts_with("s")
    //    }
    //}

    //pub fn equals_str(&self, s : &str) -> bool {
    //    match self {
    //        PartOfSpeech::n => s.starts_with("n"),
    //        PartOfSpeech::v => s.starts_with("v"),
    //        PartOfSpeech::a => s.starts_with("a"),
    //        PartOfSpeech::r => s.starts_with("r"),
    //        PartOfSpeech::s => s.starts_with("s")
    //    }
    //}

    pub fn equals_pos(&self, pos2 : &PartOfSpeech) -> bool {
        match self {
            PartOfSpeech::a => pos2 == &PartOfSpeech::a || pos2 == &PartOfSpeech::s,
            PartOfSpeech::s => pos2 == &PartOfSpeech::a || pos2 == &PartOfSpeech::s,
            _ => self == pos2
        }
    }

    pub fn ss_type(&self) -> u32 {
        match self {
            PartOfSpeech::n => 1,
            PartOfSpeech::v => 2,
            PartOfSpeech::a => 3,
            PartOfSpeech::r => 4,
            PartOfSpeech::s => 5
        }
    }
}


