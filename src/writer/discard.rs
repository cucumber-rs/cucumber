// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Wrappers providing no-op implementations.

use derive_more::with_trait::{Deref, DerefMut};

use crate::{Event, World, Writer, event::Cucumber, parser, writer};

/// Wrapper providing a no-op [`ArbitraryWriter`] implementation.
///
/// Intended to be used for feeding a non-[`ArbitraryWriter`] [`Writer`] into a
/// [`writer::Tee`], as the latter accepts only [`ArbitraryWriter`]s.
///
/// [`ArbitraryWriter`]: writer::Arbitrary
#[derive(Clone, Copy, Debug, Deref, DerefMut)]
pub struct Arbitrary<Wr: ?Sized>(Wr);

#[warn(clippy::missing_trait_methods)]
impl<W: World, Wr: Writer<W> + ?Sized> Writer<W> for Arbitrary<Wr> {
    type Cli = Wr::Cli;

    async fn handle_event(
        &mut self,
        event: parser::Result<Event<Cucumber<W>>>,
        cli: &Self::Cli,
    ) {
        self.0.handle_event(event, cli).await;
    }
}

#[warn(clippy::missing_trait_methods)]
impl<W, Val, Wr> writer::Arbitrary<W, Val> for Arbitrary<Wr>
where
    Wr: ?Sized,
    Self: Writer<W>,
{
    /// Does nothing.
    async fn write(&mut self, _: Val) {
        // Intentionally no-op.
    }
}

#[warn(clippy::missing_trait_methods)]
impl<W, Wr> writer::Stats<W> for Arbitrary<Wr>
where
    Wr: writer::Stats<W> + ?Sized,
    Self: Writer<W>,
{
    fn passed_steps(&self) -> usize {
        self.0.passed_steps()
    }

    fn skipped_steps(&self) -> usize {
        self.0.skipped_steps()
    }

    fn failed_steps(&self) -> usize {
        self.0.failed_steps()
    }

    fn retried_steps(&self) -> usize {
        self.0.retried_steps()
    }

    fn parsing_errors(&self) -> usize {
        self.0.parsing_errors()
    }

    fn hook_errors(&self) -> usize {
        self.0.hook_errors()
    }

    fn execution_has_failed(&self) -> bool {
        self.0.execution_has_failed()
    }
}

#[warn(clippy::missing_trait_methods)]
impl<Wr: writer::Normalized> writer::Normalized for Arbitrary<Wr> {}

#[warn(clippy::missing_trait_methods)]
impl<Wr: writer::NonTransforming> writer::NonTransforming for Arbitrary<Wr> {}

impl<Wr> Arbitrary<Wr> {
    /// Wraps the given [`Writer`] into a [`discard::Arbitrary`] one.
    ///
    /// [`discard::Arbitrary`]: Arbitrary
    #[must_use]
    pub const fn wrap(writer: Wr) -> Self {
        Self(writer)
    }
}

/// Wrapper providing a no-op [`StatsWriter`] implementation returning only `0`.
///
/// Intended to be used for feeding a non-[`StatsWriter`] [`Writer`] into a
/// [`writer::Tee`], as the later accepts only [`StatsWriter`]s.
///
/// [`StatsWriter`]: writer::Stats
#[derive(Clone, Copy, Debug, Deref, DerefMut)]
pub struct Stats<Wr: ?Sized>(Wr);

#[warn(clippy::missing_trait_methods)]
impl<W: World, Wr: Writer<W> + ?Sized> Writer<W> for Stats<Wr> {
    type Cli = Wr::Cli;

    async fn handle_event(
        &mut self,
        event: parser::Result<Event<Cucumber<W>>>,
        cli: &Self::Cli,
    ) {
        self.0.handle_event(event, cli).await;
    }
}

#[warn(clippy::missing_trait_methods)]
impl<W, Val, Wr> writer::Arbitrary<W, Val> for Stats<Wr>
where
    Wr: writer::Arbitrary<W, Val> + ?Sized,
    Self: Writer<W>,
{
    async fn write(&mut self, val: Val) {
        self.0.write(val).await;
    }
}

impl<W, Wr> writer::Stats<W> for Stats<Wr>
where
    Wr: Writer<W> + ?Sized,
    Self: Writer<W>,
{
    /// Always returns `0`.
    fn passed_steps(&self) -> usize {
        0
    }

    /// Always returns `0`.
    fn skipped_steps(&self) -> usize {
        0
    }

    /// Always returns `0`.
    fn failed_steps(&self) -> usize {
        0
    }

    /// Always returns `0`.
    fn retried_steps(&self) -> usize {
        0
    }

    /// Always returns `0`.
    fn parsing_errors(&self) -> usize {
        0
    }

    /// Always returns `0`.
    fn hook_errors(&self) -> usize {
        0
    }
}

#[warn(clippy::missing_trait_methods)]
impl<Wr: writer::Normalized> writer::Normalized for Stats<Wr> {}

#[warn(clippy::missing_trait_methods)]
impl<Wr: writer::NonTransforming> writer::NonTransforming for Stats<Wr> {}

