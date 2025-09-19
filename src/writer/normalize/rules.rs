// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! RulesQueue implementation for event normalization.

use crate::{
    Writer,
    event::{self, Retries, Source},
};

use super::{
    queue::Queue,
    emitter::Emitter,
};

// Forward declaration to avoid circular dependency
use super::scenarios::ScenariosQueue;

/// [`Queue`] of all events of a single [`Rule`].
///
/// [`Rule`]: gherkin::Rule
pub type RulesQueue<World> =
    Queue<(Source<gherkin::Scenario>, Option<Retries>), ScenariosQueue<World>>;

impl<'me, World> Emitter<World> for &'me mut RulesQueue<World> {
    type Current = (Source<gherkin::Scenario>, &'me mut ScenariosQueue<World>);
    type Emitted = Source<gherkin::Rule>;
    type EmittedPath = (Source<gherkin::Feature>, Source<gherkin::Rule>);

    fn current_item(self) -> Option<Self::Current> {
        self.fifo.iter_mut().next().map(|((sc, _), ev)| (sc.clone(), ev))
    }

    async fn emit<W: Writer<World>>(
        self,
        (feature, rule): Self::EmittedPath,
        writer: &mut W,
        cli: &W::Cli,
    ) -> Option<Self::Emitted> {
        if let Some(meta) = self.initial.take() {
            writer
                .handle_event(
                    Ok(meta.wrap(event::Cucumber::rule_started(
                        feature.clone(),
                        rule.clone(),
                    ))),
                    cli,
                )
                .await;
        }

        while let Some((scenario, events)) = self.current_item() {
            if let Some(should_be_removed) = events
                .emit(
                    (feature.clone(), Some(rule.clone()), scenario),
                    writer,
                    cli,
                )
                .await
            {
                self.remove(&should_be_removed);
            } else {
                break;
            }
        }

        if let Some(meta) = self.state.take_to_emit() {
            writer
                .handle_event(
                    Ok(meta.wrap(event::Cucumber::rule_finished(
                        feature,
                        rule.clone(),
                    ))),
                    cli,
                )
                .await;
            return Some(rule);
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
    use super::super::FinishedState;
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
                            event::Feature::Rule(_, rule_event) => {
                                match rule_event {
                                    event::Rule::Started => "RuleStarted",
                                    event::Rule::Finished => "RuleFinished",
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
            position: gherkin::LineCol { line: 3, col: 1 },
            examples: Vec::new(),
        })
    }

    #[test]
    fn test_rules_queue_new() {
        let queue: RulesQueue<TestWorld> = RulesQueue::new(Metadata::new(()));
        
        assert!(queue.initial.is_some());
        assert_eq!(queue.fifo.len(), 0);
        assert!(matches!(queue.state, FinishedState::NotFinished));
    }

    #[test]
    fn test_rules_queue_current_item_empty() {
        let mut queue: RulesQueue<TestWorld> = RulesQueue::new(Metadata::new(()));
        
        let current = (&mut queue).current_item();
        assert!(current.is_none());
    }

    #[test]
    fn test_rules_queue_current_item_with_scenario() {
        let mut queue: RulesQueue<TestWorld> = RulesQueue::new(Metadata::new(()));
        let scenario = create_test_scenario();
        let scenarios_queue = ScenariosQueue::new();
        
        queue.fifo.insert((scenario.clone(), None), scenarios_queue);
        
        let current = (&mut queue).current_item();
        assert!(current.is_some());
        if let Some((sc, _)) = current {
            assert_eq!(sc.value.name, "Test Scenario");
        }
    }

    #[tokio::test]
    async fn test_rules_queue_emit_with_initial() {
        let mut queue: RulesQueue<TestWorld> = RulesQueue::new(Metadata::new(()));
        let mut writer = MockWriter::new();
        let feature = create_test_feature();
        let rule = create_test_rule();
        
        // Emit should start by emitting the rule started event
        let result = (&mut queue).emit((feature, rule), &mut writer, &()).await;
        
        // Should emit rule started event
        assert!(writer.events.contains(&"RuleStarted".to_string()));
        // Should not return the rule since it's not finished
        assert!(result.is_none());
        // Initial metadata should be consumed
        assert!(queue.initial.is_none());
    }

    #[tokio::test]
    async fn test_rules_queue_emit_finished() {
        let mut queue: RulesQueue<TestWorld> = RulesQueue::new(Metadata::new(()));
        let mut writer = MockWriter::new();
        let feature = create_test_feature();
        let rule = create_test_rule();
        
        // Mark the queue as finished
        queue.finished(Metadata::new(()));
        
        // Emit should emit the rule finished event and return the rule
        let result = (&mut queue).emit((feature, rule.clone()), &mut writer, &()).await;
        
        // Should emit rule started and finished events
        assert!(writer.events.contains(&"RuleStarted".to_string()));
        assert!(writer.events.contains(&"RuleFinished".to_string()));
        // Should return the rule since it's finished
        assert_eq!(result, Some(rule));
    }

    #[tokio::test]
    async fn test_rules_queue_emit_with_scenarios() {
        let mut queue: RulesQueue<TestWorld> = RulesQueue::new(Metadata::new(()));
        let mut writer = MockWriter::new();
        let feature = create_test_feature();
        let rule = create_test_rule();
        let scenario = create_test_scenario();
        
        // Add a scenario that will finish immediately
        let mut scenarios_queue = ScenariosQueue::new();
        scenarios_queue.0.push(Event::new(
            RetryableScenario {
                event: event::Scenario::Finished,
                retries: None,
            }
        ));
        queue.fifo.insert((scenario.clone(), None), scenarios_queue);
        
        // Mark the queue as finished so it will emit
        queue.finished(Metadata::new(()));
        
        let result = (&mut queue).emit((feature, rule.clone()), &mut writer, &()).await;
        
        // Should process scenarios and finish
        assert_eq!(result, Some(rule));
        // Scenario should be removed from the queue
        assert_eq!(queue.fifo.len(), 0);
    }

    #[test]
    fn test_rules_queue_finished() {
        let mut queue: RulesQueue<TestWorld> = RulesQueue::new(Metadata::new(()));
        
        let meta = Metadata::new(());
        queue.finished(meta);
        
        assert!(matches!(queue.state, FinishedState::FinishedButNotEmitted(_)));
    }

    #[test]
    fn test_rules_queue_remove() {
        let mut queue: RulesQueue<TestWorld> = RulesQueue::new(Metadata::new(()));
        let scenario = create_test_scenario();
        let scenarios_queue = ScenariosQueue::new();
        
        queue.fifo.insert((scenario.clone(), None), scenarios_queue);
        assert_eq!(queue.fifo.len(), 1);
        
        queue.remove(&(scenario, None));
        assert_eq!(queue.fifo.len(), 0);
    }

    #[test]
    fn test_emitter_trait_implementation() {
        let mut queue: RulesQueue<TestWorld> = RulesQueue::new(Metadata::new(()));
        
        // Should implement Emitter trait
        fn requires_emitter<T: Emitter<TestWorld>>(_: T) {}
        requires_emitter(&mut queue);
    }

    #[test]
    fn test_rules_queue_type_definitions() {
        // Test that the type aliases work correctly
        let queue: RulesQueue<TestWorld> = RulesQueue::new(Metadata::new(()));
        
        // Should be a Queue with correct key and value types
        let _: &Queue<(Source<gherkin::Scenario>, Option<Retries>), ScenariosQueue<TestWorld>> = &queue;
    }
}