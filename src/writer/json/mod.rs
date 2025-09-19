// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! [Cucumber JSON format][1] [`Writer`] implementation.
//!
//! This module provides a modular implementation of the JSON writer following
//! the Single Responsibility Principle. The implementation is organized into
//! several focused modules:
//!
//! - [`types`]: Basic serializable data types
//! - [`element`]: Element (Scenario/Background) structures
//! - [`feature`]: Feature structures and utilities
//! - [`handlers`]: Event handling logic
//! - [`writer`]: Core writer implementation
//!
//! [1]: https://github.com/cucumber/cucumber-json-schema

pub mod element;
pub mod feature;
pub mod handlers;
pub mod types;
pub mod writer;

// Re-export all public types for backward compatibility
pub use self::{
    element::Element,
    feature::Feature,
    types::{Base64, Embedding, HookResult, RunResult, Status, Step, Tag},
    writer::Json,
};

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::{
        event::{Cucumber, Feature as FeatureEvent, Hook, HookType, Metadata, Scenario, Step as StepEvent},
        Event, World, Writer, cli,
        parser::Result as ParserResult,
    };
    use std::{io::Cursor, time::SystemTime};

    #[derive(Debug)]
    struct TestWorld;

    impl World for TestWorld {
        type Error = ();
    }

    fn create_test_feature() -> gherkin::Feature {
        gherkin::Feature {
            keyword: "Feature".to_string(),
            name: "Integration Test Feature".to_string(),
            description: None,
            background: None,
            scenarios: vec![],
            rules: vec![],
            tags: vec!["@integration".to_string()],
            position: gherkin::LineCol { line: 1, col: 1 },
            path: Some(std::path::PathBuf::from("integration.feature")),
        }
    }

    fn create_test_scenario() -> gherkin::Scenario {
        gherkin::Scenario {
            keyword: "Scenario".to_string(),
            name: "Integration Test Scenario".to_string(),
            description: None,
            tags: vec!["@test".to_string()],
            position: gherkin::LineCol { line: 5, col: 1 },
            steps: vec![],
            examples: vec![],
        }
    }

    fn create_test_step() -> gherkin::Step {
        gherkin::Step {
            keyword: "Given".to_string(),
            value: "integration test step".to_string(),
            docstring: None,
            table: None,
            position: gherkin::LineCol { line: 6, col: 1 },
        }
    }

    #[tokio::test]
    async fn full_scenario_lifecycle() {
        let mut writer = Json::raw(Cursor::new(Vec::new()));
        let feature = create_test_feature();
        let scenario = create_test_scenario();
        let step = create_test_step();

        // 1. Scenario started
        let event = Event::new(
            Cucumber::Feature(
                feature.clone(),
                FeatureEvent::Scenario(
                    scenario.clone(),
                    crate::event::RetryableScenario {
                        event: Scenario::Started,
                        retries: 0,
                    },
                ),
            ),
            Metadata { at: SystemTime::now() },
        );
        writer.handle_event(Ok(event), &cli::Empty).await;

        // 2. Step started
        let event = Event::new(
            Cucumber::Feature(
                feature.clone(),
                FeatureEvent::Scenario(
                    scenario.clone(),
                    crate::event::RetryableScenario {
                        event: Scenario::Step(
                            step.clone(),
                            StepEvent::Started,
                        ),
                        retries: 0,
                    },
                ),
            ),
            Metadata { at: SystemTime::now() },
        );
        writer.handle_event(Ok(event), &cli::Empty).await;

        // 3. Add a log message
        let event = Event::new(
            Cucumber::Feature(
                feature.clone(),
                FeatureEvent::Scenario(
                    scenario.clone(),
                    crate::event::RetryableScenario {
                        event: Scenario::Log("Step execution log".to_string()),
                        retries: 0,
                    },
                ),
            ),
            Metadata { at: SystemTime::now() },
        );
        writer.handle_event(Ok(event), &cli::Empty).await;

        // 4. Step passed
        let event = Event::new(
            Cucumber::Feature(
                feature.clone(),
                FeatureEvent::Scenario(
                    scenario.clone(),
                    crate::event::RetryableScenario {
                        event: Scenario::Step(
                            step.clone(),
                            StepEvent::Passed(()),
                        ),
                        retries: 0,
                    },
                ),
            ),
            Metadata { at: SystemTime::now() + std::time::Duration::from_millis(100) },
        );
        writer.handle_event(Ok(event), &cli::Empty).await;

        // 5. Before hook
        let event = Event::new(
            Cucumber::Feature(
                feature.clone(),
                FeatureEvent::Scenario(
                    scenario.clone(),
                    crate::event::RetryableScenario {
                        event: Scenario::Hook(
                            HookType::Before,
                            Hook::Started,
                        ),
                        retries: 0,
                    },
                ),
            ),
            Metadata { at: SystemTime::now() },
        );
        writer.handle_event(Ok(event), &cli::Empty).await;

        let event = Event::new(
            Cucumber::Feature(
                feature.clone(),
                FeatureEvent::Scenario(
                    scenario.clone(),
                    crate::event::RetryableScenario {
                        event: Scenario::Hook(
                            HookType::Before,
                            Hook::Passed,
                        ),
                        retries: 0,
                    },
                ),
            ),
            Metadata { at: SystemTime::now() + std::time::Duration::from_millis(50) },
        );
        writer.handle_event(Ok(event), &cli::Empty).await;

        // 6. Scenario finished
        let event = Event::new(
            Cucumber::Feature(
                feature.clone(),
                FeatureEvent::Scenario(
                    scenario.clone(),
                    crate::event::RetryableScenario {
                        event: Scenario::Finished,
                        retries: 0,
                    },
                ),
            ),
            Metadata { at: SystemTime::now() },
        );
        writer.handle_event(Ok(event), &cli::Empty).await;

        // 7. Finish and output JSON
        let event = Event::new(
            Cucumber::Finished,
            Metadata { at: SystemTime::now() },
        );
        writer.handle_event(Ok(event), &cli::Empty).await;

        // Verify the JSON was written
        let output = writer.output.into_inner();
        let json_str = String::from_utf8(output).unwrap();
        
        assert!(!json_str.is_empty());
        
        // Parse and verify the JSON structure
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let features = json.as_array().unwrap();
        
        assert_eq!(features.len(), 1);
        
        let feature_json = &features[0];
        assert_eq!(feature_json["name"], "Integration Test Feature");
        assert_eq!(feature_json["keyword"], "Feature");
        assert_eq!(feature_json["uri"], "integration.feature");
        
        let elements = feature_json["elements"].as_array().unwrap();
        assert_eq!(elements.len(), 1);
        
        let element = &elements[0];
        assert_eq!(element["name"], "Integration Test Scenario");
        assert_eq!(element["type"], "scenario");
        
        let steps = element["steps"].as_array().unwrap();
        assert_eq!(steps.len(), 1);
        
        let step_json = &steps[0];
        assert_eq!(step_json["name"], "integration test step");
        assert_eq!(step_json["keyword"], "Given");
        assert_eq!(step_json["result"]["status"], "passed");
        
        // Check for embeddings from the log
        let embeddings = step_json["embeddings"].as_array().unwrap();
        assert_eq!(embeddings.len(), 1);
        assert_eq!(embeddings[0]["mime_type"], "text/x.cucumber.log+plain");
        
        // Check hooks
        let before_hooks = element["before"].as_array().unwrap();
        assert_eq!(before_hooks.len(), 1);
        assert_eq!(before_hooks[0]["result"]["status"], "passed");
    }

    #[test]
    fn all_types_are_serializable() {
        // Test that all our types can be serialized properly
        let base64 = Base64::encode("test data");
        assert!(serde_json::to_string(&base64).is_ok());

        let embedding = Embedding::from_log("test log");
        assert!(serde_json::to_string(&embedding).is_ok());

        let tag = Tag {
            name: "@test".to_string(),
            line: 1,
        };
        assert!(serde_json::to_string(&tag).is_ok());

        let status = Status::Passed;
        assert!(serde_json::to_string(&status).is_ok());

        let run_result = RunResult {
            status: Status::Passed,
            duration: 1000,
            error_message: None,
        };
        assert!(serde_json::to_string(&run_result).is_ok());

        let step = Step {
            keyword: "Given".to_string(),
            line: 1,
            name: "test step".to_string(),
            hidden: false,
            result: run_result.clone(),
            embeddings: vec![embedding],
        };
        assert!(serde_json::to_string(&step).is_ok());

        let hook_result = HookResult {
            result: run_result,
            embeddings: vec![],
        };
        assert!(serde_json::to_string(&hook_result).is_ok());
    }

    #[test]
    fn json_schema_compatibility() {
        // Verify that our JSON output matches expected schema structure
        let feature = create_test_feature();
        let scenario = create_test_scenario();
        let element = Element::new(&feature, None, &scenario, "scenario");
        let json_feature = Feature::new(&feature);
        
        // Test required fields are present
        let element_json = serde_json::to_value(&element).unwrap();
        assert!(element_json.as_object().unwrap().contains_key("keyword"));
        assert!(element_json.as_object().unwrap().contains_key("type"));
        assert!(element_json.as_object().unwrap().contains_key("id"));
        assert!(element_json.as_object().unwrap().contains_key("line"));
        assert!(element_json.as_object().unwrap().contains_key("name"));
        assert!(element_json.as_object().unwrap().contains_key("tags"));
        assert!(element_json.as_object().unwrap().contains_key("steps"));
        
        let feature_json = serde_json::to_value(&json_feature).unwrap();
        assert!(feature_json.as_object().unwrap().contains_key("keyword"));
        assert!(feature_json.as_object().unwrap().contains_key("name"));
        assert!(feature_json.as_object().unwrap().contains_key("tags"));
        assert!(feature_json.as_object().unwrap().contains_key("elements"));
        assert!(feature_json.as_object().unwrap().contains_key("uri"));
    }
}