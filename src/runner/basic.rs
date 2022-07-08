// Copyright (c) 2018-2022  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Default [`Runner`] implementation.

use std::{
    cmp,
    collections::HashMap,
    fmt, mem,
    ops::ControlFlow,
    panic::{self, AssertUnwindSafe},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use futures::{
    channel::mpsc,
    future::{self, Either, LocalBoxFuture},
    lock::Mutex,
    pin_mut,
    stream::{self, LocalBoxStream},
    FutureExt as _, Stream, StreamExt as _, TryFutureExt as _,
    TryStreamExt as _,
};
use itertools::Itertools as _;
use regex::{CaptureLocations, Regex};

use crate::{
    event::{self, HookType, Info},
    feature::Ext as _,
    parser, step, Event, Runner, Step, World,
};

/// CLI options of a [`Basic`] [`Runner`].
#[derive(clap::Args, Clone, Copy, Debug)]
pub struct Cli {
    /// Number of scenarios to run concurrently. If not specified, uses the
    /// value configured in tests runner, or 64 by default.
    #[clap(long, short, name = "int", global = true)]
    pub concurrency: Option<usize>,

    /// Run tests until the first failure.
    #[clap(long, global = true)]
    pub fail_fast: bool,
}

/// Type determining whether [`Scenario`]s should run concurrently or
/// sequentially.
///
/// [`Scenario`]: gherkin::Scenario
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ScenarioType {
    /// Run [`Scenario`]s sequentially (one-by-one).
    ///
    /// [`Scenario`]: gherkin::Scenario
    Serial,

    /// Run [`Scenario`]s concurrently.
    ///
    /// [`Scenario`]: gherkin::Scenario
    Concurrent,
}

/// Alias for [`fn`] used to determine whether a [`Scenario`] is [`Concurrent`]
/// or a [`Serial`] one.
///
/// [`Concurrent`]: ScenarioType::Concurrent
/// [`Serial`]: ScenarioType::Serial
/// [`Scenario`]: gherkin::Scenario
pub type WhichScenarioFn = fn(
    &gherkin::Feature,
    Option<&gherkin::Rule>,
    &gherkin::Scenario,
) -> ScenarioType;

/// Alias for [`fn`] executed on each [`Scenario`] before running all [`Step`]s.
///
/// [`Scenario`]: gherkin::Scenario
/// [`Step`]: gherkin::Step
pub type BeforeHookFn<World> = for<'a> fn(
    &'a gherkin::Feature,
    Option<&'a gherkin::Rule>,
    &'a gherkin::Scenario,
    &'a mut World,
) -> LocalBoxFuture<'a, ()>;

/// Alias for [`fn`] executed on each [`Scenario`] after running all [`Step`]s.
///
/// [`Scenario`]: gherkin::Scenario
/// [`Step`]: gherkin::Step
pub type AfterHookFn<World> = for<'a> fn(
    &'a gherkin::Feature,
    Option<&'a gherkin::Rule>,
    &'a gherkin::Scenario,
    Option<&'a mut World>,
) -> LocalBoxFuture<'a, ()>;

/// Alias for a failed [`Scenario`].
///
/// [`Scenario`]: gherkin::Scenario
type Failed = bool;

/// Default [`Runner`] implementation which follows [_order guarantees_][1] from
/// the [`Runner`] trait docs.
///
/// Executes [`Scenario`]s concurrently based on the custom function, which
/// returns [`ScenarioType`]. Also, can limit maximum number of concurrent
/// [`Scenario`]s.
///
/// [1]: Runner#order-guarantees
/// [`Scenario`]: gherkin::Scenario
pub struct Basic<
    World,
    F = WhichScenarioFn,
    Before = BeforeHookFn<World>,
    After = AfterHookFn<World>,
> {
    /// Optional number of concurrently executed [`Scenario`]s.
    ///
    /// [`Scenario`]: gherkin::Scenario
    max_concurrent_scenarios: Option<usize>,

    /// [`Collection`] of functions to match [`Step`]s.
    ///
    /// [`Collection`]: step::Collection
    steps: step::Collection<World>,

    /// Function determining whether a [`Scenario`] is [`Concurrent`] or
    /// a [`Serial`] one.
    ///
    /// [`Concurrent`]: ScenarioType::Concurrent
    /// [`Serial`]: ScenarioType::Serial
    /// [`Scenario`]: gherkin::Scenario
    which_scenario: F,

    /// Function, executed on each [`Scenario`] before running all [`Step`]s,
    /// including [`Background`] ones.
    ///
    /// [`Background`]: gherkin::Background
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
    before_hook: Option<Before>,

    /// Function, executed on each [`Scenario`] after running all [`Step`]s.
    ///
    /// [`Background`]: gherkin::Background
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
    after_hook: Option<After>,

    /// Indicates whether execution should be stopped after the first failure.
    fail_fast: bool,
}

// Implemented manually to omit redundant trait bounds on `World` and to omit
// outputting `F`.
impl<World, F, B, A> fmt::Debug for Basic<World, F, B, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Basic")
            .field("max_concurrent_scenarios", &self.max_concurrent_scenarios)
            .field("steps", &self.steps)
            .finish_non_exhaustive()
    }
}

