# ADR-0007: World Lifecycle Management

## Status
Accepted

## Context
In Cucumber, the World object represents the test context and state shared across steps within a scenario. Proper lifecycle management is crucial for:
- Test isolation between scenarios
- Resource cleanup
- State initialization
- Memory efficiency

## Decision
Implement a per-scenario World lifecycle where:
- Each scenario gets a fresh World instance
- World is created asynchronously via `World::new()`
- World persists throughout all steps of a scenario
- World is accessible in before/after hooks
- World is properly dropped after scenario completion
- Failed World creation is treated as a before hook failure

## Consequences

### Positive
- Complete isolation between scenarios
- Clean state for each test
- Proper resource management
- Support for async initialization
- Clear ownership model

### Negative
- Memory overhead for concurrent scenarios
- Cannot share expensive resources between scenarios
- Repeated initialization cost
- Need for explicit World trait implementation

## World Trait
```rust
#[async_trait(?Send)]
pub trait World: Sized + 'static {
    type Error: Display;
    
    async fn new() -> Result<Self, Self::Error>;
}
```

## Lifecycle Flow
```
Scenario Start
    ↓
World::new() → Error? → Report as Before Hook Failure
    ↓ OK
Before Hook(world)
    ↓
Execute Steps(world)
    ↓
After Hook(world, scenario_result)
    ↓
Drop World
    ↓
Scenario End
```

## Error Handling
```rust
// World creation failure
let mut world = match W::new().await {
    Ok(world) => world,
    Err(e) => {
        // Emit as before hook failure
        self.send_event(Hook::Failed(None, error_info));
        return;
    }
};
```

## Hook Integration
```rust
// Before hook has mutable access
before_hook(&feature, rule.as_deref(), &scenario, &mut world).await;

// After hook receives optional world (may be None if creation failed)
after_hook(&feature, rule.as_deref(), &scenario, &result, Some(&mut world)).await;
```

## References
- Cucumber World concept
- RAII pattern in Rust
- Test isolation principles