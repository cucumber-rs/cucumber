//! Event handling logic for JUnit XML writer.

use std::{fmt::Debug, io, mem, time::SystemTime};

use junit_report::{Duration, TestSuiteBuilder};

use crate::{
    Event, World,
    event::{self, Scenario},
    writer::basic::trim_path,
};

use super::{error_handler::ErrorHandler, test_case_builder::JUnitTestCaseBuilder};

/// Advice phrase to use in panic messages of incorrect [events][1] ordering.
///
/// [1]: event::Scenario
const WRAP_ADVICE: &str = "Consider wrapping `Writer` into `writer::Normalize`";

/// Handles different types of events for JUnit XML generation.
#[derive(Debug)]
pub struct EventHandler<W, Out: io::Write> {
    test_case_builder: JUnitTestCaseBuilder<W>,
    _phantom: std::marker::PhantomData<Out>,
}

impl<W: World + Debug, Out: io::Write> EventHandler<W, Out> {
    /// Creates a new [`EventHandler`] with the specified test case builder.
    #[must_use]
    pub const fn new(test_case_builder: JUnitTestCaseBuilder<W>) -> Self {
        Self {
            test_case_builder,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Handles a feature started event by creating a new test suite.
    pub fn handle_feature_started(
        feat: &gherkin::Feature,
        meta: Event<()>,
    ) -> junit_report::TestSuite {
        TestSuiteBuilder::new(&format!(
            "Feature: {}{}",
            &feat.name,
            feat.path
                .as_deref()
                .and_then(|p| p.to_str().map(trim_path))
                .map(|path| format!(": {path}"))
                .unwrap_or_default(),
        ))
        .set_timestamp(meta.at.into())
        .build()
    }

    /// Handles a scenario event by updating the scenario state and events collection.
    pub fn handle_scenario_event(
        &self,
        feat: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        sc: &gherkin::Scenario,
        ev: event::RetryableScenario<W>,
        meta: Event<()>,
        scenario_started_at: &mut Option<SystemTime>,
        events: &mut Vec<event::RetryableScenario<W>>,
        suite: &mut Option<junit_report::TestSuite>,
    ) {
        match &ev.event {
            Scenario::Started => {
                *scenario_started_at = Some(meta.at);
                events.push(ev);
            }
            Scenario::Log(_)
            | Scenario::Hook(..)
            | Scenario::Background(..)
            | Scenario::Step(..) => {
                events.push(ev);
            }
            Scenario::Finished => {
                let started_at = scenario_started_at.take().unwrap_or_else(|| {
                    panic!(
                        "no `Started` event for `Scenario` \"{}\"\n{WRAP_ADVICE}",
                        sc.name,
                    )
                });

                let duration = JUnitTestCaseBuilder::<W>::calculate_duration(started_at, meta.at, sc);
                let scenario_events = mem::take(events);
                let test_case = self.test_case_builder.build_test_case(
                    feat,
                    rule,
                    sc,
                    &scenario_events,
                    duration,
                );

                suite
                    .as_mut()
                    .unwrap_or_else(|| {
                        panic!(
                            "no `TestSuit` for `Scenario` \"{}\"\n{WRAP_ADVICE}",
                            sc.name,
                        )
                    })
                    .add_testcase(test_case);
            }
        }
    }

    /// Handles a feature finished event by returning the completed test suite.
    pub fn handle_feature_finished(
        feat: &gherkin::Feature,
        suite: Option<junit_report::TestSuite>,
    ) -> junit_report::TestSuite {
        suite.unwrap_or_else(|| {
            eprintln!(
                "Warning: no `TestSuit` for `Feature` \"{}\"\n{WRAP_ADVICE}",
                feat.name,
            );
            TestSuiteBuilder::new(&format!("Feature: {}", feat.name)).build()
        })
    }

    /// Handles a cucumber finished event by writing the XML report.
    pub fn handle_cucumber_finished(
        report: &mut junit_report::Report,
        output: &mut Out,
    ) {
        report.write_xml(output).unwrap_or_else(|e| {
            eprintln!("Warning: failed to write XML: {e}");
        });
    }

    /// Handles parser errors by delegating to the error handler.
    pub fn handle_parser_error(
        report: &mut junit_report::Report,
        err: &crate::parser::Error,
    ) {
        ErrorHandler::handle_error(report, err);
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use gherkin::{Feature, LineCol, Scenario};
    use junit_report::Report;

    use crate::{
        Event,
        event::{self, Step},
        parser,
        writer::Verbosity,
    };

    use super::*;

    #[derive(Debug)]
    struct TestWorld;

    impl World for TestWorld {
        type Error = String;

        async fn new() -> Result<Self, Self::Error> {
            Ok(TestWorld)
        }
    }

    fn create_test_feature() -> Feature {
        Feature {
            name: "Test Feature".to_string(),
            description: None,
            background: None,
            scenarios: vec![],
            rules: vec![],
            tags: vec![],
            position: LineCol { line: 1, col: 1 },
            path: Some(PathBuf::from("/test/features/example.feature")),
        }
    }

    fn create_test_scenario() -> Scenario {
        Scenario {
            name: "Test Scenario".to_string(),
            description: None,
            steps: vec![],
            tags: vec![],
            position: LineCol { line: 5, col: 3 },
            examples: vec![],
        }
    }

    fn create_test_event() -> Event<()> {
        Event {
            value: (),
            at: SystemTime::UNIX_EPOCH,
        }
    }

    #[test]
    fn handles_feature_started_creates_test_suite() {
        let feature = create_test_feature();
        let meta = create_test_event();

        let suite = EventHandler::<TestWorld, Vec<u8>>::handle_feature_started(&feature, meta);

        assert_eq!(suite.name(), "Feature: Test Feature: example.feature");
        assert_eq!(suite.testcases().len(), 0);
    }

    #[test]
    fn handles_feature_started_without_path() {
        let mut feature = create_test_feature();
        feature.path = None;
        let meta = create_test_event();

        let suite = EventHandler::<TestWorld, Vec<u8>>::handle_feature_started(&feature, meta);

        assert_eq!(suite.name(), "Feature: Test Feature");
    }

    #[test]
    fn handles_scenario_started_sets_timestamp() {
        let handler = EventHandler::new(JUnitTestCaseBuilder::<TestWorld>::new(Verbosity::Default));
        let feature = create_test_feature();
        let scenario = create_test_scenario();
        let meta = create_test_event();
        let event = event::RetryableScenario {
            event: event::Scenario::Started,
            retries: None,
        };

        let mut scenario_started_at = None;
        let mut events = vec![];
        let mut suite = Some(
            EventHandler::<TestWorld, Vec<u8>>::handle_feature_started(&feature, meta)
        );

        handler.handle_scenario_event(
            &feature,
            None,
            &scenario,
            event,
            meta,
            &mut scenario_started_at,
            &mut events,
            &mut suite,
        );

        assert_eq!(scenario_started_at, Some(SystemTime::UNIX_EPOCH));
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn handles_scenario_step_adds_to_events() {
        let handler = EventHandler::new(JUnitTestCaseBuilder::<TestWorld>::new(Verbosity::Default));
        let feature = create_test_feature();
        let scenario = create_test_scenario();
        let meta = create_test_event();
        let step_event = event::RetryableScenario {
            event: event::Scenario::Step(
                gherkin::Step {
                    keyword: "Given".to_string(),
                    ty: gherkin::StepType::Given,
                    value: "I have a step".to_string(),
                    docstring: None,
                    table: None,
                    position: LineCol { line: 6, col: 5 },
                },
                Step::Passed {
                    captures: regex::Regex::new("").unwrap().capture_locations(),
                    location: None,
                },
            ),
            retries: None,
        };

        let mut scenario_started_at = Some(SystemTime::UNIX_EPOCH);
        let mut events = vec![];
        let mut suite = Some(
            EventHandler::<TestWorld, Vec<u8>>::handle_feature_started(&feature, meta)
        );

        handler.handle_scenario_event(
            &feature,
            None,
            &scenario,
            step_event,
            meta,
            &mut scenario_started_at,
            &mut events,
            &mut suite,
        );

        assert_eq!(events.len(), 1);
        assert!(matches!(events[0].event, event::Scenario::Step(_, _)));
    }

    #[test]
    fn handles_scenario_finished_creates_test_case() {
        let handler = EventHandler::new(JUnitTestCaseBuilder::<TestWorld>::new(Verbosity::Default));
        let feature = create_test_feature();
        let scenario = create_test_scenario();
        let meta = Event {
            value: (),
            at: SystemTime::UNIX_EPOCH + std::time::Duration::from_millis(100),
        };
        let finished_event = event::RetryableScenario {
            event: event::Scenario::Finished,
            retries: None,
        };

        let mut scenario_started_at = Some(SystemTime::UNIX_EPOCH);
        let mut events = vec![
            event::RetryableScenario {
                event: event::Scenario::Started,
                retries: None,
            },
        ];
        let mut suite = Some(
            EventHandler::<TestWorld, Vec<u8>>::handle_feature_started(&feature, create_test_event())
        );

        handler.handle_scenario_event(
            &feature,
            None,
            &scenario,
            finished_event,
            meta,
            &mut scenario_started_at,
            &mut events,
            &mut suite,
        );

        assert!(scenario_started_at.is_none());
        assert!(events.is_empty());
        assert_eq!(suite.as_ref().unwrap().testcases().len(), 1);
    }

    #[test]
    fn handles_feature_finished_returns_suite() {
        let feature = create_test_feature();
        let suite = Some(
            EventHandler::<TestWorld, Vec<u8>>::handle_feature_started(&feature, create_test_event())
        );

        let result = EventHandler::<TestWorld, Vec<u8>>::handle_feature_finished(&feature, suite);

        assert_eq!(result.name(), "Feature: Test Feature: example.feature");
    }

    #[test]
    fn handles_feature_finished_with_missing_suite() {
        let feature = create_test_feature();

        let result = EventHandler::<TestWorld, Vec<u8>>::handle_feature_finished(&feature, None);

        assert_eq!(result.name(), "Feature: Test Feature");
        assert_eq!(result.testcases().len(), 0);
    }

    #[test]
    fn handles_cucumber_finished_writes_xml() {
        let mut report = Report::new();
        let mut output = Vec::new();

        EventHandler::<TestWorld, Vec<u8>>::handle_cucumber_finished(&mut report, &mut output);

        assert!(!output.is_empty());
        let xml_str = String::from_utf8(output).unwrap();
        assert!(xml_str.contains("<?xml"));
        assert!(xml_str.contains("<testsuites"));
    }

    #[test]
    fn handles_parser_error_adds_error_suite() {
        let mut report = Report::new();
        let parse_error = gherkin::ParseFileError::Reading {
            path: PathBuf::from("/test/broken.feature"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "File not found"),
        };
        let parser_error = parser::Error::Parsing(Box::new(parse_error));

        EventHandler::<TestWorld, Vec<u8>>::handle_parser_error(&mut report, &parser_error);

        assert_eq!(report.testsuites().len(), 1);
        assert_eq!(report.testsuites()[0].name(), "Errors");
        assert_eq!(report.testsuites()[0].testcases().len(), 1);
    }

    #[test]
    #[should_panic(expected = "no `Started` event for `Scenario`")]
    fn panics_on_finished_without_started() {
        let handler = EventHandler::new(JUnitTestCaseBuilder::<TestWorld>::new(Verbosity::Default));
        let feature = create_test_feature();
        let scenario = create_test_scenario();
        let meta = create_test_event();
        let finished_event = event::RetryableScenario {
            event: event::Scenario::Finished,
            retries: None,
        };

        let mut scenario_started_at = None;
        let mut events = vec![];
        let mut suite = Some(
            EventHandler::<TestWorld, Vec<u8>>::handle_feature_started(&feature, meta)
        );

        handler.handle_scenario_event(
            &feature,
            None,
            &scenario,
            finished_event,
            meta,
            &mut scenario_started_at,
            &mut events,
            &mut suite,
        );
    }

    #[test]
    #[should_panic(expected = "no `TestSuit` for `Scenario`")]
    fn panics_on_scenario_without_suite() {
        let handler = EventHandler::new(JUnitTestCaseBuilder::<TestWorld>::new(Verbosity::Default));
        let feature = create_test_feature();
        let scenario = create_test_scenario();
        let meta = create_test_event();
        let finished_event = event::RetryableScenario {
            event: event::Scenario::Finished,
            retries: None,
        };

        let mut scenario_started_at = Some(SystemTime::UNIX_EPOCH);
        let mut events = vec![];
        let mut suite = None;

        handler.handle_scenario_event(
            &feature,
            None,
            &scenario,
            finished_event,
            meta,
            &mut scenario_started_at,
            &mut events,
            &mut suite,
        );
    }
}