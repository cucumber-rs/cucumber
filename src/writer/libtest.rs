// Copyright (c) 2018-2023  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! [Rust `libtest`][1] compatible [`Writer`] implementation.
//!
//! [1]: https://doc.rust-lang.org/rustc/tests/index.html

use std::{
    fmt::Debug,
    io, iter, mem,
    str::FromStr,
    time::{Duration, SystemTime},
};

use async_trait::async_trait;
use derive_more::From;
use either::Either;
use itertools::Itertools as _;
use serde::Serialize;

use crate::{
    cli,
    event::{self, Retries},
    parser,
    writer::{
        self,
        basic::{coerce_error, trim_path},
        out::WriteStrExt as _,
        Arbitrary, Normalize, Summarize,
    },
    Event, World, Writer, WriterExt as _,
};

/// CLI options of a [`Libtest`] [`Writer`].
#[derive(clap::Args, Clone, Debug, Default)]
#[group(skip)]
pub struct Cli {
    /// Formatting of the output.
    #[arg(long, value_name = "json")]
    pub format: Option<Format>,

    /// Show captured stdout of successful tests. Currently, outputs only step
    /// function location.
    #[arg(long)]
    pub show_output: bool,

    /// Show execution time of each test.
    #[arg(long, value_name = "plain|colored", default_missing_value = "plain")]
    pub report_time: Option<ReportTime>,

    /// Enable nightly-only flags.
    #[arg(short = 'Z')]
    pub nightly: Option<String>,
}

/// Output formats.
///
/// Currently supports only JSON.
#[derive(Clone, Copy, Debug)]
pub enum Format {
    /// [`libtest`][1]'s JSON format.
    ///
    /// [1]: https://doc.rust-lang.org/rustc/tests/index.html
    Json,
}

impl FromStr for Format {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(Self::Json),
            s @ ("pretty" | "terse" | "junit") => {
                Err(format!("`{s}` option is not supported yet"))
            }
            s => Err(format!(
                "Unknown option `{s}`, expected `pretty` or `json`",
            )),
        }
    }
}

/// Format of reporting time.
#[derive(Clone, Copy, Debug)]
pub enum ReportTime {
    /// Plain time reporting.
    Plain,

    /// Colored time reporting.
    Colored,
}

impl FromStr for ReportTime {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "plain" => Ok(Self::Plain),
            "colored" => Ok(Self::Colored),
            s => Err(format!(
                "Unknown option `{s}`, expected `plain` or `colored`",
            )),
        }
    }
}

/// [`libtest`][1] compatible [`Writer`].
///
/// Currently used only to support `--format=json` option.
///
/// # Ordering
///
/// This [`Writer`] isn't [`Normalized`] by itself, so should be wrapped into a
/// [`writer::Normalize`], otherwise will produce output [`Event`]s in a broken
/// order.
///
/// Ideally, we shouldn't wrap this into a [`writer::Normalize`] and leave this
/// to tools, parsing JSON output. Unfortunately, not all tools can do that (ex.
/// [`IntelliJ Rust`][2]), so it's still recommended to wrap this into
/// [`writer::Normalize`] even if it can mess up timing reports.
///
/// [`Normalized`]: writer::Normalized
/// [1]: https://doc.rust-lang.org/rustc/tests/index.html
/// [2]: https://github.com/intellij-rust/intellij-rust/issues/9041
#[derive(Debug)]
pub struct Libtest<W, Out: io::Write = io::Stdout> {
    /// [`io::Write`] implementor to output into.
    output: Out,

    /// Collection of events before [`ParsingFinished`] is received.
    ///
    /// Until a [`ParsingFinished`] is received, all the events are stored
    /// inside [`Libtest::events`] and outputted only after that event is
    /// received. This is done, because [`libtest`][1]'s first event must
    /// contain number of executed test cases.
    ///
    /// [`ParsingFinished`]: event::Cucumber::ParsingFinished
    /// [1]: https://doc.rust-lang.org/rustc/tests/index.html
    events: Vec<parser::Result<Event<event::Cucumber<W>>>>,

    /// Indicates whether a [`ParsingFinished`] event was received.
    ///
    /// [`ParsingFinished`]: event::Cucumber::ParsingFinished
    parsed_all: bool,

