//! Core summarize writer implementation.

use std::collections::HashMap;

use derive_more::with_trait::Deref;

use crate::{
    Event, World, Writer,
    cli::Colored,
    event::{self, Retries, Source},
    parser,
    writer::{self, out::Styles},
};

use super::{
    stats::Stats,
    state::State,
    tracking::{HandledScenarios, Indicator, ScenarioTracker},
};

/// Wrapper for a [`Writer`] for outputting an execution summary (number of
/// executed features, scenarios, steps and parsing errors).
///
/// Underlying [`Writer`] has to be [`Summarizable`] and [`ArbitraryWriter`]
/// with `Value` accepting [`String`]. If your underlying [`ArbitraryWriter`]
/// operates with something like JSON (or any other type), you should implement
/// a [`Writer`] on [`Summarize`] by yourself, to provide the required summary
/// format.
///
/// [`ArbitraryWriter`]: writer::Arbitrary
#[derive(Clone, Debug, Deref)]
pub struct Summarize<Writer> {
    /// Original [`Writer`] to summarize output of.
    #[deref]
    writer: Writer,

    /// Number of started [`Feature`]s.
    ///
    /// [`Feature`]: gherkin::Feature
    pub(super) features: usize,

    /// Number of started [`Rule`]s.
    ///
    /// [`Rule`]: gherkin::Rule
    pub(super) rules: usize,

    /// [`Scenario`]s [`Stats`].
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub(super) scenarios: Stats,

    /// [`Step`]s [`Stats`].
    ///
    /// [`Step`]: gherkin::Step
    pub(super) steps: Stats,

    /// Number of [`Parser`] errors.
    ///
    /// [`Parser`]: crate::Parser
    pub(super) parsing_errors: usize,

    /// Number of failed [`Scenario`] hooks.
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub(super) failed_hooks: usize,

    /// Current [`State`] of this [`Writer`].
    state: State,

    /// Handled [`Scenario`]s to collect [`Stats`].
    ///
    /// [`Scenario`]: gherkin::Scenario
    handled_scenarios: HandledScenarios,
}

impl<W, Wr> Writer<W> for Summarize<Wr>
where
    W: World,
    Wr: writer::Arbitrary<W, String> + Summarizable,
    Wr::Cli: Colored,
{
    type Cli = Wr::Cli;

    async fn handle_event(
        &mut self,
        event: parser::Result<Event<event::Cucumber<W>>>,
        cli: &Self::Cli,
    ) {
        use event::{Cucumber, Feature, Rule};

        // Once `Cucumber::Finished` is emitted, we just pass events through,
        // without collecting `Stats`.
        // This is done to avoid miscalculations if this `Writer` happens to be
        // wrapped by a `writer::Repeat` or similar.
        if matches!(self.state, State::InProgress) {
            match event.as_deref() {
                Err(_) => self.parsing_errors += 1,
                Ok(Cucumber::Feature(feat, ev)) => match ev {
                    Feature::Started => self.features += 1,
                    Feature::Rule(_, Rule::Started) => {
                        self.rules += 1;
                    }
                    Feature::Rule(rule, Rule::Scenario(sc, ev)) => {
                        self.handle_scenario(
                            feat.clone(),
                            Some(rule.clone()),
                            sc.clone(),
                            ev,
                        );
                    }
                    Feature::Scenario(sc, ev) => {
                        self.handle_scenario(
                            feat.clone(),
                            None,
                            sc.clone(),
                            ev,
                        );
                    }
                    Feature::Finished | Feature::Rule(..) => {}
                },
                Ok(Cucumber::Finished) => {
                    self.state = State::FinishedButNotOutput;
                }
                Ok(Cucumber::Started | Cucumber::ParsingFinished { .. }) => {}
            }
        }

        self.writer.handle_event(event, cli).await;

        if matches!(self.state, State::FinishedButNotOutput) {
            self.state = State::FinishedAndOutput;

            let mut styles = Styles::new();
            styles.apply_coloring(cli.coloring());
            self.writer.write(styles.summary(self)).await;
        }
    }
}

#[warn(clippy::missing_trait_methods)]
impl<W, Wr, Val> writer::Arbitrary<W, Val> for Summarize<Wr>
where
    W: World,
    Self: Writer<W>,
    Wr: writer::Arbitrary<W, Val>,
{
    async fn write(&mut self, val: Val) {
        self.writer.write(val).await;
    }
}

