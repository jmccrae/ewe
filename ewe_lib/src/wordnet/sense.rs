use crate::rels::SenseRelType;
use crate::wordnet::util::{escape_yaml_string, write_prop_sense};
use crate::wordnet::*;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::io::Write;
use std::result;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "redb", derive(speedy::Readable, speedy::Writable))]
pub struct Sense {
    pub id: SenseId,
    pub synset: SynsetId,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adjposition: Option<String>,
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
    pub domain_topic: Vec<UnresolvedSenseOrSynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub domain_region: Vec<UnresolvedSenseOrSynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exemplifies: Vec<UnresolvedSenseOrSynsetId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub similar: Vec<SenseId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub other: Vec<UnresolvedSenseOrSynsetId>,
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
    sent: Vec<String>,
}

impl Sense {
    pub fn new(id: SenseId, synset: SynsetId) -> Sense {
        Sense {
            id,
            synset,
            subcat: Vec::new(),
            antonym: Vec::new(),
            also: Vec::new(),
            participle: Vec::new(),
            pertainym: Vec::new(),
            derivation: Vec::new(),
            domain_topic: Vec::new(),
            domain_region: Vec::new(),
            exemplifies: Vec::new(),
            similar: Vec::new(),
            other: Vec::new(),
            agent: Vec::new(),
            material: Vec::new(),
            event: Vec::new(),
            instrument: Vec::new(),
            location: Vec::new(),
            by_means_of: Vec::new(),
            undergoer: Vec::new(),
            property: Vec::new(),
            result: Vec::new(),
            state: Vec::new(),
            uses: Vec::new(),
            destination: Vec::new(),
            body_part: Vec::new(),
            vehicle: Vec::new(),
            adjposition: None,
            sent: Vec::new(),
        }
    }

    pub fn remove_rel(&mut self, target: &SenseOrSynsetId) {
        if let SenseOrSynsetId::Sense(target) = target {
            self.antonym.retain(|x| x != target);
            self.also.retain(|x| x != target);
            self.participle.retain(|x| x != target);
            self.pertainym.retain(|x| x != target);
            self.derivation.retain(|x| x != target);
            self.similar.retain(|x| x != target);
            self.agent.retain(|x| x != target);
            self.material.retain(|x| x != target);
            self.event.retain(|x| x != target);
            self.instrument.retain(|x| x != target);
            self.location.retain(|x| x != target);
            self.by_means_of.retain(|x| x != target);
            self.undergoer.retain(|x| x != target);
            self.property.retain(|x| x != target);
            self.result.retain(|x| x != target);
            self.state.retain(|x| x != target);
            self.uses.retain(|x| x != target);
            self.destination.retain(|x| x != target);
            self.body_part.retain(|x| x != target);
            self.vehicle.retain(|x| x != target);
        }
        let target = UnresolvedSenseOrSynsetId::from(target.clone());
        self.domain_topic.retain(|x| *x != target);
        self.domain_region.retain(|x| *x != target);
        self.exemplifies.retain(|x| *x != target);
        self.other.retain(|x| *x != target);
    }

