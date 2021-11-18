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

use std::{hash::Hash, mem, sync::Arc};

use async_trait::async_trait;
use derive_more::Deref;
use either::Either;
use linked_hash_map::LinkedHashMap;

use crate::{
    event::{self, Metadata},
    parser, writer, ArbitraryWriter, Event, FailureWriter, Writer,
};

/// Wrapper for a [`Writer`] implementation for outputting events corresponding
/// to _order guarantees_ from the [`Runner`] in a [`Normalized`] readable
/// order.
///
/// Doesn't output anything by itself, but rather is used as a combinator for
/// rearranging events and feeding them to the underlying [`Writer`].
///
/// If some [`Feature`]([`Rule`]/[`Scenario`]/[`Step`]) has started to be
/// written into an output, then it will be written uninterruptedly until its
/// end, even if some other [`Feature`]s have finished their execution. It makes
/// much easier to understand what is really happening in the running
/// [`Feature`] while don't impose any restrictions on the running order.
///
/// [`Feature`]: gherkin::Feature
/// [`Rule`]: gherkin::Rule
/// [`Runner`]: crate::Runner
/// [`Scenario`]: gherkin::Scenario
/// [`Step`]: gherkin::Step
#[derive(Debug, Deref)]
pub struct Normalize<World, Writer> {
    /// Original [`Writer`] to normalize output of.
    #[deref]
    pub writer: Writer,

    /// Normalization queue of happened events.
    queue: CucumberQueue<World>,
}

impl<W, Writer> Normalize<W, Writer> {
    /// Creates a new [`Normalized`] wrapper, which will rearrange [`event`]s
    /// and feed them to the given [`Writer`].
    #[must_use]
    pub fn new(writer: Writer) -> Self {
        Self {
            writer,
            queue: CucumberQueue::new(Metadata::new(())),
        }
    }
}

#[async_trait(?Send)]
impl<World, Wr: Writer<World>> Writer<World> for Normalize<World, Wr> {
    type Cli = Wr::Cli;

    async fn handle_event(
        &mut self,
        ev: parser::Result<Event<event::Cucumber<World>>>,
        cli: &Self::Cli,
    ) {
        use event::{Cucumber, Feature, Rule};

        // Once `Cucumber::Finished` is emitted, we just pass events through,
        // without any normalization.
        // This is done to avoid panic if this `Writer` happens to be wrapped
        // inside `writer::Repeat` or similar.
        if self.queue.is_finished_and_emitted() {
            self.writer.handle_event(ev, cli).await;
            return;
        }

        match ev.map(Event::split) {
            res @ (Err(_) | Ok((Cucumber::Started, _))) => {
                self.writer
                    .handle_event(res.map(|(ev, meta)| meta.insert(ev)), cli)
                    .await;
            }
            Ok((Cucumber::Finished, meta)) => self.queue.finished(meta),
            Ok((Cucumber::Feature(f, ev), meta)) => match ev {
                Feature::Started => self.queue.new_feature(meta.wrap(f)),
                Feature::Scenario(s, ev) => {
                    self.queue.insert_scenario_event(
                        &f,
                        None,
                        s,
                        meta.wrap(ev),
                    );
                }
                Feature::Finished => self.queue.feature_finished(meta.wrap(&f)),
                Feature::Rule(r, ev) => match ev {
                    Rule::Started => self.queue.new_rule(&f, meta.wrap(r)),
                    Rule::Scenario(s, ev) => {
                        self.queue.insert_scenario_event(
                            &f,
                            Some(r),
                            s,
                            meta.wrap(ev),
                        );
                    }
                    Rule::Finished => {
                        self.queue.rule_finished(&f, meta.wrap(r));
                    }
                },
            },
        }

        while let Some(feature_to_remove) =
            self.queue.emit((), &mut self.writer, cli).await
        {
            self.queue.remove(&feature_to_remove);
        }

        if let Some(meta) = self.queue.state.take_to_emit() {
            self.writer
                .handle_event(Ok(meta.wrap(Cucumber::Finished)), cli)
                .await;
        }
    }
}

#[async_trait(?Send)]
impl<'val, W, Wr, Val> ArbitraryWriter<'val, W, Val> for Normalize<W, Wr>
where
    Wr: ArbitraryWriter<'val, W, Val>,
    Val: 'val,
{
    async fn write(&mut self, val: Val)
    where
        'val: 'async_trait,
    {
        self.writer.write(val).await;
    }
}

