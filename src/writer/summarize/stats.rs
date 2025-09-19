//! Statistics collection and management for test execution results.

/// Execution statistics for tracking test results.
///
/// Tracks counts of passed, skipped, failed, and retried test steps or scenarios.
/// The `retried` count represents items that were retried during execution and is
/// not included in the total count to avoid double-counting.
///
/// [`Step`]: gherkin::Step
/// [`Scenario`]: gherkin::Scenario
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Stats {
    /// Number of passed [`Step`]s (or [`Scenario`]s).
    ///
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
    pub passed: usize,

    /// Number of skipped [`Step`]s (or [`Scenario`]s).
    ///
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
    pub skipped: usize,

    /// Number of failed [`Step`]s (or [`Scenario`]s).
    ///
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
    pub failed: usize,

    /// Number of retried [`Step`]s (or [`Scenario`]s).
    ///
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
    pub retried: usize,
}

impl Stats {
    /// Creates a new [`Stats`] instance with all counts set to zero.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            passed: 0,
            skipped: 0,
            failed: 0,
            retried: 0,
        }
    }

    /// Returns total number of [`Step`]s (or [`Scenario`]s), these [`Stats`]
    /// have been collected for.
    ///
    /// Note: `retried` count is intentionally not included here, as retried
    /// items are already counted in either `passed` or `failed`.
    ///
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
    #[must_use]
    pub const fn total(&self) -> usize {
        // We intentionally don't include `self.retried` number here, as it's
        // already counted either in `self.passed` or `self.failed`.
        self.passed + self.skipped + self.failed
    }

    /// Increments the passed count by one.
    pub fn increment_passed(&mut self) {
        self.passed += 1;
    }

    /// Increments the skipped count by one.
    pub fn increment_skipped(&mut self) {
        self.skipped += 1;
    }

    /// Increments the failed count by one.
    pub fn increment_failed(&mut self) {
        self.failed += 1;
    }

    /// Increments the retried count by one.
    pub fn increment_retried(&mut self) {
        self.retried += 1;
    }

    /// Decrements the skipped count by one, if greater than zero.
    pub fn decrement_skipped(&mut self) {
        if self.skipped > 0 {
            self.skipped -= 1;
        }
    }

    /// Returns `true` if all counts are zero.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.passed == 0 && self.skipped == 0 && self.failed == 0 && self.retried == 0
    }

    /// Returns `true` if there are any failed items.
    #[must_use]
    pub const fn has_failures(&self) -> bool {
        self.failed > 0
    }
}

impl Default for Stats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_stats_all_zero() {
        let stats = Stats::new();
        assert_eq!(stats.passed, 0);
        assert_eq!(stats.skipped, 0);
        assert_eq!(stats.failed, 0);
        assert_eq!(stats.retried, 0);
    }

    #[test]
    fn default_stats_same_as_new() {
        assert_eq!(Stats::default(), Stats::new());
    }

    #[test]
    fn total_excludes_retried() {
        let stats = Stats {
            passed: 5,
            skipped: 2,
            failed: 1,
            retried: 3,
        };
        assert_eq!(stats.total(), 8); // 5 + 2 + 1, excluding retried
    }

    #[test]
    fn increment_operations() {
        let mut stats = Stats::new();
        
        stats.increment_passed();
        stats.increment_skipped();
        stats.increment_failed();
        stats.increment_retried();
        
        assert_eq!(stats.passed, 1);
        assert_eq!(stats.skipped, 1);
        assert_eq!(stats.failed, 1);
        assert_eq!(stats.retried, 1);
    }

    #[test]
    fn decrement_skipped_works() {
        let mut stats = Stats { passed: 0, skipped: 3, failed: 0, retried: 0 };
        stats.decrement_skipped();
        assert_eq!(stats.skipped, 2);
    }

    #[test]
    fn decrement_skipped_at_zero_stays_zero() {
        let mut stats = Stats::new();
        stats.decrement_skipped();
        assert_eq!(stats.skipped, 0);
    }

    #[test]
    fn is_empty_true_for_new_stats() {
        assert!(Stats::new().is_empty());
    }

    #[test]
    fn is_empty_false_with_any_count() {
        let stats = Stats { passed: 1, skipped: 0, failed: 0, retried: 0 };
        assert!(!stats.is_empty());
    }

    #[test]
    fn has_failures_detects_failures() {
        let stats_with_failure = Stats { passed: 0, skipped: 0, failed: 1, retried: 0 };
        let stats_without_failure = Stats { passed: 1, skipped: 1, failed: 0, retried: 1 };
        
        assert!(stats_with_failure.has_failures());
        assert!(!stats_without_failure.has_failures());
    }

    #[test]
    fn equality_works() {
        let stats1 = Stats { passed: 1, skipped: 2, failed: 3, retried: 4 };
        let stats2 = Stats { passed: 1, skipped: 2, failed: 3, retried: 4 };
        let stats3 = Stats { passed: 1, skipped: 2, failed: 3, retried: 5 };
        
        assert_eq!(stats1, stats2);
        assert_ne!(stats1, stats3);
    }
}