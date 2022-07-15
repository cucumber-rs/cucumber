// Copyright (c) 2018-2022  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! [`libtest`][1] compatible [`Writer`] implementation.
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
    event::{self, Retries},
    parser,
    writer::{self, basic::coerce_error, out::WriteStrExt as _, Arbitrary},
    Event, World, Writer,
};

/// CLI options of a [`Libtest`] [`Writer`].
#[derive(Debug, Clone, clap::Args)]
pub struct Cli {
    /// Configure formatting of output.
    #[clap(long, name = "json")]
    pub format: Option<Format>,

    /// Show captured stdout of successful tests. Currently outputs only `Step`
    /// function location.
    #[clap(long)]
    pub show_output: bool,

    /// Enable nightly-only flags.
    #[clap(short = 'Z')]
    pub nightly: Option<String>,
}

/// Output formats.
///
/// Currently supports only `JSON`.
#[derive(Clone, Copy, Debug)]
pub enum Format {
    /// [`libtest`][1]'s `JSON` format.
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

/// [`libtest`][1] compatible [`Writer`].
///
/// Currently used only to support `--format=json` option.
///
/// # Ordering
///
/// This [`Writer`] isn't [`Normalized`] by itself, so should be wrapped into
/// a [`writer::Normalize`], otherwise will produce output [`Event`]s in a
/// broken order.
/// Ideally, we shouldn't wrap this into [`writer::Normalize`] and leave this to
/// tools, parsing JSON output. Unfortunately, not all tools can do that (ex.
/// [`IntelliJ Rust`][2]), so it's still recommended to wrap this into
/// [`writer::Normalize`] even if it can mess up timing reports.
///
/// [1]: https://doc.rust-lang.org/rustc/tests/index.html
/// [2]: https://github.com/intellij-rust/intellij-rust/issues/9041
/// [`Normalized`]: writer::Normalized
#[derive(Clone, Debug)]
pub struct Libtest<W, Out: io::Write = io::Stdout> {
    /// [`io::Write`] implementor to output into.
    output: Out,

    /// Collection of events before [`ParsingFinished`] is received.
    ///
    /// Until [`ParsingFinished`] is received, all events are stored inside
    /// [`Libtest::events`] and outputted only after that event is received.
    /// This is done, because [`libtest`][1]'s first event must contain number
    /// of executed test cases.
    ///
    /// [1]: https://doc.rust-lang.org/rustc/tests/index.html
    /// [`ParsingFinished`]: event::Cucumber::ParsingFinished
    events: Vec<parser::Result<Event<event::Cucumber<W>>>>,

    /// Indicates whether [`ParsingFinished`] event was received.
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
    /// This value is used to generate a unique name to each [`Feature`] to
    /// avoid name collisions.
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`path`]: gherkin::Feature::path
    features_without_path: usize,

    /// [`SystemTime`] when the [`Started`] event was received.
    ///
    /// [`Started`]: event::Cucumber::Started
    started_at: Option<SystemTime>,
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

impl<W: Debug + World> Libtest<W, io::Stdout> {
    /// Creates a new [`Libtest`] [`Writer`] outputting into the [`io::Stdout`].
    #[must_use]
    pub fn stdout() -> Self {
        Self::new(io::stdout())
    }
}

impl<W: Debug + World, Out: io::Write> Libtest<W, Out> {
    /// Creates a new [`Libtest`] [`Writer`] outputting into the given `output`.
    #[must_use]
    pub const fn new(output: Out) -> Self {
        Self {
            output,
            events: Vec::new(),
            parsed_all: false,
            passed: 0,
            failed: 0,
            parsing_errors: 0,
            hook_errors: 0,
            ignored: 0,
            started_at: None,
            features_without_path: 0,
        }
    }

    /// Handles [`event::Cucumber`].
    ///
    /// Until [`ParsingFinished`] is received, all events are stored inside
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

    /// Outputs [`event::Cucumber`].
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

    /// Converts [`event::Cucumber`] into [`LibTestJsonEvent`]s.
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
            Ok((Cucumber::Finished, _)) => {
                let exec_time = self
                    .started_at
                    .and_then(|started| {
                        SystemTime::now().duration_since(started).ok()
                    })
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
            Ok((Cucumber::Feature(feature, ev), _)) => {
                self.expand_feature_event(&feature, ev, cli)
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
                let name = format!("Parsing {name}");

                vec![
                    TestEvent::started(name.clone()).into(),
                    TestEvent::failed(name).with_stdout(e.to_string()).into(),
                ]
            }
        }
    }

