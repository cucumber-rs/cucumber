// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! [JUnit XML report][1] [`Writer`] implementation.
//!
//! [1]: https://llg.cubic.org/docs/junit

use std::{fmt::Debug, io, mem, path::Path, time::SystemTime};

use async_trait::async_trait;
use junit_report::{
    Duration, Report, TestCase, TestCaseBuilder, TestSuite, TestSuiteBuilder,
};

use crate::{
    cli, event, parser,
    writer::{
        self,
        basic::{coerce_error, Coloring},
        out::WritableString,
        Ext as _,
    },
    Event, World, Writer,
};

/// Advice phrase to use in panic messages of incorrect [events][1] ordering.
///
/// [1]: event::Scenario
const WRAP_ADVICE: &str =
    "Consider wrapping `Writer` into `writer::Normalized`";

/// [JUnit XML report][1] [`Writer`] implementation outputting XML to an
/// [`io::Write`] implementor.
///
/// Should be wrapped into [`writer::Normalized`] to work correctly, otherwise
/// will panic in runtime as won't be able to form correct
/// [JUnit `testsuite`s][1].
///
/// [1]: https://llg.cubic.org/docs/junit
#[derive(Debug)]
pub struct JUnit<W, Out: io::Write> {
    /// [`io::Write`] implementor to output XML report into.
    output: Out,

    /// [JUnit XML report][1].
    ///
    /// [1]: https://llg.cubic.org/docs/junit
    report: Report,

    /// Current [JUnit `testsuite`][1].
    ///
    /// [1]: https://llg.cubic.org/docs/junit
    suit: Option<TestSuite>,

    /// [`SystemTime`] when the current [`Scenario`] has started.
    ///
    /// [`Scenario`]: gherkin::Scenario
    scenario_started_at: Option<SystemTime>,

    /// Current [`Scenario`] [events][1].
    ///
    /// [`Scenario`]: gherkin::Scenario
    /// [1]: event::Scenario
    events: Vec<event::Scenario<W>>,
}

#[async_trait(?Send)]
impl<W, Out> Writer<W> for JUnit<W, Out>
where
    W: World + Debug,
    Out: io::Write,
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
                            "No `TestSuit` for `Feature` \"{}\"\n{}",
                            feat.name, WRAP_ADVICE,
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

impl<W: Debug, Out: io::Write> JUnit<W, Out> {
    /// Creates a new normalized [`JUnit`] [`Writer`] outputting XML report into
    /// the given `output`.
    #[must_use]
    pub fn new(output: Out) -> writer::Normalized<W, Self>
    where
        W: World,
    {
        Self::raw(output).normalized()
    }

    /// Creates a new raw and unnormalized [`JUnit`] [`Writer`] outputting XML
    /// report into the given `output`.
    ///
    /// # Warning
    ///
    /// It may panic in runtime as won't be able to form correct
    /// [JUnit `testsuite`s][1] from unordered [`Cucumber` events][2].
    ///
    /// Use it only if you know what you're doing. Otherwise, consider using
    /// [`JUnit::new()`] which creates an already [`Normalized`] version of
    /// [`JUnit`] [`Writer`].
    ///
    /// [`Normalized`]: writer::Normalized
    /// [1]: https://llg.cubic.org/docs/junit
    /// [2]: crate::event::Cucumber
    #[must_use]
    pub fn raw(output: Out) -> Self {
        Self {
            output,
            report: Report::new(),
            suit: None,
            scenario_started_at: None,
            events: Vec::new(),
        }
    }

    /// Handles the given [`parser::Error`].
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
                    Duration::ZERO,
                    ty,
                    &format!("{}", err),
                ))
                .build(),
        );
    }

    /// Handles the given [`event::Scenario`].
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
            Scenario::Hook(..)
            | Scenario::Background(..)
            | Scenario::Step(..) => {
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
                            "No `TestSuit` for `Scenario` \"{}\"\n{}",
                            sc.name, WRAP_ADVICE,
                        )
                    })
                    .add_testcase(case);
            }
        }
    }

    /// Forms a [`TestCase`] on [`event::Scenario::Finished`].
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
                        Hook::Passed | Hook::Started,
                    ),
                )
            })
            .unwrap_or_else(|| {
                panic!(
                    "No events for `Scenario` \"{}\"\n{}",
                    sc.name, WRAP_ADVICE,
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
                    &e.to_string(),
                )
                .build()
            }
            Scenario::Finished => {
                panic!(
                    "Duplicated `Finished` event for `Scenario`: \"{}\"\n{}",
                    sc.name, WRAP_ADVICE,
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
            .unwrap_or_else(|e| {
                panic!("Failed to write with `writer::Basic`: {}", e)
            });

        case.set_system_out(&output);

        case
    }

    /// Returns [`Scenario`]'s [`Duration`] on [`event::Scenario::Finished`].
    ///
    /// [`Scenario`]: gherkin::Scenario
    fn scenario_duration(
        &mut self,
        ended: SystemTime,
        sc: &gherkin::Scenario,
    ) -> Duration {
        let started_at = self.scenario_started_at.take().unwrap_or_else(|| {
            panic!(
                "No `Started` event for `Scenario` \"{}\"\n{}",
                sc.name, WRAP_ADVICE,
            )
        });
        Duration::try_from(ended.duration_since(started_at).unwrap_or_else(
            |e| {
                panic!(
                    "Failed to compute duration between {:?} and {:?}: {}",
                    ended, started_at, e,
                )
            },
        ))
        .unwrap_or_else(|e| {
            panic!(
                "Cannot covert `std::time::Duration` to `time::Duration`: {}",
                e,
            )
        })
    }
}
