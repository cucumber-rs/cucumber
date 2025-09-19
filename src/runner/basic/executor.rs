//! Scenario execution logic for the Basic runner.

use std::{
    panic::{self, AssertUnwindSafe},
    sync::Arc,
};

use futures::{
    FutureExt as _, StreamExt as _, TryFutureExt as _, TryStreamExt as _,
    channel::mpsc,
    future::LocalBoxFuture,
    stream,
};
use regex::CaptureLocations;

#[cfg(feature = "tracing")]
use crate::tracing::SpanCloseWaiter;
use crate::{
    Event, World,
    event::{self, HookType, Info, Retries, source::Source},
    future::FutureExt as _,
    parser, step,
};

use super::{
    cli_and_types::{RetryOptions, ScenarioType},
    scenario_storage::{Features, FinishedFeaturesSender},
    supporting_structures::{
        ScenarioId, ExecutionFailure, AfterHookEventsMeta, IsFailed, IsRetried,
        coerce_into_info,
    },
};

/// Runs [`Scenario`]s and notifies about their state of completion.
///
/// [`Scenario`]: gherkin::Scenario
pub struct Executor<W, Before, After> {
    /// [`Step`]s [`Collection`].
    ///
    /// [`Collection`]: step::Collection
    collection: step::Collection<W>,

    /// Function, executed on each [`Scenario`] before running all [`Step`]s,
    /// including [`Background`] ones.
    ///
    /// [`Background`]: gherkin::Background
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
    before_hook: Option<Before>,

    /// Function, executed on each [`Scenario`] after running all [`Step`]s.
    ///
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
    after_hook: Option<After>,

    /// Sender for [`Scenario`] [events][1].
    ///
    /// [`Scenario`]: gherkin::Scenario
    /// [1]: event::Scenario
    event_sender:
        mpsc::UnboundedSender<parser::Result<Event<event::Cucumber<W>>>>,

    /// Sender for notifying of [`Scenario`]s completion.
    ///
    /// [`Scenario`]: gherkin::Scenario
    finished_sender: FinishedFeaturesSender,

    /// [`Scenario`]s storage.
    ///
    /// [`Scenario`]: gherkin::Scenario
    storage: Features,
}

