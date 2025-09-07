//! Unit tests for writer module core functionality.

use cucumber::writer::*;
use cucumber::{cli, Event, Writer, event, parser};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
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
    stats: MockStats,
}

#[derive(Debug, Clone, Default)]
struct MockStats {
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

    fn with_stats(stats: MockStats) -> Self {
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
struct MockCli;

impl cli::Colored for MockCli {}

impl Writer<TestWorld> for MockWriter {
    type Cli = MockCli;

    async fn handle_event(
        &mut self,
        event: parser::Result<Event<event::Cucumber<TestWorld>>>,
        _cli: &Self::Cli,
    ) {
        match &event {
            Ok(_) => self.push_event("event"),
            Err(_) => self.push_event("error"),
        }
    }
}

impl Arbitrary<TestWorld, &str> for MockWriter {
    async fn write(&mut self, val: &str) {
        self.push_event(&format!("write: {val}"));
    }
}

impl Stats<TestWorld> for MockWriter {
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

impl Normalized for MockWriter {}
impl NonTransforming for MockWriter {}

// Test Verbosity enum
#[test]
fn test_verbosity_default() {
    let default_verbosity = Verbosity::default();
    assert_eq!(default_verbosity as u8, 0);
    assert!(matches!(default_verbosity, Verbosity::Default));
    assert!(!default_verbosity.shows_world());
    assert!(!default_verbosity.shows_docstring());
}

#[test]
fn test_verbosity_show_world() {
    let verbosity = Verbosity::ShowWorld;
    assert_eq!(verbosity as u8, 1);
    assert!(verbosity.shows_world());
    assert!(!verbosity.shows_docstring());
}

#[test]
fn test_verbosity_show_world_and_docstring() {
    let verbosity = Verbosity::ShowWorldAndDocString;
    assert_eq!(verbosity as u8, 2);
    assert!(verbosity.shows_world());
    assert!(verbosity.shows_docstring());
}

#[test]
fn test_verbosity_from_u8() {
    assert!(matches!(Verbosity::from(0), Verbosity::Default));
    assert!(matches!(Verbosity::from(1), Verbosity::ShowWorld));
    assert!(matches!(Verbosity::from(2), Verbosity::ShowWorldAndDocString));
    assert!(matches!(Verbosity::from(255), Verbosity::ShowWorldAndDocString)); // Any value >= 2
}

#[test]
fn test_verbosity_into_u8() {
    assert_eq!(u8::from(Verbosity::Default), 0);
    assert_eq!(u8::from(Verbosity::ShowWorld), 1);
    assert_eq!(u8::from(Verbosity::ShowWorldAndDocString), 2);
}

#[test]
fn test_verbosity_clone_debug() {
    let verbosity = Verbosity::ShowWorld;
    let cloned = verbosity.clone();
    assert_eq!(verbosity as u8, cloned as u8);
    
    let debug_str = format!("{:?}", verbosity);
    assert!(debug_str.contains("ShowWorld"));
}

// Test Writer trait implementation
#[tokio::test]
async fn test_writer_trait() {
    let mut writer = MockWriter::new();
    let cli = MockCli::default();
    
    let event = Event::new(event::Cucumber::<TestWorld>::Started);
    writer.handle_event(Ok(event), &cli).await;
    
    assert_eq!(writer.events(), vec!["event"]);
}

#[tokio::test]
async fn test_arbitrary_trait() {
    let mut writer = MockWriter::new();
    
    writer.write("test message").await;
    
    assert_eq!(writer.events(), vec!["write: test message"]);
}

// Test Stats trait implementation
#[test]
fn test_stats_trait() {
    let stats = MockStats {
        passed: 5,
        skipped: 2,
        failed: 1,
        retried: 3,
        parsing_errors: 0,
        hook_errors: 1,
    };
    
    let writer = MockWriter::with_stats(stats);
    
    assert_eq!(writer.passed_steps(), 5);
    assert_eq!(writer.skipped_steps(), 2);
    assert_eq!(writer.failed_steps(), 1);
    assert_eq!(writer.retried_steps(), 3);
    assert_eq!(writer.parsing_errors(), 0);
    assert_eq!(writer.hook_errors(), 1);
}

#[test]
fn test_stats_execution_has_failed() {
    // Test no failures
    let writer1 = MockWriter::with_stats(MockStats {
        passed: 5,
        skipped: 2,
        failed: 0,
        retried: 0,
        parsing_errors: 0,
        hook_errors: 0,
    });
    assert!(!writer1.execution_has_failed());
    
    // Test with failed steps
    let writer2 = MockWriter::with_stats(MockStats {
        failed: 1,
        ..Default::default()
    });
    assert!(writer2.execution_has_failed());
    
    // Test with parsing errors
    let writer3 = MockWriter::with_stats(MockStats {
        parsing_errors: 1,
        ..Default::default()
    });
    assert!(writer3.execution_has_failed());
    
    // Test with hook errors
    let writer4 = MockWriter::with_stats(MockStats {
        hook_errors: 1,
        ..Default::default()
    });
    assert!(writer4.execution_has_failed());
}

// Test Ext trait methods
#[test]
fn test_ext_assert_normalized() {
    let writer = MockWriter::new();
    let normalized = writer.assert_normalized();
    
    // Should wrap in AssertNormalized
    let _: AssertNormalized<MockWriter> = normalized;
}

#[test]
fn test_ext_normalized() {
    let writer = MockWriter::new();
    let normalized = writer.normalized::<TestWorld>();
    
    // Should wrap in Normalize
    let _: Normalize<TestWorld, MockWriter> = normalized;
}

#[test]
fn test_ext_summarized() {
    let writer = MockWriter::new();
    let summarized = writer.summarized();
    
    // Should wrap in Summarize
    let _: Summarize<MockWriter> = summarized;
}

#[test]
fn test_ext_fail_on_skipped() {
    let writer = MockWriter::new();
    let fail_on_skipped = writer.fail_on_skipped();
    
    // Should wrap in FailOnSkipped
    let _: FailOnSkipped<MockWriter> = fail_on_skipped;
}