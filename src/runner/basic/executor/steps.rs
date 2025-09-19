//! Step execution logic for the Basic executor.

use std::{panic::AssertUnwindSafe, sync::Arc};

use futures::{FutureExt as _, TryFutureExt as _};
use regex::CaptureLocations;

use crate::{
    Event, World,
    event::{self, HookType, Info, source::Source},
    future::FutureExt as _,
    step,
};

use super::super::{
    supporting_structures::{
        ScenarioId, ExecutionFailure, coerce_into_info,
    },
};

/// Step execution functionality for the Executor.
pub(super) struct StepExecutor;

impl StepExecutor {
    /// Runs a single step.
    pub(super) async fn run_step<W, St, Ps, Sk>(
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
        St: Fn(&gherkin::Step, &mut W, &step::Context) -> step::Result<W>,
        Ps: Fn(&gherkin::Step, &mut W, &step::Context) -> step::Result<W>,
        Sk: Fn(&gherkin::Step, &mut W, &step::Context) -> step::Result<W>,
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
}