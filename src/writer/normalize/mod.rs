// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Event normalization for [`Writer`] implementations.
//!
//! This module provides tools for normalizing the order of [`Cucumber`] events
//! to ensure they are emitted in a readable and consistent manner, even when
//! tests are run concurrently.
//!
//! # Modular Architecture
//!
//! The normalization functionality is organized into several focused modules:
//!
//! - [`wrapper`]: The main [`Normalize`] wrapper struct that provides event normalization
//! - [`assert`]: The [`Normalized`] trait and [`AssertNormalized`] wrapper for asserting normalization
//! - [`queue`]: Core queue functionality with [`Queue`] and [`FinishedState`]
//! - [`emitter`]: The [`Emitter`] trait for event emission logic
//! - [`cucumber`]: [`CucumberQueue`] and [`FeatureQueue`] implementations
//! - [`rules`]: [`RulesQueue`] implementation for rule event handling
//! - [`scenarios`]: [`ScenariosQueue`] implementation for scenario event handling
//!
//! Each module follows the Single Responsibility Principle and includes comprehensive
//! unit tests to ensure correctness and maintainability.
//!
//! # Usage
//!
//! ```rust,no_run
//! use cucumber::writer::{Basic, Normalize};
//! 
//! let writer = Basic::stdout();
//! let normalized_writer = Normalize::new(writer);
//! ```
//!
//! [`Cucumber`]: crate::event::Cucumber
//! [`Writer`]: crate::Writer

pub mod assert;
pub mod cucumber;
pub mod emitter;
pub mod queue;
pub mod rules;
pub mod scenarios;
pub mod wrapper;

// Re-export all public types for backward compatibility
pub use self::{
    assert::{AssertNormalized, Normalized},
    cucumber::{CucumberQueue, FeatureQueue, NextRuleOrScenario, RuleOrScenario, RuleOrScenarioQueue},
    emitter::Emitter,
    queue::{FinishedState, Queue},
    rules::RulesQueue,
    scenarios::ScenariosQueue,
    wrapper::Normalize,
};

#[cfg(test)]
#[allow(dead_code)]
mod integration_tests {
    use super::*;
    use crate::{
        Event, Writer,
        event::{self, Metadata, Source},
        parser,
    };
    use crate::test_utils::common::{EmptyCli, TestWorld};

    // Using common TestWorld from test_utils

    // Mock Writer for integration testing
    #[derive(Debug, Clone)]
    struct MockWriter {
        events: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    }

    impl MockWriter {
        fn new() -> Self {
            Self {
                events: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            }
        }

        fn get_events(&self) -> Vec<String> {
            self.events.lock().unwrap().clone()
        }
    }

    impl<W: crate::World> Writer<W> for MockWriter {
        type Cli = EmptyCli;

        async fn handle_event(
            &mut self,
            event: parser::Result<Event<event::Cucumber<W>>>,
            _cli: &Self::Cli,
        ) {
            if let Ok(ev) = event {
                let event_name = match ev.value {
                    event::Cucumber::Started => "CucumberStarted".to_string(),
                    event::Cucumber::ParsingFinished { .. } => "ParsingFinished".to_string(),
                    event::Cucumber::Finished => "CucumberFinished".to_string(),
                    event::Cucumber::Feature(_, feature_event) => {
                        match feature_event {
                            event::Feature::Started => "FeatureStarted".to_string(),
                            event::Feature::Finished => "FeatureFinished".to_string(),
                            event::Feature::Scenario(_, scenario_event) => {
                                match scenario_event.event {
                                    event::Scenario::Started => "ScenarioStarted".to_string(),
                                    event::Scenario::Finished => "ScenarioFinished".to_string(),
                                    _ => "Scenario".to_string(),
                                }
                            }
                            event::Feature::Rule(_, rule_event) => {
                                match rule_event {
                                    event::Rule::Started => "RuleStarted".to_string(),
                                    event::Rule::Finished => "RuleFinished".to_string(),
                                    event::Rule::Scenario(_, scenario_event) => {
                                        match scenario_event.event {
                                            event::Scenario::Started => "RuleScenarioStarted".to_string(),
                                            event::Scenario::Finished => "RuleScenarioFinished".to_string(),
                                            _ => "RuleScenario".to_string(),
                                        }
                                    }
                                }
                            }
                        }
                    }
                };
                self.events.lock().unwrap().push(event_name);
            }
        }
    }

    impl<W: crate::World> crate::writer::Stats<W> for MockWriter {
        fn passed_steps(&self) -> usize { 0 }
        fn skipped_steps(&self) -> usize { 0 }
        fn failed_steps(&self) -> usize { 0 }
        fn retried_steps(&self) -> usize { 0 }
        fn parsing_errors(&self) -> usize { 0 }
        fn hook_errors(&self) -> usize { 0 }
    }

    impl crate::writer::NonTransforming for MockWriter {}

