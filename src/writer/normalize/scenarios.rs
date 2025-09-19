// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! ScenariosQueue implementation for event normalization.

use crate::{
    Event, Writer,
    event::{self, Retries, Source},
};

use super::emitter::Emitter;

/// [`Queue`] of all events of a single [`Scenario`].
///
/// [`Scenario`]: gherkin::Scenario
/// [`Queue`]: super::queue::Queue
#[derive(Debug)]
pub struct ScenariosQueue<World>(pub Vec<Event<event::RetryableScenario<World>>>);

// Implemented manually to omit redundant `World: Clone` trait bound, imposed by
// `#[derive(Clone)]`.
impl<World> Clone for ScenariosQueue<World> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<World> ScenariosQueue<World> {
    /// Creates a new [`ScenariosQueue`].
    pub const fn new() -> Self {
        Self(Vec::new())
    }
}

impl<World> Emitter<World> for &mut ScenariosQueue<World> {
    type Current = Event<event::RetryableScenario<World>>;
    type Emitted = (Source<gherkin::Scenario>, Option<Retries>);
    type EmittedPath = (
        Source<gherkin::Feature>,
        Option<Source<gherkin::Rule>>,
        Source<gherkin::Scenario>,
    );

    fn current_item(self) -> Option<Self::Current> {
        (!self.0.is_empty()).then(|| self.0.remove(0))
    }

    async fn emit<W: Writer<World>>(
        self,
        (feature, rule, scenario): Self::EmittedPath,
        writer: &mut W,
        cli: &W::Cli,
    ) -> Option<Self::Emitted> {
        while let Some((ev, meta)) = self.current_item().map(Event::split) {
            let should_be_removed =
                matches!(ev.event, event::Scenario::Finished)
                    .then(|| ev.retries);

            let ev = meta.wrap(event::Cucumber::scenario(
                feature.clone(),
                rule.clone(),
                scenario.clone(),
                ev,
            ));
            writer.handle_event(Ok(ev), cli).await;

            if let Some(retries) = should_be_removed {
                return Some((scenario.clone(), retries));
            }
        }
        None
    }
}

#[cfg(test)]
#[allow(dead_code)]
mod tests {
    use super::*;
    use crate::{Event, event::{Cucumber, Metadata, RetryableScenario}, Writer, parser, event::Source};
    use std::{sync::Arc, future::Future};
    use crate::test_utils::common::{EmptyCli, TestWorld};

    // Using common TestWorld from test_utils

    // Mock Writer for testing
    struct MockWriter {
        events: Vec<String>,
    }

    impl MockWriter {
        fn new() -> Self {
            Self {
                events: Vec::new(),
            }
        }
    }

    impl Writer<TestWorld> for MockWriter {
        type Cli = EmptyCli;

