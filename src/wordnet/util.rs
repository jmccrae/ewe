use thiserror::Error;
use serde::Deserializer;
use std::fmt;
use std::io::Write;
use serde::de::{self, Visitor};
use lazy_static::lazy_static;
use regex::Regex;
use crate::wordnet::*;
use std::result;

static YAML_LINE_LENGTH : usize = 80;
lazy_static! {
    static ref NUMBERS: Regex = Regex::new("^(\\.)?\\d+$").unwrap();
}

pub fn escape_yaml_string(s : &str, indent : usize, initial_indent : usize) -> String {

    let s2 : String = if s.starts_with("\"") || s.ends_with(":")  || s.contains(": ")
        || s.starts_with("'") || s == "true" || s == "false" 
        || s == "yes" || s == "no" || s == "null" || NUMBERS.is_match(s) 
        || s.ends_with(" ") || s.contains(": ")
        || s == "No" || s == "off" || s == "on" 
        || s.starts_with("`") || s.starts_with("...") {
        format!("'{}'", str::replace(s, "'", "''"))
    } else {
        s.to_owned()
    }; 
    if s2.chars().count() + indent > YAML_LINE_LENGTH {
        let mut s3 = String::new();
        let mut size = initial_indent;
        for s4 in s2.split(" ") {
            if size > indent && s3.chars().count() > 0 {
                s3.push_str(" ");
                size += 1;
            }
            // Very odd rule in the Python line splitting algorithm
            if s2.starts_with("\"") &&
                s4.chars().count() + size > YAML_LINE_LENGTH &&
                (s4.contains("\\x") || s4.contains("\\u")
                 || s4.contains("\\\"")) {
                let mut indices : Vec<usize> =s4.find("\\x").iter().chain(
                    s4.find("\\u").iter().chain(
                        s4.find("\\\"").iter())).map(|x| *x).collect();
                indices.sort();
                let mut s5 = s4;
                for i in indices {
                    let (s6, s7) = s5.split_at(i);
                    let n = if s7.starts_with("\\u") {
                        6
                    } else if s7.starts_with("\\x") {
                        4
                    } else /*s7.starts_with("\\\"")*/ {
                        2
                    };
                    let s6_len = s6.chars().count();
                    if s6_len + n + size > YAML_LINE_LENGTH {
                        s3.push_str(s6);
                        if n == 2 {
                            s3.push_str("\n\\");
                            s3.push_str(&s7[0..n]);
                            for _ in 0..indent {
                                s3.push_str(" ");
                            }
                            size = indent;
                            s5 = &s7[n..];
                        } else {
                            s3.push_str(&s7[0..n]);
                            s3.push_str("\\\n");
                            for _ in 0..indent {
                                s3.push_str(" ");
                            }
                            size = indent;
                            s5 = &s7[n..];
                        }
                    } else {
                        s3.push_str(s6);
                        size += s6.chars().count();
                        s5 = s7;
                    }
                }
                s3.push_str(s5);
                size += s5.chars().count();
            } else {
                s3.push_str(&s4);
                size += s4.chars().count();
                if size > YAML_LINE_LENGTH {
                    if s3.starts_with("\"") {
                        s3.push_str("\\\n");
                        for _ in 0..indent {
                            s3.push_str(" ");
                        }
                        s3.push_str("\\");
                        size = indent + 1;
                    } else {
                        s3.push_str("\n");
                        for _ in 0..indent {
                            s3.push_str(" ");
                        }
                        size = indent;
                    }
                }
            }
        } 
        if size == indent {
            s3.truncate(s3.len() - indent - 1);
        }
        if size == indent + 1 && s3.starts_with("\"") {
            s3.truncate(s3.len() - indent - 3);
        }
        s3
    } else {
        s2
    }
}


pub fn write_prop_sense<W : Write>(w : &mut W, senses : &Vec<SenseId>, name : &str, first : bool) -> std::io::Result<bool> {
    if senses.is_empty() {
        Ok(first)
    } else if !first {
        write!(w, "\n      {}:", name)?; 
        for sense_id in senses.iter() {
            write!(w, "\n      - {}", escape_yaml_string(sense_id.as_str(), 8, 8))?;
        }
        Ok(false)
    } else {
        write!(w, "{}:", name)?; 
        for sense_id in senses.iter() {
            write!(w, "\n      - {}", escape_yaml_string(sense_id.as_str(), 8, 8))?;
        }
        Ok(false)
    }
}


#[derive(Error,Debug)]
pub enum WordNetYAMLIOError {
    #[error("Could not load WordNet: {0}")]
    Io(String),
    #[error("Could not load WordNet: {0}")]
    Serde(String),
    #[error("Could not load WordNet: {0}")]
    Csv(String),
    #[error("Could not load WordNet: {0}")]
    Lexicon(#[from] LexiconError)
}

#[derive(Error,Debug)]
pub enum LexiconError {
    #[error("Synset Identifier not found: {0}")]
    SynsetIdNotFound(SynsetId),
    #[error("Sense Identifier not found: {0}")]
    SenseIdNotFound(SenseId),
    #[error("No such entry: ({0}, {1})")]
    EntryNotFound(String, PosKey),
    #[cfg(feature="redb")]
    #[error("DB error: {0}")]
    DBError(redb::StorageError)
}

#[derive(Error,Debug)]
pub enum LexiconSaveError {
    #[error("Could not save WordNet: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not save WordNet: {0}")]
    Lexicon(#[from] LexiconError)
}

/// Deserialize a string or a vector of strings
pub fn string_or_vec<'de, D>(deserializer: D) -> result::Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct StringOrVec;

    impl<'de> Visitor<'de> for StringOrVec {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or a list of strings")
        }

        fn visit_str<E>(self, value: &str) -> result::Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(vec![value.to_string()])
        }

        fn visit_string<E>(self, value: String) -> result::Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(vec![value])
        }

        fn visit_seq<A>(self, mut seq: A) -> result::Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            let mut vec = Vec::new();
            while let Some(value) = seq.next_element::<String>()? {
                vec.push(value);
            }
            Ok(vec)
        }
    }

    deserializer.deserialize_any(StringOrVec)
}


