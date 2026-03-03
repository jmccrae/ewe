//! A simple progress bar interface, and a null implementation.

/// This trait defines a simple interface for tracking progress of a long-running operation. It can be implemented by a progress bar, or by a null implementation that does nothing.
pub trait Progress {
    /// Start tracking progress, with the total number of steps to be completed.
    /// # Arguments
    /// * `total` - The total number of steps to be completed.
    /// # Examples
    /// ```rust
    /// let mut progress = NullProgress;
    /// progress.start(100);
    /// ```
    fn start(&mut self, total : u64);

    /// Increment the progress by a certain amount.
    /// # Arguments
    /// * `amount` - The amount to increment the progress by.
    /// # Examples
    /// ```rust
    /// let mut progress = NullProgress;
    /// progress.start(100);
    /// progress.inc(10);
    /// ```
    fn inc(&mut self, amount : u64);

    /// Finish tracking progress. This can be used to clean up any resources used by the progress tracker.
    /// # Examples
    /// ```rust
    /// let mut progress = NullProgress;
    /// progress.start(100);
    /// progress.inc(100);
    /// progress.finish();
    fn finish(&mut self);

    /// Set whether to display progress as a percentage. This can be used to switch between different display modes for the progress tracker.
    /// # Arguments
    /// * `percent_mode` - Whether to display progress as a percentage.
    /// # Examples
    /// ```rust
    /// let mut progress = NullProgress;
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
