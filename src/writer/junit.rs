//! [`JUnit`][1] [`Writer`] implementation.
//!
//! [1]: https://llg.cubic.org/docs/junit/

use std::{fmt::Debug, io, mem, path::Path, time::SystemTime};

use async_trait::async_trait;
use chrono::Duration;
use junit_report::{
    Report, TestCase, TestCaseBuilder, TestSuite, TestSuiteBuilder,
};

use crate::{
    cli, event, parser,
    writer::{
        self,
        basic::{coerce_error, Coloring},
        out::{WritableString, WriteStr},
    },
    Event, World, Writer,
};

/// [`JUnit`][1] [`Writer`] implementation outputting `XML` to [`io::Write`]
/// implementor.
///
/// For correct work should be wrapped into [`writer::Summarized`].
///
/// [1]: https://llg.cubic.org/docs/junit/
#[derive(Debug)]
pub struct JUnit<W, Out: WriteStr> {
    /// [`io::Write`] implementor to output `XML` into.
    output: Out,

    /// [`JUnit`][1] [`Report`].
    ///
    /// [1]: https://llg.cubic.org/docs/junit/
    report: Report,

    /// Current [`JUnit`][1] [`TestSuite`].
    ///
    /// [1]: https://llg.cubic.org/docs/junit/
    suit: Option<TestSuite>,

    /// Current [`Scenario`] start [`SystemTime`].
    ///
    /// [`Scenario`]: gherkin::Scenario
    scenario_started_at: Option<SystemTime>,

    /// Current [`Scenario`] [`events`][1].
    ///
    /// [1]: event::Scenario
    /// [`Scenario`]: gherkin::Scenario
    events: Vec<event::Scenario<W>>,
}

#[async_trait(?Send)]
impl<W, Out> Writer<W> for JUnit<W, Out>
where
    W: World + Debug,
    Out: WriteStr,
{
    type Cli = cli::Empty;

    #[allow(clippy::unused_async)] // false positive: #[async_trait]
    async fn handle_event(
        &mut self,
        ev: parser::Result<Event<event::Cucumber<W>>>,
        _: &Self::Cli,
    ) {
        use event::{Cucumber, Feature, Rule};

        match ev.map(Event::split) {
            Err(err) => self.handle_error(&err),
            Ok((Cucumber::Started, _)) => {}
            Ok((Cucumber::Feature(feat, ev), meta)) => match ev {
                Feature::Started => {
                    self.suit = Some(
                        TestSuiteBuilder::new(&format!(
                            "Feature: {}{}",
                            &feat.name,
                            feat.path
                                .as_deref()
                                .and_then(Path::to_str)
                                .map(|path| format!(": {}", path))
                                .unwrap_or_default(),
                        ))
                        .set_timestamp(meta.at.into())
                        .build(),
                    );
                }
                Feature::Rule(_, Rule::Started | Rule::Finished) => {}
                Feature::Rule(r, Rule::Scenario(sc, ev)) => {
                    self.handle_scenario_event(&feat, Some(&r), &sc, ev, meta);
                }
                Feature::Scenario(sc, ev) => {
                    self.handle_scenario_event(&feat, None, &sc, ev, meta);
                }
                Feature::Finished => {
                    let suite = self.suit.take().unwrap_or_else(|| {
                        panic!(
                            "No TestSuit for Feature \"{}\"\n\
                             Consider wrapping Writer in writer::Normalized",
                            feat.name,
                        )
                    });
                    self.report.add_testsuite(suite);
                }
            },
            Ok((Cucumber::Finished, _)) => {
                self.report
                    .write_xml(&mut self.output)
                    .unwrap_or_else(|e| panic!("Failed to write XML: {}", e));
            }
        }
    }
}

impl<W: Debug, Out: WriteStr> JUnit<W, Out> {
    /// Creates a new [`JUnit`] [`Writer`] outputting `XML` into `Out`.
    pub fn new(output: Out) -> Self {
        Self {
            output,
            report: Report::new(),
            suit: None,
            scenario_started_at: None,
            events: Vec::new(),
        }
    }

    /// Handles [`parser::Error`].
    fn handle_error(&mut self, err: &parser::Error) {
        let (name, ty) = match err {
            parser::Error::Parsing(err) => {
                let path = match err.as_ref() {
                    gherkin::ParseFileError::Reading { path, .. }
                    | gherkin::ParseFileError::Parsing { path, .. } => path,
                };

                (
                    format!(
                        "Feature{}",
                        path.to_str()
                            .map(|p| format!(": {}", p))
                            .unwrap_or_default(),
                    ),
                    "Parser Error",
                )
            }
            parser::Error::ExampleExpansion(err) => (
                format!(
                    "Feature: {}{}:{}",
                    err.path
                        .as_deref()
                        .and_then(Path::to_str)
                        .map(|p| format!("{}:", p))
                        .unwrap_or_default(),
                    err.pos.line,
                    err.pos.col,
                ),
                "Example Expansion Error",
            ),
        };

        self.report.add_testsuite(
            TestSuiteBuilder::new("Errors")
                .add_testcase(TestCase::failure(
                    &name,
                    Duration::zero(),
                    ty,
                    &format!("{}", err),
                ))
                .build(),
        );
    }

