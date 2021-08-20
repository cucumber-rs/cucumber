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
    future::{self, Either, FutureExt as _},
    lock::Mutex,
    pin_mut,
    stream::{self, LocalBoxStream, Stream, StreamExt as _, TryStreamExt as _},
    TryFutureExt,
};
use itertools::Itertools as _;
use regex::Regex;

use crate::{
    event::{self, Info},
    feature::Ext as _,
    parser, step, Runner, Step, World,
};

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

/// Default [`Runner`] implementation which follows _order guarantees_ from the
/// [`Runner`] trait docs.
///
/// Executes [`Scenario`]s concurrently based on the custom function, which
/// returns [`ScenarioType`]. Also, can limit maximum number of concurrent
/// [`Scenario`]s.
///
/// [`Scenario`]: gherkin::Scenario
pub struct Basic<World, F> {
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
}

impl<World, F> fmt::Debug for Basic<World, F> {
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
    pub fn custom() -> Basic<World, ()> {
        Self {
            max_concurrent_scenarios: None,
            steps: step::Collection::new(),
            which_scenario: (),
        }
    }
}

impl<World, F> Basic<World, F> {
    /// If `max` is [`Some`] number of concurrently executed [`Scenario`]s will
    /// be limited.
    ///
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    pub fn max_concurrent_scenarios(mut self, max: Option<usize>) -> Self {
        self.max_concurrent_scenarios = max;
        self
    }

    /// Function determining whether a [`Scenario`] is [`Concurrent`] or
    /// a [`Serial`] one.
    ///
    /// [`Concurrent`]: ScenarioType::Concurrent
    /// [`Serial`]: ScenarioType::Serial
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    pub fn which_scenario<Which>(self, func: Which) -> Basic<World, Which>
    where
        Which: Fn(
                &gherkin::Feature,
                Option<&gherkin::Rule>,
                &gherkin::Scenario,
            ) -> ScenarioType
            + 'static,
    {
        let Self {
            max_concurrent_scenarios,
            steps,
            ..
        } = self;
        Basic {
            max_concurrent_scenarios,
            steps,
            which_scenario: func,
        }
    }

    /// Replaces [`Collection`] of [`Step`]s with the given one.
    ///
    /// [`Collection`]: step::Collection
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
        self.steps = mem::take(&mut self.steps).given(regex, step);
        self
    }

    /// Adds a [When] [`Step`] matching the given `regex`.
    ///
    /// [When]: https://cucumber.io/docs/gherkin/reference/#given
    #[must_use]
    pub fn when(mut self, regex: Regex, step: Step<World>) -> Self {
        self.steps = mem::take(&mut self.steps).when(regex, step);
        self
    }

    /// Adds a [Then] [`Step`] matching the given `regex`.
    ///
    /// [Then]: https://cucumber.io/docs/gherkin/reference/#then
    #[must_use]
    pub fn then(mut self, regex: Regex, step: Step<World>) -> Self {
        self.steps = mem::take(&mut self.steps).then(regex, step);
        self
    }
}

