# ADR-0015: Stats Collection Architecture

## Status
Accepted (Evolved from v0.10.0 to v0.14.0)

## Context
Test execution metrics are crucial for:
- Understanding test suite health
- Identifying flaky tests
- Tracking test performance
- Generating reports
- CI/CD decisions

Initially, only failure tracking existed (`writer::Failure`). This was insufficient for comprehensive test analytics.

## Decision
Evolve from simple failure tracking to comprehensive statistics collection:
- Replace `writer::Failure` with `writer::Stats`
- Track all step states: passed, failed, skipped, retried
- Provide granular metrics for features, rules, scenarios, and steps
- Support filtering and aggregation
- Make stats accessible to multiple writers

## Consequences

### Positive
- Complete visibility into test execution
- Support for sophisticated reporting
- Identify patterns in test failures
- Track retry effectiveness
- Enable data-driven decisions

### Negative
- Memory overhead for stats tracking
- Additional complexity in event processing
- Need to maintain accuracy during concurrent execution
- Stats synchronization overhead

## Stats Architecture
```rust
pub struct Stats {
    pub features: Counter<Passed, Failed, Skipped>,
    pub rules: Counter<Passed, Failed, Skipped>,
    pub scenarios: Counter<Passed, Failed, Skipped>,
    pub steps: Counter<Passed, Failed, Skipped>,
    pub parsing_errors: usize,
    pub failed_hooks: usize,
    retried_steps: usize,
}

impl Stats {
    pub fn passed_steps(&self) -> usize;
    pub fn failed_steps(&self) -> usize;
    pub fn skipped_steps(&self) -> usize;
    pub fn retried_steps(&self) -> usize;
    pub fn execution_time(&self) -> Duration;
}
```

## Collection Flow
```
Event Stream → Stats Collector → Stats Snapshot → Writers/Reports
                     ↓
              Atomic Updates
                     ↓
              Thread-Safe Counters
```

## Usage Patterns

### Summary Writer
```rust
let writer = writer::Summarize::new(writer::Basic::default());
// Automatically displays stats at the end
```

### Custom Reports
```rust
impl Writer for CustomReporter {
    async fn handle_event(&mut self, event: &Event) {
        self.stats.update(event);
        
        if let Cucumber::Finished = event {
            self.generate_report(&self.stats);
        }
    }
}
```

### CI Integration
```rust
// Fail build if success rate < 95%
let stats = cucumber.run().await;
if stats.success_rate() < 0.95 {
    process::exit(1);
}
```

## Metrics Tracked
- **Features**: Total, passed, failed, skipped
- **Rules**: Total, passed, failed, skipped
- **Scenarios**: Total, passed, failed, skipped, retried
- **Steps**: Total, passed, failed, skipped, retried
- **Hooks**: Failed before/after hooks
- **Parsing**: Parser errors count
- **Timing**: Execution duration

## Evolution Timeline
1. v0.10.0: `writer::Failure` for basic failure tracking
2. v0.12.0: Added skipped step tracking
3. v0.13.0: Renamed to `writer::Stats` with comprehensive metrics
4. v0.14.0: Added retry statistics and timing information

## References
- Test metrics best practices
- xUnit test result format
- Cucumber reports format specification