impl<W, Wr> writer::Stats<W> for Summarize<Wr>
where
    W: World,
    Self: Writer<W>,
{
    fn passed_steps(&self) -> usize {
        self.steps.passed
    }

    fn skipped_steps(&self) -> usize {
        self.steps.skipped
    }

    fn failed_steps(&self) -> usize {
        self.steps.failed
    }

    fn retried_steps(&self) -> usize {
        self.steps.retried
    }

    fn parsing_errors(&self) -> usize {
        self.parsing_errors
    }

    fn hook_errors(&self) -> usize {
        self.failed_hooks
    }
}

#[warn(clippy::missing_trait_methods)]
impl<Wr: writer::Normalized> writer::Normalized for Summarize<Wr> {}

#[warn(clippy::missing_trait_methods)]
impl<Wr: writer::NonTransforming> writer::NonTransforming for Summarize<Wr> {}

impl<Writer> From<Writer> for Summarize<Writer> {
    fn from(writer: Writer) -> Self {
        Self {
            writer,
            features: 0,
            rules: 0,
            scenarios: Stats::new(),
            steps: Stats::new(),
            parsing_errors: 0,
            failed_hooks: 0,
            state: State::InProgress,
            handled_scenarios: HashMap::new(),
        }
    }
}

impl<Writer> Summarize<Writer> {
    /// Wraps the given [`Writer`] into a new [`Summarize`]d one.
    #[must_use]
    pub fn new(writer: Writer) -> Self {
        Self::from(writer)
    }

    /// Returns the original [`Writer`], wrapped by this [`Summarize`]d one.
    #[must_use]
    pub const fn inner_writer(&self) -> &Writer {
        &self.writer
    }

    /// Returns collected [`Scenario`]s [`Stats`] of this [`Summarize`]d
    /// [`Writer`].
    ///
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    pub const fn scenarios_stats(&self) -> &Stats {
        &self.scenarios
    }

    /// Returns collected [`Step`]s [`Stats`] of this [`Summarize`]d [`Writer`].
    ///
    /// [`Step`]: gherkin::Step
    #[must_use]
    pub const fn steps_stats(&self) -> &Stats {
        &self.steps
    }

    /// Returns the number of features processed.
    #[must_use]
    pub const fn features_count(&self) -> usize {
        self.features
    }

    /// Returns the number of rules processed.
    #[must_use]
    pub const fn rules_count(&self) -> usize {
        self.rules
    }

    /// Returns the number of parsing errors encountered.
    #[must_use]
    pub const fn parsing_errors_count(&self) -> usize {
        self.parsing_errors
    }

    /// Returns the number of failed hooks encountered.
    #[must_use]
    pub const fn failed_hooks_count(&self) -> usize {
        self.failed_hooks
    }

    /// Returns the current state of the summarize writer.
    #[must_use]
    pub const fn current_state(&self) -> State {
        self.state
    }

    /// Keeps track of [`Step`]'s [`Stats`].
    ///
    /// [`Step`]: gherkin::Step
    fn handle_step<W>(
        &mut self,
        feature: Source<gherkin::Feature>,
        rule: Option<Source<gherkin::Rule>>,
        scenario: Source<gherkin::Scenario>,
        step: &gherkin::Step,
        ev: &event::Step<W>,
        retries: Option<Retries>,
    ) {
        use event::Step;

        match ev {
            Step::Started => {}
            Step::Passed { .. } => {
                self.steps.increment_passed();
                if scenario.steps.last().filter(|s| *s == step).is_some() {
                    ScenarioTracker::remove_scenario(
                        &mut self.handled_scenarios,
                        feature,
                        rule,
                        scenario,
                    );
                }
            }
            Step::Skipped => {
                self.steps.increment_skipped();
                self.scenarios.increment_skipped();
                ScenarioTracker::update_scenario(
                    &mut self.handled_scenarios,
                    feature,
                    rule,
                    scenario,
                    Indicator::Skipped,
                );
            }
            Step::Failed { error, .. } => {
                if retries
                    .filter(|r| {
                        r.left > 0 && !matches!(error, event::StepError::NotFound)
                    })
                    .is_some()
                {
                    self.steps.increment_retried();

                    let inserted_before = ScenarioTracker::update_scenario(
                        &mut self.handled_scenarios,
                        feature,
                        rule,
                        scenario,
                        Indicator::Retried,
                    );

                    if inserted_before.is_none() {
                        self.scenarios.increment_retried();
                    }
                } else {
                    self.steps.increment_failed();
                    self.scenarios.increment_failed();

                    ScenarioTracker::update_scenario(
                        &mut self.handled_scenarios,
                        feature,
                        rule,
                        scenario,
                        Indicator::Failed,
                    );
                }
            }
        }
    }

