//! Hook-related events and types.

use std::sync::Arc;

use derive_more::with_trait::Display;

use super::event_struct::Info;

/// Type of hook executed before or after all [`Scenario`]'s [`Step`]s.
///
/// [`Scenario`]: gherkin::Scenario
/// [`Step`]: gherkin::Step
#[derive(Clone, Copy, Debug, Display, PartialEq, Eq)]
#[display("{self:?}")]
pub enum HookType {
    /// Executing on each [`Scenario`] before running all [`Step`]s.
    ///
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
    Before,

    /// Executing on each [`Scenario`] after running all [`Step`]s.
    ///
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
    After,
}

/// Event of running [`Before`] or [`After`] hook.
///
/// [`After`]: HookType::After
/// [`Before`]: HookType::Before
#[derive(Debug)]
pub enum Hook<World> {
    /// Hook execution being started.
    Started,

    /// Hook passed.
    Passed,

    /// Hook failed.
    Failed(Option<Arc<World>>, Info),
}

// Manual implementation is required to omit the redundant `World: Clone` trait
// bound imposed by `#[derive(Clone)]`.
impl<World> Clone for Hook<World> {
    fn clone(&self) -> Self {
        match self {
            Self::Started => Self::Started,
            Self::Passed => Self::Passed,
            Self::Failed(w, i) => Self::Failed(w.clone(), Arc::clone(i)),
        }
    }
}