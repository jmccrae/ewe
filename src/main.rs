#[macro_use]
extern crate lazy_static;

mod wordnet;
mod change_manager;

use std::io::BufReader;
use std::fs::File;

fn main() {
    let f = BufReader::new(File::open("wn.xml").expect("Cannot open file"));

    let lexicon = wordnet::parse_wordnet(f).expect("Cannot parse");

    let mut f2 = File::create("wn2.xml").expect("Cannot open file");

    lexicon[0].to_xml(&mut f2, false).expect("Cannot write");
}
