//! Top-level Cucumber execution events.

use super::{Feature, RetryableScenario, Rule, Source};

/// Top-level [Cucumber] run event.
///
/// [Cucumber]: https://cucumber.io
#[derive(Debug)]
pub enum Cucumber<World> {
    /// [`Cucumber`] execution being started.
    Started,

    /// [`Feature`] event.
    Feature(Source<gherkin::Feature>, Feature<World>),

    /// All [`Feature`]s have been parsed.
    ///
    /// [`Feature`]: gherkin::Feature
    ParsingFinished {
        /// Number of parsed [`Feature`]s.
        ///
        /// [`Feature`]: gherkin::Feature
        features: usize,

        /// Number of parsed [`Rule`]s.
        ///
        /// [`Rule`]: gherkin::Rule
        rules: usize,

        /// Number of parsed [`Scenario`]s.
        ///
        /// [`Scenario`]: gherkin::Scenario
        scenarios: usize,

        /// Number of parsed [`Step`]s.
        ///
        /// [`Step`]: gherkin::Step
        steps: usize,

        /// Number of happened [`Parser`] errors.
        ///
        /// [`Parser`]: crate::Parser
        parser_errors: usize,
    },

    /// [`Cucumber`] execution being finished.
    Finished,
}

// Implemented manually to omit redundant `World: Clone` trait bound, imposed by
// `#[derive(Clone)]`.
impl<World> Clone for Cucumber<World> {
    fn clone(&self) -> Self {
        match self {
            Self::Started => Self::Started,
            Self::Feature(f, ev) => Self::Feature(f.clone(), ev.clone()),
            Self::ParsingFinished {
                features,
                rules,
                scenarios,
                steps,
                parser_errors,
            } => Self::ParsingFinished {
                features: *features,
                rules: *rules,
                scenarios: *scenarios,
                steps: *steps,
                parser_errors: *parser_errors,
            },
            Self::Finished => Self::Finished,
        }
    }
}

impl<World> Cucumber<World> {
    /// Constructs an event of a [`Feature`] being started.
    ///
    /// [`Feature`]: gherkin::Feature
    #[must_use]
    pub fn feature_started(feat: impl Into<Source<gherkin::Feature>>) -> Self {
        Self::Feature(feat.into(), Feature::Started)
    }

    /// Constructs an event of a [`Rule`] being started.
    ///
    /// [`Rule`]: gherkin::Rule
    #[must_use]
    pub fn rule_started(
        feat: impl Into<Source<gherkin::Feature>>,
        rule: impl Into<Source<gherkin::Rule>>,
    ) -> Self {
        Self::Feature(feat.into(), Feature::Rule(rule.into(), Rule::Started))
    }

    /// Constructs an event of a [`Feature`] being finished.
    ///
    /// [`Feature`]: gherkin::Feature
    #[must_use]
    pub fn feature_finished(feat: impl Into<Source<gherkin::Feature>>) -> Self {
        Self::Feature(feat.into(), Feature::Finished)
    }

    /// Constructs an event of a [`Rule`] being finished.
    ///
    /// [`Rule`]: gherkin::Rule
    #[must_use]
    pub fn rule_finished(
        feat: impl Into<Source<gherkin::Feature>>,
        rule: impl Into<Source<gherkin::Rule>>,
    ) -> Self {
        Self::Feature(feat.into(), Feature::Rule(rule.into(), Rule::Finished))
    }

    /// Constructs a [`Cucumber`] event from the given [`Scenario`] event.
    #[must_use]
    pub fn scenario(
        feat: impl Into<Source<gherkin::Feature>>,
        rule: Option<impl Into<Source<gherkin::Rule>>>,
        scenario: impl Into<Source<gherkin::Scenario>>,
        event: RetryableScenario<World>,
    ) -> Self {
        Self::Feature(
            feat.into(),
            if let Some(r) = rule {
                Feature::Rule(r.into(), Rule::Scenario(scenario.into(), event))
            } else {
                Feature::Scenario(scenario.into(), event)
            },
        )
    }
}