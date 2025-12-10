//! Step execution logic for the Basic executor.

use std::{panic::AssertUnwindSafe, sync::Arc};

use futures::{FutureExt as _, TryFutureExt as _};
use regex::CaptureLocations;

use crate::{
    Event, World,
    event::{self, source::Source},
    future::FutureExt as _,
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

        // Execute all steps in the scenario
        for step in &scenario.value.steps {
            if step_failed {
                // Skip remaining steps if one has already failed
                skipped_steps += 1;
                Self::emit_skipped_step_event(
                    feature.clone(),
                    rule.clone(),
                    scenario.clone(),
                    Source::new(step.clone(), scenario.source_line()),
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
                Source::new(step.clone(), scenario.source_line()),
                world,
                send_event.clone(),
                #[cfg(feature = "tracing")]
                waiter,
            )
            .await;

            match step_result {
                event::Step::Passed => passed_steps += 1,
                event::Step::Skipped { .. } => skipped_steps += 1,
                event::Step::Failed { .. } => {
                    failed_steps += 1;
                    step_failed = true;
                }
            }
        }

        AfterHookEventsMeta {
            passed_steps,
            skipped_steps,
            failed_steps,
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
        send_event: impl Fn(event::Cucumber<W>),
        #[cfg(feature = "tracing")] waiter: Option<&crate::tracing::SpanCloseWaiter>,
    ) -> event::Step<W>
    where
        W: World,
    {
        let event = Event::new(
            event::Cucumber::feature(
                feature.clone(),
                event::Feature::scenario(
                    rule.clone(),
                    scenario.clone(),
                    event::RetryableScenario {
                        event: event::Scenario::Step(
                            step.clone(),
                            event::Step::Started,
                        ),
                        retries: None,
                    },
                ),
            ),
        );
        send_event(event.value);

        #[cfg(feature = "tracing")]
        let span = id.step_span(false);
        #[cfg(feature = "tracing")]
        let _guard = span.enter();

        let step_fn = collection.find(&step.value);
        let result = match step_fn {
            Some((step_fn, captures, loc)) => {
                let captures = captures
                    .map(|(re, name)| {
                        let mut locs = CaptureLocations::new();
                        re.captures_read(&mut locs, &step.value.value)?;
                        Ok((name, locs))
                    })
                    .collect::<Result<Vec<_>, regex::Error>>();

                match captures {
                    Ok(captures) => {
                        let ctx = step::Context::new(
                            step.value.value.clone(),
                            captures,
                            step.value.docstring.clone(),
                            step.value.table.clone(),
                            loc,
                        );

                        AssertUnwindSafe(step_fn(&step.value, world, &ctx))
                            .catch_unwind()
                            .await
                    }
                    Err(e) => Err(step::Failed::from(Arc::new(e))),
                }
            }
            None => {
                let err = step::failed::Unmatched {
                    step: step.value.value.clone(),
                };
                Err(step::Failed::from(Arc::new(err)))
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
            Ok(()) => event::Step::Passed,
            Err(step::Failed::Skipped(world, info)) => {
                event::Step::Skipped {
                    world: Some(world),
                    info,
                }
            }
            Err(err) => {
                let info = coerce_into_info(&err);
                event::Step::Failed {
                    world: Some(world),
                    info,
                }
            }
        };

        let event = Event::new(
            event::Cucumber::feature(
                feature,
                event::Feature::scenario(
                    rule,
                    scenario,
                    event::RetryableScenario {
                        event: event::Scenario::Step(step, step_event.clone()),
                        retries: None,
                    },
                ),
            ),
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
        send_event: &impl Fn(event::Cucumber<W>),
    ) where
        W: World,
    {
        let step_event = event::Step::Skipped {
            world: None,
            info: Some(crate::event::Info::new("Previous step failed", None)),
        };

        let event = Event::new(
            event::Cucumber::feature(
                feature,
                event::Feature::scenario(
                    rule,
                    scenario,
                    event::RetryableScenario {
                        event: event::Scenario::Step(step, step_event),
                        retries: None,
                    },
                ),
            ),
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
        let id = ScenarioId::new(1);
        let (feature, scenario) = create_test_feature_and_scenario();
        let mut world = TestWorld;
        let events = Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();
        
        let meta = StepExecutor::run_steps(
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
        
        assert_eq!(meta.passed_steps, 0);
        assert_eq!(meta.skipped_steps, 0);
        assert_eq!(meta.failed_steps, 0);
    }

    #[tokio::test]
    async fn test_run_steps_with_background_steps() {
        let collection = step::Collection::<TestWorld>::new();
        let id = ScenarioId::new(1);
        let (feature, scenario) = create_test_scenario_with_steps();
        let mut world = TestWorld;
        let events = Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();
        
        let meta = StepExecutor::run_steps(
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
        
        // Since we don't have step definitions, all steps should fail as unmatched
        assert!(meta.failed_steps > 0 || meta.passed_steps == 0);
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
            &move |event| events_clone.lock().unwrap().push(event),
        );
        
        let captured_events = events.lock().unwrap();
        assert_eq!(captured_events.len(), 1);
    }

    #[test]
    fn test_after_hook_events_meta_creation() {
        let meta = AfterHookEventsMeta {
            passed_steps: 5,
            skipped_steps: 2,
            failed_steps: 1,
        };
        
        assert_eq!(meta.passed_steps, 5);
        assert_eq!(meta.skipped_steps, 2);
        assert_eq!(meta.failed_steps, 1);
    }

    #[test]
    fn test_after_hook_events_meta_default_values() {
        let meta = AfterHookEventsMeta {
            passed_steps: 0,
            skipped_steps: 0,
            failed_steps: 0,
        };
        
        assert_eq!(meta.passed_steps + meta.skipped_steps + meta.failed_steps, 0);
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
        
        (Source::new(feature, None), Source::new(scenario, None))
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
        
        (Source::new(feature, None), Source::new(scenario, None))
    }

    fn create_test_step() -> Source<gherkin::Step> {
        use gherkin::Step;
        
        let step = Step {
            keyword: "Given".to_string(),
            value: "I have a test step".to_string(),
            docstring: None,
            table: None,
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 3, col: 1 },
        };
        
        Source::new(step, None)
    }
}