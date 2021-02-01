#[macro_use]
extern crate lazy_static;

mod wordnet;

use std::fs::File;

fn main() {
    let f = File::open("foo.xml").expect("Cannot open file");

    let lexicon = wordnet::parse_wordnet(f).expect("Cannot parse");

    let mut f2 = File::create("foo.xml").expect("Cannot open file");

    lexicon[0].to_xml(&mut f2, false).expect("Cannot write");
}
