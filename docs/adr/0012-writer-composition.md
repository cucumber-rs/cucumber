# ADR-0012: Writer Composition with Tee and Or

## Status
Accepted

## Context
Users often need to:
- Output to multiple destinations simultaneously (terminal + file + metrics)
- Conditionally route output based on runtime conditions
- Compose complex output strategies from simple writers

Creating monolithic writers for each combination would lead to:
- Exponential growth in writer implementations
- Code duplication
- Inflexible configurations
- Difficulty testing complex output logic

## Decision
Implement compositional writer combinators:

1. **`writer::Tee`**: Duplicates events to multiple writers
2. **`writer::Or`**: Routes events to different writers based on predicates
3. **`writer::Repeat`**: Replays events (for retry scenarios)
4. **`writer::FailOnSkipped`**: Transforms skipped to failures

These can be composed to create complex output strategies.

## Consequences

### Positive
- Infinite flexibility through composition
- Reusable writer components
- Easy to test individual pieces
- Clean separation of concerns
- No code duplication
- Runtime configuration possible

### Negative
- More complex type signatures
- Potential performance overhead from indirection
- Debugging complex compositions can be challenging
- Need to understand composition patterns

## Composition Patterns

### Tee Pattern
```rust
// Output to multiple destinations
let writer = writer::Tee::new(
    writer::Basic::stdout(),
    writer::Tee::new(
        writer::Json::for_file("results.json"),
        MetricsWriter::new(),
    ),
);
```

### Or Pattern
```rust
// Conditional routing
let writer = writer::Or::new(
    writer::Basic::stdout(),
    writer::Json::stdout(),
    |event| matches!(event, Cucumber::Finished),
);
```

### Complex Composition
```rust
// Verbose terminal output + JSON file + fail on skipped
let writer = writer::FailOnSkipped::new(
    writer::Tee::new(
        writer::Basic::stdout().verbose(),
        writer::Json::for_file("output.json"),
    ),
);
```

## Design Principles
1. **Composability**: Writers can be freely combined
2. **Type Safety**: Compositions are verified at compile time
3. **Zero Cost**: No overhead when not used
4. **Orthogonality**: Each writer has a single responsibility

## Implementation Details
```rust
pub struct Tee<L, R> {
    left: L,
    right: R,
}

impl<W, L, R> Writer<W> for Tee<L, R>
where
    L: Writer<W>,
    R: Writer<W>,
{
    async fn handle_event(&mut self, event: &Event<W>) {
        join!(
            self.left.handle_event(event),
            self.right.handle_event(event),
        );
    }
}
```

## References
- Composite Pattern (GoF)
- Unix pipe philosophy
- Functional composition
- Monad transformers (conceptual inspiration)