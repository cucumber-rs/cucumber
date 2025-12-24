# ADR-0021: Background Execution in Rules

## Status

Accepted

## Context

Gherkin supports both Feature-level and Rule-level backgrounds. The interaction between these backgrounds and scenarios within rules was not correctly implemented in the Rust cucumber implementation.

The issue arose when scenarios within rules were not executing feature-level background steps, which deviated from the canonical Cucumber behavior established by cucumber-js and cucumber-ruby.

### Current Behavior (Before Fix)
- Feature-level backgrounds were only executed for scenarios directly under the feature
- Scenarios within rules did not receive feature-level background steps
- This caused test failures and incorrect world state initialization

### Expected Canonical Behavior
Based on cucumber-js implementation and Gherkin specification:
- Feature-level backgrounds should execute for ALL scenarios, including those within rules
- Rule-level backgrounds should execute only for scenarios within that specific rule
- Execution order should be: Feature Background → Rule Background → Scenario steps

## Decision

We modified the `StepExecutor::run_steps` function to correctly collect and execute background steps in the proper order:

1. **Step Collection Phase**: Before executing any steps, collect all applicable steps:
   - Feature-level background steps (if any)
   - Rule-level background steps (if any)
   - Scenario steps

2. **Step Execution Phase**: Execute all collected steps in order, properly distinguishing between:
   - Background steps (emit Background events)
   - Regular scenario steps (emit Step events)

3. **Event Emission**: Ensure proper event types are emitted for observability:
   - `Scenario::Background` events for background steps
   - `Scenario::Step` events for regular steps

## Implementation Details

The fix involved modifying `/src/runner/basic/executor/steps.rs`:

```rust
// Collect all steps to execute (background steps + scenario steps)
let mut all_steps = Vec::new();

// 1. Add feature-level background steps (if any)
if let Some(background) = &feature.background {
    for step in &background.steps {
        all_steps.push((step.clone(), true)); // true = background step
    }
}

// 2. Add rule-level background steps (if any)
if let Some(ref rule) = rule {
    if let Some(background) = &rule.background {
        for step in &background.steps {
            all_steps.push((step.clone(), true)); // true = background step
        }
    }
}

// 3. Add scenario steps
for step in &scenario.steps {
    all_steps.push((step.clone(), false)); // false = regular step
}
```

## Consequences

### Positive

1. **Standards Compliance**: Aligns with canonical Cucumber behavior as implemented in cucumber-js and cucumber-ruby
2. **Correct World Initialization**: Ensures proper world state setup through feature-level backgrounds
3. **Test Compatibility**: Fixes failing output tests that expect Background events
4. **Predictable Behavior**: Users familiar with other Cucumber implementations will find expected behavior
5. **Proper Event Ordering**: Background events are correctly emitted before scenario events

### Negative

1. **Breaking Change**: Scenarios that previously didn't execute feature backgrounds now will, potentially breaking tests that relied on the incorrect behavior
2. **Performance Impact**: Additional steps are executed for scenarios in rules (though this is the correct behavior)

### Neutral

1. **Migration Path**: Existing test suites may need adjustment if they relied on the previous incorrect behavior
2. **Documentation**: Requires clear documentation of background inheritance rules

## Verification

The fix was verified against:
- cucumber-js test suite at `dev/shared-bdd/features/core/background.feature`
- Output tests expecting Background events for scenarios in rules
- Integration tests confirming proper step execution order

## Example

Given this feature file:
```gherkin
Feature: Example
  Background:
    Given feature background step

  Rule: Example Rule
    Background:
      Given rule background step
    
    Scenario: Example Scenario
      When scenario step
      Then another step
```

The execution order is now correctly:
1. `Given feature background step` (Background event)
2. `Given rule background step` (Background event)
3. `When scenario step` (Step event)
4. `Then another step` (Step event)

## References

- [Gherkin Rules Specification](https://cucumber.io/docs/gherkin/reference/#rule)
- [cucumber-js Background Implementation](https://github.com/cucumber/cucumber-js)
- PR: Fix background execution for scenarios in rules
- Related issue: Output tests failing due to missing Background events

## Decision Makers

- Cucumber-rs maintainers
- Community feedback via GitHub issues

## Date

2024-12-24