    /// Number of passed [`Step`]s.
    ///
    /// [`Step`]: gherkin::Step
    passed: usize,

    /// Number of failed [`Step`]s.
    ///
    /// [`Step`]: gherkin::Step
    failed: usize,

    /// Number of retried [`Step`]s.
    ///
    /// [`Step`]: gherkin::Step
    retried: usize,

    /// Number of skipped [`Step`]s.
    ///
    /// [`Step`]: gherkin::Step
    ignored: usize,

    /// Number of [`Parser`] errors.
    ///
    /// [`Parser`]: crate::Parser
    parsing_errors: usize,

    /// Number of [`Hook`] errors.
    ///
    /// [`Hook`]: event::Hook
    hook_errors: usize,

    /// Number of [`Feature`]s with [`path`] set to [`None`].
    ///
    /// This value is used to generate a unique name for each [`Feature`] to
    /// avoid name collisions.
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`path`]: gherkin::Feature::path
    features_without_path: usize,

    /// [`SystemTime`] when the [`Started`] event was received.
    ///
    /// [`Started`]: event::Cucumber::Started
    started_at: Option<SystemTime>,

    /// [`SystemTime`] when the [`Step::Started`]/[`Hook::Started`] event was
    /// received.
    ///
    /// [`Hook::Started`]: event::Hook::Started
    /// [`Step::Started`]: event::Step::Started
    step_started_at: Option<SystemTime>,
}

// Implemented manually to omit redundant `World: Clone` trait bound, imposed by
// `#[derive(Clone)]`.
impl<World, Out: Clone + io::Write> Clone for Libtest<World, Out> {
    fn clone(&self) -> Self {
        Self {
            output: self.output.clone(),
            events: self.events.clone(),
            parsed_all: self.parsed_all,
            passed: self.passed,
            failed: self.failed,
            retried: self.retried,
            ignored: self.ignored,
            parsing_errors: self.parsing_errors,
            hook_errors: self.hook_errors,
            features_without_path: self.features_without_path,
            started_at: self.started_at,
            step_started_at: self.step_started_at,
        }
    }
}

#[async_trait(?Send)]
impl<W: World + Debug, Out: io::Write> Writer<W> for Libtest<W, Out> {
    type Cli = Cli;

    async fn handle_event(
        &mut self,
        event: parser::Result<Event<event::Cucumber<W>>>,
        cli: &Self::Cli,
    ) {
        self.handle_cucumber_event(event, cli);
    }
}

/// Shortcut of a [`Libtest::or()`] return type.
pub type Or<W, Wr> = writer::Or<
    Wr,
    Normalize<W, Libtest<W, io::Stdout>>,
    fn(
        &parser::Result<Event<event::Cucumber<W>>>,
        &cli::Compose<<Wr as Writer<W>>::Cli, Cli>,
    ) -> bool,
>;

/// Shortcut of a [`Libtest::or_basic()`] return type.
pub type OrBasic<W> = Or<W, Summarize<Normalize<W, writer::Basic>>>;

impl<W: Debug + World> Libtest<W, io::Stdout> {
    /// Creates a new [`Normalized`] [`Libtest`] [`Writer`] outputting into the
    /// [`io::Stdout`].
    ///
    /// [`Normalized`]: writer::Normalized
    #[must_use]
    pub fn stdout() -> Normalize<W, Self> {
        Self::new(io::stdout())
    }

    /// Creates a new [`Writer`] which uses a [`Normalized`] [`Libtest`] in case
    /// [`Cli::format`] is set to [`Json`], or provided the `writer` otherwise.
    ///
    /// [`Json`]: Format::Json
    /// [`Normalized`]: writer::Normalized
    #[must_use]
    pub fn or<AnotherWriter: Writer<W>>(
        writer: AnotherWriter,
    ) -> Or<W, AnotherWriter> {
        writer::Or::new(writer, Self::stdout(), |_, cli| {
            !matches!(cli.right.format, Some(writer::libtest::Format::Json))
        })
    }

    /// Creates a new [`Writer`] which uses a [`Normalized`] [`Libtest`] in case
    /// [`Cli::format`] is set to [`Json`], or a [`Normalized`]
    /// [`writer::Basic`] otherwise.
    ///
    /// [`Json`]: Format::Json
    /// [`Normalized`]: writer::Normalized
    #[must_use]
    pub fn or_basic() -> OrBasic<W> {
        Self::or(writer::Basic::stdout().summarized())
    }
}

