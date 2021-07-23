//! [`Writer`] for outputting events in readable order.
//!
//! [`Parser`]: crate::Parser

use async_trait::async_trait;
use either::Either;
use linked_hash_map::LinkedHashMap;

use crate::{event, World, Writer};

/// [`Writer`] implementation for outputting events in readable order.
///
/// Does not output anything by itself, rather used as a combinator for
/// rearranging events and sourcing them to the underlying [`Writer`].
/// If some [`Feature`]([`Rule`]/[`Scenario`]/[`Step`]) was outputted, it will
/// be outputted uninterrupted until the end, even if some other [`Feature`]s
/// finished their execution. It makes much easier to understand what is really
/// happening in [`Feature`].
///
/// [`Feature`]: gherkin::Feature
/// [`Rule`]: gherkin::Rule
/// [`Scenario`]: gherkin::Scenario
/// [`Step`]: gherkin::Step
#[derive(Debug)]
pub struct Normalized<World, Writer> {
    writer: Writer,
    queue: Cucumber<World>,
}

impl<W: World, Writer> Normalized<W, Writer> {
    /// Creates new [`Normalized`], which will rearrange events and source them
    /// to given [`Writer`].
    pub fn new(writer: Writer) -> Self {
        Self {
            writer,
            queue: Cucumber::new(),
        }
    }
}

#[async_trait(?Send)]
impl<World, Wr: Writer<World>> Writer<World> for Normalized<World, Wr> {
    async fn handle_event(&mut self, ev: event::Cucumber<World>) {
        match ev {
            event::Cucumber::Started => {
                self.writer.handle_event(event::Cucumber::Started).await;
            }
            event::Cucumber::Finished => self.queue.finished(),
            event::Cucumber::Feature(f, ev) => match ev {
                event::Feature::Started => self.queue.new_feature(f),
                event::Feature::Scenario(s, ev) => {
                    self.queue.insert_scenario_event(&f, s, ev);
                }
                event::Feature::Finished => self.queue.feature_finished(&f),
                event::Feature::Rule(r, ev) => match ev {
                    event::Rule::Started => self.queue.new_rule(&f, r),
                    event::Rule::Scenario(s, ev) => {
                        self.queue.insert_rule_scenario_event(&f, &r, s, ev);
                    }
                    event::Rule::Finished => self.queue.rule_finished(&f, &r),
                },
            },
        }

        while let Some(feature_to_remove) =
            self.queue.emit_feature_events(&mut self.writer).await
        {
            self.queue.remove(&feature_to_remove);
        }

        if self.queue.is_finished() {
            self.writer.handle_event(event::Cucumber::Finished).await;
        }
    }
}

#[derive(Debug)]
struct Cucumber<World> {
    events: LinkedHashMap<gherkin::Feature, FeatureEvents<World>>,
    finished: bool,
}

impl<World> Cucumber<World> {
    fn new() -> Self {
        Self {
            events: LinkedHashMap::new(),
            finished: false,
        }
    }

    fn finished(&mut self) {
        self.finished = true;
    }

    fn is_finished(&self) -> bool {
        self.finished
    }

    fn new_feature(&mut self, feature: gherkin::Feature) {
        drop(self.events.insert(feature, FeatureEvents::new()));
    }

    fn feature_finished(&mut self, feature: &gherkin::Feature) {
        self.events
            .get_mut(feature)
            .unwrap_or_else(|| panic!("No Feature {}", feature.name))
            .finished = true;
    }

    fn new_rule(&mut self, feature: &gherkin::Feature, rule: gherkin::Rule) {
        self.events
            .get_mut(feature)
            .unwrap_or_else(|| panic!("No Feature {}", feature.name))
            .events
            .new_rule(rule);
    }

    fn rule_finished(
        &mut self,
        feature: &gherkin::Feature,
        rule: &gherkin::Rule,
    ) {
        self.events
            .get_mut(feature)
            .unwrap_or_else(|| panic!("No Feature {}", feature.name))
            .events
            .rule_finished(rule);
    }

    fn insert_scenario_event(
        &mut self,
        feature: &gherkin::Feature,
        scenario: gherkin::Scenario,
        event: event::Scenario<World>,
    ) {
        self.events
            .get_mut(feature)
            .unwrap_or_else(|| panic!("No Feature {}", feature.name))
            .events
            .insert_scenario_event(scenario, event);
    }

    fn insert_rule_scenario_event(
        &mut self,
        feature: &gherkin::Feature,
        rule: &gherkin::Rule,
        scenario: gherkin::Scenario,
        event: event::Scenario<World>,
    ) {
        self.events
            .get_mut(feature)
            .unwrap_or_else(|| panic!("No Feature {}", feature.name))
            .events
            .insert_rule_scenario_event(rule, scenario, event);
    }

    fn next_feature(
        &mut self,
    ) -> Option<(gherkin::Feature, &mut FeatureEvents<World>)> {
        self.events.iter_mut().next().map(|(f, ev)| (f.clone(), ev))
    }

    fn remove(&mut self, feature: &gherkin::Feature) {
        drop(self.events.remove(feature));
    }

