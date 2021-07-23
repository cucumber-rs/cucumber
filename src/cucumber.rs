//! Top-level [Cucumber] executor.
//!
//! [Cucumber]: https://cucumber.io

use std::{
    fmt::{Debug, Formatter},
    marker::PhantomData,
    path::Path,
};

use futures::StreamExt as _;
use regex::Regex;

use crate::{
    parser,
    runner::{self, basic::ScenarioType},
    step,
    writer::{self, WriterExt as _},
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
    /// [`Feature`]s sourced by [`Parser`] are fed to [`Runner`], which produces
    /// events handled by [`Writer`].
    ///
    /// [`Feature`]: gherkin::Feature
    pub async fn run(self, input: I) {
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
        runner::Basic<W, fn(&gherkin::Scenario) -> ScenarioType>,
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
                |sc| {
                    sc.tags
                        .iter()
                        .any(|tag| tag == "serial")
                        .then(|| ScenarioType::Serial)
                        .unwrap_or(ScenarioType::Concurrent)
                },
                16,
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
        runner::Basic<W, fn(&gherkin::Scenario) -> ScenarioType>,
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
    ///   * Allowed to run up to 16 [`Concurrent`] [`Scenario`]s.
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
        runner::Basic<W, fn(&gherkin::Scenario) -> ScenarioType>,
        Wr,
    >
where
    W: World,
    P: Parser<I>,
    Wr: Writer<W>,
{
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
    /// Runs [`Cucumber`] and exits with code `1` if any [`Step`] failed.
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

        if writer.is_failed() {
            let failed = writer.steps.failed;
            panic!(
                "{} step{} failed",
                failed,
                if failed > 1 { "s" } else { "" },
            );
        }
    }
}
