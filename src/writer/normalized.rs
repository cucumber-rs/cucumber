// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! [`Writer`]-wrapper for outputting events in a normalized readable order.

use std::{hash::Hash, sync::Arc};

use async_trait::async_trait;
use either::Either;
use linked_hash_map::LinkedHashMap;

use crate::{event, OutputtedWriter, World, Writer};

/// Wrapper for a [`Writer`] implementation for outputting events in a
/// normalized readable order.
///
/// Doesn't output anything by itself, but rather is used as a combinator for
/// rearranging events and sourcing them to the underlying [`Writer`].
///
/// If some [`Feature`]([`Rule`]/[`Scenario`]/[`Step`]) has started to be
/// written into an output, then it will be written uninterruptedly until its
/// end, even if some other [`Feature`]s have finished their execution. It makes
/// much easier to understand what is really happening in the running
/// [`Feature`].
///
/// [`Feature`]: gherkin::Feature
/// [`Rule`]: gherkin::Rule
/// [`Scenario`]: gherkin::Scenario
/// [`Step`]: gherkin::Step
#[derive(Debug)]
pub struct Normalized<World, Writer> {
    /// [`Writer`] to normalize output of.
    writer: Writer,

    /// Normalization queue of happened events.
    queue: CucumberQueue<World>,
}

impl<W: World, Writer> Normalized<W, Writer> {
    /// Creates a new [`Normalized`] wrapper, which will rearrange events and
    /// feed them to the given [`Writer`].
    pub fn new(writer: Writer) -> Self {
        Self {
            writer,
            queue: CucumberQueue::new(),
        }
    }
}

#[async_trait(?Send)]
impl<World, Wr: Writer<World>> Writer<World> for Normalized<World, Wr> {
    async fn handle_event(&mut self, ev: event::Cucumber<World>) {
        use event::{Cucumber, Feature, Rule};

        match ev {
            Cucumber::ParsingError(err) => {
                self.writer.handle_event(Cucumber::ParsingError(err)).await;
            }
            Cucumber::Started => {
                self.writer.handle_event(Cucumber::Started).await;
            }
            Cucumber::Finished => self.queue.finished(),
            Cucumber::Feature(f, ev) => match ev {
                Feature::Started => self.queue.new_feature(f),
                Feature::Scenario(s, ev) => {
                    self.queue.insert_scenario_event(&f, None, s, ev);
                }
                Feature::Finished => self.queue.feature_finished(&f),
                Feature::Rule(r, ev) => match ev {
                    Rule::Started => self.queue.new_rule(&f, r),
                    Rule::Scenario(s, ev) => {
                        self.queue.insert_scenario_event(&f, Some(r), s, ev);
                    }
                    Rule::Finished => self.queue.rule_finished(&f, r),
                },
            },
        }

        while let Some(feature_to_remove) =
            self.queue.emit_feature_events(&mut self.writer).await
        {
            self.queue.remove(&feature_to_remove);
        }

        if self.queue.is_finished() {
            self.writer.handle_event(Cucumber::Finished).await;
        }
    }
}

#[async_trait(?Send)]
impl<'val, W, Wr, Out> OutputtedWriter<'val, W, Out> for Normalized<W, Wr>
where
    Wr: OutputtedWriter<'val, W, Out>,
    Out: 'val,
{
    async fn write(&mut self, val: Out)
    where
        'val: 'async_trait,
    {
        self.writer.write(val).await;
    }
}

/// Normalization queue for all incoming events.
///
/// We use [`LinkedHashMap`] everywhere throughout this module to ensure FIFO
/// queue for our events. This means by calling [`next()`] we reliably get
/// the currently outputting item's events. We're doing that until it yields
/// event that corresponds to item being finished, after which we remove the
/// current item, as all its events have been printed out and we should do it
/// all over again with a [`next()`] item.
///
/// [`next()`]: std::iter::Iterator::next()
#[derive(Debug)]
struct Queue<K: Eq + Hash, V> {
    started_emitted: bool,
    queue: LinkedHashMap<K, V>,
    finished: bool,
}

