use oewn_lib::wordnet::ReDBLexicon;
use oewn_lib::wordnet::Lexicon;
use oewn_lib::progress::Progress;

mod settings;
use settings::EweSettings;

struct PrintlnProgress(u64);

impl Progress for PrintlnProgress {
    fn start(&mut self, total : u64) {
        self.0 = total;
        println!("Start loading");
    }

    fn inc(&mut self, amount : u64) {
        let percent = (amount as f64 / self.0 as f64) * 100.0;
        println!("Loading... {:.2}%", percent);
    }

    fn finish(&mut self) {
        println!("Finished loading");
    }
    fn set_percent_mode(&mut self, percent_mode: bool) {
        // This progress implementation always uses percent mode, so we can ignore this setting.
    }
}

fn main() {
    let settings = if std::path::Path::new("settings.toml").exists() {
        EweSettings::load("settings.toml").expect("Failed to load settings")
    } else {
        EweSettings::default()
    };

    if let Some(source) = settings.wordnet_source {
        let mut progress = PrintlnProgress(0);
        let lexicon = ReDBLexicon::create(&settings.database).expect("Failed to create lexicon");
        lexicon.load(source, &mut progress).expect("Failed to load lexicon from source");
        eprintln!("Lexicon loaded successfully");
    } else {
        panic!("No source provided in settings, please configure the settings with a source to load the lexicon from");
    }
}
