# ADR-0018: Panic Payload Extraction Enhancement

## Status
Accepted (Implemented in current version)

## Context
During test execution, when steps panic, the error information was being wrapped in `Box<dyn Any>` by `catch_unwind` and then re-wrapped in an `Arc<T>` by the `coerce_into_info` function. This double-wrapping prevented proper extraction of panic payloads, resulting in generic error messages like "(Could not resolve panic payload)" instead of the actual panic content.

This significantly degraded the debugging experience for test failures, as developers couldn't see the actual assertion failures or panic messages that would help them understand what went wrong.

The issue manifested in:
- Lost assertion details (e.g., `assertion failed: (left == right) left: 1, right: 101` became generic)
- Poor debugging experience with unhelpful error messages
- Inconsistent error display across different panic types

## Decision
Enhance the `coerce_error` function in `src/writer/basic/formatting.rs` to properly handle nested panic payloads by:

1. **Direct extraction first**: Attempt to downcast directly to common panic types (`String`, `&str`)
2. **Box unwrapping**: When direct extraction fails, check if the value is `Box<dyn Any + Send>` from `catch_unwind`
3. **Nested extraction**: Extract the actual panic payload from within the Box
4. **Graceful fallback**: Fall back to generic message only when extraction is impossible

### Implementation
```rust
pub fn coerce_error(err: &Info) -> Cow<'static, str> {
    use std::any::Any;
    
    // First try direct downcast
    if let Some(s) = (**err).downcast_ref::<String>() {
        return s.clone().into();
    }
    if let Some(&s) = (**err).downcast_ref::<&str>() {
        return s.to_owned().into();
    }
    
    // Handle Box<dyn Any> from catch_unwind
    if let Some(boxed) = (**err).downcast_ref::<Box<dyn Any + Send>>() {
        if let Some(s) = boxed.downcast_ref::<String>() {
            return s.clone().into();
        }
        if let Some(&s) = boxed.downcast_ref::<&str>() {
            return s.to_owned().into();
        }
    }
    
    "(Could not resolve panic payload)".into()
}
```

## Consequences

### Positive
- **Improved debugging experience**: Developers see actual panic messages and assertion details
- **Better error visibility**: Specific failure reasons are clearly displayed
- **Maintained backwards compatibility**: Generic fallback still works for unextractable payloads
- **Enhanced test feedback**: More informative test output helps faster issue resolution
- **Consistent error handling**: Uniform panic extraction across the framework

### Negative
- **Slightly increased complexity**: Additional downcast logic in error handling path
- **Performance overhead**: Extra type checking during error processing (minimal impact)
- **Maintenance burden**: Need to handle different panic payload types

## Implementation Details

### Error Extraction Strategy
1. **Type hierarchy awareness**: Handle both direct panic types and boxed types
2. **Memory safety**: Use safe downcasting without moving values out of Boxes
3. **Performance consideration**: Fast-path for common types, fallback for edge cases

### Testing Impact
- Updated test expectations to match actual panic message formats
- Ensured test output files reflect improved error display
- Maintained test suite compatibility across all output formats

### Architectural Fit
This enhancement aligns with the framework's commitment to:
- **Developer experience**: Clear, actionable error messages
- **Robustness**: Graceful handling of edge cases
- **Maintainability**: Clean error handling patterns

## Behavioral Changes
- **Before**: Generic panic messages for most assertion failures
- **After**: Specific assertion details and panic content displayed
- **Compatibility**: All existing panic handling still works, with better output

## Example Output Improvement
```
// Before:
Step panicked. Captured output: (Could not resolve panic payload)

// After:  
Step panicked. Captured output: assertion failed: `(left == right)`
  left: `1`,
 right: `101`
```

## Use Cases
- **Test debugging**: Developers can immediately see assertion failures
- **CI/CD feedback**: Clear error messages in automated test runs
- **Error investigation**: Detailed panic information for troubleshooting
- **Development workflow**: Faster identification of test issues

## References
- Rust panic handling documentation
- `std::any::Any` trait documentation
- `catch_unwind` behavior and return types
- Test framework error reporting best practices