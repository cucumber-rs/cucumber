# ADR-0014: Libtest Integration for IDE Support

## Status
Accepted (Implemented in v0.13.0)

## Context
Rust IDEs, particularly IntelliJ Rust, provide excellent test runner integration through the libtest protocol. However, Cucumber tests were not visible to IDE test runners because:
- Cucumber uses its own test execution model
- No integration with Rust's built-in test framework
- IDE couldn't discover or run individual scenarios
- No clickable test results in IDE

Developers were forced to use terminal for all Cucumber test execution, losing IDE benefits like:
- Click-to-run individual tests
- Visual test results
- Debugging integration
- Test failure navigation

## Decision
Implement `writer::Libtest` that outputs in libtest JSON format:
- Behind `libtest` feature flag for optional dependency
- Translates Cucumber events to libtest protocol
- Enables full IDE integration
- Supports --format json output

## Consequences

### Positive
- Full IDE test runner integration
- Click-to-run scenarios from IDE
- Visual pass/fail indicators
- Integrated test results view
- Better developer experience
- Debugging support in IDEs

### Negative
- Additional complexity in event translation
- Feature flag increases test matrix
- Potential version compatibility issues
- Not all Cucumber features map cleanly to libtest

## Implementation
```rust
// Enable libtest output
#[tokio::main]
async fn main() {
    MyWorld::cucumber()
        .with_writer(writer::Libtest::new())
        .run_and_exit("tests/features")
        .await;
}

// Or via CLI
// cargo test --test cucumber -- --format json
```

## Libtest Protocol Mapping
| Cucumber Event | Libtest Output |
|---------------|----------------|
| Feature | Test Suite |
| Scenario | Test Case |
| Step | Test Output/Assertion |
| Passed | `"status": "ok"` |
| Failed | `"status": "failed"` |
| Skipped | `"status": "ignored"` |

## IDE Features Enabled
1. **Test Discovery**: IDEs can find all scenarios
2. **Selective Execution**: Run single scenarios or features
3. **Result Visualization**: Tree view of test results
4. **Failure Navigation**: Click to jump to failures
5. **Test History**: Track test runs over time
6. **Debugging**: Set breakpoints in step definitions

## Configuration
```toml
# Cargo.toml
[dependencies]
cucumber = { version = "0.21", features = ["libtest"] }

# .cargo/config.toml
[test]
protocol = "json"
```

## Limitations
- Scenario Outlines appear as single test
- Background steps not separately visible
- Retry attempts shown as single result
- Some Cucumber-specific metadata lost

## References
- [Libtest JSON format documentation](https://doc.rust-lang.org/rustc/tests/index.html)
- [IntelliJ Rust test integration](https://github.com/intellij-rust/intellij-rust)
- Similar: Jest IDE integration