# ADR-0003: Step Enum Using Struct Variants Instead of Tuples

## Status
Accepted

## Context
The Step enum used tuple variants for its states (e.g., `Step::Passed(CaptureLocations, Option<Location>)`), which:
- Made the code less readable and self-documenting
- Required remembering parameter positions
- Made pattern matching verbose and error-prone
- Complicated adding new fields in the future

## Decision
Convert all Step enum variants from tuple format to struct format with named fields:
```rust
// Before
Step::Passed(CaptureLocations, Option<Location>)
Step::Failed(Option<CaptureLocations>, Option<Location>, Option<Arc<World>>, StepError)

// After
Step::Passed {
    captures: CaptureLocations,
    location: Option<Location>,
}
Step::Failed {
    captures: Option<CaptureLocations>,
    location: Option<Location>,
    world: Option<Arc<World>>,
    error: StepError,
}
```

## Consequences

### Positive
- Self-documenting code with named fields
- Easier to understand at usage sites
- Simplified pattern matching with field names
- Better IDE support and autocomplete
- Easier to add new fields without breaking positions
- Clearer API for library users

### Negative
- Slightly more verbose syntax
- Breaking change for existing code
- Need to update all test fixtures
- Larger diff for the refactoring

## Migration Impact
- All pattern matches need updating to use field names
- Test output fixtures require regeneration
- External crates using the Step enum need updates

## Implementation Example
```rust
// Pattern matching becomes clearer
match step {
    Step::Passed { captures, location } => {
        // Clear what each field represents
    }
    Step::Failed { error, .. } => {
        // Can easily destructure specific fields
    }
}
```

## References
- Rust API Guidelines - C-STRUCT-VARIANT
- Clean Code principles for self-documenting code