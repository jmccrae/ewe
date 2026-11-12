use serde::{Serialize,Deserialize};
use std::fmt;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone,Eq,Hash,PartialOrd,Ord)]
#[cfg_attr(feature="redb", derive(speedy::Readable, speedy::Writable))]
#[cfg_attr(feature="schema", derive(schemars::JsonSchema))]
pub struct PosKey(String);

impl PosKey {
    pub fn new(s : impl Into<String>) -> PosKey { PosKey(s.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
    pub fn to_part_of_speech(&self) -> Option<PartOfSpeech> {
        self.0.chars().next().and_then(PartOfSpeech::from_letter)
    }
    #[allow(dead_code)]
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

/// Part of speech, using the one-letter codes of WN-LMF 1.5 (globalwordnet/schemas#114).
///
/// `n v a r s` come first and new values must only ever be appended: the enum is stored by
/// discriminant in the redb database.
///
/// The deprecated `t` (phrase) is read as `z`.
#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Clone, Copy)]
#[cfg_attr(feature="redb", derive(speedy::Readable, speedy::Writable))]
pub enum PartOfSpeech {
    n, v, a, r, s,
    /// Conjunction (coordinating)
    c,
    /// Adposition
    p,
    /// Other
    x,
    /// Unknown
    u,
    /// Frozen phrase (`t` is the deprecated spelling)
    #[serde(alias = "t")]
    z,
    /// Number
    b,
    /// Determiner
    d,
    /// Proper noun (name)
    e,
    /// Auxiliary
    h,
    /// Pronoun
    i,
    /// Particle
    l,
    /// Punctuation
    m,
    /// Subordinating conjunction
    o,
    /// Symbol
    y,
}

/// Everything that is known about one part of speech. This is the only place the set of parts
/// of speech is listed, other code should use the methods on `PartOfSpeech`.
struct PosInfo {
    pos: PartOfSpeech,
    /// The one letter code
    code: &'static str,
    /// The name used in JSON and RDF
    long: &'static str,
    /// Human readable name
    label: &'static str,
    /// The heading for a list of things of this part of speech (shared by `a` and `s`)
    heading: &'static str,
    /// The prefix of the lexicographer files (`noun.person`) holding this part of speech.
    /// Where there is a Universal Dependencies UPOS tag, this is its lower case name
    /// (https://universaldependencies.org/u/pos/); the Princeton ones keep their traditional names.
    lexfile: &'static str,
    /// The `ss_type` used in sense keys (None if the part of speech has no sense keys)
    ss_type: Option<u32>,
    /// The name of the WNDB file (None if it is not part of the WNDB format)
    wndb_file: Option<&'static str>,
    /// The part of speech this one is interchangeable with in relations
    base: PartOfSpeech,
}

#[allow(clippy::too_many_arguments)]
const fn info(pos: PartOfSpeech, code: &'static str, long: &'static str, label: &'static str,
              heading: &'static str, lexfile: &'static str, ss_type: Option<u32>,
              wndb_file: Option<&'static str>, base: PartOfSpeech) -> PosInfo {
    PosInfo { pos, code, long, label, heading, lexfile, ss_type, wndb_file, base }
}

/// In the order of `PartOfSpeech::ALL` (checked by a test)
const POS_INFO: [PosInfo; 19] = [
    info(PartOfSpeech::n, "n", "noun", "noun", "Nouns", "noun", Some(1), Some("noun"), PartOfSpeech::n),
    info(PartOfSpeech::v, "v", "verb", "verb", "Verbs", "verb", Some(2), Some("verb"), PartOfSpeech::v),
    info(PartOfSpeech::a, "a", "adjective", "adjective", "Adjectives", "adj", Some(3), Some("adj"), PartOfSpeech::a),
    info(PartOfSpeech::r, "r", "adverb", "adverb", "Adverbs", "adv", Some(4), Some("adv"), PartOfSpeech::r),
    info(PartOfSpeech::s, "s", "adjective_satellite", "adjective satellite", "Adjectives", "adj", Some(5), Some("adj"), PartOfSpeech::a),
    info(PartOfSpeech::c, "c", "conjunction", "conjunction", "Conjunctions", "cconj", None, None, PartOfSpeech::c),
    info(PartOfSpeech::p, "p", "adposition", "adposition", "Adpositions", "adp", None, None, PartOfSpeech::p),
    info(PartOfSpeech::x, "x", "other", "other", "Other", "other", None, None, PartOfSpeech::x),
    info(PartOfSpeech::u, "u", "unknown", "unknown", "Unknown", "unknown", None, None, PartOfSpeech::u),
    info(PartOfSpeech::z, "z", "phrase", "phrase", "Phrases", "phrase", None, None, PartOfSpeech::z),
    info(PartOfSpeech::b, "b", "number", "number", "Numbers", "num", None, None, PartOfSpeech::b),
    info(PartOfSpeech::d, "d", "determiner", "determiner", "Determiners", "det", None, None, PartOfSpeech::d),
    info(PartOfSpeech::e, "e", "proper_noun", "name", "Names", "name", Some(1), None, PartOfSpeech::n),
    info(PartOfSpeech::h, "h", "auxiliary", "auxiliary", "Auxiliaries", "aux", None, None, PartOfSpeech::h),
    info(PartOfSpeech::i, "i", "pronoun", "pronoun", "Pronouns", "pron", None, None, PartOfSpeech::i),
    info(PartOfSpeech::l, "l", "particle", "particle", "Particles", "part", None, None, PartOfSpeech::l),
    info(PartOfSpeech::m, "m", "punctuation", "punctuation", "Punctuation", "punct", None, None, PartOfSpeech::m),
    info(PartOfSpeech::o, "o", "subordinating_conjunction", "subordinating conjunction", "Subordinating conjunctions", "sconj", None, None, PartOfSpeech::o),
    info(PartOfSpeech::y, "y", "symbol", "symbol", "Symbols", "sym", None, None, PartOfSpeech::y),
];

impl PartOfSpeech {
    /// Every part of speech
    pub const ALL: [PartOfSpeech; 19] = [
        PartOfSpeech::n, PartOfSpeech::v, PartOfSpeech::a, PartOfSpeech::r, PartOfSpeech::s,
        PartOfSpeech::c, PartOfSpeech::p, PartOfSpeech::x, PartOfSpeech::u, PartOfSpeech::z,
        PartOfSpeech::b, PartOfSpeech::d, PartOfSpeech::e, PartOfSpeech::h, PartOfSpeech::i,
        PartOfSpeech::l, PartOfSpeech::m, PartOfSpeech::o, PartOfSpeech::y,
    ];

    /// Position in `ALL` and `POS_INFO`
    fn pos_idx(&self) -> usize {
        match self {
            PartOfSpeech::n => 0, PartOfSpeech::v => 1, PartOfSpeech::a => 2, PartOfSpeech::r => 3,
            PartOfSpeech::s => 4, PartOfSpeech::c => 5, PartOfSpeech::p => 6, PartOfSpeech::x => 7,
            PartOfSpeech::u => 8, PartOfSpeech::z => 9, PartOfSpeech::b => 10, PartOfSpeech::d => 11,
            PartOfSpeech::e => 12, PartOfSpeech::h => 13, PartOfSpeech::i => 14, PartOfSpeech::l => 15,
            PartOfSpeech::m => 16, PartOfSpeech::o => 17, PartOfSpeech::y => 18,
        }
    }

    fn info(&self) -> &'static PosInfo {
        &POS_INFO[self.pos_idx()]
    }

    /// The one letter code
    pub fn value(&self) -> &'static str {
        self.info().code
    }

    /// Parse a one letter code. The deprecated `t` is read as `z`.
    pub fn from_letter(c: char) -> Option<PartOfSpeech> {
        if c == 't' {
            return Some(PartOfSpeech::z);
        }
        POS_INFO.iter().find(|i| i.code.starts_with(c)).map(|i| i.pos)
    }

    /// Parse a one letter code
    pub fn from_code(s: &str) -> Option<PartOfSpeech> {
        let mut cs = s.chars();
        match (cs.next(), cs.next()) {
            (Some(c), None) => PartOfSpeech::from_letter(c),
            _ => None,
        }
    }

    pub fn to_pos_key(&self) -> PosKey {
        PosKey::new(self.value().to_string())
    }

    /// Whether this and `pos2` are interchangeable (adjectives and satellites)
    pub fn equals_pos(&self, pos2 : &PartOfSpeech) -> bool {
        self.info().base == pos2.info().base && (self.info().base == PartOfSpeech::a || self == pos2)
    }

    /// The part of speech this behaves like in relations: satellites are adjectives and names
    /// are nouns.
    pub fn base(&self) -> PartOfSpeech {
        self.info().base
    }

    /// Whether an entry with part of speech `entry` may be a member of a synset with part of
    /// speech `synset`: the same part of speech, a satellite synset for an adjective entry,
    /// or a noun synset for a name entry.
    pub fn entry_accepts_synset(entry: PartOfSpeech, synset: PartOfSpeech) -> bool {
        entry == synset
            || (entry == PartOfSpeech::a && synset == PartOfSpeech::s)
            || (entry == PartOfSpeech::e && synset == PartOfSpeech::n)
    }

    /// The `ss_type` of sense keys, if this part of speech has sense keys
    pub fn ss_type(&self) -> Option<u32> {
        self.info().ss_type
    }

    /// The name of the WNDB file, if this part of speech is part of WNDB
    pub fn wndb_file(&self) -> Option<&'static str> {
        self.info().wndb_file
    }