    fn create_test_feature() -> Source<gherkin::Feature> {
        Source::new(gherkin::Feature {
            name: "Test Feature".to_string(),
            description: None,
            background: None,
            scenarios: Vec::new(),
            rules: Vec::new(),
            tags: Vec::new(),
            keyword: "Feature".to_string(),
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 1, col: 1 },
            path: None,
        })
    }

    #[tokio::test]
    async fn test_normalize_wrapper_integration() {
        let mock_writer = MockWriter::new();
        let mut normalize: wrapper::Normalize<TestWorld, _> = Normalize::new(mock_writer.clone());

        // Test that Cucumber::Started events pass through immediately
        let started_event = Ok(Event::new(event::Cucumber::<TestWorld>::Started));
        normalize.handle_event(started_event, &EmptyCli).await;

        let events = normalize.inner_writer().get_events();
        assert!(events.contains(&"CucumberStarted".to_string()));
    }

    #[tokio::test]
    async fn test_normalize_feature_lifecycle() {
        let mock_writer = MockWriter::new();
        let mut normalize: wrapper::Normalize<TestWorld, _> = Normalize::new(mock_writer.clone());
        let feature = create_test_feature();

        // Start a feature
        let feature_started = Ok(Event::new(event::Cucumber::<TestWorld>::feature_started(feature.clone())));
        normalize.handle_event(feature_started, &EmptyCli).await;

        // Finish the feature
        let feature_finished = Ok(Event::new(event::Cucumber::<TestWorld>::feature_finished(feature)));
        normalize.handle_event(feature_finished, &EmptyCli).await;

        // Finish cucumber
        let cucumber_finished = Ok(Event::new(event::Cucumber::<TestWorld>::Finished));
        normalize.handle_event(cucumber_finished, &EmptyCli).await;

        let events = normalize.inner_writer().get_events();
        assert!(events.contains(&"FeatureStarted".to_string()));
        assert!(events.contains(&"FeatureFinished".to_string()));
        assert!(events.contains(&"CucumberFinished".to_string()));
    }

    #[test]
    fn test_assert_normalized_wrapper() {
        let mock_writer = MockWriter::new();
        let assert_normalized = AssertNormalized::new(mock_writer);

        // Should implement Normalized trait
        fn requires_normalized<T: Normalized>(_: T) {}
        requires_normalized(assert_normalized);
    }

    #[test]
    fn test_module_re_exports() {
        // Test that all expected types are re-exported
        let _: Queue<String, i32> = Queue::new(Metadata::new(()));
        let _: FinishedState = FinishedState::NotFinished;
        let _: ScenariosQueue<TestWorld> = ScenariosQueue::new();
        
        // Test that the main wrapper types are available
        let mock_writer = MockWriter::new();
        let _: Normalize<TestWorld, MockWriter> = Normalize::new(mock_writer.clone());
        let _: AssertNormalized<MockWriter> = AssertNormalized::new(mock_writer);
    }

    #[test]
    fn test_queue_integration() {
        let mut queue: Queue<String, i32> = Queue::new(Metadata::new(()));
        
        // Test basic queue operations
        queue.fifo.insert("key1".to_string(), 42);
        queue.finished(Metadata::new(()));
        
        assert!(!queue.is_finished_and_emitted());
        let meta = queue.state.take_to_emit();
        assert!(meta.is_some());
        assert!(queue.is_finished_and_emitted());
    }

    #[test]
    fn test_finished_state_transitions() {
        let mut state = FinishedState::NotFinished;
        
        // Should start as not finished
        assert!(matches!(state, FinishedState::NotFinished));
        
        // Should not have metadata to emit when not finished
        assert!(state.take_to_emit().is_none());
        
        // Transition to finished but not emitted
        state = FinishedState::FinishedButNotEmitted(Metadata::new(()));
        assert!(matches!(state, FinishedState::FinishedButNotEmitted(_)));
        
        // Should have metadata to emit
        let meta = state.take_to_emit();
        assert!(meta.is_some());
        assert!(matches!(state, FinishedState::FinishedAndEmitted));
    }

    #[test]
    fn test_scenarios_queue_integration() {
        let mut queue: scenarios::ScenariosQueue<()> = ScenariosQueue::new();
        
        // Start with empty queue
        assert!((&mut queue).current_item().is_none());
        
        // Add an event
        queue.0.push(Event::new(
            event::RetryableScenario {
                event: event::Scenario::Started,
                retries: None,
            }
        ));
        
        // Should now have a current item
        assert!((&mut queue).current_item().is_some());
        
        // After getting current item, queue should be empty again
        assert!((&mut queue).current_item().is_none());
    }

    #[test]
    fn test_backward_compatibility() {
        // Test that all the original public API is still available through re-exports
        let mock_writer = MockWriter::new();
        
        // Should be able to create a Normalize wrapper
        let _normalize: Normalize<TestWorld, _> = Normalize::new(mock_writer.clone());
        
        // Should be able to create an AssertNormalized wrapper
        let _assert: AssertNormalized<_> = AssertNormalized::new(mock_writer);
        
        // Should be able to use the Normalized trait
        fn test_normalized<T: Normalized>(_: T) {}
        test_normalized(_assert);
    }
}