impl<W, F> Runner<W> for Basic<W, F>
where
    W: World,
    F: Fn(
            &gherkin::Feature,
            Option<&gherkin::Rule>,
            &gherkin::Scenario,
        ) -> ScenarioType
        + 'static,
{
    type EventStream = LocalBoxStream<'static, event::Cucumber<W>>;

    fn run<S>(self, features: S) -> Self::EventStream
    where
        S: Stream<Item = parser::Result<gherkin::Feature>> + 'static,
    {
        let Self {
            max_concurrent_scenarios,
            steps,
            which_scenario,
        } = self;

        let buffer = Features::default();
        let (sender, receiver) = mpsc::unbounded();

        let insert = insert_features(
            buffer.clone(),
            features,
            which_scenario,
            sender.clone(),
        );
        let execute = execute(buffer, max_concurrent_scenarios, steps, sender);

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
async fn insert_features<S, F, W>(
    into: Features,
    features: S,
    which_scenario: F,
    sender: mpsc::UnboundedSender<event::Cucumber<W>>,
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
    while let Some(feature) = features.next().await {
        match feature {
            Ok(f) => into.insert(f, &which_scenario).await,
            Err(e) => sender
                .unbounded_send(event::Cucumber::ParsingError(e))
                .unwrap(),
        }
    }

    into.finish();
}

/// Retrieves [`Feature`]s and executes them.
///
/// [`Feature`]: gherkin::Feature
async fn execute<W: World>(
    features: Features,
    max_concurrent_scenarios: Option<usize>,
    collection: step::Collection<W>,
    sender: mpsc::UnboundedSender<event::Cucumber<W>>,
) {
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

    let mut executor = Executor::new(collection, sender);

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
struct Executor<W> {
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

    /// Sender for notifying state of [`Feature`]s completion.
    ///
    /// [`Feature`]: gherkin::Feature
    sender: mpsc::UnboundedSender<event::Cucumber<W>>,
}

impl<W: World> Executor<W> {
    /// Creates new [`Executor`].
    fn new(
        collection: step::Collection<W>,
        sender: mpsc::UnboundedSender<event::Cucumber<W>>,
    ) -> Self {
        Self {
            features_scenarios_count: HashMap::new(),
            rule_scenarios_count: HashMap::new(),
            collection,
            sender,
        }
    }

    /// Runs a [`Scenario`].
    ///
    /// # Events
    ///
    /// - Emits all [`Scenario`] events.
    /// - If [`Scenario`] was last for particular [`Rule`] or [`Feature`] also
    ///   emits finishing events for them.
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Rule`]: gherkin::Rule
    /// [`Scenario`]: gherkin::Scenario
    async fn run_scenario(
        &self,
        feature: Arc<gherkin::Feature>,
        rule: Option<Arc<gherkin::Rule>>,
        scenario: Arc<gherkin::Scenario>,
    ) {
        self.send(event::Cucumber::scenario(
            feature.clone(),
            rule.clone(),
            scenario.clone(),
            event::Scenario::Started,
        ));

        let ok = |e: fn(Arc<gherkin::Step>) -> event::Scenario<W>| {
            let (f, r, s) = (&feature, &rule, &scenario);
            move |step| {
                let (f, r, s) = (f.clone(), r.clone(), s.clone());
                event::Cucumber::scenario(f, r, s, e(step))
            }
        };
        let err = |e: fn(Arc<gherkin::Step>, W, Info) -> event::Scenario<W>| {
            let (f, r, s) = (&feature, &rule, &scenario);
            move |step, world, info| {
                let (f, r, s) = (f.clone(), r.clone(), s.clone());
                event::Cucumber::scenario(f, r, s, e(step, world, info))
            }
        };

        let res = async {
            let feature_background = feature
                .background
                .as_ref()
                .map(|b| b.steps.iter().map(|s| Arc::new(s.clone())))
                .into_iter()
                .flatten();

            let feature_background = stream::iter(feature_background)
                .map(Ok)
                .try_fold(None, |world, bg_step| {
                    self.run_step(
                        world,
                        bg_step,
                        ok(event::Scenario::background_step_started),
                        ok(event::Scenario::background_step_passed),
                        ok(event::Scenario::background_step_skipped),
                        err(event::Scenario::background_step_failed),
                    )
                    .map_ok(Some)
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
                    self.run_step(
                        world,
                        bg_step,
                        ok(event::Scenario::background_step_started),
                        ok(event::Scenario::background_step_passed),
                        ok(event::Scenario::background_step_skipped),
                        err(event::Scenario::background_step_failed),
                    )
                    .map_ok(Some)
                })
                .await?;

            stream::iter(scenario.steps.iter().map(|s| Arc::new(s.clone())))
                .map(Ok)
                .try_fold(rule_background, |world, step| {
                    self.run_step(
                        world,
                        step,
                        ok(event::Scenario::step_started),
                        ok(event::Scenario::step_passed),
                        ok(event::Scenario::step_skipped),
                        err(event::Scenario::step_failed),
                    )
                    .map_ok(Some)
                })
                .await
        };

        drop(res.await);

        self.send(event::Cucumber::scenario(
            feature.clone(),
            rule.clone(),
            scenario.clone(),
            event::Scenario::Finished,
        ));

        if let Some(rule) = rule {
            if let Some(finished) =
                self.rule_scenario_finished(feature.clone(), rule)
            {
                self.send(finished);
            }
        }

        if let Some(finished) = self.feature_scenario_finished(feature) {
            self.send(finished);
        }
    }

    /// Runs a [`Step`].
    ///
    /// # Events
    ///
    /// - Emits all [`Step`] events.
    ///
    /// [`Step`]: gherkin::Step
    async fn run_step(
        &self,
        mut world: Option<W>,
        step: Arc<gherkin::Step>,
        started: impl FnOnce(Arc<gherkin::Step>) -> event::Cucumber<W>,
        passed: impl FnOnce(Arc<gherkin::Step>) -> event::Cucumber<W>,
        skipped: impl FnOnce(Arc<gherkin::Step>) -> event::Cucumber<W>,
        failed: impl FnOnce(Arc<gherkin::Step>, W, Info) -> event::Cucumber<W>,
    ) -> Result<W, ()> {
        self.send(started(step.clone()));

        let run = async {
            if world.is_none() {
                world =
                    Some(W::new().await.expect("failed to initialize World"));
            }

            let (step_fn, ctx) = self.collection.find(&step)?;
            step_fn(world.as_mut().unwrap(), ctx).await;
            Some(())
        };

        let res = match AssertUnwindSafe(run).catch_unwind().await {
            Ok(Some(())) => {
                self.send(passed(step));
                Ok(world.unwrap())
            }
            Ok(None) => {
                self.send(skipped(step));
                Err(())
            }
            Err(err) => {
                self.send(failed(step, world.unwrap(), err));
                Err(())
            }
        };

        res
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
            .get(&(feature.path.clone(), rule.clone()))
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
        for feature in runnable.iter().map(|(f, ..)| f.clone()).dedup() {
            let _ = self
                .features_scenarios_count
                .entry(feature.clone())
                .or_insert_with(|| {
                    started_features.push(feature);
                    0.into()
                });
        }

        let mut started_rules = Vec::new();
        for (feature, rule) in runnable
            .iter()
            .filter_map(|(f, r, _)| r.clone().map(|r| (f.clone(), r)))
            .dedup()
        {
            let _ = self
                .rule_scenarios_count
                .entry((feature.path.clone(), rule.clone()))
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

    /// Notifies with given [`Cucumber`] event.
    ///
    /// [`Cucumber`]: event::Cucumber
    fn send(&self, event: event::Cucumber<W>) {
        self.sender.unbounded_send(event).unwrap();
    }

    /// Notifies with given [`Cucumber`] events.
    ///
    /// [`Cucumber`]: event::Cucumber
    fn send_all(&self, events: impl Iterator<Item = event::Cucumber<W>>) {
        for event in events {
            self.send(event);
        }
    }
}

/// Storage sorted by [`ScenarioType`] [`Feature`]'s [`Scenario`]s.
///
/// [`Feature`]: gherkin::Feature
/// [`Scenario`]: gherkin::Scenario
#[derive(Clone, Default)]
struct Features {
    /// Storage itself.
    scenarios: Arc<Mutex<Scenarios>>,

    /// Indicates whether all parsed [`Feature`]s are sorted and stored.
    finished: Arc<AtomicBool>,
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

impl Features {
    /// Splits [`Feature`] into [`Scenario`]s, sorts by [`ScenarioType`] and
    /// stores them.
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Scenario`]: gherkin::Scenario
    async fn insert<F>(&self, feature: gherkin::Feature, which_scenario: &F)
    where
        F: Fn(
                &gherkin::Feature,
                Option<&gherkin::Rule>,
                &gherkin::Scenario,
            ) -> ScenarioType
            + 'static,
    {
        let f = feature.expand_examples();

        let local = f
            .scenarios
            .iter()
            .map(|s| (&f, None, s))
            .chain(f.rules.iter().flat_map(|r| {
                r.scenarios
                    .iter()
                    .map(|s| (&f, Some(r), s))
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

    /// Returns [`Scenario`]s which are ready to be run.
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

    /// Marks that there will be no additional [`Feature`]s.
    ///
    /// [`Feature`]: gherkin::Feature
    fn finish(&self) {
        self.finished.store(true, Ordering::SeqCst);
    }

    /// Indicates whether there will additional [`Feature`]s.
    ///
    /// [`Feature`]: gherkin::Feature
    fn is_finished(&self) -> bool {
        self.finished.load(Ordering::SeqCst)
    }
}
