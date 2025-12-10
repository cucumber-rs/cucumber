//! Hook execution logic for the Basic executor.

use std::panic::AssertUnwindSafe;

use futures::{FutureExt as _, future::LocalBoxFuture};

use crate::{
    Event, World,
    event::{self, HookType, Info, source::Source},
};

use super::super::supporting_structures::{
    ScenarioId, ExecutionFailure, coerce_into_info,
};

/// Hook execution functionality for the Executor.
pub(super) struct HookExecutor;

impl HookExecutor {
    /// Runs a before hook if present.
    pub(super) async fn run_before_hook<W, Before>(
        hook: Option<&Before>,
        id: ScenarioId,
        feature: Source<gherkin::Feature>,
        rule: Option<Source<gherkin::Rule>>,
        scenario: Source<gherkin::Scenario>,
        world: &mut W,
        send_event: impl Fn(event::Cucumber<W>),
        #[cfg(feature = "tracing")] waiter: Option<&crate::tracing::SpanCloseWaiter>,
    ) -> Result<(), ExecutionFailure<W>>
    where
        W: World,
        Before: for<'a> Fn(
                &'a gherkin::Feature,
                Option<&'a gherkin::Rule>,
                &'a gherkin::Scenario,
                &'a mut W,
            ) -> LocalBoxFuture<'a, ()>,
    {
        if let Some(before_hook) = hook {
            let event = Event::new(
                event::Cucumber::scenario(
                    feature.clone(),
                    rule.clone(),
                    scenario.clone(),
                    event::RetryableScenario {
                        event: event::Scenario::Hook(HookType::Before, event::Hook::Started),
                        retries: None,
                    },
                )
            );
            send_event(event.value);

            #[cfg(feature = "tracing")]
            let span = id.hook_span(HookType::Before);
            #[cfg(feature = "tracing")]
            let _guard = span.enter();

            let result = AssertUnwindSafe(before_hook(
                &*feature,
                rule.as_ref().map(|r| &**r),
                &*scenario,
                world,
            ))
            .catch_unwind()
            .await;

            #[cfg(feature = "tracing")]
            {
                drop(_guard);
                if let Some(waiter) = waiter {
                    waiter.wait_for_span_close(span.id()).await;
                }
            }

            let (hook_event, should_fail) = match result {
                Ok(()) => (event::Hook::Passed, false),
                Err(err) => {
                    let info = coerce_into_info(err);
                    (event::Hook::Failed(None, info), true)
                }
            };

            let event = Event::new(
                event::Cucumber::scenario(
                    feature,
                    rule,
                    scenario,
                    event::RetryableScenario {
                        event: event::Scenario::Hook(HookType::Before, hook_event),
                        retries: None,
                    },
                )
            );
            send_event(event.value);

            if should_fail {
                return Err(ExecutionFailure::Before);
            }
        }

        Ok(())
    }

    /// Runs an after hook if present.
    pub(super) async fn run_after_hook<W, After>(
        hook: Option<&After>,
        id: ScenarioId,
        feature: Source<gherkin::Feature>,
        rule: Option<Source<gherkin::Rule>>,
        scenario: Source<gherkin::Scenario>,
        world: Option<&mut W>,
        scenario_finished: &event::ScenarioFinished,
        send_event: impl Fn(event::Cucumber<W>),
        #[cfg(feature = "tracing")] waiter: Option<&crate::tracing::SpanCloseWaiter>,
    ) -> Option<Info>
    where
        W: World,
        After: for<'a> Fn(
                &'a gherkin::Feature,
                Option<&'a gherkin::Rule>,
                &'a gherkin::Scenario,
                &'a event::ScenarioFinished,
                Option<&'a mut W>,
            ) -> LocalBoxFuture<'a, ()>,
    {
        if let Some(after_hook) = hook {
            let event = Event::new(
                event::Cucumber::scenario(
                    feature.clone(),
                    rule.clone(),
                    scenario.clone(),
                    event::RetryableScenario {
                        event: event::Scenario::Hook(HookType::After, event::Hook::Started),
                        retries: None,
                    },
                )
            );
            send_event(event.value);

            #[cfg(feature = "tracing")]
            let span = id.hook_span(HookType::After);
            #[cfg(feature = "tracing")]
            let _guard = span.enter();

            let result = AssertUnwindSafe(after_hook(
                &*feature,
                rule.as_ref().map(|r| &**r),
                &*scenario,
                scenario_finished,
                world,
            ))
            .catch_unwind()
            .await;

            #[cfg(feature = "tracing")]
            {
                drop(_guard);
                if let Some(waiter) = waiter {
                    waiter.wait_for_span_close(span.id()).await;
                }
            }

            let (hook_event, error) = match result {
                Ok(()) => (event::Hook::Passed, None),
                Err(err) => {
                    let info = coerce_into_info(err);
                    (event::Hook::Failed(None, info.clone()), Some(info))
                }
            };

            let event = Event::new(
                event::Cucumber::scenario(
                    feature,
                    rule,
                    scenario,
                    event::RetryableScenario {
                        event: event::Scenario::Hook(HookType::After, hook_event),
                        retries: None,
                    },
                )
            );
            send_event(event.value);

            return error;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{event, test_utils::common::TestWorld};
    use std::sync::Mutex;

    #[tokio::test]
    async fn test_run_before_hook_none() {
        let id = ScenarioId::new();
        let (feature, scenario) = create_test_feature_and_scenario();
        let mut world = TestWorld;
        let events = Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();
        
        let result = HookExecutor::run_before_hook(
            None::<&for<'a> fn(&'a gherkin::Feature, Option<&'a gherkin::Rule>, &'a gherkin::Scenario, &'a mut TestWorld) -> LocalBoxFuture<'a, ()>>,
            id,
            feature,
            None,
            scenario,
            &mut world,
            move |event| events_clone.lock().unwrap().push(event),
            #[cfg(feature = "tracing")]
            None,
        ).await;
        
        assert!(result.is_ok());
        assert!(events.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_run_before_hook_success() {
        let id = ScenarioId::new();
        let (feature, scenario) = create_test_feature_and_scenario();
        let mut world = TestWorld;
        let events = Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();
        
        fn before_hook<'a>(_: &'a gherkin::Feature, _: Option<&'a gherkin::Rule>, _: &'a gherkin::Scenario, _: &'a mut TestWorld) -> LocalBoxFuture<'a, ()> {
            Box::pin(async {})
        }
        
        let result = HookExecutor::run_before_hook(
            Some(&before_hook),
            id,
            feature,
            None,
            scenario,
            &mut world,
            move |event| events_clone.lock().unwrap().push(event),
            #[cfg(feature = "tracing")]
            None,
        ).await;
        
        assert!(result.is_ok());
        let captured_events = events.lock().unwrap();
        assert_eq!(captured_events.len(), 2); // Started and Passed events
    }

    #[tokio::test]
    async fn test_run_after_hook_none() {
        let id = ScenarioId::new();
        let (feature, scenario) = create_test_feature_and_scenario();
        let mut world = TestWorld;
        let events = Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();
        
        let scenario_finished = event::ScenarioFinished::StepPassed;
        
        HookExecutor::run_after_hook(
            None::<&for<'a> fn(&'a gherkin::Feature, Option<&'a gherkin::Rule>, &'a gherkin::Scenario, &'a event::ScenarioFinished, Option<&'a mut TestWorld>) -> LocalBoxFuture<'a, ()>>,
            id,
            feature,
            None,
            scenario,
            Some(&mut world),
            &scenario_finished,
            move |event| events_clone.lock().unwrap().push(event),
            #[cfg(feature = "tracing")]
            None,
        ).await;
        
        assert!(events.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_run_after_hook_success() {
        let id = ScenarioId::new();
        let (feature, scenario) = create_test_feature_and_scenario();
        let mut world = TestWorld;
        let events = Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();
        
        let scenario_finished = event::ScenarioFinished::StepPassed;
        
        fn after_hook<'a>(_: &'a gherkin::Feature, _: Option<&'a gherkin::Rule>, _: &'a gherkin::Scenario, _: &'a event::ScenarioFinished, _: Option<&'a mut TestWorld>) -> LocalBoxFuture<'a, ()> {
            Box::pin(async {})
        }
        
        HookExecutor::run_after_hook(
            Some(&after_hook),
            id,
            feature,
            None,
            scenario,
            Some(&mut world),
            &scenario_finished,
            move |event| events_clone.lock().unwrap().push(event),
            #[cfg(feature = "tracing")]
            None,
        ).await;
        
        let captured_events = events.lock().unwrap();
        assert_eq!(captured_events.len(), 2); // Started and Passed events
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
}