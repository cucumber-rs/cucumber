//! Hook execution logic for the Basic executor.

use std::{panic::AssertUnwindSafe, sync::Arc};

use futures::{FutureExt as _, TryFutureExt as _};

use crate::{
    Event, World,
    event::{self, HookType, Info, source::Source},
    future::FutureExt as _,
    step,
};

use super::super::{
    supporting_structures::{
        ScenarioId, ExecutionFailure, AfterHookEventsMeta, IsFailed, IsRetried,
        coerce_into_info,
    },
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
            ) -> futures::future::LocalBoxFuture<'_, ()>
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
            ) -> futures::future::LocalBoxFuture<'_, ()>
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