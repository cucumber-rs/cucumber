// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! CucumberQueue and FeatureQueue implementations for event normalization.

use either::Either;

use crate::{
    Event, Writer,
    event::{self, Retries, Source},
};

use super::{
    queue::Queue,
    emitter::Emitter,
};

// Forward declarations for circular dependencies
use super::rules::RulesQueue;
use super::scenarios::ScenariosQueue;

/// [`Queue`] of all incoming events.
pub type CucumberQueue<World> =
    Queue<Source<gherkin::Feature>, FeatureQueue<World>>;

impl<World> CucumberQueue<World> {
    /// Inserts a new [`Feature`] on [`event::Feature::Started`].
    ///
    /// [`Feature`]: gherkin::Feature
    pub fn new_feature(&mut self, feat: Event<Source<gherkin::Feature>>) {
        let (feat, meta) = feat.split();
        drop(self.fifo.insert(feat, FeatureQueue::new(meta)));
    }

    /// Marks a [`Feature`] as finished on [`event::Feature::Finished`].
    ///
    /// We don't emit it by the way, as there may be other in-progress
    /// [`Feature`]s holding the output.
    ///
    /// [`Feature`]: gherkin::Feature
    pub fn feature_finished(&mut self, feat: Event<&Source<gherkin::Feature>>) {
        let (feat, meta) = feat.split();
        self.fifo
            .get_mut(feat)
            .unwrap_or_else(|| panic!("no `Feature: {}`", feat.name))
            .finished(meta);
    }

    /// Inserts a new [`Rule`] on [`event::Rule::Started`].
    ///
    /// [`Rule`]: gherkin::Rule
    pub fn new_rule(
        &mut self,
        feat: &Source<gherkin::Feature>,
        rule: Event<Source<gherkin::Rule>>,
    ) {
        self.fifo
            .get_mut(feat)
            .unwrap_or_else(|| panic!("no `Feature: {}`", feat.name))
            .new_rule(rule);
    }

    /// Marks a [`Rule`] as finished on [`event::Rule::Finished`].
    ///
    /// We don't emit it by the way, as there may be other in-progress [`Rule`]s
    /// holding the output.
    ///
    /// [`Rule`]: gherkin::Rule
    pub fn rule_finished(
        &mut self,
        feat: &Source<gherkin::Feature>,
        rule: Event<Source<gherkin::Rule>>,
    ) {
        self.fifo
            .get_mut(feat)
            .unwrap_or_else(|| panic!("no `Feature: {}`", feat.name))
            .rule_finished(rule);
    }

    /// Inserts a new [`event::Scenario::Started`].
    pub fn insert_scenario_event(
        &mut self,
        feat: &Source<gherkin::Feature>,
        rule: Option<Source<gherkin::Rule>>,
        scenario: Source<gherkin::Scenario>,
        event: Event<event::RetryableScenario<World>>,
    ) {
        self.fifo
            .get_mut(feat)
            .unwrap_or_else(|| panic!("no `Feature: {}`", feat.name))
            .insert_scenario_event(rule, scenario, event.retries, event);
    }
}