    pub(crate) fn save<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        write!(w, "\n    - ")?;
        let mut first = true;
        match self.adjposition {
            Some(ref adjposition) => {
                write!(w, "adjposition: {}", adjposition)?;
                first = false
            }
            None => {}
        };
        first = write_prop_sense(w, &self.agent, "agent", first)?;
        first = write_prop_sense(w, &self.also, "also", first)?;
        first = write_prop_sense(w, &self.antonym, "antonym", first)?;
        first = write_prop_sense(w, &self.body_part, "body_part", first)?;
        first = write_prop_sense(w, &self.by_means_of, "by_means_of", first)?;
        first = write_prop_sense(w, &self.derivation, "derivation", first)?;
        first = write_prop_sense(w, &self.destination, "destination", first)?;
        first = write_prop_sense(w, &self.domain_region, "domain_region", first)?;
        first = write_prop_sense(w, &self.domain_topic, "domain_topic", first)?;
        first = write_prop_sense(w, &self.event, "event", first)?;
        first = write_prop_sense(w, &self.exemplifies, "exemplifies", first)?;
        if first {
            write!(w, "id: {}", escape_yaml_string(self.id.as_str(), 8, 8))?;
            first = false;
        } else {
            write!(
                w,
                "\n      id: {}",
                escape_yaml_string(self.id.as_str(), 8, 8)
            )?;
        }
        write_prop_sense(w, &self.instrument, "instrument", first)?;
        write_prop_sense(w, &self.location, "location", first)?;
        write_prop_sense(w, &self.material, "material", first)?;
        write_prop_sense(w, &self.other, "other", first)?;
        write_prop_sense(w, &self.participle, "participle", first)?;
        write_prop_sense(w, &self.pertainym, "pertainym", first)?;
        write_prop_sense(w, &self.property, "property", first)?;
        write_prop_sense(w, &self.result, "result", first)?;
        if !self.sent.is_empty() {
            write!(w, "\n      sent:")?;
            for subcat_id in self.sent.iter() {
                write!(w, "\n      - {}", subcat_id)?;
            }
        }
        write_prop_sense(w, &self.similar, "similar", first)?;
        write_prop_sense(w, &self.state, "state", first)?;
        if !self.subcat.is_empty() {
            write!(w, "\n      subcat:")?;
            for subcat_id in self.subcat.iter() {
                write!(w, "\n      - {}", subcat_id)?;
            }
        }
        write!(w, "\n      synset: {}", self.synset.as_str())?;
        write_prop_sense(w, &self.undergoer, "undergoer", first)?;
        write_prop_sense(w, &self.uses, "uses", first)?;
        write_prop_sense(w, &self.vehicle, "vehicle", first)?;

