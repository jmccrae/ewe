use serde::{Serialize,Deserialize,Serializer,Deserializer};
use std::fmt;
use std::io::Write;
use serde::de::{self, Visitor, MapAccess};
use crate::serde::ser::SerializeMap;
use crate::wordnet::util::{escape_yaml_string, format_confidence};


#[derive(Debug, PartialEq,Clone)]
#[cfg_attr(feature="redb", derive(speedy::Readable, speedy::Writable))]
pub struct Example {
    pub text : String,
    pub source : Option<String>,
    pub confidence : Option<f64>
}

impl Example {
    pub fn new(text : String, source : Option<String>) -> Example {
        Example {
            text: text, source, confidence: None
        }
    }

    pub(crate) fn save<W : Write>(&self, w : &mut W) -> std::io::Result<()> {
        write!(w, "\n  - ")?;
        if self.source.is_none() && self.confidence.is_none() {
            write!(w, "{}", escape_yaml_string(&self.text, 4, 4))?;
            return Ok(());
        }
        // Map form, keys in alphabetical order like every other YAML writer here.
        if let Some(c) = self.confidence {
            write!(w, "confidence: {}\n    ", format_confidence(c))?;
        }
        if let Some(s) = &self.source {
            write!(w, "source: {}\n    ", escape_yaml_string(s, 6, 10))?;
        }
        write!(w, "text: {}", escape_yaml_string(&self.text, 6, 10))?;
        Ok(())
    }
}

impl Serialize for Example {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.source.is_none() && self.confidence.is_none() {
            return serializer.serialize_str(&self.text);
        }
        let n = 1 + self.source.is_some() as usize + self.confidence.is_some() as usize;
        let mut map = serializer.serialize_map(Some(n))?;
        if let Some(c) = self.confidence {
            map.serialize_entry("confidence", &c)?;
        }
        if let Some(ref s) = self.source {
            map.serialize_entry("source", s)?;
        }
        map.serialize_entry("text", &self.text)?;
        map.end()
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
        Ok(Example::new(value.to_string(), None))
    }

    fn visit_map<M>(self, mut map: M) -> Result<Example, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut text = None;
        let mut source = None;
        let mut confidence = None;
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "text" => text = Some(map.next_value::<String>()?),
                "source" => source = Some(map.next_value::<String>()?),
                "confidence" => confidence = Some(map.next_value::<f64>()?),
                _ => return Err(de::Error::unknown_field(&key, &["text", "source", "confidence"])),
            }
        }
        let text = text.ok_or_else(|| de::Error::missing_field("text"))?;
        Ok(Example { text, source, confidence })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example_forms_round_trip() {
        for yaml in ["plain text\n", "source: S\ntext: T\n", "confidence: 0.25\ntext: T\n",
                     "confidence: 0.25\nsource: S\ntext: T\n"] {
            let ex: Example = serde_yaml::from_str(yaml).unwrap();
            assert_eq!(serde_yaml::to_string(&ex).unwrap(), yaml);
        }
        let ex: Example = serde_yaml::from_str("text: T\nconfidence: 0.5\n").unwrap();
        assert_eq!(ex.confidence, Some(0.5));
        assert!(serde_yaml::from_str::<Example>("source: S\n").is_err());
        assert!(serde_yaml::from_str::<Example>("text: T\nbogus: x\n").is_err());
    }

    #[test]
    fn test_example_save() {
        let mut out = Vec::new();
        let mut ex = Example::new("T".to_string(), Some("S".to_string()));
        ex.confidence = Some(0.5);
        ex.save(&mut out).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "\n  - confidence: 0.5\n    source: S\n    text: T");
    }
}