    /// Keeps track of [`Scenario`]'s [`Stats`].
    ///
    /// [`Scenario`]: gherkin::Scenario
    fn handle_scenario<W>(
        &mut self,
        feature: Source<gherkin::Feature>,
        rule: Option<Source<gherkin::Rule>>,
        scenario: Source<gherkin::Scenario>,
        ev: &event::RetryableScenario<W>,
    ) {
        use event::{Hook, Scenario};

        let path = (feature, rule, scenario);

        let ret = ev.retries;
        match &ev.event {
            Scenario::Started
            | Scenario::Hook(_, Hook::Passed | Hook::Started)
            | Scenario::Log(_) => {}
            Scenario::Hook(_, Hook::Failed(..)) => {
                // - If Scenario's last Step failed and then After Hook failed
                //   too, we don't need to track second failure;
                // - If Scenario's last Step was skipped and then After Hook
                //   failed, we need to override skipped Scenario with failed;
                // - If Scenario executed no Steps and then Hook failed, we
                //   track Scenario as failed.
                match self.handled_scenarios.get(&path) {
                    Some(Indicator::Failed | Indicator::Retried) => {}
                    Some(Indicator::Skipped) => {
                        self.scenarios.decrement_skipped();
                        self.scenarios.increment_failed();
                    }
                    None => {
                        self.scenarios.increment_failed();
                        ScenarioTracker::update_scenario(
                            &mut self.handled_scenarios,
                            path.0,
                            path.1,
                            path.2,
                            Indicator::Failed,
                        );
                    }
                }
                self.failed_hooks += 1;
            }
            Scenario::Background(st, ev) | Scenario::Step(st, ev) => {
                self.handle_step(path.0, path.1, path.2, st.as_ref(), ev, ret);
            }
            Scenario::Finished => {
                // We don't remove retried `Scenario`s immediately, because we
                // want to deduplicate. For example if some `Scenario` is
                // retried 3 times, we'll see in summary 1 retried `Scenario`
                // and 3 retried `Step`s.
                let is_retried = self
                    .handled_scenarios
                    .get(&path)
                    .is_some_and(|ind| matches!(ind, Indicator::Retried));

                if !is_retried && self.handled_scenarios.remove(&path).is_none() {
                    self.scenarios.increment_passed();
                }
            }
        }
    }
}

/// Marker indicating that a [`Writer`] can be wrapped into a [`Summarize`].
///
/// Not any [`Writer`] can be wrapped into a [`Summarize`], as it may transform
/// events inside and the summary won't reflect outputted events correctly.
///
/// So, this trait ensures that a wrong [`Writer`]s pipeline cannot be build.
///
/// # Example
///
/// ```rust,compile_fail
/// # use cucumber::{writer, World, WriterExt as _};
/// #
/// # #[derive(Debug, Default, World)]
/// # struct MyWorld;
/// #
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// MyWorld::cucumber()
///     .with_writer(
///         // `Writer`s pipeline is constructed in a reversed order.
///         writer::Basic::stdout()
///             .fail_on_skipped() // Fails as `Summarize` will count skipped
///             .summarized()      // steps instead of failed.
///     )
///     .run_and_exit("tests/features/readme")
///     .await;
/// # }
/// ```
///
/// ```rust
/// # use std::panic::AssertUnwindSafe;
/// #
/// # use cucumber::{writer, World, WriterExt as _};
/// # use futures::FutureExt as _;
/// #
/// # #[derive(Debug, Default, World)]
/// # struct MyWorld;
/// #
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// # let fut = async {
/// MyWorld::cucumber()
///     .with_writer(
///         // `Writer`s pipeline is constructed in a reversed order.
///         writer::Basic::stdout() // And, finally, print them.
///             .summarized()       // Only then, count summary for them.
///             .fail_on_skipped(), // First, transform skipped steps to failed.
///     )
///     .run_and_exit("tests/features/readme")
///     .await;
/// # };
/// # let err = AssertUnwindSafe(fut)
/// #         .catch_unwind()
/// #         .await
/// #         .expect_err("should err");
/// # let err = err.downcast_ref::<String>().unwrap();
/// # assert_eq!(err, "1 step failed");
/// # }
/// ```
pub trait Summarizable {}

impl<T: writer::NonTransforming> Summarizable for T {}