    /// The name used in JSON and RDF
    pub fn long_pos(&self) -> &'static str {
        self.info().long
    }

    /// A human readable name
    pub fn label(&self) -> &'static str {
        self.info().label
    }

    /// Whether this part of speech is new in WN-LMF 1.5 (`z` replaces the deprecated `t`)
    pub fn is_new_in_lmf_1_5(&self) -> bool {
        use PartOfSpeech::*;
        matches!(self, b | d | e | h | i | l | m | o | y | z)
    }

    /// The closest code WN-LMF 1.4 has: names are nouns, phrases are `t`, the rest `x` (other)
    pub fn lmf_1_4_code(&self) -> &'static str {
        match self {
            PartOfSpeech::e => "n",
            PartOfSpeech::z => "t",
            _ if self.is_new_in_lmf_1_5() => "x",
            _ => self.value(),
        }
    }

    /// The parts of speech a lexicographer file may contain, judged by its prefix
    /// (`noun.person` holds nouns, `adj.all` adjectives and satellites, ...)
    pub fn for_lexfile(lexfile: &str) -> Vec<PartOfSpeech> {
        let Some((prefix, _)) = lexfile.split_once('.') else {
            return Vec::new();
        };
        POS_INFO.iter().filter(|i| i.lexfile == prefix).map(|i| i.pos).collect()
    }

    /// The name of the lexicographer file new synsets of this part of speech go to by default
    pub fn default_lexname(&self) -> String {
        format!("{}.all", self.info().lexfile)
    }

    /// The heading for a list of things of this part of speech ("Nouns", "Names", ...)
    pub fn heading(&self) -> &'static str {
        self.info().heading
    }
}

