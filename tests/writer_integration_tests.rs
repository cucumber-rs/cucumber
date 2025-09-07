use std::sync::{Arc, Mutex};
use cucumber::{cli, Writer, writer};

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
    id: String,
}

impl MockWriter {
    fn new(id: &str) -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            id: id.to_string(),
        }
    }

    #[allow(dead_code)]
    fn events(&self) -> Vec<String> {
        self.events.lock().unwrap().clone()
    }

    fn push_event(&self, event: &str) {
        self.events.lock().unwrap().push(format!("{}: {}", self.id, event));
    }
}

#[derive(Debug, Clone, Default, clap::Args)]
struct MockCli;

impl cli::Colored for MockCli {}

impl Writer<TestWorld> for MockWriter {
    type Cli = MockCli;

    async fn handle_event(
        &mut self,
        event: cucumber::parser::Result<cucumber::Event<cucumber::event::Cucumber<TestWorld>>>,
        _cli: &Self::Cli,
    ) {
        match &event {
            Ok(event) if matches!(**event, cucumber::event::Cucumber::Started) => self.push_event("Started"),
            Ok(event) if matches!(**event, cucumber::event::Cucumber::Finished) => self.push_event("Finished"),
            _ => self.push_event("Other"),
        }
    }
}

impl writer::Arbitrary<TestWorld, &str> for MockWriter {
    async fn write(&mut self, val: &str) {
        self.push_event(&format!("Write: {val}"));
    }
}

impl writer::Stats<TestWorld> for MockWriter {
    fn passed_steps(&self) -> usize { 3 }
    fn skipped_steps(&self) -> usize { 1 }
    fn failed_steps(&self) -> usize { 2 }
    fn retried_steps(&self) -> usize { 1 }
    fn parsing_errors(&self) -> usize { 0 }
    fn hook_errors(&self) -> usize { 0 }
}

impl writer::Normalized for MockWriter {}
impl writer::NonTransforming for MockWriter {}

// Helper function to create events
fn create_started_event() -> cucumber::Event<cucumber::event::Cucumber<TestWorld>> {
    cucumber::Event::new(cucumber::event::Cucumber::Started)
}

fn create_finished_event() -> cucumber::Event<cucumber::event::Cucumber<TestWorld>> {
    cucumber::Event::new(cucumber::event::Cucumber::Finished)
}

// Or Writer Tests
#[tokio::test]
async fn test_or_writer_left_predicate() {
    use cucumber::writer::Or;

    let left = MockWriter::new("Left");
    let right = MockWriter::new("Right");
    let left_events = left.events.clone();
    let right_events = right.events.clone();

    fn always_true(_: &cucumber::parser::Result<cucumber::Event<cucumber::event::Cucumber<TestWorld>>>, _: &cli::Compose<MockCli, MockCli>) -> bool {
        true
    }

    let mut or_writer = Or::new(left, right, always_true);
    let cli = cli::Compose { left: MockCli::default(), right: MockCli::default() };
    
    or_writer.handle_event(Ok(create_started_event()), &cli).await;

    let left_result = left_events.lock().unwrap().clone();
    let right_result = right_events.lock().unwrap().clone();
    
    assert!(!left_result.is_empty());
    assert!(right_result.is_empty());
    assert_eq!(left_result[0], "Left: Started");
}

#[tokio::test]
async fn test_or_writer_right_predicate() {
    use cucumber::writer::Or;

    let left = MockWriter::new("Left");
    let right = MockWriter::new("Right");
    let left_events = left.events.clone();
    let right_events = right.events.clone();

    fn always_false(_: &cucumber::parser::Result<cucumber::Event<cucumber::event::Cucumber<TestWorld>>>, _: &cli::Compose<MockCli, MockCli>) -> bool {
        false
    }

    let mut or_writer = Or::new(left, right, always_false);
    let cli = cli::Compose { left: MockCli::default(), right: MockCli::default() };
    
    or_writer.handle_event(Ok(create_started_event()), &cli).await;

    let left_result = left_events.lock().unwrap().clone();
    let right_result = right_events.lock().unwrap().clone();
    
    assert!(left_result.is_empty());
    assert!(!right_result.is_empty());
    assert_eq!(right_result[0], "Right: Started");
}

// Tee Writer Tests
#[tokio::test]
async fn test_tee_writer_event_handling() {
    use cucumber::writer::Tee;

    let writer1 = MockWriter::new("Writer1");
    let writer2 = MockWriter::new("Writer2");
    let events1 = writer1.events.clone();
    let events2 = writer2.events.clone();

    let mut tee_writer = Tee::new(writer1, writer2);
    let cli = cli::Compose { left: MockCli::default(), right: MockCli::default() };

    tee_writer.handle_event(Ok(create_started_event()), &cli).await;

    let events1_result = events1.lock().unwrap().clone();
    let events2_result = events2.lock().unwrap().clone();
    
    assert_eq!(events1_result, vec!["Writer1: Started"]);
    assert_eq!(events2_result, vec!["Writer2: Started"]);
}

