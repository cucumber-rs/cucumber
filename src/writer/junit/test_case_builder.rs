//! Test case building utilities for JUnit XML writer.

use std::{fmt::Debug, io, mem, time::SystemTime};

use junit_report::{Duration, TestCase, TestCaseBuilder};

use crate::{
    Event, World, 
    event::{self, Hook, HookType, Scenario, Step},
    writer::{
        Verbosity,
        basic::{Coloring, coerce_error, trim_path},
        out::WritableString,
    },
};

/// Advice phrase to use in panic messages of incorrect [events][1] ordering.
///
/// [1]: event::Scenario
const WRAP_ADVICE: &str = "Consider wrapping `Writer` into `writer::Normalize`";

/// Builder for creating JUnit test cases from Cucumber scenario events.
pub struct JUnitTestCaseBuilder<W> {
    verbosity: Verbosity,
    _phantom: std::marker::PhantomData<W>,
}

impl<W: World + Debug> JUnitTestCaseBuilder<W> {
    /// Creates a new [`JUnitTestCaseBuilder`] with the specified verbosity.
    #[must_use]
    pub const fn new(verbosity: Verbosity) -> Self {
        Self {
            verbosity,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Forms a [`TestCase`] from scenario events and metadata.
    pub fn build_test_case(
        &self,
        feat: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        sc: &gherkin::Scenario,
        events: &[event::RetryableScenario<W>],
        duration: Duration,
    ) -> TestCase {
        let last_event = self.find_last_meaningful_event(events, sc);
        let case_name = self.build_case_name(feat, rule, sc);
        let mut case = self.create_test_case(last_event, &case_name, duration);

        // Add system output using basic writer
        let output = self.generate_system_output(feat, sc, events);
        case.set_system_out(&output);

        case
    }

    /// Calculates scenario duration from start and end times.
    pub fn calculate_duration(
        started_at: SystemTime,
        ended_at: SystemTime,
        sc: &gherkin::Scenario,
    ) -> Duration {
        Duration::try_from(ended_at.duration_since(started_at).unwrap_or_else(
            |e| {
                panic!(
                    "failed to compute duration between {ended_at:?} and \
                     {started_at:?}: {e}",
                )
            },
        ))
        .unwrap_or_else(|e| {
            panic!(
                "cannot convert `std::time::Duration` to `time::Duration`: {e}",
            )
        })
    }

    /// Finds the last meaningful event (excluding logs and after hooks).
    fn find_last_meaningful_event<'a>(
        &self,
        events: &'a [event::RetryableScenario<W>],
        sc: &gherkin::Scenario,
    ) -> &'a event::RetryableScenario<W> {
        events
            .iter()
            .rev()
            .find(|ev| {
                !matches!(
                    ev.event,
                    Scenario::Log(_)
                        | Scenario::Hook(
                            HookType::After,
                            Hook::Passed | Hook::Started,
                        ),
                )
            })
            .unwrap_or_else(|| {
                panic!(
                    "no events for `Scenario` \"{}\"\n{WRAP_ADVICE}",
                    sc.name,
                )
            })
    }

    /// Builds the test case name from feature, rule, and scenario information.
    fn build_case_name(
        &self,
        feat: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        sc: &gherkin::Scenario,
    ) -> String {
        format!(
            "{}Scenario: {}: {}{}:{}",
            rule.map(|r| format!("Rule: {}: ", r.name)).unwrap_or_default(),
            sc.name,
            feat.path
                .as_ref()
                .and_then(|p| p.to_str().map(trim_path))
                .map(|path| format!("{path}:"))
                .unwrap_or_default(),
            sc.position.line,
            sc.position.col,
        )
    }

    /// Creates a test case based on the last event's outcome.
    fn create_test_case(
        &self,
        last_event: &event::RetryableScenario<W>,
        case_name: &str,
        duration: Duration,
    ) -> TestCase {
        match &last_event.event {
            Scenario::Started
            | Scenario::Log(_)
            | Scenario::Hook(_, Hook::Started | Hook::Passed)
            | Scenario::Background(_, Step::Started | Step::Passed(_, _))
            | Scenario::Step(_, Step::Started | Step::Passed(_, _)) => {
                TestCaseBuilder::success(case_name, duration).build()
            }
            Scenario::Background(_, Step::Skipped)
            | Scenario::Step(_, Step::Skipped) => {
                TestCaseBuilder::skipped(case_name).build()
            }
            Scenario::Hook(_, Hook::Failed(_, e)) => TestCaseBuilder::failure(
                case_name,
                duration,
                "Hook Panicked",
                coerce_error(e).as_ref(),
            )
            .build(),
            Scenario::Background(_, Step::Failed(_, _, _, e))
            | Scenario::Step(_, Step::Failed(_, _, _, e)) => {
                TestCaseBuilder::failure(
                    case_name,
                    duration,
                    "Step Panicked",
                    &e.to_string(),
                )
                .build()
            }
            Scenario::Finished => {
                panic!(
                    "Duplicated `Finished` event for `Scenario`: \"{}\"\n\
                     {WRAP_ADVICE}",
                    case_name,
                );
            }
        }
    }

