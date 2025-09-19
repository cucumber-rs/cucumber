// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Normalized trait and AssertNormalized wrapper for declaring normalized writers.

use derive_more::with_trait::Deref;

use crate::{Event, World, Writer, event, parser, writer};

/// Marker indicating that a [`Writer`] can accept events in a [happened-before]
/// order.
///
/// This means one of two things:
///
/// 1. Either [`Writer`] doesn't depend on events ordering.
///    For example, [`Writer`] which prints only [`Failed`] [`Step`]s.
///
/// 2. Or [`Writer`] does depend on events ordering, but implements some logic
///    to rearrange them.
///    For example, a [`Normalize`] wrapper will rearrange events and pass them
///    to the underlying [`Writer`], like a [`Runner`] wasn't concurrent at all.
///
/// [`Step`]: gherkin::Step
/// [`Failed`]: event::Step::Failed
/// [`Runner`]: crate::Runner
/// [`Normalize`]: super::wrapper::Normalize
/// [happened-before]: https://en.wikipedia.org/wiki/Happened-before
pub trait Normalized {}

/// Wrapper for a [`Writer`] asserting it being [`Normalized`].
///
/// Technically is no-op, only forcing the [`Writer`] to become [`Normalized`]
/// despite it actually doesn't represent the one.
///
/// > ⚠️ __WARNING__: Should be used only in case you are absolutely sure, that
/// >                 incoming events will be emitted in a [`Normalized`] order.
/// >                 For example, in case [`max_concurrent_scenarios()`][1] is
/// >                 set to `1`.
///
/// [1]: crate::runner::Basic::max_concurrent_scenarios
#[derive(Clone, Copy, Debug, Deref)]
pub struct AssertNormalized<W: ?Sized>(W);

impl<Writer> AssertNormalized<Writer> {
    /// Creates a new no-op [`AssertNormalized`] wrapper forcing [`Normalized`]
    /// implementation.
    ///
    /// > ⚠️ __WARNING__: Should be used only in case you are absolutely sure,
    /// >                 that incoming events will be emitted in a
    /// >                 [`Normalized`] order.
    /// >                 For example, in case [`max_concurrent_scenarios()`][1]
    /// >                 is set to `1`.
    ///
    /// [1]: crate::runner::Basic::max_concurrent_scenarios
    #[must_use]
    pub const fn new(writer: Writer) -> Self {
        Self(writer)
    }
}

#[warn(clippy::missing_trait_methods)]
impl<W: World, Wr: Writer<W> + ?Sized> Writer<W> for AssertNormalized<Wr> {
    type Cli = Wr::Cli;

    async fn handle_event(
        &mut self,
        event: parser::Result<Event<event::Cucumber<W>>>,
        cli: &Self::Cli,
    ) {
        self.0.handle_event(event, cli).await;
    }
}

#[warn(clippy::missing_trait_methods)]
impl<W, Wr, Val> writer::Arbitrary<W, Val> for AssertNormalized<Wr>
where
    W: World,
    Wr: writer::Arbitrary<W, Val> + ?Sized,
{
    async fn write(&mut self, val: Val) {
        self.0.write(val).await;
    }
}

#[warn(clippy::missing_trait_methods)]
impl<W, Wr> writer::Stats<W> for AssertNormalized<Wr>
where
    Wr: writer::Stats<W>,
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
impl<Wr: writer::NonTransforming> writer::NonTransforming
    for AssertNormalized<Wr>
{
}

#[warn(clippy::missing_trait_methods)]
impl<Writer> Normalized for AssertNormalized<Writer> {}

