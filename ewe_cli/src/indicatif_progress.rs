use oewn_lib::progress::Progress;
use indicatif::ProgressBar;

pub struct IndicatifProgress {
    progress_bar: Option<ProgressBar>,
}

impl Progress for IndicatifProgress {
    fn start(&mut self, total : u64) {
        self.progress_bar = Some(ProgressBar::new(total));
    }
    fn inc(&mut self, amount : u64) {
        if let Some(ref progress_bar) = self.progress_bar {
            progress_bar.inc(amount);
        }
    }
    fn finish(&mut self) {
        if let Some(ref progress_bar) = self.progress_bar {
            progress_bar.finish();
        }
    }
    fn set_percent_mode(&mut self, percent_mode: bool) {
        if let Some(ref progress_bar) = self.progress_bar {
            if percent_mode {
                progress_bar.set_style(indicatif::ProgressStyle::default_bar()
                    .template("{bar:40.cyan/blue} {pos:>7}/{len:7} {percent}%")
                    .expect("Invalid progress bar template")
                    .progress_chars("##-"));
            } else {
                progress_bar.set_style(indicatif::ProgressStyle::default_bar()
                    .template("{bar:40.cyan/blue} {pos:>7}/{len:7}")
                    .expect("Invalid progress bar template")
                    .progress_chars("##-"));
            }
        }
    }
}

impl IndicatifProgress {
    pub fn new() -> Self {
        IndicatifProgress { progress_bar: None }
    }
}