        Ok(())
    }

    pub fn sense_links_from(&self) -> Vec<(SenseRelType, UnresolvedSenseOrSynsetId)> {
        self.antonym.iter().map(|id| (SenseRelType::Antonym, UnresolvedSenseOrSynsetId::Sense(id.clone()))).chain(
        self.also.iter().map(|id| (SenseRelType::Also, UnresolvedSenseOrSynsetId::Sense(id.clone()))).chain(
        self.participle.iter().map(|id| (SenseRelType::Participle, UnresolvedSenseOrSynsetId::Sense(id.clone()))).chain(
        self.pertainym.iter().map(|id| (SenseRelType::Pertainym, UnresolvedSenseOrSynsetId::Sense(id.clone()))).chain(
        self.derivation.iter().map(|id| (SenseRelType::Derivation, UnresolvedSenseOrSynsetId::Sense(id.clone()))).chain(
        self.domain_topic.iter().map(|id| (SenseRelType::DomainTopic, id.clone())).chain(
        self.domain_region.iter().map(|id| (SenseRelType::DomainRegion, id.clone())).chain(
        self.exemplifies.iter().map(|id| (SenseRelType::Exemplifies, id.clone())).chain(
        self.similar.iter().map(|id| (SenseRelType::Similar, UnresolvedSenseOrSynsetId::Sense(id.clone()))).chain(
        self.other.iter().map(|id| (SenseRelType::Other, id.clone())).chain(
        self.agent.iter().map(|id| (SenseRelType::Agent, UnresolvedSenseOrSynsetId::Sense(id.clone()))).chain(
        self.material.iter().map(|id| (SenseRelType::Material, UnresolvedSenseOrSynsetId::Sense(id.clone()))).chain(
        self.event.iter().map(|id| (SenseRelType::Event, UnresolvedSenseOrSynsetId::Sense(id.clone()))).chain(
        self.instrument.iter().map(|id| (SenseRelType::Instrument, UnresolvedSenseOrSynsetId::Sense(id.clone()))).chain(
        self.location.iter().map(|id| (SenseRelType::Location, UnresolvedSenseOrSynsetId::Sense(id.clone()))).chain(
        self.by_means_of.iter().map(|id| (SenseRelType::ByMeansOf, UnresolvedSenseOrSynsetId::Sense(id.clone()))).chain(
        self.undergoer.iter().map(|id| (SenseRelType::Undergoer, UnresolvedSenseOrSynsetId::Sense(id.clone()))).chain(
        self.property.iter().map(|id| (SenseRelType::Property, UnresolvedSenseOrSynsetId::Sense(id.clone()))).chain(
        self.result.iter().map(|id| (SenseRelType::Result, UnresolvedSenseOrSynsetId::Sense(id.clone()))).chain(
        self.state.iter().map(|id| (SenseRelType::State, UnresolvedSenseOrSynsetId::Sense(id.clone()))).chain(
        self.uses.iter().map(|id| (SenseRelType::Uses, UnresolvedSenseOrSynsetId::Sense(id.clone()))).chain(
        self.destination.iter().map(|id| (SenseRelType::Destination, UnresolvedSenseOrSynsetId::Sense(id.clone()))).chain(
        self.body_part.iter().map(|id| (SenseRelType::BodyPart, UnresolvedSenseOrSynsetId::Sense(id.clone()))).chain(
        self.vehicle.iter().map(|id| (SenseRelType::Vehicle, UnresolvedSenseOrSynsetId::Sense(id.clone())))
                                                    ))))))))))))))))))))))).collect()
    }

    /// `target` is the resolved type: by the time anything calls this, the
    /// caller (CLI/`change_manager`/automaton) has already validated the
    /// target against the lexicon. For relation types that only ever
    /// support a sense target, a `Synset` target is a caller bug and is
    /// silently ignored (mirrors the existing no-op treatment of the
    /// not-directly-storable `Has*`/`Is*` arms below).
    pub(crate) fn add_rel(&mut self, rel: SenseRelType, target: SenseOrSynsetId) {
        match rel {
            SenseRelType::Antonym => {
                if let SenseOrSynsetId::Sense(target) = target {
                    if !self.antonym.iter().any(|x| *x == target) {
                        self.antonym.push(target)
                    }
                }
            }
            SenseRelType::Also => {
                if let SenseOrSynsetId::Sense(target) = target {
                    if !self.also.iter().any(|x| *x == target) {
                        self.also.push(target)
                    }
                }
            }
            SenseRelType::Participle => {
                if let SenseOrSynsetId::Sense(target) = target {
                    if !self.participle.iter().any(|x| *x == target) {
                        self.participle.push(target)
                    }
                }
            }
            SenseRelType::Pertainym => {
                if let SenseOrSynsetId::Sense(target) = target {
                    if !self.pertainym.iter().any(|x| *x == target) {
                        self.pertainym.push(target)
                    }
                }
            }
            SenseRelType::Derivation => {
                if let SenseOrSynsetId::Sense(target) = target {
                    if !self.derivation.iter().any(|x| *x == target) {
                        self.derivation.push(target)
                    }
                }
            }
            SenseRelType::DomainTopic => {
                let target = UnresolvedSenseOrSynsetId::from(target);
                if !self.domain_topic.iter().any(|x| *x == target) {
                    self.domain_topic.push(target)
                }
            }
            // Not directly storable - the OEWN format only persists the canonical
            // direction; add_sense_rel in lexicon.rs swaps these to their canonical
            // form (DomainTopic/DomainRegion/Exemplifies) before this is ever reached.
            SenseRelType::HasDomainTopic => {}
            SenseRelType::DomainRegion => {
                let target = UnresolvedSenseOrSynsetId::from(target);
                if !self.domain_region.iter().any(|x| *x == target) {
                    self.domain_region.push(target)
                }
            }
            SenseRelType::HasDomainRegion => {}
            SenseRelType::Exemplifies => {
                let target = UnresolvedSenseOrSynsetId::from(target);
                if !self.exemplifies.iter().any(|x| *x == target) {
                    self.exemplifies.push(target)
                }
            }
            SenseRelType::IsExemplifiedBy => {}
            SenseRelType::Similar => {
                if let SenseOrSynsetId::Sense(target) = target {
                    if !self.similar.iter().any(|x| *x == target) {
                        self.similar.push(target)
                    }
                }
            }
            SenseRelType::Agent => {
                if let SenseOrSynsetId::Sense(target) = target {
                    if !self.agent.iter().any(|x| *x == target) {
                        self.agent.push(target)
                    }
                }
            }
            SenseRelType::Material => {
                if let SenseOrSynsetId::Sense(target) = target {
                    if !self.material.iter().any(|x| *x == target) {
                        self.material.push(target)
                    }
                }
            }
            SenseRelType::Event => {
                if let SenseOrSynsetId::Sense(target) = target {
                    if !self.event.iter().any(|x| *x == target) {
                        self.event.push(target)
                    }
                }
            }
            SenseRelType::Instrument => {
                if let SenseOrSynsetId::Sense(target) = target {
                    if !self.instrument.iter().any(|x| *x == target) {
                        self.instrument.push(target)
                    }
                }
            }
            SenseRelType::Location => {
                if let SenseOrSynsetId::Sense(target) = target {
                    if !self.location.iter().any(|x| *x == target) {
                        self.location.push(target)
                    }
                }
            }
            SenseRelType::ByMeansOf => {
                if let SenseOrSynsetId::Sense(target) = target {
                    if !self.by_means_of.iter().any(|x| *x == target) {
                        self.by_means_of.push(target)
                    }
                }
            }
            SenseRelType::Undergoer => {
                if let SenseOrSynsetId::Sense(target) = target {
                    if !self.undergoer.iter().any(|x| *x == target) {
                        self.undergoer.push(target)
                    }
                }
            }
            SenseRelType::Property => {
                if let SenseOrSynsetId::Sense(target) = target {
                    if !self.property.iter().any(|x| *x == target) {
                        self.property.push(target)
                    }
                }
            }
            SenseRelType::Result => {
                if let SenseOrSynsetId::Sense(target) = target {
                    if !self.result.iter().any(|x| *x == target) {
                        self.result.push(target)
                    }
                }
            }
            SenseRelType::State => {
                if let SenseOrSynsetId::Sense(target) = target {
                    if !self.state.iter().any(|x| *x == target) {
                        self.state.push(target)
                    }
                }
            }
            SenseRelType::Uses => {
                if let SenseOrSynsetId::Sense(target) = target {
                    if !self.uses.iter().any(|x| *x == target) {
                        self.uses.push(target)
                    }
                }
            }
            SenseRelType::Destination => {
                if let SenseOrSynsetId::Sense(target) = target {
                    if !self.destination.iter().any(|x| *x == target) {
                        self.destination.push(target)
                    }
                }
            }
            SenseRelType::BodyPart => {
                if let SenseOrSynsetId::Sense(target) = target {
                    if !self.body_part.iter().any(|x| *x == target) {
                        self.body_part.push(target)
                    }
                }
            }
            SenseRelType::Vehicle => {
                if let SenseOrSynsetId::Sense(target) = target {
                    if !self.vehicle.iter().any(|x| *x == target) {
                        self.vehicle.push(target)
                    }
                }
            }

            SenseRelType::Other => {
                let target = UnresolvedSenseOrSynsetId::from(target);
                if !self.other.iter().any(|x| *x == target) {
                    self.other.push(target)
                }
            }
        };
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Eq, Hash)]
#[cfg_attr(feature = "redb", derive(speedy::Readable, speedy::Writable))]
pub struct SenseId(String);

