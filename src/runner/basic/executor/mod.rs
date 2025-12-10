//! Modularized executor implementation following Single Responsibility Principle.
//!
//! This module breaks down the large executor implementation into focused components:
//! - `core`: Main Executor struct and orchestration logic
//! - `hooks`: Before/after hook execution logic
//! - `steps`: Step execution logic
//! - `events`: Event sending functionality

mod core;
mod events;
mod hooks;
mod steps;

pub use core::Executor;

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::{
        event::{self, source::Source},
        test_utils::common::TestWorld,
        step,
        parser,
        Event,
    };
    use futures::{channel::mpsc, future::LocalBoxFuture, TryStreamExt as _};

    type BeforeHook = for<'a> fn(&'a gherkin::Feature, Option<&'a gherkin::Rule>, &'a gherkin::Scenario, &'a mut TestWorld) -> LocalBoxFuture<'a, ()>;
    type AfterHook = for<'a> fn(&'a gherkin::Feature, Option<&'a gherkin::Rule>, &'a gherkin::Scenario, &'a event::ScenarioFinished, Option<&'a mut TestWorld>) -> LocalBoxFuture<'a, ()>;

    #[test]
    fn test_executor_creation() {
        let (_executor, _receiver) = create_test_executor();
        
        // Verify executor is created successfully
        assert!(true); // Basic creation test
    }

    #[test]
    fn test_executor_send_event() {
        let (executor, mut receiver) = create_test_executor();
        let (feature, scenario) = create_test_feature_and_scenario();
        
        let event: event::Cucumber<TestWorld> = event::Cucumber::scenario(
            feature,
            None::<event::source::Source<gherkin::Rule>>,
            scenario,
            event::RetryableScenario {
                event: event::Scenario::<TestWorld>::Started,
                retries: None,
            },
        );
        
        executor.send_event(event);
        
        // Should receive the event
        let received = receiver.try_next().unwrap().unwrap().unwrap();
        assert!(matches!(received.value, event::Cucumber::Feature { .. }));
    }

    #[test]
    fn test_executor_send_all_events() {
        let (executor, mut receiver) = create_test_executor();
        let (feature, scenario) = create_test_feature_and_scenario();
        
        let events = vec![
            event::Cucumber::<TestWorld>::scenario(
                feature.clone(),
                None::<event::source::Source<gherkin::Rule>>,
                scenario.clone(),
                event::RetryableScenario {
                    event: event::Scenario::<TestWorld>::Started,
                    retries: None,
                },
            ),
            event::Cucumber::<TestWorld>::scenario(
                feature,
                None::<event::source::Source<gherkin::Rule>>,
                scenario,
                event::RetryableScenario {
                    event: event::Scenario::<TestWorld>::Finished,
                    retries: None,
                },
            ),
        ];
        
        executor.send_all_events(events);
        
        // Should receive both events
        let first = receiver.try_next().unwrap().unwrap().unwrap();
        let second = receiver.try_next().unwrap().unwrap().unwrap();
        
        assert!(matches!(first.value, event::Cucumber::Feature { .. }));
        assert!(matches!(second.value, event::Cucumber::Feature { .. }));
    }

    fn create_test_executor() -> (Executor<TestWorld, BeforeHook, AfterHook>, mpsc::UnboundedReceiver<parser::Result<Event<event::Cucumber<TestWorld>>>>) {
        use super::super::scenario_storage::Features;
        
        let collection = step::Collection::<TestWorld>::new();
        let (event_sender, event_receiver) = mpsc::unbounded();
        let (finished_sender, _finished_receiver) = mpsc::unbounded();
        let storage = Features::default();
        
        let executor = Executor::new(
            collection,
            None,
            None,
            event_sender,
            finished_sender,
            storage,
            #[cfg(feature = "observability")]
            std::sync::Arc::new(std::sync::Mutex::new(crate::observer::ObserverRegistry::new())),
        );
        
        (executor, event_receiver)
    }

    fn create_test_feature_and_scenario() -> (Source<gherkin::Feature>, Source<gherkin::Scenario>) {
        use gherkin::{Feature, Scenario};
        
        let feature = Feature {
            keyword: "Feature".to_string(),
            name: "Test Feature".to_string(),
            description: None,
            background: None,
            scenarios: vec![],
            rules: vec![],
            tags: vec![],
            span: gherkin::Span {
                start: 0,
                end: 0,
            },
            position: gherkin::LineCol { line: 1, col: 1 },
            path: None,
        };
        
        let scenario = Scenario {
            keyword: "Scenario".to_string(),
            name: "Test Scenario".to_string(),
            description: None,
            steps: vec![],
            examples: vec![],
            tags: vec![],
            span: gherkin::Span {
                start: 0,
                end: 0,
            },
            position: gherkin::LineCol { line: 2, col: 1 },
        };
        
        (Source::new(feature), Source::new(scenario))
    }
}