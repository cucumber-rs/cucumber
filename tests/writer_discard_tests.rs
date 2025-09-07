//! Unit tests for writer discard wrappers.

use cucumber::writer::discard::*;
use cucumber::{cli, Event, Writer, writer::{Arbitrary as ArbitraryTrait, Stats as StatsTrait}};
use std::sync::{Arc, Mutex};

#[derive(Debug, Default)]
struct TestWorld;

impl cucumber::World for TestWorld {
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

    #[allow(dead_code)]
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
        event: cucumber::parser::Result<Event<cucumber::event::Cucumber<TestWorld>>>,
        _cli: &Self::Cli,
    ) {
        match &event {
            Ok(event) if matches!(**event, cucumber::event::Cucumber::Started) => self.push_event("Started"),
            Ok(event) if matches!(**event, cucumber::event::Cucumber::Finished) => self.push_event("Finished"),
            _ => self.push_event("Other"),
        }
    }
}

impl cucumber::writer::Stats<TestWorld> for MockWriter {
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

impl cucumber::writer::Arbitrary<TestWorld, &str> for MockWriter {
    async fn write(&mut self, val: &str) {
        self.push_event(&format!("Arbitrary: {val}"));
    }
}

impl cucumber::writer::Normalized for MockWriter {}
impl cucumber::writer::NonTransforming for MockWriter {}

// Test Arbitrary wrapper
#[tokio::test]
async fn test_arbitrary_discard_writer() {
    let inner = MockWriter::new();
    let events = inner.events.clone();
    let mut arbitrary_writer = Arbitrary::wrap(inner);

    let started_event = Event::new(cucumber::event::Cucumber::Started);
    let finished_event = Event::new(cucumber::event::Cucumber::Finished);
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

    let started_event = Event::new(cucumber::event::Cucumber::Started);
    let finished_event = Event::new(cucumber::event::Cucumber::Finished);
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
fn test_wrapper_construction() {
    let inner = MockWriter::new();

    // Test that both wrappers can be constructed  
    let arbitrary_wrapper = Arbitrary::wrap(inner.clone());
    let stats_wrapper = Stats::wrap(inner.clone());

    // Verify they wrap the inner writer by checking they deref to the same type
    let _: &MockWriter = &*arbitrary_wrapper;
    let _: &MockWriter = &*stats_wrapper;
}