    /// Converts [`event::Feature`] into [`LibTestJsonEvent`]s.
    fn expand_feature_event(
        &mut self,
        feature: &gherkin::Feature,
        ev: event::Feature<W>,
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
                    cli,
                ),
            Feature::Scenario(scenario, ev) => {
                self.expand_scenario_event(feature, None, &scenario, ev, cli)
            }
        }
    }

    /// Converts [`event::Scenario`] into [`LibTestJsonEvent`]s.
    fn expand_scenario_event(
        &mut self,
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        ev: event::Scenario<W>,
        cli: &Cli,
    ) -> Vec<LibTestJsonEvent> {
        use event::Scenario;

        match ev {
            Scenario::Started(_) | Scenario::Finished(_) => Vec::new(),
            Scenario::Hook(ty, ev, retries) => {
                self.expand_hook_event(feature, rule, scenario, ty, ev, retries)
            }
            Scenario::Background(step, ev, retries) => self.expand_step_event(
                feature, rule, scenario, &step, ev, retries, false, cli,
            ),
            Scenario::Step(step, ev, retries) => self.expand_step_event(
                feature, rule, scenario, &step, ev, retries, false, cli,
            ),
        }
    }

    /// Converts [`event::Hook`] into [`LibTestJsonEvent`]s.
    fn expand_hook_event(
        &mut self,
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        hook: event::HookType,
        ev: event::Hook<W>,
        retries: Option<Retries>,
    ) -> Vec<LibTestJsonEvent> {
        match ev {
            event::Hook::Started | event::Hook::Passed => Vec::new(),
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
                    TestEvent::failed(name)
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

    /// Converts [`event::Step`] into [`LibTestJsonEvent`]s.
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
            Step::Started => TestEvent::started(name),
            Step::Passed(_, loc) => {
                self.passed += 1;

                let event = TestEvent::ok(name);
                if let Some(loc) = loc.filter(|_| cli.show_output) {
                    event.with_stdout(format!(
                        "{}:{}:{}",
                        loc.path, loc.line, loc.column,
                    ))
                } else {
                    event
                }
            }
            Step::Skipped => {
                self.ignored += 1;

                TestEvent::ignored(name)
            }
            Step::Failed(_, loc, world, err) => {
                self.failed += 1;

                TestEvent::failed(name).with_stdout(format!(
                    "{}{err}{}",
                    loc.map(|l| format!(
                        "{}:{}:{}\n",
                        l.path, l.line, l.column,
                    ))
                    .unwrap_or_default(),
                    world.map(|w| format!("\n{:#?}", w)).unwrap_or_default(),
                ))
            }
        }
        .into();

        vec![ev]
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
        let feature = format!(
            "{} {}",
            feature.name,
            feature.path.as_ref().and_then(|p| p.to_str()).map_or_else(
                || {
                    self.features_without_path += 1;
                    self.features_without_path.to_string()
                },
                |s| s.escape_default().to_string()
            ),
        );
        let rule = rule
            .as_ref()
            .map(|r| format!("{}: {} {}", r.position.line, r.keyword, r.name));
        let scenario = format!(
            "{}: {} {} | {retries:?}",
            scenario.position.line, scenario.keyword, scenario.name,
        );
        let step = match step {
            Either::Left(hook) => format!("{hook} hook"),
            Either::Right((step, is_bg)) => format!(
                "{}: {}{}{}",
                step.position.line,
                is_bg.then_some("Background ").unwrap_or_default(),
                step.keyword,
                step.value,
            ),
        };

        [Some(feature), rule, Some(scenario), Some(step)]
            .into_iter()
            .flatten()
            .join("::")
    }
}

/// Indicator, whether [`Step`] is [`Background`] or not.
///
/// [`Background`]: event::Scenario::Background
/// [`Step`]: gherkin::Step
type IsBackground = bool;

impl<W: World, O: io::Write> writer::NonTransforming for Libtest<W, O> {}

impl<W: World + Debug, O: io::Write> writer::Failure<W> for Libtest<W, O> {
    fn failed_steps(&self) -> usize {
        self.failed
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
/// This format isn't stable, so this implementation uses [this PR][1] as a
/// reference point.
///
/// [1]: https://github.com/rust-lang/rust/pull/46450
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

    // TODO: figure out a way to actually report this.
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
    #[allow(dead_code)]
    Timeout(TestEventInner),
}

impl TestEvent {
    /// Creates a new [`TestEvent::Started`].
    const fn started(name: String) -> Self {
        Self::Started(TestEventInner::new(name))
    }

    /// Creates a new [`TestEvent::Ok`].
    const fn ok(name: String) -> Self {
        Self::Ok(TestEventInner::new(name))
    }

    /// Creates a new [`TestEvent::Failed`].
    const fn failed(name: String) -> Self {
        Self::Failed(TestEventInner::new(name))
    }

    /// Creates a new [`TestEvent::Ignored`].
    const fn ignored(name: String) -> Self {
        Self::Ignored(TestEventInner::new(name))
    }

    /// Creates a new [`TestEvent::Timeout`].
    #[allow(dead_code)]
    const fn timeout(name: String) -> Self {
        Self::Timeout(TestEventInner::new(name))
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

/// Inner value of [`TestEvent`].
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
}

impl TestEventInner {
    /// Creates a new [`TestEventInner`].
    const fn new(name: String) -> Self {
        Self {
            name,
            stdout: None,
            stderr: None,
        }
    }

    /// Adds a [`TestEventInner::stdout`].
    #[allow(clippy::missing_const_for_fn)] // false positive: drop in const
    fn with_stdout(mut self, stdout: String) -> Self {
        self.stdout = Some(stdout);
        self
    }
}