impl<'me, World> Emitter<World> for &'me mut CucumberQueue<World> {
    type Current = (Source<gherkin::Feature>, &'me mut FeatureQueue<World>);
    type Emitted = Source<gherkin::Feature>;
    type EmittedPath = ();

    fn current_item(self) -> Option<Self::Current> {
        self.fifo.iter_mut().next().map(|(f, ev)| (f.clone(), ev))
    }

    async fn emit<W: Writer<World>>(
        self,
        (): (),
        writer: &mut W,
        cli: &W::Cli,
    ) -> Option<Self::Emitted> {
        if let Some((f, events)) = self.current_item() {
            if let Some(meta) = events.initial.take() {
                writer
                    .handle_event(
                        Ok(meta
                            .wrap(event::Cucumber::feature_started(f.clone()))),
                        cli,
                    )
                    .await;
            }

            while let Some(scenario_or_rule_to_remove) =
                events.emit(f.clone(), writer, cli).await
            {
                events.remove(&scenario_or_rule_to_remove);
            }

            if let Some(meta) = events.state.take_to_emit() {
                writer
                    .handle_event(
                        Ok(meta.wrap(event::Cucumber::feature_finished(
                            f.clone(),
                        ))),
                        cli,
                    )
                    .await;
                return Some(f.clone());
            }
        }
        None
    }
}

/// Either a [`Rule`] or a [`Scenario`].
///
/// [`Rule`]: gherkin::Rule
/// [`Scenario`]: gherkin::Scenario
pub type RuleOrScenario =
    Either<Source<gherkin::Rule>, (Source<gherkin::Scenario>, Option<Retries>)>;

/// Either a [`Rule`]'s or a [`Scenario`]'s [`Queue`].
///
/// [`Rule`]: gherkin::Rule
/// [`Scenario`]: gherkin::Scenario
pub type RuleOrScenarioQueue<World> =
    Either<RulesQueue<World>, ScenariosQueue<World>>;

/// Either a [`Rule`]'s or a [`Scenario`]'s [`Queue`] with the corresponding
/// [`Rule`] or [`Scenario`] which is currently being outputted.
///
/// [`Rule`]: gherkin::Rule
/// [`Scenario`]: gherkin::Scenario
pub type NextRuleOrScenario<'events, World> = Either<
    (Source<gherkin::Rule>, &'events mut RulesQueue<World>),
    (Source<gherkin::Scenario>, &'events mut ScenariosQueue<World>),
>;

/// [`Queue`] of all events of a single [`Feature`].
///
/// [`Feature`]: gherkin::Feature
pub type FeatureQueue<World> = Queue<RuleOrScenario, RuleOrScenarioQueue<World>>;

impl<World> FeatureQueue<World> {
    /// Inserts a new [`Rule`].
    ///
    /// [`Rule`]: gherkin::Rule
    pub fn new_rule(&mut self, rule: Event<Source<gherkin::Rule>>) {
        let (rule, meta) = rule.split();
        drop(
            self.fifo.insert(
                Either::Left(rule),
                Either::Left(RulesQueue::new(meta)),
            ),
        );
    }

    /// Marks a [`Rule`] as finished on [`event::Rule::Finished`].
    ///
    /// [`Rule`]: gherkin::Rule
    pub fn rule_finished(&mut self, rule: Event<Source<gherkin::Rule>>) {
        let (rule, meta) = rule.split();
        match self.fifo.get_mut(&Either::Left(rule)) {
            Some(Either::Left(ev)) => {
                ev.finished(meta);
            }
            Some(Either::Right(_)) | None => unreachable!(),
        }
    }

    /// Inserts a new [`Scenario`] event.
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub fn insert_scenario_event(
        &mut self,
        rule: Option<Source<gherkin::Rule>>,
        scenario: Source<gherkin::Scenario>,
        retries: Option<Retries>,
        ev: Event<event::RetryableScenario<World>>,
    ) {
        if let Some(r) = rule {
            match self
                .fifo
                .get_mut(&Either::Left(r.clone()))
                .unwrap_or_else(|| panic!("no `Rule: {}`", r.name))
            {
                Either::Left(rules) => rules
                    .fifo
                    .entry((scenario, retries))
                    .or_insert_with(ScenariosQueue::new)
                    .0
                    .push(ev),
                Either::Right(_) => unreachable!(),
            }
        } else {
            match self
                .fifo
                .entry(Either::Right((scenario, retries)))
                .or_insert_with(|| Either::Right(ScenariosQueue::new()))
            {
                Either::Right(events) => events.0.push(ev),
                Either::Left(_) => unreachable!(),
            }
        }
    }
}

impl<'me, World> Emitter<World> for &'me mut FeatureQueue<World> {
    type Current = NextRuleOrScenario<'me, World>;
    type Emitted = RuleOrScenario;
    type EmittedPath = Source<gherkin::Feature>;

    fn current_item(self) -> Option<Self::Current> {
        Some(match self.fifo.iter_mut().next()? {
            (Either::Left(rule), Either::Left(events)) => {
                Either::Left((rule.clone(), events))
            }
            (Either::Right((scenario, _)), Either::Right(events)) => {
                Either::Right((scenario.clone(), events))
            }
            _ => unreachable!(),
        })
    }

    async fn emit<W: Writer<World>>(
        self,
        path: Self::EmittedPath,
        writer: &mut W,
        cli: &W::Cli,
    ) -> Option<Self::Emitted> {
        match self.current_item()? {
            Either::Left((rule, events)) => {
                events.emit((path, rule), writer, cli).await.map(Either::Left)
            }
            Either::Right((scenario, events)) => events
                .emit((path, None, scenario), writer, cli)
                .await
                .map(Either::Right),
        }
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
                    Cucumber::Started => "Started",
                    Cucumber::ParsingFinished { .. } => "ParsingFinished",
                    Cucumber::Finished => "Finished",
                    Cucumber::Feature(_, feature_event) => {
                        match feature_event {
                            event::Feature::Started => "FeatureStarted",
                            event::Feature::Finished => "FeatureFinished",
                            _ => "Feature",
                        }
                    }
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
    fn test_cucumber_queue_new_feature() {
        let mut queue = CucumberQueue::new(Metadata::new(()));
        let feature = create_test_feature();
        let feature_event = Event::new(feature.clone());
        
        queue.new_feature(feature_event);
        
        assert!(queue.fifo.contains_key(&feature));
        assert_eq!(queue.fifo.len(), 1);
    }

    #[test]
    fn test_cucumber_queue_feature_finished() {
        let mut queue = CucumberQueue::new(Metadata::new(()));
        let feature = create_test_feature();
        let feature_event = Event::new(feature.clone());
        
        // First add the feature
        queue.new_feature(feature_event);
        
        // Then mark it as finished
        let finish_event = Event::new(&feature);
        queue.feature_finished(finish_event);
        
        // Check that the feature queue is marked as finished
        let feature_queue = queue.fifo.get(&feature).unwrap();
        assert!(matches!(feature_queue.state, FinishedState::FinishedButNotEmitted(_)));
    }

    #[test]
    fn test_cucumber_queue_new_rule() {
        let mut queue = CucumberQueue::new(Metadata::new(()));
        let feature = create_test_feature();
        let rule = create_test_rule();
        
        // First add the feature
        queue.new_feature(Event::new(feature.clone()));
        
        // Then add a rule to it
        queue.new_rule(&feature, Event::new(rule.clone()));
        
        let feature_queue = queue.fifo.get(&feature).unwrap();
        assert!(feature_queue.fifo.contains_key(&Either::Left(rule)));
    }

    #[test]
    fn test_cucumber_queue_rule_finished() {
        let mut queue = CucumberQueue::new(Metadata::new(()));
        let feature = create_test_feature();
        let rule = create_test_rule();
        
        // Setup feature and rule
        queue.new_feature(Event::new(feature.clone()));
        queue.new_rule(&feature, Event::new(rule.clone()));
        
        // Mark rule as finished
        queue.rule_finished(&feature, Event::new(rule.clone()));
        
        let feature_queue = queue.fifo.get(&feature).unwrap();
        let rule_queue = feature_queue.fifo.get(&Either::Left(rule)).unwrap();
        if let Either::Left(rule_queue) = rule_queue {
            assert!(matches!(rule_queue.state, FinishedState::FinishedButNotEmitted(_)));
        } else {
            panic!("Expected rule queue");
        }
    }

    #[test]
    fn test_cucumber_queue_insert_scenario_event() {
        let mut queue = CucumberQueue::new(Metadata::new(()));
        let feature = create_test_feature();
        let scenario = create_test_scenario();
        
        // Setup feature
        queue.new_feature(Event::new(feature.clone()));
        
        // Add scenario event
        let scenario_event = Event::new(
            RetryableScenario {
                event: event::Scenario::Started,
                retries: None,
            }
        );
        queue.insert_scenario_event(&feature, None, scenario.clone(), scenario_event);
        
        let feature_queue = queue.fifo.get(&feature).unwrap();
        assert!(feature_queue.fifo.contains_key(&Either::Right((scenario, None))));
    }

    #[tokio::test]
    async fn test_cucumber_queue_emit() {
        let mut queue = CucumberQueue::new(Metadata::new(()));
        let mut writer = MockWriter::new();
        let feature = create_test_feature();
        
        // Add and finish a feature
        queue.new_feature(Event::new(feature.clone()));
        queue.feature_finished(Event::new(&feature));
        
        // Emit should handle the feature
        let result = (&mut queue).emit((), &mut writer, &EmptyCli).await;
        
        // Should emit feature started and finished events
        assert!(writer.events.contains(&"FeatureStarted".to_string()));
        assert!(writer.events.contains(&"FeatureFinished".to_string()));
        assert_eq!(result, Some(feature));
    }

    #[test]
    fn test_feature_queue_new_rule() {
        let mut feature_queue = FeatureQueue::new(Metadata::new(()));
        let rule = create_test_rule();
        
        feature_queue.new_rule(Event::new(rule.clone()));
        
        assert!(feature_queue.fifo.contains_key(&Either::Left(rule)));
        assert_eq!(feature_queue.fifo.len(), 1);
    }

    #[test]
    fn test_feature_queue_rule_finished() {
        let mut feature_queue = FeatureQueue::new(Metadata::new(()));
        let rule = create_test_rule();
        
        // Add rule first
        feature_queue.new_rule(Event::new(rule.clone()));
        
        // Mark as finished
        feature_queue.rule_finished(Event::new(rule.clone()));
        
        let rule_queue = feature_queue.fifo.get(&Either::Left(rule)).unwrap();
        if let Either::Left(rule_queue) = rule_queue {
            assert!(matches!(rule_queue.state, FinishedState::FinishedButNotEmitted(_)));
        }
    }

    #[test]
    fn test_feature_queue_insert_scenario_event_with_rule() {
        let mut feature_queue = FeatureQueue::new(Metadata::new(()));
        let rule = create_test_rule();
        let scenario = create_test_scenario();
        
        // Add rule first
        feature_queue.new_rule(Event::new(rule.clone()));
        
        // Add scenario to rule
        let scenario_event = Event::new(
            RetryableScenario {
                event: event::Scenario::Started,
                retries: None,
            }
        );
        feature_queue.insert_scenario_event(Some(rule.clone()), scenario.clone(), None, scenario_event);
        
        // Check that scenario was added to the rule
        let rule_queue = feature_queue.fifo.get(&Either::Left(rule)).unwrap();
        if let Either::Left(rule_queue) = rule_queue {
            assert!(rule_queue.fifo.contains_key(&(scenario, None)));
        }
    }

    #[test]
    fn test_feature_queue_insert_scenario_event_without_rule() {
        let mut feature_queue = FeatureQueue::new(Metadata::new(()));
        let scenario = create_test_scenario();
        
        // Add scenario directly to feature
        let scenario_event = Event::new(
            RetryableScenario {
                event: event::Scenario::Started,
                retries: None,
            }
        );
        feature_queue.insert_scenario_event(None, scenario.clone(), None, scenario_event);
        
        // Check that scenario was added directly to feature
        assert!(feature_queue.fifo.contains_key(&Either::Right((scenario, None))));
    }
}