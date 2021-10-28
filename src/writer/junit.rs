//! TODO

use std::fmt::Debug;
use std::time::SystemTime;

use async_trait::async_trait;
use junit_report::{
    Duration, Report, TestCase, TestCaseBuilder, TestSuite, TestSuiteBuilder,
};

use crate::{cli, event, parser, Event, World, Writer};

/// TODO
#[derive(Debug)]
pub struct JUnit<W> {
    /// TODO
    report: Report,

    /// TODO
    suit: Option<TestSuite>,

    /// TODO
    scenario_started_at: Option<SystemTime>,

    /// TODO
    event: Option<event::Scenario<W>>,
}

impl<W> Default for JUnit<W> {
    fn default() -> Self {
        Self {
            report: Report::new(),
            suit: None,
            scenario_started_at: None,
            event: None,
        }
    }
}

#[async_trait(?Send)]
impl<W: World + Debug> Writer<W> for JUnit<W> {
    type Cli = cli::Empty;

    #[allow(clippy::unused_async)] // false positive: #[async_trait]
    async fn handle_event(
        &mut self,
        ev: parser::Result<Event<event::Cucumber<W>>>,
        _: &Self::Cli,
    ) {
        use event::{Cucumber, Feature, Rule};

        match ev.map(Event::split) {
            Err(e) => {
                self.report.add_testsuite(
                    TestSuiteBuilder::new("")
                        .add_testcase(TestCase::failure(
                            "",
                            Duration::zero(),
                            "",
                            &format!("{}", e),
                        ))
                        .build(),
                );
            }
            Ok((Cucumber::Started, _meta)) => {}
            Ok((Cucumber::Feature(feat, ev), meta)) => match ev {
                Feature::Started => {
                    self.suit = Some(
                        TestSuiteBuilder::new(&feat.name)
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
            Ok((Cucumber::Finished, _meta)) => {
                self.report
                    .write_xml(std::io::stdout())
                    .unwrap_or_else(|e| panic!("Failed to write XML: {}", e));
            }
        }
    }
}

impl<W: Debug> JUnit<W> {
    /// TODO
    fn handle_scenario_event(
        &mut self,
        feat: &gherkin::Feature,
        _rule: Option<&gherkin::Rule>,
        sc: &gherkin::Scenario,
        ev: event::Scenario<W>,
        meta: Event<()>,
    ) {
        use event::{Hook, Scenario, Step};

        match ev {
            Scenario::Started => {
                self.scenario_started_at = Some(meta.at);
                self.event = Some(Scenario::Started);
            }
            ev
            @
            (Scenario::Hook(..)
            | Scenario::Background(..)
            | Scenario::Step(..)) => {
                self.event = Some(ev);
            }
            Scenario::Finished => {
                let last_sc_event = self.event.take().unwrap_or_else(|| {
                    panic!(
                        "No TestSuit for Feature \"{}\"\n\
                         Consider wrapping Writer in writer::Normalized",
                        sc.name,
                    )
                });

                let dbg = format!("{:?}", last_sc_event);
                let started_at =
                    self.scenario_started_at.take().unwrap_or_else(|| {
                        panic!(
                            "No event::Scenario::Started for Feature \"{}\"\n\
                             Consider wrapping Writer in writer::Normalized",
                            sc.name,
                        )
                    });
                let dur = Duration::from_std(
                    meta.at.duration_since(started_at).unwrap_or_else(|e| {
                        panic!(
                            "Failed to compute Duration between \
                             {:?} and {:?}: {}",
                            meta.at, started_at, e,
                        )
                    }),
                )
                .unwrap_or_else(|e| {
                    panic!(
                        "Failed to covert std::time::Duration to \
                         chrono::Duration: {}",
                        e,
                    )
                });

                let mut case = match last_sc_event {
                    Scenario::Started
                    | Scenario::Hook(_, Hook::Started | Hook::Passed)
                    | Scenario::Background(
                        _,
                        Step::Started | Step::Passed(_),
                    )
                    | Scenario::Step(_, Step::Started | Step::Passed(_)) => {
                        TestCaseBuilder::success(&sc.name, dur).build()
                    }
                    Scenario::Background(_, Step::Skipped)
                    | Scenario::Step(_, Step::Skipped) => {
                        TestCaseBuilder::skipped(&sc.name).build()
                    }
                    Scenario::Hook(_, Hook::Failed(..))
                    | Scenario::Background(_, Step::Failed(..))
                    | Scenario::Step(_, Step::Failed(..)) => {
                        TestCaseBuilder::failure(&sc.name, dur, "", "").build()
                    }
                    Scenario::Finished => {
                        panic!(
                            "Duplicated Finished event for Scenario: \"{}\"",
                            sc.name,
                        );
                    }
                };

                case.set_system_out(&dbg);

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
}
