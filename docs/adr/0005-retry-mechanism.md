# ADR-0005: Scenario Retry Mechanism for Flaky Tests

## Status
Accepted

## Context
In real-world test suites, some tests may fail intermittently due to:
- Network timeouts
- Resource contention
- Timing issues in async operations
- External service availability

Manual re-runs are time-consuming and may miss transient issues. Automatic retry mechanisms can improve test reliability and developer productivity.

## Decision
Implement a comprehensive retry mechanism that:
- Allows configuration of retry count per scenario
- Supports retry delays between attempts
- Can filter which scenarios are eligible for retry (via tags)
- Tracks retry attempts in events for visibility
- Integrates with the event system for proper reporting
- Preserves retry state across the execution pipeline

## Consequences

### Positive
- Improved test suite reliability
- Reduced false negatives from transient failures
- Clear visibility of flaky tests through retry reporting
- Flexible configuration via CLI args or tags
- Better CI/CD pipeline stability

### Negative
- May hide genuine intermittent bugs
- Increased total test execution time
- Additional complexity in execution flow
- Memory overhead for retry state tracking

## Configuration
```rust
// Via builder pattern
cucumber
    .retries(3)                           // Global retry count
    .retry_after(Duration::from_secs(2))  // Delay between retries
    .retry_filter(TagOperation::from("@retry"))  // Only retry tagged scenarios

// Via tags in feature files
@retry(3)
@retry-after(2s)
Scenario: Flaky network test
```

## Retry Flow
```
Scenario Fails -> Check Retry Options -> Insert into Retry Queue
                                      ↓
                    Wait for Retry Delay -> Re-execute Scenario
                                          ↓
                            Update Retry Count -> Report Final Status
```

## Event Structure
```rust
pub struct Retries {
    pub current: usize,  // Current attempt number
    pub left: usize,     // Remaining retries
}

pub struct RetryableScenario<World> {
    pub event: Scenario<World>,
    pub retries: Option<Retries>,
}
```

## Implementation Details
- Retry state flows through: CLI -> Runner -> Executor -> Events
- Failed scenarios are re-queued with decremented retry count
- Final status only reported after all retries exhausted
- Each retry attempt is visible in the event stream

## References
- Test retry patterns in testing frameworks
- Exponential backoff strategies
- Cucumber-JVM retry functionality