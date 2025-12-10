# ADR-0006: Concurrent Scenario Execution Model

## Status
Accepted

## Context
Serial test execution can be prohibitively slow for large test suites. However, unrestricted parallel execution can cause:
- Resource exhaustion
- Test interference and flakiness
- Race conditions in shared resources
- Difficulty in debugging failures

A balanced concurrency model is needed that provides speed benefits while maintaining control and reliability.

## Decision
Implement a controlled concurrency model with:
- Configurable maximum concurrent scenarios (default: 64)
- Scenario classification into Concurrent vs Serial execution
- Serial scenarios run one at a time in order
- Concurrent scenarios run in parallel up to the limit
- Tag-based control (@serial tag forces serial execution)
- Feature and Rule level coordination for proper event ordering

## Consequences

### Positive
- Significant speedup for large test suites
- Maintains execution order where needed
- Prevents resource exhaustion
- Clear control via tags
- Predictable behavior for debugging

### Negative
- Complexity in execution scheduling
- Need to handle concurrent World instances
- Potential for test interference if not properly isolated
- More complex event ordering logic

## Execution Model
```
Scenario Queue -> Classifier -> Concurrent Queue -> Executor Pool
                            ↘                     ↙ (max N workers)
                              Serial Queue -> Serial Executor
                                            (single worker)
```

## Configuration
```rust
// Set max concurrent scenarios
cucumber.max_concurrent_scenarios(10)

// Classification function
cucumber.which_scenario(|feature, rule, scenario| {
    if scenario.tags.contains("@serial") {
        ScenarioType::Serial
    } else {
        ScenarioType::Concurrent
    }
})
```

## Synchronization Points
- Feature start/end events are properly ordered
- Rule start/end events are properly ordered
- Scenarios within a rule maintain relative ordering
- Serial scenarios block until all prior scenarios complete

## Implementation
```rust
// Concurrent execution with FuturesUnordered
let mut run_scenarios = stream::FuturesUnordered::new();
for (id, feature, rule, scenario, ty, retry) in runnable {
    if ty == ScenarioType::Concurrent {
        run_scenarios.push(executor.run_scenario(...));
    } else {
        // Wait for all concurrent to finish first
        while let Some(_) = run_scenarios.next().await { }
        executor.run_scenario(...).await;
    }
}
```

## References
- Tokio async runtime patterns
- Test parallelization in Jest/Mocha
- Rust futures and async/await