impl<W: Debug + World, Out: io::Write> Libtest<W, Out> {
    /// Creates a new [`Normalized`] [`Libtest`] [`Writer`] outputting into the
    /// provided `output`.
    ///
    /// Theoretically, normalization should be done by the tool that's consuming
    /// the output og this [`Writer`]. But lack of clear specification of the
    /// [`libtest`][1]'s JSON output leads to some tools [struggling][2] to
    /// interpret it. So, we recommend using a [`Normalized`] [`Libtest::new()`]
    /// rather than a non-[`Normalized`] [`Libtest::raw()`].
    ///
    /// [`Normalized`]: writer::Normalized
    /// [1]: https://doc.rust-lang.org/rustc/tests/index.html
    /// [2]: https://github.com/intellij-rust/intellij-rust/issues/9041
    #[must_use]
    pub fn new(output: Out) -> Normalize<W, Self> {
        Self::raw(output).normalized()
    }

    /// Creates a new non-[`Normalized`] [`Libtest`] [`Writer`] outputting into
    /// the provided `output`.
    ///
    /// Theoretically, normalization should be done by the tool that's consuming
    /// the output og this [`Writer`]. But lack of clear specification of the
    /// [`libtest`][1]'s JSON output leads to some tools [struggling][2] to
    /// interpret it. So, we recommend using a [`Normalized`] [`Libtest::new()`]
    /// rather than a non-[`Normalized`] [`Libtest::raw()`].
    ///
    /// [`Normalized`]: writer::Normalized
    /// [1]: https://doc.rust-lang.org/rustc/tests/index.html
    /// [2]: https://github.com/intellij-rust/intellij-rust/issues/9041
    #[must_use]
    pub const fn raw(output: Out) -> Self {
        Self {
            output,
            events: Vec::new(),
            parsed_all: false,
            passed: 0,
            failed: 0,
            retried: 0,
            parsing_errors: 0,
            hook_errors: 0,
            ignored: 0,
            features_without_path: 0,
            started_at: None,
            step_started_at: None,
        }
    }

    /// Handles the provided [`event::Cucumber`].
    ///
    /// Until [`ParsingFinished`] is received, all the events are stored inside
    /// [`Libtest::events`] and outputted only after that event is received.
    /// This is done, because [`libtest`][1]'s first event must contain number
    /// of executed test cases.
    ///
    /// [1]: https://doc.rust-lang.org/rustc/tests/index.html
    /// [`ParsingFinished`]: event::Cucumber::ParsingFinished
    fn handle_cucumber_event(
        &mut self,
        event: parser::Result<Event<event::Cucumber<W>>>,
        cli: &Cli,
    ) {
        use event::{Cucumber, Metadata};

        let unite = |ev: Result<(Cucumber<W>, Metadata), _>| {
            ev.map(|(e, m)| m.insert(e))
        };

        match (event.map(Event::split), self.parsed_all) {
            (event @ Ok((Cucumber::ParsingFinished { .. }, _)), false) => {
                self.parsed_all = true;

                let all_events =
                    iter::once(unite(event)).chain(mem::take(&mut self.events));
                for ev in all_events {
                    self.output_event(ev, cli);
                }
            }
            (event, false) => self.events.push(unite(event)),
            (event, true) => self.output_event(unite(event), cli),
        }
    }

    /// Outputs the provided [`event::Cucumber`].
    fn output_event(
        &mut self,
        event: parser::Result<Event<event::Cucumber<W>>>,
        cli: &Cli,
    ) {
        for ev in self.expand_cucumber_event(event, cli) {
            self.output
                .write_line(serde_json::to_string(&ev).unwrap_or_else(|e| {
                    panic!("Failed to serialize `LibTestJsonEvent`: {e}")
                }))
                .unwrap_or_else(|e| panic!("Failed to write: {e}"));
        }
    }

