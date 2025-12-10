# ADR-0002: Observer Pattern for External Test Monitoring

## Status
Accepted

## Context
External test monitoring systems like ObservaBDD need to observe test execution events without modifying the existing writer chain. The current architecture only supports writer wrappers, which:
- Cannot be added to existing Cucumber instances
- Require compile-time configuration
- Don't allow multiple independent observers
- Create tight coupling between monitoring and output formatting

## Decision
Implement an observer pattern that allows runtime registration of test observers:
- Create a `TestObserver` trait for external monitoring systems
- Add an `ObserverRegistry` to manage multiple observers
- Enable observers through an optional `observability` feature flag
- Maintain zero-cost abstraction when feature is disabled
- Provide `register_observer()` method on Cucumber instances

## Consequences

### Positive
- External systems can monitor tests without modifying writers
- Multiple observers can be registered independently
- Runtime configuration of monitoring systems
- Clean separation between output formatting and monitoring
- Zero overhead when feature is disabled
- Backward compatible with existing code

### Negative
- Additional complexity in event propagation
- Potential performance impact with many observers
- Need to maintain observer notification points
- Feature flag increases testing matrix

## Implementation
```rust
// When observability feature is enabled
cucumber
    .register_observer(Box::new(ObservaBDDAdapter::new()))
    .register_observer(Box::new(MetricsCollector::new()))
    .run_and_exit("tests/features");
```

## Architecture
```
Cucumber -> Basic Runner -> Executor -> ObserverRegistry
                                            ├── Observer 1
                                            ├── Observer 2
                                            └── Observer N
```

## References
- Observer Design Pattern (GoF)
- Feature flags for conditional compilation
- Zero-cost abstractions in Rust