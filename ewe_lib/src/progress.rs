//! A simple progress bar interface, and a null implementation.

/// This trait defines a simple interface for tracking progress of a long-running operation. It can be implemented by a progress bar, or by a null implementation that does nothing.
pub trait Progress {
    /// Start tracking progress, with the total number of steps to be completed.
    /// # Arguments
    /// * `total` - The total number of steps to be completed.
    /// # Examples
    /// ```rust
    /// use ewe_lib::progress::Progress;
    /// let mut progress = ewe_lib::progress::NullProgress;
    /// progress.start(100);
    /// ```
    fn start(&mut self, total : u64);

    /// Increment the progress by a certain amount.
    /// # Arguments
    /// * `amount` - The amount to increment the progress by.
    /// # Examples
    /// ```rust
    /// use ewe_lib::progress::Progress;
    /// let mut progress = ewe_lib::progress::NullProgress;
    /// progress.start(100);
    /// progress.inc(10);
    /// ```
    fn inc(&mut self, amount : u64);

    /// Finish tracking progress. This can be used to clean up any resources used by the progress tracker.
    /// # Examples
    /// ```rust
    /// use ewe_lib::progress::Progress;
    /// let mut progress = ewe_lib::progress::NullProgress;
    /// progress.start(100);
    /// progress.inc(100);
    /// progress.finish();
    fn finish(&mut self);

    /// Set whether to display progress as a percentage. This can be used to switch between different display modes for the progress tracker.
    /// # Arguments
    /// * `percent_mode` - Whether to display progress as a percentage.
    /// # Examples
    /// ```rust
    /// use ewe_lib::progress::Progress;
    /// let mut progress = ewe_lib::progress::NullProgress;
    /// progress.start(100);
    /// progress.set_percent_mode(true);
    /// progress.inc(10);
    /// ```
    fn set_percent_mode(&mut self, percent_mode: bool);
}

/// A null implementation of the `Progress` trait that does nothing. 
/// This can be used when you don't want to track progress, but need 
/// to provide a `Progress` implementation to a function that requires it.
pub struct NullProgress;

impl Progress for NullProgress {
    fn start(&mut self, _total : u64) {}
    fn inc(&mut self, _amount : u64) {}
    fn finish(&mut self) {}
    fn set_percent_mode(&mut self, _percent_mode: bool) {}
}

/// A `Progress` implementation that logs progress to stderr with `eprintln!`.
/// Reports at most once per percentage point, so it stays readable even when
/// `total` is very large (e.g. loading NameNet, which has far more source
/// files than the base WordNet).
pub struct LoggingProgress {
    current: u64,
    total: u64,
    last_logged_percent: u64,
}

impl LoggingProgress {
    pub fn new() -> Self {
        LoggingProgress { current: 0, total: 0, last_logged_percent: 0 }
    }
}

impl Default for LoggingProgress {
    fn default() -> Self {
        Self::new()
    }
}

impl Progress for LoggingProgress {
    fn start(&mut self, total : u64) {
        self.current = 0;
        self.total = total;
        self.last_logged_percent = 0;
        eprintln!("Loading: 0/{} files", total);
    }

    fn inc(&mut self, amount : u64) {
        self.current += amount;
        if self.total == 0 {
            return;
        }
        let percent = (self.current * 100 / self.total).min(100);
        if percent > self.last_logged_percent || self.current >= self.total {
            self.last_logged_percent = percent;
            eprintln!("Loading: {}/{} files ({}%)", self.current, self.total, percent);
        }
    }

    fn finish(&mut self) {
        eprintln!("Loading: done ({} files)", self.current);
    }

    fn set_percent_mode(&mut self, _percent_mode: bool) {}
}
