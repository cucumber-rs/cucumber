//! Retry logic for scenarios.

use std::hash::Hash;

/// Number of retry attempts for a [`Scenario`].
///
/// [`Scenario`]: gherkin::Scenario
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Retries {
    /// Current retry attempt.
    pub current: usize,

    /// Available retries left.
    pub left: usize,
}

impl Retries {
    /// Creates initial [`Retries`].
    #[must_use]
    pub const fn initial(left: usize) -> Self {
        Self { left, current: 0 }
    }

    /// Returns [`Some`], in case next retry attempt is available, or [`None`]
    /// otherwise.
    #[must_use]
    pub fn next_try(self) -> Option<Self> {
        self.left
            .checked_sub(1)
            .map(|left| Self { left, current: self.current + 1 })
    }
}