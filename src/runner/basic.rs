// Copyright (c) 2018-2026  Brendan Molloy <brendan@bbqsrc.net>,
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
    any::Any,
    cmp,
    collections::{HashMap, VecDeque},
    iter, mem,
    ops::ControlFlow,
    panic::{self, AssertUnwindSafe},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

#[cfg(feature = "tracing")]
use crossbeam_utils::atomic::AtomicCell;
use derive_more::with_trait::{Debug, Display, FromStr};
use futures::{
    FutureExt as _, Stream, StreamExt as _,
    channel::{mpsc, oneshot},
    future::{self, Either, LocalBoxFuture},
    lock::Mutex,
    pin_mut,
    stream::{self, LocalBoxStream},
};
use gherkin::tagexpr::TagOperation;
use itertools::Itertools as _;
use regex::{CaptureLocations, Regex};

#[cfg(feature = "tracing")]
use crate::tracing::{Collector as TracingCollector, SpanCloseWaiter};
use crate::{
    Event, Runner, Step, World,
    event::{self, HookType, Info, Retries, Source},
    execution::{
        ExecutionControl, ExecutionMode, Outcome, OutcomePolicy, OutcomeStatus,
        WorldContext,
    },
    feature::Ext as _,
    future::{FutureExt as _, select_with_biased_first},
    parser, step,
    tag::Ext as _,
};

/// CLI options of a [`Basic`] [`Runner`].
#[derive(Clone, Debug, Default, clap::Args)]
#[group(skip)]
pub struct Cli {
    /// Number of scenarios to run concurrently. If not specified, uses the
    /// value configured in tests runner, or 64 by default.
    #[arg(long, short, value_name = "int", global = true, value_parser = positive_concurrency)]
    pub concurrency: Option<usize>,

    /// Run tests until the first failure.
    #[arg(long, global = true, visible_alias = "ff")]
    pub fail_fast: bool,

    /// Number of times a scenario will be retried in case of a failure.
    #[arg(long, value_name = "int", global = true)]
    pub retry: Option<usize>,

    /// Delay between each scenario retry attempt.
    ///
    /// Duration is represented in a human-readable format like `12min5s`.
    /// Supported suffixes:
    /// - `nsec`, `ns` — nanoseconds.
    /// - `usec`, `us` — microseconds.
    /// - `msec`, `ms` — milliseconds.
    /// - `seconds`, `second`, `sec`, `s` - seconds.
    /// - `minutes`, `minute`, `min`, `m` - minutes.
    #[arg(
        long,
        value_name = "duration",
        value_parser = humantime::parse_duration,
        verbatim_doc_comment,
        global = true,
    )]
    pub retry_after: Option<Duration>,

    /// Tag expression to filter retried scenarios.
    #[arg(long, value_name = "tagexpr", global = true)]
    pub retry_tag_filter: Option<TagOperation>,
}

fn positive_concurrency(value: &str) -> Result<usize, String> {
    value.parse::<usize>().ok().filter(|n| *n > 0)
        .ok_or_else(|| "concurrency must be a positive integer".to_owned())
}

/// Whether backgrounds execute for scenarios without their own steps.
///
/// This execution policy is independent of parser syntax/content policies.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum BackgroundSemantics {
    /// Preserve existing behavior: run backgrounds even for empty scenarios.
    #[default]
    Legacy,
    /// Match official Gherkin 43 Pickles: empty scenarios have no background steps.
    /// Scenario hooks still run, including on expanded empty outline rows.
    Gherkin43,
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
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
        self.retries
            .next_try()
            .map(|num| Self { retries: num, after: self.after })
    }

    /// Parses [`RetryOptions`] from [`Feature`]'s, [`Rule`]'s, [`Scenario`]'s
    /// tags and [`Cli`] options.
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Rule`]: gherkin::Rule
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    pub fn parse_from_tags(
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        cli: &Cli,
    ) -> Option<Self> {
        let parse_tags = |tags: &[String]| {
            tags.iter().find_map(|tag| {
                tag.strip_prefix("retry").map(|retries| {
                    let (num, rest) = retries
                        .strip_prefix('(')
                        .and_then(|s| {
                            let (num, rest) = s.split_once(')')?;
                            num.parse::<usize>()
                                .ok()
                                .map(|num| (Some(num), rest))
                        })
                        .unwrap_or((None, retries));

                    let after = rest.strip_prefix(".after").and_then(|after| {
                        let after = after.strip_prefix('(')?;
                        let (dur, _) = after.split_once(')')?;
                        humantime::parse_duration(dur).ok()
                    });

                    (num, after)
                })
            })
        };

        let apply_cli = |options: Option<_>| {
            let matched = cli.retry_tag_filter.as_ref().map_or_else(
                || cli.retry.is_some() || cli.retry_after.is_some(),
                |op| {
                    op.eval(scenario.tags.iter().chain(
                        rule.iter().flat_map(|r| &r.tags).chain(&feature.tags),
                    ))
                },
            );

            (options.is_some() || matched).then(|| Self {
                retries: Retries::initial(
                    options.and_then(|(r, _)| r).or(cli.retry).unwrap_or(1),
                ),
                after: options.and_then(|(_, a)| a).or(cli.retry_after),
            })
        };

        apply_cli(
            parse_tags(&scenario.tags)
                .or_else(|| parse_tags(&rule?.tags))
                .or_else(|| parse_tags(&feature.tags)),
        )
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
    /// initial [`Scenario`] run, where we don't need to wait for the delay.
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
        Self { retries: v.retries, after: v.after.map(|(at, _)| at) }
    }
}

impl RetryOptionsWithDeadline {
    /// Returns [`Duration`] after which a [`Scenario`] could be retried. If
    /// [`None`], then [`Scenario`] is ready for the retry.
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

/// Alias for [`Arc`]ed [`Fn`] used to determine [`Scenario`]'s
/// [`RetryOptions`].
///
/// [`Scenario`]: gherkin::Scenario
pub type RetryOptionsFn = Arc<
    dyn Fn(
        &gherkin::Feature,
        Option<&gherkin::Rule>,
        &gherkin::Scenario,
        &Cli,
    ) -> Option<RetryOptions>,
>;

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
    &'a event::ScenarioFinished,
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
#[derive(Debug)]
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
    #[debug(ignore)]
    which_scenario: F,

    /// Function determining [`Scenario`]'s [`RetryOptions`].
    ///
    /// [`Scenario`]: gherkin::Scenario
    #[debug(ignore)]
    retry_options: RetryOptionsFn,

    /// Function, executed on each [`Scenario`] before running all [`Step`]s,
    /// including [`Background`] ones.
    ///
    /// [`Background`]: gherkin::Background
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
    #[debug(ignore)]
    before_hook: Option<Before>,

    /// Function, executed on each [`Scenario`] after running all [`Step`]s.
    ///
    /// [`Background`]: gherkin::Background
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
    #[debug(ignore)]
    after_hook: Option<After>,

    /// Indicates whether execution should be stopped after the first failure.
    fail_fast: bool,

    /// Execution policy for backgrounds belonging to empty scenarios.
    background_semantics: BackgroundSemantics,

    /// Ordered callbacks around resolved step bodies.
    step_hooks: super::StepHooks<World>,
    /// Retained callbacks with native scenario identity.
    scenario_hooks: Option<super::Hooks<World>>,
    /// Ordered global setup and cleanup for one executor invocation.
    run_hooks: super::RunHooks,
    timeouts: super::Timeouts,
    outcome_policy: OutcomePolicy,
    execution_mode: ExecutionMode,
    scheduling: Option<super::SchedulingOptions>,
    attachments: Option<crate::attachment::AttachmentStore>,
    support_code: Option<crate::support_code::Store>,

    #[cfg(feature = "tracing")]
    /// [`TracingCollector`] for [`event::Scenario::Log`]s forwarding.
    #[debug(ignore)]
    pub(crate) logs_collector: Arc<AtomicCell<Box<Option<TracingCollector>>>>,
}

#[cfg(feature = "tracing")]
/// Assertion that [`Basic::logs_collector`] [`AtomicCell::is_lock_free`].
const _: () = {
    assert!(
        AtomicCell::<Box<Option<TracingCollector>>>::is_lock_free(),
        "`AtomicCell::<Box<Option<TracingCollector>>>` is not lock-free",
    );
};

// Implemented manually to omit redundant `World: Clone` trait bound, imposed by
// `#[derive(Clone)]`.
impl<World, F: Clone, B: Clone, A: Clone> Clone for Basic<World, F, B, A> {
    fn clone(&self) -> Self {
        Self {
            max_concurrent_scenarios: self.max_concurrent_scenarios,
            retries: self.retries,
            retry_after: self.retry_after,
            retry_filter: self.retry_filter.clone(),
            steps: self.steps.clone(),
            which_scenario: self.which_scenario.clone(),
            retry_options: Arc::clone(&self.retry_options),
            before_hook: self.before_hook.clone(),
            after_hook: self.after_hook.clone(),
            fail_fast: self.fail_fast,
            background_semantics: self.background_semantics,
            step_hooks: self.step_hooks.clone(),
            scenario_hooks: self.scenario_hooks.clone(),
            run_hooks: self.run_hooks.clone(),
            timeouts: self.timeouts.clone(),
            outcome_policy: self.outcome_policy,
            execution_mode: self.execution_mode,
            scheduling: self.scheduling.clone(),
            attachments: self.attachments.clone(),
            support_code: self.support_code.clone(),
            #[cfg(feature = "tracing")]
            logs_collector: Arc::clone(&self.logs_collector),
        }
    }
}

impl<World> Default for Basic<World> {
    fn default() -> Self {
        let which_scenario: WhichScenarioFn = |feature, rule, scenario| {
            scenario
                .tags
                .iter()
                .chain(rule.iter().flat_map(|r| &r.tags))
                .chain(&feature.tags)
                .find(|tag| *tag == "serial")
                .map_or(ScenarioType::Concurrent, |_| ScenarioType::Serial)
        };

        Self {
            max_concurrent_scenarios: Some(64),
            retries: None,
            retry_after: None,
            retry_filter: None,
            steps: step::Collection::new(),
            which_scenario,
            retry_options: Arc::new(RetryOptions::parse_from_tags),
            before_hook: None,
            after_hook: None,
            fail_fast: false,
            background_semantics: BackgroundSemantics::Legacy,
            step_hooks: super::StepHooks::default(),
            scenario_hooks: None,
            run_hooks: super::RunHooks::default(),
            timeouts: super::Timeouts::default(),
            outcome_policy: OutcomePolicy::Legacy,
            execution_mode: ExecutionMode::Run,
            scheduling: None,
            attachments: None,
            support_code: None,
            #[cfg(feature = "tracing")]
            logs_collector: Arc::new(AtomicCell::new(Box::new(None))),
        }
    }
}

impl<World, Which, Before, After> Basic<World, Which, Before, After> {
    /// Enables authoritative completion semantics explicitly; Legacy is default.
    #[must_use]
    pub fn with_outcome_policy(mut self, policy: OutcomePolicy) -> Self {
        self.outcome_policy = policy;
        self
    }

    /// Selects ordinary execution or explicit definition inspection.
    #[must_use]
    pub fn with_execution_mode(mut self, mode: ExecutionMode) -> Self {
        self.execution_mode = mode;
        self
    }

    /// Buffers the selected application and schedules its global case plan.
    #[must_use]
    pub fn with_scheduling_options(mut self, options: super::SchedulingOptions) -> Self {
        self.scheduling = Some(options);
        self
    }

