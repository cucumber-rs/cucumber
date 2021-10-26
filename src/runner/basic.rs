// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>,
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
    convert::identity,
    fmt, mem,
    panic::{self, AssertUnwindSafe},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
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
use structopt::StructOpt;

use crate::{
    event::{self, HookType, Info},
    feature::Ext as _,
    parser, step, Event, Runner, Step, World,
};

// Workaround for overwritten doc-comments.
// https://github.com/TeXitoi/structopt/issues/333#issuecomment-712265332
#[cfg_attr(doc, doc = "CLI options of a [`Basic`] [`Runner`].")]
#[cfg_attr(
    not(doc),
    allow(clippy::missing_docs_in_private_items, missing_docs)
)]
#[derive(Clone, Copy, Debug, StructOpt)]
pub struct Cli {
    /// Number of scenarios to run concurrently. If not specified, uses the
    /// value configured in tests runner, or 64 by default.
    #[structopt(long, short, name = "int")]
    pub concurrency: Option<usize>,
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
                .then(|| ScenarioType::Serial)
                .unwrap_or(ScenarioType::Concurrent)
        };

        Self {
            max_concurrent_scenarios: Some(64),
            steps: step::Collection::new(),
            which_scenario,
            before_hook: None,
            after_hook: None,
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

    /// Function determining whether a [`Scenario`] is [`Concurrent`] or
    /// a [`Serial`] one.
    ///
    /// [`Concurrent`]: ScenarioType::Concurrent
    /// [`Serial`]: ScenarioType::Serial
    /// [`Scenario`]: gherkin::Scenario
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
            ..
        } = self;
        Basic {
            max_concurrent_scenarios,
            steps,
            which_scenario: func,
            before_hook,
            after_hook,
        }
    }

    /// Sets a hook, executed on each [`Scenario`] before running all its
    /// [`Step`]s, including [`Background`] ones.
    ///
    /// [`Background`]: gherkin::Background
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
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
            ..
        } = self;
        Basic {
            max_concurrent_scenarios,
            steps,
            which_scenario,
            before_hook: Some(func),
            after_hook,
        }
    }

    /// Sets hook, executed on each [`Scenario`] after running all its
    /// [`Step`]s, even after [`Skipped`] of [`Failed`] ones.
    ///
    /// Last `World` argument is supplied to the function, in case it was
    /// initialized before by running [`before`] hook or any non-failed
    /// [`Step`]. In case the last [`Scenario`]'s [`Step`] failed, we want to
    /// return event with an exact `World` state. Also, we don't want to impose
    /// additional [`Clone`] bounds on `World`, so the only option left is to
    /// pass [`None`] to the function.
    ///
    /// [`before`]: Self::before()
    /// [`Failed`]: event::Step::Failed
    /// [`Scenario`]: gherkin::Scenario
    /// [`Skipped`]: event::Step::Skipped
    /// [`Step`]: gherkin::Step
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
            ..
        } = self;
        Basic {
            max_concurrent_scenarios,
            steps,
            which_scenario,
            before_hook,
            after_hook: Some(func),
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
        } = self;

        let buffer = Features::default();
        let (sender, receiver) = mpsc::unbounded();

        let insert = insert_features(
            buffer.clone(),
            features,
            which_scenario,
            sender.clone(),
        );
        let execute = execute(
            buffer,
            cli.concurrency.or(max_concurrent_scenarios),
            steps,
            sender,
            before_hook,
            after_hook,
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
    features: S,
    which_scenario: F,
    sender: mpsc::UnboundedSender<parser::Result<Event<event::Cucumber<W>>>>,
) where
    S: Stream<Item = parser::Result<gherkin::Feature>> + 'static,
    F: Fn(
            &gherkin::Feature,
            Option<&gherkin::Rule>,
            &gherkin::Scenario,
        ) -> ScenarioType
        + 'static,
{
    pin_mut!(features);
    while let Some(feat) = features.next().await {
        match feat {
            Ok(f) => into.insert(f, &which_scenario).await,
            // If the receiver end is dropped, then no one listens for events
            // so we can just stop from here.
            Err(e) => {
                if sender.unbounded_send(Err(e)).is_err() {
                    break;
                }
            }
        }
    }

    into.finish();
}

