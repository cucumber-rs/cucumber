//! Validated retry budget and official inherited-tag selection.
//!
//! This helper supplies the existing retry-options callback. Select
//! `OutcomePolicy::CucumberJs` separately for FAILED-only retry eligibility.
use super::basic::{Cli, RetryOptions};
use crate::{event::Retries, tag::Expression};

/// Invalid retry configuration, rejected before parsing or execution begins.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum RetryPolicyError {
    /// An explicitly supplied tag expression requires a positive retry count.
    TagGateWithoutPositiveBudget,
}
impl std::fmt::Display for RetryPolicyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(
            "an explicit retry tag gate requires a positive retry count",
        )
    }
}
impl std::error::Error for RetryPolicyError {}

/// Explicit retry budget selected by the official inherited tag-expression grammar.
///
/// Native `@retry` magic tags and legacy retry CLI fields do not override this
/// policy. A later `retry_options` callback replaces the policy callback normally.
/// Configuration does not silently change the separate outcome policy.
#[derive(Clone, Debug)]
pub struct RetryPolicy {
    count: usize,
    tags: Option<Expression>,
}
impl RetryPolicy {
    /// Validates a retry count and optional official tag gate.
    ///
    /// Zero without a gate disables retry. `Some(expression)` is an explicit gate,
    /// even when the expression matches everything. Applications translating a
    /// JavaScript configuration's empty filter string should pass `None`.
    ///
    /// # Errors
    /// Returns [`RetryPolicyError::TagGateWithoutPositiveBudget`] for zero with a gate.
    pub fn new(
        count: usize,
        tags: Option<Expression>,
    ) -> Result<Self, RetryPolicyError> {
        if count == 0 && tags.is_some() {
            Err(RetryPolicyError::TagGateWithoutPositiveBudget)
        } else {
            Ok(Self { count, tags })
        }
    }

    /// Resolves this budget for a selected scenario, including inherited Examples tags.
    ///
    /// No magic tags are interpreted and no delay is added. This selects available
    /// retries; the executor's outcome policy independently decides eligibility.
    #[must_use]
    pub fn options(
        &self,
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
    ) -> Option<RetryOptions> {
        (self.count > 0
            && self
                .tags
                .as_ref()
                .is_none_or(|tags| tags.matches(feature, rule, scenario)))
        .then(|| RetryOptions {
            retries: Retries::initial(self.count),
            after: None,
        })
    }

    /// Adapts this validated policy to the existing `retry_options` builder.
    ///
    /// The callback deliberately ignores merged legacy CLI retry settings. Parse
    /// and validate application-owned typed CLI inputs before creating the policy
    /// when those inputs should control its budget or official tag gate.
    #[must_use]
    pub fn into_options(
        self,
    ) -> impl Fn(
        &gherkin::Feature,
        Option<&gherkin::Rule>,
        &gherkin::Scenario,
        &Cli,
    ) -> Option<RetryOptions>
    + Clone
    + 'static {
        move |feature, rule, scenario, _legacy_cli| {
            self.options(feature, rule, scenario)
        }
    }
}