impl<Wr> Stats<Wr> {
    /// Wraps the given [`Writer`] into a [`discard::Stats`] one.
    ///
    /// [`discard::Stats`]: Stats
    #[must_use]
    pub const fn wrap(writer: Wr) -> Self {
        Self(writer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cli, Event, Writer, writer::{Arbitrary as ArbitraryTrait, Stats as StatsTrait}};
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Default)]
    struct TestWorld;
    
    impl crate::World for TestWorld {
        type Error = std::convert::Infallible;
        
        async fn new() -> Result<Self, Self::Error> {
            Ok(Self::default())
        }
    }

    #[derive(Debug, Clone, Default)]
    struct MockWriter {
        events: Arc<Mutex<Vec<String>>>,
        stats: MockWriterStats,
    }

    #[derive(Debug, Clone, Default)]
    struct MockWriterStats {
        passed: usize,
        skipped: usize,
        failed: usize,
        retried: usize,
        parsing_errors: usize,
        hook_errors: usize,
    }

    impl MockWriter {
        fn new() -> Self {
            Self::default()
        }

        fn with_stats(stats: MockWriterStats) -> Self {
            Self {
                stats,
                ..Default::default()
            }
        }

        fn events(&self) -> Vec<String> {
            self.events.lock().unwrap().clone()
        }

        fn push_event(&self, event: &str) {
            self.events.lock().unwrap().push(event.to_string());
        }
    }

    #[derive(Debug, Clone, Default, clap::Args)]
    struct MockCli {
        #[arg(long)]
        flag: bool,
    }

    impl cli::Colored for MockCli {}

    impl Writer<TestWorld> for MockWriter {
        type Cli = MockCli;

        async fn handle_event(
            &mut self,
            event: parser::Result<Event<crate::event::Cucumber<TestWorld>>>,
            _cli: &Self::Cli,
        ) {
            match &event {
                Ok(event) if matches!(**event, crate::event::Cucumber::Started) => self.push_event("Started"),
                Ok(event) if matches!(**event, crate::event::Cucumber::Finished) => self.push_event("Finished"),
                _ => self.push_event("Other"),
            }
        }
    }

    impl writer::Stats<TestWorld> for MockWriter {
        fn passed_steps(&self) -> usize {
            self.stats.passed
        }

        fn skipped_steps(&self) -> usize {
            self.stats.skipped
        }

        fn failed_steps(&self) -> usize {
            self.stats.failed
        }

        fn retried_steps(&self) -> usize {
            self.stats.retried
        }

        fn parsing_errors(&self) -> usize {
            self.stats.parsing_errors
        }

        fn hook_errors(&self) -> usize {
            self.stats.hook_errors
        }
    }

    impl writer::Arbitrary<TestWorld, &str> for MockWriter {
        async fn write(&mut self, val: &str) {
            self.push_event(&format!("Arbitrary: {val}"));
        }
    }

    impl writer::Normalized for MockWriter {}
    impl writer::NonTransforming for MockWriter {}

    // Test Arbitrary wrapper
    #[tokio::test]
    async fn test_arbitrary_discard_writer() {
        let inner = MockWriter::new();
        let events = inner.events.clone();
        let mut arbitrary_writer = Arbitrary::wrap(inner);

        #[cfg(feature = "timestamps")]
        let started_event = Event { value: crate::event::Cucumber::Started, at: std::time::SystemTime::now() };
        #[cfg(feature = "timestamps")]
        let finished_event = Event { value: crate::event::Cucumber::Finished, at: std::time::SystemTime::now() };
        
        #[cfg(not(feature = "timestamps"))]
        let started_event = Event { value: crate::event::Cucumber::Started };
        #[cfg(not(feature = "timestamps"))]
        let finished_event = Event { value: crate::event::Cucumber::Finished };
        arbitrary_writer.handle_event(Ok(started_event), &MockCli::default()).await;
        arbitrary_writer.handle_event(Ok(finished_event), &MockCli::default()).await;

        // Events should be passed through to inner writer
        assert_eq!(events.lock().unwrap().as_slice(), &["Started", "Finished"]);
    }

    #[tokio::test]
    async fn test_arbitrary_discard_write() {
        let inner = MockWriter::new();
        let events = inner.events.clone();
        let mut arbitrary_writer = Arbitrary::wrap(inner);

        // The arbitrary write should be discarded (no-op)
        ArbitraryTrait::write(&mut arbitrary_writer, "test message").await;

        // Should be empty because write is discarded
        assert!(events.lock().unwrap().is_empty());
    }

    #[test]
    fn test_arbitrary_discard_stats_passthrough() {
        let mock_stats = MockWriterStats {
            passed: 5,
            skipped: 2,
            failed: 1,
            retried: 3,
            parsing_errors: 1,
            hook_errors: 0,
        };
        
        let inner = MockWriter::with_stats(mock_stats);
        let arbitrary_writer = Arbitrary::wrap(inner);

        // Stats should be passed through to inner writer
        assert_eq!(StatsTrait::passed_steps(&arbitrary_writer), 5);
        assert_eq!(StatsTrait::skipped_steps(&arbitrary_writer), 2);
        assert_eq!(StatsTrait::failed_steps(&arbitrary_writer), 1);
        assert_eq!(StatsTrait::retried_steps(&arbitrary_writer), 3);
        assert_eq!(StatsTrait::parsing_errors(&arbitrary_writer), 1);
        assert_eq!(StatsTrait::hook_errors(&arbitrary_writer), 0);
    }

    #[test]
    fn test_arbitrary_deref() {
        let inner = MockWriter::new();
        let arbitrary_writer = Arbitrary::wrap(inner);

        // Test that we can access the inner writer through deref
        let _inner_ref: &MockWriter = &*arbitrary_writer;
    }

    #[test]
    fn test_arbitrary_deref_mut() {
        let inner = MockWriter::new();
        let mut arbitrary_writer = Arbitrary::wrap(inner);

        // Test that we can access the inner writer mutably through deref
        let _inner_ref: &mut MockWriter = &mut *arbitrary_writer;
    }

    // Test Stats wrapper
    #[tokio::test]
    async fn test_stats_discard_writer() {
        let inner = MockWriter::new();
        let events = inner.events.clone();
        let mut stats_writer = Stats::wrap(inner);

        #[cfg(feature = "timestamps")]
        let started_event = Event { value: crate::event::Cucumber::Started, at: std::time::SystemTime::now() };
        #[cfg(feature = "timestamps")]
        let finished_event = Event { value: crate::event::Cucumber::Finished, at: std::time::SystemTime::now() };
        
        #[cfg(not(feature = "timestamps"))]
        let started_event = Event { value: crate::event::Cucumber::Started };
        #[cfg(not(feature = "timestamps"))]
        let finished_event = Event { value: crate::event::Cucumber::Finished };
        stats_writer.handle_event(Ok(started_event), &MockCli::default()).await;
        stats_writer.handle_event(Ok(finished_event), &MockCli::default()).await;

        // Events should be passed through to inner writer
        assert_eq!(events.lock().unwrap().as_slice(), &["Started", "Finished"]);
    }

    #[tokio::test]
    async fn test_stats_discard_write_passthrough() {
        let inner = MockWriter::new();
        let events = inner.events.clone();
        let mut stats_writer = Stats::wrap(inner);

        // The arbitrary write should pass through to inner writer
        ArbitraryTrait::write(&mut stats_writer, "test message").await;

        // Should contain the arbitrary write from inner writer
        assert_eq!(events.lock().unwrap().as_slice(), &["Arbitrary: test message"]);
    }

    #[test]
    fn test_stats_discard_all_zero() {
        let mock_stats = MockWriterStats {
            passed: 5,
            skipped: 2,
            failed: 1,
            retried: 3,
            parsing_errors: 1,
            hook_errors: 2,
        };
        
        let inner = MockWriter::with_stats(mock_stats);
        let stats_writer = Stats::wrap(inner);

        // All stats should return 0 (discarded)
        assert_eq!(StatsTrait::passed_steps(&stats_writer), 0);
        assert_eq!(StatsTrait::skipped_steps(&stats_writer), 0);
        assert_eq!(StatsTrait::failed_steps(&stats_writer), 0);
        assert_eq!(StatsTrait::retried_steps(&stats_writer), 0);
        assert_eq!(StatsTrait::parsing_errors(&stats_writer), 0);
        assert_eq!(StatsTrait::hook_errors(&stats_writer), 0);
    }

    #[test]
    fn test_stats_deref() {
        let inner = MockWriter::new();
        let stats_writer = Stats::wrap(inner);

        // Test that we can access the inner writer through deref
        let _inner_ref: &MockWriter = &*stats_writer;
    }

    #[test]
    fn test_stats_deref_mut() {
        let inner = MockWriter::new();
        let mut stats_writer = Stats::wrap(inner);

        // Test that we can access the inner writer mutably through deref
        let _inner_ref: &mut MockWriter = &mut *stats_writer;
    }

    #[test]
    fn test_wrapper_construction() {
        let inner = MockWriter::new();

        // Test that both wrappers can be constructed  
        let arbitrary_wrapper = Arbitrary::wrap(inner.clone());
        let stats_wrapper = Stats::wrap(inner.clone());

        // Verify they wrap the inner writer by checking they deref to the same type
        let _: &MockWriter = &*arbitrary_wrapper;
        let _: &MockWriter = &*stats_wrapper;
    }

    #[test]
    fn test_normalized_and_non_transforming_traits() {
        let inner = MockWriter::new();
        
        let arbitrary_writer = Arbitrary::wrap(inner.clone());
        let stats_writer = Stats::wrap(inner);

        // Test that trait implementations are available (compile test)
        fn assert_normalized<W: writer::Normalized>(_w: &W) {}
        fn assert_non_transforming<W: writer::NonTransforming>(_w: &W) {}

        assert_normalized(&arbitrary_writer);
        assert_non_transforming(&arbitrary_writer);
        assert_normalized(&stats_writer);  
        assert_non_transforming(&stats_writer);
    }
}
