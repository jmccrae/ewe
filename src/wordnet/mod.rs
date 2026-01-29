pub mod lexicon;
pub use lexicon::Lexicon;

pub mod entry;
pub use entry::{Entry,Entries};

pub mod pronunciation;
pub use pronunciation::Pronunciation;

pub mod sense;
pub use sense::{Sense,SenseId};

pub mod synset;
pub use synset::{Synset,Synsets,SynsetId, ILIID,BTSynsets};

pub mod example;
pub use example::Example;

pub mod pos;
pub use pos::{PartOfSpeech, PosKey};

mod util;
pub use util::{WordNetYAMLIOError, string_or_vec};

pub mod deprecation;
pub use deprecation::DeprecationRecord;

pub mod hashmap;
pub use hashmap::LexiconHashMapBackend;

#[cfg(test)]
#[allow(unused_variables)]
mod tests {
    use super::*;

    #[test]
    fn test_entry() {
        let entry_str = "sense:
- id: 'foo%1:01:00::'
  synset: 00001740-n
";
        assert_eq!(serde_yaml::from_str::<Entry>(&entry_str).unwrap(),
            Entry {
                sense: vec![Sense::new(
                    SenseId("foo%1:01:00::".to_string()),
                    SynsetId("00001740-n".to_string())
                )],
                form: Vec::new(),
                pronunciation: Vec::new()
            });
    }

    #[test]
    fn test_save_entry() {
        let entry_str = "    sense:
    - id: 'foo%1:01:00::'
      synset: 00001740-n
";
        let mut gen_str : Vec<u8> = Vec::new();

        Entry {
            sense: vec![Sense::new(
                SenseId("foo%1:01:00::".to_string()),
                SynsetId("00001740-n".to_string())
            )],
            form: Vec::new(),
            pronunciation: Vec::new()
        }.save(&mut gen_str).unwrap();
        assert_eq!(entry_str, String::from_utf8(gen_str).unwrap());
    }
 

    #[test]
    fn test_entries() {
        let entry_str = "abate:
  v:
    sense:
    - derivation:
      - abatable%5:00:00:stoppable:00
      - 'abator%1:18:00::'
      id: 'abate%2:30:01::'
      subcat:
      - vtai
      - vtii
      synset: 00246175-v
    - derivation:
      - 'abatement%1:11:01::'
      id: 'abate%2:30:00::'
      subcat:
      - vii
      synset: 00245945-v
abatement:
  n:
    sense:
    - derivation:
      - 'abate%2:30:00::'
      id: 'abatement%1:11:01::'
      synset: 07382856-n
    - id: 'abatement%1:04:00::'
      synset: 00362159-n";
        let e : Entries = serde_yaml::from_str(&entry_str).unwrap();
    }

    #[test]
    fn test_synset() {
        let synset_str = "definition:
- part of a meal served at one time
example:
- '\"she prepared a three course meal\"'
hypernym:
- 07586285-n
ili: i76474
members:
- course
partOfSpeech: n";
        let s : Synset = serde_yaml::from_str(&synset_str).unwrap();
    }

    #[test]
    fn test_save_synset() {
        let synset_str = "
  definition:
  - part of a meal served at one time
  example:
  - '\"she prepared a three course meal\"'
  hypernym:
  - 07586285-n
  ili: i76474
  members:
  - course
  partOfSpeech: n";
        let mut ss = Synset::new(PartOfSpeech::n);
        ss.definition.push("part of a meal served at one time".to_owned());
        ss.example.push(Example::new(
            "\"she prepared a three course meal\"".to_owned(), None));
        ss.hypernym.push(SynsetId::new("07586285-n"));
        ss.ili = Some(ILIID::new("i76474"));
        ss.members.push("course".to_owned());
        let mut gen_str : Vec<u8> = Vec::new();
        ss.save(&mut gen_str).unwrap();
        assert_eq!(synset_str, String::from_utf8(gen_str).unwrap());
    }

    #[test]
    fn test_split_line() {
        let string = "especially of muscles; drawing away from the midline of the body or from an adjacent part";
        assert_eq!("especially of muscles; drawing away from the midline of the body or from an adjacent\n    part", escape_yaml_string(string, 4, 4));
    }


    #[test]
    fn test_split_line2() {
        let string = "(usually followed by `to') having the necessary means or skill or know-how or authority to do something";
        assert_eq!("(usually followed by `to') having the necessary means or skill or know-how or\n    authority to do something", escape_yaml_string(string, 4, 4));
    }