impl<World> Basic<World, ()> {
    /// Creates a new empty [`Runner`].
    #[must_use]
    pub fn custom() -> Self {
        Self {
            max_concurrent_scenarios: None,
            steps: step::Collection::new(),
            which_scenario: (),
            before_hook: None,
            after_hook: None,
            fail_fast: false,
        }
    }
}

impl<World> Default for Basic<World> {
    fn default() -> Self {
        let which_scenario: WhichScenarioFn = |_, _, scenario| {
            scenario
                .tags
                .iter()
                .any(|tag| tag == "serial")
                .then_some(ScenarioType::Serial)
                .unwrap_or(ScenarioType::Concurrent)
        };

        Self {
            max_concurrent_scenarios: Some(64),
            steps: step::Collection::new(),
            which_scenario,
            before_hook: None,
            after_hook: None,
            fail_fast: false,
        }
    }
}

impl<World, Which, Before, After> Basic<World, Which, Before, After> {
    /// If `max` is [`Some`], then number of concurrently executed [`Scenario`]s
    /// will be limited.
    ///
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    pub fn max_concurrent_scenarios(
        mut self,
        max: impl Into<Option<usize>>,
    ) -> Self {
        self.max_concurrent_scenarios = max.into();
        self
    }

    /// Run tests until the first failure.
    ///
    /// __NOTE__: All the already started [`Scenario`]s at the moment of failure
    ///           will be finished.
    ///
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    pub const fn fail_fast(mut self) -> Self {
        self.fail_fast = true;
        self
    }

    /// Function determining whether a [`Scenario`] is [`Concurrent`] or
    /// a [`Serial`] one.
    ///
    /// [`Concurrent`]: ScenarioType::Concurrent
    /// [`Serial`]: ScenarioType::Serial
    /// [`Scenario`]: gherkin::Scenario
    #[allow(clippy::missing_const_for_fn)] // false positive: drop in const
    #[must_use]
    pub fn which_scenario<F>(self, func: F) -> Basic<World, F, Before, After>
    where
        F: Fn(
                &gherkin::Feature,
                Option<&gherkin::Rule>,
                &gherkin::Scenario,
            ) -> ScenarioType
            + 'static,
    {
        let Self {
            max_concurrent_scenarios,
            steps,
            before_hook,
            after_hook,
            fail_fast,
            ..
        } = self;
        Basic {
            max_concurrent_scenarios,
            steps,
            which_scenario: func,
            before_hook,
            after_hook,
            fail_fast,
        }
    }

    /// Sets a hook, executed on each [`Scenario`] before running all its
    /// [`Step`]s, including [`Background`] ones.
    ///
    /// [`Background`]: gherkin::Background
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
    #[allow(clippy::missing_const_for_fn)] // false positive: drop in const
    #[must_use]
    pub fn before<Func>(self, func: Func) -> Basic<World, Which, Func, After>
    where
        Func: for<'a> Fn(
            &'a gherkin::Feature,
            Option<&'a gherkin::Rule>,
            &'a gherkin::Scenario,
            &'a mut World,
        ) -> LocalBoxFuture<'a, ()>,
    {
        let Self {
            max_concurrent_scenarios,
            steps,
            which_scenario,
            after_hook,
            fail_fast,
            ..
        } = self;
        Basic {
            max_concurrent_scenarios,
            steps,
            which_scenario,
            before_hook: Some(func),
            after_hook,
            fail_fast,
        }
    }

    /// Sets hook, executed on each [`Scenario`] after running all its
    /// [`Step`]s, even after [`Skipped`] of [`Failed`] ones.
    ///
    /// Last `World` argument is supplied to the function, in case it was
    /// initialized before by running [`before`] hook or any [`Step`].
    ///
    /// [`before`]: Self::before()
    /// [`Failed`]: event::Step::Failed
    /// [`Scenario`]: gherkin::Scenario
    /// [`Skipped`]: event::Step::Skipped
    /// [`Step`]: gherkin::Step
    #[allow(clippy::missing_const_for_fn)] // false positive: drop in const
    #[must_use]
    pub fn after<Func>(self, func: Func) -> Basic<World, Which, Before, Func>
    where
        Func: for<'a> Fn(
            &'a gherkin::Feature,
            Option<&'a gherkin::Rule>,
            &'a gherkin::Scenario,
            Option<&'a mut World>,
        ) -> LocalBoxFuture<'a, ()>,
    {
        let Self {
            max_concurrent_scenarios,
            steps,
            which_scenario,
            before_hook,
            fail_fast,
            ..
        } = self;
        Basic {
            max_concurrent_scenarios,
            steps,
            which_scenario,
            before_hook,
            after_hook: Some(func),
            fail_fast,
        }
    }

    /// Sets the given [`Collection`] of [`Step`]s to this [`Runner`].
    ///
    /// [`Collection`]: step::Collection
    #[allow(clippy::missing_const_for_fn)] // false positive: drop in const
    #[must_use]
    pub fn steps(mut self, steps: step::Collection<World>) -> Self {
        self.steps = steps;
        self
    }

    /// Adds a [Given] [`Step`] matching the given `regex`.
    ///
    /// [Given]: https://cucumber.io/docs/gherkin/reference/#given
    #[must_use]
    pub fn given(mut self, regex: Regex, step: Step<World>) -> Self {
        self.steps = mem::take(&mut self.steps).given(None, regex, step);
        self
    }

    /// Adds a [When] [`Step`] matching the given `regex`.
    ///
    /// [When]: https://cucumber.io/docs/gherkin/reference/#given
    #[must_use]
    pub fn when(mut self, regex: Regex, step: Step<World>) -> Self {
        self.steps = mem::take(&mut self.steps).when(None, regex, step);
        self
    }

    /// Adds a [Then] [`Step`] matching the given `regex`.
    ///
    /// [Then]: https://cucumber.io/docs/gherkin/reference/#then
    #[must_use]
    pub fn then(mut self, regex: Regex, step: Step<World>) -> Self {
        self.steps = mem::take(&mut self.steps).then(None, regex, step);
        self
    }
}

