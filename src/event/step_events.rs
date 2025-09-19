//! Step-level events and errors.

use std::sync::Arc;

use derive_more::with_trait::{Display, Error, From};

use crate::{step, writer::basic::coerce_error};

use super::event_struct::Info;

/// Event specific to a particular [Step].
///
/// [Step]: https://cucumber.io/docs/gherkin/reference#step
#[derive(Debug)]
pub enum Step<World> {
    /// [`Step`] execution being started.
    ///
    /// [`Step`]: gherkin::Step
    Started,

    /// [`Step`] being skipped.
    ///
    /// That means there is no [`Regex`] matching [`Step`] in a
    /// [`step::Collection`].
    ///
    /// [`Regex`]: regex::Regex
    /// [`Step`]: gherkin::Step
    /// [`step::Collection`]: crate::step::Collection
    Skipped,

    /// [`Step`] passed.
    ///
    /// [`Step`]: gherkin::Step
    Passed(regex::CaptureLocations, Option<step::Location>),

    /// [`Step`] failed.
    ///
    /// [`Step`]: gherkin::Step
    Failed(
        Option<regex::CaptureLocations>,
        Option<step::Location>,
        Option<Arc<World>>,
        StepError,
    ),
}

// Manual implementation is required to omit the redundant `World: Clone` trait
// bound imposed by `#[derive(Clone)]`.
impl<World> Clone for Step<World> {
    fn clone(&self) -> Self {
        match self {
            Self::Started => Self::Started,
            Self::Skipped => Self::Skipped,
            Self::Passed(captures, loc) => Self::Passed(captures.clone(), *loc),
            Self::Failed(captures, loc, w, info) => {
                Self::Failed(captures.clone(), *loc, w.clone(), info.clone())
            }
        }
    }
}

/// Error of executing a [`Step`].
///
/// [`Step`]: gherkin::Step
#[derive(Clone, Debug, Display, Error, From)]
pub enum StepError {
    /// [`Step`] doesn't match any [`Regex`].
    ///
    /// It's emitted whenever a [`Step::Skipped`] event cannot be tolerated
    /// (such as when [`fail_on_skipped()`] is used).
    ///
    /// [`Regex`]: regex::Regex
    /// [`fail_on_skipped()`]: crate::WriterExt::fail_on_skipped()
    #[display("Step doesn't match any function")]
    NotFound,

    /// [`Step`] matches multiple [`Regex`]es.
    ///
    /// [`Regex`]: regex::Regex
    /// [`Step`]: gherkin::Step
    #[display("Step match is ambiguous: {_0}")]
    AmbiguousMatch(step::AmbiguousMatchError),

    /// [`Step`] panicked.
    ///
    /// [`Step`]: gherkin::Step
    #[display("Step panicked. Captured output: {}", coerce_error(_0))]
    Panic(#[error(not(source))] Info),
}