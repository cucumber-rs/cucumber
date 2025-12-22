//! Scenario tracking utilities for test execution state management.

use std::collections::HashMap;

use crate::event::Source;

/// Indicator of a [`Failed`], [`Skipped`] or retried [`Scenario`].
///
/// This enum tracks the current state of a scenario for statistical purposes.
/// It helps distinguish between different types of scenario outcomes to provide
/// accurate reporting.
///
/// [`Failed`]: crate::event::Step::Failed
/// [`Scenario`]: gherkin::Scenario
/// [`Skipped`]: crate::event::Step::Skipped
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Indicator {
    /// [`Failed`] [`Scenario`].
    ///
    /// Indicates that the scenario failed during execution, either due to a
    /// failed step or a failed hook.
    ///
    /// [`Failed`]: crate::event::Step::Failed
    /// [`Scenario`]: gherkin::Scenario
    Failed,

    /// [`Skipped`] [`Scenario`].
    ///
    /// Indicates that the scenario was skipped during execution, typically
    /// due to a skipped step or unmet preconditions.
    ///
    /// [`Scenario`]: gherkin::Scenario
    /// [`Skipped`]: crate::event::Step::Skipped
    Skipped,

    /// Retried [`Scenario`].
    ///
    /// Indicates that the scenario was retried during execution due to a
    /// transient failure. This state helps track retry statistics.
    ///
    /// [`Scenario`]: gherkin::Scenario
    Retried,
}

impl Indicator {
    /// Returns `true` if the indicator represents a failed scenario.
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        matches!(self, Self::Failed)
    }

    /// Returns `true` if the indicator represents a skipped scenario.
    #[must_use]
    pub const fn is_skipped(&self) -> bool {
        matches!(self, Self::Skipped)
    }

    /// Returns `true` if the indicator represents a retried scenario.
    #[must_use]
    pub const fn is_retried(&self) -> bool {
        matches!(self, Self::Retried)
    }

    /// Returns a string representation of the indicator for display purposes.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Failed => "failed",
            Self::Skipped => "skipped",
            Self::Retried => "retried",
        }
    }
}

/// Type alias for tracking handled scenarios during test execution.
///
/// This [`HashMap`] keeps track of handled [`Scenario`]s using their full path
/// (including [`Feature`] and optional [`Rule`]) as the key to avoid collisions
/// when scenarios with identical content exist in different contexts.
///
/// The key is a tuple containing:
/// - [`Source<gherkin::Feature>`]: The feature containing the scenario
/// - [`Option<Source<gherkin::Rule>>`]: The optional rule containing the scenario
/// - [`Source<gherkin::Scenario>`]: The scenario itself
///
/// The value is an [`Indicator`] representing the current state of the scenario.
///
/// [`Feature`]: gherkin::Feature
/// [`Rule`]: gherkin::Rule
/// [`Scenario`]: gherkin::Scenario
pub type HandledScenarios = HashMap<
    (
        Source<gherkin::Feature>,
        Option<Source<gherkin::Rule>>,
        Source<gherkin::Scenario>,
    ),
    Indicator,
>;

/// Utility functions for working with [`HandledScenarios`].
#[derive(Clone, Copy, Debug)]
pub struct ScenarioTracker;

impl ScenarioTracker {
    /// Creates a new empty [`HandledScenarios`] map.
    #[must_use]
    pub fn new() -> HandledScenarios {
        HashMap::new()
    }

    /// Creates a scenario path tuple from the given components.
    ///
    /// This is a convenience function for creating the complex key type used
    /// in [`HandledScenarios`].
    #[must_use]
    pub fn create_path(
        feature: Source<gherkin::Feature>,
        rule: Option<Source<gherkin::Rule>>,
        scenario: Source<gherkin::Scenario>,
    ) -> (
        Source<gherkin::Feature>,
        Option<Source<gherkin::Rule>>,
        Source<gherkin::Scenario>,
    ) {
        (feature, rule, scenario)
    }

