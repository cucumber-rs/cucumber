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
    thread,
    time::{Duration, Instant},
};

use drain_filter_polyfill::VecExt;
use futures::{
    channel::{mpsc, oneshot},
    future::{self, Either, LocalBoxFuture},
    lock::Mutex,
    pin_mut,
    stream::{self, LocalBoxStream},
    FutureExt as _, Stream, StreamExt as _, TryFutureExt as _,
    TryStreamExt as _,
};
use gherkin::tagexpr::TagOperation;
use humantime::parse_duration;
use itertools::Itertools as _;
use regex::{CaptureLocations, Regex};

use crate::{
    event::{self, HookType, Info, Retries},
    feature::Ext as _,
    future::yield_now,
    parser, step,
    tag::Ext as _,
    Event, Runner, Step, World,
};

/// CLI options of a [`Basic`] [`Runner`].
#[derive(clap::Args, Clone, Debug)]
pub struct Cli {
    /// Number of scenarios to run concurrently. If not specified, uses the
    /// value configured in tests runner, or 64 by default.
    #[clap(long, short, value_name = "int", global = true)]
    pub concurrency: Option<usize>,

    /// Run tests until the first failure.
    #[clap(long, global = true)]
    pub fail_fast: bool,

    /// Number of times scenario will be rerun in case of a failure.
    #[clap(long, value_name = "int", global = true)]
    pub retry: Option<usize>,

    /// Delay between each retry attempt.
    ///
    /// Duration is represented like `12min 5s`. Supported suffixes:
    /// - `nsec`, `ns` — nanoseconds.
    /// - `usec`, `us` — microseconds.
    /// - `msec`, `ms` — milliseconds.
    /// - `seconds`, `second`, `sec`, `s`.
    /// - `minutes`, `minute`, `min`, `m`.
    #[clap(
        long,
        value_name = "duration",
        parse(try_from_str = parse_duration),
        verbatim_doc_comment,
        global = true,
    )]
    pub retry_after: Option<Duration>,

    /// Tag expression to filter retried scenarios.
    #[clap(long, value_name = "tagexpr", global = true)]
    pub retry_tag_filter: Option<TagOperation>,
}

impl Cli {
    /// Applies retry settings from the [`Cli`].
    fn apply_retry(
        &self,
        scenario: &gherkin::Scenario,
        retries: Option<usize>,
        after: Option<Duration>,
    ) -> Option<RetryOptions> {
        let matched = self.retry_tag_filter.as_ref().map_or_else(
            || self.retry.is_some() || self.retry_after.is_some(),
            |op| op.eval(&scenario.tags),
        );

        (retries.is_some() || after.is_some() || matched).then(|| {
            RetryOptions {
                retries: Retries::initial(retries.or(self.retry).unwrap_or(1)),
                after: after.or(self.retry_after),
            }
        })
    }
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

/// Options for retrying [`Scenario`]s.
///
/// [`Scenario`]: gherkin::Scenario
#[derive(Clone, Copy, Debug)]
pub struct RetryOptions {
    /// Number of [`Retries`].
    pub retries: Retries,

    /// Delay before next retry attempt will be executed.
    pub after: Option<Duration>,
}

impl RetryOptions {
    /// Returns [`Some`], in case next retry attempt is available, or [`None`]
    /// otherwise.
    #[must_use]
    pub fn next_try(self) -> Option<Self> {
        self.retries.next_try().map(|num| Self {
            retries: num,
            after: self.after,
        })
    }

    /// Constructs [`RetryOptionsWithDeadline`], that will reschedule
    /// [`Scenario`] [`after`] delay.
    ///
    /// [`after`]: RetryOptions::after
    /// [`Scenario`]: gherkin::Scenario
    fn with_deadline(self, now: Instant) -> RetryOptionsWithDeadline {
        RetryOptionsWithDeadline {
            retries: self.retries,
            after: self.after.map(|at| (at, Some(now))),
        }
    }

    /// Constructs [`RetryOptionsWithDeadline`], that will reschedule
    /// [`Scenario`] immediately, ignoring [`RetryOptions::after`]. Used for
    /// initial [`Scenario`] run, where we don't need to wait for delay.
    ///
    /// [`Scenario`]: gherkin::Scenario
    fn without_deadline(self) -> RetryOptionsWithDeadline {
        RetryOptionsWithDeadline {
            retries: self.retries,
            after: self.after.map(|at| (at, None)),
        }
    }
}

/// [`RetryOptions`] with an [`Option`]al [`Instant`] to determine, whether
/// [`Scenario`] should be already rescheduled or not.
///
/// [`Scenario`]: gherkin::Scenario
#[derive(Clone, Copy, Debug)]
pub struct RetryOptionsWithDeadline {
    /// Number of [`Retries`].
    pub retries: Retries,