    /// Converts the provided [`event::Cucumber`] into [`LibTestJsonEvent`]s.
    fn expand_cucumber_event(
        &mut self,
        event: parser::Result<Event<event::Cucumber<W>>>,
        cli: &Cli,
    ) -> Vec<LibTestJsonEvent> {
        use event::Cucumber;

        match event.map(Event::split) {
            Ok((Cucumber::Started, meta)) => {
                self.started_at = Some(meta.at);
                Vec::new()
            }
            Ok((
                Cucumber::ParsingFinished {
                    steps,
                    parser_errors,
                    ..
                },
                _,
            )) => {
                vec![SuiteEvent::Started {
                    test_count: steps + parser_errors,
                }
                .into()]
            }
            Ok((Cucumber::Finished, meta)) => {
                let exec_time = self
                    .started_at
                    .and_then(|started| meta.at.duration_since(started).ok())
                    .as_ref()
                    .map(Duration::as_secs_f64);

                let failed =
                    self.failed + self.parsing_errors + self.hook_errors;
                let results = SuiteResults {
                    passed: self.passed,
                    failed,
                    ignored: self.ignored,
                    measured: 0,
                    filtered_out: 0,
                    exec_time,
                };
                let ev = if failed == 0 {
                    SuiteEvent::Ok { results }
                } else {
                    SuiteEvent::Failed { results }
                }
                .into();

                vec![ev]
            }
            Ok((Cucumber::Feature(feature, ev), meta)) => {
                self.expand_feature_event(&feature, ev, meta, cli)
            }
            Err(e) => {
                self.parsing_errors += 1;

                let path = match &e {
                    parser::Error::Parsing(e) => match &**e {
                        gherkin::ParseFileError::Parsing { path, .. }
                        | gherkin::ParseFileError::Reading { path, .. } => {
                            Some(path)
                        }
                    },
                    parser::Error::ExampleExpansion(e) => e.path.as_ref(),
                };
                let name = path.and_then(|p| p.to_str()).map_or_else(
                    || self.parsing_errors.to_string(),
                    |p| p.escape_default().to_string(),
                );
                let name = format!("Feature: Parsing {name}");

                vec![
                    TestEvent::started(name.clone()).into(),
                    TestEvent::failed(name, None)
                        .with_stdout(e.to_string())
                        .into(),
                ]
            }
        }
    }

    /// Converts the provided [`event::Feature`] into [`LibTestJsonEvent`]s.
    fn expand_feature_event(
        &mut self,
        feature: &gherkin::Feature,
        ev: event::Feature<W>,
        meta: event::Metadata,
        cli: &Cli,
    ) -> Vec<LibTestJsonEvent> {
        use event::{Feature, Rule};

        match ev {
            Feature::Started
            | Feature::Finished
            | Feature::Rule(_, Rule::Started | Rule::Finished) => Vec::new(),
            Feature::Rule(rule, Rule::Scenario(scenario, ev)) => self
                .expand_scenario_event(
                    feature,
                    Some(&rule),
                    &scenario,
                    ev,
                    meta,
                    cli,
                ),
            Feature::Scenario(scenario, ev) => self
                .expand_scenario_event(feature, None, &scenario, ev, meta, cli),
        }
    }

    /// Converts the provided [`event::Scenario`] into [`LibTestJsonEvent`]s.
    fn expand_scenario_event(
        &mut self,
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        ev: event::RetryableScenario<W>,
        meta: event::Metadata,
        cli: &Cli,
    ) -> Vec<LibTestJsonEvent> {
        use event::Scenario;

        let retries = ev.retries;
        match ev.event {
            Scenario::Started | Scenario::Finished => Vec::new(),
            Scenario::Hook(ty, ev) => self.expand_hook_event(
                feature, rule, scenario, ty, ev, retries, meta, cli,
            ),
            Scenario::Background(step, ev) => self.expand_step_event(
                feature, rule, scenario, &step, ev, retries, true, meta, cli,
            ),
            Scenario::Step(step, ev) => self.expand_step_event(
                feature, rule, scenario, &step, ev, retries, false, meta, cli,
            ),
            // We do use `print!()` intentionally here to support `libtest`
            // output capturing properly, which can only capture output from
            // the standard library’s `print!()` macro.
            // This is the same as `tracing_subscriber::fmt::TestWriter` does
            // (check its documentation for details).
            #[allow(clippy::print_stdout)]
            Scenario::Log(msg) => {
                print!("{msg}");
                vec![]
            }
        }
    }

