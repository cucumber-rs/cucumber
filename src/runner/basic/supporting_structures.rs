//! Supporting structures and utilities for the Basic runner.

use std::{
    any::Any,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use derive_more::with_trait::{Display, FromStr};
use regex::CaptureLocations;

use crate::{
    event::{self, Info, Metadata, StepError},
    event::source::Source,
    step,
};

use std::collections::HashMap;

/// ID of a [`Scenario`], uniquely identifying it.
///
/// **NOTE**: Retried [`Scenario`] has a different ID from a failed one.
///
/// [`Scenario`]: gherkin::Scenario
#[derive(Clone, Copy, Debug, Display, Eq, FromStr, Hash, PartialEq)]
pub struct ScenarioId(pub u64);

impl ScenarioId {
    /// Creates a new unique [`ScenarioId`].
    pub fn new() -> Self {
        /// [`AtomicU64`] ID.
        static ID: AtomicU64 = AtomicU64::new(0);

        Self(ID.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for ScenarioId {
    fn default() -> Self {
        Self::new()
    }
}


/// Alias for a failed [`Scenario`].
///
/// [`Scenario`]: gherkin::Scenario
pub(super) type IsFailed = bool;

/// Alias for a retried [`Scenario`].
///
/// [`Scenario`]: gherkin::Scenario
pub(super) type IsRetried = bool;

/// Failure encountered during execution of [`HookType::Before`] or [`Step`].
/// See [`Executor::emit_failed_events()`] for more info.
///
/// [`Executor::emit_failed_events()`]: super::executor::Executor::emit_failed_events
/// [`Step`]: gherkin::Step
#[derive(Debug)]
pub(super) enum ExecutionFailure<World> {
    /// [`HookType::Before`] panicked.
    BeforeHookPanicked {
        /// [`World`] at the time [`HookType::Before`] has panicked.
        world: Option<World>,

        /// [`catch_unwind()`] of the [`HookType::Before`] panic.
        ///
        /// [`catch_unwind()`]: std::panic::catch_unwind
        panic_info: Info,

        /// [`Metadata`] at the time [`HookType::Before`] panicked.
        meta: Metadata,
    },

    /// [`Step`] was skipped.
    ///
    /// [`Step`]: gherkin::Step.
    StepSkipped(Option<World>),

    /// [`Step`] failed.
    ///
    /// [`Step`]: gherkin::Step.
    StepPanicked {
        /// [`World`] at the time when [`Step`] has failed.
        ///
        /// [`Step`]: gherkin::Step
        world: Option<World>,

        /// [`Step`] itself.
        ///
        /// [`Step`]: gherkin::Step
        step: Source<gherkin::Step>,

        /// [`Step`]s [`regex`] [`CaptureLocations`].
        ///
        /// [`Step`]: gherkin::Step
        captures: Option<CaptureLocations>,

        /// [`Location`] of the [`fn`] that matched this [`Step`].
        ///
        /// [`Location`]: step::Location
        /// [`Step`]: gherkin::Step
        loc: Option<step::Location>,

        /// [`StepError`] of the [`Step`].
        ///
        /// [`Step`]: gherkin::Step
        /// [`StepError`]: event::StepError
        err: event::StepError,

        /// [`Metadata`] at the time when [`Step`] failed.
        ///
        /// [`Step`]: gherkin::Step.
        meta: Metadata,

        /// Indicator whether the [`Step`] was background or not.
        ///
        /// [`Step`]: gherkin::Step
        is_background: bool,
    },

    /// [`HookType::Before`] failed.
    Before,
}

impl<W> ExecutionFailure<W> {
    /// Takes the [`World`] leaving a [`None`] in its place.
    pub(super) const fn take_world(&mut self) -> Option<W> {
        match self {
            Self::BeforeHookPanicked { world, .. }
            | Self::StepSkipped(world)
            | Self::StepPanicked { world, .. } => world.take(),
            Self::Before => None,
        }
    }

    /// Creates an [`event::ScenarioFinished`] from this [`ExecutionFailure`].
    pub(super) fn get_scenario_finished_event(&self) -> event::ScenarioFinished {
        use event::ScenarioFinished::{
            BeforeHookFailed, StepFailed, StepSkipped,
        };

        match self {
            Self::BeforeHookPanicked { panic_info, .. } => {
                BeforeHookFailed(Arc::clone(panic_info))
            }
            Self::StepSkipped(_) => StepSkipped,
            Self::StepPanicked { captures, loc, err, .. } => {
                StepFailed(captures.clone(), *loc, err.clone())
            }
            Self::Before => BeforeHookFailed(Arc::new("Before hook failed")),
        }
    }
}


/// [`Metadata`] of [`HookType::After`] events.
pub(super) struct AfterHookEventsMeta {
    /// [`Metadata`] at the time [`HookType::After`] started.
    pub(super) started: Metadata,

    /// [`Metadata`] at the time [`HookType::After`] finished.
    pub(super) finished: Metadata,
    
    /// The outcome of the scenario execution.
    pub(super) scenario_finished: event::ScenarioFinished,
}

/// Coerces the given `value` into a type-erased [`Info`].
pub(super) fn coerce_into_info<T: Any + Send + 'static>(val: T) -> Info {
    Arc::new(val)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_id_creation() {
        let id1 = ScenarioId::new();
        let id2 = ScenarioId::new();
        
        assert_ne!(id1, id2);
        assert!(id2.0 > id1.0);
    }

    #[test]
    fn test_scenario_id_default() {
        let id1 = ScenarioId::default();
        let id2 = ScenarioId::default();
        
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_scenario_id_display() {
        let id = ScenarioId(42);
        assert_eq!(format!("{}", id), "42");
    }

    #[test]
    fn test_scenario_id_hash() {
        use std::collections::HashMap;
        
        let id1 = ScenarioId(1);
        let id2 = ScenarioId(2);
        
        let mut map = HashMap::new();
        map.insert(id1, "first");
        map.insert(id2, "second");
        
        assert_eq!(map.get(&id1), Some(&"first"));
        assert_eq!(map.get(&id2), Some(&"second"));
    }

    #[cfg(feature = "tracing")]
    #[test]
    fn test_scenario_tracing_spans() {
        let id = ScenarioId(123);
        
        let scenario_span = id.scenario_span();
        let step_span = id.step_span(false);
        let bg_step_span = id.step_span(true);
        let before_hook_span = id.hook_span(event::HookType::Before);
        let after_hook_span = id.hook_span(event::HookType::After);
        
        // Just test that spans are created without errors
        assert_eq!(scenario_span.metadata().unwrap().name(), "Scenario");
        assert_eq!(step_span.metadata().unwrap().name(), "Step");
        assert_eq!(bg_step_span.metadata().unwrap().name(), "Background Step");
        assert_eq!(before_hook_span.metadata().unwrap().name(), "Before Hook");
        assert_eq!(after_hook_span.metadata().unwrap().name(), "After Hook");
    }

    #[test]
    fn test_execution_failure_world_take() {
        #[derive(Debug, PartialEq)]
        struct TestWorld(i32);
        
        let mut failure = ExecutionFailure::StepSkipped(Some(TestWorld(42)));
        let world = failure.take_world();
        
        assert_eq!(world, Some(TestWorld(42)));
        assert_eq!(failure.take_world(), None); // Should be None after taking
    }

    #[test]
    fn test_execution_failure_scenario_finished_event() {
        use event::ScenarioFinished;
        
        let failure = ExecutionFailure::<()>::StepSkipped(None);
        let event = failure.get_scenario_finished_event();
        
        assert!(matches!(event, ScenarioFinished::StepSkipped));
    }

    #[test]
    fn test_coerce_into_info() {
        let info = coerce_into_info("test string");
        
        // Should be able to downcast back
        assert!(info.downcast_ref::<&str>().is_some());
    }

    #[test]
    fn test_after_hook_events_meta() {
        let _meta = AfterHookEventsMeta {
            started: Metadata::new(()),
            finished: Metadata::new(()),
            scenario_finished: event::ScenarioFinished::StepPassed,
        };
        
        // Just test that structure can be created and has timing info
        #[cfg(feature = "timestamps")]
        {
            let started_time = meta.started.at;
            let finished_time = meta.finished.at;
            assert!(started_time.elapsed().as_nanos() > 0);
            assert!(finished_time.elapsed().as_nanos() > 0);
        }
    }

    #[test]
    fn test_execution_failure_before_hook_panicked() {
        let info = coerce_into_info("panic message");
        let meta = Metadata::new(());
        
        let failure = ExecutionFailure::<i32>::BeforeHookPanicked {
            world: Some(42),
            panic_info: info.clone(),
            meta,
        };
        
        match failure {
            ExecutionFailure::BeforeHookPanicked { world, panic_info, .. } => {
                assert_eq!(world, Some(42));
                assert!(panic_info.downcast_ref::<&str>().is_some());
            }
            _ => panic!("Expected BeforeHookPanicked variant"),
        }
    }

    #[test]
    fn test_execution_failure_step_panicked() {
        use crate::event::source::Source;
        
        let step = Source::new(gherkin::Step {
            ty: gherkin::StepType::Given,
            value: "test step".to_string(),
            docstring: None,
            table: None,
            span: gherkin::Span { start: 0, end: 0 },
            keyword: "Given".to_string(),
            position: gherkin::LineCol { line: 1, col: 1 },
        });
        
        let failure = ExecutionFailure::<()>::StepPanicked {
            world: None,
            step,
            captures: None,
            loc: None,
            err: event::StepError::Panic(coerce_into_info("step panic")),
            meta: Metadata::new(()),
            is_background: false,
        };
        
        match failure {
            ExecutionFailure::StepPanicked { is_background, .. } => {
                assert!(!is_background);
            }
            _ => panic!("Expected StepPanicked variant"),
        }
    }
}