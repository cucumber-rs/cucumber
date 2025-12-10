# ADR-0009: Migration from async_trait to Native async fn

## Status
Accepted (Implemented in v0.21.0)

## Context
Originally, Rust did not support `async fn` in traits. The `async_trait` macro was the standard workaround, but it had drawbacks:
- Performance overhead from heap allocations (Boxing)
- Less readable error messages
- Additional macro complexity
- Extra dependency

Rust 1.75 stabilized native `async fn` in traits, presenting an opportunity to improve the codebase.

## Decision
Remove the `#[async_trait]` attribute from all public traits and migrate to native `async fn`:
- `World` trait
- `Writer` trait  
- `writer::Arbitrary` trait

This is a breaking change but aligns with Rust's evolution and improves performance.

## Consequences

### Positive
- Better performance (no boxing overhead)
- Clearer error messages
- Simpler code without macro magic
- Reduced dependencies
- More idiomatic Rust
- Better IDE support and type inference

### Negative
- Breaking change for all users
- Requires MSRV bump to 1.75
- Migration effort for existing codebases
- Lost compatibility with older Rust versions

## Migration Guide
```rust
// Before (with async_trait)
#[async_trait(?Send)]
impl World for MyWorld {
    type Error = Infallible;
    
    async fn new() -> Result<Self, Self::Error> {
        Ok(Self::default())
    }
}

// After (native async)
impl World for MyWorld {
    type Error = Infallible;
    
    async fn new() -> Result<Self, Self::Error> {
        Ok(Self::default())
    }
}
```

## Performance Impact
- Eliminated heap allocations for async function returns
- Reduced indirection in hot paths
- Particularly beneficial for high-frequency operations like step execution

## Timeline
- v0.20.x: Last version with `async_trait`
- v0.21.0: Migration to native `async fn`
- MSRV bumped from 1.70 to 1.75

## References
- [Rust RFC: async fn in traits](https://rust-lang.github.io/rfcs/3185-static-async-fn-in-trait.html)
- [async_trait crate documentation](https://docs.rs/async-trait)