    /// Delay before next retry attempt will be executed.
    pub after: Option<(Duration, Option<Instant>)>,
}

impl From<RetryOptionsWithDeadline> for RetryOptions {
    fn from(v: RetryOptionsWithDeadline) -> Self {
        Self {
            retries: v.retries,
            after: v.after.map(|(at, _)| at),
        }
    }
}

impl RetryOptionsWithDeadline {
    /// Returns [`Duration`] after [`Scenario`] could be retried. If [`None`],
    /// [`Scenario`] is ready for the retry.
    ///
    /// [`Scenario`]: gherkin::Scenario
    fn left_until_retry(&self) -> Option<Duration> {
        let (dur, instant) = self.after?;
        dur.checked_sub(instant?.elapsed())
    }
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

/// Alias for [`fn`] used to determine [`Scenario`]'s [`RetryOptions`].
///
/// [`Scenario`]: gherkin::Scenario
pub type RetryOptionsFn = fn(
    &gherkin::Feature,
    Option<&gherkin::Rule>,
    &gherkin::Scenario,
    &Cli,
) -> Option<RetryOptions>;

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
type IsFailed = bool;

/// Alias for a retried [`Scenario`].
///
/// [`Scenario`]: gherkin::Scenario
type IsRetried = bool;

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
    R = RetryOptionsFn,
    Before = BeforeHookFn<World>,
    After = AfterHookFn<World>,
> {
    /// Optional number of concurrently executed [`Scenario`]s.
    ///
    /// [`Scenario`]: gherkin::Scenario
    max_concurrent_scenarios: Option<usize>,

    /// Optional number of retries of failed [`Scenario`]s.
    ///
    /// [`Scenario`]: gherkin::Scenario
    retries: Option<usize>,

    /// Optional [`Duration`] between retries of failed [`Scenario`]s.
    ///
    /// [`Scenario`]: gherkin::Scenario
    retry_after: Option<Duration>,

    /// Optional [`TagOperation`] filter for retries of failed [`Scenario`]s.
    ///
    /// [`Scenario`]: gherkin::Scenario
    retry_filter: Option<TagOperation>,

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

    /// Function determining [`Scenario`]'s [`RetryOptions`].
    ///
    /// [`Scenario`]: gherkin::Scenario
    retry_options: R,

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
impl<World, F, R, B, A> fmt::Debug for Basic<World, F, R, B, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Basic")
            .field("max_concurrent_scenarios", &self.max_concurrent_scenarios)
            .field("steps", &self.steps)
            .finish_non_exhaustive()
    }
}

impl<World> Basic<World, (), ()> {
    /// Creates a new empty [`Runner`].
    #[must_use]
    pub fn custom() -> Self {
        Self {
            max_concurrent_scenarios: None,
            retries: None,
            retry_after: None,
            retry_filter: None,
            steps: step::Collection::new(),
            which_scenario: (),
            retry_options: (),
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

        #[allow(clippy::shadow_unrelated)]
        let retry: RetryOptionsFn = |f, r, sc, cli| {
            let parse_tags = |tags: &[String]| {
                tags.iter().find_map(|tag| {
                    tag.strip_prefix("retry").map(|retries| {
                        let (num, rest) = retries
                            .strip_prefix('(')
                            .and_then(|s| {
                                s.split_once(')').and_then(|(num, rest)| {
                                    num.parse::<usize>()
                                        .ok()
                                        .map(|num| (Some(num), rest))
                                })
                            })
                            .unwrap_or((None, retries));

                        let at =
                            rest.strip_prefix(".after").and_then(|after| {
                                after.strip_prefix('(').and_then(|after| {
                                    let (dur, _) = after.split_once(')')?;
                                    parse_duration(dur).ok()
                                })
                            });

                        (num, at)
                    })
                })
            };

            let (num, after) = parse_tags(&sc.tags)
                .or_else(|| r.and_then(|r| parse_tags(&r.tags)))
                .or_else(|| parse_tags(&f.tags))
                .unwrap_or((None, None));

            cli.apply_retry(sc, num, after)
        };

        Self {
            max_concurrent_scenarios: Some(64),
            retries: None,
            retry_after: None,
            retry_filter: None,
            steps: step::Collection::new(),
            which_scenario,
            retry_options: retry,
            before_hook: None,
            after_hook: None,
            fail_fast: false,
        }
    }
}

impl<World, Which, Retry, Before, After>
    Basic<World, Which, Retry, Before, After>
{
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

    /// If `retries` is [`Some`], then failed [`Scenario`]s will be retried
    /// specified number of times.
    ///
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    pub fn retries(mut self, retries: impl Into<Option<usize>>) -> Self {
        self.retries = retries.into();
        self
    }

    /// If `after` is [`Some`], then failed [`Scenario`]s will be retried after
    /// specified [`Duration`].
    ///
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    pub fn retry_after(mut self, after: impl Into<Option<Duration>>) -> Self {
        self.retry_after = after.into();
        self
    }

    /// If `filter` is [`Some`], then failed [`Scenario`]s will be retried
    /// only if they are matched by [`TagOperation`].
    ///
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    pub fn retry_filter(
        mut self,
        filter: impl Into<Option<TagOperation>>,
    ) -> Self {
        self.retry_filter = filter.into();
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
    pub fn which_scenario<F>(
        self,
        func: F,
    ) -> Basic<World, F, Retry, Before, After>
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
            retries,
            retry_after,
            retry_filter,
            steps,
            retry_options: retry,
            before_hook,
            after_hook,
            fail_fast,
            ..
        } = self;
        Basic {
            max_concurrent_scenarios,
            retries,
            retry_after,
            retry_filter,
            steps,
            which_scenario: func,
            retry_options: retry,
            before_hook,
            after_hook,
            fail_fast,
        }
    }

