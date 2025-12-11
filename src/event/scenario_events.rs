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
        Self::Step(step.into(), Step::Passed { captures, location: loc })
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
        Self::Background(step.into(), Step::Passed { captures, location: loc })
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
        Self::Step(step.into(), Step::Failed { captures, location: loc, world, error: info.into() })
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
            Step::Failed { captures, location: loc, world, error: info.into() },
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::step;

    #[derive(Debug, Clone)]
    struct TestWorld {
        value: String,
    }

    fn create_test_step() -> gherkin::Step {
        gherkin::Step {
            keyword: "Given".to_string(),
            ty: gherkin::StepType::Given,
            value: "a test step".to_string(),
            docstring: None,
            table: None,
            span: gherkin::Span { start: 0, end: 10 },
            position: gherkin::LineCol { line: 1, col: 1 },
        }
    }

    #[test]
    fn test_scenario_started_event() {
        let event: Scenario<TestWorld> = Scenario::Started;
        match event {
            Scenario::Started => {},
            _ => panic!("Expected Started event"),
        }
    }

    #[test]
    fn test_scenario_finished_event() {
        let event: Scenario<TestWorld> = Scenario::Finished;
        match event {
            Scenario::Finished => {},
            _ => panic!("Expected Finished event"),
        }
    }

    #[test]
    fn test_scenario_log_event() {
        let log_msg = "Test log message";
        let event: Scenario<TestWorld> = Scenario::Log(log_msg.to_string());
        match event {
            Scenario::Log(msg) => assert_eq!(msg, log_msg),
            _ => panic!("Expected Log event"),
        }
    }

    #[test]
    fn test_scenario_clone() {
        let events = vec![
            Scenario::<TestWorld>::Started,
            Scenario::Finished,
            Scenario::Log("test".to_string()),
            Scenario::hook_started(HookType::Before),
            Scenario::hook_passed(HookType::After),
        ];

        for event in events {
            let cloned = event.clone();
            match (&event, &cloned) {
                (Scenario::Started, Scenario::Started) => {},
                (Scenario::Finished, Scenario::Finished) => {},
                (Scenario::Log(a), Scenario::Log(b)) => assert_eq!(a, b),
                (Scenario::Hook(t1, h1), Scenario::Hook(t2, h2)) => {
                    assert_eq!(t1, t2);
                    match (h1, h2) {
                        (Hook::Started, Hook::Started) => {},
                        (Hook::Passed, Hook::Passed) => {},
                        _ => {},
                    }
                },
                _ => {},
            }
        }
    }

    #[test]
    fn test_hook_events() {
        let started = Scenario::<TestWorld>::hook_started(HookType::Before);
        assert!(matches!(started, Scenario::Hook(HookType::Before, Hook::Started)));

        let passed = Scenario::<TestWorld>::hook_passed(HookType::After);
        assert!(matches!(passed, Scenario::Hook(HookType::After, Hook::Passed)));

        let world = Arc::new(TestWorld { value: "test".to_string() });
        let info = Arc::new("Hook failed".to_string());
        let failed = Scenario::hook_failed(HookType::Before, Some(world.clone()), info.clone());
        
        match failed {
            Scenario::Hook(HookType::Before, Hook::Failed(w, i)) => {
                assert!(w.is_some());
                assert_eq!(w.unwrap().value, "test");
            },
            _ => panic!("Expected Hook::Failed"),
        }
    }

    #[test]
    fn test_step_started_events() {
        let step = create_test_step();
        
        let step_started = Scenario::<TestWorld>::step_started(Source::new(step.clone()));
        assert!(matches!(step_started, Scenario::Step(_, Step::Started)));

        let bg_step_started = Scenario::<TestWorld>::background_step_started(Source::new(step));
        assert!(matches!(bg_step_started, Scenario::Background(_, Step::Started)));
    }

    #[test]
    fn test_step_passed_events() {
        let step = create_test_step();
        let captures = regex::Regex::new("test").unwrap().capture_locations();
        let loc = step::Location::new("file.rs", 10, 5);
        
        let step_passed = Scenario::<TestWorld>::step_passed(
            Source::new(step.clone()),
            captures.clone(),
            Some(loc),
        );
        
        match step_passed {
            Scenario::Step(_, Step::Passed { location, .. }) => {
                assert!(location.is_some());
                assert_eq!(location.unwrap().line, 10);
            },
            _ => panic!("Expected Step::Passed"),
        }

        let bg_step_passed = Scenario::<TestWorld>::background_step_passed(
            Source::new(step),
            captures,
            Some(loc),
        );
        
        assert!(matches!(bg_step_passed, Scenario::Background(_, Step::Passed { .. })));
    }

    #[test]
    fn test_step_skipped_events() {
        let step = create_test_step();
        
        let step_skipped = Scenario::<TestWorld>::step_skipped(Source::new(step.clone()));
        assert!(matches!(step_skipped, Scenario::Step(_, Step::Skipped)));

        let bg_step_skipped = Scenario::<TestWorld>::background_step_skipped(Source::new(step));
        assert!(matches!(bg_step_skipped, Scenario::Background(_, Step::Skipped)));
    }

    #[test]
    fn test_step_failed_events() {
        let step = create_test_step();
        let captures = regex::Regex::new("test").unwrap().capture_locations();
        let loc = step::Location::new("file.rs", 10, 5);
        let world = Arc::new(TestWorld { value: "test".to_string() });
        let error = StepError::NotFound;
        
        let step_failed = Scenario::step_failed(
            Source::new(step.clone()),
            Some(captures.clone()),
            Some(loc),
            Some(world.clone()),
            error.clone(),
        );
        
        match step_failed {
            Scenario::Step(_, Step::Failed { location, world: w, error: e, .. }) => {
                assert!(location.is_some());
                assert!(w.is_some());
                assert!(matches!(e, StepError::NotFound));
            },
            _ => panic!("Expected Step::Failed"),
        }

        let bg_step_failed = Scenario::background_step_failed(
            Source::new(step),
            Some(captures),
            Some(loc),
            Some(world),
            error,
        );
        
        assert!(matches!(bg_step_failed, Scenario::Background(_, Step::Failed { .. })));
    }

    #[test]
    fn test_with_retries() {
        let event = Scenario::<TestWorld>::Started;
        let retries = Retries { current: 1, left: 2 };
        
        let retryable = event.with_retries(Some(retries));
        assert!(matches!(retryable.event, Scenario::Started));
        assert!(retryable.retries.is_some());
        assert_eq!(retryable.retries.unwrap().current, 1);
        assert_eq!(retryable.retries.unwrap().left, 2);
        
        let no_retry = Scenario::<TestWorld>::Finished.with_retries(None);
        assert!(no_retry.retries.is_none());
    }

    #[test]
    fn test_retryable_scenario_clone() {
        let retries = Retries { current: 1, left: 2 };
        let retryable = RetryableScenario {
            event: Scenario::<TestWorld>::Started,
            retries: Some(retries),
        };
        
        let cloned = retryable.clone();
        assert!(matches!(cloned.event, Scenario::Started));
        assert_eq!(cloned.retries, retryable.retries);
    }

    #[test]
    fn test_scenario_finished_variants() {
        let info = Arc::new("Before hook failed".to_string());
        let before_failed = ScenarioFinished::BeforeHookFailed(info);
        assert!(matches!(before_failed, ScenarioFinished::BeforeHookFailed(_)));
        
        let step_passed = ScenarioFinished::StepPassed;
        assert!(matches!(step_passed, ScenarioFinished::StepPassed));
        
        let step_skipped = ScenarioFinished::StepSkipped;
        assert!(matches!(step_skipped, ScenarioFinished::StepSkipped));
        
        let captures = regex::Regex::new("test").unwrap().capture_locations();
        let loc = step::Location::new("file.rs", 10, 5);
        let error = StepError::NotFound;
        let step_failed = ScenarioFinished::StepFailed(Some(captures), Some(loc), error);
        
        match step_failed {
            ScenarioFinished::StepFailed(c, l, e) => {
                assert!(c.is_some());
                assert!(l.is_some());
                assert!(matches!(e, StepError::NotFound));
            },
            _ => panic!("Expected StepFailed"),
        }
    }

    #[test]
    fn test_scenario_finished_clone() {
        let variants = vec![
            ScenarioFinished::BeforeHookFailed(Arc::new("error".to_string())),
            ScenarioFinished::StepPassed,
            ScenarioFinished::StepSkipped,
            ScenarioFinished::StepFailed(None, None, StepError::NotFound),
        ];
        
        for variant in variants {
            let cloned = variant.clone();
            match (&variant, &cloned) {
                (ScenarioFinished::BeforeHookFailed(_), ScenarioFinished::BeforeHookFailed(_)) => {},
                (ScenarioFinished::StepPassed, ScenarioFinished::StepPassed) => {},
                (ScenarioFinished::StepSkipped, ScenarioFinished::StepSkipped) => {},
                (ScenarioFinished::StepFailed(..), ScenarioFinished::StepFailed(..)) => {},
                _ => panic!("Clone produced different variant"),
            }
        }
    }
}