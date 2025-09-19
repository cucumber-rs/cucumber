//! Rule-level events.

use super::{RetryableScenario, Source};

/// Event specific to a particular [Rule].
///
/// [Rule]: https://cucumber.io/docs/gherkin/reference#rule
#[derive(Debug)]
pub enum Rule<World> {
    /// [`Rule`] execution being started.
    ///
    /// [`Rule`]: gherkin::Rule
    Started,

    /// [`Scenario`] event.
    Scenario(Source<gherkin::Scenario>, RetryableScenario<World>),

    /// [`Rule`] execution being finished.
    ///
    /// [`Rule`]: gherkin::Rule
    Finished,
}

// Manual implementation is required to omit the redundant `World: Clone` trait
// bound imposed by `#[derive(Clone)]`.
impl<World> Clone for Rule<World> {
    fn clone(&self) -> Self {
        match self {
            Self::Started => Self::Started,
            Self::Scenario(s, ev) => Self::Scenario(s.clone(), ev.clone()),
            Self::Finished => Self::Finished,
        }
    }
}