#[tokio::test]
async fn test_tee_writer_arbitrary() {
    use cucumber::writer::{Tee, Arbitrary};

    let writer1 = MockWriter::new("Writer1");
    let writer2 = MockWriter::new("Writer2");
    let events1 = writer1.events.clone();
    let events2 = writer2.events.clone();

    let mut tee_writer = Tee::new(writer1, writer2);

    Arbitrary::write(&mut tee_writer, "test message").await;

    let events1_result = events1.lock().unwrap().clone();
    let events2_result = events2.lock().unwrap().clone();
    
    assert_eq!(events1_result, vec!["Writer1: Write: test message"]);
    assert_eq!(events2_result, vec!["Writer2: Write: test message"]);
}

// Repeat Writer Tests
#[tokio::test]
async fn test_repeat_writer_with_filter() {
    use cucumber::writer::Repeat;

    let inner = MockWriter::new("Inner");
    let events = inner.events.clone();
    
    fn capture_started(event: &cucumber::parser::Result<cucumber::Event<cucumber::event::Cucumber<TestWorld>>>) -> bool {
        match event {
            Ok(event) => matches!(**event, cucumber::event::Cucumber::Started),
            _ => false,
        }
    }

    let mut repeat_writer = Repeat::new(inner, capture_started);

    repeat_writer.handle_event(Ok(create_started_event()), &MockCli::default()).await;
    repeat_writer.handle_event(Ok(create_finished_event()), &MockCli::default()).await;

    let result = events.lock().unwrap().clone();
    assert_eq!(result.len(), 3);
    assert_eq!(result[0], "Inner: Started");
    assert_eq!(result[1], "Inner: Finished");
    assert_eq!(result[2], "Inner: Started"); // Repeated event
}

#[tokio::test]
async fn test_repeat_writer_arbitrary() {
    use cucumber::writer::{Repeat, Arbitrary};

    let inner = MockWriter::new("Inner");
    let events = inner.events.clone();
    
    fn never_match(_: &cucumber::parser::Result<cucumber::Event<cucumber::event::Cucumber<TestWorld>>>) -> bool {
        false
    }
    
    let mut repeat_writer = Repeat::new(inner, never_match);

    Arbitrary::write(&mut repeat_writer, "test message").await;

    let result = events.lock().unwrap().clone();
    assert_eq!(result, vec!["Inner: Write: test message"]);
}

// Stats delegation tests
#[test]
fn test_tee_writer_stats_delegation() {
    use cucumber::writer::{Tee, Stats};

    let writer1 = MockWriter::new("Writer1");
    let writer2 = MockWriter::new("Writer2");
    let tee_writer = Tee::new(writer1, writer2);

    assert_eq!(Stats::passed_steps(&tee_writer), 3);
    assert_eq!(Stats::skipped_steps(&tee_writer), 1);
    assert_eq!(Stats::failed_steps(&tee_writer), 2);
}

#[test]
fn test_repeat_writer_stats_delegation() {
    use cucumber::writer::{Repeat, Stats};

    let inner = MockWriter::new("Inner");
    
    fn never_match(_: &cucumber::parser::Result<cucumber::Event<cucumber::event::Cucumber<TestWorld>>>) -> bool {
        false
    }
    
    let repeat_writer = Repeat::new(inner, never_match);

    assert_eq!(Stats::passed_steps(&repeat_writer), 3);
    assert_eq!(Stats::skipped_steps(&repeat_writer), 1);
    assert_eq!(Stats::failed_steps(&repeat_writer), 2);
}

// Accessor tests
#[test]
fn test_writer_accessors() {
    use cucumber::writer::{Or, Tee};

    let writer1 = MockWriter::new("Writer1");
    let writer2 = MockWriter::new("Writer2");
    let inner = MockWriter::new("Inner");
    
    fn dummy_predicate(_: &cucumber::parser::Result<cucumber::Event<cucumber::event::Cucumber<TestWorld>>>, _: &cli::Compose<MockCli, MockCli>) -> bool {
        true
    }
    
    // Test Or writer accessors
    let or_writer = Or::new(writer1.clone(), writer2.clone(), dummy_predicate);
    assert_eq!(or_writer.left_writer().id, "Writer1");
    assert_eq!(or_writer.right_writer().id, "Writer2");
    
    // Test Tee writer accessors
    let tee_writer = Tee::new(writer1.clone(), writer2.clone());
    assert_eq!(tee_writer.left_writer().id, "Writer1");
    assert_eq!(tee_writer.right_writer().id, "Writer2");
    
    // Test Repeat writer constructors (inner_writer only available for default filter)
    let _repeat_failed: writer::Repeat<TestWorld, MockWriter> = writer::Repeat::failed(inner.clone());
    let _repeat_skipped: writer::Repeat<TestWorld, MockWriter> = writer::Repeat::skipped(inner.clone());
    
    // Test that the accessor works for default constructors
    let repeat_failed: writer::Repeat<TestWorld, MockWriter> = writer::Repeat::failed(inner);
    assert_eq!(repeat_failed.inner_writer().id, "Inner");
}