# ADR-0017: Fail-Fast Mechanism

## Status
Accepted (Implemented in v0.11.3)

## Context
During development and debugging, running an entire test suite after encountering failures is often wasteful:
- Wastes CI/CD resources
- Delays feedback to developers
- Makes debugging harder with too many failures
- Can cascade into more failures

However, always stopping on first failure would:
- Hide multiple independent issues
- Make it hard to assess overall test health
- Prevent gathering comprehensive failure data

A configurable fail-fast mechanism provides balance.

## Decision
Implement fail-fast behavior that:
- Stops scheduling new scenarios after first failure
- Allows already-started scenarios to complete
- Considers retry exhaustion as failure trigger
- Is disabled by default
- Can be enabled via CLI flag or builder method

## Consequences

### Positive
- Faster feedback during development
- Resource savings in CI pipelines
- Easier debugging with focused failures
- Respects work already in progress
- Optional and backward compatible

### Negative
- May hide related failures
- Incomplete test results
- Complex interaction with concurrent execution
- Retry logic adds complexity

## Implementation Details

### Execution Behavior
```rust
// Fail-fast logic in executor
loop {
    let scenarios = get_next_batch();
    
    for scenario in scenarios {
        let result = run_scenario(scenario).await;
        
        if fail_fast && result.is_failed() && !result.will_retry() {
            // Stop scheduling new scenarios
            stop_scheduling = true;
            break;
        }
    }
    
    if stop_scheduling {
        // Wait for in-progress scenarios
        wait_for_running_scenarios().await;
        break;
    }
}
```

### Configuration
```rust
// Via builder
cucumber
    .fail_fast()
    .run("tests/features")
    .await;

// Via CLI
cargo test -- --fail-fast
```

### Interaction with Retries
- Failure only triggers fail-fast after all retries exhausted
- Prevents premature termination on flaky tests
- Maintains test reliability

## Behavioral Rules
1. **Started scenarios complete**: Never interrupt running scenarios
2. **Queued scenarios cancelled**: Don't start new work after failure
3. **Retries respected**: Only fail-fast after final retry attempt
4. **Features/Rules continue**: Current feature/rule completes its scenarios
5. **Stats accurate**: All executed scenarios included in final stats

## Example Execution Flow
```
Scenario 1: Started → Passed
Scenario 2: Started → Failed (will retry)
Scenario 3: Started → (continues running)
Scenario 4: Queued → (cancelled due to fail-fast)
Scenario 2: Retry → Failed (no more retries)
→ Fail-fast triggered
Scenario 3: → Passed (allowed to complete)
→ Execution stops
```

## Use Cases
- **Development**: Quick feedback while writing tests
- **Debugging**: Focus on first failure
- **CI Smoke Tests**: Fail fast on critical issues
- **Resource Conservation**: Stop expensive test runs early

## References
- JUnit's fail-fast behavior
- Mocha's --bail option
- Test runner best practices