    /// If `max` is [`Some`], then number of concurrently executed [`Scenario`]s
    /// will be limited.
    ///
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    pub fn max_concurrent_scenarios(
        mut self,
        max: impl Into<Option<usize>>,
    ) -> Self {
        let max = max.into();
        assert!(max != Some(0), "concurrency must be a positive integer");
        self.max_concurrent_scenarios = max;
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
    /// the specified [`Duration`].
    ///
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    pub fn retry_after(mut self, after: impl Into<Option<Duration>>) -> Self {
        self.retry_after = after.into();
        self
    }

    /// If `filter` is [`Some`], then failed [`Scenario`]s will be retried only
    /// if they're matching the specified `tag_expression`.
    ///
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    pub fn retry_filter(
        mut self,
        tag_expression: impl Into<Option<TagOperation>>,
    ) -> Self {
        self.retry_filter = tag_expression.into();
        self
    }

    /// Makes stop running tests on the first failure.
    ///
    /// __NOTE__: All the already started [`Scenario`]s at the moment of failure
    ///           will be finished.
    ///
    /// __NOTE__: Retried [`Scenario`]s are considered as failed, only in case
    ///           they exhaust all retry attempts and still fail.
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
            retries,
            retry_after,
            retry_filter,
            steps,
            retry_options,
            before_hook,
            after_hook,
            fail_fast,
            background_semantics,
            step_hooks,
            scenario_hooks,
            run_hooks,
            timeouts,
            outcome_policy,
            execution_mode,
            scheduling,
            attachments,
            support_code,
            #[cfg(feature = "tracing")]
            logs_collector,
            ..
        } = self;
        Basic {
            max_concurrent_scenarios,
            retries,
            retry_after,
            retry_filter,
            steps,
            which_scenario: func,
            retry_options,
            before_hook,
            after_hook,
            fail_fast,
            background_semantics,
            step_hooks,
            scenario_hooks,
            run_hooks,
            timeouts,
            outcome_policy,
            execution_mode,
            scheduling,
            attachments,
            support_code,
            #[cfg(feature = "tracing")]
            logs_collector,
        }
    }

    /// Function determining [`Scenario`]'s [`RetryOptions`].
    ///
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    pub fn retry_options<R>(mut self, func: R) -> Self
    where
        R: Fn(
                &gherkin::Feature,
                Option<&gherkin::Rule>,
                &gherkin::Scenario,
                &Cli,
            ) -> Option<RetryOptions>
            + 'static,
    {
        self.retry_options = Arc::new(func);
        self
    }

    /// Sets a hook, executed on each [`Scenario`] before running all its
    /// [`Step`]s, including [`Background`] ones.
    ///
    /// > **NOTE**: Only one [`before`] hook can be registered. If
    /// >           multiple calls are made, only the last one will be
    /// >           run.
    ///
    /// [`before`]: Self::before()
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
            retries,
            retry_after,
            retry_filter,
            steps,
            which_scenario,
            retry_options,
            after_hook,
            fail_fast,
            background_semantics,
            step_hooks,
            scenario_hooks,
            run_hooks,
            timeouts,
            outcome_policy,
            execution_mode,
            scheduling,
            attachments,
            support_code,
            #[cfg(feature = "tracing")]
            logs_collector,
            ..
        } = self;
        Basic {
            max_concurrent_scenarios,
            retries,
            retry_after,
            retry_filter,
            steps,
            which_scenario,
            retry_options,
            before_hook: Some(func),
            after_hook,
            fail_fast,
            background_semantics,
            step_hooks,
            scenario_hooks: scenario_hooks.map(|mut hooks| {
                hooks.clear_before();
                hooks
            }),
            run_hooks,
            timeouts,
            outcome_policy,
            execution_mode,
            scheduling,
            attachments,
            support_code,
            #[cfg(feature = "tracing")]
            logs_collector,
        }
    }

    /// Sets hook, executed on each [`Scenario`] after running all its
    /// [`Step`]s, even after [`Skipped`] of [`Failed`] ones.
    ///
    /// > **NOTE**: Only one [`after`] hook can be registered. If
    /// >           multiple calls are made, only the last one will be
    /// >           run.
    ///
    /// Last `World` argument is supplied to the function, in case it was
    /// initialized before by running [`before`] hook or any [`Step`].
    ///
    /// [`after`]: Self::after()
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
            &'a event::ScenarioFinished,
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
            retry_options,
            before_hook,
            fail_fast,
            background_semantics,
            step_hooks,
            scenario_hooks,
            run_hooks,
            timeouts,
            outcome_policy,
            execution_mode,
            scheduling,
            attachments,
            support_code,
            #[cfg(feature = "tracing")]
            logs_collector,
            ..
        } = self;
        Basic {
            max_concurrent_scenarios,
            retries,
            retry_after,
            retry_filter,
            steps,
            which_scenario,
            retry_options,
            before_hook,
            after_hook: Some(func),
            fail_fast,
            background_semantics,
            step_hooks,
            scenario_hooks: scenario_hooks.map(|mut hooks| {
                hooks.clear_after();
                hooks
            }),
            run_hooks,
            timeouts,
            outcome_policy,
            execution_mode,
            scheduling,
            attachments,
            support_code,
            #[cfg(feature = "tracing")]
            logs_collector,
        }
    }

    /// Installs ordered scenario hooks, replacing both native hook stages.
    ///
    /// Setup executes in registration order; teardown executes in reverse and
    /// continues after panics. Empty stages preserve lazy World creation.
    /// See [`super::Hooks`] for reporting and compatibility boundaries.
    #[must_use]
    pub fn with_hooks(
        self,
        hooks: super::Hooks<World>,
    ) -> Basic<
        World,
        Which,
        super::hooks::Before<World>,
        super::hooks::After<World>,
    >
    where
        World: 'static,
    {
        let Self {
            max_concurrent_scenarios,
            retries,
            retry_after,
            retry_filter,
            steps,
            which_scenario,
            retry_options,
            fail_fast,
            background_semantics,
            step_hooks,
            run_hooks,
            timeouts,
            outcome_policy,
            execution_mode,
            scheduling,
            attachments,
            support_code,
            #[cfg(feature = "tracing")]
            logs_collector,
            ..
        } = self;
        let (before_hook, after_hook) = (None, None);
        let scenario_hooks = Some(hooks);
        Basic {
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
            background_semantics,
            step_hooks,
            scenario_hooks,
            run_hooks,
            timeouts,
            outcome_policy,
            execution_mode,
            scheduling,
            attachments,
            support_code,
            #[cfg(feature = "tracing")]
            logs_collector,
        }
    }

    /// Selects background execution for scenarios with no own steps.
    ///
    /// [`BackgroundSemantics::Gherkin43`] skips feature and Rule backgrounds
    /// for empty scenarios only, preserving before/after hooks. Legacy is default.
    #[must_use]
    pub fn with_background_semantics(
        mut self,
        semantics: BackgroundSemantics,
    ) -> Self {
        self.background_semantics = semantics;
        self
    }

    /// Configures independent deadlines for actual callback and step-body invocations.
    /// World initialization is outside these deadlines. Defaults remain unlimited.
    #[must_use]
    pub fn with_timeouts(mut self, timeouts: super::Timeouts) -> Self {
        self.timeouts = timeouts;
        self
    }

    /// Installs ordered callbacks around resolved step bodies.
    /// This replaces only the step-hook registry and retains scenario hooks.
    #[must_use]
    pub fn with_step_hooks(mut self, hooks: super::StepHooks<World>) -> Self {
        self.step_hooks = hooks;
        self
    }

    /// Installs callbacks around one complete executor invocation.
    ///
    /// Custom writers can observe failures through the registry's shared store.
    /// [`crate::Cucumber::with_run_hooks`] installs failure statistics automatically.
    #[must_use]
    pub fn with_run_hooks(mut self, hooks: super::RunHooks) -> Self {
        self.run_hooks = hooks;
        self
    }

    /// Enables runner-owned attachment scopes with the provided per-run store.
    #[must_use]
    pub fn with_attachments(
        mut self,
        store: crate::attachment::AttachmentStore,
    ) -> Self {
        self.attachments = Some(store);
        self
    }

    /// Sets the given [`Collection`] of [`Step`]s to this [`Runner`].
    ///
    /// [`Collection`]: step::Collection
    #[must_use]
    pub fn steps(mut self, steps: step::Collection<World>) -> Self {
        self.steps = steps;
        self
    }

    /// Selects the step-definition matching policy.
    ///
    /// Configure this after [`Self::steps()`], which replaces the collection
    /// together with its matching policy.
    #[must_use]
    pub fn step_matching(mut self, matching: step::Matching) -> Self {
        self.steps = mem::take(&mut self.steps).matching(matching);
        self
    }

    /// Adds a [Given] [`Step`] matching the given `regex`.
    ///
    /// [Given]: https://cucumber.io/docs/gherkin/reference#given
    #[must_use]
    pub fn given(mut self, regex: Regex, step: Step<World>) -> Self {
        self.steps = mem::take(&mut self.steps).given(None, regex, step);
        self
    }

    /// Adds a [When] [`Step`] matching the given `regex`.
    ///
    /// [When]: https://cucumber.io/docs/gherkin/reference#given
    #[must_use]
    pub fn when(mut self, regex: Regex, step: Step<World>) -> Self {
        self.steps = mem::take(&mut self.steps).when(None, regex, step);
        self
    }

    /// Adds a [Then] [`Step`] matching the given `regex`.
    ///
    /// [Then]: https://cucumber.io/docs/gherkin/reference#then
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
            &'a event::ScenarioFinished,
            Option<&'a mut W>,
        ) -> LocalBoxFuture<'a, ()>
        + 'static,
{
    type Cli = Cli;

    type EventStream =
        LocalBoxStream<'static, parser::Result<Event<event::Cucumber<W>>>>;

    fn with_support_code_store(
        mut self,
        store: crate::support_code::Store,
    ) -> Self {
        self.support_code = Some(store);
        self
    }

    fn run<S>(self, features: S, mut cli: Cli) -> Self::EventStream
    where
        S: Stream<Item = parser::Result<gherkin::Feature>> + 'static,
    {
        if let Some(store) = &self.support_code {
            store.publish(self.steps.support_code());
        }
        #[cfg(feature = "tracing")]
        let logs_collector = *self.logs_collector.swap(Box::new(None));
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
            background_semantics,
            step_hooks,
            scenario_hooks,
            run_hooks,
            timeouts,
            outcome_policy,
            execution_mode,
            scheduling,
            attachments,
            ..
        } = self;

        cli.retry = cli.retry.or(retries);
        cli.retry_after = cli.retry_after.or(retry_after);
        cli.retry_tag_filter = cli.retry_tag_filter.or(retry_filter);
        let fail_fast = cli.fail_fast || fail_fast;
        let concurrency = cli.concurrency
            .or_else(|| scheduling.as_ref().and_then(|s| s.concurrency))
            .or(max_concurrent_scenarios);
        assert!(concurrency != Some(0), "concurrency must be a positive integer");

        let buffer = Features { ordered: scheduling.is_some(), ..Features::default() };
        let (parsed_sender, parsed_receiver) = oneshot::channel();
        let (sender, receiver) = mpsc::unbounded();

        let insert = insert_features(
            buffer.clone(),
            features,
            which_scenario,
            retry_options,
            sender.clone(),
            cli,
            fail_fast,
        ).map(move |()| { let _ = parsed_sender.send(()); });
        let execute = execute(
            buffer,
            concurrency,
            steps,
            sender,
            before_hook,
            after_hook,
            fail_fast,
            background_semantics,
            step_hooks,
            scenario_hooks,
            run_hooks,
            timeouts,
            outcome_policy,
            execution_mode,
            scheduling,
            parsed_receiver,
            attachments,
            #[cfg(feature = "tracing")]
            logs_collector,
        );

        stream::select(
            receiver.map(Either::Left),
            future::join(insert, execute).into_stream().map(Either::Right),
        )
        .filter_map(async |r| match r {
            Either::Left(ev) => Some(ev),
            Either::Right(_) => None,
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
    retries: RetryOptionsFn,
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
            Err(e) => {
                parser_errors += 1;

                // If the receiver end is dropped, then no one listens for the
                // events, so we can just stop from here.
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
// TODO: Needs refactoring.
#[expect(clippy::too_many_lines, reason = "needs refactoring")]
#[cfg_attr(
    feature = "tracing",
    expect(clippy::too_many_arguments, reason = "needs refactoring")
)]
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
    background_semantics: BackgroundSemantics,
    step_hooks: super::StepHooks<W>,
    scenario_hooks: Option<super::Hooks<W>>,
    run_hooks: super::RunHooks,
    timeouts: super::Timeouts,
    outcome_policy: OutcomePolicy,
    execution_mode: ExecutionMode,
    scheduling: Option<super::SchedulingOptions>,
    parsed: oneshot::Receiver<()>,
    attachments: Option<crate::attachment::AttachmentStore>,
    #[cfg(feature = "tracing")] mut logs_collector: Option<TracingCollector>,
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
            &'a event::ScenarioFinished,
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
    if scheduling.is_some() { let _ = parsed.await; }
    let hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));

    let (finished_sender, finished_receiver) = mpsc::unbounded();
    let mut storage = FinishedRulesAndFeatures::new(finished_receiver);
    let executor = Executor::new(
        collection,
        background_semantics,
        step_hooks,
        scenario_hooks,
        timeouts.clone(),
        outcome_policy,
        execution_mode,
        attachments,
        before_hook,
        after_hook,
        event_sender,
        finished_sender,
        features.clone(),
    );

    let run_hook_id = run_hooks.begin_run();
    executor.send_event(event::Cucumber::Started);
    let setup_passed = if execution_mode == ExecutionMode::DryRun {
        run_hooks.skip_before(run_hook_id);
        true
    } else {
        run_hooks.run_before(run_hook_id, &timeouts).await
    };

    #[cfg(feature = "tracing")]
    let waiter = logs_collector
        .as_ref()
        .map(TracingCollector::scenario_span_event_waiter);

    if setup_passed && let Some(options) = scheduling {
        execute_ordered(&executor, &mut storage, &features, max_concurrent_scenarios,
            fail_fast, &options,
            #[cfg(feature = "tracing")] logs_collector.take(),
        ).await;
    } else if setup_passed {
        let mut started_scenarios =
            ControlFlow::Continue(max_concurrent_scenarios);
        let mut run_scenarios = stream::FuturesUnordered::new();
        loop {
            let (runnable, sleep) = features
                .get(started_scenarios.continue_value().unwrap_or(Some(0)))
                .await;
            if run_scenarios.is_empty() && runnable.is_empty() {
                if features.is_finished(started_scenarios.is_break()).await {
                    break;
                }

                // To avoid busy-polling of `Features::get()`, in case there are no
                // scenarios that are running or scheduled for execution, we spawn a
                // thread, that sleeps for minimal deadline of all retried
                // scenarios.
                // TODO: Replace `thread::spawn` with async runtime agnostic sleep,
                //       once it's available.
                if let Some(dur) = sleep {
                    let (sender, receiver) = oneshot::channel();
                    drop(thread::spawn(move || {
                        thread::sleep(dur);
                        sender.send(())
                    }));
                    _ = receiver.await.ok();
                }

                continue;
            }

            let started = storage.start_scenarios(&runnable);
            executor.send_all_events(started);

            {
                #[cfg(feature = "tracing")]
                let forward_logs = {
                    if let Some(coll) = logs_collector.as_mut() {
                        coll.start_scenarios(&runnable);
                    }
                    async {
                        loop {
                            while let Some(logs) = logs_collector
                                .as_mut()
                                .and_then(TracingCollector::emitted_logs)
                            {
                                executor.send_all_events(logs);
                            }
                            future::ready(()).then_yield().await;
                        }
                    }
                };
                #[cfg(feature = "tracing")]
                pin_mut!(forward_logs);
                #[cfg(not(feature = "tracing"))]
                let forward_logs = future::pending();

                if let ControlFlow::Continue(Some(sc)) = &mut started_scenarios
                {
                    *sc -= runnable.len();
                }

                for (id, f, r, s, ty, retries) in runnable {
                    run_scenarios.push(
                        executor
                            .run_scenario(
                                id,
                                f,
                                r,
                                s,
                                ty,
                                retries,
                                RetryDispatch::Queue,
                                #[cfg(feature = "tracing")]
                                waiter.as_ref(),
                            )
                            .then_yield(),
                    );
                }

                let (finished_scenario, _) = select_with_biased_first(
                    forward_logs,
                    run_scenarios.next(),
                )
                .await
                .factor_first();
                if finished_scenario.is_some()
                    && let ControlFlow::Continue(Some(sc)) =
                        &mut started_scenarios
                {
                    *sc += 1;
                }
            }

            while let Ok((id, feat, rule, scenario_failed, retried)) =
                storage.finished_receiver.try_recv()
            {
                if let Some(rule) = rule
                    && let Some(f) = storage.rule_scenario_finished(
                        feat.clone(),
                        rule,
                        retried,
                    )
                {
                    executor.send_event(f);
                }
                if let Some(f) =
                    storage.feature_scenario_finished(feat, retried)
                {
                    executor.send_event(f);
                }
                #[cfg(feature = "tracing")]
                {
                    if let Some(coll) = logs_collector.as_mut() {
                        coll.finish_scenario(id);
                    }
                }
                #[cfg(not(feature = "tracing"))]
                let _: ScenarioId = id;

                if fail_fast && scenario_failed && !retried {
                    started_scenarios = ControlFlow::Break(());
                }
            }
        }
    } else {
        // The parser runs concurrently. Let it finish forwarding source errors
        // and ParsingFinished before closing this run, without scheduling cases.
        while !features.is_finished(true).await {
            future::ready(()).then_yield().await;
        }
    }

    // This is done in case of `fail_fast: true`, when not all `Scenario`s might
    // be executed.
    executor.send_all_events(storage.finish_all_rules_and_features());

    if execution_mode == ExecutionMode::DryRun {
        run_hooks.skip_after(run_hook_id);
        true
    } else {
        run_hooks.run_after(run_hook_id, &timeouts).await
    };
    executor.send_event(event::Cucumber::Finished);

    panic::set_hook(hook);
}

