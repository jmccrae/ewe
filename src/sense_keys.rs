use lazy_static::lazy_static;
use std::collections::HashMap;
use crate::wordnet_yaml::{Lexicon, Synset, SynsetId, Entry, Sense, SenseId, PartOfSpeech};
use regex::Regex;
use std::cmp::max;

lazy_static! {
    static ref LEX_FILENUMS : HashMap<&'static str, usize> = {
        let mut map = HashMap::new();
            map.insert("adj.all", 0);
            map.insert("adj.pert", 1);
            map.insert("adv.all", 2);
            map.insert("noun.Tops", 3);
            map.insert("noun.act", 4);
            map.insert("noun.animal", 5);
            map.insert("noun.artifact", 6);
            map.insert("noun.attribute", 7);
            map.insert("noun.body", 8);
            map.insert("noun.cognition", 9);
            map.insert("noun.communication", 10);
            map.insert("noun.event", 11);
            map.insert("noun.feeling", 12);
            map.insert("noun.food", 13);
            map.insert("noun.group", 14);
            map.insert("noun.location", 15);
            map.insert("noun.motive", 16);
            map.insert("noun.object", 17);
            map.insert("noun.person", 18);
            map.insert("noun.phenomenon", 19);
            map.insert("noun.plant", 20);
            map.insert("noun.possession", 21);
            map.insert("noun.process", 22);
            map.insert("noun.quantity", 23);
            map.insert("noun.relation", 24);
            map.insert("noun.shape", 25);
            map.insert("noun.state", 26);
            map.insert("noun.substance", 27);
            map.insert("noun.time", 28);
            map.insert("verb.body", 29);
            map.insert("verb.change", 30);
            map.insert("verb.cognition", 31);
            map.insert("verb.communication", 32);
            map.insert("verb.competition", 33);
            map.insert("verb.consumption", 34);
            map.insert("verb.contact", 35);
            map.insert("verb.creation", 36);
            map.insert("verb.emotion", 37);
            map.insert("verb.motion", 38);
            map.insert("verb.perception", 39);
            map.insert("verb.possession", 40);
            map.insert("verb.social", 41);
            map.insert("verb.stative", 42);
            map.insert("verb.weather", 43);
            map.insert("adj.ppl", 44);
            map.insert("contrib.colloq", 50);
            map.insert("contrib.plwn", 51);
            map
    };
}

lazy_static! {
    static ref SENSE_ID_LEX_ID : Regex = Regex::new("^.*%\\d:\\d\\d:(\\d\\d):.*$").unwrap();
}

fn gen_lex_id(e : &Entry) -> i32 {
    let mut max_id = 0;
    for s2 in e.sense.iter() {
        for m in SENSE_ID_LEX_ID.captures_iter(s2.id.as_str()) {
            match m[1].parse() {
                Ok(id2) => {
                    max_id = max(max_id, id2);
                },
                Err(_) => {
                    eprintln!("cannot parse sense id {}", s2.id.as_str());
                }
            }
        }
    }
    max_id + 1
}
        
fn extract_lex_id(sense_key : &SenseId) -> i32 {
    for m in SENSE_ID_LEX_ID.captures_iter(sense_key.as_str()) {
        match m[1].parse() {
            Ok(id2) => { return id2; }
            Err(_) => {}
        }
    }
    0
}

fn sense_for_entry_synset_id<'a>(wn : &'a Lexicon, ss_id : &SynsetId, lemma : &str) -> Vec<&'a Sense> {
    wn.entry_by_lemma(lemma).iter().flat_map(|e| e.sense.iter()).
        filter(|sense| sense.synset == *ss_id).
        collect()
}

fn get_head_word(wn : &Lexicon, ss : &Synset) -> (String, String) {
    // The hack here is we don't care about satellites in non-Princeton sets
    let mut srs : Vec<&SynsetId> = ss.similar.iter().filter(|target_id| 
        !target_id.as_str().starts_with("8") &&
        !target_id.as_str().starts_with("9")).collect();
        
    if srs.len() != 1 {
        ("???".to_string(), "00".to_string())
    } else {
        let tss = srs.pop().unwrap();
        match wn.members_by_id(tss).iter().flat_map(|m| {
            sense_for_entry_synset_id(wn, tss,  m) 
        }).next() {
            Some(s2) => {
                let (entry_id, _) = s2.id.as_str().split_at(
                    s2.id.as_str().find("%").unwrap_or(0));
                (entry_id.to_string(), 
                 format!("{:02}", extract_lex_id(&s2.id)))
            },
            None => {
                ("???".to_string(), "00".to_string())
            }
        }
    }
}

