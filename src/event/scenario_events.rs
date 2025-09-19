//! Scenario-level events and retries.

use std::sync::Arc;

use crate::step;

use super::{Hook, HookType, Source, Step, StepError, event_struct::Info, retries::Retries};

/// Event specific to a particular [Scenario].
///
/// [Scenario]: https://cucumber.io/docs/gherkin/reference#example
#[derive(Debug)]
pub enum Scenario<World> {
    /// [`Scenario`] execution being started.
    ///
    /// [`Scenario`]: gherkin::Scenario
    Started,

    /// [`Hook`] event.
    Hook(HookType, Hook<World>),

    /// [`Background`] [`Step`] event.
    ///
    /// [`Background`]: gherkin::Background
    Background(Source<gherkin::Step>, Step<World>),

    /// [`Step`] event.
    Step(Source<gherkin::Step>, Step<World>),

    /// [`Scenario`]'s log entry is emitted.
    Log(String),

    /// [`Scenario`] execution being finished.
    ///
    /// [`Scenario`]: gherkin::Scenario
    Finished,
}

// Manual implementation is required to omit the redundant `World: Clone` trait
// bound imposed by `#[derive(Clone)]`.
impl<World> Clone for Scenario<World> {
    fn clone(&self) -> Self {
        match self {
            Self::Started => Self::Started,
            Self::Hook(ty, ev) => Self::Hook(*ty, ev.clone()),
            Self::Background(bg, ev) => {
                Self::Background(bg.clone(), ev.clone())
            }
            Self::Step(st, ev) => Self::Step(st.clone(), ev.clone()),
            Self::Log(msg) => Self::Log(msg.clone()),
            Self::Finished => Self::Finished,
        }
    }
}

impl<World> Scenario<World> {
    /// Constructs an event of a [`Scenario`] hook being started.
    ///
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    pub const fn hook_started(which: HookType) -> Self {
        Self::Hook(which, Hook::Started)
    }

    /// Constructs an event of a passed [`Scenario`] hook.
    ///
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    pub const fn hook_passed(which: HookType) -> Self {
        Self::Hook(which, Hook::Passed)
    }

    /// Constructs an event of a failed [`Scenario`] hook.
    ///
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    pub fn hook_failed(
        which: HookType,
        world: Option<Arc<World>>,
        info: Info,
    ) -> Self {
        Self::Hook(which, Hook::Failed(world, info))
    }

    /// Constructs an event of a [`Step`] being started.
    ///
    /// [`Step`]: gherkin::Step
    #[must_use]
    pub fn step_started(step: impl Into<Source<gherkin::Step>>) -> Self {
        Self::Step(step.into(), Step::Started)
    }

    /// Constructs an event of a [`Background`] [`Step`] being started.
    ///
    /// [`Background`]: gherkin::Background
    /// [`Step`]: gherkin::Step
    #[must_use]
    pub fn background_step_started(
        step: impl Into<Source<gherkin::Step>>,
    ) -> Self {
        Self::Background(step.into(), Step::Started)
    }

    /// Constructs an event of a passed [`Step`].
    ///
    /// [`Step`]: gherkin::Step
    #[must_use]
    pub fn step_passed(
        step: impl Into<Source<gherkin::Step>>,
        captures: regex::CaptureLocations,
        loc: Option<step::Location>,
    ) -> Self {
        Self::Step(step.into(), Step::Passed(captures, loc))
    }

    /// Constructs an event of a passed [`Background`] [`Step`].
    ///
    /// [`Background`]: gherkin::Background
    /// [`Step`]: gherkin::Step
    #[must_use]
    pub fn background_step_passed(
        step: impl Into<Source<gherkin::Step>>,
        captures: regex::CaptureLocations,
        loc: Option<step::Location>,
    ) -> Self {
        Self::Background(step.into(), Step::Passed(captures, loc))
    }

    /// Constructs an event of a skipped [`Step`].
    ///
    /// [`Step`]: gherkin::Step
    #[must_use]
    pub fn step_skipped(step: impl Into<Source<gherkin::Step>>) -> Self {
        Self::Step(step.into(), Step::Skipped)
    }
    /// Constructs an event of a skipped [`Background`] [`Step`].
    ///
    /// [`Background`]: gherkin::Background
    /// [`Step`]: gherkin::Step
    #[must_use]
    pub fn background_step_skipped(
        step: impl Into<Source<gherkin::Step>>,
    ) -> Self {
        Self::Background(step.into(), Step::Skipped)
    }

    /// Constructs an event of a failed [`Step`].
    ///
    /// [`Step`]: gherkin::Step
    #[must_use]
    pub fn step_failed(
        step: impl Into<Source<gherkin::Step>>,
        captures: Option<regex::CaptureLocations>,
        loc: Option<step::Location>,
        world: Option<Arc<World>>,
        info: impl Into<StepError>,
    ) -> Self {
        Self::Step(step.into(), Step::Failed(captures, loc, world, info.into()))
    }

    /// Constructs an event of a failed [`Background`] [`Step`].
    ///
    /// [`Background`]: gherkin::Background
    /// [`Step`]: gherkin::Step
    #[must_use]
    pub fn background_step_failed(
        step: impl Into<Source<gherkin::Step>>,
        captures: Option<regex::CaptureLocations>,
        loc: Option<step::Location>,
        world: Option<Arc<World>>,
        info: impl Into<StepError>,
    ) -> Self {
        Self::Background(
            step.into(),
            Step::Failed(captures, loc, world, info.into()),
        )
    }

    /// Transforms this [`Scenario`] event into a [`RetryableScenario`] event.
    #[must_use]
    pub const fn with_retries(
        self,
        retries: Option<Retries>,
    ) -> RetryableScenario<World> {
        RetryableScenario { event: self, retries }
    }
}

/// Event specific to a particular retryable [Scenario].
///
/// [Scenario]: https://cucumber.io/docs/gherkin/reference#example
#[derive(Debug)]
pub struct RetryableScenario<World> {
    /// Happened [`Scenario`] event.
    pub event: Scenario<World>,

    /// Number of [`Retries`].
    pub retries: Option<Retries>,
}

// Manual implementation is required to omit the redundant `World: Clone` trait
// bound imposed by `#[derive(Clone)]`.
impl<World> Clone for RetryableScenario<World> {
    fn clone(&self) -> Self {
        Self { event: self.event.clone(), retries: self.retries }
    }
}

/// Event explaining why a [Scenario] has finished.
///
/// [Scenario]: https://cucumber.io/docs/gherkin/reference#example
#[derive(Clone, Debug)]
pub enum ScenarioFinished {
    /// [`Before`] [`Hook::Failed`].
    ///
    /// [`Before`]: HookType::Before
    BeforeHookFailed(Info),

    /// [`Step::Passed`].
    StepPassed,

    /// [`Step::Skipped`].
    StepSkipped,

    /// [`Step::Failed`].
    StepFailed(
        Option<regex::CaptureLocations>,
        Option<step::Location>,
        StepError,
    ),
}