// Implement Normalized for our Normalize wrapper
impl<World, Writer> Normalized for super::wrapper::Normalize<World, Writer> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Event, event::{Cucumber, Metadata}};
    use crate::test_utils::common::{EmptyCli, TestWorld};
    use crate::writer::Stats;

    // Mock writer for testing
    #[derive(Debug, Clone)]
    struct MockWriter {
        events_count: std::cell::RefCell<usize>,
    }

    impl MockWriter {
        fn new() -> Self {
            Self {
                events_count: std::cell::RefCell::new(0),
            }
        }

        fn get_events_count(&self) -> usize {
            *self.events_count.borrow()
        }
    }

    impl<W: World> Writer<W> for MockWriter {
        type Cli = EmptyCli;

        async fn handle_event(
            &mut self,
            _event: parser::Result<Event<event::Cucumber<W>>>,
            _cli: &Self::Cli,
        ) {
            *self.events_count.borrow_mut() += 1;
        }
    }

    impl<W: crate::World> writer::Stats<W> for MockWriter {
        fn passed_steps(&self) -> usize { 5 }
        fn skipped_steps(&self) -> usize { 2 }
        fn failed_steps(&self) -> usize { 1 }
        fn retried_steps(&self) -> usize { 0 }
        fn parsing_errors(&self) -> usize { 0 }
        fn hook_errors(&self) -> usize { 0 }
    }

    impl<W: crate::World, V> writer::Arbitrary<W, V> for MockWriter {
        async fn write(&mut self, _val: V) {
            // No-op for testing
        }
    }

    impl writer::NonTransforming for MockWriter {}

    // Using common TestWorld from test_utils

    #[test]
    fn test_assert_normalized_new() {
        let mock_writer = MockWriter::new();
        let assert_normalized = AssertNormalized::new(mock_writer);
        
        assert_eq!(assert_normalized.get_events_count(), 0);
    }

    #[test]
    fn test_assert_normalized_deref() {
        let mock_writer = MockWriter::new();
        let assert_normalized = AssertNormalized::new(mock_writer);
        
        // Should be able to access inner writer methods through Deref
        assert_eq!(assert_normalized.get_events_count(), 0);
    }

    #[tokio::test]
    async fn test_assert_normalized_handle_event() {
        let mock_writer = MockWriter::new();
        let mut assert_normalized = AssertNormalized::new(mock_writer);
        
        let event = Ok(Event::new(Cucumber::Started));
        
        assert_normalized.handle_event(event, &()).await;
        
        // Event should be passed through to inner writer
        assert_eq!(assert_normalized.get_events_count(), 1);
    }

    #[test]
    fn test_assert_normalized_stats() {
        let mock_writer = MockWriter::new();
        let assert_normalized = AssertNormalized::new(mock_writer);
        
        // Stats should be delegated to inner writer
        assert_eq!(assert_normalized.passed_steps(), 5);
        assert_eq!(assert_normalized.skipped_steps(), 2);
        assert_eq!(assert_normalized.failed_steps(), 1);
        assert_eq!(assert_normalized.retried_steps(), 0);
        assert_eq!(assert_normalized.parsing_errors(), 0);
        assert_eq!(assert_normalized.hook_errors(), 0);
        assert!(assert_normalized.execution_has_failed());
    }

    #[tokio::test]
    async fn test_assert_normalized_arbitrary() {
        let mock_writer = MockWriter::new();
        let mut assert_normalized = AssertNormalized::new(mock_writer);
        
        // Should delegate arbitrary writes to inner writer
        writer::Arbitrary::<TestWorld, String>::write(&mut assert_normalized, "test".to_string()).await;
        // This is a no-op in our mock, so we just verify it compiles and runs
    }

    #[test]
    fn test_normalized_trait_implementation() {
        let mock_writer = MockWriter::new();
        let assert_normalized = AssertNormalized::new(mock_writer);
        
        // Should implement Normalized trait
        fn requires_normalized<T: Normalized>(_: T) {}
        requires_normalized(assert_normalized);
    }

    #[test]
    fn test_non_transforming_trait() {
        let mock_writer = MockWriter::new();
        let assert_normalized = AssertNormalized::new(mock_writer);
        
        // Should implement NonTransforming trait
        fn requires_non_transforming<T: writer::NonTransforming>(_: T) {}
        requires_non_transforming(assert_normalized);
    }
}