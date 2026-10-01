//! `ScoredVec` - a list (of relation targets, or of definitions) where each item may carry a
//! WN-LMF `confidenceScore`.
//!
//! Derefs to the plain `Vec<T>`, so code that only cares about the items is unaffected. Scores
//! are kept to one side, keyed by the item's *string form* rather than by `T` itself: that way
//! a score survives `Lexicon::load` resolving an `UnresolvedSenseOrSynsetId::Unresolved("x")`
//! in place into a `Sense("x")`/`Synset("x")`, and a `retain`/`push` through `DerefMut` can
//! never attach a score to the wrong item. A score whose item has gone (removed through
//! `DerefMut`) is simply ignored by every reader, and dropped on serialization. Only an actual
//! change of id (e.g. a sense key update) needs [`ScoredVec::rename`] to carry the score along.
//!
//! On disk (YAML/JSON) each item is either the bare value, exactly as before, or - only when it
//! has a score - a map such as `{target: 00001740-n, confidence: 0.7}` (relations) or
//! `{text: "a definition", confidence: 0.7}` (definitions), see [`ScoredItem::MAP_KEY`].

use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::ser::SerializeMap;
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use std::fmt;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

/// An item type that can live in a [`ScoredVec`].
pub trait ScoredItem: AsRef<str> + Sized {
    /// The key naming the item itself in the scored (map) form.
    const MAP_KEY: &'static str;
    fn from_string(s: String) -> Self;
}

impl ScoredItem for String {
    const MAP_KEY: &'static str = "text";
    fn from_string(s: String) -> Self {
        s
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "redb", derive(speedy::Readable, speedy::Writable))]
pub struct ScoredVec<T> {
    items: Vec<T>,
    scores: BTreeMap<String, f64>,
}

impl<T> ScoredVec<T> {
    pub const fn new() -> ScoredVec<T> {
        ScoredVec {
            items: Vec::new(),
            scores: BTreeMap::new(),
        }
    }

    pub fn into_vec(self) -> Vec<T> {
        self.items
    }
}

impl<T: AsRef<str>> ScoredVec<T> {
    /// The confidence of `item`, if it is in this list and has one.
    pub fn confidence(&self, item: &str) -> Option<f64> {
        if self.items.iter().any(|x| x.as_ref() == item) {
            self.scores.get(item).copied()
        } else {
            None
        }
    }

    /// Set (or, with `None`, clear) the confidence of `item`. Returns false if `item` is not in
    /// this list, in which case nothing changes.
    pub fn set_confidence(&mut self, item: &str, confidence: Option<f64>) -> bool {
        if !self.items.iter().any(|x| x.as_ref() == item) {
            return false;
        }
        match confidence {
            Some(c) => {
                self.scores.insert(item.to_string(), c);
            }
            None => {
                self.scores.remove(item);
            }
        }
        true
    }

    /// Push `item` with an optional confidence.
    pub fn push_scored(&mut self, item: T, confidence: Option<f64>) {
        let key = item.as_ref().to_string();
        self.items.push(item);
        match confidence {
            Some(c) => {
                self.scores.insert(key, c);
            }
            None => {
                self.scores.remove(&key);
            }
        }
    }

    /// Replace every occurrence of `old` by `new` (as produced by `f`), carrying its score along.
    pub fn rename(&mut self, old: &str, mut new: impl FnMut() -> T) {
        let score = self.scores.remove(old);
        let mut new_key = None;
        for x in self.items.iter_mut() {
            if x.as_ref() == old {
                *x = new();
                new_key = Some(x.as_ref().to_string());
            }
        }
        if let (Some(score), Some(new_key)) = (score, new_key) {
            self.scores.insert(new_key, score);
        }
    }

    /// Every item paired with its confidence, in list order.
    pub fn iter_scored(&self) -> impl Iterator<Item = (&T, Option<f64>)> {
        self.items
            .iter()
            .map(move |x| (x, self.scores.get(x.as_ref()).copied()))
    }

    /// Every confidence actually attached to a current item.
    pub fn scores(&self) -> impl Iterator<Item = (&T, f64)> {
        self.iter_scored().filter_map(|(x, c)| c.map(|c| (x, c)))
    }
}

impl<T> Default for ScoredVec<T> {
    fn default() -> Self {
        ScoredVec::new()
    }
}

impl<T> Deref for ScoredVec<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Vec<T> {
        &self.items
    }
}

impl<T> DerefMut for ScoredVec<T> {
    fn deref_mut(&mut self) -> &mut Vec<T> {
        &mut self.items
    }
}

impl<T> From<Vec<T>> for ScoredVec<T> {
    fn from(items: Vec<T>) -> Self {
        ScoredVec {
            items,
            scores: BTreeMap::new(),
        }
    }
}

impl<T> FromIterator<T> for ScoredVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        ScoredVec::from(iter.into_iter().collect::<Vec<T>>())
    }
}

impl<T> IntoIterator for ScoredVec<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a ScoredVec<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

/// Equal when the items are equal and every item has the same score - orphaned scores (left
/// behind by a `DerefMut` removal) don't count.
impl<T: AsRef<str> + PartialEq> PartialEq for ScoredVec<T> {
    fn eq(&self, other: &Self) -> bool {
        self.items == other.items
            && self
                .items
                .iter()
                .all(|x| self.scores.get(x.as_ref()) == other.scores.get(x.as_ref()))
    }
}