/// Retrieves [`Feature`]s and executes them.
///
/// [`Feature`]: gherkin::Feature
async fn execute<W, Before, After>(
    features: Features,
    max_concurrent_scenarios: Option<usize>,
    collection: step::Collection<W>,
    sender: mpsc::UnboundedSender<parser::Result<Event<event::Cucumber<W>>>>,
    before_hook: Option<Before>,
    after_hook: Option<After>,
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
    // 3. We return original panic hook, because suppressing all panics doesn't
    //    sound like a very good idea.
    let hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));

    let mut executor =
        Executor::new(collection, before_hook, after_hook, sender);

    executor.send(event::Cucumber::Started);

    loop {
        let runnable = features.get(max_concurrent_scenarios).await;
        if runnable.is_empty() {
            if features.is_finished() {
                break;
            }
            continue;
        }

        let started = executor.start_scenarios(&runnable);
        executor.send_all(started);

        drop(
            runnable
                .into_iter()
                .map(|(f, r, s)| executor.run_scenario(f, r, s))
                .collect::<future::JoinAll<_>>()
                .await,
        );

        executor.cleanup_finished_rules_and_features();
    }

    executor.send(event::Cucumber::Finished);

    panic::set_hook(hook);
}

/// Stores currently ran [`Feature`]s and notifies about their state of
/// completion.
///
/// [`Feature`]: gherkin::Feature.
struct Executor<W, Before, After> {
    /// Number of finished [`Scenario`]s of [`Feature`].
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Scenario`]: gherkin::Scenario
    features_scenarios_count: HashMap<Arc<gherkin::Feature>, AtomicUsize>,

    /// Number of finished [`Scenario`]s of [`Rule`].
    ///
    /// We also store path to `.feature` file so [`Rule`]s with same names and
    /// spans in different files will have different hashes.
    ///
    /// [`Rule`]: gherkin::Rule
    /// [`Scenario`]: gherkin::Scenario
    rule_scenarios_count:
        HashMap<(Option<PathBuf>, Arc<gherkin::Rule>), AtomicUsize>,

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

    /// Sender for notifying state of [`Feature`]s completion.
    ///
    /// [`Feature`]: gherkin::Feature
    sender: mpsc::UnboundedSender<parser::Result<Event<event::Cucumber<W>>>>,
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
    fn new(
        collection: step::Collection<W>,
        before_hook: Option<Before>,
        after_hook: Option<After>,
        sender: mpsc::UnboundedSender<
            parser::Result<Event<event::Cucumber<W>>>,
        >,
    ) -> Self {
        Self {
            features_scenarios_count: HashMap::new(),
            rule_scenarios_count: HashMap::new(),
            collection,
            before_hook,
            after_hook,
            sender,
        }
    }

    /// Runs a [`Scenario`].
    ///
    /// # Events
    ///
    /// - Emits all [`Scenario`] events.
    /// - If [`Scenario`] was last for particular [`Rule`] or [`Feature`], also
    ///   emits finishing events for them.
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
        let err = |e: fn(_, _, _, _) -> event::Scenario<W>| {
            let (f, r, s) = (&feature, &rule, &scenario);
            move |step, captures, w, info| {
                let (f, r, s) = (Arc::clone(f), r.clone(), Arc::clone(s));
                event::Cucumber::scenario(f, r, s, e(step, captures, w, info))
            }
        };

        let compose = |started, passed, skipped, failed| {
            (ok(started), ok_capt(passed), ok(skipped), err(failed))
        };
        let into_bg_step_ev = compose(
            event::Scenario::background_step_started,
            event::Scenario::background_step_passed,
            event::Scenario::background_step_skipped,
            event::Scenario::background_step_failed,
        );
        let into_step_ev = compose(
            event::Scenario::step_started,
            event::Scenario::step_passed,
            event::Scenario::step_skipped,
            event::Scenario::step_failed,
        );

        self.send(event::Cucumber::scenario(
            Arc::clone(&feature),
            rule.clone(),
            Arc::clone(&scenario),
            event::Scenario::Started,
        ));

        let world = async {
            let before_hook = self
                .run_before_hook(&feature, rule.as_ref(), &scenario)
                .await
                .map_err(|_unit| None)?;

            let feature_background = feature
                .background
                .as_ref()
                .map(|b| b.steps.iter().map(|s| Arc::new(s.clone())))
                .into_iter()
                .flatten();

            let feature_background = stream::iter(feature_background)
                .map(Ok)
                .try_fold(before_hook, |world, bg_step| {
                    self.run_step(world, bg_step, into_bg_step_ev).map_ok(Some)
                })
                .await?;

            let rule_background = rule
                .as_ref()
                .map(|rule| {
                    rule.background
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
                    self.run_step(world, bg_step, into_bg_step_ev).map_ok(Some)
                })
                .await?;

            stream::iter(scenario.steps.iter().map(|s| Arc::new(s.clone())))
                .map(Ok)
                .try_fold(rule_background, |world, step| {
                    self.run_step(world, step, into_step_ev).map_ok(Some)
                })
                .await
        }
        .await
        .unwrap_or_else(identity);

        self.run_after_hook(world, &feature, rule.as_ref(), &scenario)
            .await
            .map_or((), drop);

        self.send(event::Cucumber::scenario(
            Arc::clone(&feature),
            rule.clone(),
            Arc::clone(&scenario),
            event::Scenario::Finished,
        ));

        if let Some(r) = rule {
            if let Some(f) =
                self.rule_scenario_finished(Arc::clone(&feature), r)
            {
                self.send(f);
            }
        }

        if let Some(f) = self.feature_scenario_finished(feature) {
            self.send(f);
        }
    }

    /// Executes [`HookType::Before`], if present.
    async fn run_before_hook(
        &self,
        feature: &Arc<gherkin::Feature>,
        rule: Option<&Arc<gherkin::Rule>>,
        scenario: &Arc<gherkin::Scenario>,
    ) -> Result<Option<W>, ()> {
        let init_world = async {
            AssertUnwindSafe(W::new())
                .catch_unwind()
                .await
                .map_err(Info::from)
                .and_then(|r| {
                    r.map_err(|e| {
                        coerce_into_info(format!(
                            "failed to initialize World: {}",
                            e,
                        ))
                    })
                })
                .map_err(|info| (info, None))
        };

        if let Some(hook) = self.before_hook.as_ref() {
            self.send(event::Cucumber::scenario(
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
                    self.send(event::Cucumber::scenario(
                        Arc::clone(feature),
                        rule.map(Arc::clone),
                        Arc::clone(scenario),
                        event::Scenario::hook_passed(HookType::Before),
                    ));
                    Ok(Some(world))
                }
                Err((info, world)) => {
                    self.send(event::Cucumber::scenario(
                        Arc::clone(feature),
                        rule.map(Arc::clone),
                        Arc::clone(scenario),
                        event::Scenario::hook_failed(
                            HookType::Before,
                            world.map(Arc::new),
                            info,
                        ),
                    ));
                    Err(())
                }
            }
        } else {
            Ok(None)
        }
    }

    /// Executes [`HookType::After`], if present.
    async fn run_after_hook(
        &self,
        mut world: Option<W>,
        feature: &Arc<gherkin::Feature>,
        rule: Option<&Arc<gherkin::Rule>>,
        scenario: &Arc<gherkin::Scenario>,
    ) -> Result<Option<W>, ()> {
        if let Some(hook) = self.after_hook.as_ref() {
            self.send(event::Cucumber::scenario(
                Arc::clone(feature),
                rule.map(Arc::clone),
                Arc::clone(scenario),
                event::Scenario::hook_started(HookType::After),
            ));

            let fut = async {
                let fut = (hook)(
                    feature.as_ref(),
                    rule.as_ref().map(AsRef::as_ref),
                    scenario.as_ref(),
                    world.as_mut(),
                );
                match AssertUnwindSafe(fut).catch_unwind().await {
                    Ok(()) => Ok(world),
                    Err(info) => Err((info, world)),
                }
            };

            match fut.await {
                Ok(world) => {
                    self.send(event::Cucumber::scenario(
                        Arc::clone(feature),
                        rule.map(Arc::clone),
                        Arc::clone(scenario),
                        event::Scenario::hook_passed(HookType::After),
                    ));
                    Ok(world)
                }
                Err((info, world)) => {
                    self.send(event::Cucumber::scenario(
                        Arc::clone(feature),
                        rule.map(Arc::clone),
                        Arc::clone(scenario),
                        event::Scenario::hook_failed(
                            HookType::After,
                            world.map(Arc::new),
                            info.into(),
                        ),
                    ));
                    Err(())
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
    /// - Emits all [`Step`] events.
    ///
    /// [`Step`]: gherkin::Step
    async fn run_step<St, Ps, Sk, F>(
        &self,
        world: Option<W>,
        step: Arc<gherkin::Step>,
        (started, passed, skipped, failed): (St, Ps, Sk, F),
    ) -> Result<W, Option<W>>
    where
        St: FnOnce(Arc<gherkin::Step>) -> event::Cucumber<W>,
        Ps: FnOnce(Arc<gherkin::Step>, CaptureLocations) -> event::Cucumber<W>,
        Sk: FnOnce(Arc<gherkin::Step>) -> event::Cucumber<W>,
        F: FnOnce(
            Arc<gherkin::Step>,
            Option<CaptureLocations>,
            Option<Arc<W>>,
            event::StepError,
        ) -> event::Cucumber<W>,
    {
        self.send(started(Arc::clone(&step)));

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
                            format!("failed to initialize World: {}", e),
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

        match run.await {
            Ok((Some(captures), Some(world))) => {
                self.send(passed(step, captures));
                Ok(world)
            }
            Ok((_, world)) => {
                self.send(skipped(step));
                Err(world)
            }
            Err((err, captures, world)) => {
                self.send(failed(step, captures, world.map(Arc::new), err));
                Err(None)
            }
        }
    }

    /// Marks [`Rule`]'s [`Scenario`] as finished and returns [`Rule::Finished`]
    /// event if no [`Scenario`]s left.
    ///
    /// [`Rule`]: gherkin::Rule
    /// [`Rule::Finished`]: event::Rule::Finished
    /// [`Scenario`]: gherkin::Scenario
    fn rule_scenario_finished(
        &self,
        feature: Arc<gherkin::Feature>,
        rule: Arc<gherkin::Rule>,
    ) -> Option<event::Cucumber<W>> {
        let finished_scenarios = self
            .rule_scenarios_count
            .get(&(feature.path.clone(), Arc::clone(&rule)))
            .unwrap_or_else(|| panic!("No Rule {}", rule.name))
            .fetch_add(1, Ordering::SeqCst)
            + 1;
        (rule.scenarios.len() == finished_scenarios)
            .then(|| event::Cucumber::rule_finished(feature, rule))
    }

    /// Marks [`Feature`]'s [`Scenario`] as finished and returns
    /// [`Feature::Finished`] event if no [`Scenario`]s left.
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Feature::Finished`]: event::Feature::Finished
    /// [`Scenario`]: gherkin::Scenario
    fn feature_scenario_finished(
        &self,
        feature: Arc<gherkin::Feature>,
    ) -> Option<event::Cucumber<W>> {
        let finished_scenarios = self
            .features_scenarios_count
            .get(&feature)
            .unwrap_or_else(|| panic!("No Feature {}", feature.name))
            .fetch_add(1, Ordering::SeqCst)
            + 1;
        let scenarios = feature.count_scenarios();
        (scenarios == finished_scenarios)
            .then(|| event::Cucumber::feature_finished(feature))
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
    fn start_scenarios(
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
                    0.into()
                });
        }

        let mut started_rules = Vec::new();
        for (feature, rule) in runnable
            .iter()
            .filter_map(|(f, r, _)| r.clone().map(|r| (Arc::clone(f), r)))
            .dedup()
        {
            let _ = self
                .rule_scenarios_count
                .entry((feature.path.clone(), Arc::clone(&rule)))
                .or_insert_with(|| {
                    started_rules.push((feature, rule));
                    0.into()
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

    /// Removes all finished [`Rule`]s and [`Feature`]s as all their events are
    /// emitted already.
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Rule`]: gherkin::Rule
    fn cleanup_finished_rules_and_features(&mut self) {
        self.features_scenarios_count = self
            .features_scenarios_count
            .drain()
            .filter(|(f, count)| {
                f.count_scenarios() != count.load(Ordering::SeqCst)
            })
            .collect();

        self.rule_scenarios_count = self
            .rule_scenarios_count
            .drain()
            .filter(|((_, r), count)| {
                r.scenarios.len() != count.load(Ordering::SeqCst)
            })
            .collect();
    }

    /// Notifies with the given [`Cucumber`] event.
    ///
    /// [`Cucumber`]: event::Cucumber
    fn send(&self, event: event::Cucumber<W>) {
        // If the receiver end is dropped, then no one listens for events
        // so we can just ignore it.
        drop(self.sender.unbounded_send(Ok(Event::new(event))));
    }

    /// Notifies with the given [`Cucumber`] events.
    ///
    /// [`Cucumber`]: event::Cucumber
    fn send_all(&self, events: impl Iterator<Item = event::Cucumber<W>>) {
        for ev in events {
            // If the receiver end is dropped, then no one listens for events
            // so we can just stop from here.
            if self.sender.unbounded_send(Ok(Event::new(ev))).is_err() {
                break;
            }
        }
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
            .map(|(f, r, s)| {
                (
                    Arc::new(f.clone()),
                    r.map(|r| Arc::new(r.clone())),
                    Arc::new(s.clone()),
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
            .and_then(|s| s.pop().map(|s| vec![s]))
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
