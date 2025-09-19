//! Feature-level events.

use super::{RetryableScenario, Rule, Source};

/// Event specific to a particular [Feature].
///
/// [Feature]: https://cucumber.io/docs/gherkin/reference#feature
#[derive(Debug)]
pub enum Feature<World> {
    /// [`Feature`] execution being started.
    ///
    /// [`Feature`]: gherkin::Feature
    Started,

    /// [`Rule`] event.
    Rule(Source<gherkin::Rule>, Rule<World>),

    /// [`Scenario`] event.
    Scenario(Source<gherkin::Scenario>, RetryableScenario<World>),

    /// [`Feature`] execution being finished.
    ///
    /// [`Feature`]: gherkin::Feature
    Finished,
}

// Manual implementation is required to omit the redundant `World: Clone` trait
// bound imposed by `#[derive(Clone)]`.
impl<World> Clone for Feature<World> {
    fn clone(&self) -> Self {
        match self {
            Self::Started => Self::Started,
            Self::Rule(r, ev) => Self::Rule(r.clone(), ev.clone()),
            Self::Scenario(s, ev) => Self::Scenario(s.clone(), ev.clone()),
            Self::Finished => Self::Finished,
        }
    }
}