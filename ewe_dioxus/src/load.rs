use std::path::Path;
use std::env;
use std::fs;

mod backend;
use backend::Lexicon;

fn main() {
    if env::args().len() > 1 {
        let path = env::args().nth(1).unwrap();
        let path = if Path::new(&path).join("entries-a.yaml").exists() {
            Path::new(&path).to_path_buf()
        } else if Path::new(&path).join("src").join("yaml").join("entries-a.yaml").exists() {
            Path::new(&path).join("src").join("yaml").to_path_buf()
        } else {
            eprintln!("Could not find WordNet at {}", path);
            std::process::exit(1);
        };
        // Delete wordnet.db and wordnet.data if exist
        // Ignore NotFound errors
        if let Err(e) = fs::remove_file("wordnet.db") {
            if e.kind() != std::io::ErrorKind::NotFound {
                eprintln!("Error removing wordnet.db: {}", e);
                std::process::exit(1);
            }
        }
        if let Err(e) = fs::remove_file("wordnet.data") {
            if e.kind() != std::io::ErrorKind::NotFound {
                eprintln!("Error removing wordnet.data: {}", e);
                std::process::exit(1);
            }
        }
        if let Err(e) = Lexicon::load(&path) {
            eprintln!("Error loading WordNet: {}", e);
            std::process::exit(1);
        };
    } else {
        eprintln!("Usage: load <path to WordNet>");
        std::process::exit(1);
    }
}
    

