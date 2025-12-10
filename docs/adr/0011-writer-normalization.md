# ADR-0011: Writer Normalization Pattern

## Status
Accepted (Original Design)

## Context
Cucumber events can arrive in different orders depending on:
- Concurrent vs serial execution
- Retry attempts
- Runner implementation details
- Network delays (in distributed setups)

Writers need consistent, predictable event ordering to:
- Produce deterministic output
- Properly nest output hierarchies
- Avoid race conditions in display
- Support proper indentation and formatting

## Decision
Introduce a `Normalize` writer wrapper that:
- Buffers and reorders events as needed
- Ensures proper nesting (Feature → Rule → Scenario → Step)
- Handles concurrent execution while maintaining logical order
- Is applied by default to `writer::Basic`

The `writer::Basic::new()` method returns `Normalize<writer::Basic>` rather than bare `writer::Basic`.

## Consequences

### Positive
- Deterministic output regardless of execution order
- Correct visual hierarchy in terminal output
- Writers can assume normalized event streams
- Simplifies writer implementations
- Prevents output corruption

### Negative
- Additional buffering overhead
- Slight delay in output (waiting for reordering)
- Memory usage for buffered events
- Complexity in the normalization logic

## Architecture
```rust
// Events flow through normalization
Runner → Events (unordered) → Normalize → Events (ordered) → Writer

// Automatic wrapping
let writer = writer::Basic::new(); // Returns Normalize<Basic>
```

## Normalization Rules
1. Feature events bracket all child events
2. Rule events bracket their scenarios
3. Scenario events bracket their steps
4. Background steps appear before scenario steps
5. Retry attempts are grouped
6. Started events precede Finished events

## Example
```rust
// Unordered events from concurrent execution:
Step(scenario2, step1) → Step(scenario1, step2) → Scenario(scenario1, Started)

// After normalization:
Scenario(scenario1, Started) → Step(scenario1, step2) → Step(scenario2, step1)
```

## Implementation
```rust
impl writer::Basic {
    pub fn new() -> writer::Normalize<Self> {
        writer::Normalize::new(Self {
            // configuration
        })
    }
}

// Users can skip normalization if needed
let unnormalized = writer::Basic::raw(); // hypothetical API
```

## References
- Event Sourcing patterns
- Message reordering in distributed systems
- Terminal UI best practices