impl<W, Wr> FailureWriter<W> for Normalize<W, Wr>
where
    Wr: FailureWriter<W>,
    Self: Writer<W>,
{
    fn failed_steps(&self) -> usize {
        self.writer.failed_steps()
    }

    fn parsing_errors(&self) -> usize {
        self.writer.parsing_errors()
    }

    fn hook_errors(&self) -> usize {
        self.writer.hook_errors()
    }
}

impl<W, Wr: writer::NotTransformEvents> writer::NotTransformEvents
    for Normalize<W, Wr>
{
}

/// Marker trait indicating that [`Writer`] can accept events in
/// [happens-before] order. This means one of two things:
///
/// 1. [`Writer`] doesn't depend on events ordering.
///    For example, [`Writer`] which prints only [`Failed`] [`Step`]s.
/// 2. [`Writer`] does depend on events ordering, but implements some logic to
///    rearrange them.
///    For example [`Normalized`] wrapper will rearrange events
///    and pass them to the underlying [`Writer`], like [`Runner`] wasn't
///    concurrent at all.
///
/// [happened-before]: https://en.wikipedia.org/wiki/Happened-before
/// [`Step`]: gherkin::Step
/// [`Failed`]: event::Step::Failed
/// [`Runner`]: crate::Runner
pub trait Normalized {}

impl<World, Writer> Normalized for Normalize<World, Writer> {}

/// Normalization queue for incoming events.
///
/// We use [`LinkedHashMap`] everywhere throughout this module to ensure FIFO
/// queue for our events. This means by calling [`next()`] we reliably get the
/// currently outputting item's events. We're doing that until it yields an
/// event that corresponds to the item being finished, after which we remove the
/// current item, as all its events have been printed out and we should do it
/// all over again with a [`next()`] item.
///
/// [`next()`]: std::iter::Iterator::next()
#[derive(Debug)]
struct Queue<K: Eq + Hash, V> {
    /// Underlying FIFO queue of values.
    queue: LinkedHashMap<K, V>,

    /// Initial [`Metadata`] of this [`Queue`] creation.
    ///
    /// If this value is [`Some`], then `Started` [`Event`] hasn't been passed
    /// on to the inner [`Writer`] yet.
    initial: Option<Metadata>,

    /// [`FinishedState`] of this [`Queue`].
    state: FinishedState,
}

impl<K: Eq + Hash, V> Queue<K, V> {
    /// Creates a new normalization [`Queue`] with an initial metadata.
    fn new(initial: Metadata) -> Self {
        Self {
            queue: LinkedHashMap::new(),
            initial: Some(initial),
            state: FinishedState::NotFinished,
        }
    }

    /// Marks this [`Queue`] as [`FinishedButNotEmitted`].
    ///
    /// [`FinishedButNotEmitted`]: FinishedState::FinishedButNotEmitted
    fn finished(&mut self, meta: Metadata) {
        self.state = FinishedState::FinishedButNotEmitted(meta);
    }

    /// Checks whether this [`Queue`] transited to [`FinishedAndEmitted`] state.
    ///
    /// [`FinishedAndEmitted`]: FinishedState::FinishedAndEmitted
    fn is_finished_and_emitted(&self) -> bool {
        matches!(self.state, FinishedState::FinishedAndEmitted)
    }

    /// Removes the given `key` from this [`Queue`].
    fn remove(&mut self, key: &K) {
        drop(self.queue.remove(key));
    }
}

/// Finishing state of a [`Queue`].
#[derive(Clone, Copy, Debug)]
enum FinishedState {
    /// `Finished` event hasn't been encountered yet.
    NotFinished,

    /// `Finished` event has been encountered, but not passed to the inner
    /// [`Writer`] yet.
    ///
    /// This happens when output is busy due to outputting some other item.
    FinishedButNotEmitted(Metadata),

    /// `Finished` event has been encountered and passed to the inner
    /// [`Writer`].
    FinishedAndEmitted,
}

