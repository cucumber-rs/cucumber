//! Basic runner struct and its core implementation methods.

use std::{
    mem,
    sync::Arc,
    time::Duration,
};

#[cfg(feature = "tracing")]
use crossbeam_utils::atomic::AtomicCell;
use derive_more::with_trait::Debug;
use futures::future::LocalBoxFuture;
use gherkin::tagexpr::TagOperation;
use regex::Regex;

#[cfg(feature = "tracing")]
use crate::tracing::Collector as TracingCollector;
use crate::{
    Step, step,
    event,
};

use super::cli_and_types::{
    Cli, ScenarioType, RetryOptions, RetryOptionsFn, WhichScenarioFn,
    BeforeHookFn, AfterHookFn,
};

/// Default [`Runner`] implementation which follows [_order guarantees_][1] from
/// the [`Runner`] trait docs.
///
/// Executes [`Scenario`]s concurrently based on the custom function, which
/// returns [`ScenarioType`]. Also, can limit maximum number of concurrent
/// [`Scenario`]s.
///
/// [1]: crate::Runner#order-guarantees
/// [`Runner`]: crate::Runner
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
    pub(super) max_concurrent_scenarios: Option<usize>,

    /// Optional number of retries of failed [`Scenario`]s.
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub(super) retries: Option<usize>,

    /// Optional [`Duration`] between retries of failed [`Scenario`]s.
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub(super) retry_after: Option<Duration>,

    /// Optional [`TagOperation`] filter for retries of failed [`Scenario`]s.
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub(super) retry_filter: Option<TagOperation>,

    /// [`Collection`] of functions to match [`Step`]s.
    ///
    /// [`Collection`]: step::Collection
    pub(super) steps: step::Collection<World>,

    /// Function determining whether a [`Scenario`] is [`Concurrent`] or
    /// a [`Serial`] one.
    ///
    /// [`Concurrent`]: ScenarioType::Concurrent
    /// [`Serial`]: ScenarioType::Serial
    /// [`Scenario`]: gherkin::Scenario
    #[debug(ignore)]
    pub(super) which_scenario: F,

    /// Function determining [`Scenario`]'s [`RetryOptions`].
    ///
    /// [`Scenario`]: gherkin::Scenario
    #[debug(ignore)]
    pub(super) retry_options: RetryOptionsFn,

    /// Function, executed on each [`Scenario`] before running all [`Step`]s,
    /// including [`Background`] ones.
    ///
    /// [`Background`]: gherkin::Background
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
    #[debug(ignore)]
    pub(super) before_hook: Option<Before>,

    /// Function, executed on each [`Scenario`] after running all [`Step`]s.
    ///
    /// [`Background`]: gherkin::Background
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
    #[debug(ignore)]
    pub(super) after_hook: Option<After>,

    /// Indicates whether execution should be stopped after the first failure.
    pub(super) fail_fast: bool,

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
            #[cfg(feature = "tracing")]
            logs_collector: Arc::clone(&self.logs_collector),
        }
    }
}

impl<World> Default for Basic<World> {
    fn default() -> Self {
        let which_scenario: WhichScenarioFn = |feature, rule, scenario| {
            use crate::tag::Ext as _;
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
            #[cfg(feature = "tracing")]
            logs_collector: Arc::new(AtomicCell::new(Box::new(None))),
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
            #[cfg(feature = "tracing")]
            logs_collector,
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
            #[cfg(feature = "tracing")]
            logs_collector,
        }
    }

    /// Sets the given [`Collection`] of [`Step`]s to this [`Runner`].
    ///
    /// [`Collection`]: step::Collection
    /// [`Runner`]: crate::Runner
    #[must_use]
    pub fn steps(mut self, steps: step::Collection<World>) -> Self {
        self.steps = steps;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::common::TestWorld;

    // Using common TestWorld from test_utils

    #[test]
    fn test_basic_default() {
        let basic = Basic::<TestWorld>::default();
        assert_eq!(basic.max_concurrent_scenarios, Some(64));
        assert_eq!(basic.retries, None);
        assert_eq!(basic.retry_after, None);
        assert!(basic.retry_filter.is_none());
        assert!(!basic.fail_fast);
    }

    #[test]
    fn test_basic_builder_methods() {
        let basic = Basic::<TestWorld>::default()
            .max_concurrent_scenarios(10)
            .retries(3)
            .retry_after(Duration::from_secs(1))
            .fail_fast();

        assert_eq!(basic.max_concurrent_scenarios, Some(10));
        assert_eq!(basic.retries, Some(3));
        assert_eq!(basic.retry_after, Some(Duration::from_secs(1)));
        assert!(basic.fail_fast);
    }

    #[test]
    fn test_basic_clone() {
        let basic = Basic::<TestWorld>::default()
            .max_concurrent_scenarios(5)
            .retries(2);

        let cloned = basic.clone();
        assert_eq!(cloned.max_concurrent_scenarios, Some(5));
        assert_eq!(cloned.retries, Some(2));
    }
}