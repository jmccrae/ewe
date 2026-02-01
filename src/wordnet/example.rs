use serde::{Serialize,Deserialize,Serializer,Deserializer};
use std::fmt;
use std::io::Write;
use serde::de::{self, Visitor, MapAccess};
use crate::serde::ser::SerializeMap;
use crate::wordnet::util::escape_yaml_string;


#[derive(Debug, PartialEq,Clone)]
#[cfg_attr(feature="redb", derive(speedy::Readable, speedy::Writable))]pub struct Example {
    pub text : String,
    pub source : Option<String>
}

impl Example {
    pub fn new(text : String, source : Option<String>) -> Example {
        Example {
            text: text, source 
        }
    }

    pub(crate) fn save<W : Write>(&self, w : &mut W) -> std::io::Result<()> {
        write!(w, "\n  - ")?;
        match &self.source {
            Some(s) => {
                write!(w, "source: {}\n    text: {}", 
                       escape_yaml_string(s, 6, 10),
                       escape_yaml_string(&self.text, 6, 10))?;
            },
            None => {
                write!(w, "{}", escape_yaml_string(&self.text, 4, 4))?;
            }
        }
        Ok(())
    }
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