impl FinishedState {
    /// Returns [`Metadata`] of this [`FinishedState::FinishedButNotEmitted`],
    /// and makes it [`FinishedAndEmitted`].
    ///
    /// [`FinishedAndEmitted`]: FinishedState::FinishedAndEmitted
    fn take_to_emit(&mut self) -> Option<Metadata> {
        let current = mem::replace(self, Self::FinishedAndEmitted);
        if let Self::FinishedButNotEmitted(meta) = current {
            Some(meta)
        } else {
            *self = current;
            None
        }
    }
}

/// [`Queue`] which can remember its current item ([`Feature`], [`Rule`],
/// [`Scenario`] or [`Step`]) and pass events connected to it to the provided
/// [`Writer`].
///
/// [`Feature`]: gherkin::Feature
/// [`Rule`]: gherkin::Rule
/// [`Scenario`]: gherkin::Scenario
/// [`Step`]: gherkin::Step
#[async_trait(?Send)]
trait Emitter<World> {
    /// Currently outputted key and value from this [`Queue`].
    type Current;

    /// Currently outputted item ([`Feature`], [`Rule`], [`Scenario`] or
    /// [`Step`]). If returned from [`Self::emit()`], means that all events
    /// associated with that item were passed to the underlying [`Writer`], so
    /// should be removed from the [`Queue`].
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Rule`]: gherkin::Rule
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
    type Emitted;

    /// Path to the [`Self::Emitted`] item. For [`Feature`] its `()`, as it's
    /// top-level item. For [`Scenario`] it's
    /// `(`[`Feature`]`, `[`Option`]`<`[`Rule`]`>)`, because [`Scenario`]
    /// definitely has parent [`Feature`] and optionally can have parent
    /// [`Rule`].
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Rule`]: gherkin::Rule
    /// [`Scenario`]: gherkin::Scenario
    type EmittedPath;

    /// Currently outputted key and value from this [`Queue`].
    fn current_item(self) -> Option<Self::Current>;

    /// Passes events of the current item ([`Feature`], [`Rule`], [`Scenario`]
    /// or [`Step`]) to the provided [`Writer`].
    ///
    /// If this method returns [`Some`], then all events of the current item
    /// were passed to the provided [`Writer`] and that means it should be
    /// [`remove`]d.
    ///
    /// [`remove`]: Queue::remove()
    /// [`Feature`]: gherkin::Feature
    /// [`Rule`]: gherkin::Rule
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
    async fn emit<W: Writer<World>>(
        self,
        path: Self::EmittedPath,
        writer: &mut W,
        cli: &W::Cli,
    ) -> Option<Self::Emitted>;
}

/// [`Queue`] of all incoming events.
type CucumberQueue<World> = Queue<Arc<gherkin::Feature>, FeatureQueue<World>>;

impl<World> CucumberQueue<World> {
    /// Inserts a new [`Feature`] on [`event::Feature::Started`].
    ///
    /// [`Feature`]: gherkin::Feature
    fn new_feature(&mut self, feat: Event<Arc<gherkin::Feature>>) {
        let (feat, meta) = feat.split();
        drop(self.queue.insert(feat, FeatureQueue::new(meta)));
    }

    /// Marks a [`Feature`] as finished on [`event::Feature::Finished`].
    ///
    /// We don't emit it by the way, as there may be other in-progress
    /// [`Feature`]s holding the output.
    ///
    /// [`Feature`]: gherkin::Feature
    fn feature_finished(&mut self, feat: Event<&gherkin::Feature>) {
        let (feat, meta) = feat.split();
        self.queue
            .get_mut(feat)
            .unwrap_or_else(|| panic!("No Feature {}", feat.name))
            .finished(meta);
    }

    /// Inserts a new [`Rule`] on [`event::Rule::Started`].
    ///
    /// [`Rule`]: gherkin::Feature
    fn new_rule(
        &mut self,
        feat: &gherkin::Feature,
        rule: Event<Arc<gherkin::Rule>>,
    ) {
        self.queue
            .get_mut(feat)
            .unwrap_or_else(|| panic!("No Feature {}", feat.name))
            .new_rule(rule);
    }