impl SenseId {
    pub fn new(s: impl Into<String>) -> SenseId {
        SenseId(s.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SenseId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for SenseId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// A sense-synset relation target confirmed to be either a real sense or a
/// real synset. Use this everywhere a target has already been (or is about
/// to be) validated against the lexicon - which in practice is almost
/// everywhere. See [`UnresolvedSenseOrSynsetId`] for the one place that
/// can't make that guarantee: reading a target id from a file.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
#[cfg_attr(feature = "redb", derive(speedy::Readable, speedy::Writable))]
pub enum SenseOrSynsetId {
    Sense(SenseId),
    Synset(SynsetId),
}

impl SenseOrSynsetId {
    pub fn as_str(&self) -> &str {
        match self {
            SenseOrSynsetId::Sense(id) => id.as_str(),
            SenseOrSynsetId::Synset(id) => id.as_str(),
        }
    }
}

impl AsRef<str> for SenseOrSynsetId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for SenseOrSynsetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Serialize for SenseOrSynsetId {
    fn serialize<S>(&self, serializer: S) -> result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

/// A sense-synset relation target as read from a file: possibly not yet
/// checked against the lexicon, and (for a genuinely broken reference)
/// possibly never resolvable at all. `Deserialize` always produces
/// `Unresolved` - it has no lexicon access to classify the raw string.
/// This is the type stored on [`Sense`]'s `domain_topic`/`domain_region`/
/// `exemplifies`/`other` fields, since a WordNet mid-edit can legitimately
/// contain a dangling relation target that still needs to round-trip and
/// be reported by validation, not silently dropped. `Lexicon::load()`
/// resolves every occurrence of this in-place right after loading (see
/// `resolve()` below); application code downstream of that should not
/// normally observe the `Unresolved` variant.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
#[cfg_attr(feature = "redb", derive(speedy::Readable, speedy::Writable))]
pub enum UnresolvedSenseOrSynsetId {
    Sense(SenseId),
    Synset(SynsetId),
    Unresolved(String),
}

impl UnresolvedSenseOrSynsetId {
    pub fn as_str(&self) -> &str {
        match self {
            UnresolvedSenseOrSynsetId::Sense(id) => id.as_str(),
            UnresolvedSenseOrSynsetId::Synset(id) => id.as_str(),
            UnresolvedSenseOrSynsetId::Unresolved(s) => s.as_str(),
        }
    }

    /// Classify this target against the lexicon. Idempotent (and free) for
    /// an already-resolved value; does the actual `has_sense`/`synset_by_id`
    /// lookup only for `Unresolved`.
    pub fn resolve<L: crate::wordnet::Lexicon>(
        &self,
        wn: &L,
    ) -> crate::wordnet::Result<SenseOrSynsetId> {
        let raw = match self {
            UnresolvedSenseOrSynsetId::Sense(id) => return Ok(SenseOrSynsetId::Sense(id.clone())),
            UnresolvedSenseOrSynsetId::Synset(id) => {
                return Ok(SenseOrSynsetId::Synset(id.clone()))
            }
            UnresolvedSenseOrSynsetId::Unresolved(s) => s,
        };
        if wn.has_sense(&SenseId::new(raw.clone()))? {
            Ok(SenseOrSynsetId::Sense(SenseId::new(raw.clone())))
        } else if wn.synset_by_id(&SynsetId::new(raw))?.is_some() {
            Ok(SenseOrSynsetId::Synset(SynsetId::new(raw)))
        } else {
            Err(LexiconError::SenseOrSynsetIdNotFound(raw.clone()))
        }
    }
}

impl From<SenseOrSynsetId> for UnresolvedSenseOrSynsetId {
    fn from(id: SenseOrSynsetId) -> Self {
        match id {
            SenseOrSynsetId::Sense(id) => UnresolvedSenseOrSynsetId::Sense(id),
            SenseOrSynsetId::Synset(id) => UnresolvedSenseOrSynsetId::Synset(id),
        }
    }
}

impl AsRef<str> for UnresolvedSenseOrSynsetId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for UnresolvedSenseOrSynsetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Serialize for UnresolvedSenseOrSynsetId {
    fn serialize<S>(&self, serializer: S) -> result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for UnresolvedSenseOrSynsetId {
    fn deserialize<D>(deserializer: D) -> result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(UnresolvedSenseOrSynsetId::Unresolved(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::change_manager;
    use crate::change_manager::ChangeList;

    #[test]
    fn test_unresolved_resolve_classifies_by_lexicon_not_format() {
        let mut lexicon = LexiconHashMapBackend::new();
        let mut change_list = ChangeList::new();
        lexicon.add_lexfile("noun.animal").unwrap();
        let ssid = change_manager::add_synset(
            &mut lexicon,
            "def".to_string(),
            "noun.animal".to_string(),
            PosKey::new("n".to_string()),
            None,
            &mut change_list,
        )
        .expect("Could not create synset");
        change_manager::add_entry(
            &mut lexicon,
            ssid.clone(),
            "foo".to_owned(),
            PosKey::new("n".to_string()),
            Vec::new(),
            None,
            &mut change_list,
        )
        .unwrap();
        let foo_sense = lexicon.get_sense_id2("foo", &ssid).unwrap().unwrap();

        // Resolves to a Sense when the raw string names a known sense id.
        let unresolved = UnresolvedSenseOrSynsetId::Unresolved(foo_sense.as_str().to_string());
        assert_eq!(
            unresolved.resolve(&lexicon).unwrap(),
            SenseOrSynsetId::Sense(foo_sense.clone())
        );

        // Resolves to a Synset when the raw string names a known synset id.
        let unresolved = UnresolvedSenseOrSynsetId::Unresolved(ssid.as_str().to_string());
        assert_eq!(
            unresolved.resolve(&lexicon).unwrap(),
            SenseOrSynsetId::Synset(ssid.clone())
        );

        // Neither - a dangling/broken reference - is an error.
        let unresolved = UnresolvedSenseOrSynsetId::Unresolved("no-such-id".to_string());
        assert!(unresolved.resolve(&lexicon).is_err());

        // Already-resolved values are returned as-is, with no lookup.
        let already = UnresolvedSenseOrSynsetId::Sense(SenseId::new("bogus%1:08:00::"));
        assert_eq!(
            already.resolve(&lexicon).unwrap(),
            SenseOrSynsetId::Sense(SenseId::new("bogus%1:08:00::"))
        );
    }
}
