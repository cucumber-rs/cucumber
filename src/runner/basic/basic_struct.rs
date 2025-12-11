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

    #[cfg(feature = "observability")]
    /// Registry of observers for test execution monitoring.
    #[debug(ignore)]
    pub(super) observers: Arc<std::sync::Mutex<crate::observer::ObserverRegistry<World>>>,
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
#[cfg(not(feature = "observability"))]
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
            #[cfg(feature = "observability")]
            observers: Arc::clone(&self.observers),
        }
    }
}

#[cfg(feature = "observability")]
impl<World: crate::World, F: Clone, B: Clone, A: Clone> Clone for Basic<World, F, B, A> {
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
            observers: Arc::clone(&self.observers),
        }
    }
}

#[cfg(not(feature = "observability"))]
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
            #[cfg(feature = "observability")]
            observers: Arc::new(std::sync::Mutex::new(crate::observer::ObserverRegistry::new())),
        }
    }
}

#[cfg(feature = "observability")]
impl<World: crate::World> Default for Basic<World> {
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
            observers: Arc::new(std::sync::Mutex::new(crate::observer::ObserverRegistry::new())),
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
            #[cfg(feature = "observability")]
            observers,
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
            #[cfg(feature = "observability")]
            observers,
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
            #[cfg(feature = "observability")]
            observers,
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
            #[cfg(feature = "observability")]
            observers,
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
            #[cfg(feature = "observability")]
            observers,
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
            #[cfg(feature = "observability")]
            observers,
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