    /// Marks a [`Rule`] as finished on [`event::Rule::Finished`].
    ///
    /// We don't emit it by the way, as there may be other in-progress [`Rule`]s
    /// holding the output.
    ///
    /// [`Rule`]: gherkin::Feature
    fn rule_finished(
        &mut self,
        feat: &gherkin::Feature,
        rule: Event<Arc<gherkin::Rule>>,
    ) {
        self.queue
            .get_mut(feat)
            .unwrap_or_else(|| panic!("No Feature {}", feat.name))
            .rule_finished(rule);
    }

    /// Inserts a new [`event::Scenario::Started`].
    fn insert_scenario_event(
        &mut self,
        feat: &gherkin::Feature,
        rule: Option<Arc<gherkin::Rule>>,
        scenario: Arc<gherkin::Scenario>,
        event: Event<event::Scenario<World>>,
    ) {
        self.queue
            .get_mut(feat)
            .unwrap_or_else(|| panic!("No Feature {}", feat.name))
            .insert_scenario_event(rule, scenario, event);
    }
}

#[async_trait(?Send)]
impl<'me, World> Emitter<World> for &'me mut CucumberQueue<World> {
    type Current = (Arc<gherkin::Feature>, &'me mut FeatureQueue<World>);
    type Emitted = Arc<gherkin::Feature>;
    type EmittedPath = ();

    fn current_item(self) -> Option<Self::Current> {
        self.queue
            .iter_mut()
            .next()
            .map(|(f, ev)| (Arc::clone(f), ev))
    }

    async fn emit<W: Writer<World>>(
        self,
        _: (),
        writer: &mut W,
        cli: &W::Cli,
    ) -> Option<Self::Emitted> {
        if let Some((f, events)) = self.current_item() {
            if let Some(meta) = events.initial.take() {
                writer
                    .handle_event(
                        Ok(meta.wrap(event::Cucumber::feature_started(
                            Arc::clone(&f),
                        ))),
                        cli,
                    )
                    .await;
            }

            while let Some(scenario_or_rule_to_remove) =
                events.emit(Arc::clone(&f), writer, cli).await
            {
                events.remove(&scenario_or_rule_to_remove);
            }

            if let Some(meta) = events.state.take_to_emit() {
                writer
                    .handle_event(
                        Ok(meta.wrap(event::Cucumber::feature_finished(
                            Arc::clone(&f),
                        ))),
                        cli,
                    )
                    .await;
                return Some(Arc::clone(&f));
            }
        }
        None
    }
}

/// Either a [`Rule`] or a [`Scenario`].
///
/// [`Rule`]: gherkin::Rule
/// [`Scenario`]: gherkin::Scenario
type RuleOrScenario = Either<Arc<gherkin::Rule>, Arc<gherkin::Scenario>>;

/// Either a [`Rule`]'s or a [`Scenario`]'s [`Queue`].
///
/// [`Rule`]: gherkin::Rule
/// [`Scenario`]: gherkin::Scenario
type RuleOrScenarioQueue<World> =
    Either<RulesQueue<World>, ScenariosQueue<World>>;

/// Either a [`Rule`]'s or a [`Scenario`]'s [`Queue`] with the corresponding
/// [`Rule`] or [`Scenario`] which is currently being outputted.
///
/// [`Rule`]: gherkin::Rule
/// [`Scenario`]: gherkin::Scenario
type NextRuleOrScenario<'events, World> = Either<
    (Arc<gherkin::Rule>, &'events mut RulesQueue<World>),
    (Arc<gherkin::Scenario>, &'events mut ScenariosQueue<World>),
>;

/// [`Queue`] of all events of a single [`Feature`].
///
/// [`Feature`]: gherkin::Feature
type FeatureQueue<World> = Queue<RuleOrScenario, RuleOrScenarioQueue<World>>;

impl<World> FeatureQueue<World> {
    /// Inserts a new [`Rule`].
    ///
    /// [`Rule`]: gherkin::Rule
    fn new_rule(&mut self, rule: Event<Arc<gherkin::Rule>>) {
        let (rule, meta) = rule.split();
        drop(
            self.queue.insert(
                Either::Left(rule),
                Either::Left(RulesQueue::new(meta)),
            ),
        );
    }

    /// Marks a [`Rule`] as finished on [`event::Rule::Finished`].
    ///
    /// [`Rule`]: gherkin::Rule
    fn rule_finished(&mut self, rule: Event<Arc<gherkin::Rule>>) {
        let (rule, meta) = rule.split();
        match self.queue.get_mut(&Either::Left(rule)).unwrap() {
            Either::Left(ev) => {
                ev.finished(meta);
            }
            Either::Right(_) => unreachable!(),
        }
    }

    /// Inserts a new [`Scenario`] event.
    ///
    /// [`Scenario`]: gherkin::Scenario
    fn insert_scenario_event(
        &mut self,
        rule: Option<Arc<gherkin::Rule>>,
        scenario: Arc<gherkin::Scenario>,
        ev: Event<event::Scenario<World>>,
    ) {
        if let Some(rule) = rule {
            match self
                .queue
                .get_mut(&Either::Left(Arc::clone(&rule)))
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
}

#[async_trait(?Send)]
impl<'me, World> Emitter<World> for &'me mut FeatureQueue<World> {
    type Current = NextRuleOrScenario<'me, World>;
    type Emitted = RuleOrScenario;
    type EmittedPath = Arc<gherkin::Feature>;

    fn current_item(self) -> Option<Self::Current> {
        Some(match self.queue.iter_mut().next()? {
            (Either::Left(rule), Either::Left(events)) => {
                Either::Left((Arc::clone(rule), events))
            }
            (Either::Right(scenario), Either::Right(events)) => {
                Either::Right((Arc::clone(scenario), events))
            }
            _ => unreachable!(),
        })
    }

    async fn emit<W: Writer<World>>(
        self,
        feature: Self::EmittedPath,
        writer: &mut W,
        cli: &W::Cli,
    ) -> Option<Self::Emitted> {
        match self.current_item()? {
            Either::Left((rule, events)) => events
                .emit((feature, rule), writer, cli)
                .await
                .map(Either::Left),
            Either::Right((scenario, events)) => events
                .emit((feature, None, scenario), writer, cli)
                .await
                .map(Either::Right),
        }
    }
}

/// [`Queue`] of all events of a single [`Rule`].
///
/// [`Rule`]: gherkin::Rule
type RulesQueue<World> = Queue<Arc<gherkin::Scenario>, ScenariosQueue<World>>;

#[async_trait(?Send)]
impl<'me, World> Emitter<World> for &'me mut RulesQueue<World> {
    type Current = (Arc<gherkin::Scenario>, &'me mut ScenariosQueue<World>);
    type Emitted = Arc<gherkin::Rule>;
    type EmittedPath = (Arc<gherkin::Feature>, Arc<gherkin::Rule>);

    fn current_item(self) -> Option<Self::Current> {
        self.queue
            .iter_mut()
            .next()
            .map(|(sc, ev)| (Arc::clone(sc), ev))
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
                        Arc::clone(&feature),
                        Arc::clone(&rule),
                    ))),
                    cli,
                )
                .await;
        }

        while let Some((scenario, events)) = self.current_item() {
            if let Some(should_be_removed) = events
                .emit(
                    (Arc::clone(&feature), Some(Arc::clone(&rule)), scenario),
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
                        Arc::clone(&rule),
                    ))),
                    cli,
                )
                .await;
            return Some(rule);
        }

        None
    }
}

/// [`Queue`] of all events of a single [`Scenario`].
///
/// [`Scenario`]: gherkin::Scenario
#[derive(Debug)]
struct ScenariosQueue<World>(Vec<Event<event::Scenario<World>>>);

impl<World> ScenariosQueue<World> {
    /// Creates a new [`ScenariosQueue`].
    const fn new() -> Self {
        Self(Vec::new())
    }
}

#[async_trait(?Send)]
impl<World> Emitter<World> for &mut ScenariosQueue<World> {
    type Current = Event<event::Scenario<World>>;
    type Emitted = Arc<gherkin::Scenario>;
    type EmittedPath = (
        Arc<gherkin::Feature>,
        Option<Arc<gherkin::Rule>>,
        Arc<gherkin::Scenario>,
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
            let should_be_removed = matches!(ev, event::Scenario::Finished);

            let ev = meta.wrap(event::Cucumber::scenario(
                Arc::clone(&feature),
                rule.as_ref().map(Arc::clone),
                Arc::clone(&scenario),
                ev,
            ));
            writer.handle_event(Ok(ev), cli).await;

            if should_be_removed {
                return Some(Arc::clone(&scenario));
            }
        }
        None
    }
}
