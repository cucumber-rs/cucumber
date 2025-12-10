//! Hook execution logic for the Basic executor.

use std::{panic::AssertUnwindSafe, sync::Arc};

use futures::{FutureExt as _, TryFutureExt as _, future::LocalBoxFuture};

use crate::{
    Event, World,
    event::{self, HookType, Info, source::Source},
    future::FutureExt as _,
    step,
};

use super::super::supporting_structures::{
    ScenarioId, ExecutionFailure, AfterHookEventsMeta, coerce_into_info,
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
        Before: Fn(
                &gherkin::Feature,
                Option<&gherkin::Rule>,
                &gherkin::Scenario,
                &mut W,
            ) -> LocalBoxFuture<'_, ()>
            + Send
            + Sync,
    {
        if let Some(before_hook) = hook {
            let event = Event::new(
                event::Cucumber::feature(
                    feature.clone(),
                    event::Feature::scenario(
                        rule.clone(),
                        scenario.clone(),
                        event::RetryableScenario {
                            event: event::Scenario::Hook(HookType::Before, event::Hook::Started),
                            retries: None,
                        },
                    ),
                ),
            );
            send_event(event.value);

            #[cfg(feature = "tracing")]
            let span = id.hook_span(HookType::Before);
            #[cfg(feature = "tracing")]
            let _guard = span.enter();

            let result = AssertUnwindSafe(before_hook(
                &feature.value,
                rule.as_ref().map(|r| &r.value),
                &scenario.value,
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

            let hook_event = match result {
                Ok(()) => event::Hook::Passed,
                Err(err) => {
                    let info = coerce_into_info(&err);
                    event::Hook::Failed { 
                        world: Some(world), 
                        info 
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
                            event: event::Scenario::Hook(HookType::Before, hook_event),
                            retries: None,
                        },
                    ),
                ),
            );
            send_event(event.value);

            if let event::Hook::Failed { .. } = hook_event {
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
        meta: AfterHookEventsMeta,
        send_event: impl Fn(event::Cucumber<W>),
        #[cfg(feature = "tracing")] waiter: Option<&crate::tracing::SpanCloseWaiter>,
    ) where
        W: World,
        After: Fn(
                &gherkin::Feature,
                Option<&gherkin::Rule>,
                &gherkin::Scenario,
                &event::ScenarioFinished,
                Option<&mut W>,
            ) -> LocalBoxFuture<'_, ()>
            + Send
            + Sync,
    {
        if let Some(after_hook) = hook {
            let event = Event::new(
                event::Cucumber::feature(
                    feature.clone(),
                    event::Feature::scenario(
                        rule.clone(),
                        scenario.clone(),
                        event::RetryableScenario {
                            event: event::Scenario::Hook(HookType::After, event::Hook::Started),
                            retries: None,
                        },
                    ),
                ),
            );
            send_event(event.value);

            #[cfg(feature = "tracing")]
            let span = id.hook_span(HookType::After);
            #[cfg(feature = "tracing")]
            let _guard = span.enter();

            let finished = event::ScenarioFinished {
                passed_steps: meta.passed_steps,
                skipped_steps: meta.skipped_steps,
                failed_steps: meta.failed_steps,
            };

            let result = AssertUnwindSafe(after_hook(
                &feature.value,
                rule.as_ref().map(|r| &r.value),
                &scenario.value,
                &finished,
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

            let hook_event = match result {
                Ok(()) => event::Hook::Passed,
                Err(err) => {
                    let info = coerce_into_info(&err);
                    event::Hook::Failed { 
                        world, 
                        info 
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
                            event: event::Scenario::Hook(HookType::After, hook_event),
                            retries: None,
                        },
                    ),
                ),
            );
            send_event(event.value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{event, test_utils::common::TestWorld};
    use futures::future::BoxFuture;
    use std::sync::Mutex;

    #[tokio::test]
    async fn test_run_before_hook_none() {
        let id = ScenarioId::new(1);
        let (feature, scenario) = create_test_feature_and_scenario();
        let mut world = TestWorld;
        let events = Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();
        
        let result = HookExecutor::run_before_hook(
            None::<&fn(&gherkin::Feature, Option<&gherkin::Rule>, &gherkin::Scenario, &mut TestWorld) -> BoxFuture<'_, ()>>,
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
        let id = ScenarioId::new(1);
        let (feature, scenario) = create_test_feature_and_scenario();
        let mut world = TestWorld;
        let events = Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();
        
        let before_hook = |_: &gherkin::Feature, _: Option<&gherkin::Rule>, _: &gherkin::Scenario, _: &mut TestWorld| {
            Box::pin(async {}) as BoxFuture<'_, ()>
        };
        
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
        let id = ScenarioId::new(1);
        let (feature, scenario) = create_test_feature_and_scenario();
        let mut world = TestWorld;
        let events = Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();
        
        let meta = AfterHookEventsMeta {
            passed_steps: 1,
            skipped_steps: 0,
            failed_steps: 0,
        };
        
        HookExecutor::run_after_hook(
            None::<&fn(&gherkin::Feature, Option<&gherkin::Rule>, &gherkin::Scenario, &event::ScenarioFinished, Option<&mut TestWorld>) -> BoxFuture<'_, ()>>,
            id,
            feature,
            None,
            scenario,
            Some(&mut world),
            meta,
            move |event| events_clone.lock().unwrap().push(event),
            #[cfg(feature = "tracing")]
            None,
        ).await;
        
        assert!(events.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_run_after_hook_success() {
        let id = ScenarioId::new(1);
        let (feature, scenario) = create_test_feature_and_scenario();
        let mut world = TestWorld;
        let events = Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();
        
        let meta = AfterHookEventsMeta {
            passed_steps: 1,
            skipped_steps: 0,
            failed_steps: 0,
        };
        
        let after_hook = |_: &gherkin::Feature, _: Option<&gherkin::Rule>, _: &gherkin::Scenario, _: &event::ScenarioFinished, _: Option<&mut TestWorld>| {
            Box::pin(async {}) as BoxFuture<'_, ()>
        };
        
        HookExecutor::run_after_hook(
            Some(&after_hook),
            id,
            feature,
            None,
            scenario,
            Some(&mut world),
            meta,
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
        
        (Source::new(feature, None), Source::new(scenario, None))
    }
}