    #[test]
    fn test_split_line3() {
        let string = "\"the abaxial surface of a leaf is the underside or side facing away from the stem\"";
        assert_eq!("'\"the abaxial surface of a leaf is the underside or side facing away from the\n    stem\"'", escape_yaml_string(string, 4, 4));
    }

//    #[test]
//    fn test_split_line4() {
//        let string = "Canned cream of mushroom soup has been described as \"America's béchamel\"";
//        assert_eq!("\"Canned cream of mushroom soup has been described as \\\"America's b\\xE9chamel\\\n\\\"", escape_yaml_string(string, 6, 6));
//    }
//
//    #[test]
//    fn test_split_line5() {
//        let string = "If you consider a point on a radius of the rolling curve in generating a cardioid that is not on its circumference, the result is a conchoid called the limaçon of Pascal.";
//        assert_eq!("\"If you consider a point on a radius of the rolling curve in generating a cardioid\\\n    \\ that is not on its circumference, the result is a conchoid called the lima\\xE7\\\n    on of Pascal.\"", escape_yaml_string(string, 4, 4));
//    }

    #[test]
    fn test_entry_deriv() {
        let entry_str = "    sense:
    - derivation:
      - 'foo%1:01:00::'
      id: 'foo%1:01:00::'
      synset: 00001740-n
";
        let mut gen_str : Vec<u8> = Vec::new();
        let mut sense = Sense::new(
                SenseId("foo%1:01:00::".to_string()),
                SynsetId("00001740-n".to_string())
            );
        sense.derivation.push(SenseId::new("foo%1:01:00::".to_owned()));

        Entry {
            sense: vec![sense],
            form: Vec::new(),
            pronunciation: Vec::new()
        }.save(&mut gen_str).unwrap();
        assert_eq!(entry_str, String::from_utf8(gen_str).unwrap());
    }

    #[test]
    fn test_line_align() {
        let input = "00001740-a:
  attribute:
  - 05207437-n
  - 05624029-n
  definition:
  - (usually followed by ‘to’) having the necessary means or skill or know-how
    or authority to do something
  example:
  - able to swim
  - she was able to program her computer
  - we were at last able to buy a car
  - able to get a grant for the project
  ili: i1
  members:
  - able
  partOfSpeech: a";
        let output = "00001740-a:
  attribute:
  - 05207437-n
  - 05624029-n
  definition:
  - (usually followed by ‘to’) having the necessary means or skill or know-how or
    authority to do something
  example:
  - able to swim
  - she was able to program her computer
  - we were at last able to buy a car
  - able to get a grant for the project
  ili: i1
  members:
  - able
  partOfSpeech: a
";
        let synsets = serde_yaml::from_str::<Synsets>(input).unwrap();
        let mut buf = Vec::new();
        synsets.save(&mut buf).unwrap();
        assert_eq!(output, String::from_utf8(buf).unwrap());
    }

    #[test]
    fn test_string_or_vec() {
        let input = "00001740-n:
  wikidata: Q1
  definition:
  - foobar
  members:
  - foo
  partOfSpeech: n
00001741-a:
  wikidata:
  - Q2
  - Q3
  definition:
  - foobar
  members:
  - foo
  partOfSpeech: a
00001742-a:
  definition:
  - foobar
  members:
  - foo
  partOfSpeech: a";
    let synsets = serde_yaml::from_str::<Synsets>(input).unwrap();
    synsets.0.iter().for_each(|(key, ss)| {
        if key.as_str() == "00001740-n" {
            assert_eq!(ss.wikidata.len(), 1);
            assert_eq!(ss.wikidata[0], "Q1");
        } else if key.as_str() == "00001741-a" {
            assert_eq!(ss.wikidata.len(), 2);
            assert_eq!(ss.wikidata[0], "Q2");
            assert_eq!(ss.wikidata[1], "Q3");
        } else if key.as_str() == "00001742-a" {
            assert_eq!(ss.wikidata.len(), 0);
        }
    });
    }

//    #[test]
//    fn test_unicode_convert() {
//        assert_eq!("\"f\\xF6o\"",escape_yaml_string("föo", 0, 0));
//        assert_eq!("\"\\\"f\\xF6o\\\"\"",escape_yaml_string("\"föo\"", 0, 0));
//    }

//    #[test]
//    fn test_load() {
//        Lexicon::load("/home/jmccrae/projects/globalwordnet/english-wordnet/src/yaml/").unwrap();
//    }
}