    async fn emit_feature_events<Wr: Writer<World>>(
        &mut self,
        writer: &mut Wr,
    ) -> Option<gherkin::Feature> {
        if let Some((f, events)) = self.next_feature() {
            if !events.is_started() {
                writer
                    .handle_event(event::Cucumber::feature_started(f.clone()))
                    .await;
                events.started();
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

#[derive(Debug)]
struct FeatureEvents<World> {
    started_emitted: bool,
    events: RulesAndScenarios<World>,
    finished: bool,
}

impl<World> FeatureEvents<World> {
    fn new() -> Self {
        Self {
            started_emitted: false,
            events: RulesAndScenarios::new(),
            finished: false,
        }
    }

    fn is_started(&self) -> bool {
        self.started_emitted
    }

    fn started(&mut self) {
        self.started_emitted = true;
    }

    fn is_finished(&self) -> bool {
        self.finished
    }

    async fn emit_scenario_and_rule_events<Wr: Writer<World>>(
        &mut self,
        feature: gherkin::Feature,
        writer: &mut Wr,
    ) -> Option<RuleOrScenario> {
        match self.events.next_rule_or_scenario() {
            Some(Either::Left((rule, events))) => events
                .emit_rule_events(feature, rule, writer)
                .await
                .map(Either::Left),
            Some(Either::Right((scenario, events))) => events
                .emit_scenario_events(feature, None, scenario, writer)
                .await
                .map(Either::Right),
            None => None,
        }
    }

    fn remove(&mut self, rule_or_scenario: &RuleOrScenario) {
        drop(self.events.0.remove(rule_or_scenario));
    }
}

#[derive(Debug)]
struct RulesAndScenarios<World>(
    LinkedHashMap<RuleOrScenario, RuleOrScenarioEvents<World>>,
);

type RuleOrScenario = Either<gherkin::Rule, gherkin::Scenario>;

type RuleOrScenarioEvents<World> =
    Either<RuleEvents<World>, ScenarioEvents<World>>;

type NextRuleOrScenario<'events, World> = Either<
    (gherkin::Rule, &'events mut RuleEvents<World>),
    (gherkin::Scenario, &'events mut ScenarioEvents<World>),
>;

impl<World> RulesAndScenarios<World> {
    fn new() -> Self {
        RulesAndScenarios(LinkedHashMap::new())
    }

    fn new_rule(&mut self, rule: gherkin::Rule) {
        drop(
            self.0
                .insert(Either::Left(rule), Either::Left(RuleEvents::new())),
        );
    }

    fn rule_finished(&mut self, rule: &gherkin::Rule) {
        match self
            .0
            .get_mut(&Either::Left(rule.clone()))
            .unwrap_or_else(|| panic!("No Rule {}", rule.name))
        {
            Either::Left(ev) => {
                ev.finished = true;
            }
            Either::Right(_) => unreachable!(),
        }
    }

    fn insert_scenario_event(
        &mut self,
        scenario: gherkin::Scenario,
        ev: event::Scenario<World>,
    ) {
        match self
            .0
            .entry(Either::Right(scenario))
            .or_insert_with(|| Either::Right(ScenarioEvents::new()))
        {
            Either::Right(events) => events.0.push(ev),
            Either::Left(_) => unreachable!(),
        }
    }

    fn insert_rule_scenario_event(
        &mut self,
        rule: &gherkin::Rule,
        scenario: gherkin::Scenario,
        ev: event::Scenario<World>,
    ) {
        match self
            .0
            .get_mut(&Either::Left(rule.clone()))
            .unwrap_or_else(|| panic!("No Rule {}", rule.name))
        {
            Either::Left(rules) => rules
                .scenarios
                .entry(scenario)
                .or_insert_with(ScenarioEvents::new)
                .0
                .push(ev),
            Either::Right(_) => unreachable!(),
        }
    }

    fn next_rule_or_scenario(
        &mut self,
    ) -> Option<NextRuleOrScenario<'_, World>> {
        Some(match self.0.iter_mut().next()? {
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

#[derive(Debug)]
struct RuleEvents<World> {
    started_emitted: bool,
    scenarios: LinkedHashMap<gherkin::Scenario, ScenarioEvents<World>>,
    finished: bool,
}

impl<World> RuleEvents<World> {
    fn new() -> Self {
        Self {
            started_emitted: false,
            scenarios: LinkedHashMap::new(),
            finished: false,
        }
    }

    fn next_scenario(
        &mut self,
    ) -> Option<(gherkin::Scenario, &mut ScenarioEvents<World>)> {
        self.scenarios
            .iter_mut()
            .next()
            .map(|(sc, ev)| (sc.clone(), ev))
    }

    async fn emit_rule_events<Wr: Writer<World>>(
        &mut self,
        feature: gherkin::Feature,
        rule: gherkin::Rule,
        writer: &mut Wr,
    ) -> Option<gherkin::Rule> {
        if !self.started_emitted {
            writer
                .handle_event(event::Cucumber::rule_started(
                    feature.clone(),
                    rule.clone(),
                ))
                .await;
            self.started_emitted = true;
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
                drop(self.scenarios.remove(&should_be_removed));
            } else {
                break;
            }
        }

        if self.finished {
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

#[derive(Debug)]
struct ScenarioEvents<World>(Vec<event::Scenario<World>>);

impl<World> ScenarioEvents<World> {
    fn new() -> Self {
        Self(Vec::new())
    }

    async fn emit_scenario_events<Wr: Writer<World>>(
        &mut self,
        feature: gherkin::Feature,
        rule: Option<gherkin::Rule>,
        scenario: gherkin::Scenario,
        writer: &mut Wr,
    ) -> Option<gherkin::Scenario> {
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
