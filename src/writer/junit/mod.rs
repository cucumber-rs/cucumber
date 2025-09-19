//! [JUnit XML report][1] [`Writer`] implementation.
//!
//! This module provides a modular implementation of a JUnit XML writer that follows
//! the Single Responsibility Principle. The implementation is split across several
//! focused modules:
//!
//! - [`cli`]: CLI configuration and argument parsing
//! - [`error_handler`]: Error handling for parser and expansion errors
//! - [`event_handlers`]: Event processing logic for different Cucumber events
//! - [`test_case_builder`]: Test case creation from scenario events
//! - [`writer`]: Main JUnit writer implementation
//!
//! [1]: https://llg.cubic.org/docs/junit

pub mod cli;
pub mod error_handler;
pub mod event_handlers;
pub mod test_case_builder;
pub mod writer;

// Re-export main types for backward compatibility
pub use cli::Cli;
pub use writer::JUnit;

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, time::SystemTime};

    use gherkin::{Feature, LineCol, Scenario};

    use crate::{
        Event, World,
        event::{self, Cucumber, Feature as FeatureEvent, Scenario as ScenarioEvent, Step},
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
            name: "Integration Test Feature".to_string(),
            description: None,
            background: None,
            scenarios: vec![],
            rules: vec![],
            tags: vec![],
            position: LineCol { line: 1, col: 1 },
            path: Some(PathBuf::from("/test/features/integration.feature")),
        }
    }

    fn create_test_scenario() -> Scenario {
        Scenario {
            name: "Integration Test Scenario".to_string(),
            description: None,
            steps: vec![],
            tags: vec![],
            position: LineCol { line: 5, col: 3 },
            examples: vec![],
        }
    }

    #[tokio::test]
    async fn full_integration_test_successful_scenario() {
        let output = Vec::new();
        let mut writer = JUnit::<TestWorld, _>::raw(output, Verbosity::Default);
        let feature = create_test_feature();
        let scenario = create_test_scenario();
        let cli = Cli::default();

        // Start Cucumber
        let cucumber_start = Ok(Event {
            value: Cucumber::Started,
            at: SystemTime::UNIX_EPOCH,
        });
        writer.handle_event(cucumber_start, &cli).await;

        // Start Feature
        let feature_start = Ok(Event {
            value: Cucumber::Feature(feature.clone(), FeatureEvent::Started),
            at: SystemTime::UNIX_EPOCH,
        });
        writer.handle_event(feature_start, &cli).await;

        // Start Scenario
        let scenario_start = Ok(Event {
            value: Cucumber::Feature(
                feature.clone(),
                FeatureEvent::Scenario(
                    scenario.clone(),
                    event::RetryableScenario {
                        event: ScenarioEvent::Started,
                        retries: None,
                    },
                ),
            ),
            at: SystemTime::UNIX_EPOCH,
        });
        writer.handle_event(scenario_start, &cli).await;

        // Add a successful step
        let step = gherkin::Step {
            keyword: "Given".to_string(),
            ty: gherkin::StepType::Given,
            value: "I have a successful step".to_string(),
            docstring: None,
            table: None,
            position: LineCol { line: 6, col: 5 },
        };
        let step_event = Ok(Event {
            value: Cucumber::Feature(
                feature.clone(),
                FeatureEvent::Scenario(
                    scenario.clone(),
                    event::RetryableScenario {
                        event: ScenarioEvent::Step(step, Step::Passed("".to_string(), None)),
                        retries: None,
                    },
                ),
            ),
            at: SystemTime::UNIX_EPOCH + std::time::Duration::from_millis(50),
        });
        writer.handle_event(step_event, &cli).await;

        // Finish Scenario
        let scenario_finish = Ok(Event {
            value: Cucumber::Feature(
                feature.clone(),
                FeatureEvent::Scenario(
                    scenario.clone(),
                    event::RetryableScenario {
                        event: ScenarioEvent::Finished,
                        retries: None,
                    },
                ),
            ),
            at: SystemTime::UNIX_EPOCH + std::time::Duration::from_millis(100),
        });
        writer.handle_event(scenario_finish, &cli).await;

        // Finish Feature
        let feature_finish = Ok(Event {
            value: Cucumber::Feature(feature.clone(), FeatureEvent::Finished),
            at: SystemTime::UNIX_EPOCH + std::time::Duration::from_millis(150),
        });
        writer.handle_event(feature_finish, &cli).await;

        // Finish Cucumber
        let cucumber_finish = Ok(Event {
            value: Cucumber::Finished,
            at: SystemTime::UNIX_EPOCH + std::time::Duration::from_millis(200),
        });
        writer.handle_event(cucumber_finish, &cli).await;

        // Verify the XML output
        let output_str = String::from_utf8(writer.output).unwrap();
        assert!(output_str.contains("<?xml"));
        assert!(output_str.contains("<testsuites"));
        assert!(output_str.contains("Integration Test Feature"));
        assert!(output_str.contains("Integration Test Scenario"));
    }

    #[tokio::test]
    async fn integration_test_with_failed_scenario() {
        let output = Vec::new();
        let mut writer = JUnit::<TestWorld, _>::raw(output, Verbosity::Default);
        let feature = create_test_feature();
        let scenario = create_test_scenario();
        let cli = Cli::default();

        // Start feature and scenario
        writer.handle_event(
            Ok(Event {
                value: Cucumber::Feature(feature.clone(), FeatureEvent::Started),
                at: SystemTime::UNIX_EPOCH,
            }),
            &cli,
        ).await;

        writer.handle_event(
            Ok(Event {
                value: Cucumber::Feature(
                    feature.clone(),
                    FeatureEvent::Scenario(
                        scenario.clone(),
                        event::RetryableScenario {
                            event: ScenarioEvent::Started,
                            retries: None,
                        },
                    ),
                ),
                at: SystemTime::UNIX_EPOCH,
            }),
            &cli,
        ).await;

        // Add a failed step
        let failed_step = gherkin::Step {
            keyword: "When".to_string(),
            ty: gherkin::StepType::When,
            value: "I fail".to_string(),
            docstring: None,
            table: None,
            position: LineCol { line: 7, col: 5 },
        };
        writer.handle_event(
            Ok(Event {
                value: Cucumber::Feature(
                    feature.clone(),
                    FeatureEvent::Scenario(
                        scenario.clone(),
                        event::RetryableScenario {
                            event: ScenarioEvent::Step(
                                failed_step,
                                Step::Failed(
                                    "".to_string(),
                                    None,
                                    None,
                                    std::sync::Arc::new(crate::event::StepError::NotFound),
                                ),
                            ),
                            retries: None,
                        },
                    ),
                ),
                at: SystemTime::UNIX_EPOCH + std::time::Duration::from_millis(50),
            }),
            &cli,
        ).await;

        // Finish scenario and feature
        writer.handle_event(
            Ok(Event {
                value: Cucumber::Feature(
                    feature.clone(),
                    FeatureEvent::Scenario(
                        scenario.clone(),
                        event::RetryableScenario {
                            event: ScenarioEvent::Finished,
                            retries: None,
                        },
                    ),
                ),
                at: SystemTime::UNIX_EPOCH + std::time::Duration::from_millis(100),
            }),
            &cli,
        ).await;

        writer.handle_event(
            Ok(Event {
                value: Cucumber::Feature(feature.clone(), FeatureEvent::Finished),
                at: SystemTime::UNIX_EPOCH + std::time::Duration::from_millis(150),
            }),
            &cli,
        ).await;

        writer.handle_event(
            Ok(Event {
                value: Cucumber::Finished,
                at: SystemTime::UNIX_EPOCH + std::time::Duration::from_millis(200),
            }),
            &cli,
        ).await;

        // Verify failed test case in XML
        let output_str = String::from_utf8(writer.output).unwrap();
        assert!(output_str.contains("failure"));
        assert!(output_str.contains("Step Panicked"));
    }

    #[tokio::test]
    async fn integration_test_with_parser_error() {
        let output = Vec::new();
        let mut writer = JUnit::<TestWorld, _>::raw(output, Verbosity::Default);
        let cli = Cli::default();

        // Send a parser error
        let parse_error = gherkin::ParseFileError::Reading {
            path: PathBuf::from("/test/broken.feature"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "File not found"),
        };
        let error = Err(parser::Error::Parsing(Box::new(parse_error)));

        writer.handle_event(error, &cli).await;

        // Finish cucumber to generate XML
        writer.handle_event(
            Ok(Event {
                value: Cucumber::Finished,
                at: SystemTime::UNIX_EPOCH,
            }),
            &cli,
        ).await;

        // Verify error suite in XML
        let output_str = String::from_utf8(writer.output).unwrap();
        assert!(output_str.contains("Errors"));
        assert!(output_str.contains("Parser Error"));
        assert!(output_str.contains("broken.feature"));
    }

    #[test]
    fn cli_verbosity_integration() {
        let cli_default = Cli::default();
        let cli_verbose = Cli::with_verbosity(Some(1));

        assert_eq!(cli_default.to_verbosity(), None);
        assert_eq!(cli_verbose.to_verbosity(), Some(Verbosity::ShowWorld));
    }

    #[test]
    fn module_re_exports_work() {
        let _cli: Cli = Cli::default();
        let output = Vec::new();
        let _writer: JUnit<TestWorld, _> = JUnit::raw(output, Verbosity::Default);
    }
}