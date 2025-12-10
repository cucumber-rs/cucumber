# ADR-0008: Three-Component Architecture (Parser-Runner-Writer)

## Status
Accepted (Original Design)

## Context
Testing frameworks often become monolithic, making it difficult to:
- Source features from non-file systems (databases, queues, APIs)
- Execute tests in non-standard ways (distributed, cloud-based)
- Output results to multiple or custom formats
- Extend functionality without modifying core code

A tightly coupled architecture would limit the framework's applicability to diverse use cases.

## Decision
Structure the framework around three independent, replaceable components:
1. **Parser**: Sources and emits features as a Stream
2. **Runner**: Executes scenarios and emits events
3. **Writer**: Consumes events and produces output

Each component is defined as a trait, allowing complete replacement with custom implementations.

## Consequences

### Positive
- Ultimate flexibility for exotic use cases
- Clean separation of concerns
- Easy to add new parsers, runners, or writers
- Can compose complex pipelines
- Enables distributed architectures
- No need to fork the framework for custom needs

### Negative
- More complex initial learning curve
- Need to understand trait boundaries
- Potential for incompatible component combinations
- More abstraction layers

## Architecture
```rust
Cucumber<W, P, I, R, Wr>
    where
        P: Parser,     // Feature source
        R: Runner,     // Execution engine
        Wr: Writer,    // Output handler
```

## Example Use Cases
1. **Distributed Testing**: Parser reads from Kafka, Runner executes on Kubernetes, Writer sends to multiple reporting systems
2. **Database Features**: Parser queries test scenarios from database
3. **Real-time Monitoring**: Writer streams events to monitoring dashboard
4. **Custom Formats**: Writer produces company-specific report formats

## Implementation
```rust
// Default configuration
let cucumber = AnimalWorld::cucumber()
    .with_parser(parser::Basic::new())  // reads .feature files
    .with_runner(runner::Basic::default()) // concurrent execution
    .with_writer(writer::Basic::default()); // terminal output

// Custom configuration
let cucumber = AnimalWorld::cucumber()
    .with_parser(CustomDatabaseParser::new())
    .with_runner(DistributedRunner::new())
    .with_writer(writer::Tee::new(
        writer::Basic::default(),
        CustomMetricsWriter::new(),
    ));
```

## References
- Strategy Pattern (GoF)
- Hexagonal Architecture
- Plugin Architecture Pattern