impl fmt::Display for PartOfSpeech {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_letters_round_trip() {
        for (i, pos) in PartOfSpeech::ALL.iter().enumerate() {
            let pos = *pos;
            assert_eq!(pos.pos_idx(), i);
            assert_eq!(POS_INFO[i].pos, pos);
            assert_eq!(PartOfSpeech::from_code(pos.value()), Some(pos));
            assert_eq!(pos.to_pos_key().to_part_of_speech(), Some(pos));
        }
        assert_eq!(PartOfSpeech::from_letter('q'), None);
        assert_eq!(PartOfSpeech::from_code("nv"), None);
        assert_eq!(PosKey::new("e2").to_part_of_speech(), Some(PartOfSpeech::e));
        assert_eq!(PosKey::new("").to_part_of_speech(), None);
    }

    #[test]
    fn test_deprecated_t_is_phrase() {
        assert_eq!(PartOfSpeech::from_letter('t'), Some(PartOfSpeech::z));
        let pos: PartOfSpeech = serde_yaml::from_str("t").unwrap();
        assert_eq!(pos, PartOfSpeech::z);
        assert_eq!(serde_yaml::to_string(&PartOfSpeech::z).unwrap().trim(), "z");
    }

    #[test]
    fn test_base_and_equivalence() {
        assert_eq!(PartOfSpeech::s.base(), PartOfSpeech::a);
        assert_eq!(PartOfSpeech::e.base(), PartOfSpeech::n);
        assert!(PartOfSpeech::a.equals_pos(&PartOfSpeech::s));
        assert!(PartOfSpeech::s.equals_pos(&PartOfSpeech::a));
        assert!(!PartOfSpeech::e.equals_pos(&PartOfSpeech::n));
        assert!(PartOfSpeech::c.equals_pos(&PartOfSpeech::c));
        assert!(!PartOfSpeech::c.equals_pos(&PartOfSpeech::p));
    }

