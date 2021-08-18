// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Top-level [Cucumber] executor.
//!
//! [Cucumber]: https://cucumber.io

use std::{
    fmt::{Debug, Formatter},
    marker::PhantomData,
    mem,
    path::Path,
};

use futures::StreamExt as _;
use regex::Regex;

use crate::{
    parser,
    runner::{self, basic::ScenarioType},
    step,
    writer::{self, Ext as _},
    Parser, Runner, Step, World, Writer,
};

/// Top-level [Cucumber] executor.
///
/// [Cucumber]: https://cucumber.io
pub struct Cucumber<W, P, I, R, Wr> {
    parser: P,
    runner: R,
    writer: Wr,
    _world: PhantomData<W>,
    _parser_input: PhantomData<I>,
}

impl<W> Cucumber<W, (), (), (), ()> {
    /// Creates an empty [`Cucumber`].
    #[must_use]
    pub fn custom() -> Self {
        Self {
            parser: (),
            runner: (),
            writer: (),
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }
}

impl<W, P, I, R, Wr> Cucumber<W, P, I, R, Wr> {
    /// Replaces [`Parser`].
    #[must_use]
    pub fn with_parser<NewP, NewI>(
        self,
        parser: NewP,
    ) -> Cucumber<W, NewP, NewI, R, Wr>
    where
        NewP: Parser<NewI>,
    {
        let Self { runner, writer, .. } = self;
        Cucumber {
            parser,
            runner,
            writer,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }

    /// Replaces [`Runner`].
    #[must_use]
    pub fn with_runner<NewR>(self, runner: NewR) -> Cucumber<W, P, I, NewR, Wr>
    where
        NewR: Runner<W>,
    {
        let Self { parser, writer, .. } = self;
        Cucumber {
            parser,
            runner,
            writer,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }

    /// Replaces [`Writer`].
    #[must_use]
    pub fn with_writer<NewWr>(
        self,
        writer: NewWr,
    ) -> Cucumber<W, P, I, R, NewWr>
    where
        NewWr: Writer<W>,
    {
        let Self { parser, runner, .. } = self;
        Cucumber {
            parser,
            runner,
            writer,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }
}

impl<W, P, I, R, Wr> Cucumber<W, P, I, R, Wr>
where
    W: World,
    P: Parser<I>,
    R: Runner<W>,
    Wr: Writer<W>,
{
    /// Runs [`Cucumber`].
    ///
    /// [`Feature`]s sourced [`Parser`] are fed to [`Runner`], which produces
    /// events handled by [`Writer`].
    ///
    /// [`Feature`]: gherkin::Feature
    pub async fn run(self, input: I) -> Wr {
        let Cucumber {
            parser,
            runner,
            mut writer,
            ..
        } = self;

        let events_stream = runner.run(parser.parse(input));
        futures::pin_mut!(events_stream);
        while let Some(ev) = events_stream.next().await {
            writer.handle_event(ev).await;
        }
        writer
    }

    /// Runs [`Cucumber`] with [`Scenario`]s filter.
    ///
    /// [`Feature`]s sourced [`Parser`] are fed to [`Runner`], which produces
    /// events handled by [`Writer`].
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Scenario`]: gherkin::Scenario
    pub async fn filter_run<F>(self, input: I, filter: F) -> Wr
    where
        F: Fn(
                &gherkin::Feature,
                Option<&gherkin::Rule>,
                &gherkin::Scenario,
            ) -> bool
            + 'static,
    {
        let Cucumber {
            parser,
            runner,
            mut writer,
            ..
        } = self;

        let features = parser.parse(input);

        let filtered = features.map(move |feature| {
            let mut feature = feature?;
            let scenarios = mem::take(&mut feature.scenarios);
            feature.scenarios = scenarios
                .into_iter()
                .filter(|s| filter(&feature, None, s))
                .collect();

            let mut rules = mem::take(&mut feature.rules);
            for r in &mut rules {
                let scenarios = mem::take(&mut r.scenarios);
                r.scenarios = scenarios
                    .into_iter()
                    .filter(|s| filter(&feature, Some(r), s))
                    .collect();
            }
            feature.rules = rules;

            Ok(feature)
        });

        let events_stream = runner.run(filtered);
        futures::pin_mut!(events_stream);
        while let Some(ev) = events_stream.next().await {
            writer.handle_event(ev).await;
        }
        writer
    }
}

impl<W, P, I, R, Wr> Debug for Cucumber<W, P, I, R, Wr>
where
    P: Debug,
    R: Debug,
    Wr: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cucumber")
            .field("parser", &self.parser)
            .field("runner", &self.runner)
            .field("writer", &self.writer)
            .finish()
    }
}

type DefaultCucumber<W, I> = Cucumber<
    W,
    parser::Basic,
    I,
    runner::Basic<
        W,
        fn(
            &gherkin::Feature,
            Option<&gherkin::Rule>,
            &gherkin::Scenario,
        ) -> ScenarioType,
    >,
    writer::Summarized<writer::Normalized<W, writer::Basic>>,
>;

impl<W, I> Default for DefaultCucumber<W, I>
where
    W: World + Debug,
    I: AsRef<Path>,
{
    fn default() -> Self {
        let f: fn(
            &gherkin::Feature,
            Option<&gherkin::Rule>,
            &gherkin::Scenario,
        ) -> _ = |_, _, scenario| {
            scenario
                .tags
                .iter()
                .any(|tag| tag == "serial")
                .then(|| ScenarioType::Serial)
                .unwrap_or(ScenarioType::Concurrent)
        };

        Cucumber::custom()
            .with_parser(parser::Basic)
            .with_runner(
                runner::Basic::custom()
                    .which_scenario(f)
                    .max_concurrent_scenarios(Some(64)),
            )
            .with_writer(writer::Basic::new().normalized().summarized())
    }
}

impl<W, I> DefaultCucumber<W, I>
where
    W: World + Debug,
    I: AsRef<Path>,
{
    /// Creates default [`Cucumber`] instance.
    ///
    /// * [`Parser`] — [`parser::Basic`]
    ///
    /// * [`Runner`] — [`runner::Basic`]
    ///   * [`ScenarioType`] — [`Concurrent`] by default, [`Serial`] if
    ///     `@serial` [tag] is present on a [`Scenario`];
    ///   * Allowed to run up to 64 [`Concurrent`] [`Scenario`]s.
    ///
    /// * [`Writer`] — [`Normalized`] [`writer::Basic`].
    ///
    /// [`Concurrent`]: runner::basic::ScenarioType::Concurrent
    /// [`Normalized`]: writer::Normalized
    /// [`Parser`]: parser::Parser
    /// [`Scenario`]: gherkin::Scenario
    /// [`Serial`]: runner::basic::ScenarioType::Serial
    /// [`ScenarioType`]: runner::basic::ScenarioType
    ///
    /// [tag]: https://cucumber.io/docs/cucumber/api/#tags
    #[must_use]
    pub fn new() -> Self {
        Cucumber::default()
    }
}

impl<W, I, P, Wr, F> Cucumber<W, P, I, runner::Basic<W, F>, Wr>
where
    W: World,
    P: Parser<I>,
    Wr: Writer<W>,
    F: Fn(
        &gherkin::Feature,
        Option<&gherkin::Rule>,
        &gherkin::Scenario,
    ) -> ScenarioType,
{
    /// If `max` is [`Some`] number of concurrently executed [`Scenarios`] will
    /// be limited.
    #[must_use]
    pub fn max_concurrent_scenarios(mut self, max: Option<usize>) -> Self {
        self.runner = self.runner.max_concurrent_scenarios(max);
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
    ) -> Cucumber<W, P, I, runner::Basic<W, Which>, Wr>
    where
        Which: Fn(
                &gherkin::Feature,
                Option<&gherkin::Rule>,
                &gherkin::Scenario,
            ) -> ScenarioType
            + 'static,
    {
        let Self {
            parser,
            runner,
            writer,
            ..
        } = self;
        Cucumber {
            parser,
            runner: runner.which_scenario(func),
            writer,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }

    /// Replaces [`Collection`] of [`Step`]s.
    ///
    /// [`Collection`]: step::Collection
    /// [`Step`]: step::Step
    pub fn steps(mut self, steps: step::Collection<W>) -> Self {
        self.runner = self.runner.steps(steps);
        self
    }

    /// Inserts [Given] [`Step`].
    ///
    /// [Given]: https://cucumber.io/docs/gherkin/reference/#given
    pub fn given(mut self, regex: Regex, step: Step<W>) -> Self {
        self.runner = self.runner.given(regex, step);
        self
    }

    /// Inserts [When] [`Step`].
    ///
    /// [When]: https://cucumber.io/docs/gherkin/reference/#When
    pub fn when(mut self, regex: Regex, step: Step<W>) -> Self {
        self.runner = self.runner.when(regex, step);
        self
    }

    /// Inserts [Then] [`Step`].
    ///
    /// [Then]: https://cucumber.io/docs/gherkin/reference/#then
    pub fn then(mut self, regex: Regex, step: Step<W>) -> Self {
        self.runner = self.runner.then(regex, step);
        self
    }
}

impl<W, I, P, R, Wr> Cucumber<W, P, I, R, writer::Summarized<Wr>>
where
    W: World,
    P: Parser<I>,
    R: Runner<W>,
    Wr: Writer<W>,
{
    /// Runs [`Cucumber`].
    ///
    /// [`Feature`]s sourced by [`Parser`] are fed to [`Runner`], which produces
    /// events handled by [`Writer`].
    ///
    /// # Panics
    ///
    /// If at least one [`Step`] failed.
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Step`]: gherkin::Step
    pub async fn run_and_exit(self, input: I) {
        self.filter_run_and_exit(input, |_, _, _| true).await;
    }

    /// Runs [`Cucumber`] with [`Scenario`]s filter.
    ///
    /// [`Feature`]s sourced by [`Parser`] are fed to [`Runner`], which produces
    /// events handled by [`Writer`].
    ///
    /// # Panics
    ///
    /// If at least one [`Step`] failed.
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
    pub async fn filter_run_and_exit<F>(self, input: I, filter: F)
    where
        F: Fn(
                &gherkin::Feature,
                Option<&gherkin::Rule>,
                &gherkin::Scenario,
            ) -> bool
            + 'static,
    {
        let summary = self.filter_run(input, filter).await;
        if summary.is_failed() {
            let failed_steps = summary.steps.failed;
            let parsing_errors = summary.parsing_errors;
            panic!(
                "{} step{} failed, {} parsing error{}",
                failed_steps,
                if failed_steps == 1 { "" } else { "s" },
                parsing_errors,
                if parsing_errors == 1 { "" } else { "s" },
            );
        }
    }
}