impl<K: Eq + Hash, V> Queue<K, V> {
    /// Creates a new [`Queue`].
    fn new() -> Self {
        Self {
            started_emitted: false,
            queue: LinkedHashMap::new(),
            finished: false,
        }
    }

    /// Marks that [`Queue`]'s started event was emitted.
    fn started_emitted(&mut self) {
        self.started_emitted = true;
    }

    /// Checks if [`Queue`]'s started event was emitted.
    fn is_started_emitted(&self) -> bool {
        self.started_emitted
    }

    /// Marks this [`Queue`] as finished.
    fn finished(&mut self) {
        self.finished = true;
    }

    /// Checks whether [`Queue`] has been received.
    fn is_finished(&self) -> bool {
        self.finished
    }

    fn remove(&mut self, key: &K) {
        drop(self.queue.remove(key));
    }
}

/// Queue of all incoming events.
type CucumberQueue<World> = Queue<Arc<gherkin::Feature>, FeatureQueue<World>>;

impl<World> CucumberQueue<World> {
    /// Inserts a new [`Feature`] on [`event::Feature::Started`].
    ///
    /// [`Feature`]: gherkin::Feature
    fn new_feature(&mut self, feature: Arc<gherkin::Feature>) {
        drop(self.queue.insert(feature, FeatureQueue::new()));
    }

    /// Marks a [`Feature`] as finished on [`event::Feature::Finished`].
    ///
    /// We don't emit it by the way, as there may be other in-progress
    /// [`Feature`]s which hold the output.
    ///
    /// [`Feature`]: gherkin::Feature
    fn feature_finished(&mut self, feature: &gherkin::Feature) {
        self.queue
            .get_mut(feature)
            .unwrap_or_else(|| panic!("No Feature {}", feature.name))
            .finished();
    }

    /// Inserts a new [`Rule`] on [`event::Rule::Started`].
    ///
    /// [`Rule`]: gherkin::Feature
    fn new_rule(
        &mut self,
        feature: &gherkin::Feature,
        rule: Arc<gherkin::Rule>,
    ) {
        self.queue
            .get_mut(feature)
            .unwrap_or_else(|| panic!("No Feature {}", feature.name))
            .new_rule(rule);
    }

    /// Marks [`Rule`] as finished on [`event::Rule::Finished`].
    ///
    /// We don't emit it by the way, as there may be other in-progress [`Rule`]s
    /// which hold the output.
    ///
    /// [`Rule`]: gherkin::Feature
    fn rule_finished(
        &mut self,
        feature: &gherkin::Feature,
        rule: Arc<gherkin::Rule>,
    ) {
        self.queue
            .get_mut(feature)
            .unwrap_or_else(|| panic!("No Feature {}", feature.name))
            .rule_finished(rule);
    }

    /// Inserts a new [`event::Scenario::Started`].
    fn insert_scenario_event(
        &mut self,
        feature: &gherkin::Feature,
        rule: Option<Arc<gherkin::Rule>>,
        scenario: Arc<gherkin::Scenario>,
        event: event::Scenario<World>,
    ) {
        self.queue
            .get_mut(feature)
            .unwrap_or_else(|| panic!("No Feature {}", feature.name))
            .insert_scenario_event(rule, scenario, event);
    }

    /// Returns currently outputted [`Feature`].
    ///
    /// [`Feature`]: gherkin::Feature
    fn next_feature(
        &mut self,
    ) -> Option<(Arc<gherkin::Feature>, &mut FeatureQueue<World>)> {
        self.queue.iter_mut().next().map(|(f, ev)| (f.clone(), ev))
    }

