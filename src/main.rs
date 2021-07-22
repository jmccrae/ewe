extern crate lazy_static;
extern crate serde;
extern crate serde_yaml;

//mod wordnet;
//mod change_manager;
mod wordnet_yaml;

fn main() {
    wordnet_yaml::Lexicon::load("/home/jmccrae/projects/globalwordnet/english-wordnet/src/yaml/").unwrap();
//    let f = BufReader::new(File::open("wn.xml").expect("Cannot open file"));
//
//    let lexicon = wordnet::parse_wordnet(f).expect("Cannot parse");
//
//    let mut f2 = File::create("wn2.xml").expect("Cannot open file");
//
//    lexicon[0].to_xml(&mut f2, false).expect("Cannot write");
}