    /// Inserts or updates a scenario's indicator in the tracking map.
    ///
    /// Returns the previous indicator if one existed.
    pub fn update_scenario(
        scenarios: &mut HandledScenarios,
        feature: Source<gherkin::Feature>,
        rule: Option<Source<gherkin::Rule>>,
        scenario: Source<gherkin::Scenario>,
        indicator: Indicator,
    ) -> Option<Indicator> {
        let path = Self::create_path(feature, rule, scenario);
        scenarios.insert(path, indicator)
    }

    /// Removes a scenario from the tracking map.
    ///
    /// Returns the indicator that was associated with the scenario, if any.
    pub fn remove_scenario(
        scenarios: &mut HandledScenarios,
        feature: Source<gherkin::Feature>,
        rule: Option<Source<gherkin::Rule>>,
        scenario: Source<gherkin::Scenario>,
    ) -> Option<Indicator> {
        let path = Self::create_path(feature, rule, scenario);
        scenarios.remove(&path)
    }

    /// Gets the current indicator for a scenario.
    ///
    /// Returns `None` if the scenario is not being tracked.
    #[must_use]
    pub fn get_scenario_indicator<'a>(
        scenarios: &'a HandledScenarios,
        feature: &Source<gherkin::Feature>,
        rule: &Option<Source<gherkin::Rule>>,
        scenario: &Source<gherkin::Scenario>,
    ) -> Option<&'a Indicator> {
        let path = (feature.clone(), rule.clone(), scenario.clone());
        scenarios.get(&path)
    }
}

impl Default for ScenarioTracker {
    fn default() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indicator_is_methods() {
        assert!(Indicator::Failed.is_failed());
        assert!(!Indicator::Failed.is_skipped());
        assert!(!Indicator::Failed.is_retried());

        assert!(!Indicator::Skipped.is_failed());
        assert!(Indicator::Skipped.is_skipped());
        assert!(!Indicator::Skipped.is_retried());

        assert!(!Indicator::Retried.is_failed());
        assert!(!Indicator::Retried.is_skipped());
        assert!(Indicator::Retried.is_retried());
    }

    #[test]
    fn indicator_as_str() {
        assert_eq!(Indicator::Failed.as_str(), "failed");
        assert_eq!(Indicator::Skipped.as_str(), "skipped");
        assert_eq!(Indicator::Retried.as_str(), "retried");
    }

    #[test]
    fn scenario_tracker_new() {
        let scenarios = ScenarioTracker::new();
        assert!(scenarios.is_empty());
    }

    #[test]
    fn scenario_tracker_default() {
        let scenarios1 = ScenarioTracker::new();
        let scenarios2 = ScenarioTracker::new();
        
        // Both should create empty maps
        assert!(scenarios1.is_empty());
        assert!(scenarios2.is_empty());
    }

    #[test]
    fn indicator_equality() {
        assert_eq!(Indicator::Failed, Indicator::Failed);
        assert_eq!(Indicator::Skipped, Indicator::Skipped);
        assert_eq!(Indicator::Retried, Indicator::Retried);
        
        assert_ne!(Indicator::Failed, Indicator::Skipped);
        assert_ne!(Indicator::Failed, Indicator::Retried);
        assert_ne!(Indicator::Skipped, Indicator::Retried);
    }

    #[test]
    fn indicator_copy_and_clone() {
        let failed = Indicator::Failed;
        let failed_copy = failed; // Should work because it implements Copy
        let failed_clone = failed.clone(); // Should work because it implements Clone
        
        assert_eq!(failed, failed_copy);
        assert_eq!(failed, failed_clone);
        assert_eq!(failed_copy, failed_clone);
    }

    #[test]
    fn indicator_debug() {
        let failed = Indicator::Failed;
        let debug_str = format!("{:?}", failed);
        assert!(debug_str.contains("Failed"));
    }
}