impl<W: World, Before, After> Executor<W, Before, After>
where
    Before: 'static
        + for<'a> Fn(
            &'a gherkin::Feature,
            Option<&'a gherkin::Rule>,
            &'a gherkin::Scenario,
            &'a mut W,
        ) -> LocalBoxFuture<'a, ()>,
    After: 'static
        + for<'a> Fn(
            &'a gherkin::Feature,
            Option<&'a gherkin::Rule>,
            &'a gherkin::Scenario,
            &'a event::ScenarioFinished,
            Option<&'a mut W>,
        ) -> LocalBoxFuture<'a, ()>,
{
    /// Creates a new [`Executor`].
    pub const fn new(
        collection: step::Collection<W>,
        before_hook: Option<Before>,
        after_hook: Option<After>,
        event_sender: mpsc::UnboundedSender<
            parser::Result<Event<event::Cucumber<W>>>,
        >,
        finished_sender: FinishedFeaturesSender,
        storage: Features,
    ) -> Self {
        Self {
            collection,
            before_hook,
            after_hook,
            event_sender,
            finished_sender,
            storage,
        }
    }

    /// Runs a [`Scenario`].
    ///
    /// # Events
    ///
    /// - Emits all [`Scenario`] events.
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Rule`]: gherkin::Rule
    /// [`Scenario`]: gherkin::Scenario
    #[cfg_attr(
        feature = "tracing",
        expect(clippy::too_many_arguments, reason = "needs refactoring")
    )]
    pub async fn run_scenario(
        &self,
        id: ScenarioId,
        feature: Source<gherkin::Feature>,
        rule: Option<Source<gherkin::Rule>>,
        scenario: Source<gherkin::Scenario>,
        scenario_ty: ScenarioType,
        retries: Option<RetryOptions>,
        #[cfg(feature = "tracing")] waiter: Option<&SpanCloseWaiter>,
    ) {
        let retry_num = retries.map(|r| r.retries);
        let ok = |e: fn(_) -> event::Scenario<W>| {
            let (f, r, s) = (&feature, &rule, &scenario);
            move |step| {
                let (f, r, s) = (f.clone(), r.clone(), s.clone());
                let event = e(step).with_retries(retry_num);
                event::Cucumber::scenario(f, r, s, event)
            }
        };
        let ok_capt = |e: fn(_, _, _) -> event::Scenario<W>| {
            let (f, r, s) = (&feature, &rule, &scenario);
            move |step, cap, loc| {
                let (f, r, s) = (f.clone(), r.clone(), s.clone());
                let event = e(step, cap, loc).with_retries(retry_num);
                event::Cucumber::scenario(f, r, s, event)
            }
        };

        let compose = |started, passed, skipped| {
            (ok(started), ok_capt(passed), ok(skipped))
        };
        let into_bg_step_ev = compose(
            event::Scenario::background_step_started,
            event::Scenario::background_step_passed,
            event::Scenario::background_step_skipped,
        );
        let into_step_ev = compose(
            event::Scenario::step_started,
            event::Scenario::step_passed,
            event::Scenario::step_skipped,
        );

        self.send_event(event::Cucumber::scenario(
            feature.clone(),
            rule.clone(),
            scenario.clone(),
            event::Scenario::Started.with_retries(retry_num),
        ));

        let is_failed = async {
            let mut result = async {
                let before_hook = self
                    .run_before_hook(
                        &feature,
                        rule.as_ref(),
                        &scenario,
                        retry_num,
                        id,
                        #[cfg(feature = "tracing")]
                        waiter,
                    )
                    .await?;

                let feature_background = feature
                    .background
                    .as_ref()
                    .map(|b| b.steps.iter().map(|s| Source::new(s.clone())))
                    .into_iter()
                    .flatten();

                let feature_background = stream::iter(feature_background)
                    .map(Ok)
                    .try_fold(before_hook, |world, bg_step| {
                        self.run_step(
                            world,
                            bg_step,
                            true,
                            into_bg_step_ev,
                            id,
                            #[cfg(feature = "tracing")]
                            waiter,
                        )
                        .map_ok(Some)
                    })
                    .await?;

                let rule_background = rule
                    .as_ref()
                    .map(|r| {
                        r.background
                            .as_ref()
                            .map(|b| {
                                b.steps.iter().map(|s| Source::new(s.clone()))
                            })
                            .into_iter()
                            .flatten()
                    })
                    .into_iter()
                    .flatten();

                let rule_background = stream::iter(rule_background)
                    .map(Ok)
                    .try_fold(feature_background, |world, bg_step| {
                        self.run_step(
                            world,
                            bg_step,
                            true,
                            into_bg_step_ev,
                            id,
                            #[cfg(feature = "tracing")]
                            waiter,
                        )
                        .map_ok(Some)
                    })
                    .await?;

                stream::iter(
                    scenario.steps.iter().map(|s| Source::new(s.clone())),
                )
                .map(Ok)
                .try_fold(rule_background, |world, step| {
                    self.run_step(
                        world,
                        step,
                        false,
                        into_step_ev,
                        id,
                        #[cfg(feature = "tracing")]
                        waiter,
                    )
                    .map_ok(Some)
                })
                .await
            }
            .await;

            let (world, scenario_finished_ev) = match &mut result {
                Ok(world) => {
                    (world.take(), event::ScenarioFinished::StepPassed)
                }
                Err(exec_err) => (
                    exec_err.take_world(),
                    exec_err.get_scenario_finished_event(),
                ),
            };

            let (world, after_hook_meta, after_hook_error) = self
                .run_after_hook(
                    world,
                    &feature,
                    rule.as_ref(),
                    &scenario,
                    scenario_finished_ev,
                    id,
                    #[cfg(feature = "tracing")]
                    waiter,
                )
                .await
                .map_or_else(
                    |(w, meta, info)| (w.map(Arc::new), Some(meta), Some(info)),
                    |(w, meta)| (w.map(Arc::new), meta, None),
                );

            let scenario_failed = match &result {
                Ok(_) | Err(ExecutionFailure::StepSkipped(_)) => false,
                Err(
                    ExecutionFailure::BeforeHookPanicked { .. }
                    | ExecutionFailure::StepPanicked { .. },
                ) => true,
            };
            let is_failed = scenario_failed || after_hook_error.is_some();

            if let Some(exec_error) = result.err() {
                self.emit_failed_events(
                    feature.clone(),
                    rule.clone(),
                    scenario.clone(),
                    world.clone(),
                    exec_error,
                    retry_num,
                );
            }

            self.emit_after_hook_events(
                feature.clone(),
                rule.clone(),
                scenario.clone(),
                world,
                after_hook_meta,
                after_hook_error,
                retry_num,
            );

            is_failed
        };
        #[cfg(feature = "tracing")]
        let (is_failed, span_id) = {
            let span = id.scenario_span();
            let span_id = span.id();
            let is_failed = tracing::Instrument::instrument(is_failed, span);
            (is_failed, span_id)
        };
        let is_failed = is_failed.then_yield().await;

        #[cfg(feature = "tracing")]
        if let Some((waiter, span_id)) = waiter.zip(span_id) {
            waiter.wait_for_span_close(span_id).then_yield().await;
        }

        self.send_event(event::Cucumber::scenario(
            feature.clone(),
            rule.clone(),
            scenario.clone(),
            event::Scenario::Finished.with_retries(retry_num),
        ));

        let next_try =
            retries.filter(|_| is_failed).and_then(RetryOptions::next_try);
        if let Some(next_try) = next_try {
            self.storage
                .insert_retried_scenario(
                    feature.clone(),
                    rule.clone(),
                    scenario,
                    scenario_ty,
                    Some(next_try),
                )
                .await;
        }

        self.scenario_finished(
            id,
            feature,
            rule,
            is_failed,
            next_try.is_some(),
        );
    }

    /// Executes [`HookType::Before`], if present.
    ///
    /// # Events
    ///
    /// - Emits all the [`HookType::Before`] events, except [`Hook::Failed`].
    ///   See [`Self::emit_failed_events()`] for more details.
    ///
    /// [`Hook::Failed`]: event::Hook::Failed
    async fn run_before_hook(
        &self,
        feature: &Source<gherkin::Feature>,
        rule: Option<&Source<gherkin::Rule>>,
        scenario: &Source<gherkin::Scenario>,
        retries: Option<Retries>,
        scenario_id: ScenarioId,
        #[cfg(feature = "tracing")] waiter: Option<&SpanCloseWaiter>,
    ) -> Result<Option<W>, ExecutionFailure<W>> {
        let init_world = async {
            AssertUnwindSafe(async { W::new().await })
                .catch_unwind()
                .then_yield()
                .await
                .map_err(Info::from)
                .and_then(|r| {
                    r.map_err(|e| {
                        coerce_into_info(format!(
                            "failed to initialize World: {e}",
                        ))
                    })
                })
                .map_err(|info| (info, None))
        };

        if let Some(hook) = self.before_hook.as_ref() {
            self.send_event(event::Cucumber::scenario(
                feature.clone(),
                rule.cloned(),
                scenario.clone(),
                event::Scenario::hook_started(HookType::Before)
                    .with_retries(retries),
            ));

            let fut = init_world.and_then(async |mut world| {
                let fut = async {
                    (hook)(
                        feature.as_ref(),
                        rule.as_ref().map(AsRef::as_ref),
                        scenario.as_ref(),
                        &mut world,
                    )
                    .await;
                };
                match AssertUnwindSafe(fut).catch_unwind().await {
                    Ok(()) => Ok(world),
                    Err(i) => Err((Info::from(i), Some(world))),
                }
            });

            #[cfg(feature = "tracing")]
            let (fut, span_id) = {
                let span = scenario_id.hook_span(HookType::Before);
                let span_id = span.id();
                let fut = tracing::Instrument::instrument(fut, span);
                (fut, span_id)
            };
            #[cfg(not(feature = "tracing"))]
            let _: ScenarioId = scenario_id;

            let result = fut.then_yield().await;

            #[cfg(feature = "tracing")]
            if let Some((waiter, id)) = waiter.zip(span_id) {
                waiter.wait_for_span_close(id).then_yield().await;
            }

            match result {
                Ok(world) => {
                    self.send_event(event::Cucumber::scenario(
                        feature.clone(),
                        rule.cloned(),
                        scenario.clone(),
                        event::Scenario::hook_passed(HookType::Before)
                            .with_retries(retries),
                    ));
                    Ok(Some(world))
                }
                Err((panic_info, world)) => {
                    Err(ExecutionFailure::BeforeHookPanicked {
                        world,
                        panic_info,
                        meta: event::Metadata::new(()),
                    })
                }
            }
        } else {
            Ok(None)
        }
    }

    /// Runs a [`Step`].
    ///
    /// # Events
    ///
    /// - Emits all the [`Step`] events, except [`Step::Failed`]. See
    ///   [`Self::emit_failed_events()`] for more details.
    ///
    /// [`Step`]: gherkin::Step
    /// [`Step::Failed`]: event::Step::Failed
    async fn run_step<St, Ps, Sk>(
        &self,
        world_opt: Option<W>,
        step: Source<gherkin::Step>,
        is_background: bool,
        (started, passed, skipped): (St, Ps, Sk),
        scenario_id: ScenarioId,
        #[cfg(feature = "tracing")] waiter: Option<&SpanCloseWaiter>,
    ) -> Result<W, ExecutionFailure<W>>
    where
        St: FnOnce(Source<gherkin::Step>) -> event::Cucumber<W>,
        Ps: FnOnce(
            Source<gherkin::Step>,
            CaptureLocations,
            Option<step::Location>,
        ) -> event::Cucumber<W>,
        Sk: FnOnce(Source<gherkin::Step>) -> event::Cucumber<W>,
    {
        self.send_event(started(step.clone()));

        let run = async {
            let (step_fn, captures, loc, ctx) =
                match self.collection.find(&step) {
                    Ok(Some(f)) => f,
                    Ok(None) => return Ok((None, None, world_opt)),
                    Err(e) => {
                        let e = event::StepError::AmbiguousMatch(e);
                        return Err((e, None, None, world_opt));
                    }
                };

            let mut world = if let Some(w) = world_opt {
                w
            } else {
                match AssertUnwindSafe(async { W::new().await })
                    .catch_unwind()
                    .then_yield()
                    .await
                {
                    Ok(Ok(w)) => w,
                    Ok(Err(e)) => {
                        let e = event::StepError::Panic(coerce_into_info(
                            format!("failed to initialize `World`: {e}"),
                        ));
                        return Err((e, None, loc, None));
                    }
                    Err(e) => {
                        let e = event::StepError::Panic(e.into());
                        return Err((e, None, loc, None));
                    }
                }
            };

            match AssertUnwindSafe(async { step_fn(&mut world, ctx).await })
                .catch_unwind()
                .await
            {
                Ok(()) => Ok((Some(captures), loc, Some(world))),
                Err(e) => {
                    let e = event::StepError::Panic(e.into());
                    Err((e, Some(captures), loc, Some(world)))
                }
            }
        };

        #[cfg(feature = "tracing")]
        let (run, span_id) = {
            let span = scenario_id.step_span(is_background);
            let span_id = span.id();
            let run = tracing::Instrument::instrument(run, span);
            (run, span_id)
        };
        let result = run.then_yield().await;

        #[cfg(feature = "tracing")]
        if let Some((waiter, id)) = waiter.zip(span_id) {
            waiter.wait_for_span_close(id).then_yield().await;
        }
        #[cfg(not(feature = "tracing"))]
        let _: ScenarioId = scenario_id;

        match result {
            Ok((Some(captures), loc, Some(world))) => {
                self.send_event(passed(step, captures, loc));
                Ok(world)
            }
            Ok((_, _, world)) => {
                self.send_event(skipped(step));
                Err(ExecutionFailure::StepSkipped(world))
            }
            Err((err, captures, loc, world)) => {
                Err(ExecutionFailure::StepPanicked {
                    world,
                    step,
                    captures,
                    loc,
                    err,
                    meta: event::Metadata::new(()),
                    is_background,
                })
            }
        }
    }

    /// Emits all the failure events of [`HookType::Before`] or [`Step`] after
    /// executing the [`Self::run_after_hook()`].
    ///
    /// This is done because [`HookType::After`] requires a mutable reference to
    /// the [`World`] while on the other hand we store immutable reference to it
    /// inside failure events for easier debugging. So, to avoid imposing
    /// additional [`Clone`] bounds on the [`World`], we run the
    /// [`HookType::After`] first without emitting any events about its
    /// execution, then emit failure event of the [`HookType::Before`] or
    /// [`Step`], if present, and finally emit all the [`HookType::After`]
    /// events. This allows us to ensure [order guarantees][1] while not
    /// restricting the [`HookType::After`] to the immutable reference. The only
    /// downside of this approach is that we may emit failure events of
    /// [`HookType::Before`] or [`Step`] with the [`World`] state being changed
    /// by the [`HookType::After`].
    ///
    /// [`Step`]: gherkin::Step
    /// [1]: crate::Runner#order-guarantees
    fn emit_failed_events(
        &self,
        feature: Source<gherkin::Feature>,
        rule: Option<Source<gherkin::Rule>>,
        scenario: Source<gherkin::Scenario>,
        world: Option<Arc<W>>,
        err: ExecutionFailure<W>,
        retries: Option<Retries>,
    ) {
        match err {
            ExecutionFailure::StepSkipped(_) => {}
            ExecutionFailure::BeforeHookPanicked {
                panic_info, meta, ..
            } => {
                self.send_event_with_meta(
                    event::Cucumber::scenario(
                        feature,
                        rule,
                        scenario,
                        event::Scenario::hook_failed(
                            HookType::Before,
                            world,
                            panic_info,
                        )
                        .with_retries(retries),
                    ),
                    meta,
                );
            }
            ExecutionFailure::StepPanicked {
                step,
                captures,
                loc,
                err: error,
                meta,
                is_background: true,
                ..
            } => self.send_event_with_meta(
                event::Cucumber::scenario(
                    feature,
                    rule,
                    scenario,
                    event::Scenario::background_step_failed(
                        step, captures, loc, world, error,
                    )
                    .with_retries(retries),
                ),
                meta,
            ),
            ExecutionFailure::StepPanicked {
                step,
                captures,
                loc,
                err: error,
                meta,
                is_background: false,
                ..
            } => self.send_event_with_meta(
                event::Cucumber::scenario(
                    feature,
                    rule,
                    scenario,
                    event::Scenario::step_failed(
                        step, captures, loc, world, error,
                    )
                    .with_retries(retries),
                ),
                meta,
            ),
        }
    }

    /// Executes the [`HookType::After`], if present.
    ///
    /// Doesn't emit any events, see [`Self::emit_failed_events()`] for more
    /// details.
    #[cfg_attr(
        feature = "tracing",
        expect(clippy::too_many_arguments, reason = "needs refactoring")
    )]
    async fn run_after_hook(
        &self,
        mut world: Option<W>,
        feature: &Source<gherkin::Feature>,
        rule: Option<&Source<gherkin::Rule>>,
        scenario: &Source<gherkin::Scenario>,
        ev: event::ScenarioFinished,
        scenario_id: ScenarioId,
        #[cfg(feature = "tracing")] waiter: Option<&SpanCloseWaiter>,
    ) -> Result<
        (Option<W>, Option<AfterHookEventsMeta>),
        (Option<W>, AfterHookEventsMeta, Info),
    > {
        if let Some(hook) = self.after_hook.as_ref() {
            let fut = async {
                (hook)(
                    feature.as_ref(),
                    rule.as_ref().map(AsRef::as_ref),
                    scenario.as_ref(),
                    &ev,
                    world.as_mut(),
                )
                .await;
            };

            let started = event::Metadata::new(());
            let fut = AssertUnwindSafe(fut).catch_unwind();

            #[cfg(feature = "tracing")]
            let (fut, span_id) = {
                let span = scenario_id.hook_span(HookType::After);
                let span_id = span.id();
                let fut = tracing::Instrument::instrument(fut, span);
                (fut, span_id)
            };
            #[cfg(not(feature = "tracing"))]
            let _: ScenarioId = scenario_id;

            let res = fut.then_yield().await;

            #[cfg(feature = "tracing")]
            if let Some((waiter, id)) = waiter.zip(span_id) {
                waiter.wait_for_span_close(id).then_yield().await;
            }

            let finished = event::Metadata::new(());
            let meta = AfterHookEventsMeta { started, finished };

            match res {
                Ok(()) => Ok((world, Some(meta))),
                Err(info) => Err((world, meta, info.into())),
            }
        } else {
            Ok((world, None))
        }
    }

    /// Emits all the [`HookType::After`] events.
    ///
    /// See [`Self::emit_failed_events()`] for the explanation why we don't do
    /// that inside [`Self::run_after_hook()`].
    #[expect(clippy::too_many_arguments, reason = "needs refactoring")]
    fn emit_after_hook_events(
        &self,
        feature: Source<gherkin::Feature>,
        rule: Option<Source<gherkin::Rule>>,
        scenario: Source<gherkin::Scenario>,
        world: Option<Arc<W>>,
        meta: Option<AfterHookEventsMeta>,
        err: Option<Info>,
        retries: Option<Retries>,
    ) {
        debug_assert_eq!(
            self.after_hook.is_some(),
            meta.is_some(),
            "`AfterHookEventsMeta` is not passed, despite `self.after_hook` \
             being set",
        );

        if let Some(meta) = meta {
            self.send_event_with_meta(
                event::Cucumber::scenario(
                    feature.clone(),
                    rule.clone(),
                    scenario.clone(),
                    event::Scenario::hook_started(HookType::After)
                        .with_retries(retries),
                ),
                meta.started,
            );

            let ev = if let Some(e) = err {
                event::Cucumber::scenario(
                    feature,
                    rule,
                    scenario,
                    event::Scenario::hook_failed(HookType::After, world, e)
                        .with_retries(retries),
                )
            } else {
                event::Cucumber::scenario(
                    feature,
                    rule,
                    scenario,
                    event::Scenario::hook_passed(HookType::After)
                        .with_retries(retries),
                )
            };

            self.send_event_with_meta(ev, meta.finished);
        }
    }

    /// Notifies [`FinishedRulesAndFeatures`] about [`Scenario`] being finished.
    ///
    /// [`FinishedRulesAndFeatures`]: super::scenario_storage::FinishedRulesAndFeatures
    /// [`Scenario`]: gherkin::Scenario
    fn scenario_finished(
        &self,
        id: ScenarioId,
        feature: Source<gherkin::Feature>,
        rule: Option<Source<gherkin::Rule>>,
        is_failed: IsFailed,
        is_retried: IsRetried,
    ) {
        // If the receiver end is dropped, then no one listens for events
        // so we can just ignore it.
        drop(
            self.finished_sender
                .unbounded_send((id, feature, rule, is_failed, is_retried)),
        );
    }

    /// Notifies with the given [`Cucumber`] event.
    ///
    /// [`Cucumber`]: event::Cucumber
    pub fn send_event(&self, event: event::Cucumber<W>) {
        // If the receiver end is dropped, then no one listens for events,
        // so we can just ignore it.
        drop(self.event_sender.unbounded_send(Ok(Event::new(event))));
    }

    /// Notifies with the given [`Cucumber`] event along with its [`Metadata`].
    ///
    /// [`Cucumber`]: event::Cucumber
    /// [`Metadata`]: event::Metadata
    fn send_event_with_meta(
        &self,
        event: event::Cucumber<W>,
        meta: event::Metadata,
    ) {
        // If the receiver end is dropped, then no one listens for events,
        // so we can just ignore it.
        drop(self.event_sender.unbounded_send(Ok(meta.wrap(event))));
    }

    /// Notifies with the given [`Cucumber`] events.
    ///
    /// [`Cucumber`]: event::Cucumber
    pub fn send_all_events(
        &self,
        events: impl IntoIterator<Item = event::Cucumber<W>>,
    ) {
        for v in events {
            // If the receiver end is dropped, then no one listens for events,
            // so we can just stop from here.
            if self.event_sender.unbounded_send(Ok(Event::new(v))).is_err() {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::channel::mpsc;
    use std::sync::Arc;
    use crate::test_utils::common::TestWorld;

    // Using common TestWorld from test_utils

    fn create_test_executor() -> (Executor<TestWorld, impl Future<Output = ()>, impl Future<Output = ()>>, mpsc::UnboundedReceiver<event::Cucumber<TestWorld>>) {
        let (event_sender, event_receiver) = mpsc::unbounded();
        let (finished_sender, _) = mpsc::unbounded();
        let collection = step::Collection::<TestWorld>::new();
        let features = Features::default();

        let before_hook = |_: &gherkin::Feature, _: Option<&gherkin::Rule>, _: &gherkin::Scenario, _: &mut TestWorld| {
            Box::pin(async {}) as LocalBoxFuture<'_, ()>
        };
        let after_hook = |_: &gherkin::Feature, _: Option<&gherkin::Rule>, _: &gherkin::Scenario, _: &event::ScenarioFinished, _: Option<&mut TestWorld>| {
            Box::pin(async {}) as LocalBoxFuture<'_, ()>
        };
        
        let executor = Executor::new(
            collection,
            Some(before_hook),
            Some(after_hook),
            event_sender,
            finished_sender,
            features,
        );

        (executor, event_receiver)
    }

    fn create_test_feature_and_scenario() -> (Source<gherkin::Feature>, Source<gherkin::Scenario>) {
        let feature = Source::new(gherkin::Feature {
            tags: vec![],
            keyword: "Feature".to_string(),
            name: "Test Feature".to_string(),
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 1, col: 1 },
            path: None,
            description: None,
            background: None,
            scenarios: vec![],
            rules: vec![],
        });

        let scenario = Source::new(gherkin::Scenario {
            tags: vec![],
            keyword: "Scenario".to_string(),
            name: "Test Scenario".to_string(),
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 1, col: 1 },
            description: None,
            steps: vec![],
            examples: vec![],
        });

        (feature, scenario)
    }

    #[test]
    fn test_executor_creation() {
        let (executor, _) = create_test_executor();
        
        // Test that executor can be created without issues
        assert!(executor.before_hook.is_none());
        assert!(executor.after_hook.is_none());
    }

    #[test]
    fn test_executor_send_event() {
        let (executor, mut receiver) = create_test_executor();
        let (feature, scenario) = create_test_feature_and_scenario();
        
        let event = event::Cucumber::scenario(
            feature,
            None,
            scenario,
            event::RetryableScenario {
                event: event::Scenario::Started,
                retries: None,
            },
        );
        
        executor.send_event(event);
        
        // Should receive the event
        let received = receiver.try_next().unwrap().unwrap().unwrap();
        match received.value {
            event::Cucumber::Feature(_, event::Feature::Scenario(_, scenario_event)) => {
                match scenario_event.event {
                    event::Scenario::Started => {},
                    _ => panic!("Expected Scenario::Started event"),
                }
            },
            _ => panic!("Expected Feature::Scenario event"),
        }
    }

    #[test]
    fn test_executor_send_all_events() {
        let (executor, mut receiver) = create_test_executor();
        let (feature, scenario) = create_test_feature_and_scenario();
        
        let events = vec![
            event::Cucumber::scenario(
                feature.clone(),
                None,
                scenario.clone(),
                event::RetryableScenario {
                    event: event::Scenario::Started,
                    retries: None,
                },
            ),
            event::Cucumber::scenario(
                feature,
                None,
                scenario,
                event::RetryableScenario {
                    event: event::Scenario::Finished,
                    retries: None,
                },
            ),
        ];
        
        executor.send_all_events(events);
        
        // Should receive both events
        let first = receiver.try_next().unwrap().unwrap().unwrap();
        let second = receiver.try_next().unwrap().unwrap().unwrap();
        
        match (&first.value, &second.value) {
            (
                event::Cucumber::Feature(_, event::Feature::Scenario(_, first_scenario)),
                event::Cucumber::Feature(_, event::Feature::Scenario(_, second_scenario)),
            ) => {
                match (&first_scenario.event, &second_scenario.event) {
                    (event::Scenario::Started, event::Scenario::Finished) => {},
                    _ => panic!("Expected Started and Finished events"),
                }
            },
            _ => panic!("Expected Feature::Scenario events"),
        }
    }

    #[tokio::test]
    async fn test_executor_run_before_hook_none() {
        let (executor, _) = create_test_executor();
        let (feature, scenario) = create_test_feature_and_scenario();
        let id = ScenarioId::new();
        
        let result = executor.run_before_hook(
            &feature,
            None,
            &scenario,
            None,
            id,
            #[cfg(feature = "tracing")]
            None,
        ).await;
        
        // Should return Ok(None) when no before hook is set
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_executor_run_step_empty_collection() {
        let (executor, mut receiver) = create_test_executor();
        let id = ScenarioId::new();
        
        let step = Source::new(gherkin::Step {
            ty: gherkin::StepType::Given,
            keyword: "Given".to_string(),
            value: "test step".to_string(),
            docstring: None,
            table: None,
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 1, col: 1 },
        });
        
        let started = |step| event::Cucumber::Started;
        let passed = |_step, _cap, _loc| event::Cucumber::Started;
        let skipped = |_step| event::Cucumber::Started;
        
        let result = executor.run_step(
            None,
            step,
            false,
            (started, passed, skipped),
            id,
            #[cfg(feature = "tracing")]
            None,
        ).await;
        
        // Should emit started event first
        let started_event = receiver.try_next().unwrap().unwrap().unwrap();
        match started_event.value {
            event::Cucumber::Started => {},
            _ => panic!("Expected Started event"),
        }
        
        // Should emit skipped event and fail with StepSkipped
        let skipped_event = receiver.try_next().unwrap().unwrap().unwrap();
        match skipped_event.value {
            event::Cucumber::Started => {},
            _ => panic!("Expected Started (skipped) event"),
        }
        
        assert!(result.is_err());
        match result.unwrap_err() {
            ExecutionFailure::StepSkipped(_) => {},
            _ => panic!("Expected StepSkipped error"),
        }
    }

    #[test]
    fn test_executor_scenario_finished() {
        let (executor, _) = create_test_executor();
        let (feature, _) = create_test_feature_and_scenario();
        let id = ScenarioId::new();
        
        // Should not panic when calling scenario_finished
        executor.scenario_finished(id, feature, None, false, false);
    }

    #[test]
    fn test_executor_emit_failed_events_step_skipped() {
        let (executor, mut receiver) = create_test_executor();
        let (feature, scenario) = create_test_feature_and_scenario();
        
        let failure = ExecutionFailure::StepSkipped(None);
        
        // Should not emit any events for StepSkipped
        executor.emit_failed_events(feature, None, scenario, None, failure, None);
        
        // No events should be emitted
        assert!(receiver.try_next().is_err());
    }

    #[test]
    fn test_executor_emit_failed_events_before_hook_panicked() {
        let (executor, mut receiver) = create_test_executor();
        let (feature, scenario) = create_test_feature_and_scenario();
        
        let failure = ExecutionFailure::BeforeHookPanicked {
            world: None,
            panic_info: Arc::new("test panic"),
            meta: event::Metadata::new(()),
        };
        
        executor.emit_failed_events(feature, None, scenario, None, failure, None);
        
        // Should emit hook failed event
        let event = receiver.try_next().unwrap().unwrap().unwrap();
        match event.value {
            event::Cucumber::Feature(_, event::Feature::Scenario(_, scenario_event)) => {
                match scenario_event.event {
                    event::Scenario::Hook { ty: HookType::Before, event: event::Hook::Failed { .. }} => {},
                    _ => panic!("Expected Before Hook Failed event"),
                }
            },
            _ => panic!("Expected Feature::Scenario event"),
        }
    }
}