    /// Converts the provided [`event::Hook`] into [`LibTestJsonEvent`]s.
    #[allow(clippy::too_many_arguments)]
    fn expand_hook_event(
        &mut self,
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        hook: event::HookType,
        ev: event::Hook<W>,
        retries: Option<Retries>,
        meta: event::Metadata,
        cli: &Cli,
    ) -> Vec<LibTestJsonEvent> {
        match ev {
            event::Hook::Started => {
                self.step_started_at(meta, cli);
                Vec::new()
            }
            event::Hook::Passed => Vec::new(),
            event::Hook::Failed(world, info) => {
                self.hook_errors += 1;

                let name = self.test_case_name(
                    feature,
                    rule,
                    scenario,
                    Either::Left(hook),
                    retries,
                );

                vec![
                    TestEvent::started(name.clone()).into(),
                    TestEvent::failed(name, self.step_exec_time(meta, cli))
                        .with_stdout(format!(
                            "{}{}",
                            coerce_error(&info),
                            world
                                .map(|w| format!("\n{w:#?}"))
                                .unwrap_or_default(),
                        ))
                        .into(),
                ]
            }
        }
    }

    /// Converts the provided [`event::Step`] into [`LibTestJsonEvent`]s.
    #[allow(clippy::too_many_arguments)]
    fn expand_step_event(
        &mut self,
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        step: &gherkin::Step,
        ev: event::Step<W>,
        retries: Option<Retries>,
        is_background: bool,
        meta: event::Metadata,
        cli: &Cli,
    ) -> Vec<LibTestJsonEvent> {
        use event::Step;

        let name = self.test_case_name(
            feature,
            rule,
            scenario,
            Either::Right((step, is_background)),
            retries,
        );

        let ev = match ev {
            Step::Started => {
                self.step_started_at(meta, cli);
                TestEvent::started(name)
            }
            Step::Passed(_, loc) => {
                self.passed += 1;

                let event = TestEvent::ok(name, self.step_exec_time(meta, cli));
                if cli.show_output {
                    event.with_stdout(format!(
                        "{}:{}:{} (defined){}",
                        feature
                            .path
                            .as_ref()
                            .and_then(|p| p.to_str().map(trim_path))
                            .unwrap_or(&feature.name),
                        step.position.line,
                        step.position.col,
                        loc.map(|l| format!(
                            "\n{}:{}:{} (matched)",
                            l.path, l.line, l.column,
                        ))
                        .unwrap_or_default()
                    ))
                } else {
                    event
                }
            }
            Step::Skipped => {
                self.ignored += 1;

                let event =
                    TestEvent::ignored(name, self.step_exec_time(meta, cli));
                if cli.show_output {
                    event.with_stdout(format!(
                        "{}:{}:{} (defined)",
                        feature
                            .path
                            .as_ref()
                            .and_then(|p| p.to_str().map(trim_path))
                            .unwrap_or(&feature.name),
                        step.position.line,
                        step.position.col,
                    ))
                } else {
                    event
                }
            }
            Step::Failed(_, loc, world, err) => {
                if retries
                    .map(|r| {
                        r.left > 0 && !matches!(err, event::StepError::NotFound)
                    })
                    .unwrap_or_default()
                {
                    self.retried += 1;
                } else {
                    self.failed += 1;
                }

                TestEvent::failed(name, self.step_exec_time(meta, cli))
                    .with_stdout(format!(
                        "{}:{}:{} (defined){}\n{err}{}",
                        feature
                            .path
                            .as_ref()
                            .and_then(|p| p.to_str().map(trim_path))
                            .unwrap_or(&feature.name),
                        step.position.line,
                        step.position.col,
                        loc.map(|l| format!(
                            "\n{}:{}:{} (matched)",
                            l.path, l.line, l.column,
                        ))
                        .unwrap_or_default(),
                        world.map(|w| format!("\n{w:#?}")).unwrap_or_default(),
                    ))
            }
        };

        vec![ev.into()]
    }

