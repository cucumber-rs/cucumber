//! Runner configuration methods for Cucumber executor.

use std::{time::Duration, marker::PhantomData};

use gherkin::tagexpr::TagOperation;

use crate::{
    Parser, World, Writer, ScenarioType,
    runner::{self, basic::RetryOptions},
};

use super::core::Cucumber;

impl<W, I, P, Wr, F, B, A, Cli>
    Cucumber<W, P, I, runner::Basic<W, F, B, A>, Wr, Cli>
where
    W: World,
    P: Parser<I>,
    Wr: Writer<W>,
    Cli: clap::Args,
    F: Fn(
            &gherkin::Feature,
            Option<&gherkin::Rule>,
            &gherkin::Scenario,
        ) -> ScenarioType
        + 'static,
    B: for<'a> Fn(
            &'a gherkin::Feature,
            Option<&'a gherkin::Rule>,
            &'a gherkin::Scenario,
            &'a mut W,
        ) -> futures::future::LocalBoxFuture<'a, ()>
        + 'static,
    A: for<'a> Fn(
            &'a gherkin::Feature,
            Option<&'a gherkin::Rule>,
            &'a gherkin::Scenario,
            &'a crate::event::ScenarioFinished,
            Option<&'a mut W>,
        ) -> futures::future::LocalBoxFuture<'a, ()>
        + 'static,
{
    /// If `max` is [`Some`] number of concurrently executed [`Scenario`]s will
    /// be limited.
    ///
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    pub fn max_concurrent_scenarios(
        mut self,
        max: impl Into<Option<usize>>,
    ) -> Self {
        self.runner = self.runner.max_concurrent_scenarios(max);
        self
    }

    /// Makes failed [`Scenario`]s being retried the specified number of times.
    ///
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    pub fn retries(mut self, retries: impl Into<Option<usize>>) -> Self {
        self.runner = self.runner.retries(retries);
        self
    }

    /// Makes stop running tests on the first failure.
    ///
    /// __NOTE__: All the already started [`Scenario`]s at the moment of failure
    ///           will be finished.
    ///
    /// __NOTE__: Retried [`Scenario`]s are considered as failed, only in case
    ///           they exhaust all retry attempts and still do fail.
    ///
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    pub fn fail_fast(mut self) -> Self {
        self.runner = self.runner.fail_fast();
        self
    }

    /// Makes failed [`Scenario`]s being retried after the specified
    /// [`Duration`] passes.
    ///
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    pub fn retry_after(mut self, after: impl Into<Option<Duration>>) -> Self {
        self.runner = self.runner.retry_after(after);
        self
    }

    /// Makes failed [`Scenario`]s being retried only if they're matching the
    /// specified `tag_expression`.
    ///
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    pub fn retry_filter(
        mut self,
        tag_expression: impl Into<Option<TagOperation>>,
    ) -> Self {
        self.runner = self.runner.retry_filter(tag_expression);
        self
    }

    /// Function determining whether a [`Scenario`] is [`Concurrent`] or
    /// a [`Serial`] one.
    ///
    /// [`Concurrent`]: ScenarioType::Concurrent
    /// [`Serial`]: ScenarioType::Serial
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    pub fn which_scenario<Which>(
        self,
        func: Which,
    ) -> Cucumber<W, P, I, runner::Basic<W, Which, B, A>, Wr, Cli>
    where
        Which: Fn(
                &gherkin::Feature,
                Option<&gherkin::Rule>,
                &gherkin::Scenario,
            ) -> ScenarioType
            + 'static,
    {
        let Self { parser, runner, writer, cli, .. } = self;
        Cucumber {
            parser,
            runner: runner.which_scenario(func),
            writer,
            cli,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }

    /// Function determining [`Scenario`]'s [`RetryOptions`].
    ///
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    pub fn retry_options<Retry>(mut self, func: Retry) -> Self
    where
        Retry: Fn(
                &gherkin::Feature,
                Option<&gherkin::Rule>,
                &gherkin::Scenario,
                &runner::basic::Cli,
            ) -> Option<RetryOptions>
            + 'static,
    {
        self.runner = self.runner.retry_options(func);
        self
    }
}