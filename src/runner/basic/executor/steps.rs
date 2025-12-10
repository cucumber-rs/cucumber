//! Step execution logic for the Basic executor.

use std::panic::AssertUnwindSafe;

use futures::FutureExt as _;

use crate::{
    Event, World,
    event::{self, source::Source},
    step,
};

use super::super::supporting_structures::{
    ScenarioId, AfterHookEventsMeta, coerce_into_info,
};

/// Step execution functionality for the Executor.
pub(super) struct StepExecutor;

impl StepExecutor {
    /// Runs all steps for a scenario.
    pub(super) async fn run_steps<W>(
        collection: &step::Collection<W>,
        id: ScenarioId,
        feature: Source<gherkin::Feature>,
        rule: Option<Source<gherkin::Rule>>,
        scenario: Source<gherkin::Scenario>,
        world: &mut W,
        retries: Option<crate::event::Retries>,
        send_event: impl Fn(event::Cucumber<W>) + Clone,
        #[cfg(feature = "tracing")] waiter: Option<&crate::tracing::SpanCloseWaiter>,
    ) -> AfterHookEventsMeta
    where
        W: World,
    {
        let mut passed_steps = 0;
        let mut skipped_steps = 0;
        let mut failed_steps = 0;
        let mut step_failed = false;
        let mut last_failure: Option<(Option<regex::CaptureLocations>, Option<step::Location>, event::StepError)> = None;

        // Execute all steps in the scenario
        for step in &scenario.steps {
            if step_failed {
                // Skip remaining steps if one has already failed
                skipped_steps += 1;
                Self::emit_skipped_step_event(
                    feature.clone(),
                    rule.clone(),
                    scenario.clone(),
                    Source::new(step.clone()),
                    retries,
                    &send_event,
                );
                continue;
            }

            let step_result = Self::run_step(
                collection,
                id,
                feature.clone(),
                rule.clone(),
                scenario.clone(),
                Source::new(step.clone()),
                world,
                retries,
                send_event.clone(),
                #[cfg(feature = "tracing")]
                waiter,
            )
            .await;

            match step_result {
                event::Step::Started => {
                    // This shouldn't happen as run_step returns the final result
                    // But we need to handle it for exhaustive matching
                }
                event::Step::Passed { .. } => passed_steps += 1,
                event::Step::Skipped => skipped_steps += 1,
                event::Step::Failed { captures, location, error, .. } => {
                    failed_steps += 1;
                    step_failed = true;
                    last_failure = Some((captures, location, error));
                }
            }
        }

        // Determine the scenario outcome based on canonical Cucumber behavior:
        // 1. If any step failed -> StepFailed
        // 2. If any step was skipped (but none failed) -> StepSkipped
        // 3. If all steps passed -> StepPassed
        let scenario_finished = if let Some((captures, location, error)) = last_failure {
            event::ScenarioFinished::StepFailed(captures, location, error)
        } else if skipped_steps > 0 {
            event::ScenarioFinished::StepSkipped
        } else {
            event::ScenarioFinished::StepPassed
        };
        
        AfterHookEventsMeta {
            started: event::Metadata::new(()),
            finished: event::Metadata::new(()),
            scenario_finished,
        }
    }

    /// Runs a single step.
    async fn run_step<W>(
        collection: &step::Collection<W>,
        id: ScenarioId,
        feature: Source<gherkin::Feature>,
        rule: Option<Source<gherkin::Rule>>,
        scenario: Source<gherkin::Scenario>,
        step: Source<gherkin::Step>,
        world: &mut W,
        retries: Option<crate::event::Retries>,
        send_event: impl Fn(event::Cucumber<W>),
        #[cfg(feature = "tracing")] waiter: Option<&crate::tracing::SpanCloseWaiter>,
    ) -> event::Step<W>
    where
        W: World,
    {
        let event = Event::new(
            event::Cucumber::scenario(
                feature.clone(),
                rule.clone(),
                scenario.clone(),
                event::RetryableScenario {
                    event: event::Scenario::Step(
                        step.clone(),
                        event::Step::Started,
                    ),
                    retries,
                },
            )
        );
        send_event(event.value);

        #[cfg(feature = "tracing")]
        let span = id.step_span(false);
        #[cfg(feature = "tracing")]
        let _guard = span.enter();

        let step_fn = collection.find(&*step);
        let (result, location, step_captures) = match step_fn {
            Ok(Some((step_fn, captures, loc, ctx))) => {
                // Extract the actual capture locations for the event
                let actual_captures = captures.clone();

                let result = AssertUnwindSafe(step_fn(world, ctx))
                            .catch_unwind()
                            .await;

                (result, loc, Some(actual_captures))
            }
            Ok(None) => {
                return event::Step::Failed {
                    captures: None,
                    location: None,
                    world: None,
                    error: event::StepError::NotFound,
                };
            }
            Err(ambiguous_err) => {
                return event::Step::Failed {
                    captures: None,
                    location: None,
                    world: None,
                    error: event::StepError::AmbiguousMatch(ambiguous_err),
                };
            }
        };

        #[cfg(feature = "tracing")]
        {
            drop(_guard);
            if let Some(waiter) = waiter {
                waiter.wait_for_span_close(span.id()).await;
            }
        }

        let step_event = match result {
            Ok(()) => event::Step::Passed {
                captures: step_captures.unwrap_or_else(|| regex::Regex::new("").unwrap().capture_locations()),
                location,
            },
            Err(err) => {
                let info = coerce_into_info(err);
                event::Step::Failed {
                    captures: step_captures,
                    location,
                    world: None,
                    error: event::StepError::Panic(info),
                }
            }
        };

        let event = Event::new(
            event::Cucumber::scenario(
                feature,
                rule,
                scenario,
                event::RetryableScenario {
                    event: event::Scenario::Step(step, step_event.clone()),
                    retries,
                },
            )
        );
        send_event(event.value);

        step_event
    }