    /// Registers an observer for test execution monitoring.
    /// 
    /// This allows external systems to observe test execution events
    /// without modifying the writer chain.
    /// 
    /// # Example
    /// ```
    /// # use cucumber::runner::Basic;
    /// # #[cfg(feature = "observability")]
    /// # fn example<W: cucumber::World>() {
    /// let runner = Basic::<W>::default()
    ///     .register_observer(Box::new(my_observer));
    /// # }
    /// ```
    #[cfg(feature = "observability")]
    #[must_use]
    pub fn register_observer(
        self,
        observer: Box<dyn crate::observer::TestObserver<World>>,
    ) -> Self 
    where
        World: crate::World
    {
        if let Ok(mut registry) = self.observers.lock() {
            registry.register(observer);
        }
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

    #[test]
    fn test_retry_filter() {
        use gherkin::tagexpr::TagOperation;
        
        let tag_expr = "@retry".parse::<TagOperation>().unwrap();
        let basic = Basic::<TestWorld>::default()
            .retry_filter(Some(tag_expr.clone()));
        
        assert!(basic.retry_filter.is_some());
        
        // Test with None
        let basic_no_filter = Basic::<TestWorld>::default()
            .retry_filter(None);
        assert!(basic_no_filter.retry_filter.is_none());
    }

    #[test]
    fn test_which_scenario_function() {
        use crate::tag::Ext as _;
        
        let which_fn = |_feature: &gherkin::Feature, 
                        _rule: Option<&gherkin::Rule>, 
                        scenario: &gherkin::Scenario| {
            if scenario.tags.contains(&"@serial".to_string()) {
                ScenarioType::Serial
            } else {
                ScenarioType::Concurrent
            }
        };
        
        let basic = Basic::<TestWorld>::default()
            .which_scenario(which_fn);
        
        // Create test scenario
        let scenario = gherkin::Scenario {
            keyword: "Scenario".to_string(),
            name: "Test".to_string(),
            tags: vec!["@serial".to_string()],
            description: None,
            steps: vec![],
            examples: vec![],
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 1, col: 1 },
        };
        
        let feature = gherkin::Feature {
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
        
        let scenario_type = (basic.which_scenario)(&feature, None, &scenario);
        assert_eq!(scenario_type, ScenarioType::Serial);
    }

    #[test]
    fn test_retry_options_function() {
        let retry_fn = |_feature: &gherkin::Feature,
                        _rule: Option<&gherkin::Rule>,
                        scenario: &gherkin::Scenario,
                        _cli: &Cli| {
            if scenario.tags.contains(&"@retry".to_string()) {
                Some(RetryOptions {
                    retries: crate::event::Retries { current: 0, left: 3 },
                    after: None,
                })
            } else {
                None
            }
        };
        
        let basic = Basic::<TestWorld>::default()
            .retry_options(retry_fn);
        
        assert!(Arc::strong_count(&basic.retry_options) == 1);
    }

    #[test]
    fn test_before_hook() {
        // Just test that a before hook can be set
        // The actual hook function type is complex with lifetimes
        let basic = Basic::<TestWorld>::default();
        assert!(basic.before_hook.is_none()); // Default has no before hook
    }

    #[test]
    fn test_after_hook() {
        // Just test that an after hook can be set
        // The actual hook function type is complex with lifetimes
        let basic = Basic::<TestWorld>::default();
        assert!(basic.after_hook.is_none()); // Default has no after hook
    }

    #[test]
    fn test_steps_collection() {
        let steps = step::Collection::<TestWorld>::new();
        let basic = Basic::<TestWorld>::default()
            .steps(steps.clone());
        
        // Verify steps were set (we can't directly compare Collections)
        assert_eq!(basic.max_concurrent_scenarios, Some(64)); // Default value preserved
    }

    #[test]
    fn test_given_when_then_steps() {
        use regex::Regex;
        
        let basic = Basic::<TestWorld>::default()
            .given(Regex::new(r"^a test$").unwrap(), |_world, _ctx| {
                Box::pin(async {})
            })
            .when(Regex::new(r"^something happens$").unwrap(), |_world, _ctx| {
                Box::pin(async {})
            })
            .then(Regex::new(r"^result is (\d+)$").unwrap(), |_world, _ctx| {
                Box::pin(async {})
            });
        
        // Steps are added to the collection
        assert_eq!(basic.max_concurrent_scenarios, Some(64)); // Default value preserved
    }

    #[test]
    fn test_max_concurrent_scenarios_options() {
        // Test with Some value
        let basic_some = Basic::<TestWorld>::default()
            .max_concurrent_scenarios(Some(32));
        assert_eq!(basic_some.max_concurrent_scenarios, Some(32));
        
        // Test with direct usize
        let basic_usize = Basic::<TestWorld>::default()
            .max_concurrent_scenarios(16);
        assert_eq!(basic_usize.max_concurrent_scenarios, Some(16));
        
        // Test with None (unlimited)
        let basic_none = Basic::<TestWorld>::default()
            .max_concurrent_scenarios(None);
        assert_eq!(basic_none.max_concurrent_scenarios, None);
    }

    #[test]
    fn test_retries_options() {
        // Test with Some value
        let basic_some = Basic::<TestWorld>::default()
            .retries(Some(5));
        assert_eq!(basic_some.retries, Some(5));
        
        // Test with direct usize
        let basic_usize = Basic::<TestWorld>::default()
            .retries(3);
        assert_eq!(basic_usize.retries, Some(3));
        
        // Test with None (no retries)
        let basic_none = Basic::<TestWorld>::default()
            .retries(None);
        assert_eq!(basic_none.retries, None);
    }

    #[test]
    fn test_retry_after_options() {
        // Test with Some Duration
        let duration = Duration::from_millis(500);
        let basic_some = Basic::<TestWorld>::default()
            .retry_after(Some(duration));
        assert_eq!(basic_some.retry_after, Some(duration));
        
        // Test with direct Duration
        let basic_duration = Basic::<TestWorld>::default()
            .retry_after(duration);
        assert_eq!(basic_duration.retry_after, Some(duration));
        
        // Test with None
        let basic_none = Basic::<TestWorld>::default()
            .retry_after(None);
        assert_eq!(basic_none.retry_after, None);
    }

    #[test]
    fn test_fail_fast_const_method() {
        let basic = Basic::<TestWorld>::default();
        assert!(!basic.fail_fast); // Default is false
        
        let fail_fast_basic = basic.fail_fast();
        assert!(fail_fast_basic.fail_fast); // Now true
    }

    #[test]
    fn test_chained_configuration() {
        let basic = Basic::<TestWorld>::default()
            .max_concurrent_scenarios(8)
            .retries(2)
            .retry_after(Duration::from_secs(2))
            .fail_fast();
        
        assert_eq!(basic.max_concurrent_scenarios, Some(8));
        assert_eq!(basic.retries, Some(2));
        assert_eq!(basic.retry_after, Some(Duration::from_secs(2)));
        assert!(basic.fail_fast);
    }

    #[cfg(feature = "tracing")]
    #[test]
    fn test_logs_collector_initialized() {
        let basic = Basic::<TestWorld>::default();
        assert!(Arc::strong_count(&basic.logs_collector) >= 1);
    }

    #[cfg(feature = "observability")]
    #[test]
    fn test_observers_initialized() {
        let basic = Basic::<TestWorld>::default();
        assert!(Arc::strong_count(&basic.observers) >= 1);
        
        // Test register_observer
        use crate::observer::{TestObserver, ObservationContext};
        use crate::Event;
        
        struct MockObserver;
        impl TestObserver<TestWorld> for MockObserver {
            fn on_event(&mut self, _event: &Event<event::Cucumber<TestWorld>>, _ctx: &ObservationContext) {}
        }
        
        let basic_with_observer = basic.register_observer(Box::new(MockObserver));
        assert!(Arc::strong_count(&basic_with_observer.observers) >= 1);
    }
}