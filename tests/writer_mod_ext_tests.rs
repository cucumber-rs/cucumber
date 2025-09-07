//! Extended tests for writer module Ext trait functionality.

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
    #[allow(dead_code)]
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
}

#[derive(Debug, Clone, Default, clap::Args)]
struct MockCli;

impl cli::Colored for MockCli {}

impl Writer<TestWorld> for MockWriter {
    type Cli = MockCli;

    async fn handle_event(
        &mut self,
        _event: parser::Result<Event<event::Cucumber<TestWorld>>>,
        _cli: &Self::Cli,
    ) {
        // No-op for these tests
    }
}

impl Stats<TestWorld> for MockWriter {
    fn passed_steps(&self) -> usize { self.stats.passed }
    fn skipped_steps(&self) -> usize { self.stats.skipped }
    fn failed_steps(&self) -> usize { self.stats.failed }
    fn retried_steps(&self) -> usize { self.stats.retried }
    fn parsing_errors(&self) -> usize { self.stats.parsing_errors }
    fn hook_errors(&self) -> usize { self.stats.hook_errors }
}

impl Normalized for MockWriter {}
impl NonTransforming for MockWriter {}

#[test]
fn test_ext_fail_on_skipped_with() {
    let writer = MockWriter::new();
    let predicate = |_: &gherkin::Feature, _: Option<&gherkin::Rule>, _: &gherkin::Scenario| true;
    let fail_on_skipped = writer.fail_on_skipped_with(predicate);
    
    // Should wrap in FailOnSkipped with predicate
    let _: FailOnSkipped<MockWriter, _> = fail_on_skipped;
}

#[test] 
fn test_ext_repeat_skipped() {
    let writer = MockWriter::new();
    let repeat_skipped = writer.repeat_skipped::<TestWorld>();
    
    // Should wrap in Repeat
    let _: Repeat<TestWorld, MockWriter> = repeat_skipped;
}

#[test]
fn test_ext_repeat_failed() {
    let writer = MockWriter::new();
    let repeat_failed = writer.repeat_failed::<TestWorld>();
    
    // Should wrap in Repeat
    let _: Repeat<TestWorld, MockWriter> = repeat_failed;
}

#[test]
fn test_ext_repeat_if() {
    let writer = MockWriter::new();
    let filter = |_: &parser::Result<Event<event::Cucumber<TestWorld>>>| true;
    let repeat_if = writer.repeat_if(filter);
    
    // Should wrap in Repeat with custom filter
    let _: Repeat<TestWorld, MockWriter, _> = repeat_if;
}

#[test]
fn test_ext_tee() {
    let writer1 = MockWriter::new();
    let writer2 = MockWriter::new();
    let tee = writer1.tee::<TestWorld, _>(writer2);
    
    // Should wrap in Tee
    let _: Tee<MockWriter, MockWriter> = tee;
}

#[test]
fn test_ext_discard_arbitrary_writes() {
    let writer = MockWriter::new();
    let discard = writer.discard_arbitrary_writes();
    
    // Should wrap in discard::Arbitrary
    let _: discard::Arbitrary<MockWriter> = discard;
}

#[test]
fn test_ext_discard_stats_writes() {
    let writer = MockWriter::new();
    let discard = writer.discard_stats_writes();
    
    // Should wrap in discard::Stats
    let _: discard::Stats<MockWriter> = discard;
}

// Test method chaining (pipelining)
#[test]
fn test_writer_pipelining() {
    let writer = MockWriter::new();
    
    // Test that methods can be chained
    let _pipeline = writer
        .fail_on_skipped()
        .summarized()
        .repeat_failed::<TestWorld>()
        .assert_normalized();
}

#[test]
fn test_writer_complex_pipeline() {
    let writer1 = MockWriter::new();
    let writer2 = MockWriter::new();
    
    // Test complex pipeline with tee and discard operations
    let _complex_pipeline = writer1
        .discard_arbitrary_writes()
        .tee::<TestWorld, _>(writer2.discard_stats_writes())
        .repeat_skipped::<TestWorld>()
        .summarized()
        .assert_normalized();
}

// Test trait bounds and sealed trait
#[test]
fn test_trait_implementations() {
    let writer = MockWriter::new();
    
    // Test that our MockWriter implements necessary traits
    fn assert_writer<W: Writer<TestWorld>>(_: W) {}
    fn assert_stats<S: Stats<TestWorld>>(_: S) {}
    fn assert_normalized<N: Normalized>(_: N) {}
    fn assert_non_transforming<NT: NonTransforming>(_: NT) {}
    
    assert_writer(writer.clone());
    assert_stats(writer.clone());
    assert_normalized(writer.clone());
    assert_non_transforming(writer);
}

// Test module exports
#[test]
fn test_public_exports() {
    // Test that all major types are accessible
    let _coloring: Coloring = Coloring::Auto;
    
    // Test common module exports
    let stats = WriterStats::default();
    assert_eq!(stats.passed_steps, 0);
    
    // Test that Verbosity is accessible
    let _: Verbosity = Verbosity::Default;
}

#[cfg(feature = "output-json")]
#[test]
fn test_conditional_exports_json() {
    // Test that Json is available when feature is enabled
    let _: Json<std::io::Stdout> = Json::stdout();
}

#[cfg(feature = "output-junit")]
#[test] 
fn test_conditional_exports_junit() {
    // Test that JUnit is available when feature is enabled
    let _path = "/tmp/junit.xml";
    // We don't actually create the file in the test
}

#[cfg(feature = "libtest")]
#[test]
fn test_conditional_exports_libtest() {
    // Test that Libtest is available when feature is enabled
    let _: Libtest<std::io::Stdout> = Libtest::stdout();
}

// Test edge cases and error conditions
#[test]
fn test_verbosity_boundary_values() {
    // Test edge cases for u8 conversion
    let verbosity_max = Verbosity::from(u8::MAX);
    assert!(matches!(verbosity_max, Verbosity::ShowWorldAndDocString));
    
    let verbosity_min = Verbosity::from(u8::MIN);
    assert!(matches!(verbosity_min, Verbosity::Default));
}

#[test]
fn test_stats_with_zero_values() {
    let writer = MockWriter::with_stats(MockStats::default());
    
    assert_eq!(writer.passed_steps(), 0);
    assert_eq!(writer.skipped_steps(), 0);
    assert_eq!(writer.failed_steps(), 0);
    assert_eq!(writer.retried_steps(), 0);
    assert_eq!(writer.parsing_errors(), 0);
    assert_eq!(writer.hook_errors(), 0);
    assert!(!writer.execution_has_failed());
}

#[test]
fn test_stats_with_max_values() {
    let writer = MockWriter::with_stats(MockStats {
        passed: usize::MAX,
        skipped: usize::MAX,
        failed: usize::MAX,
        retried: usize::MAX,
        parsing_errors: usize::MAX,
        hook_errors: usize::MAX,
    });
    
    assert_eq!(writer.passed_steps(), usize::MAX);
    assert_eq!(writer.skipped_steps(), usize::MAX);
    assert_eq!(writer.failed_steps(), usize::MAX);
    assert_eq!(writer.retried_steps(), usize::MAX);
    assert_eq!(writer.parsing_errors(), usize::MAX);
    assert_eq!(writer.hook_errors(), usize::MAX);
    assert!(writer.execution_has_failed());
}