# ADR-0004: Event-Driven Architecture for Test Execution

## Status
Accepted

## Context
The cucumber crate needs to coordinate multiple components during test execution:
- Feature parsing and discovery
- Scenario execution with concurrency control
- Step matching and execution
- Hook execution (before/after)
- Progress reporting and output formatting
- Retry logic for failed scenarios

A tightly coupled approach would make the system rigid and hard to extend.

## Decision
Implement an event-driven architecture where:
- Components communicate through a central event stream
- Each component is responsible for specific event types
- Events flow through channels (mpsc) for decoupling
- Writers consume events to produce output
- Observers can tap into the event stream without modification

## Consequences

### Positive
- Loose coupling between components
- Easy to add new event consumers
- Clear data flow through the system
- Natural support for concurrent execution
- Simplified testing of individual components
- Support for multiple output formats

### Negative
- Indirect communication can be harder to trace
- Potential for event ordering issues
- Memory overhead of event queuing
- Complexity in event type hierarchy

## Event Flow
```
Parser -> Features Storage -> Executor -> Event Stream -> Writers/Observers
                                 ↑                     ↘
                            Retry Queue              Terminal Output
                                                    JSON Output
                                                    JUnit Output
```

## Key Events
- `Cucumber::Started` - Test run begins
- `Cucumber::ParsingFinished` - Feature parsing complete
- `Cucumber::Feature` - Feature execution events
- `Cucumber::Rule` - Rule execution events  
- `Cucumber::Scenario` - Scenario lifecycle events
- `Cucumber::Step` - Step execution events
- `Cucumber::Finished` - Test run complete

## Implementation
```rust
// Event emission
executor.send_event(event::Cucumber::scenario(
    feature,
    rule,
    scenario,
    event::RetryableScenario {
        event: event::Scenario::Started,
        retries,
    },
));

// Event consumption in writers
match event {
    Cucumber::Scenario(scenario_event) => {
        self.handle_scenario(scenario_event)
    }
    // ...
}
```

## References
- Event-Driven Architecture patterns
- Reactive Systems principles
- Actor Model (partial inspiration)