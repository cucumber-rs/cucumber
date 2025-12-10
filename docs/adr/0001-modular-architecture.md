# ADR-0001: Modular Architecture with 300 LOC Limit

## Status
Accepted

## Context
The cucumber crate had several large files exceeding 1000 lines of code, making them difficult to understand, test, and maintain. Large files violate the Single Responsibility Principle and increase cognitive load for developers.

## Decision
We will enforce a modular architecture where:
- No implementation file should exceed 300 lines of code (including inline unit tests)
- Large modules must be broken down into focused sub-modules
- Each module should have a single, clear responsibility
- Related functionality should be grouped in module directories with a mod.rs file

## Consequences

### Positive
- Improved code readability and maintainability
- Easier to understand individual components
- Better testability with focused unit tests
- Clearer separation of concerns
- Reduced merge conflicts in team development
- Faster compilation times for incremental builds

### Negative
- More files to navigate
- Potential for over-modularization
- Need to maintain module boundaries
- Increased import statements

## Implementation
Example modularization of executor.rs (1000+ LOC) into:
```
executor/
├── mod.rs       # Public interface and integration tests
├── core.rs      # Main Executor struct and orchestration (~250 LOC)
├── events.rs    # Event sending functionality (~150 LOC)
├── hooks.rs     # Before/after hook execution (~200 LOC)
└── steps.rs     # Step execution logic (~250 LOC)
```

## References
- Single Responsibility Principle (SOLID)
- Clean Code by Robert C. Martin