    /// Function determining [`Scenario`]'s [`RetryOptions`].
    ///
    /// [`Scenario`]: gherkin::Scenario
    #[allow(clippy::missing_const_for_fn)] // false positive: drop in const
    #[must_use]
    pub fn retry_options<R>(
        self,
        func: R,
    ) -> Basic<World, Which, R, Before, After>
    where
        R: Fn(
                &gherkin::Feature,
                Option<&gherkin::Rule>,
                &gherkin::Scenario,
                &Cli,
            ) -> Option<RetryOptions>
            + 'static,
    {
        let Self {
            max_concurrent_scenarios,
            retries,
            retry_after,
            retry_filter,
            steps,
            which_scenario,
            before_hook,
            after_hook,
            fail_fast,
            ..
        } = self;
        Basic {
            max_concurrent_scenarios,
            retries,
            retry_after,
            retry_filter,
            steps,
            which_scenario,
            retry_options: func,
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
    pub fn before<Func>(
        self,
        func: Func,
    ) -> Basic<World, Which, Retry, Func, After>
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
            retries,
            retry_after,
            retry_filter,
            steps,
            which_scenario,
            retry_options: retry,
            after_hook,
            fail_fast,
            ..
        } = self;
        Basic {
            max_concurrent_scenarios,
            retries,
            retry_after,
            retry_filter,
            steps,
            which_scenario,
            retry_options: retry,
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
    pub fn after<Func>(
        self,
        func: Func,
    ) -> Basic<World, Which, Retry, Before, Func>
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
            retries,
            retry_after,
            retry_filter,
            steps,
            which_scenario,
            retry_options: retry,
            before_hook,
            fail_fast,
            ..
        } = self;
        Basic {
            max_concurrent_scenarios,
            retries,
            retry_after,
            retry_filter,
            steps,
            which_scenario,
            retry_options: retry,
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

impl<W, Which, Retry, Before, After> Runner<W>
    for Basic<W, Which, Retry, Before, After>
where
    W: World,
    Which: Fn(
            &gherkin::Feature,
            Option<&gherkin::Rule>,
            &gherkin::Scenario,
        ) -> ScenarioType
        + 'static,
    Retry: Fn(
            &gherkin::Feature,
            Option<&gherkin::Rule>,
            &gherkin::Scenario,
            &Cli,
        ) -> Option<RetryOptions>
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

    fn run<S>(self, features: S, mut cli: Cli) -> Self::EventStream
    where
        S: Stream<Item = parser::Result<gherkin::Feature>> + 'static,
    {
        let Self {
            max_concurrent_scenarios,
            retries,
            retry_after,
            retry_filter,
            steps,
            which_scenario,
            retry_options,
            before_hook,
            after_hook,
            fail_fast,
        } = self;

        cli.retry = cli.retry.or(retries);
        cli.retry_after = cli.retry_after.or(retry_after);
        cli.retry_tag_filter = cli.retry_tag_filter.or(retry_filter);
        let fail_fast = if cli.fail_fast { true } else { fail_fast };
        let concurrency = cli.concurrency.or(max_concurrent_scenarios);

        let buffer = Features::default();
        let (sender, receiver) = mpsc::unbounded();

        let insert = insert_features(
            buffer.clone(),
            features,
            which_scenario,
            retry_options,
            sender.clone(),
            cli,
            fail_fast,
        );
        let execute = execute(
            buffer,
            concurrency,
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
async fn insert_features<W, S, F, R>(
    into: Features,
    features_stream: S,
    which_scenario: F,
    retries: R,
    sender: mpsc::UnboundedSender<parser::Result<Event<event::Cucumber<W>>>>,
    cli: Cli,
    fail_fast: bool,
) where
    S: Stream<Item = parser::Result<gherkin::Feature>> + 'static,
    F: Fn(
            &gherkin::Feature,
            Option<&gherkin::Rule>,
            &gherkin::Scenario,
        ) -> ScenarioType
        + 'static,
    R: Fn(
            &gherkin::Feature,
            Option<&gherkin::Rule>,
            &gherkin::Scenario,
            &Cli,
        ) -> Option<RetryOptions>
        + 'static,
{
    let mut features = 0;
    let mut rules = 0;
    let mut scenarios = 0;
    let mut steps = 0;
    let mut parser_errors = 0;

    pin_mut!(features_stream);
    while let Some(feat) = features_stream.next().await {
        match feat {
            Ok(f) => {
                features += 1;
                rules += f.rules.len();
                scenarios += f.count_scenarios();
                steps += f.count_steps();

                into.insert(f, &which_scenario, &retries, &cli).await;
            }
            // If the receiver end is dropped, then no one listens for events
            // so we can just stop from here.
            Err(e) => {
                parser_errors += 1;

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
            parser_errors,
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
        features.clone(),
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
        // We yield once on every iteration, because there is a chance, that
        // this function never yields otherwise. In this case event sender won't
        // send anything to the `Writer` until the end. This is the case, when
        // all parsing is done, so there is no contention on `Mutex` inside
        // `Features` storage and all `Step` functions don't yield.
        yield_now().await;

        let (runnable, sleep) =
            features.get(map_break(started_scenarios)).await;
        if run_scenarios.is_empty() && runnable.is_empty() {
            if features.is_finished().await {
                break;
            }

            // To avoid busy-polling of `Features::get()`, in case there are no
            // scenarios that are running or scheduled for execution, we spawn a
            // thread, that sleeps for minimal deadline of all retried
            // scenarios.
            // TODO: replace `thread::spawn` with async runtime agnostic sleep,
            //       once it's available.
            if let Some(dur) = sleep {
                let (sender, receiver) = oneshot::channel();
                drop(thread::spawn(move || {
                    thread::sleep(dur);
                    sender.send(())
                }));
                let _ = receiver.await.ok();
            }

            continue;
        }

        let started = storage.start_scenarios(&runnable);
        executor.send_all_events(started);

        if let ControlFlow::Continue(Some(sc)) = &mut started_scenarios {
            *sc -= runnable.len();
        }

        for (f, r, s, ty, retries) in runnable {
            run_scenarios.push(executor.run_scenario(f, r, s, ty, retries));
        }

        if run_scenarios.next().await.is_some() {
            if let ControlFlow::Continue(Some(sc)) = &mut started_scenarios {
                *sc += 1;
            }
        }

        while let Ok(Some((feat, rule, scenario_failed, retried))) =
            storage.finished_receiver.try_next()
        {
            if let Some(rule) = rule {
                if let Some(f) = storage.rule_scenario_finished(
                    Arc::clone(&feat),
                    rule,
                    retried,
                ) {
                    executor.send_event(f);
                }
            }
            if let Some(f) = storage.feature_scenario_finished(feat, retried) {
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
    #[allow(clippy::too_many_lines)]
    async fn run_scenario(
        &self,
        feature: Arc<gherkin::Feature>,
        rule: Option<Arc<gherkin::Rule>>,
        scenario: Arc<gherkin::Scenario>,
        scenario_ty: ScenarioType,
        retries: Option<RetryOptions>,
    ) {
        let retry_num = retries.map(|r| r.retries);
        let ok = |e: fn(_, _) -> event::Scenario<W>| {
            let (f, r, s) = (&feature, &rule, &scenario);
            move |step| {
                let (f, r, s) = (Arc::clone(f), r.clone(), Arc::clone(s));
                event::Cucumber::scenario(f, r, s, e(step, retry_num))
            }
        };
        let ok_capt = |e: fn(_, _, _, _) -> event::Scenario<W>| {
            let (f, r, s) = (&feature, &rule, &scenario);
            move |step, cap, loc| {
                let (f, r, s) = (Arc::clone(f), r.clone(), Arc::clone(s));
                event::Cucumber::scenario(f, r, s, e(step, cap, loc, retry_num))
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
            event::Scenario::Started(retry_num),
        ));

        let mut result = async {
            let before_hook = self
                .run_before_hook(&feature, rule.as_ref(), &scenario, retry_num)
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
                Arc::clone(&feature),
                rule.clone(),
                Arc::clone(&scenario),
                world.clone(),
                exec_error,
                retry_num,
            );
        }

        self.emit_after_hook_events(
            Arc::clone(&feature),
            rule.clone(),
            Arc::clone(&scenario),
            world,
            after_hook_meta,
            after_hook_error,
            retry_num,
        );

        self.send_event(event::Cucumber::scenario(
            Arc::clone(&feature),
            rule.clone(),
            Arc::clone(&scenario),
            event::Scenario::Finished(retry_num),
        ));

        let next_try = retries
            .filter(|_| is_failed)
            .and_then(RetryOptions::next_try);
        if let Some(next_try) = next_try {
            self.storage
                .insert_retried_scenario(
                    Arc::clone(&feature),
                    rule.clone(),
                    scenario,
                    scenario_ty,
                    Some(next_try),
                )
                .await;
        }

        self.scenario_finished(feature, rule, is_failed, next_try.is_some());
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
        retries: Option<Retries>,
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
                event::Scenario::hook_started(HookType::Before, retries),
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
                        event::Scenario::hook_passed(HookType::Before, retries),
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
        Ps: FnOnce(
            Arc<gherkin::Step>,
            CaptureLocations,
            Option<step::Location>,
        ) -> event::Cucumber<W>,
        Sk: FnOnce(Arc<gherkin::Step>) -> event::Cucumber<W>,
    {
        self.send_event(started(Arc::clone(&step)));

        let run = async {
            let (step_fn, captures, loc, ctx) =
                match self.collection.find(&step) {
                    Ok(Some(f)) => f,
                    Ok(None) => return Ok((None, None, world)),
                    Err(e) => {
                        let e = event::StepError::AmbiguousMatch(e);
                        return Err((e, None, None, world));
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
                        return Err((e, None, loc, None));
                    }
                    Err(e) => {
                        let e = event::StepError::Panic(e.into());
                        return Err((e, None, loc, None));
                    }
                }
            };

            match AssertUnwindSafe(step_fn(&mut world, ctx))
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

        #[allow(clippy::shadow_unrelated)]
        match run.await {
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
        feature: Arc<gherkin::Feature>,
        rule: Option<Arc<gherkin::Rule>>,
        scenario: Arc<gherkin::Scenario>,
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
                            retries,
                        ),
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
                        step, captures, loc, world, error, retries,
                    ),
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
                        step, captures, loc, world, error, retries,
                    ),
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
    #[allow(clippy::too_many_arguments)]
    fn emit_after_hook_events(
        &self,
        feature: Arc<gherkin::Feature>,
        rule: Option<Arc<gherkin::Rule>>,
        scenario: Arc<gherkin::Scenario>,
        world: Option<Arc<W>>,
        meta: Option<AfterHookEventsMeta>,
        err: Option<Info>,
        retries: Option<Retries>,
    ) {
        debug_assert_eq!(self.after_hook.is_some(), meta.is_some());

        if let Some(meta) = meta {
            self.send_event_with_meta(
                event::Cucumber::scenario(
                    Arc::clone(&feature),
                    rule.clone(),
                    Arc::clone(&scenario),
                    event::Scenario::hook_started(HookType::After, retries),
                ),
                meta.started,
            );

            let ev = if let Some(err) = err {
                event::Cucumber::scenario(
                    feature,
                    rule,
                    scenario,
                    event::Scenario::hook_failed(
                        HookType::After,
                        world,
                        err,
                        retries,
                    ),
                )
            } else {
                event::Cucumber::scenario(
                    feature,
                    rule,
                    scenario,
                    event::Scenario::hook_passed(HookType::After, retries),
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
        is_failed: IsFailed,
        is_retried: IsRetried,
    ) {
        // If the receiver end is dropped, then no one listens for events
        // so we can just ignore it.
        drop(
            self.finished_sender
                .unbounded_send((feature, rule, is_failed, is_retried)),
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
    finished_receiver: FinishedFeaturesReceiver,
}

/// Alias for [`mpsc::UnboundedSender`] that notifies about finished
/// [`Feature`]s.
///
/// [`Feature`]: gherkin::Feature
type FinishedFeaturesSender = mpsc::UnboundedSender<(
    Arc<gherkin::Feature>,
    Option<Arc<gherkin::Rule>>,
    IsFailed,
    IsRetried,
)>;

/// Alias for [`mpsc::UnboundedReceiver`] that receives events about finished
/// [`Feature`]s.
///
/// [`Feature`]: gherkin::Feature
type FinishedFeaturesReceiver = mpsc::UnboundedReceiver<(
    Arc<gherkin::Feature>,
    Option<Arc<gherkin::Rule>>,
    IsFailed,
    IsRetried,
)>;

impl FinishedRulesAndFeatures {
    /// Creates a new [`FinishedRulesAndFeatures`] store.
    fn new(finished_receiver: FinishedFeaturesReceiver) -> Self {
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
        is_retried: bool,
    ) -> Option<event::Cucumber<W>> {
        if is_retried {
            return None;
        }

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
        is_retried: bool,
    ) -> Option<event::Cucumber<W>> {
        if is_retried {
            return None;
        }

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
                ScenarioType,
                Option<RetryOptions>,
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
            .filter_map(|(feat, rule, _, _, _)| {
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
        Option<RetryOptionsWithDeadline>,
    )>,
>;

/// Alias for [`Features::insert_scenarios()`] argument.
type InsertedScenarios = HashMap<
    ScenarioType,
    Vec<(
        Arc<gherkin::Feature>,
        Option<Arc<gherkin::Rule>>,
        Arc<gherkin::Scenario>,
        Option<RetryOptions>,
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
    async fn insert<Which, Retry>(
        &self,
        feature: gherkin::Feature,
        which_scenario: &Which,
        retry: &Retry,
        cli: &Cli,
    ) where
        Which: Fn(
                &gherkin::Feature,
                Option<&gherkin::Rule>,
                &gherkin::Scenario,
            ) -> ScenarioType
            + 'static,
        Retry: Fn(
                &gherkin::Feature,
                Option<&gherkin::Rule>,
                &gherkin::Scenario,
                &Cli,
            ) -> Option<RetryOptions>
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
                let retries = retry(feat, rule, scenario, cli);
                (
                    Arc::new(feat.clone()),
                    rule.map(|r| Arc::new(r.clone())),
                    Arc::new(scenario.clone()),
                    retries,
                )
            })
            .into_group_map_by(|(f, r, s, _)| {
                which_scenario(f, r.as_ref().map(AsRef::as_ref), s)
            });

        self.insert_scenarios(local).await;
    }

    /// Inserts retried [`Scenario`].
    ///
    /// [`Scenario`]: gherkin::Scenario
    async fn insert_retried_scenario(
        &self,
        feature: Arc<gherkin::Feature>,
        rule: Option<Arc<gherkin::Rule>>,
        scenario: Arc<gherkin::Scenario>,
        scenario_ty: ScenarioType,
        retries: Option<RetryOptions>,
    ) {
        self.insert_scenarios(
            [(scenario_ty, vec![(feature, rule, scenario, retries)])]
                .into_iter()
                .collect(),
        )
        .await;
    }

    /// Inserts [`Scenario`]s into this [`Features`] storage.
    ///
    /// [`Scenario`]: gherkin::Scenario
    async fn insert_scenarios(&self, scenarios: InsertedScenarios) {
        let now = Instant::now();

        let mut with_retries = HashMap::<_, Vec<_>>::new();
        let mut without_retries: Scenarios = HashMap::new();
        for (which, values) in scenarios {
            for (f, r, s, ret) in values {
                match ret {
                    ret @ (None
                    | Some(RetryOptions {
                        retries: Retries { current: 0, .. },
                        ..
                    })) => {
                        // `Retries::current` is `0`, so this `Scenario` run is
                        // initial and we don't need to wait for retry delay.
                        let ret = ret.map(RetryOptions::without_deadline);
                        without_retries
                            .entry(which)
                            .or_default()
                            .push((f, r, s, ret));
                    }
                    Some(ret) => {
                        let ret = ret.with_deadline(now);
                        with_retries
                            .entry(which)
                            .or_default()
                            .push((f, r, s, ret));
                    }
                }
            }
        }

        let mut storage = self.scenarios.lock().await;

        for (which, values) in with_retries {
            let ty_storage = storage.entry(which).or_default();
            for (f, r, s, ret) in values {
                ty_storage.insert(0, (f, r, s, Some(ret)));
            }
        }

        if without_retries.get(&ScenarioType::Serial).is_none() {
            // If there are no Serial Scenarios we just extending already
            // existing Concurrent Scenarios.
            for (which, values) in without_retries {
                storage.entry(which).or_default().extend(values);
            }
        } else {
            // If there are Serial Scenarios we insert all Serial and Concurrent
            // Scenarios in front.
            // This is done to execute them closely to one another, so the
            // output wouldn't hang on executing other Concurrent Scenarios.
            for (which, mut values) in without_retries {
                let old = mem::take(storage.entry(which).or_default());
                values.extend(old);
                storage.entry(which).or_default().extend(values);
            }
        }
    }

    /// Returns [`Scenario`]s which are ready to run and minimal deadline of all
    /// retried [`Scenario`]s.
    ///
    /// [`Scenario`]: gherkin::Scenario
    async fn get(
        &self,
        max_concurrent_scenarios: Option<usize>,
    ) -> (
        Vec<(
            Arc<gherkin::Feature>,
            Option<Arc<gherkin::Rule>>,
            Arc<gherkin::Scenario>,
            ScenarioType,
            Option<RetryOptions>,
        )>,
        Option<Duration>,
    ) {
        use ScenarioType::{Concurrent, Serial};

        let mut min_dur = None;
        let mut drain =
            |storage: &mut Vec<(_, _, _, Option<RetryOptionsWithDeadline>)>,
             ty,
             count: Option<usize>| {
                let mut i = 0;
                // TODO: Replace with `drain_filter`, once stabilized.
                //       https://github.com/rust-lang/rust/issues/43244
                let drained =
                    VecExt::drain_filter(storage, |(_, _, _, ret)| {
                        // Because `drain_filter` runs over entire `Vec` on
                        // `Drop`, we can't just `.take(count)`.
                        if count.filter(|c| i >= *c).is_some() {
                            return false;
                        }

                        ret.as_ref()
                            .and_then(
                                RetryOptionsWithDeadline::left_until_retry,
                            )
                            .map_or_else(
                                || {
                                    i += 1;
                                    true
                                },
                                |left| {
                                    min_dur = min_dur
                                        .map(|min| cmp::min(min, left))
                                        .or(Some(left));
                                    false
                                },
                            )
                    })
                    .map(|(f, r, s, ret)| (f, r, s, ty, ret.map(Into::into)))
                    .collect::<Vec<_>>();
                (!drained.is_empty()).then(|| drained)
            };

        let mut guard = self.scenarios.lock().await;
        let scenarios = guard
            .get_mut(&Serial)
            .and_then(|storage| drain(storage, Serial, Some(1)))
            .or_else(|| {
                guard.get_mut(&Concurrent).and_then(|storage| {
                    drain(storage, Concurrent, max_concurrent_scenarios)
                })
            })
            .unwrap_or_default();

        (scenarios, min_dur)
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
    async fn is_finished(&self) -> bool {
        self.finished.load(Ordering::SeqCst)
            && self.scenarios.lock().await.values().all(Vec::is_empty)
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

        /// [`Location`] of the [`fn`] that matched this [`Step`].
        ///
        /// [`Location`]: step::Location
        /// [`Step`]: gherkin::Step
        loc: Option<step::Location>,

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