    /// Generates test case name.
    fn test_case_name(
        &mut self,
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        step: Either<event::HookType, (&gherkin::Step, IsBackground)>,
        retries: Option<Retries>,
    ) -> String {
        let feature_name = format!(
            "{}: {} {}",
            feature.keyword,
            feature.name,
            feature
                .path
                .as_ref()
                .and_then(|p| p.to_str().map(trim_path))
                .map_or_else(
                    || {
                        self.features_without_path += 1;
                        self.features_without_path.to_string()
                    },
                    |s| s.escape_default().to_string()
                ),
        );
        let rule_name = rule
            .as_ref()
            .map(|r| format!("{}: {}: {}", r.position.line, r.keyword, r.name));
        let scenario_name = format!(
            "{}: {}: {}{}",
            scenario.position.line,
            scenario.keyword,
            scenario.name,
            retries
                .filter(|r| r.current > 0)
                .map(|r| format!(
                    " | Retry attempt {}/{}",
                    r.current,
                    r.current + r.left,
                ))
                .unwrap_or_default(),
        );
        let step_name = match step {
            Either::Left(hook) => format!("{hook} hook"),
            Either::Right((step, is_bg)) => format!(
                "{}: {} {}{}",
                step.position.line,
                is_bg
                    .then(|| feature
                        .background
                        .as_ref()
                        .map_or("Background", |bg| bg.keyword.as_str()))
                    .unwrap_or_default(),
                step.keyword,
                step.value,
            ),
        };

        [
            Some(feature_name),
            rule_name,
            Some(scenario_name),
            Some(step_name),
        ]
        .into_iter()
        .flatten()
        .join("::")
    }

    /// Saves [`Step`] starting [`SystemTime`].
    ///
    /// [`Step`]: gherkin::Step
    fn step_started_at(&mut self, meta: event::Metadata, cli: &Cli) {
        self.step_started_at =
            Some(meta.at).filter(|_| cli.report_time.is_some());
    }

    /// Retrieves [`Duration`] since the last [`Libtest::step_started_at()`]
    /// call.
    fn step_exec_time(
        &mut self,
        meta: event::Metadata,
        cli: &Cli,
    ) -> Option<Duration> {
        self.step_started_at.take().and_then(|started| {
            meta.at
                .duration_since(started)
                .ok()
                .filter(|_| cli.report_time.is_some())
        })
    }
}

/// Indicator, whether a [`Step`] is [`Background`] or not.
///
/// [`Background`]: event::Scenario::Background
/// [`Step`]: gherkin::Step
type IsBackground = bool;

impl<W, O: io::Write> writer::NonTransforming for Libtest<W, O> {}

impl<W, O> writer::Stats<W> for Libtest<W, O>
where
    O: io::Write,
    Self: Writer<W>,
{
    fn passed_steps(&self) -> usize {
        self.passed
    }

    fn skipped_steps(&self) -> usize {
        self.ignored
    }

    fn failed_steps(&self) -> usize {
        self.failed
    }

    fn retried_steps(&self) -> usize {
        self.retried
    }

    fn parsing_errors(&self) -> usize {
        self.parsing_errors
    }

    fn hook_errors(&self) -> usize {
        self.hook_errors
    }
}

#[async_trait(?Send)]
impl<'val, W, Val, Out> Arbitrary<'val, W, Val> for Libtest<W, Out>
where
    W: World + Debug,
    Val: AsRef<str> + 'val,
    Out: io::Write,
{
    async fn write(&mut self, val: Val)
    where
        'val: 'async_trait,
    {
        self.output
            .write_line(val.as_ref())
            .unwrap_or_else(|e| panic!("Failed to write: {e}"));
    }
}

/// [`libtest`][1]'s JSON event.
///
/// This format isn't stable, so this implementation uses [implementation][1] as
/// a reference point.
///
/// [1]: https://bit.ly/3PrLtKC
#[derive(Clone, Debug, From, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum LibTestJsonEvent {
    /// Event of test suite.
    Suite {
        /// [`SuiteEvent`]
        #[serde(flatten)]
        event: SuiteEvent,
    },

    /// Event of the test case.
    Test {
        /// [`TestEvent`]
        #[serde(flatten)]
        event: TestEvent,
    },
}

/// Test suite event.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum SuiteEvent {
    /// Test suite started.
    Started {
        /// Number of test cases. In our case, this is number of parsed
        /// [`Step`]s and [`Parser`] errors.
        ///
        /// [`Parser`]: crate::Parser
        /// [`Step`]: gherkin::Step
        test_count: usize,
    },

    /// Test suite finished without errors.
    Ok {
        /// Execution results.
        #[serde(flatten)]
        results: SuiteResults,
    },

    /// Test suite encountered errors during the execution.
    Failed {
        /// Execution results.
        #[serde(flatten)]
        results: SuiteResults,
    },
}

