# ADR-0010: World Trait Unification

## Status
Accepted (Implemented in v0.14.0)

## Context
Originally, the framework had two separate traits:
- `World`: Basic trait for test context
- `WorldInit`: Trait for world initialization logic

This separation caused several issues:
- Confusing for users (which trait to implement?)
- Boilerplate code to implement both traits
- Unnecessary complexity in the type system
- Poor ergonomics for the common case

Most users just wanted a simple way to define their test context.

## Decision
Merge `WorldInit` trait into `World` trait and provide `#[derive(World)]` macro:
- Single `World` trait with `async fn new()` method
- Derive macro for automatic implementation
- Remove need for manual trait implementation in common cases

## Consequences

### Positive
- Simpler API with single trait
- Less boilerplate for users
- Clearer documentation
- Better developer experience
- Easier to teach and learn

### Negative
- Breaking change for existing code
- Lost fine-grained control (mitigated by manual impl option)
- Macro magic might hide details

## Implementation
```rust
// Before: Two traits, manual implementation
struct MyWorld {
    state: String,
}

impl World for MyWorld {
    type Error = Infallible;
}

impl WorldInit for MyWorld {
    async fn new() -> Result<Self, Self::Error> {
        Ok(Self {
            state: String::new(),
        })
    }
}

// After: Single trait with derive
#[derive(Debug, Default, World)]
struct MyWorld {
    state: String,
}

// Or manual implementation for complex cases
impl World for ComplexWorld {
    type Error = MyError;
    
    async fn new() -> Result<Self, Self::Error> {
        // Complex initialization
    }
}
```

## Design Rationale
- Follows Rust ecosystem patterns (like `Default` derive)
- Optimizes for the common case (simple initialization)
- Preserves flexibility for complex cases
- Reduces cognitive load

## Migration Path
1. Remove `WorldInit` implementation
2. Either:
   - Add `#[derive(World)]` for default initialization
   - Implement `World::new()` directly for custom initialization

## References
- Rust API Guidelines: Convenience traits
- Similar patterns: serde's Serialize/Deserialize