    /// Emits all ready [`Feature`] events. If some [`Feature`] was fully
    /// outputted, returns it. After that it should be [`remove`]d.
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`remove`]: CucumberQueue::remove()
    async fn emit_feature_events<Wr: Writer<World>>(
        &mut self,
        writer: &mut Wr,
    ) -> Option<Arc<gherkin::Feature>> {
        if let Some((f, events)) = self.next_feature() {
            if !events.is_started_emitted() {
                writer
                    .handle_event(event::Cucumber::feature_started(f.clone()))
                    .await;
                events.started_emitted();
            }

            while let Some(scenario_or_rule_to_remove) = events
                .emit_scenario_and_rule_events(f.clone(), writer)
                .await
            {
                events.remove(&scenario_or_rule_to_remove);
            }

            if events.is_finished() {
                writer
                    .handle_event(event::Cucumber::feature_finished(f.clone()))
                    .await;
                return Some(f.clone());
            }
        }
        None
    }
}

/// Queue of all events of a single [`Feature`].
///
/// [`Feature`]: gherkin::Feature
type FeatureQueue<World> = Queue<RuleOrScenario, RuleOrScenarioQueue<World>>;

/// Either a [`gherkin::Rule`] or a [`gherkin::Scenario`].
type RuleOrScenario = Either<Arc<gherkin::Rule>, Arc<gherkin::Scenario>>;

/// Either a [`gherkin::Rule`]'s or a [`gherkin::Scenario`]'s [`Queue`].
type RuleOrScenarioQueue<World> =
    Either<RulesQueue<World>, ScenariosQueue<World>>;

