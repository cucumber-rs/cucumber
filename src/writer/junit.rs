//! TODO

use std::{fmt::Debug, io, time::SystemTime};

use async_trait::async_trait;
use junit_report::{
    Duration, Report, TestCase, TestCaseBuilder, TestSuite, TestSuiteBuilder,
};

use crate::{
    cli, event, parser,
    writer::{self, basic::Coloring, out::WriteStr},
    Event, World, Writer,
};

/// TODO
#[derive(Debug)]
pub struct JUnit<W, Out: WriteStr> {
    /// TODO
    output: Out,

    /// TODO
    report: Report,

    /// TODO
    suit: Option<TestSuite>,

    /// TODO
    scenario_started_at: Option<SystemTime>,

    /// TODO
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
            Err(e) => {
                self.report.add_testsuite(
                    TestSuiteBuilder::new("Parser Error")
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
                        TestSuiteBuilder::new(&format!(
                            "Feature: {}{}",
                            &feat.name,
                            feat.path
                                .as_ref()
                                .and_then(|p| p.to_str())
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
            Ok((Cucumber::Finished, _meta)) => {
                self.report
                    .write_xml(&mut self.output)
                    .unwrap_or_else(|e| panic!("Failed to write XML: {}", e));
            }
        }
    }
}

impl<W: Debug, Out: WriteStr> JUnit<W, Out> {
    /// TODO
    pub fn new(output: Out) -> Self {
        Self {
            output,
            report: Report::new(),
            suit: None,
            scenario_started_at: None,
            events: Vec::new(),
        }
    }

    /// TODO
    #[allow(clippy::too_many_lines)]
    fn handle_scenario_event(
        &mut self,
        feat: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        sc: &gherkin::Scenario,
        ev: event::Scenario<W>,
        meta: Event<()>,
    ) {
        use event::{Hook, HookType, Scenario, Step};

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
                let events = std::mem::take(&mut self.events);
                let last_sc_event = events
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

                let mut basic_wr = writer::Basic::new(
                    WriteString(String::new()),
                    Coloring::Never,
                    false,
                );
                let output = events
                    .iter()
                    .map(|ev| {
                        basic_wr.scenario(feat, sc, ev)?;
                        Ok(std::mem::take(&mut (*basic_wr).0))
                    })
                    .collect::<io::Result<String>>()
                    .unwrap();

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

                let case_name = format!(
                    "{}Scenario: {}{}",
                    rule.map(|r| format!("Rule: {}: ", r.name))
                        .unwrap_or_default(),
                    sc.name,
                    feat.path
                        .as_ref()
                        .and_then(|p| p.to_str())
                        .map(|path| format!(
                            ": {}:{}:{}",
                            path, sc.position.line, sc.position.col,
                        ))
                        .unwrap_or_default(),
                );

                let mut case = match last_sc_event {
                    Scenario::Started
                    | Scenario::Hook(_, Hook::Started | Hook::Passed)
                    | Scenario::Background(
                        _,
                        Step::Started | Step::Passed(_),
                    )
                    | Scenario::Step(_, Step::Started | Step::Passed(_)) => {
                        TestCaseBuilder::success(&case_name, dur).build()
                    }
                    Scenario::Background(_, Step::Skipped)
                    | Scenario::Step(_, Step::Skipped) => {
                        TestCaseBuilder::skipped(&case_name).build()
                    }
                    Scenario::Hook(_, Hook::Failed(..))
                    | Scenario::Background(_, Step::Failed(..))
                    | Scenario::Step(_, Step::Failed(..)) => {
                        TestCaseBuilder::failure(&case_name, dur, "", "")
                            .build()
                    }
                    Scenario::Finished => {
                        panic!(
                            "Duplicated Finished event for Scenario: \"{}\"",
                            sc.name,
                        );
                    }
                };

                case.set_system_out(&output);

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

struct WriteString(String);

impl io::Write for WriteString {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.push_str(std::str::from_utf8(buf).unwrap());
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