/// Test suite execution results.
#[derive(Clone, Copy, Debug, Serialize)]
struct SuiteResults {
    /// Number of passed test cases.
    passed: usize,

    /// Number of failed test cases.
    failed: usize,

    /// Number of ignored test cases.
    ignored: usize,

    /// Number of measured benches.
    measured: usize,

    // TODO: Figure out a way to actually report this.
    /// Number of filtered out test cases.
    filtered_out: usize,

    /// Test suite execution time.
    #[serde(skip_serializing_if = "Option::is_none")]
    exec_time: Option<f64>,
}

/// Test case event.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum TestEvent {
    /// Test case started.
    Started(TestEventInner),

    /// Test case finished successfully.
    Ok(TestEventInner),

    /// Test case failed.
    Failed(TestEventInner),

    /// Test case ignored.
    Ignored(TestEventInner),

    /// Test case timed out.
    Timeout(TestEventInner),
}

impl TestEvent {
    /// Creates a new [`TestEvent::Started`].
    const fn started(name: String) -> Self {
        Self::Started(TestEventInner::new(name))
    }

    /// Creates a new [`TestEvent::Ok`].
    fn ok(name: String, exec_time: Option<Duration>) -> Self {
        Self::Ok(TestEventInner::new(name).with_exec_time(exec_time))
    }

    /// Creates a new [`TestEvent::Failed`].
    fn failed(name: String, exec_time: Option<Duration>) -> Self {
        Self::Failed(TestEventInner::new(name).with_exec_time(exec_time))
    }

    /// Creates a new [`TestEvent::Ignored`].
    fn ignored(name: String, exec_time: Option<Duration>) -> Self {
        Self::Ignored(TestEventInner::new(name).with_exec_time(exec_time))
    }

    /// Creates a new [`TestEvent::Timeout`].
    #[allow(dead_code)]
    fn timeout(name: String, exec_time: Option<Duration>) -> Self {
        Self::Timeout(TestEventInner::new(name).with_exec_time(exec_time))
    }

    /// Adds a [`TestEventInner::stdout`].
    fn with_stdout(self, mut stdout: String) -> Self {
        if !stdout.ends_with('\n') {
            stdout.push('\n');
        }

        match self {
            Self::Started(inner) => Self::Started(inner.with_stdout(stdout)),
            Self::Ok(inner) => Self::Ok(inner.with_stdout(stdout)),
            Self::Failed(inner) => Self::Failed(inner.with_stdout(stdout)),
            Self::Ignored(inner) => Self::Ignored(inner.with_stdout(stdout)),
            Self::Timeout(inner) => Self::Timeout(inner.with_stdout(stdout)),
        }
    }
}

/// Inner value of a [`TestEvent`].
#[derive(Clone, Debug, Serialize)]
struct TestEventInner {
    /// Name of this test case.
    name: String,

    /// [`Stdout`] of this test case.
    ///
    /// [`Stdout`]: io::Stdout
    #[serde(skip_serializing_if = "Option::is_none")]
    stdout: Option<String>,

    /// [`Stderr`] of this test case.
    ///
    /// Isn't actually used, as [IntelliJ Rust][1] ignores it.
    ///
    /// [1]: https://github.com/intellij-rust/intellij-rust/issues/9041
    /// [`Stderr`]: io::Stderr
    #[serde(skip_serializing_if = "Option::is_none")]
    stderr: Option<String>,

    /// Test case execution time.
    #[serde(skip_serializing_if = "Option::is_none")]
    exec_time: Option<f64>,
}

impl TestEventInner {
    /// Creates a new [`TestEventInner`].
    const fn new(name: String) -> Self {
        Self {
            name,
            stdout: None,
            stderr: None,
            exec_time: None,
        }
    }

    /// Adds a [`TestEventInner::exec_time`].
    fn with_exec_time(mut self, exec_time: Option<Duration>) -> Self {
        self.exec_time = exec_time.as_ref().map(Duration::as_secs_f64);
        self
    }

    /// Adds a [`TestEventInner::stdout`].
    #[allow(clippy::missing_const_for_fn)] // false positive: drop in const
    fn with_stdout(mut self, stdout: String) -> Self {
        self.stdout = Some(stdout);
        self
    }
}