pub fn get_sense_key2(wn : &Lexicon, lemma : &str, sense_key : Option<&SenseId>,
                      synset_id : &SynsetId) -> Option<SenseId> {
    match wn.synset_by_id(synset_id) {
        Some(synset) => {
            for entry in wn.entry_by_lemma(lemma) {
                if entry.sense.iter().any(|sense| sense.synset == *synset_id) {
                    return Some(get_sense_key(wn, lemma, entry, sense_key,
                                              synset, synset_id));
                }
            }
        },
        None => {}
    }
    None
}

/// Calculate the sense key of an entry
/// Pass `None` for `sense_key` for new senses
pub fn get_sense_key(wn : &Lexicon, lemma : &str,
                 entry : &Entry, sense_key : Option<&SenseId>,
                 synset : &Synset, synset_id : &SynsetId) -> SenseId {
    let lemma = lemma.replace(" ", "_").replace("&apos", "'").to_lowercase();
    let ss_type = synset.part_of_speech.ss_type();
    let lex_filenum = wn.lex_name_for(synset_id).and_then(|lex_name|
            LEX_FILENUMS.get(lex_name.as_str()))
        .map(|x| *x)
        .unwrap_or(99);
    let lex_id = match sense_key {
        Some(sense_key) => extract_lex_id(sense_key),
        None => gen_lex_id(entry)
    };
    let (head_word, head_id) = if synset.part_of_speech == PartOfSpeech::s {
        get_head_word(wn, synset)
    } else {
        (String::new(), String::new())
    };
    SenseId::new_owned(format!("{}%{}:{:02}:{:02}:{}:{}",
            lemma, ss_type, lex_filenum,
            lex_id, head_word, head_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wordnet_yaml::PosKey;

    #[test]
    fn test_sense_key_1() {
        let mut lexicon = Lexicon::new();
        let synset = Synset::new(PartOfSpeech::n);
        let entry = Entry::new();
        lexicon.insert_entry("foot".to_string(),
            PosKey::new("n".to_string()), entry.clone());
        lexicon.insert_synset("noun.body".to_string(),
            SynsetId::new("00001740-n"), synset.clone());
        assert_eq!(SenseId::new("foot%1:08:01::"),
                   get_sense_key(&lexicon, "foot", &entry, None, &synset,
                                 &SynsetId::new("00001740-n")));
    }

    #[test]
    fn test_sense_key_2() {
        let mut lexicon = Lexicon::new();
        let synset = Synset::new(PartOfSpeech::n);
        let mut entry = Entry::new();
        entry.sense.push(Sense::new(
                SenseId::new("foot%1:08:01::"),
                SynsetId::new("00001750-n")
                ));
        lexicon.insert_entry("foot".to_string(),
            PosKey::new("n".to_string()), entry.clone());
        lexicon.insert_synset("noun.body".to_string(),
            SynsetId::new("00001740-n"), synset.clone());
        assert_eq!(SenseId::new("foot%1:08:02::"),
                   get_sense_key(&lexicon, "foot", &entry, None, &synset,
                                 &SynsetId::new("00001740-n")));
    }

    #[test]
    fn test_sense_key_3() {
        let mut lexicon = Lexicon::new();
        let mut synset1 = Synset::new(PartOfSpeech::a);
        synset1.members.push("hot".to_string());
        let mut synset2 = Synset::new(PartOfSpeech::s);
        synset2.similar.push(SynsetId::new("00000001-a"));
        synset1.members.push("scorching".to_string());
        let mut entry1 = Entry::new();
        entry1.sense.push(Sense::new(
                SenseId::new("hot%3:00:01::"),
                SynsetId::new("00000001-a")));
        let mut entry2 = Entry::new();
        entry2.sense.push(Sense::new(
                SenseId::new("scorching%5:00:01:???:"),
                SynsetId::new("00000002-s")
                ));
        lexicon.insert_entry("hot".to_string(),
            PosKey::new("a".to_string()), entry1.clone());
        lexicon.insert_entry("scorching".to_string(),
            PosKey::new("s".to_string()), entry2.clone());
        lexicon.insert_synset("adj.all".to_string(),
            SynsetId::new("00000001-a"), synset1.clone());
        lexicon.insert_synset("adj.all".to_string(),
            SynsetId::new("00000002-s"), synset2.clone());
        assert_eq!(SenseId::new("scorching%5:00:01:hot:01"),
                   get_sense_key(&lexicon, "scorching", &entry2, 
                                 Some(&SenseId::new("scorching%5:00:01:???:")),
                                 &synset2,
                                 &SynsetId::new("00000002-s")));
    }


}
