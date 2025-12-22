# ADR-0020: Rust 1.90+ Language Modernization

## Status

Accepted

## Context

Rust 1.90 introduced several language improvements that make code more idiomatic and readable:

1. **`is_some_and` method** - Provides a cleaner way to check Option values with predicates
2. **`?` operator in closures** - Allows using the `?` operator in more contexts
3. **Pattern simplifications** - More ergonomic patterns for Option and Result handling

Our codebase was using older patterns that, while functionally correct, were less readable and more verbose than modern Rust idioms.

## Decision

We modernized the codebase to use Rust 1.90+ language features following these principles:

### 1. Replace map comparisons with `is_some_and`

**Before:**
```rust
self.arg_name_of_step_context.as_ref().map(|i| *i == *ident) == Some(true)
```

**After:**
```rust
self.arg_name_of_step_context.as_ref().is_some_and(|i| i == ident)
```

### 2. Use `?` operator in closure chains

**Before:**
```rust
.or_else(|| rule.and_then(|r| parse_tags(&r.tags)))
```

**After:**
```rust
.or_else(|| parse_tags(&rule?.tags))
```

### 3. Simplify Option handling patterns

**Before:**
```rust
func.sig.inputs.iter().find_map(|arg| {
    if let Ok((ident, _)) = parse_fn_arg(arg) {
        if ident == "step" {
            return Some(ident.clone());
        }
    }
    None
})
```

**After:**
```rust
func.sig.inputs.iter().find_map(|arg| {
    parse_fn_arg(arg).ok().and_then(|(ident, _)| {
        (ident == "step").then(|| ident.clone())
    })
})
```

## Consequences

### Positive

- **Improved readability** - Modern idioms are clearer and more concise
- **Reduced boilerplate** - Less verbose code with same functionality
- **Better error handling** - `?` operator in more contexts reduces nesting
- **Alignment with ecosystem** - Following Rust community best practices

### Negative

- **Higher MSRV** - Requires Rust 1.90 or later (already bumped to 1.88)
- **Learning curve** - Developers need familiarity with newer patterns

### Neutral

- **No functional changes** - Pure refactoring with identical behavior
- **Clippy compliance** - Satisfies newer clippy lints

## Implementation

Changes were applied across the codebase, primarily in:
- `codegen/src/attribute.rs` - Step attribute processing
- `src/runner/basic/cli_and_types.rs` - CLI option handling
- Various test files

All changes maintain backward compatibility at the API level while modernizing internal implementation.

## References

- [Rust 1.70 Release Notes - is_some_and](https://blog.rust-lang.org/2023/06/01/Rust-1.70.0.html)
- [PR #2 from upstream](https://github.com/sravinet/cucumber/pull/2)
- [Clippy lint: option_map_unit_fn](https://rust-lang.github.io/rust-clippy/master/index.html#option_map_unit_fn)