    /// Handles [`event::Scenario`].
    fn handle_scenario_event(
        &mut self,
        feat: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        sc: &gherkin::Scenario,
        ev: event::Scenario<W>,
        meta: Event<()>,
    ) {
        use event::Scenario;

        match ev {
            Scenario::Started => {
                self.scenario_started_at = Some(meta.at);
                self.events.push(Scenario::Started);
            }
            ev
            @
            (Scenario::Hook(..)
            | Scenario::Background(..)
            | Scenario::Step(..)) => {
                self.events.push(ev);
            }
            Scenario::Finished => {
                let dur = self.scenario_duration(meta.at, sc);
                let events = mem::take(&mut self.events);
                let case = Self::test_case(feat, rule, sc, &events, dur);

                self.suit
                    .as_mut()
                    .unwrap_or_else(|| {
                        panic!(
                            "No TestSuit for Feature \"{}\"\n\
                             Consider wrapping Writer in writer::Normalized",
                            feat.name,
                        )
                    })
                    .add_testcase(case);
            }
        }
    }

    /// Return [`TestCase`] on [`event::Scenario::Finished`].
    fn test_case(
        feat: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        sc: &gherkin::Scenario,
        events: &[event::Scenario<W>],
        duration: Duration,
    ) -> TestCase {
        use event::{Hook, HookType, Scenario, Step};

        let last_event = events
            .iter()
            .rev()
            .find(|ev| {
                !matches!(
                    ev,
                    Scenario::Hook(
                        HookType::After,
                        Hook::Passed | Hook::Started
                    )
                )
            })
            .unwrap_or_else(|| {
                panic!(
                    "No events for Scenario \"{}\"\n\
                             Consider wrapping Writer in writer::Normalized",
                    sc.name,
                )
            });

        let case_name = format!(
            "{}Scenario: {}: {}{}:{}",
            rule.map(|r| format!("Rule: {}: ", r.name))
                .unwrap_or_default(),
            sc.name,
            feat.path
                .as_ref()
                .and_then(|p| p.to_str())
                .map(|path| format!("{}:", path))
                .unwrap_or_default(),
            sc.position.line,
            sc.position.col,
        );

        let mut case = match last_event {
            Scenario::Started
            | Scenario::Hook(_, Hook::Started | Hook::Passed)
            | Scenario::Background(_, Step::Started | Step::Passed(_))
            | Scenario::Step(_, Step::Started | Step::Passed(_)) => {
                TestCaseBuilder::success(&case_name, duration).build()
            }
            Scenario::Background(_, Step::Skipped)
            | Scenario::Step(_, Step::Skipped) => {
                TestCaseBuilder::skipped(&case_name).build()
            }
            Scenario::Hook(_, Hook::Failed(_, e)) => TestCaseBuilder::failure(
                &case_name,
                duration,
                "Hook Panicked",
                coerce_error(e).as_ref(),
            )
            .build(),
            Scenario::Background(_, Step::Failed(_, _, e))
            | Scenario::Step(_, Step::Failed(_, _, e)) => {
                TestCaseBuilder::failure(
                    &case_name,
                    duration,
                    "Step Panicked",
                    &format!("{}", e),
                )
                .build()
            }
            Scenario::Finished => {
                panic!(
                    "Duplicated Finished event for Scenario: \"{}\"",
                    sc.name,
                );
            }
        };

        let mut basic_wr = writer::Basic::new(
            WritableString(String::new()),
            Coloring::Never,
            false,
        );
        let output = events
            .iter()
            .map(|ev| {
                basic_wr.scenario(feat, sc, ev)?;
                Ok(mem::take(&mut **basic_wr))
            })
            .collect::<io::Result<String>>()
            .unwrap_or_else(|e| panic!("Failed to write: {}", e));

        case.set_system_out(&output);

        case
    }

    /// Returns [`Duration`] on [`event::Scenario::Finished`].
    fn scenario_duration(
        &mut self,
        ended: SystemTime,
        sc: &gherkin::Scenario,
    ) -> Duration {
        let started_at = self.scenario_started_at.take().unwrap_or_else(|| {
            panic!(
                "No Started event for Scenario \"{}\"\n\
                 Consider wrapping Writer in writer::Normalized",
                sc.name,
            )
        });
        Duration::from_std(ended.duration_since(started_at).unwrap_or_else(
            |e| {
                panic!(
                    "Failed to compute Duration between {:?} and {:?}: {}",
                    ended, started_at, e,
                )
            },
        ))
        .unwrap_or_else(|e| {
            panic!(
                "Failed to covert std::time::Duration to chrono::Duration: {}",
                e,
            )
        })
    }
}