impl<W, Which, Before, After> Runner<W> for Basic<W, Which, Before, After>
where
    W: World,
    Which: Fn(
            &gherkin::Feature,
            Option<&gherkin::Rule>,
            &gherkin::Scenario,
        ) -> ScenarioType
        + 'static,
    Before: for<'a> Fn(
            &'a gherkin::Feature,
            Option<&'a gherkin::Rule>,
            &'a gherkin::Scenario,
            &'a mut W,
        ) -> LocalBoxFuture<'a, ()>
        + 'static,
    After: for<'a> Fn(
            &'a gherkin::Feature,
            Option<&'a gherkin::Rule>,
            &'a gherkin::Scenario,
            Option<&'a mut W>,
        ) -> LocalBoxFuture<'a, ()>
        + 'static,
{
    type Cli = Cli;

    type EventStream =
        LocalBoxStream<'static, parser::Result<Event<event::Cucumber<W>>>>;

    fn run<S>(self, features: S, cli: Cli) -> Self::EventStream
    where
        S: Stream<Item = parser::Result<gherkin::Feature>> + 'static,
    {
        let Self {
            max_concurrent_scenarios,
            steps,
            which_scenario,
            before_hook,
            after_hook,
            fail_fast,
        } = self;

        let fail_fast = if cli.fail_fast { true } else { fail_fast };
        let buffer = Features::default();
        let (sender, receiver) = mpsc::unbounded();

        let insert = insert_features(
            buffer.clone(),
            features,
            which_scenario,
            sender.clone(),
            fail_fast,
        );
        let execute = execute(
            buffer,
            cli.concurrency.or(max_concurrent_scenarios),
            steps,
            sender,
            before_hook,
            after_hook,
            fail_fast,
        );

        stream::select(
            receiver.map(Either::Left),
            future::join(insert, execute)
                .into_stream()
                .map(Either::Right),
        )
        .filter_map(|r| async {
            match r {
                Either::Left(ev) => Some(ev),
                Either::Right(_) => None,
            }
        })
        .boxed_local()
    }
}

/// Stores [`Feature`]s for later use by [`execute()`].
///
/// [`Feature`]: gherkin::Feature
async fn insert_features<W, S, F>(
    into: Features,
    features_stream: S,
    which_scenario: F,
    sender: mpsc::UnboundedSender<parser::Result<Event<event::Cucumber<W>>>>,
    fail_fast: bool,
) where
    S: Stream<Item = parser::Result<gherkin::Feature>> + 'static,
    F: Fn(
            &gherkin::Feature,
            Option<&gherkin::Rule>,
            &gherkin::Scenario,
        ) -> ScenarioType
        + 'static,
{
    let mut features = 0;
    let mut rules = 0;
    let mut scenarios = 0;
    let mut steps = 0;

    pin_mut!(features_stream);
    while let Some(feat) = features_stream.next().await {
        match feat {
            Ok(f) => {
                features += 1;
                rules += f.rules.len();
                scenarios += f.count_scenarios();
                steps += f.count_steps();

                into.insert(f, &which_scenario).await;
            }
            // If the receiver end is dropped, then no one listens for events
            // so we can just stop from here.
            Err(e) => {
                if sender.unbounded_send(Err(e)).is_err() || fail_fast {
                    break;
                }
            }
        }
    }

    drop(sender.unbounded_send(Ok(Event::new(
        event::Cucumber::ParsingFinished {
            features,
            rules,
            scenarios,
            steps,
        },
    ))));

    into.finish();
}

