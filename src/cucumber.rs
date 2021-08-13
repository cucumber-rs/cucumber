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

impl<W, P, I, R, Wr> Cucumber<W, P, I, R, Wr>
where
    W: World,
    P: Parser<I>,
    R: Runner<W>,
    Wr: Writer<W>,
{
    /// Creates [`Cucumber`] with custom [`Parser`], [`Runner`] and [`Writer`].
    #[must_use]
    pub fn custom(parser: P, runner: R, writer: Wr) -> Self {
        Self {
            parser,
            runner,
            writer,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }

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

        let filtered = features.map(move |mut feature| {
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

            feature
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

impl<W, I> Default
    for Cucumber<
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
        writer::Summary<writer::Normalized<W, writer::Basic>>,
    >
where
    W: World + Debug,
    I: AsRef<Path>,
{
    fn default() -> Self {
        Cucumber::custom(
            parser::Basic,
            runner::basic::Basic::new(
                |_, _, sc| {
                    sc.tags
                        .iter()
                        .any(|tag| tag == "serial")
                        .then(|| ScenarioType::Serial)
                        .unwrap_or(ScenarioType::Concurrent)
                },
                Some(64),
                step::Collection::new(),
            ),
            writer::Basic::new().normalize().summarize(),
        )
    }
}

impl<W, I>
    Cucumber<
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
        writer::Summary<writer::Normalized<W, writer::Basic>>,
    >
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

impl<W, I, P, Wr>
    Cucumber<
        W,
        P,
        I,
        runner::Basic<
            W,
            fn(
                &gherkin::Feature,
                Option<&gherkin::Rule>,
                &gherkin::Scenario,
            ) -> ScenarioType,
        >,
        Wr,
    >
where
    W: World,
    P: Parser<I>,
    Wr: Writer<W>,
{
    /// Replaces [`Collection`] of [`Step`]s.
    ///
    /// [`Collection`]: step::Collection
    /// [`Step`]: step::Step
    pub fn steps(self, steps: step::Collection<W>) -> Self {
        let Cucumber {
            parser,
            runner,
            writer,
            ..
        } = self;
        Cucumber {
            parser,
            runner: runner.steps(steps),
            writer,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }

    /// Inserts [Given] [`Step`].
    ///
    /// [Given]: https://cucumber.io/docs/gherkin/reference/#given
    pub fn given(self, regex: Regex, step: Step<W>) -> Self {
        let Cucumber {
            parser,
            runner,
            writer,
            ..
        } = self;
        Cucumber {
            parser,
            runner: runner.given(regex, step),
            writer,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }

    /// Inserts [When] [`Step`].
    ///
    /// [When]: https://cucumber.io/docs/gherkin/reference/#When
    pub fn when(self, regex: Regex, step: Step<W>) -> Self {
        let Cucumber {
            parser,
            runner,
            writer,
            ..
        } = self;
        Cucumber {
            parser,
            runner: runner.when(regex, step),
            writer,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }

    /// Inserts [Then] [`Step`].
    ///
    /// [Then]: https://cucumber.io/docs/gherkin/reference/#then
    pub fn then(self, regex: Regex, step: Step<W>) -> Self {
        let Cucumber {
            parser,
            runner,
            writer,
            ..
        } = self;
        Cucumber {
            parser,
            runner: runner.then(regex, step),
            writer,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }
}

impl<W, I, P, R, Wr> Cucumber<W, P, I, R, writer::Summary<Wr>>
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
        let summary = self.run(input).await;
        if summary.is_failed() {
            let failed = summary.steps.failed;
            panic!(
                "{} step{} failed",
                failed,
                if failed > 1 { "s" } else { "" },
            );
        }
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
            let failed = summary.steps.failed;
            panic!(
                "{} step{} failed",
                failed,
                if failed > 1 { "s" } else { "" },
            );
        }
    }
}