impl<T: AsRef<str> + PartialEq> PartialEq<Vec<T>> for ScoredVec<T> {
    fn eq(&self, other: &Vec<T>) -> bool {
        self.items == *other
    }
}

struct ScoredItemRef<'a, T>(&'a T, Option<f64>);

impl<'a, T: ScoredItem + Serialize> Serialize for ScoredItemRef<'a, T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.1 {
            None => self.0.serialize(serializer),
            Some(c) => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry(T::MAP_KEY, self.0)?;
                map.serialize_entry("confidence", &c)?;
                map.end()
            }
        }
    }
}

impl<T: ScoredItem + Serialize> Serialize for ScoredVec<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.items.len()))?;
        for (x, c) in self.iter_scored() {
            seq.serialize_element(&ScoredItemRef(x, c))?;
        }
        seq.end()
    }
}

/// One deserialized item: the bare value or the `{<MAP_KEY>: ..., confidence: ...}` map.
struct ScoredItemOwned<T>(T, Option<f64>);

impl<'de, T: ScoredItem> Deserialize<'de> for ScoredItemOwned<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(ScoredItemVisitor(PhantomData))
    }
}

struct ScoredItemVisitor<T>(PhantomData<T>);

impl<'de, T: ScoredItem> Visitor<'de> for ScoredItemVisitor<T> {
    type Value = ScoredItemOwned<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "a string or a map with `{}` and `confidence` keys",
            T::MAP_KEY
        )
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        Ok(ScoredItemOwned(T::from_string(value.to_string()), None))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
        Ok(ScoredItemOwned(T::from_string(value), None))
    }

    // A bare YAML scalar that happens to look like a number/bool is still meant as text,
    // exactly as a plain `Vec<String>` would have read it.
    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
        self.visit_string(value.to_string())
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
        self.visit_string(value.to_string())
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
        self.visit_string(value.to_string())
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
        self.visit_string(value.to_string())
    }

    // An empty YAML item (`- `, e.g. the "no definition yet" empty definition) is a null,
    // which a plain `Vec<String>` read as "".
    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        self.visit_string(String::new())
    }

    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        self.visit_string(String::new())
    }

    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
        let mut item = None;
        let mut confidence = None;
        while let Some(key) = map.next_key::<String>()? {
            if key == T::MAP_KEY {
                item = Some(map.next_value::<String>()?);
            } else if key == "confidence" {
                confidence = Some(map.next_value::<f64>()?);
            } else {
                return Err(de::Error::unknown_field(&key, &[T::MAP_KEY, "confidence"]));
            }
        }
        let item = item.ok_or_else(|| de::Error::missing_field(T::MAP_KEY))?;
        Ok(ScoredItemOwned(T::from_string(item), confidence))
    }
}

impl<'de, T: ScoredItem> Deserialize<'de> for ScoredVec<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct SeqVisitor<T>(PhantomData<T>);

        impl<'de, T: ScoredItem> Visitor<'de> for SeqVisitor<T> {
            type Value = ScoredVec<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a list")
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut out = ScoredVec::new();
                while let Some(ScoredItemOwned(x, c)) = seq.next_element::<ScoredItemOwned<T>>()? {
                    out.push_scored(x, c);
                }
                Ok(out)
            }
        }

        deserializer.deserialize_seq(SeqVisitor(PhantomData))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wordnet::SynsetId;

    #[test]
    fn test_bare_and_scored_items_round_trip() {
        let yaml = "- 00001740-n\n- target: 00001741-n\n  confidence: 0.7\n";
        let v: ScoredVec<SynsetId> = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v.confidence("00001740-n"), None);
        assert_eq!(v.confidence("00001741-n"), Some(0.7));
        assert_eq!(serde_yaml::to_string(&v).unwrap(), yaml);
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(serde_json::from_str::<ScoredVec<SynsetId>>(&json).unwrap(), v);
    }

    #[test]
    fn test_definition_map_key_is_text() {
        let yaml = "- text: a definition\n  confidence: 0.5\n- '1984'\n";
        let v: ScoredVec<String> = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(*v, vec!["a definition".to_string(), "1984".to_string()]);
        assert_eq!(v.confidence("a definition"), Some(0.5));
    }

    #[test]
    fn test_unknown_key_is_an_error() {
        let yaml = "- target: 00001741-n\n  confidance: 0.7\n";
        assert!(serde_yaml::from_str::<ScoredVec<SynsetId>>(yaml).is_err());
    }

    #[test]
    fn test_retain_drops_score_and_rename_carries_it() {
        let mut v = ScoredVec::new();
        v.push_scored(SynsetId::new("a"), Some(0.3));
        v.push_scored(SynsetId::new("b"), Some(0.4));
        v.retain(|x| x.as_str() != "a");
        assert_eq!(v.confidence("a"), None);
        // Re-adding a removed item doesn't resurrect its old score.
        v.push_scored(SynsetId::new("a"), None);
        assert_eq!(v.confidence("a"), None);
        v.rename("b", || SynsetId::new("c"));
        assert_eq!(v.confidence("b"), None);
        assert_eq!(v.confidence("c"), Some(0.4));
        assert!(!v.set_confidence("zzz", Some(0.1)));
    }
}
