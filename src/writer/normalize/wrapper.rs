// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Normalize wrapper for outputting events in normalized readable order.

use derive_more::with_trait::Deref;

use crate::{
    Event, Writer,
    event::{self, Metadata},
    parser, writer,
};

use super::{cucumber::CucumberQueue, emitter::Emitter};

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
    writer: Writer,

    /// Normalization queue of happened events.
    queue: CucumberQueue<World>,
}

// Implemented manually to omit redundant `World: Clone` trait bound, imposed by
// `#[derive(Clone)]`.
impl<World, Writer: Clone> Clone for Normalize<World, Writer> {
    fn clone(&self) -> Self {
        Self { writer: self.writer.clone(), queue: self.queue.clone() }
    }
}

impl<W, Writer> Normalize<W, Writer> {
    /// Creates a new [`Normalize`] wrapper, which will rearrange [`event`]s
    /// and feed them to the given [`Writer`].
    #[must_use]
    pub fn new(writer: Writer) -> Self {
        Self { writer, queue: CucumberQueue::new(Metadata::new(())) }
    }

    /// Returns the original [`Writer`], wrapped by this [`Normalize`] one.
    #[must_use]
    pub const fn inner_writer(&self) -> &Writer {
        &self.writer
    }
}

impl<World, Wr: Writer<World>> Writer<World> for Normalize<World, Wr> {
    type Cli = Wr::Cli;

    async fn handle_event(
        &mut self,
        event: parser::Result<Event<event::Cucumber<World>>>,
        cli: &Self::Cli,
    ) {
        use event::{Cucumber, Feature, Rule};

        // Once `Cucumber::Finished` is emitted, we just pass events through,
        // without any normalization.
        // This is done to avoid panic if this `Writer` happens to be wrapped
        // inside `writer::Repeat` or similar.
        if self.queue.is_finished_and_emitted() {
            self.writer.handle_event(event, cli).await;
            return;
        }

        match event.map(Event::split) {
            res @ (Err(_)
            | Ok((
                Cucumber::Started | Cucumber::ParsingFinished { .. },
                _,
            ))) => {
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
            Emitter::emit(&mut self.queue, (), &mut self.writer, cli).await
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

#[warn(clippy::missing_trait_methods)]
impl<W, Wr, Val> writer::Arbitrary<W, Val> for Normalize<W, Wr>
where
    Wr: writer::Arbitrary<W, Val>,
{
    async fn write(&mut self, val: Val) {
        self.writer.write(val).await;
    }
}

#[warn(clippy::missing_trait_methods)]
impl<W, Wr> writer::Stats<W> for Normalize<W, Wr>
where
    Wr: writer::Stats<W>,
    Self: Writer<W>,
{
    fn passed_steps(&self) -> usize {
        self.writer.passed_steps()
    }

    fn skipped_steps(&self) -> usize {
        self.writer.skipped_steps()
    }

    fn failed_steps(&self) -> usize {
        self.writer.failed_steps()
    }

    fn retried_steps(&self) -> usize {
        self.writer.retried_steps()
    }

    fn parsing_errors(&self) -> usize {
        self.writer.parsing_errors()
    }

    fn hook_errors(&self) -> usize {
        self.writer.hook_errors()
    }

    fn execution_has_failed(&self) -> bool {
        self.writer.execution_has_failed()
    }
}

#[warn(clippy::missing_trait_methods)]
impl<W, Wr: writer::NonTransforming> writer::NonTransforming
    for Normalize<W, Wr>
{
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Event, event::{Cucumber, Metadata}, writer};

    // Mock writer for testing
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

    impl<W> Writer<W> for MockWriter {
        type Cli = crate::test_utils::common::EmptyCli;

        async fn handle_event(
            &mut self,
            event: parser::Result<Event<event::Cucumber<W>>>,
            _cli: &Self::Cli,
        ) {
            if let Ok(ev) = event {
                let event_name = match ev.value {
                    Cucumber::Started => "Started".to_string(),
                    Cucumber::ParsingFinished { .. } => "ParsingFinished".to_string(),
                    Cucumber::Finished => "Finished".to_string(),
                    Cucumber::Feature(_, _) => "Feature".to_string(),
                };
                self.events.lock().unwrap().push(event_name);
            }
        }
    }

    impl<W> writer::Stats<W> for MockWriter {
        fn passed_steps(&self) -> usize { 0 }
        fn skipped_steps(&self) -> usize { 0 }
        fn failed_steps(&self) -> usize { 0 }
        fn retried_steps(&self) -> usize { 0 }
        fn parsing_errors(&self) -> usize { 0 }
        fn hook_errors(&self) -> usize { 0 }
    }

    impl writer::NonTransforming for MockWriter {}

    #[test]
    fn test_normalize_new() {
        let mock_writer = MockWriter::new();
        let normalize = Normalize::new(mock_writer.clone());
        
        assert_eq!(normalize.inner_writer().get_events().len(), 0);
    }

    #[test]
    fn test_normalize_clone() {
        let mock_writer = MockWriter::new();
        let normalize = Normalize::new(mock_writer);
        let cloned = normalize.clone();
        
        // Both should have separate but equivalent states
        assert_eq!(cloned.inner_writer().get_events().len(), 0);
    }

    #[tokio::test]
    async fn test_normalize_cucumber_started_event() {
        let mock_writer = MockWriter::new();
        let mut normalize = Normalize::new(mock_writer.clone());
        
        let event = Ok(Event::new(Cucumber::Started));
        
        normalize.handle_event(event, &()).await;
        
        // Started events should pass through immediately
        assert_eq!(normalize.inner_writer().get_events(), vec!["Started"]);
    }

    #[tokio::test]
    async fn test_normalize_finished_state() {
        let mock_writer = MockWriter::new();
        let mut normalize = Normalize::new(mock_writer.clone());
        
        // First, finish the queue
        let finish_event = Ok(Event::new(Cucumber::Finished));
        normalize.handle_event(finish_event, &()).await;
        
        // Now any event should pass through without normalization
        let event = Ok(Event::new(Cucumber::Started));
        normalize.handle_event(event, &()).await;
        
        assert!(normalize.queue.is_finished_and_emitted());
    }
}