    #[test]
    fn test_entry_accepts_synset() {
        use PartOfSpeech::*;
        assert!(PartOfSpeech::entry_accepts_synset(a, s));
        assert!(PartOfSpeech::entry_accepts_synset(e, n));
        assert!(PartOfSpeech::entry_accepts_synset(e, e));
        assert!(!PartOfSpeech::entry_accepts_synset(n, e));
        assert!(!PartOfSpeech::entry_accepts_synset(s, a));
        assert!(!PartOfSpeech::entry_accepts_synset(e, v));
    }

    #[test]
    fn test_sense_key_types() {
        assert_eq!(PartOfSpeech::n.ss_type(), Some(1));
        assert_eq!(PartOfSpeech::s.ss_type(), Some(5));
        assert_eq!(PartOfSpeech::e.ss_type(), Some(1));
        assert_eq!(PartOfSpeech::d.ss_type(), None);
        assert_eq!(PartOfSpeech::e.wndb_file(), None);
        assert_eq!(PartOfSpeech::a.wndb_file(), Some("adj"));
        assert_eq!(PartOfSpeech::s.wndb_file(), Some("adj"));
    }

    #[test]
    fn test_lmf_1_4_codes() {
        assert_eq!(PartOfSpeech::e.lmf_1_4_code(), "n");
        assert_eq!(PartOfSpeech::z.lmf_1_4_code(), "t");
        assert_eq!(PartOfSpeech::d.lmf_1_4_code(), "x");
        assert_eq!(PartOfSpeech::c.lmf_1_4_code(), "c");
        assert_eq!(PartOfSpeech::s.lmf_1_4_code(), "s");
        assert!(PartOfSpeech::ALL.iter().all(|pos| pos.is_new_in_lmf_1_5() == (pos.lmf_1_4_code() != pos.value())));
    }

    #[test]
    fn test_for_lexfile() {
        assert_eq!(PartOfSpeech::for_lexfile("noun.person"), vec![PartOfSpeech::n]);
        assert_eq!(PartOfSpeech::for_lexfile("verb.body"), vec![PartOfSpeech::v]);
        assert_eq!(PartOfSpeech::for_lexfile("adj.all"), vec![PartOfSpeech::a, PartOfSpeech::s]);
        assert_eq!(PartOfSpeech::for_lexfile("adv.manner"), vec![PartOfSpeech::r]);
        assert_eq!(PartOfSpeech::for_lexfile("name.location"), vec![PartOfSpeech::e]);
        assert_eq!(PartOfSpeech::for_lexfile("det.all"), vec![PartOfSpeech::d]);
        assert_eq!(PartOfSpeech::for_lexfile("sconj.all"), vec![PartOfSpeech::o]);
        assert!(PartOfSpeech::for_lexfile("contrib.colloq").is_empty());
        assert!(PartOfSpeech::for_lexfile("noun").is_empty());
        assert!(PartOfSpeech::for_lexfile("nounish").is_empty());
    }

    #[test]
    fn test_every_pos_has_its_own_lexfile_and_heading() {
        // Every part of speech can be put in a default lexfile that maps back to it
        for pos in PartOfSpeech::ALL {
            assert!(PartOfSpeech::for_lexfile(&pos.default_lexname()).contains(&pos), "{pos}");
            assert!(!pos.heading().is_empty());
        }
        assert_eq!(PartOfSpeech::d.default_lexname(), "det.all");
        assert_eq!(PartOfSpeech::s.heading(), PartOfSpeech::a.heading());
        // the other parts of speech are not dumped into one lexfile
        let lexnames: std::collections::HashSet<_> =
            PartOfSpeech::ALL.iter().filter(|pos| **pos != PartOfSpeech::s).map(|pos| pos.default_lexname()).collect();
        assert_eq!(lexnames.len(), PartOfSpeech::ALL.len() - 1);
    }
}