type NextRuleOrScenario<'events, World> = Either<
    (Arc<gherkin::Rule>, &'events mut RulesQueue<World>),
    (Arc<gherkin::Scenario>, &'events mut ScenariosQueue<World>),
>;

impl<World> FeatureQueue<World> {
    /// Emits all ready [`RuleOrScenario`] events. If some [`RuleOrScenario`]
    /// was fully outputted, then returns it. After that it should be
    /// [`remove`]d.
    ///
    /// [`remove`]: FeatureQueue::remove()
    async fn emit_scenario_and_rule_events<Wr: Writer<World>>(
        &mut self,
        feature: Arc<gherkin::Feature>,
        writer: &mut Wr,
    ) -> Option<RuleOrScenario> {
        match self.next_rule_or_scenario()? {
            Either::Left((rule, events)) => events
                .emit_rule_events(feature, rule, writer)
                .await
                .map(Either::Left),
            Either::Right((scenario, events)) => events
                .emit_scenario_events(feature, None, scenario, writer)
                .await
                .map(Either::Right),
        }
    }

    /// Inserts new [`Rule`].
    ///
    /// [`Rule`]: gherkin::Rule
    fn new_rule(&mut self, rule: Arc<gherkin::Rule>) {
        drop(
            self.queue
                .insert(Either::Left(rule), Either::Left(RulesQueue::new())),
        );
    }

    /// Marks [`Rule`] as finished on [`Rule::Finished`].
    ///
    /// [`Rule`]: gherkin::Rule
    /// [`Rule::Finished`]: event::Rule::Finished
    fn rule_finished(&mut self, rule: Arc<gherkin::Rule>) {
        match self.queue.get_mut(&Either::Left(rule)).unwrap() {
            Either::Left(ev) => {
                ev.finished();
            }
            Either::Right(_) => unreachable!(),
        }
    }

    /// Inserts new [`Scenario`] event.
    ///
    /// [`Scenario`]: gherkin::Scenario
    fn insert_scenario_event(
        &mut self,
        rule: Option<Arc<gherkin::Rule>>,
        scenario: Arc<gherkin::Scenario>,
        ev: event::Scenario<World>,
    ) {
        if let Some(rule) = rule {
            match self
                .queue
                .get_mut(&Either::Left(rule.clone()))
                .unwrap_or_else(|| panic!("No Rule {}", rule.name))
            {
                Either::Left(rules) => rules
                    .queue
                    .entry(scenario)
                    .or_insert_with(ScenariosQueue::new)
                    .0
                    .push(ev),
                Either::Right(_) => unreachable!(),
            }
        } else {
            match self
                .queue
                .entry(Either::Right(scenario))
                .or_insert_with(|| Either::Right(ScenariosQueue::new()))
            {
                Either::Right(events) => events.0.push(ev),
                Either::Left(_) => unreachable!(),
            }
        }
    }

    /// Returns currently outputted [`RuleOrScenario`].
    fn next_rule_or_scenario(
        &mut self,
    ) -> Option<NextRuleOrScenario<'_, World>> {
        Some(match self.queue.iter_mut().next()? {
            (Either::Left(rule), Either::Left(events)) => {
                Either::Left((rule.clone(), events))
            }
            (Either::Right(scenario), Either::Right(events)) => {
                Either::Right((scenario.clone(), events))
            }
            _ => unreachable!(),
        })
    }
}

/// Queue of all events of a single [`Rule`].
///
/// [`Rule`]: gherkin::Rule
type RulesQueue<World> = Queue<Arc<gherkin::Scenario>, ScenariosQueue<World>>;

impl<World> RulesQueue<World> {
    /// Returns currently outputted [`Scenario`].
    ///
    /// [`Scenario`]: gherkin::Scenario
    fn next_scenario(
        &mut self,
    ) -> Option<(Arc<gherkin::Scenario>, &mut ScenariosQueue<World>)> {
        self.queue
            .iter_mut()
            .next()
            .map(|(sc, ev)| (sc.clone(), ev))
    }

    /// Emits all ready [`Rule`] events. If some [`Rule`] was fully outputted,
    /// returns it. After that it should be removed.
    ///
    /// [`Rule`]: gherkin::Rule
    async fn emit_rule_events<Wr: Writer<World>>(
        &mut self,
        feature: Arc<gherkin::Feature>,
        rule: Arc<gherkin::Rule>,
        writer: &mut Wr,
    ) -> Option<Arc<gherkin::Rule>> {
        if !self.is_started_emitted() {
            writer
                .handle_event(event::Cucumber::rule_started(
                    feature.clone(),
                    rule.clone(),
                ))
                .await;
            self.started_emitted();
        }

        while let Some((scenario, events)) = self.next_scenario() {
            if let Some(should_be_removed) = events
                .emit_scenario_events(
                    feature.clone(),
                    Some(rule.clone()),
                    scenario,
                    writer,
                )
                .await
            {
                self.remove(&should_be_removed);
            } else {
                break;
            }
        }

        if self.is_finished() {
            writer
                .handle_event(event::Cucumber::rule_finished(
                    feature,
                    rule.clone(),
                ))
                .await;
            return Some(rule);
        }

        None
    }
}

/// Storage for [`Scenario`] events.
///
/// [`Scenario`]: gherkin::Scenario
#[derive(Debug)]
struct ScenariosQueue<World>(Vec<event::Scenario<World>>);

impl<World> ScenariosQueue<World> {
    /// Creates new [`ScenarioEvents`].
    fn new() -> Self {
        Self(Vec::new())
    }

    /// Emits all ready [`Scenario`] events. If some [`Scenario`] was fully
    /// outputted, returns it. After that it should be removed.
    ///
    /// [`Scenario`]: gherkin::Scenario
    async fn emit_scenario_events<Wr: Writer<World>>(
        &mut self,
        feature: Arc<gherkin::Feature>,
        rule: Option<Arc<gherkin::Rule>>,
        scenario: Arc<gherkin::Scenario>,
        writer: &mut Wr,
    ) -> Option<Arc<gherkin::Scenario>> {
        while !self.0.is_empty() {
            let ev = self.0.remove(0);
            let should_be_removed = matches!(ev, event::Scenario::Finished);

            let ev = event::Cucumber::scenario(
                feature.clone(),
                rule.clone(),
                scenario.clone(),
                ev,
            );
            writer.handle_event(ev).await;

            if should_be_removed {
                return Some(scenario.clone());
            }
        }
        None
    }
}