/// Alias for [`fn`] used to determine should [`Skipped`] test considered as
/// [`Failed`] or not.
///
/// [`Failed`]: event::Step::Failed
/// [`Skipped`]: event::Step::Skipped
pub type SkipFn =
    fn(&gherkin::Feature, Option<&gherkin::Rule>, &gherkin::Scenario) -> bool;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::common::{EmptyCli, TestWorld};
    use crate::{Writer, parser, Event};

    #[derive(Debug, Clone)]
    struct MockWriter;

    impl<W: World> Writer<W> for MockWriter {
        type Cli = EmptyCli;

        async fn handle_event(
            &mut self,
            _event: parser::Result<Event<crate::event::Cucumber<W>>>,
            _cli: &Self::Cli,
        ) {
            // No-op for testing
        }
    }

    impl<W: World> writer::Arbitrary<W, String> for MockWriter {
        async fn write(&mut self, _val: String) {}
    }

    impl writer::NonTransforming for MockWriter {}

    #[test]
    fn summarize_new() {
        let writer = MockWriter;
        let summarize = Summarize::new(writer);
        
        assert_eq!(summarize.features_count(), 0);
        assert_eq!(summarize.rules_count(), 0);
        assert_eq!(summarize.scenarios_stats().total(), 0);
        assert_eq!(summarize.steps_stats().total(), 0);
        assert_eq!(summarize.parsing_errors_count(), 0);
        assert_eq!(summarize.failed_hooks_count(), 0);
        assert_eq!(summarize.current_state(), State::InProgress);
    }

    #[test]
    fn summarize_from() {
        let writer = MockWriter;
        let summarize = Summarize::from(writer);
        
        assert_eq!(summarize.features_count(), 0);
        assert_eq!(summarize.scenarios_stats(), &Stats::new());
        assert_eq!(summarize.steps_stats(), &Stats::new());
    }

    #[test]
    fn summarize_getters() {
        let writer = MockWriter;
        let mut summarize = Summarize::new(writer);
        
        // Test initial values
        assert_eq!(summarize.scenarios_stats().total(), 0);
        assert_eq!(summarize.steps_stats().total(), 0);
        
        // Manually set some values to test getters
        summarize.features = 5;
        summarize.rules = 3;
        summarize.parsing_errors = 2;
        summarize.failed_hooks = 1;
        
        assert_eq!(summarize.features_count(), 5);
        assert_eq!(summarize.rules_count(), 3);
        assert_eq!(summarize.parsing_errors_count(), 2);
        assert_eq!(summarize.failed_hooks_count(), 1);
    }

    #[test]
    fn stats_trait_implementation() {
        let writer = MockWriter;
        let mut summarize = Summarize::new(writer);
        
        // Set some step statistics
        summarize.steps.passed = 10;
        summarize.steps.skipped = 3;
        summarize.steps.failed = 2;
        summarize.steps.retried = 5;
        summarize.parsing_errors = 1;
        summarize.failed_hooks = 2;
        
        // Test public getter methods
        assert_eq!(summarize.steps_stats().passed, 10);
        assert_eq!(summarize.steps_stats().skipped, 3);
        assert_eq!(summarize.steps_stats().failed, 2);
        assert_eq!(summarize.steps_stats().retried, 5);
        assert_eq!(summarize.parsing_errors_count(), 1);
        assert_eq!(summarize.failed_hooks_count(), 2);
    }

    #[test]
    fn summarizable_trait_is_implemented_for_non_transforming() {
        // This test just verifies the trait implementation compiles
        fn _test_summarizable<T: Summarizable>(_: T) {}
        _test_summarizable(MockWriter);
    }

    #[test]
    fn state_transitions() {
        let writer = MockWriter;
        let mut summarize = Summarize::new(writer);
        
        // Initial state
        assert_eq!(summarize.current_state(), State::InProgress);
        
        // Set state to finished but not output
        summarize.state = State::FinishedButNotOutput;
        assert_eq!(summarize.current_state(), State::FinishedButNotOutput);
        
        // Set state to finished and output
        summarize.state = State::FinishedAndOutput;
        assert_eq!(summarize.current_state(), State::FinishedAndOutput);
    }

    #[test]
    fn inner_writer_access() {
        let writer = MockWriter;
        let summarize = Summarize::new(writer.clone());
        
        // Test that we can access the inner writer
        assert_eq!(format!("{:?}", summarize.inner_writer()), format!("{:?}", &writer));
    }

    #[test]
    fn basic_step_counting() {
        let writer = MockWriter;
        let mut summarize = Summarize::new(writer);
        
        // Directly increment stats to test basic functionality
        summarize.steps.increment_passed();
        summarize.steps.increment_skipped();
        summarize.steps.increment_failed();
        summarize.steps.increment_retried();
        
        assert_eq!(summarize.steps_stats().passed, 1);
        assert_eq!(summarize.steps_stats().skipped, 1);
        assert_eq!(summarize.steps_stats().failed, 1);
        assert_eq!(summarize.steps_stats().retried, 1);
        assert_eq!(summarize.steps_stats().total(), 3); // passed + skipped + failed
    }
}