        async fn handle_event(
            &mut self,
            event: parser::Result<Event<event::Cucumber<TestWorld>>>,
            _cli: &Self::Cli,
        ) {
            if let Ok(ev) = event {
                let event_name = match ev.value {
                    event::Cucumber::Feature(_, feature_event) => {
                        match feature_event {
                            event::Feature::Scenario(_, scenario_event) => {
                                match scenario_event.event {
                                    event::Scenario::Started => "ScenarioStarted",
                                    event::Scenario::Finished => "ScenarioFinished",
                                    _ => "Scenario",
                                }
                            }
                            event::Feature::Rule(_, rule_event) => {
                                match rule_event {
                                    event::Rule::Scenario(_, scenario_event) => {
                                        match scenario_event.event {
                                            event::Scenario::Started => "ScenarioStarted",
                                            event::Scenario::Finished => "ScenarioFinished",
                                            _ => "Scenario",
                                        }
                                    }
                                    _ => "Rule",
                                }
                            }
                            _ => "Feature",
                        }
                    }
                    _ => "Other",
                };
                self.events.push(event_name.to_string());
            }
        }
    }

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
            path: Some("test.feature".into()),
        })
    }

    fn create_test_rule() -> Source<gherkin::Rule> {
        Source::new(gherkin::Rule {
            name: "Test Rule".to_string(),
            description: None,
            background: None,
            scenarios: Vec::new(),
            tags: Vec::new(),
            keyword: "Rule".to_string(),
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 2, col: 1 },
        })
    }

    fn create_test_scenario() -> Source<gherkin::Scenario> {
        Source::new(gherkin::Scenario {
            name: "Test Scenario".to_string(),
            description: None,
            steps: Vec::new(),
            tags: Vec::new(),
            keyword: "Scenario".to_string(),
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 3, col: 1 },
            examples: Vec::new(),
        })
    }

    #[test]
    fn test_scenarios_queue_new() {
        let queue: ScenariosQueue<TestWorld> = ScenariosQueue::new();
        
        assert_eq!(queue.0.len(), 0);
    }

    #[test]
    fn test_scenarios_queue_clone() {
        let mut queue: ScenariosQueue<TestWorld> = ScenariosQueue::new();
        queue.0.push(Event::new(
            RetryableScenario {
                event: event::Scenario::Started,
                retries: None,
            }
        ));
        
        let cloned = queue.clone();
        assert_eq!(cloned.0.len(), 1);
        assert_eq!(queue.0.len(), 1);
    }

    #[test]
    fn test_scenarios_queue_current_item_empty() {
        let mut queue: ScenariosQueue<TestWorld> = ScenariosQueue::new();
        
        let current = (&mut queue).current_item();
        assert!(current.is_none());
    }

    #[test]
    fn test_scenarios_queue_current_item_with_event() {
        let mut queue: ScenariosQueue<TestWorld> = ScenariosQueue::new();
        queue.0.push(Event::new(
            RetryableScenario {
                event: event::Scenario::Started,
                retries: None,
            }
        ));
        
        let current = (&mut queue).current_item();
        assert!(current.is_some());
        
        if let Some(event) = current {
            assert!(matches!(event.value.event, event::Scenario::Started));
        }
        
        // After calling current_item, the event should be removed
        assert_eq!(queue.0.len(), 0);
    }

    #[tokio::test]
    async fn test_scenarios_queue_emit_started_event() {
        let mut queue: ScenariosQueue<TestWorld> = ScenariosQueue::new();
        let mut writer = MockWriter::new();
        let feature = create_test_feature();
        let scenario = create_test_scenario();
        
        queue.0.push(Event::new(
            RetryableScenario {
                event: event::Scenario::Started,
                retries: None,
            }
        ));
        
        let result = (&mut queue).emit((feature, None, scenario), &mut writer, &()).await;
        
        // Should emit the scenario started event
        assert!(writer.events.contains(&"ScenarioStarted".to_string()));
        // Should not return the scenario since it's not finished
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_scenarios_queue_emit_finished_event() {
        let mut queue: ScenariosQueue<TestWorld> = ScenariosQueue::new();
        let mut writer = MockWriter::new();
        let feature = create_test_feature();
        let scenario = create_test_scenario();
        
        queue.0.push(Event::new(
            RetryableScenario {
                event: event::Scenario::Finished,
                retries: None,
            }
        ));
        
        let result = (&mut queue).emit((feature, None, scenario.clone()), &mut writer, &()).await;
        
        // Should emit the scenario finished event
        assert!(writer.events.contains(&"ScenarioFinished".to_string()));
        // Should return the scenario since it's finished
        assert_eq!(result, Some((scenario, None)));
    }

    #[tokio::test]
    async fn test_scenarios_queue_emit_with_rule() {
        let mut queue: ScenariosQueue<TestWorld> = ScenariosQueue::new();
        let mut writer = MockWriter::new();
        let feature = create_test_feature();
        let rule = create_test_rule();
        let scenario = create_test_scenario();
        
        queue.0.push(Event::new(
            RetryableScenario {
                event: event::Scenario::Started,
                retries: None,
            }
        ));
        
        let result = (&mut queue).emit((feature, Some(rule), scenario), &mut writer, &()).await;
        
        // Should emit the scenario event within a rule context
        assert!(writer.events.contains(&"ScenarioStarted".to_string()));
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_scenarios_queue_emit_with_retries() {
        let mut queue: ScenariosQueue<TestWorld> = ScenariosQueue::new();
        let mut writer = MockWriter::new();
        let feature = create_test_feature();
        let scenario = create_test_scenario();
        let retries = Some(3);
        
        queue.0.push(Event::new(
            RetryableScenario {
                event: event::Scenario::Finished,
                retries,
            }
        ));
        
        let result = (&mut queue).emit((feature, None, scenario.clone()), &mut writer, &()).await;
        
        // Should return the scenario with retries
        assert_eq!(result, Some((scenario, retries)));
    }

    #[tokio::test]
    async fn test_scenarios_queue_emit_multiple_events() {
        let mut queue: ScenariosQueue<TestWorld> = ScenariosQueue::new();
        let mut writer = MockWriter::new();
        let feature = create_test_feature();
        let scenario = create_test_scenario();
        
        // Add multiple events
        queue.0.push(Event::new(
            RetryableScenario {
                event: event::Scenario::Started,
                retries: None,
            }
        ));
        queue.0.push(Event::new(
            RetryableScenario {
                event: event::Scenario::Finished,
                retries: None,
            }
        ));
        
        let result = (&mut queue).emit((feature, None, scenario.clone()), &mut writer, &()).await;
        
        // Should emit both events
        assert!(writer.events.contains(&"ScenarioStarted".to_string()));
        assert!(writer.events.contains(&"ScenarioFinished".to_string()));
        // Should return the scenario since the last event was finished
        assert_eq!(result, Some((scenario, None)));
        // All events should be processed
        assert_eq!(queue.0.len(), 0);
    }

    #[test]
    fn test_emitter_trait_implementation() {
        let mut queue: ScenariosQueue<TestWorld> = ScenariosQueue::new();
        
        // Should implement Emitter trait
        fn requires_emitter<T: Emitter<TestWorld>>(_: T) {}
        requires_emitter(&mut queue);
    }

    #[test]
    fn test_scenarios_queue_type_definitions() {
        // Test that the type definitions work correctly
        let queue: ScenariosQueue<TestWorld> = ScenariosQueue::new();
        
        // Should be a vector of RetryableScenario events
        let _: &Vec<Event<event::RetryableScenario<TestWorld>>> = &queue.0;
    }

    #[test] 
    fn test_scenarios_queue_fifo_behavior() {
        let mut queue: ScenariosQueue<TestWorld> = ScenariosQueue::new();
        
        // Add events in order
        queue.0.push(Event::new(
            RetryableScenario {
                event: event::Scenario::Started,
                retries: None,
            }
        ));
        queue.0.push(Event::new(
            RetryableScenario {
                event: event::Scenario::Finished,
                retries: None,
            }
        ));
        
        // Should get events in FIFO order
        let first = (&mut queue).current_item();
        assert!(first.is_some());
        if let Some(event) = first {
            assert!(matches!(event.value.event, event::Scenario::Started));
        }
        
        let second = (&mut queue).current_item();
        assert!(second.is_some());
        if let Some(event) = second {
            assert!(matches!(event.value.event, event::Scenario::Finished));
        }
        
        // Should be empty now
        assert!((&mut queue).current_item().is_none());
    }
}