/// Retrieves [`Feature`]s and executes them.
///
/// # Events
///
/// - [`Scenario`] events are emitted by [`Executor`].
/// - If [`Scenario`] was first or last for particular [`Rule`] or [`Feature`],
///   emits starting or finishing events for them.
///
/// [`Feature`]: gherkin::Feature
/// [`Rule`]: gherkin::Rule
/// [`Scenario`]: gherkin::Scenario
async fn execute<W, Before, After>(
    features: Features,
    max_concurrent_scenarios: Option<usize>,
    collection: step::Collection<W>,
    event_sender: mpsc::UnboundedSender<
        parser::Result<Event<event::Cucumber<W>>>,
    >,
    before_hook: Option<Before>,
    after_hook: Option<After>,
    fail_fast: bool,
) where
    W: World,
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
            Option<&'a mut W>,
        ) -> LocalBoxFuture<'a, ()>,
{
    // Those panic hook shenanigans are done to avoid console messages like
    // "thread 'main' panicked at ..."
    //
    // 1. We obtain the current panic hook and replace it with an empty one.
    // 2. We run tests, which can panic. In that case we pass all panic info
    //    down the line to the Writer, which will print it at a right time.
    // 3. We restore original panic hook, because suppressing all panics doesn't
    //    sound like a very good idea.
    let hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));

    let (finished_sender, finished_receiver) = mpsc::unbounded();
    let mut storage = FinishedRulesAndFeatures::new(finished_receiver);
    let executor = Executor::new(
        collection,
        before_hook,
        after_hook,
        event_sender,
        finished_sender,
    );

    executor.send_event(event::Cucumber::Started);

    // TODO: Replace with `ControlFlow::map_break()` once stabilized:
    //       https://github.com/rust-lang/rust/issues/75744
    let map_break = |cf| match cf {
        ControlFlow::Continue(cont) => cont,
        ControlFlow::Break(()) => Some(0),
    };

    let mut started_scenarios = ControlFlow::Continue(max_concurrent_scenarios);
    let mut run_scenarios = stream::FuturesUnordered::new();
    loop {
        let runnable = features.get(map_break(started_scenarios)).await;
        if run_scenarios.is_empty() && runnable.is_empty() {
            if features.is_finished() {
                break;
            }
            continue;
        }

        let started = storage.start_scenarios(&runnable);
        executor.send_all_events(started);

        if let ControlFlow::Continue(Some(sc)) = &mut started_scenarios {
            *sc -= runnable.len();
        }

        for (f, r, s) in runnable {
            run_scenarios.push(executor.run_scenario(f, r, s));
        }

        if run_scenarios.next().await.is_some() {
            if let ControlFlow::Continue(Some(sc)) = &mut started_scenarios {
                *sc += 1;
            }
        }

        while let Ok(Some((feat, rule, scenario_failed))) =
            storage.finished_receiver.try_next()
        {
            if let Some(r) = rule {
                if let Some(f) =
                    storage.rule_scenario_finished(Arc::clone(&feat), r)
                {
                    executor.send_event(f);
                }
            }
            if let Some(f) = storage.feature_scenario_finished(feat) {
                executor.send_event(f);
            }

            if fail_fast && scenario_failed {
                started_scenarios = ControlFlow::Break(());
            }
        }
    }

    // This is done in case of `fail_fast: true`, when not all `Scenario`s might
    // be executed.
    executor.send_all_events(storage.finish_all_rules_and_features());

    executor.send_event(event::Cucumber::Finished);

    panic::set_hook(hook);
}