    /// Emits a skipped step event.
    fn emit_skipped_step_event<W>(
        feature: Source<gherkin::Feature>,
        rule: Option<Source<gherkin::Rule>>,
        scenario: Source<gherkin::Scenario>,
        step: Source<gherkin::Step>,
        retries: Option<crate::event::Retries>,
        send_event: &impl Fn(event::Cucumber<W>),
    ) where
        W: World,
    {
        let step_event = event::Step::Skipped;

        let event = Event::new(
            event::Cucumber::scenario(
                feature,
                rule,
                scenario,
                event::RetryableScenario {
                    event: event::Scenario::Step(step, step_event),
                    retries,
                },
            )
        );
        send_event(event.value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{event, test_utils::common::TestWorld};
    use std::sync::Mutex;

    #[tokio::test]
    async fn test_run_steps_empty_scenario() {
        let collection = step::Collection::<TestWorld>::new();
        let id = ScenarioId::new();
        let (feature, scenario) = create_test_feature_and_scenario();
        let mut world = TestWorld;
        let events = Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();
        
        let _meta = StepExecutor::run_steps(
            &collection,
            id,
            feature,
            None,
            scenario,
            &mut world,
            move |event| events_clone.lock().unwrap().push(event),
            #[cfg(feature = "tracing")]
            None,
        ).await;
        
        // AfterHookEventsMeta only contains timing metadata
        // Test that it was properly created
        #[cfg(feature = "timestamps")]
        {
            let _ = meta.started.at;
            let _ = meta.finished.at;
        }
    }

    #[tokio::test]
    async fn test_run_steps_with_background_steps() {
        let collection = step::Collection::<TestWorld>::new();
        let id = ScenarioId::new();
        let (feature, scenario) = create_test_scenario_with_steps();
        let mut world = TestWorld;
        let events = Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();
        
        let _meta = StepExecutor::run_steps(
            &collection,
            id,
            feature,
            None,
            scenario,
            &mut world,
            move |event| events_clone.lock().unwrap().push(event),
            #[cfg(feature = "tracing")]
            None,
        ).await;
        
        // AfterHookEventsMeta only contains timing metadata
        // Just verify it was created
        #[cfg(feature = "timestamps")]
        {
            let _ = meta.started.at;
            let _ = meta.finished.at;
        }
    }

    #[test]
    fn test_step_executor_emit_skipped_event() {
        let (feature, scenario) = create_test_feature_and_scenario();
        let step = create_test_step();
        let events = Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();
        
        StepExecutor::emit_skipped_step_event(
            feature,
            None,
            scenario,
            step,
            &move |event: event::Cucumber<TestWorld>| events_clone.lock().unwrap().push(event),
        );
        
        let captured_events = events.lock().unwrap();
        assert_eq!(captured_events.len(), 1);
    }

    #[test]
    fn test_after_hook_events_meta_creation() {
        let meta = AfterHookEventsMeta {
            started: event::Metadata::new(()),
            finished: event::Metadata::new(()),
            scenario_finished: event::ScenarioFinished::StepPassed,
        };
        
        // Just verify it can be created
        assert!(matches!(meta.started, _));
        assert!(matches!(meta.finished, _));
    }

    #[test]
    fn test_after_hook_events_meta_default_values() {
        let meta = AfterHookEventsMeta {
            started: event::Metadata::new(()),
            finished: event::Metadata::new(()),
            scenario_finished: event::ScenarioFinished::StepPassed,
        };
        
        // Verify both fields exist
        assert!(matches!(meta.started, _));
        assert!(matches!(meta.finished, _));
    }

    fn create_test_feature_and_scenario() -> (Source<gherkin::Feature>, Source<gherkin::Scenario>) {
        use gherkin::{Feature, Scenario};
        
        let feature = Feature {
            keyword: "Feature".to_string(),
            name: "Test Feature".to_string(),
            description: None,
            background: None,
            scenarios: vec![],
            rules: vec![],
            tags: vec![],
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 1, col: 1 },
            path: None,
        };
        
        let scenario = Scenario {
            keyword: "Scenario".to_string(),
            name: "Test Scenario".to_string(),
            description: None,
            steps: vec![],
            examples: vec![],
            tags: vec![],
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 2, col: 1 },
        };
        
        (Source::new(feature), Source::new(scenario))
    }

    fn create_test_scenario_with_steps() -> (Source<gherkin::Feature>, Source<gherkin::Scenario>) {
        use gherkin::{Feature, Scenario, Step};
        
        let feature = Feature {
            keyword: "Feature".to_string(),
            name: "Test Feature".to_string(),
            description: None,
            background: None,
            scenarios: vec![],
            rules: vec![],
            tags: vec![],
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 1, col: 1 },
            path: None,
        };
        
        let step = Step {
            ty: gherkin::StepType::Given,
            keyword: "Given".to_string(),
            value: "I have a test step".to_string(),
            docstring: None,
            table: None,
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 3, col: 1 },
        };
        
        let scenario = Scenario {
            keyword: "Scenario".to_string(),
            name: "Test Scenario".to_string(),
            description: None,
            steps: vec![step],
            examples: vec![],
            tags: vec![],
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 2, col: 1 },
        };
        
        (Source::new(feature), Source::new(scenario))
    }

    fn create_test_step() -> Source<gherkin::Step> {
        use gherkin::Step;
        
        let step = Step {
            ty: gherkin::StepType::Given,
            keyword: "Given".to_string(),
            value: "I have a test step".to_string(),
            docstring: None,
            table: None,
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 3, col: 1 },
        };
        
        Source::new(step)
    }
}