    /// Generates system output using the basic writer.
    fn generate_system_output(
        &self,
        feat: &gherkin::Feature,
        sc: &gherkin::Scenario,
        events: &[event::RetryableScenario<W>],
    ) -> String {
        // We should be passing normalized events here,
        // so using `writer::Basic::raw()` is OK.
        let mut basic_wr = crate::writer::Basic::raw(
            WritableString(String::new()),
            Coloring::Never,
            self.verbosity,
        );

        events
            .iter()
            .map(|ev| {
                basic_wr.scenario(feat, sc, ev)?;
                Ok(mem::take(&mut **basic_wr))
            })
            .collect::<io::Result<String>>()
            .unwrap_or_else(|e| {
                panic!("Failed to write with `writer::Basic`: {e}")
            })
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use gherkin::{Feature, LineCol, Scenario};
    use junit_report::Duration;

    use crate::{
        Event,
        event::{self, Hook, HookType, Step, StepError},
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
            examples: None,
        }
    }

    #[test]
    fn builds_successful_test_case() {
        let builder = JUnitTestCaseBuilder::<TestWorld>::new(Verbosity::Default);
        let feature = create_test_feature();
        let scenario = create_test_scenario();
        let events = vec![
            event::RetryableScenario {
                event: event::Scenario::Started,
                retries: None,
            },
            event::RetryableScenario {
                event: event::Scenario::Step(
                    gherkin::Step {
                        keyword: "Given".to_string(),
                        ty: gherkin::StepType::Given,
                        value: "I have a step".to_string(),
                        docstring: None,
                        table: None,
                        position: LineCol { line: 6, col: 5 },
                    },
                    Step::Passed("".to_string(), None),
                ),
                retries: None,
            },
        ];

        let test_case = builder.build_test_case(
            &feature,
            None,
            &scenario,
            &events,
            Duration::milliseconds(100),
        );

        assert_eq!(test_case.name(), "Scenario: Test Scenario: example.feature:5:3");
        assert!(test_case.result().is_success());
    }

    #[test]
    fn builds_failed_test_case_with_step_error() {
        let builder = JUnitTestCaseBuilder::<TestWorld>::new(Verbosity::Default);
        let feature = create_test_feature();
        let scenario = create_test_scenario();
        let events = vec![
            event::RetryableScenario {
                event: event::Scenario::Step(
                    gherkin::Step {
                        keyword: "When".to_string(),
                        ty: gherkin::StepType::When,
                        value: "I fail".to_string(),
                        docstring: None,
                        table: None,
                        position: LineCol { line: 7, col: 5 },
                    },
                    Step::Failed(
                        "".to_string(),
                        None,
                        None,
                        Arc::new(StepError::NotFound),
                    ),
                ),
                retries: None,
            },
        ];

        let test_case = builder.build_test_case(
            &feature,
            None,
            &scenario,
            &events,
            Duration::milliseconds(200),
        );

        assert!(test_case.result().is_failure());
        let failure = test_case.result().as_failure().unwrap();
        assert_eq!(failure.type_(), "Step Panicked");
    }

    #[test]
    fn builds_skipped_test_case() {
        let builder = JUnitTestCaseBuilder::<TestWorld>::new(Verbosity::Default);
        let feature = create_test_feature();
        let scenario = create_test_scenario();
        let events = vec![
            event::RetryableScenario {
                event: event::Scenario::Step(
                    gherkin::Step {
                        keyword: "Then".to_string(),
                        ty: gherkin::StepType::Then,
                        value: "I am skipped".to_string(),
                        docstring: None,
                        table: None,
                        position: LineCol { line: 8, col: 5 },
                    },
                    Step::Skipped,
                ),
                retries: None,
            },
        ];

        let test_case = builder.build_test_case(
            &feature,
            None,
            &scenario,
            &events,
            Duration::milliseconds(0),
        );

        assert!(test_case.result().is_skipped());
    }

    #[test]
    fn builds_test_case_with_rule() {
        let builder = JUnitTestCaseBuilder::<TestWorld>::new(Verbosity::Default);
        let feature = create_test_feature();
        let scenario = create_test_scenario();
        let rule = gherkin::Rule {
            name: "Test Rule".to_string(),
            description: None,
            background: None,
            scenarios: vec![],
            tags: vec![],
            position: LineCol { line: 3, col: 1 },
        };
        let events = vec![
            event::RetryableScenario {
                event: event::Scenario::Started,
                retries: None,
            },
        ];

        let test_case = builder.build_test_case(
            &feature,
            Some(&rule),
            &scenario,
            &events,
            Duration::milliseconds(50),
        );

        assert!(test_case.name().starts_with("Rule: Test Rule: Scenario: Test Scenario"));
    }

    #[test]
    fn calculates_duration_correctly() {
        let start = SystemTime::UNIX_EPOCH;
        let end = start + std::time::Duration::from_millis(500);
        let scenario = create_test_scenario();

        let duration = JUnitTestCaseBuilder::<TestWorld>::calculate_duration(start, end, &scenario);

        assert_eq!(duration, Duration::milliseconds(500));
    }

    #[test]
    #[should_panic(expected = "no events for `Scenario`")]
    fn panics_on_empty_events() {
        let builder = JUnitTestCaseBuilder::<TestWorld>::new(Verbosity::Default);
        let feature = create_test_feature();
        let scenario = create_test_scenario();
        let events = vec![];

        builder.build_test_case(&feature, None, &scenario, &events, Duration::ZERO);
    }

    use std::sync::Arc;
}