/// Runs [`Scenario`]s and notifies about their state of completion.
///
/// [`Scenario`]: gherkin::Scenario
struct Executor<W, Before, After> {
    /// [`Step`]s [`Collection`].
    ///
    /// [`Collection`]: step::Collection
    /// [`Step`]: step::Step
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
    finished_sender: mpsc::UnboundedSender<(
        Arc<gherkin::Feature>,
        Option<Arc<gherkin::Rule>>,
        Failed,
    )>,
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
            Option<&'a mut W>,
        ) -> LocalBoxFuture<'a, ()>,
{
    /// Creates a new [`Executor`].
    const fn new(
        collection: step::Collection<W>,
        before_hook: Option<Before>,
        after_hook: Option<After>,
        event_sender: mpsc::UnboundedSender<
            parser::Result<Event<event::Cucumber<W>>>,
        >,
        finished_sender: mpsc::UnboundedSender<(
            Arc<gherkin::Feature>,
            Option<Arc<gherkin::Rule>>,
            Failed,
        )>,
    ) -> Self {
        Self {
            collection,
            before_hook,
            after_hook,
            event_sender,
            finished_sender,
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
    #[allow(clippy::too_many_lines)]
    async fn run_scenario(
        &self,
        feature: Arc<gherkin::Feature>,
        rule: Option<Arc<gherkin::Rule>>,
        scenario: Arc<gherkin::Scenario>,
    ) {
        let ok = |e: fn(_) -> event::Scenario<W>| {
            let (f, r, s) = (&feature, &rule, &scenario);
            move |step| {
                let (f, r, s) = (Arc::clone(f), r.clone(), Arc::clone(s));
                event::Cucumber::scenario(f, r, s, e(step))
            }
        };
        let ok_capt = |e: fn(_, _) -> event::Scenario<W>| {
            let (f, r, s) = (&feature, &rule, &scenario);
            move |step, captures| {
                let (f, r, s) = (Arc::clone(f), r.clone(), Arc::clone(s));
                event::Cucumber::scenario(f, r, s, e(step, captures))
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
            Arc::clone(&feature),
            rule.clone(),
            Arc::clone(&scenario),
            event::Scenario::Started,
        ));

        let mut result = async {
            let before_hook = self
                .run_before_hook(&feature, rule.as_ref(), &scenario)
                .await?;

            let feature_background = feature
                .background
                .as_ref()
                .map(|b| b.steps.iter().map(|s| Arc::new(s.clone())))
                .into_iter()
                .flatten();

            let feature_background = stream::iter(feature_background)
                .map(Ok)
                .try_fold(before_hook, |world, bg_step| {
                    self.run_step(world, bg_step, true, into_bg_step_ev)
                        .map_ok(Some)
                })
                .await?;

            let rule_background = rule
                .as_ref()
                .map(|r| {
                    r.background
                        .as_ref()
                        .map(|b| b.steps.iter().map(|s| Arc::new(s.clone())))
                        .into_iter()
                        .flatten()
                })
                .into_iter()
                .flatten();

            let rule_background = stream::iter(rule_background)
                .map(Ok)
                .try_fold(feature_background, |world, bg_step| {
                    self.run_step(world, bg_step, true, into_bg_step_ev)
                        .map_ok(Some)
                })
                .await?;

            stream::iter(scenario.steps.iter().map(|s| Arc::new(s.clone())))
                .map(Ok)
                .try_fold(rule_background, |world, step| {
                    self.run_step(world, step, false, into_step_ev).map_ok(Some)
                })
                .await
        }
        .await;

        let world = match &mut result {
            Ok(world) => world.take(),
            Err(exec_err) => exec_err.take_world(),
        };

        let (world, after_hook_meta, after_hook_error) = self
            .run_after_hook(world, &feature, rule.as_ref(), &scenario)
            .await
            .map_or_else(
                |(w, meta, info)| (w.map(Arc::new), Some(meta), Some(info)),
                |(w, meta)| (w.map(Arc::new), meta, None),
            );

        let is_failed = result.is_err() || after_hook_error.is_some();

        if let Some(exec_error) = result.err() {
            self.emit_failed_events(
                Arc::clone(&feature),
                rule.clone(),
                Arc::clone(&scenario),
                world.clone(),
                exec_error,
            );
        }

        self.emit_after_hook_events(
            Arc::clone(&feature),
            rule.clone(),
            Arc::clone(&scenario),
            world,
            after_hook_meta,
            after_hook_error,
        );

        self.send_event(event::Cucumber::scenario(
            Arc::clone(&feature),
            rule.clone(),
            Arc::clone(&scenario),
            event::Scenario::Finished,
        ));

        self.scenario_finished(feature, rule, is_failed);
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
        feature: &Arc<gherkin::Feature>,
        rule: Option<&Arc<gherkin::Rule>>,
        scenario: &Arc<gherkin::Scenario>,
    ) -> Result<Option<W>, ExecutionFailure<W>> {
        let init_world = async {
            AssertUnwindSafe(W::new())
                .catch_unwind()
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
                Arc::clone(feature),
                rule.map(Arc::clone),
                Arc::clone(scenario),
                event::Scenario::hook_started(HookType::Before),
            ));

            let fut = init_world.and_then(|mut world| async {
                let fut = (hook)(
                    feature.as_ref(),
                    rule.as_ref().map(AsRef::as_ref),
                    scenario.as_ref(),
                    &mut world,
                );
                match AssertUnwindSafe(fut).catch_unwind().await {
                    Ok(()) => Ok(world),
                    Err(i) => Err((Info::from(i), Some(world))),
                }
            });

            match fut.await {
                Ok(world) => {
                    self.send_event(event::Cucumber::scenario(
                        Arc::clone(feature),
                        rule.map(Arc::clone),
                        Arc::clone(scenario),
                        event::Scenario::hook_passed(HookType::Before),
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
        world: Option<W>,
        step: Arc<gherkin::Step>,
        is_background: bool,
        (started, passed, skipped): (St, Ps, Sk),
    ) -> Result<W, ExecutionFailure<W>>
    where
        St: FnOnce(Arc<gherkin::Step>) -> event::Cucumber<W>,
        Ps: FnOnce(Arc<gherkin::Step>, CaptureLocations) -> event::Cucumber<W>,
        Sk: FnOnce(Arc<gherkin::Step>) -> event::Cucumber<W>,
    {
        self.send_event(started(Arc::clone(&step)));

        let run = async {
            let (step_fn, captures, ctx) = match self.collection.find(&step) {
                Ok(Some(f)) => f,
                Ok(None) => return Ok((None, world)),
                Err(e) => {
                    let e = event::StepError::AmbiguousMatch(e);
                    return Err((e, None, world));
                }
            };

            let mut world = if let Some(w) = world {
                w
            } else {
                match AssertUnwindSafe(W::new()).catch_unwind().await {
                    Ok(Ok(w)) => w,
                    Ok(Err(e)) => {
                        let e = event::StepError::Panic(coerce_into_info(
                            format!("failed to initialize World: {e}"),
                        ));
                        return Err((e, None, None));
                    }
                    Err(e) => {
                        let e = event::StepError::Panic(e.into());
                        return Err((e, None, None));
                    }
                }
            };

            match AssertUnwindSafe(step_fn(&mut world, ctx))
                .catch_unwind()
                .await
            {
                Ok(()) => Ok((Some(captures), Some(world))),
                Err(e) => {
                    let e = event::StepError::Panic(e.into());
                    Err((e, Some(captures), Some(world)))
                }
            }
        };

        #[allow(clippy::shadow_unrelated)]
        match run.await {
            Ok((Some(captures), Some(world))) => {
                self.send_event(passed(step, captures));
                Ok(world)
            }
            Ok((_, world)) => {
                self.send_event(skipped(step));
                Err(ExecutionFailure::StepSkipped(world))
            }
            Err((err, captures, world)) => {
                Err(ExecutionFailure::StepPanicked {
                    world,
                    step,
                    captures,
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
        feature: Arc<gherkin::Feature>,
        rule: Option<Arc<gherkin::Rule>>,
        scenario: Arc<gherkin::Scenario>,
        world: Option<Arc<W>>,
        err: ExecutionFailure<W>,
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
                        ),
                    ),
                    meta,
                );
            }
            ExecutionFailure::StepPanicked {
                step,
                captures,
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
                        step, captures, world, error,
                    ),
                ),
                meta,
            ),
            ExecutionFailure::StepPanicked {
                step,
                captures,
                err: error,
                meta,
                is_background: false,
                ..
            } => self.send_event_with_meta(
                event::Cucumber::scenario(
                    feature,
                    rule,
                    scenario,
                    event::Scenario::step_failed(step, captures, world, error),
                ),
                meta,
            ),
        }
    }

    /// Executes the [`HookType::After`], if present.
    ///
    /// Doesn't emit any events, see [`Self::emit_failed_events()`] for more
    /// details.
    async fn run_after_hook(
        &self,
        mut world: Option<W>,
        feature: &Arc<gherkin::Feature>,
        rule: Option<&Arc<gherkin::Rule>>,
        scenario: &Arc<gherkin::Scenario>,
    ) -> Result<
        (Option<W>, Option<AfterHookEventsMeta>),
        (Option<W>, AfterHookEventsMeta, Info),
    > {
        if let Some(hook) = self.after_hook.as_ref() {
            let fut = (hook)(
                feature.as_ref(),
                rule.as_ref().map(AsRef::as_ref),
                scenario.as_ref(),
                world.as_mut(),
            );

            let started = event::Metadata::new(());
            let res = AssertUnwindSafe(fut).catch_unwind().await;
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
    fn emit_after_hook_events(
        &self,
        feature: Arc<gherkin::Feature>,
        rule: Option<Arc<gherkin::Rule>>,
        scenario: Arc<gherkin::Scenario>,
        world: Option<Arc<W>>,
        meta: Option<AfterHookEventsMeta>,
        err: Option<Info>,
    ) {
        debug_assert_eq!(self.after_hook.is_some(), meta.is_some());

        if let Some(meta) = meta {
            self.send_event_with_meta(
                event::Cucumber::scenario(
                    Arc::clone(&feature),
                    rule.clone(),
                    Arc::clone(&scenario),
                    event::Scenario::hook_started(HookType::After),
                ),
                meta.started,
            );

            let ev = if let Some(err) = err {
                event::Cucumber::scenario(
                    feature,
                    rule,
                    scenario,
                    event::Scenario::hook_failed(HookType::After, world, err),
                )
            } else {
                event::Cucumber::scenario(
                    feature,
                    rule,
                    scenario,
                    event::Scenario::hook_passed(HookType::After),
                )
            };

            self.send_event_with_meta(ev, meta.finished);
        }
    }

    /// Notifies [`FinishedRulesAndFeatures`] about [`Scenario`] being finished.
    ///
    /// [`Scenario`]: gherkin::Scenario
    fn scenario_finished(
        &self,
        feature: Arc<gherkin::Feature>,
        rule: Option<Arc<gherkin::Rule>>,
        is_failed: Failed,
    ) {
        // If the receiver end is dropped, then no one listens for events
        // so we can just ignore it.
        drop(
            self.finished_sender
                .unbounded_send((feature, rule, is_failed)),
        );
    }

    /// Notifies with the given [`Cucumber`] event.
    ///
    /// [`Cucumber`]: event::Cucumber
    fn send_event(&self, event: event::Cucumber<W>) {
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
    fn send_all_events(
        &self,
        events: impl Iterator<Item = event::Cucumber<W>>,
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

/// Stores currently running [`Rule`]s and [`Feature`]s and notifies about their
/// state of completion.
///
/// [`Feature`]: gherkin::Feature
/// [`Rule`]: gherkin::Rule
struct FinishedRulesAndFeatures {
    /// Number of finished [`Scenario`]s of [`Feature`].
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Scenario`]: gherkin::Scenario
    features_scenarios_count: HashMap<Arc<gherkin::Feature>, usize>,

    /// Number of finished [`Scenario`]s of [`Rule`].
    ///
    /// We also store path to a [`Feature`], so [`Rule`]s with same names and
    /// spans in different `.feature` files will have different hashes.
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Rule`]: gherkin::Rule
    /// [`Scenario`]: gherkin::Scenario
    rule_scenarios_count:
        HashMap<(Arc<gherkin::Feature>, Arc<gherkin::Rule>), usize>,

    /// Receiver for notifying state of [`Scenario`]s completion.
    ///
    /// [`Scenario`]: gherkin::Scenario
    finished_receiver: mpsc::UnboundedReceiver<(
        Arc<gherkin::Feature>,
        Option<Arc<gherkin::Rule>>,
        Failed,
    )>,
}

impl FinishedRulesAndFeatures {
    /// Creates a new [`FinishedRulesAndFeatures`] store.
    fn new(
        finished_receiver: mpsc::UnboundedReceiver<(
            Arc<gherkin::Feature>,
            Option<Arc<gherkin::Rule>>,
            Failed,
        )>,
    ) -> Self {
        Self {
            features_scenarios_count: HashMap::new(),
            rule_scenarios_count: HashMap::new(),
            finished_receiver,
        }
    }

    /// Marks [`Rule`]'s [`Scenario`] as finished and returns [`Rule::Finished`]
    /// event if no [`Scenario`]s left.
    ///
    /// [`Rule`]: gherkin::Rule
    /// [`Rule::Finished`]: event::Rule::Finished
    /// [`Scenario`]: gherkin::Scenario
    fn rule_scenario_finished<W>(
        &mut self,
        feature: Arc<gherkin::Feature>,
        rule: Arc<gherkin::Rule>,
    ) -> Option<event::Cucumber<W>> {
        let finished_scenarios = self
            .rule_scenarios_count
            .get_mut(&(Arc::clone(&feature), Arc::clone(&rule)))
            .unwrap_or_else(|| panic!("No Rule {}", rule.name));
        *finished_scenarios += 1;
        (rule.scenarios.len() == *finished_scenarios).then(|| {
            let _ = self
                .rule_scenarios_count
                .remove(&(Arc::clone(&feature), Arc::clone(&rule)));
            event::Cucumber::rule_finished(feature, rule)
        })
    }

    /// Marks [`Feature`]'s [`Scenario`] as finished and returns
    /// [`Feature::Finished`] event if no [`Scenario`]s left.
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Feature::Finished`]: event::Feature::Finished
    /// [`Scenario`]: gherkin::Scenario
    fn feature_scenario_finished<W>(
        &mut self,
        feature: Arc<gherkin::Feature>,
    ) -> Option<event::Cucumber<W>> {
        let finished_scenarios = self
            .features_scenarios_count
            .get_mut(&feature)
            .unwrap_or_else(|| panic!("No Feature {}", feature.name));
        *finished_scenarios += 1;
        let scenarios = feature.count_scenarios();
        (scenarios == *finished_scenarios).then(|| {
            let _ = self.features_scenarios_count.remove(&feature);
            event::Cucumber::feature_finished(feature)
        })
    }

    /// Marks all the unfinished [`Rule`]s and [`Feature`]s as finished, and
    /// returns all the appropriate finished events.
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Rule`]: gherkin::Rule
    fn finish_all_rules_and_features<W>(
        &mut self,
    ) -> impl Iterator<Item = event::Cucumber<W>> + '_ {
        self.rule_scenarios_count
            .drain()
            .map(|((feat, rule), _)| event::Cucumber::rule_finished(feat, rule))
            .chain(
                self.features_scenarios_count
                    .drain()
                    .map(|(feat, _)| event::Cucumber::feature_finished(feat)),
            )
    }

    /// Marks [`Scenario`]s as started and returns [`Rule::Started`] and
    /// [`Feature::Started`] if given [`Scenario`] was first for particular
    /// [`Rule`] or [`Feature`].
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Feature::Started`]: event::Feature::Started
    /// [`Rule`]: gherkin::Rule
    /// [`Rule::Started`]: event::Rule::Started
    /// [`Scenario`]: gherkin::Scenario
    fn start_scenarios<W>(
        &mut self,
        runnable: impl AsRef<
            [(
                Arc<gherkin::Feature>,
                Option<Arc<gherkin::Rule>>,
                Arc<gherkin::Scenario>,
            )],
        >,
    ) -> impl Iterator<Item = event::Cucumber<W>> {
        let runnable = runnable.as_ref();

        let mut started_features = Vec::new();
        for feature in runnable.iter().map(|(f, ..)| Arc::clone(f)).dedup() {
            let _ = self
                .features_scenarios_count
                .entry(Arc::clone(&feature))
                .or_insert_with(|| {
                    started_features.push(feature);
                    0
                });
        }

        let mut started_rules = Vec::new();
        for (feat, rule) in runnable
            .iter()
            .filter_map(|(feat, rule, _)| {
                rule.clone().map(|r| (Arc::clone(feat), r))
            })
            .dedup()
        {
            let _ = self
                .rule_scenarios_count
                .entry((Arc::clone(&feat), Arc::clone(&rule)))
                .or_insert_with(|| {
                    started_rules.push((feat, rule));
                    0
                });
        }

        started_features
            .into_iter()
            .map(event::Cucumber::feature_started)
            .chain(
                started_rules
                    .into_iter()
                    .map(|(f, r)| event::Cucumber::rule_started(f, r)),
            )
    }
}

/// [`Scenario`]s storage.
///
/// [`Scenario`]: gherkin::Scenario
type Scenarios = HashMap<
    ScenarioType,
    Vec<(
        Arc<gherkin::Feature>,
        Option<Arc<gherkin::Rule>>,
        Arc<gherkin::Scenario>,
    )>,
>;

/// Storage sorted by [`ScenarioType`] [`Feature`]'s [`Scenario`]s.
///
/// [`Feature`]: gherkin::Feature
/// [`Scenario`]: gherkin::Scenario
#[derive(Clone, Default)]
struct Features {
    /// Storage itself.
    scenarios: Arc<Mutex<Scenarios>>,

    /// Indicates whether all parsed [`Feature`]s are sorted and stored.
    ///
    /// [`Feature`]: gherkin::Feature
    finished: Arc<AtomicBool>,
}

impl Features {
    /// Splits [`Feature`] into [`Scenario`]s, sorts by [`ScenarioType`] and
    /// stores them.
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Scenario`]: gherkin::Scenario
    async fn insert<Which>(
        &self,
        feature: gherkin::Feature,
        which_scenario: &Which,
    ) where
        Which: Fn(
                &gherkin::Feature,
                Option<&gherkin::Rule>,
                &gherkin::Scenario,
            ) -> ScenarioType
            + 'static,
    {
        let local = feature
            .scenarios
            .iter()
            .map(|s| (&feature, None, s))
            .chain(feature.rules.iter().flat_map(|r| {
                r.scenarios
                    .iter()
                    .map(|s| (&feature, Some(r), s))
                    .collect::<Vec<_>>()
            }))
            .map(|(feat, rule, scenario)| {
                (
                    Arc::new(feat.clone()),
                    rule.map(|r| Arc::new(r.clone())),
                    Arc::new(scenario.clone()),
                )
            })
            .into_group_map_by(|(f, r, s)| {
                which_scenario(f, r.as_ref().map(AsRef::as_ref), s)
            });

        let mut scenarios = self.scenarios.lock().await;
        if local.get(&ScenarioType::Serial).is_none() {
            // If there are no Serial Scenarios we just extending already
            // existing Concurrent Scenarios.
            for (which, values) in local {
                scenarios.entry(which).or_default().extend(values);
            }
        } else {
            // If there are Serial Scenarios we insert all Serial and Concurrent
            // Scenarios in front.
            // This is done to execute them closely to one another, so the
            // output wouldn't hang on executing other Concurrent Scenarios.
            for (which, mut values) in local {
                let old = mem::take(scenarios.entry(which).or_default());
                values.extend(old);
                scenarios.entry(which).or_default().extend(values);
            }
        }
    }

    /// Returns [`Scenario`]s which are ready to run.
    ///
    /// [`Scenario`]: gherkin::Scenario
    async fn get(
        &self,
        max_concurrent_scenarios: Option<usize>,
    ) -> Vec<(
        Arc<gherkin::Feature>,
        Option<Arc<gherkin::Rule>>,
        Arc<gherkin::Scenario>,
    )> {
        let mut scenarios = self.scenarios.lock().await;
        scenarios
            .get_mut(&ScenarioType::Serial)
            .and_then(|s| (!s.is_empty()).then(|| vec![s.remove(0)]))
            .or_else(|| {
                scenarios.get_mut(&ScenarioType::Concurrent).and_then(|s| {
                    (!s.is_empty()).then(|| {
                        let end = cmp::min(
                            s.len(),
                            max_concurrent_scenarios.unwrap_or(s.len()),
                        );
                        s.drain(0..end).collect()
                    })
                })
            })
            .unwrap_or_default()
    }

    /// Marks that there will be no more [`Feature`]s to execute.
    ///
    /// [`Feature`]: gherkin::Feature
    fn finish(&self) {
        self.finished.store(true, Ordering::SeqCst);
    }

    /// Indicates whether there are more [`Feature`]s to execute.
    ///
    /// [`Feature`]: gherkin::Feature
    fn is_finished(&self) -> bool {
        self.finished.load(Ordering::SeqCst)
    }
}

/// Coerces the given `value` into a type-erased [`Info`].
fn coerce_into_info<T: std::any::Any + Send + 'static>(val: T) -> Info {
    Arc::new(val)
}

/// Failure encountered during execution of [`HookType::Before`] or [`Step`].
/// See [`Executor::emit_failed_events()`] for more info.
///
/// [`Step`]: gherkin::Step
enum ExecutionFailure<World> {
    /// [`HookType::Before`] panicked.
    BeforeHookPanicked {
        /// [`World`] at the time [`HookType::Before`] has panicked.
        world: Option<World>,

        /// [`catch_unwind()`] of the [`HookType::Before`] panic.
        ///
        /// [`catch_unwind()`]: std::panic::catch_unwind
        panic_info: Info,

        /// [`Metadata`] at the time [`HookType::Before`] panicked.
        ///
        /// [`Metadata`]: event::Metadata
        meta: event::Metadata,
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
        step: Arc<gherkin::Step>,

        /// [`Step`]s [`regex`] [`CaptureLocations`].
        ///
        /// [`Step`]: gherkin::Step
        captures: Option<CaptureLocations>,

        /// [`StepError`] of the [`Step`].
        ///
        /// [`Step`]: gherkin::Step
        /// [`StepError`]: event::StepError
        err: event::StepError,

        /// [`Metadata`] at the time when [`Step`] failed.
        ///
        /// [`Metadata`]: event::Metadata
        /// [`Step`]: gherkin::Step.
        meta: event::Metadata,

        /// Indicator whether the [`Step`] was background or not.
        ///
        /// [`Step`]: gherkin::Step
        is_background: bool,
    },
}

/// [`Metadata`] of [`HookType::After`] events.
///
/// [`Metadata`]: event::Metadata
struct AfterHookEventsMeta {
    /// [`Metadata`] at the time [`HookType::After`] started.
    ///
    /// [`Metadata`]: event::Metadata
    started: event::Metadata,

    /// [`Metadata`] at the time [`HookType::After`] finished.
    ///
    /// [`Metadata`]: event::Metadata
    finished: event::Metadata,
}

impl<W> ExecutionFailure<W> {
    /// Takes the [`World`] leaving a [`None`] in its place.
    fn take_world(&mut self) -> Option<W> {
        match self {
            Self::BeforeHookPanicked { world, .. }
            | Self::StepSkipped(world)
            | Self::StepPanicked { world, .. } => world.take(),
        }
    }
}