/// Retry placement is private: the legacy queue and an ordered logical slot
/// share exactly the same attempt lifecycle and outcome reporting.
#[derive(Clone, Copy)]
enum RetryDispatch { Queue, ReturnToSlot }

type PlannedCase = (
    ScenarioId, Source<gherkin::Feature>, Option<Source<gherkin::Rule>>,
    Source<gherkin::Scenario>, ScenarioType, Option<RetryOptions>,
);

/// Executes a globally permuted plan, retaining one reservation through retries.
async fn execute_ordered<W, Before, After>(
    executor: &Executor<W, Before, After>,
    storage: &mut FinishedRulesAndFeatures,
    features: &Features,
    limit: Option<usize>,
    fail_fast: bool,
    options: &super::SchedulingOptions,
    #[cfg(feature = "tracing")] logs: Option<TracingCollector>,
) where
    W: World,
    Before: 'static + for<'a> Fn(&'a gherkin::Feature, Option<&'a gherkin::Rule>, &'a gherkin::Scenario, &'a mut W) -> LocalBoxFuture<'a, ()>,
    After: 'static + for<'a> Fn(&'a gherkin::Feature, Option<&'a gherkin::Rule>, &'a gherkin::Scenario, &'a event::ScenarioFinished, Option<&'a mut W>) -> LocalBoxFuture<'a, ()>,
{
    let mut initial = mem::take(&mut *features.plan.lock().await);
    options.order(&mut initial);
    let mut queue = VecDeque::from(initial.into_iter().enumerate().map(|(plan_index, case)| {
        let identity = super::AssignmentCase {
            case_id: plan_index as u64, plan_index, feature: case.1.clone(),
            rule: case.2.clone(), scenario: case.3.clone(), kind: case.4,
        };
        (case, identity)
    }).collect::<Vec<_>>());
    let assignment = options.assignment.as_ref().map(|policy| (policy, policy.begin()));
    let mut reservations = Vec::<super::AssignmentCase>::new();
    let mut revision = 0usize;
    let mut blocked_revision = None;
    let mut idle_interventions = 0usize;
    let mut stopped = false;
    let mut serial_active = false;
    #[cfg(feature = "tracing")]
    let waiter = logs.as_ref().map(TracingCollector::scenario_span_event_waiter);
    #[cfg(feature = "tracing")]
    let logs = std::rc::Rc::new(std::cell::RefCell::new(logs));
    let mut running = stream::FuturesUnordered::<LocalBoxFuture<'_, u64>>::new();

    loop {
        // Drain every attempt, including retries while its logical slot remains
        // occupied. This also closes trace registrations promptly during delays.
        while let Ok((id, feature, rule, failed, retried)) = storage.finished_receiver.try_recv() {
            if let Some(rule) = rule
                && let Some(event) = storage.rule_scenario_finished(feature.clone(), rule, retried) {
                executor.send_event(event);
            }
            if let Some(event) = storage.feature_scenario_finished(feature, retried) { executor.send_event(event); }
            #[cfg(feature = "tracing")]
            if let Some(collector) = logs.borrow_mut().as_mut() { collector.finish_scenario(id); }
            #[cfg(not(feature = "tracing"))]
            let _ = id;
            if fail_fast && failed && !retried { stopped = true; }
        }
        while !stopped && !serial_active && running.len() < limit.unwrap_or(usize::MAX) {
            let Some((_, next)) = queue.front() else { break; };
            // Retry/trace notices do not change the assigned logical set. A
            // blocked snapshot must not re-evaluate or grow records on those notices.
            if blocked_revision == Some(revision) { break; }
            let serial_front = next.kind == ScenarioType::Serial;
            if serial_front && !reservations.is_empty() {
                if let Some((policy, run_id)) = assignment { policy.waiting(run_id, super::AssignmentWait::SerialBarrier, &reservations); }
                blocked_revision = Some(revision);
                break;
            }
            let accessible = if serial_front { 1 } else {
                queue.iter().take_while(|(_, case)| case.kind != ScenarioType::Serial).count()
            };
            let mut reason = super::AssignmentAdmission::Eligible;
            let selected = if let Some((policy, run_id)) = assignment {
                let eligible = queue.iter().take(accessible).position(|(_, candidate)| policy.evaluate(run_id, candidate, &reservations));
                if let Some(index) = eligible { index }
                else if reservations.is_empty() {
                    idle_interventions += 1;
                    reason = super::AssignmentAdmission::IdleFallback;
                    0
                } else {
                    policy.waiting(run_id, super::AssignmentWait::NoEligible, &reservations);
                    blocked_revision = Some(revision);
                    break;
                }
            } else { 0 };
            let (case, identity) = queue.remove(selected).expect("selected queue entry exists");
            let case_id = identity.case_id;
            serial_active = identity.kind == ScenarioType::Serial;
            reservations.push(identity.clone());
            revision += 1;
            if let Some((policy, run_id)) = assignment { policy.admitted(run_id, &identity, reason); }
            executor.send_all_events(storage.start_scenarios([case.clone()]));
            #[cfg(feature = "tracing")]
            let attempt_logs = logs.clone();
            #[cfg(feature = "tracing")]
            let attempt_waiter = waiter.as_ref();
            running.push(async move {
                let (mut id, feature, rule, scenario, ty, mut retries) = case;
                loop {
                    #[cfg(feature = "tracing")]
                    if let Some(collector) = attempt_logs.borrow_mut().as_mut() {
                        collector.start_scenarios([(id, feature.clone(), rule.clone(), scenario.clone(), ty, retries)]);
                    }
                    let next = executor.run_scenario(id, feature.clone(), rule.clone(), scenario.clone(), ty, retries,
                        RetryDispatch::ReturnToSlot,
                        #[cfg(feature = "tracing")] attempt_waiter,
                    ).await;
                    let Some(next) = next else { break; };
                    if let Some(delay) = next.after { options.delay(delay).await; }
                    // Yield even for an immediate retry, permitting the dispatcher
                    // to consume the previous attempt's finish and trace records.
                    future::ready(()).then_yield().await;
                    id = ScenarioId::new();
                    retries = Some(next);
                }
                case_id
            }.boxed_local());
        }
        if running.is_empty() { break; }

        // Notifications wake the scheduler even while all slots await a retry
        // delay. No polling of a potentially unfinished parser is necessary.
        let progress_result = {
        let progress = async {
            let completed = running.next();
            let notice = storage.finished_receiver.next();
            pin_mut!(completed, notice);
            match future::select(completed, notice).await {
                Either::Left((completed, _)) => Either::Left(completed),
                Either::Right((notice, _)) => Either::Right(notice),
            }
        };
        #[cfg(feature = "tracing")]
        let forward = async {
            loop {
                while let Some(events) = logs.borrow_mut().as_mut().and_then(TracingCollector::emitted_logs) {
                    executor.send_all_events(events);
                }
                future::ready(()).then_yield().await;
            }
        };
        #[cfg(not(feature = "tracing"))]
        let forward = future::pending::<()>();
        pin_mut!(progress, forward);
        let result = match select_with_biased_first(forward, progress).await {
            Either::Right((result, _)) => result,
            Either::Left(_) => unreachable!("log forwarding does not complete"),
        };
        result
        };
        match progress_result {
        Either::Left(Some(case_id)) => {
            let index = reservations.iter().position(|case| case.case_id == case_id).expect("completed logical reservation exists");
            let released = reservations.remove(index);
            if released.kind == ScenarioType::Serial { serial_active = false; }
            revision += 1;
            if let Some((policy, run_id)) = assignment { policy.released(run_id, case_id); }
        }
        Either::Right(Some((id, feature, rule, failed, retried))) => {
            if let Some(rule) = rule
                && let Some(event) = storage.rule_scenario_finished(feature.clone(), rule, retried) { executor.send_event(event); }
            if let Some(event) = storage.feature_scenario_finished(feature, retried) { executor.send_event(event); }
            #[cfg(feature = "tracing")]
            if let Some(collector) = logs.borrow_mut().as_mut() { collector.finish_scenario(id); }
            #[cfg(not(feature = "tracing"))]
            let _ = id;
            if fail_fast && failed && !retried { stopped = true; }
        }
        Either::Left(None) | Either::Right(None) => break,
        }
    }
    if let Some((policy, run_id)) = assignment { policy.finish(run_id, idle_interventions); }
}

/// Runs [`Scenario`]s and notifies about their state of completion.
///
/// [`Scenario`]: gherkin::Scenario
struct Executor<W, Before, After> {
    /// [`Step`]s [`Collection`].
    ///
    /// [`Collection`]: step::Collection
    collection: step::Collection<W>,

    /// Background policy inherited from the configured runner.
    background_semantics: BackgroundSemantics,

    /// Ordered callbacks around resolved step bodies.
    step_hooks: super::StepHooks<W>,
    scenario_hooks: Option<super::Hooks<W>>,
    timeouts: super::Timeouts,
    outcome_policy: OutcomePolicy,
    execution_mode: ExecutionMode,
    attachments: Option<crate::attachment::AttachmentStore>,

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
    const fn new(
        collection: step::Collection<W>,
        background_semantics: BackgroundSemantics,
        step_hooks: super::StepHooks<W>,
        scenario_hooks: Option<super::Hooks<W>>,
        timeouts: super::Timeouts,
        outcome_policy: OutcomePolicy,
        execution_mode: ExecutionMode,
        attachments: Option<crate::attachment::AttachmentStore>,
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
            background_semantics,
            step_hooks,
            scenario_hooks,
            timeouts,
            outcome_policy,
            execution_mode,
            attachments,
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
    // TODO: Needs refactoring.
    #[expect(clippy::too_many_lines, reason = "needs refactoring")]
    #[cfg_attr(
        feature = "tracing",
        expect(clippy::too_many_arguments, reason = "needs refactoring")
    )]
    async fn run_scenario(
        &self,
        id: ScenarioId,
        feature: Source<gherkin::Feature>,
        rule: Option<Source<gherkin::Rule>>,
        scenario: Source<gherkin::Scenario>,
        scenario_ty: ScenarioType,
        retries: Option<RetryOptions>,
        retry_dispatch: RetryDispatch,
        #[cfg(feature = "tracing")] waiter: Option<&SpanCloseWaiter>,
    ) -> Option<RetryOptions> {
        let retry_num = retries.map(|r| r.retries);
        let attachment_context = self.attachments.as_ref().map_or_else(
            crate::attachment::AttachmentContext::default,
            |store| {
                store.context(
                    &feature,
                    rule.as_deref(),
                    &scenario,
                    retry_num.map_or(0, |retry| retry.current),
                )
            },
        );

        let has_steps = !scenario.steps.is_empty()
            || (self.background_semantics == BackgroundSemantics::Legacy
                && (feature
                    .background
                    .as_ref()
                    .is_some_and(|b| !b.steps.is_empty())
                    || rule
                        .as_ref()
                        .and_then(|r| r.background.as_ref())
                        .is_some_and(|b| !b.steps.is_empty())));
        let hook_attempt = super::ScenarioHookAttempt {
            feature: feature.clone(),
            rule: rule.clone(),
            scenario: scenario.clone(),
            scenario_id: id,
            attempt: retry_num.map_or(0, |r| r.current),
            retries_remaining: retry_num.map_or(0, |r| r.left),
            has_steps,
        };
        if let Some(hooks) = &self.scenario_hooks {
            hooks.begin_attempt(hook_attempt.clone());
        }

        let execution = if self.outcome_policy == OutcomePolicy::Legacy
            && self.execution_mode == ExecutionMode::Run
        {
            ExecutionControl::default()
        } else {
            ExecutionControl::enabled()
        };
        let runtime_context =
            WorldContext::new(attachment_context.clone(), execution);

        self.send_event(event::Cucumber::scenario(
            feature.clone(),
            rule.clone(),
            scenario.clone(),
            event::Scenario::Started.with_retries(retry_num),
        ));

        let is_failed = async {
            let mut deferred = Vec::new();
            let mut status = if self.execution_mode == ExecutionMode::DryRun {
                OutcomeStatus::Skipped
            } else {
                OutcomeStatus::Passed
            };
            let mut body_snapshot = event::ScenarioFinished::StepPassed;
            let mut world = match self
                .run_before_hook(
                    &feature,
                    rule.as_ref(),
                    &scenario,
                    retry_num,
                    id,
                    &attachment_context,
                    &hook_attempt,
                    &runtime_context,
                    #[cfg(feature = "tracing")]
                    waiter,
                )
                .await
            {
                Ok(world) => world,
                Err(mut error) => {
                    status = error.status();
                    body_snapshot = error.get_scenario_finished_event();
                    let world = error.take_world();
                    deferred.push(DeferredEvent::Failure(error));
                    world
                }
            };
            let backgrounds = self.background_semantics
                == BackgroundSemantics::Legacy
                || !scenario.steps.is_empty();
            let feature_steps = feature
                .background
                .as_ref()
                .filter(|_| backgrounds)
                .into_iter()
                .flat_map(|b| b.steps.iter())
                .map(|step| (Source::new(step.clone()), true));
            let rule_steps = rule
                .as_ref()
                .and_then(|r| r.background.as_ref())
                .filter(|_| backgrounds)
                .into_iter()
                .flat_map(|b| b.steps.iter())
                .map(|step| (Source::new(step.clone()), true));
            let own_steps = scenario
                .steps
                .iter()
                .map(|step| (Source::new(step.clone()), false));
            for (step, is_background) in
                feature_steps.chain(rule_steps).chain(own_steps)
            {
                if self.execution_mode == ExecutionMode::Run
                    && self.outcome_policy == OutcomePolicy::Legacy
                    && status != OutcomeStatus::Passed
                {
                    break;
                }
                let mode = if self.execution_mode == ExecutionMode::DryRun {
                    StepMode::LookupOnly
                } else if status == OutcomeStatus::Passed {
                    StepMode::Execute
                } else if status == OutcomeStatus::Skipped {
                    StepMode::ExplicitSkip
                } else {
                    StepMode::LookupOnly
                };
                match self
                    .run_step(
                        world,
                        step,
                        is_background,
                        mode,
                        &hook_attempt,
                        retry_num,
                        &attachment_context,
                        &runtime_context,
                        &mut deferred,
                        #[cfg(feature = "tracing")]
                        waiter,
                    )
                    .await
                {
                    Ok(value) => world = Some(value),
                    Err(mut error) => {
                        world = error.take_world();
                        let next = error.status();
                        if next > status {
                            status = next;
                            body_snapshot = error.get_scenario_finished_event();
                        }
                        if !matches!(
                            error,
                            ExecutionFailure::StepSkipped { .. }
                        ) {
                            deferred.push(DeferredEvent::Failure(error));
                        }
                    }
                }
            }
            let body_outcome =
                self.execution_mode.outcome(self.outcome_policy, status);
            let (world, after_hook_meta, after_hook_error) = self
                .run_after_hook(
                    world,
                    &feature,
                    rule.as_ref(),
                    &scenario,
                    body_snapshot,
                    id,
                    &attachment_context,
                    &hook_attempt,
                    &runtime_context,
                    body_outcome,
                    #[cfg(feature = "tracing")]
                    waiter,
                )
                .await
                .map_or_else(
                    |(world, meta, error)| {
                        (world.map(Arc::new), Some(meta), Some(error))
                    },
                    |(world, meta)| (world.map(Arc::new), meta, None),
                );
            if let Some(meta) = &after_hook_meta {
                let after = meta.finished.outcome.map(|o| o.status).unwrap_or(
                    if after_hook_error.is_some() {
                        OutcomeStatus::Failed
                    } else {
                        OutcomeStatus::Passed
                    },
                );
                status = status.max(after);
            }
            if let Some(hooks) = &self.scenario_hooks {
                hooks.finish_attempt(id);
            }
            for pending in deferred {
                match pending {
                    DeferredEvent::Native(event) => {
                        drop(self.event_sender.unbounded_send(Ok(event)))
                    }
                    DeferredEvent::Failure(error) => self.emit_failed_events(
                        feature.clone(),
                        rule.clone(),
                        scenario.clone(),
                        world.clone(),
                        error,
                        retry_num,
                    ),
                }
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
            self.execution_mode.outcome(self.outcome_policy, status)
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

        self.send_event_with_meta(
            event::Cucumber::scenario(
                feature.clone(),
                rule.clone(),
                scenario.clone(),
                event::Scenario::Finished.with_retries(retry_num),
            ),
            self.outcome_meta(is_failed.status),
        );

        let next_try = retries
            .filter(|_| is_failed.retryable)
            .and_then(RetryOptions::next_try);
        if let Some(next_try) = next_try.filter(|_| matches!(retry_dispatch, RetryDispatch::Queue)) {
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
            is_failed.fails_run,
            next_try.is_some(),
        );
        next_try
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
        attachment_context: &crate::attachment::AttachmentContext,
        hook_attempt: &super::ScenarioHookAttempt,
        runtime_context: &WorldContext,
        #[cfg(feature = "tracing")] waiter: Option<&SpanCloseWaiter>,
    ) -> Result<Option<W>, ExecutionFailure<W>> {
        if self.execution_mode == ExecutionMode::DryRun {
            return self
                .inspect_world(
                    feature,
                    rule,
                    scenario,
                    retries,
                    hook_attempt,
                    runtime_context,
                )
                .await;
        }
        let registry =
            self.scenario_hooks.as_ref().filter(|hooks| hooks.has_before());
        if self.before_hook.is_none() && registry.is_none() {
            return Ok(None);
        }
        let mut attachment_scope = Some(
            attachment_context.enter(crate::attachment::Target::BeforeScenario),
        );
        self.send_event(event::Cucumber::scenario(
            feature.clone(),
            rule.cloned(),
            scenario.clone(),
            event::Scenario::hook_started(HookType::Before)
                .with_retries(retries),
        ));
        let run = async {
            let started_at = std::time::SystemTime::now();
            let initialized = AssertUnwindSafe(async {
                W::new_with_runtime_context(runtime_context.clone()).await
            })
            .catch_unwind()
            .then_yield()
            .await
            .map_err(Info::from)
            .and_then(|r| {
                r.map_err(|error| {
                    coerce_into_info(format!(
                        "failed to initialize World: {error}"
                    ))
                })
            });
            if let Some(hooks) = registry {
                hooks.initialized(
                    hook_attempt,
                    started_at,
                    initialized.as_ref().map(|_| ()).map_err(|info| {
                        super::step_hooks::panic_message(info.as_ref())
                    }),
                );
                drop(attachment_scope.take());
            }
            let mut world = initialized
                .map_err(|info| (info, None, OutcomeStatus::Failed))?;
            let execution = runtime_context.execution();
            let (status, error) = if let Some(hooks) = registry {
                let summary = hooks
                    .run_before(
                        hook_attempt,
                        &mut world,
                        attachment_context,
                        &self.timeouts,
                        &execution,
                        self.outcome_policy,
                    )
                    .await;
                let error =
                    (summary.status != OutcomeStatus::Passed).then(|| {
                        coerce_into_info(summary.diagnostic("Before hook"))
                    });
                (summary.status, error)
            } else {
                let control = execution.enter();
                let callback_world = &mut world;
                let result = self
                    .timeouts
                    .invoke_callback(
                        move || -> LocalBoxFuture<'_, ()> {
                            if let Some(hook) = &self.before_hook {
                                hook(feature, rule.map(AsRef::as_ref), scenario, callback_world)
                            } else { Box::pin(future::ready(())) }
                        },
                        None,
                    )
                    .await;
                let completion = control.finish();
                let status = super::outcomes::status(&result, completion);
                let error = result
                    .err()
                    .map(|error| Info::from(super::timeout::into_panic(error)))
                    .or_else(|| {
                        (status != OutcomeStatus::Passed).then(|| {
                            coerce_into_info(format!(
                                "Before hook requested {status:?}"
                            ))
                        })
                    });
                (status, error)
            };
            match error {
                Some(error) => Err((error, Some(world), status)),
                None => Ok(world),
            }
        };
        #[cfg(feature = "tracing")]
        let (run, span_id) = {
            let span = scenario_id.hook_span(HookType::Before);
            let id = span.id();
            (tracing::Instrument::instrument(run, span), id)
        };
        #[cfg(not(feature = "tracing"))]
        let _ = scenario_id;
        let result = run.then_yield().await;
        drop(attachment_scope);
        #[cfg(feature = "tracing")]
        if let Some((waiter, id)) = waiter.zip(span_id) {
            waiter.wait_for_span_close(id).then_yield().await;
        }
        match result {
            Ok(world) => {
                self.send_event_with_meta(
                    event::Cucumber::scenario(
                        feature.clone(),
                        rule.cloned(),
                        scenario.clone(),
                        event::Scenario::hook_passed(HookType::Before)
                            .with_retries(retries),
                    ),
                    self.outcome_meta(OutcomeStatus::Passed),
                );
                Ok(Some(world))
            }
            Err((panic_info, world, status)) => {
                Err(ExecutionFailure::BeforeHookPanicked {
                    world,
                    panic_info,
                    meta: self.outcome_meta(status),
                })
            }
        }
    }

    /// Initializes a selected dry-run World without entering callback capabilities.
    async fn inspect_world(
        &self,
        feature: &Source<gherkin::Feature>,
        rule: Option<&Source<gherkin::Rule>>,
        scenario: &Source<gherkin::Scenario>,
        retries: Option<Retries>,
        attempt: &super::ScenarioHookAttempt,
        runtime: &WorldContext,
    ) -> Result<Option<W>, ExecutionFailure<W>> {
        let registry = self.scenario_hooks.as_ref().filter(|h| h.has_before());
        let stage = self.before_hook.is_some() || registry.is_some();
        let started = event::Metadata::new(());
        if stage {
            self.send_event_with_meta(
                event::Cucumber::scenario(
                    feature.clone(),
                    rule.cloned(),
                    scenario.clone(),
                    event::Scenario::hook_started(HookType::Before)
                        .with_retries(retries),
                ),
                started,
            );
        }
        let initialized_at = std::time::SystemTime::now();
        let result = AssertUnwindSafe(async {
            W::new_with_runtime_context(runtime.clone()).await
        })
        .catch_unwind()
        .then_yield()
        .await
        .map_err(Info::from)
        .and_then(|r| {
            r.map_err(|error| {
                coerce_into_info(format!("failed to initialize World: {error}"))
            })
        });
        if let Some(hooks) = registry {
            hooks.initialized(
                attempt,
                initialized_at,
                result.as_ref().map(|_| ()).map_err(|info| {
                    super::step_hooks::panic_message(info.as_ref())
                }),
            );
            if result.is_ok() {
                hooks.skip_stage(
                    attempt,
                    super::ScenarioHookKind::Before,
                    super::ScenarioHookSkipReason::DryRun,
                );
            }
        }
        match result {
            Ok(world) if !stage => Ok(Some(world)),
            Ok(world) => {
                let mut meta = started;
                meta.outcome = self.outcome_metadata(OutcomeStatus::Skipped);
                Err(ExecutionFailure::BeforeHookPanicked {
                    world: Some(world),
                    panic_info: coerce_into_info(
                        "Before hooks skipped during dry-run".to_owned(),
                    ),
                    meta,
                })
            }
            Err(panic_info) => {
                if !stage {
                    self.send_event_with_meta(
                        event::Cucumber::scenario(
                            feature.clone(),
                            rule.cloned(),
                            scenario.clone(),
                            event::Scenario::hook_started(HookType::Before)
                                .with_retries(retries),
                        ),
                        started,
                    );
                }
                Err(ExecutionFailure::BeforeHookPanicked {
                    world: None,
                    panic_info,
                    meta: self.outcome_meta(OutcomeStatus::Failed),
                })
            }
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
    async fn run_step(
        &self,
        world_opt: Option<W>,
        step: Source<gherkin::Step>,
        is_background: bool,
        mode: StepMode,
        attempt: &super::ScenarioHookAttempt,
        retries: Option<Retries>,
        attachments: &crate::attachment::AttachmentContext,
        runtime: &WorldContext,
        deferred: &mut Vec<DeferredEvent<W>>,
        #[cfg(feature = "tracing")] waiter: Option<&SpanCloseWaiter>,
    ) -> Result<W, ExecutionFailure<W>> {
        let started = event::Metadata::new(());
        self.queue_step_event(
            attempt,
            step.clone(),
            is_background,
            event::Step::Started,
            retries,
            started,
            deferred,
        );
        // Only a real invocation receives an attachment scope or timeout.
        let mut attachment_scope =
            (mode == StepMode::Execute).then(|| attachments.enter_step());
        let run = async {
            if mode == StepMode::ExplicitSkip {
                return Ok((OutcomeStatus::Skipped, None, None, world_opt));
            }
            let ((step_fn, captures, loc, ctx), options) =
                match self.collection.find_with_options(&step) {
                    Ok(Some(found)) => found,
                    Ok(None) => {
                        return Ok((
                            OutcomeStatus::Undefined,
                            None,
                            None,
                            world_opt,
                        ));
                    }
                    Err(error) => {
                        return Err((
                            event::StepError::AmbiguousMatch(error),
                            None,
                            None,
                            world_opt,
                        ));
                    }
                };
            if mode == StepMode::LookupOnly {
                return Ok((
                    OutcomeStatus::Skipped,
                    Some(captures),
                    loc,
                    world_opt,
                ));
            }
            let mut world = if let Some(world) = world_opt {
                world
            } else {
                match AssertUnwindSafe(async {
                    W::new_with_runtime_context(runtime.clone()).await
                })
                .catch_unwind()
                .then_yield()
                .await
                {
                    Ok(Ok(world)) => world,
                    Ok(Err(error)) => {
                        return Err((
                            event::StepError::Panic(coerce_into_info(format!(
                                "failed to initialize `World`: {error}"
                            ))),
                            None,
                            loc,
                            None,
                        ));
                    }
                    Err(error) => {
                        return Err((
                            event::StepError::Panic(error.into()),
                            None,
                            loc,
                            None,
                        ));
                    }
                }
            };
            let scope = super::StepHookScope {
                feature: attempt.feature.clone(),
                rule: attempt.rule.clone(),
                scenario: attempt.scenario.clone(),
                step: step.clone(),
                scenario_id: attempt.scenario_id,
                attempt: attempt.attempt,
                is_background,
            };
            let execution = runtime.execution();
            let guard = attachment_scope
                .as_mut()
                .expect("executing step has attachment scope");
            let mut summary = self
                .step_hooks
                .run_before(
                    &scope,
                    &mut world,
                    &self.timeouts,
                    guard,
                    &execution,
                    self.outcome_policy,
                )
                .await;
            let (body, body_status) = if summary.status != OutcomeStatus::Failed
            {
                let control = execution.enter();
                let callback_world = &mut world;
                let result = self
                    .timeouts
                    .invoke_callback(
                        move || step_fn(callback_world, ctx),
                        options.timeout_policy(),
                    )
                    .await;
                let completion = control.finish();
                if result
                    .as_ref()
                    .is_err_and(|error| error.is::<super::TimeoutError>())
                {
                    guard.renew();
                }
                let status = super::outcomes::status(&result, completion);
                (Some(result.map_err(super::timeout::into_panic)), status)
            } else {
                (None, OutcomeStatus::Skipped)
            };
            let body_outcome = match &body {
                None => super::StepBodyOutcome::NotRun,
                Some(Err(error)) => super::StepBodyOutcome::Failed(
                    super::step_hooks::panic_message(error.as_ref()),
                ),
                Some(Ok(())) => match body_status {
                    OutcomeStatus::Pending => super::StepBodyOutcome::Pending,
                    OutcomeStatus::Skipped => super::StepBodyOutcome::Skipped,
                    _ => super::StepBodyOutcome::Passed,
                },
            };
            if body.is_some() {
                summary.status = summary.status.max(body_status);
            }
            summary.merge(
                self.step_hooks
                    .run_after(
                        &scope,
                        &body_outcome,
                        &mut world,
                        &self.timeouts,
                        guard,
                        &execution,
                        self.outcome_policy,
                    )
                    .await,
            );
            if !summary.failures.is_empty() {
                if let super::StepBodyOutcome::Failed(message) = body_outcome {
                    summary.failures.insert(0, format!("Step body: {message}"));
                }
                return Err((
                    event::StepError::Panic(coerce_into_info(
                        summary.failures.join("; "),
                    )),
                    Some(captures),
                    loc,
                    Some(world),
                ));
            }
            if let Some(Err(error)) = body {
                return Err((
                    event::StepError::Panic(error.into()),
                    Some(captures),
                    loc,
                    Some(world),
                ));
            }
            Ok((summary.status, Some(captures), loc, Some(world)))
        };
        #[cfg(feature = "tracing")]
        let (run, span_id) = {
            let span = attempt.scenario_id.step_span(is_background);
            let id = span.id();
            (tracing::Instrument::instrument(run, span), id)
        };
        let result = run.then_yield().await;
        drop(attachment_scope);
        #[cfg(feature = "tracing")]
        if let Some((waiter, id)) = waiter.zip(span_id) {
            waiter.wait_for_span_close(id).then_yield().await;
        }
        let mut finished = if mode == StepMode::Execute {
            event::Metadata::new(())
        } else {
            started
        };
        match result {
            Ok((OutcomeStatus::Passed, Some(captures), loc, Some(world))) => {
                finished.outcome = self.outcome_metadata(OutcomeStatus::Passed);
                self.queue_step_event(
                    attempt,
                    step,
                    is_background,
                    event::Step::Passed(captures, loc),
                    retries,
                    finished,
                    deferred,
                );
                Ok(world)
            }
            Ok((status, _, _, world)) => {
                finished.outcome = self.outcome_metadata(status);
                self.queue_step_event(
                    attempt,
                    step,
                    is_background,
                    event::Step::Skipped,
                    retries,
                    finished,
                    deferred,
                );
                Err(ExecutionFailure::StepSkipped {
                    world,
                    outcome: finished.outcome,
                })
            }
            Err((err, captures, loc, world)) => {
                let status =
                    if matches!(err, event::StepError::AmbiguousMatch(_)) {
                        OutcomeStatus::Ambiguous
                    } else {
                        OutcomeStatus::Failed
                    };
                finished.outcome = self.outcome_metadata(status);
                Err(ExecutionFailure::StepPanicked {
                    world,
                    step,
                    captures,
                    loc,
                    err,
                    meta: finished,
                    is_background,
                })
            }
        }
    }

    fn queue_step_event(
        &self,
        attempt: &super::ScenarioHookAttempt,
        step: Source<gherkin::Step>,
        background: bool,
        value: event::Step<W>,
        retries: Option<Retries>,
        meta: event::Metadata,
        deferred: &mut Vec<DeferredEvent<W>>,
    ) {
        let value = if background {
            event::Scenario::Background(step, value)
        } else {
            event::Scenario::Step(step, value)
        };
        let event = meta.wrap(event::Cucumber::scenario(
            attempt.feature.clone(),
            attempt.rule.clone(),
            attempt.scenario.clone(),
            value.with_retries(retries),
        ));
        if deferred.is_empty() {
            drop(self.event_sender.unbounded_send(Ok(event)));
        } else {
            deferred.push(DeferredEvent::Native(event));
        }
    }

    fn outcome_metadata(&self, status: OutcomeStatus) -> Option<Outcome> {
        if self.execution_mode == ExecutionMode::DryRun {
            Some(self.execution_mode.outcome(self.outcome_policy, status))
        } else {
            super::outcomes::metadata(self.outcome_policy, status)
        }
    }

    fn outcome_meta(&self, status: OutcomeStatus) -> event::Metadata {
        let mut meta = event::Metadata::new(());
        meta.outcome = self.outcome_metadata(status);
        meta
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
            ExecutionFailure::StepSkipped { .. } => {}
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
    // TODO: Needs refactoring.
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
        attachment_context: &crate::attachment::AttachmentContext,
        attempt: &super::ScenarioHookAttempt,
        runtime: &WorldContext,
        body_outcome: Outcome,
        #[cfg(feature = "tracing")] waiter: Option<&SpanCloseWaiter>,
    ) -> Result<
        (Option<W>, Option<AfterHookEventsMeta>),
        (Option<W>, AfterHookEventsMeta, Info),
    > {
        if self.execution_mode == ExecutionMode::DryRun {
            let registry =
                self.scenario_hooks.as_ref().filter(|h| h.has_after());
            if self.after_hook.is_none() && registry.is_none() {
                return Ok((world, None));
            }
            if let Some(hooks) = registry {
                hooks.skip_stage(
                    attempt,
                    super::ScenarioHookKind::After,
                    super::ScenarioHookSkipReason::DryRun,
                );
            }
            let started = event::Metadata::new(());
            let mut finished = started;
            finished.outcome = self.outcome_metadata(OutcomeStatus::Skipped);
            return Err((
                world,
                AfterHookEventsMeta { started, finished },
                coerce_into_info("After hooks skipped during dry-run".to_owned()),
            ));
        }
        let registry = self.scenario_hooks.as_ref().filter(|h| h.has_after());
        if self.after_hook.is_none() && registry.is_none() {
            return Ok((world, None));
        }
        let attachment_scope = registry.is_none().then(|| {
            attachment_context.enter(crate::attachment::Target::AfterScenario)
        });
        let started = event::Metadata::new(());
        let run = async {
            let execution = runtime.execution();
            if let Some(hooks) = registry {
                let summary = hooks
                    .run_after(
                        attempt,
                        &ev,
                        world.as_mut(),
                        attachment_context,
                        &self.timeouts,
                        &execution,
                        self.outcome_policy,
                        body_outcome,
                    )
                    .await;
                let error =
                    (summary.status != OutcomeStatus::Passed).then(|| {
                        coerce_into_info(summary.diagnostic("After hook"))
                    });
                (summary.status, error)
            } else {
                let control = execution.enter();
                let callback_world = world.as_mut();
                let callback_event = &ev;
                let result = self
                    .timeouts
                    .invoke_callback(
                        move || -> LocalBoxFuture<'_, ()> {
                            if let Some(hook) = &self.after_hook {
                                hook(feature, rule.map(AsRef::as_ref), scenario, callback_event, callback_world)
                            } else { Box::pin(future::ready(())) }
                        },
                        None,
                    )
                    .await;
                let completion = control.finish();
                let status = super::outcomes::status(&result, completion);
                let error = result
                    .err()
                    .map(|error| Info::from(super::timeout::into_panic(error)))
                    .or_else(|| {
                        (status != OutcomeStatus::Passed).then(|| {
                            coerce_into_info(format!(
                                "After hook requested {status:?}"
                            ))
                        })
                    });
                (status, error)
            }
        };
        #[cfg(feature = "tracing")]
        let (run, span_id) = {
            let span = scenario_id.hook_span(HookType::After);
            let id = span.id();
            (tracing::Instrument::instrument(run, span), id)
        };
        #[cfg(not(feature = "tracing"))]
        let _ = scenario_id;
        let (status, error) = run.then_yield().await;
        drop(attachment_scope);
        #[cfg(feature = "tracing")]
        if let Some((waiter, id)) = waiter.zip(span_id) {
            waiter.wait_for_span_close(id).then_yield().await;
        }
        let meta = AfterHookEventsMeta {
            started,
            finished: self.outcome_meta(status),
        };
        match error {
            Some(error) => Err((world, meta, error)),
            None => Ok((world, Some(meta))),
        }
    }

    /// Emits all the [`HookType::After`] events.
    ///
    /// See [`Self::emit_failed_events()`] for the explanation why we don't do
    /// that inside [`Self::run_after_hook()`].
    // TODO: Needs refactoring.
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
            self.after_hook.is_some()
                || self.scenario_hooks.as_ref().is_some_and(|h| h.has_after()),
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

/// ID of a [`Scenario`], uniquely identifying it.
///
/// **NOTE**: Retried [`Scenario`] has a different ID from a failed one.
///
/// [`Scenario`]: gherkin::Scenario
#[derive(Clone, Copy, Debug, Display, Eq, FromStr, Hash, PartialEq)]
pub struct ScenarioId(pub(crate) u64);

impl ScenarioId {
    /// Creates a new unique [`ScenarioId`].
    pub fn new() -> Self {
        /// [`AtomicU64`] ID.
        static ID: AtomicU64 = AtomicU64::new(0);

        Self(ID.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for ScenarioId {
    fn default() -> Self {
        Self::new()
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
    features_scenarios_count: HashMap<Source<gherkin::Feature>, usize>,

    /// Number of finished [`Scenario`]s of [`Rule`].
    ///
    /// We also store path to a [`Feature`], so [`Rule`]s with same names and
    /// spans in different `.feature` files will have different hashes.
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Rule`]: gherkin::Rule
    /// [`Scenario`]: gherkin::Scenario
    rule_scenarios_count:
        HashMap<(Source<gherkin::Feature>, Source<gherkin::Rule>), usize>,

    /// Receiver for notifying state of [`Scenario`]s completion.
    ///
    /// [`Scenario`]: gherkin::Scenario
    finished_receiver: FinishedFeaturesReceiver,
}

/// Alias of a [`mpsc::UnboundedSender`] that notifies about finished
/// [`Feature`]s.
///
/// [`Feature`]: gherkin::Feature
type FinishedFeaturesSender = mpsc::UnboundedSender<(
    ScenarioId,
    Source<gherkin::Feature>,
    Option<Source<gherkin::Rule>>,
    IsFailed,
    IsRetried,
)>;

/// Alias of a [`mpsc::UnboundedReceiver`] that receives events about finished
/// [`Feature`]s.
///
/// [`Feature`]: gherkin::Feature
type FinishedFeaturesReceiver = mpsc::UnboundedReceiver<(
    ScenarioId,
    Source<gherkin::Feature>,
    Option<Source<gherkin::Rule>>,
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
        feature: Source<gherkin::Feature>,
        rule: Source<gherkin::Rule>,
        is_retried: bool,
    ) -> Option<event::Cucumber<W>> {
        if is_retried {
            return None;
        }

        let finished_scenarios = self
            .rule_scenarios_count
            .get_mut(&(feature.clone(), rule.clone()))
            .unwrap_or_else(|| panic!("no `Rule: {}`", rule.name));
        *finished_scenarios += 1;
        (rule.scenarios.len() == *finished_scenarios).then(|| {
            _ = self
                .rule_scenarios_count
                .remove(&(feature.clone(), rule.clone()));
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
        feature: Source<gherkin::Feature>,
        is_retried: bool,
    ) -> Option<event::Cucumber<W>> {
        if is_retried {
            return None;
        }

        let finished_scenarios = self
            .features_scenarios_count
            .get_mut(&feature)
            .unwrap_or_else(|| panic!("no `Feature: {}`", feature.name));
        *finished_scenarios += 1;
        let scenarios = feature.count_scenarios();
        (scenarios == *finished_scenarios).then(|| {
            _ = self.features_scenarios_count.remove(&feature);
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
    ) -> impl Iterator<Item = event::Cucumber<W>> {
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
    fn start_scenarios<W, R>(
        &mut self,
        runnable: R,
    ) -> impl Iterator<Item = event::Cucumber<W>> + use<W, R>
    where
        R: AsRef<
            [(
                ScenarioId,
                Source<gherkin::Feature>,
                Option<Source<gherkin::Rule>>,
                Source<gherkin::Scenario>,
                ScenarioType,
                Option<RetryOptions>,
            )],
        >,
    {
        let runnable = runnable.as_ref();

        let mut started_features = Vec::new();
        for feature in runnable.iter().map(|(_, f, ..)| f.clone()).dedup() {
            _ = self
                .features_scenarios_count
                .entry(feature.clone())
                .or_insert_with(|| {
                    started_features.push(feature);
                    0
                });
        }

        let mut started_rules = Vec::new();
        for (feat, rule) in runnable
            .iter()
            .filter_map(|(_, feat, rule, _, _, _)| {
                rule.clone().map(|r| (feat.clone(), r))
            })
            .dedup()
        {
            _ = self
                .rule_scenarios_count
                .entry((feat.clone(), rule.clone()))
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
        ScenarioId,
        Source<gherkin::Feature>,
        Option<Source<gherkin::Rule>>,
        Source<gherkin::Scenario>,
        Option<RetryOptionsWithDeadline>,
    )>,
>;

/// Alias of a [`Features::insert_scenarios()`] argument.
type InsertedScenarios = HashMap<
    ScenarioType,
    Vec<(
        ScenarioId,
        Source<gherkin::Feature>,
        Option<Source<gherkin::Rule>>,
        Source<gherkin::Scenario>,
        Option<RetryOptions>,
    )>,
>;

/// Storage sorted by [`ScenarioType`] [`Feature`]'s [`Scenario`]s.
///
/// [`Feature`]: gherkin::Feature
/// [`Scenario`]: gherkin::Scenario
#[derive(Clone, Default)]
struct Features {
    ordered: bool,
    plan: Arc<Mutex<Vec<PlannedCase>>>,
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
        retry: &RetryOptionsFn,
        cli: &Cli,
    ) where
        Which: Fn(
                &gherkin::Feature,
                Option<&gherkin::Rule>,
                &gherkin::Scenario,
            ) -> ScenarioType
            + 'static,
    {
        let feature = Source::new(feature);

        let mut cases = feature.scenarios.iter().map(|s| (None, s))
            .chain(feature.rules.iter().flat_map(|r| {
                let rule = Some(Source::new(r.clone()));
                r.scenarios.iter().map(|s| (rule.clone(), s)).collect::<Vec<_>>()
            })).collect::<Vec<_>>();
        if self.ordered { cases.sort_by_key(|(_, scenario)| scenario.position.line); }
        let cases = cases.into_iter().map(|(rule, scenario)| {
            let retries = retry(&feature, rule.as_deref(), scenario, cli);
            let ty = which_scenario(&feature, rule.as_deref(), scenario);
            (ScenarioId::new(), feature.clone(), rule, Source::new(scenario.clone()), ty, retries)
        }).collect::<Vec<_>>();
        if self.ordered { self.plan.lock().await.extend(cases); return; }
        let local = cases.into_iter().map(|(id,f,r,s,ty,retry)| (ty,(id,f,r,s,retry)))
            .into_group_map();

        self.insert_scenarios(local).await;
    }

    /// Inserts the provided retried [`Scenario`] into this [`Features`]
    /// storage.
    ///
    /// [`Scenario`]: gherkin::Scenario
    async fn insert_retried_scenario(
        &self,
        feature: Source<gherkin::Feature>,
        rule: Option<Source<gherkin::Rule>>,
        scenario: Source<gherkin::Scenario>,
        scenario_ty: ScenarioType,
        retries: Option<RetryOptions>,
    ) {
        self.insert_scenarios(
            iter::once((
                scenario_ty,
                vec![(ScenarioId::new(), feature, rule, scenario, retries)],
            ))
            .collect(),
        )
        .await;
    }

    /// Inserts the provided [`Scenario`]s into this [`Features`] storage.
    ///
    /// [`Scenario`]: gherkin::Scenario
    async fn insert_scenarios(&self, scenarios: InsertedScenarios) {
        let now = Instant::now();

        let mut with_retries = HashMap::<_, Vec<_>>::new();
        let mut without_retries: Scenarios = HashMap::new();
        #[expect(clippy::iter_over_hash_type, reason = "order doesn't matter")]
        for (which, values) in scenarios {
            for (id, f, r, s, ret) in values {
                match ret {
                    ret @ (None
                    | Some(RetryOptions {
                        retries: Retries { current: 0, .. },
                        ..
                    })) => {
                        // `Retries::current` is `0`, so this `Scenario` run is
                        // initial, and we don't need to wait for retry delay.
                        let ret = ret.map(RetryOptions::without_deadline);
                        without_retries
                            .entry(which)
                            .or_default()
                            .push((id, f, r, s, ret));
                    }
                    Some(ret) => {
                        let ret = ret.with_deadline(now);
                        with_retries
                            .entry(which)
                            .or_default()
                            .push((id, f, r, s, ret));
                    }
                }
            }
        }

        let mut storage = self.scenarios.lock().await;

        #[expect(clippy::iter_over_hash_type, reason = "order doesn't matter")]
        for (which, values) in with_retries {
            let ty_storage = storage.entry(which).or_default();
            for (id, f, r, s, ret) in values {
                ty_storage.insert(0, (id, f, r, s, Some(ret)));
            }
        }

        if without_retries.contains_key(&ScenarioType::Serial) {
            // If there are Serial Scenarios we insert all Serial and Concurrent
            // Scenarios in front.
            // This is done to execute them closely to one another, so the
            // output wouldn't hang on executing other Concurrent Scenarios.
            #[expect(
                clippy::iter_over_hash_type,
                reason = "order doesn't matter"
            )]
            for (which, mut values) in without_retries {
                let old = mem::take(storage.entry(which).or_default());
                values.extend(old);
                storage.entry(which).or_default().extend(values);
            }
        } else {
            // If there are no Serial Scenarios, we just extend already existing
            // Concurrent Scenarios.
            #[expect(
                clippy::iter_over_hash_type,
                reason = "order doesn't matter"
            )]
            for (which, values) in without_retries {
                storage.entry(which).or_default().extend(values);
            }
        }
    }

    /// Returns [`Scenario`]s which are ready to run and the minimal deadline of
    /// all retried [`Scenario`]s.
    ///
    /// [`Scenario`]: gherkin::Scenario
    #[expect(clippy::significant_drop_tightening, reason = "false positive")]
    async fn get(
        &self,
        max_concurrent_scenarios: Option<usize>,
    ) -> (
        Vec<(
            ScenarioId,
            Source<gherkin::Feature>,
            Option<Source<gherkin::Rule>>,
            Source<gherkin::Scenario>,
            ScenarioType,
            Option<RetryOptions>,
        )>,
        Option<Duration>,
    ) {
        use RetryOptionsWithDeadline as WithDeadline;
        use ScenarioType::{Concurrent, Serial};

        if max_concurrent_scenarios == Some(0) {
            return (Vec::new(), None);
        }

        let mut min_dur = None;
        let mut drain =
            |storage: &mut Vec<(_, _, _, _, Option<WithDeadline>)>,
             ty,
             count: Option<usize>| {
                let mut i = 0;
                let drained = storage
                    .extract_if(.., |(_, _, _, _, ret)| {
                        // Because of retries involved, we cannot just specify
                        // `..count` range to `.extract_if()`.
                        if count.as_ref().is_some_and(|c| i >= *c) {
                            return false;
                        }

                        ret.as_ref()
                            .and_then(WithDeadline::left_until_retry)
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
                    .map(|(id, f, r, s, ret)| {
                        (id, f, r, s, ty, ret.map(Into::into))
                    })
                    .collect::<Vec<_>>();
                (!drained.is_empty()).then_some(drained)
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
    /// `fail_fast` argument indicates whether not yet executed scenarios should
    /// be omitted.
    ///
    /// [`Feature`]: gherkin::Feature
    async fn is_finished(&self, fail_fast: bool) -> bool {
        self.finished.load(Ordering::SeqCst)
            && (fail_fast
                || self.scenarios.lock().await.values().all(Vec::is_empty))
    }
}

/// Coerces the given `value` into a type-erased [`Info`].
fn coerce_into_info<T: Any + Send + 'static>(val: T) -> Info {
    Arc::new(val)
}

/// Classification-only slots never invoke a body, hooks or constructor.
#[derive(Clone, Copy, Eq, PartialEq)]
enum StepMode {
    Execute,
    LookupOnly,
    ExplicitSkip,
}

/// World-free events/failure descriptors queued behind a deferred terminal.
enum DeferredEvent<W> {
    Native(Event<event::Cucumber<W>>),
    Failure(ExecutionFailure<W>),
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
    StepSkipped { world: Option<World>, outcome: Option<Outcome> },

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
        step: Source<gherkin::Step>,

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
    fn status(&self) -> OutcomeStatus {
        match self {
            Self::BeforeHookPanicked { meta, .. } => {
                meta.outcome.map_or(OutcomeStatus::Failed, |o| o.status)
            }
            Self::StepSkipped { outcome, .. } => {
                outcome.map_or(OutcomeStatus::Undefined, |o| o.status)
            }
            Self::StepPanicked { meta, err, .. } => meta.outcome.map_or(
                if matches!(err, event::StepError::AmbiguousMatch(_)) {
                    OutcomeStatus::Ambiguous
                } else {
                    OutcomeStatus::Failed
                },
                |o| o.status,
            ),
        }
    }

    /// Takes the [`World`] leaving a [`None`] in its place.
    const fn take_world(&mut self) -> Option<W> {
        match self {
            Self::BeforeHookPanicked { world, .. }
            | Self::StepSkipped { world, .. }
            | Self::StepPanicked { world, .. } => world.take(),
        }
    }

    /// Creates an [`event::ScenarioFinished`] from this [`ExecutionFailure`].
    fn get_scenario_finished_event(&self) -> event::ScenarioFinished {
        use event::ScenarioFinished::{
            BeforeHookFailed, StepFailed, StepSkipped,
        };

        match self {
            Self::BeforeHookPanicked { panic_info, .. } => {
                BeforeHookFailed(Arc::clone(panic_info))
            }
            Self::StepSkipped { .. } => StepSkipped,
            Self::StepPanicked { captures, loc, err, .. } => {
                StepFailed(captures.clone(), *loc, err.clone())
            }
        }
    }
}

#[cfg(test)]
mod retry_options {
    use gherkin::GherkinEnv;
    use humantime::parse_duration;

    use super::{Cli, Duration, Retries, RetryOptions};

    mod scenario_tags {
        use super::*;

        // language=Gherkin
        const FEATURE: &str = r"
Feature: only scenarios
  Scenario: no tags
    Given a step

  @retry
  Scenario: tag
    Given a step

  @retry(5)
  Scenario: tag with explicit value
    Given a step

  @retry.after(3s)
  Scenario: tag with explicit after
    Given a step

  @retry(5).after(15s)
  Scenario: tag with explicit value and after
    Given a step
";

        #[test]
        fn empty_cli() {
            let cli = Cli {
                concurrency: None,
                fail_fast: false,
                retry: None,
                retry_after: None,
                retry_tag_filter: None,
            };
            let f = gherkin::Feature::parse(FEATURE, GherkinEnv::default())
                .expect("failed to parse feature");

            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[0], &cli),
                None,
            );
            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[1], &cli),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 1 },
                    after: None,
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[2], &cli),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 5 },
                    after: None,
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[3], &cli),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 1 },
                    after: Some(Duration::from_secs(3)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[4], &cli),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 5 },
                    after: Some(Duration::from_secs(15)),
                }),
            );
        }

        #[test]
        fn cli_retries() {
            let cli = Cli {
                concurrency: None,
                fail_fast: false,
                retry: Some(7),
                retry_after: None,
                retry_tag_filter: None,
            };
            let f = gherkin::Feature::parse(FEATURE, GherkinEnv::default())
                .expect("failed to parse feature");

            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[0], &cli),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 7 },
                    after: None,
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[1], &cli),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 7 },
                    after: None,
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[2], &cli),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 5 },
                    after: None,
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[3], &cli),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 7 },
                    after: Some(Duration::from_secs(3)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[4], &cli),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 5 },
                    after: Some(Duration::from_secs(15)),
                }),
            );
        }

        #[test]
        fn cli_retry_after() {
            let cli = Cli {
                concurrency: None,
                fail_fast: false,
                retry: Some(7),
                retry_after: Some(parse_duration("5s").unwrap()),
                retry_tag_filter: None,
            };
            let f = gherkin::Feature::parse(FEATURE, GherkinEnv::default())
                .expect("failed to parse feature");

            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[0], &cli),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 7 },
                    after: Some(Duration::from_secs(5)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[1], &cli),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 7 },
                    after: Some(Duration::from_secs(5)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[2], &cli),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 5 },
                    after: Some(Duration::from_secs(5)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[3], &cli),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 7 },
                    after: Some(Duration::from_secs(3)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[4], &cli),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 5 },
                    after: Some(Duration::from_secs(15)),
                }),
            );
        }

        #[test]
        fn cli_retry_filter() {
            let cli = Cli {
                concurrency: None,
                fail_fast: false,
                retry: Some(7),
                retry_after: None,
                retry_tag_filter: Some("@retry".parse().unwrap()),
            };
            let f = gherkin::Feature::parse(FEATURE, GherkinEnv::default())
                .expect("failed to parse feature");

            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[0], &cli),
                None,
            );
            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[1], &cli),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 7 },
                    after: None,
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[2], &cli),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 5 },
                    after: None,
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[3], &cli),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 7 },
                    after: Some(Duration::from_secs(3)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[4], &cli),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 5 },
                    after: Some(Duration::from_secs(15)),
                }),
            );
        }

        #[test]
        fn cli_retry_after_and_filter() {
            let cli = Cli {
                concurrency: None,
                fail_fast: false,
                retry: Some(7),
                retry_after: Some(parse_duration("5s").unwrap()),
                retry_tag_filter: Some("@retry".parse().unwrap()),
            };
            let f = gherkin::Feature::parse(FEATURE, GherkinEnv::default())
                .expect("failed to parse feature");

            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[0], &cli),
                None,
            );
            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[1], &cli),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 7 },
                    after: Some(Duration::from_secs(5)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[2], &cli),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 5 },
                    after: Some(Duration::from_secs(5)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[3], &cli),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 7 },
                    after: Some(Duration::from_secs(3)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[4], &cli),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 5 },
                    after: Some(Duration::from_secs(15)),
                }),
            );
        }
    }

    mod rule_tags {
        use super::*;

        // language=Gherkin
        const FEATURE: &str = r#"
Feature: only scenarios
  Rule: no tags
    Scenario: no tags
      Given a step

    @retry
    Scenario: tag
      Given a step

    @retry(5)
    Scenario: tag with explicit value
      Given a step

    @retry.after(3s)
    Scenario: tag with explicit after
      Given a step

    @retry(5).after(15s)
    Scenario: tag with explicit value and after
      Given a step

  @retry(3).after(5s)
  Rule: retry tag
    Scenario: no tags
      Given a step

    @retry
    Scenario: tag
      Given a step

    @retry(5)
    Scenario: tag with explicit value
      Given a step

    @retry.after(3s)
    Scenario: tag with explicit after
      Given a step

    @retry(5).after(15s)
    Scenario: tag with explicit value and after
      Given a step
"#;

        #[test]
        fn empty_cli() {
            let cli = Cli {
                concurrency: None,
                fail_fast: false,
                retry: None,
                retry_after: None,
                retry_tag_filter: None,
            };
            let f = gherkin::Feature::parse(FEATURE, GherkinEnv::default())
                .expect("failed to parse feature");

            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[0]),
                    &f.rules[0].scenarios[0],
                    &cli
                ),
                None,
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[0]),
                    &f.rules[0].scenarios[1],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 1 },
                    after: None,
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[0]),
                    &f.rules[0].scenarios[2],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 5 },
                    after: None,
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[0]),
                    &f.rules[0].scenarios[3],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 1 },
                    after: Some(Duration::from_secs(3)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[0]),
                    &f.rules[0].scenarios[4],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 5 },
                    after: Some(Duration::from_secs(15)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[1]),
                    &f.rules[1].scenarios[0],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 3 },
                    after: Some(Duration::from_secs(5)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[1]),
                    &f.rules[1].scenarios[1],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 1 },
                    after: None,
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[1]),
                    &f.rules[1].scenarios[2],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 5 },
                    after: None,
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[1]),
                    &f.rules[1].scenarios[3],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 1 },
                    after: Some(Duration::from_secs(3)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[1]),
                    &f.rules[1].scenarios[4],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 5 },
                    after: Some(Duration::from_secs(15)),
                }),
            );
        }

        #[test]
        fn cli_retry_after_and_filter() {
            let cli = Cli {
                concurrency: None,
                fail_fast: false,
                retry: Some(7),
                retry_after: Some(parse_duration("5s").unwrap()),
                retry_tag_filter: Some("@retry".parse().unwrap()),
            };
            let f = gherkin::Feature::parse(FEATURE, GherkinEnv::default())
                .expect("failed to parse feature");

            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[0]),
                    &f.rules[0].scenarios[0],
                    &cli
                ),
                None,
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[0]),
                    &f.rules[0].scenarios[1],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 7 },
                    after: Some(Duration::from_secs(5)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[0]),
                    &f.rules[0].scenarios[2],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 5 },
                    after: Some(Duration::from_secs(5)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[0]),
                    &f.rules[0].scenarios[3],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 7 },
                    after: Some(Duration::from_secs(3)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[0]),
                    &f.rules[0].scenarios[4],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 5 },
                    after: Some(Duration::from_secs(15)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[1]),
                    &f.rules[1].scenarios[0],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 3 },
                    after: Some(Duration::from_secs(5)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[1]),
                    &f.rules[1].scenarios[1],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 7 },
                    after: Some(Duration::from_secs(5)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[1]),
                    &f.rules[1].scenarios[2],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 5 },
                    after: Some(Duration::from_secs(5)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[1]),
                    &f.rules[1].scenarios[3],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 7 },
                    after: Some(Duration::from_secs(3)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[1]),
                    &f.rules[1].scenarios[4],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 5 },
                    after: Some(Duration::from_secs(15)),
                }),
            );
        }
    }

    mod feature_tags {
        use super::*;

        // language=Gherkin
        const FEATURE: &str = r"
@retry(8)
Feature: only scenarios
  Scenario: no tags
    Given a step

  @retry
  Scenario: tag
    Given a step

  @retry(5)
  Scenario: tag with explicit value
    Given a step

  @retry.after(3s)
  Scenario: tag with explicit after
    Given a step

  @retry(5).after(15s)
  Scenario: tag with explicit value and after
    Given a step

  Rule: no tags
    Scenario: no tags
      Given a step

    @retry
    Scenario: tag
      Given a step

    @retry(5)
    Scenario: tag with explicit value
      Given a step

    @retry.after(3s)
    Scenario: tag with explicit after
      Given a step

    @retry(5).after(15s)
    Scenario: tag with explicit value and after
      Given a step

  @retry(3).after(5s)
  Rule: retry tag
    Scenario: no tags
      Given a step

    @retry
    Scenario: tag
      Given a step

    @retry(5)
    Scenario: tag with explicit value
      Given a step

    @retry.after(3s)
    Scenario: tag with explicit after
      Given a step

    @retry(5).after(15s)
    Scenario: tag with explicit value and after
      Given a step
";

        #[test]
        fn empty_cli() {
            let cli = Cli {
                concurrency: None,
                fail_fast: false,
                retry: None,
                retry_after: None,
                retry_tag_filter: None,
            };
            let f = gherkin::Feature::parse(FEATURE, GherkinEnv::default())
                .unwrap_or_else(|e| panic!("failed to parse feature: {e}"));

            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[0], &cli),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 8 },
                    after: None,
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[1], &cli),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 1 },
                    after: None,
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[2], &cli),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 5 },
                    after: None,
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[3], &cli),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 1 },
                    after: Some(Duration::from_secs(3)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[4], &cli),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 5 },
                    after: Some(Duration::from_secs(15)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[0]),
                    &f.rules[0].scenarios[0],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 8 },
                    after: None,
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[0]),
                    &f.rules[0].scenarios[1],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 1 },
                    after: None,
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[0]),
                    &f.rules[0].scenarios[2],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 5 },
                    after: None,
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[0]),
                    &f.rules[0].scenarios[3],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 1 },
                    after: Some(Duration::from_secs(3)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[0]),
                    &f.rules[0].scenarios[4],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 5 },
                    after: Some(Duration::from_secs(15)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[1]),
                    &f.rules[1].scenarios[0],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 3 },
                    after: Some(Duration::from_secs(5)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[1]),
                    &f.rules[1].scenarios[1],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 1 },
                    after: None,
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[1]),
                    &f.rules[1].scenarios[2],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 5 },
                    after: None,
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[1]),
                    &f.rules[1].scenarios[3],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 1 },
                    after: Some(Duration::from_secs(3)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[1]),
                    &f.rules[1].scenarios[4],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 5 },
                    after: Some(Duration::from_secs(15)),
                }),
            );
        }

        #[test]
        fn cli_retry_after_and_filter() {
            let cli = Cli {
                concurrency: None,
                fail_fast: false,
                retry: Some(7),
                retry_after: Some(parse_duration("5s").unwrap()),
                retry_tag_filter: Some("@retry".parse().unwrap()),
            };
            let f = gherkin::Feature::parse(FEATURE, GherkinEnv::default())
                .expect("failed to parse feature");

            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[0], &cli),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 8 },
                    after: Some(Duration::from_secs(5)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[1], &cli),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 7 },
                    after: Some(Duration::from_secs(5)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[2], &cli),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 5 },
                    after: Some(Duration::from_secs(5)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[3], &cli),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 7 },
                    after: Some(Duration::from_secs(3)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(&f, None, &f.scenarios[4], &cli),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 5 },
                    after: Some(Duration::from_secs(15)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[0]),
                    &f.rules[0].scenarios[0],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 8 },
                    after: Some(Duration::from_secs(5)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[0]),
                    &f.rules[0].scenarios[1],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 7 },
                    after: Some(Duration::from_secs(5)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[0]),
                    &f.rules[0].scenarios[2],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 5 },
                    after: Some(Duration::from_secs(5)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[0]),
                    &f.rules[0].scenarios[3],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 7 },
                    after: Some(Duration::from_secs(3)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[0]),
                    &f.rules[0].scenarios[4],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 5 },
                    after: Some(Duration::from_secs(15)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[1]),
                    &f.rules[1].scenarios[0],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 3 },
                    after: Some(Duration::from_secs(5)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[1]),
                    &f.rules[1].scenarios[1],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 7 },
                    after: Some(Duration::from_secs(5)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[1]),
                    &f.rules[1].scenarios[2],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 5 },
                    after: Some(Duration::from_secs(5)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[1]),
                    &f.rules[1].scenarios[3],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 7 },
                    after: Some(Duration::from_secs(3)),
                }),
            );
            assert_eq!(
                RetryOptions::parse_from_tags(
                    &f,
                    Some(&f.rules[1]),
                    &f.rules[1].scenarios[4],
                    &cli
                ),
                Some(RetryOptions {
                    retries: Retries { current: 0, left: 5 },
                    after: Some(Duration::from_secs(15)),
                }),
            );
        }
    }
}
