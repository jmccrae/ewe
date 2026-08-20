//! Fixed lookup tables WNDB's format needs that aren't derived from lexicon content.

use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    /// The 39 Princeton verb-frame sentences in `WNDB.PRINCETON_FRAMES` (`gwn-scala-api`'s
    /// `wndb.scala`) - even though ids 36-39 are always filtered out of real output
    /// (`WNDB.write` does `frameRefs.filter(_._1 < 36)`), they still need to be *present* here:
    /// a description matching one of them must resolve to its real id (36-39, later filtered)
    /// rather than falling through to this table's "not found" default of `0` - which, unlike a
    /// filtered 36-39, is NOT excluded by that `< 36` filter and would wrongly appear in output.
    /// Confirmed empirically: omitting these produced a spurious `+ 00 00` for "via-for" senses
    /// (frame 36, "Somebody ----s for something") that real `data.verb` never emits.
    pub(crate) static ref PRINCETON_FRAMES: HashMap<&'static str, u32> = {
        let mut map = HashMap::new();
        map.insert("Something ----s", 1);
        map.insert("Somebody ----s", 2);
        map.insert("It is ----ing", 3);
        map.insert("Something is ----ing PP", 4);
        map.insert("Something ----s something Adjective/Noun", 5);
        map.insert("Something ----s Adjective/Noun", 6);
        map.insert("Somebody ----s Adjective", 7);
        map.insert("Somebody ----s something", 8);
        map.insert("Somebody ----s somebody", 9);
        map.insert("Something ----s somebody", 10);
        map.insert("Something ----s something", 11);
        map.insert("Something ----s to somebody", 12);
        map.insert("Somebody ----s on something", 13);
        map.insert("Somebody ----s somebody something", 14);
        map.insert("Somebody ----s something to somebody", 15);
        map.insert("Somebody ----s something from somebody", 16);
        map.insert("Somebody ----s somebody with something", 17);
        map.insert("Somebody ----s somebody of something", 18);
        map.insert("Somebody ----s something on somebody", 19);
        map.insert("Somebody ----s somebody PP", 20);
        map.insert("Somebody ----s something PP", 21);
        map.insert("Somebody ----s PP", 22);
        map.insert("Somebody's (body part) ----s", 23);
        map.insert("Somebody ----s somebody to INFINITIVE", 24);
        map.insert("Somebody ----s somebody INFINITIVE", 25);
        map.insert("Somebody ----s that CLAUSE", 26);
        map.insert("Somebody ----s to somebody", 27);
        map.insert("Somebody ----s to INFINITIVE", 28);
        map.insert("Somebody ----s whether INFINITIVE", 29);
        map.insert("Somebody ----s somebody into V-ing something", 30);
        map.insert("Somebody ----s something with something", 31);
        map.insert("Somebody ----s INFINITIVE", 32);
        map.insert("Somebody ----s VERB-ing", 33);
        map.insert("It ----s that CLAUSE", 34);
        map.insert("Something ----s INFINITIVE", 35);
        map.insert("Somebody ----s for something", 36);
        map.insert("Somebody ----s at something", 37);
        map.insert("Somebody ----s on somebody", 38);
        map.insert("Somebody ----s out of somebody", 39);
        map
    };
}
