# Architecture Decision Records

This directory contains Architecture Decision Records (ADRs) for the cucumber crate.

## What is an ADR?

An Architecture Decision Record captures an important architectural decision made along with its context and consequences. ADRs help future developers understand why certain decisions were made.

## ADR Index

| ADR | Title | Status | Summary |
|-----|-------|--------|---------|
| [0001](0001-modular-architecture.md) | Modular Architecture with 300 LOC Limit | Accepted | Enforce modular design with files limited to 300 lines for better maintainability |
| [0002](0002-observer-pattern-integration.md) | Observer Pattern for External Test Monitoring | Accepted | Enable runtime registration of test observers for external monitoring systems |
| [0003](0003-step-enum-struct-format.md) | Step Enum Using Struct Variants | Accepted | Convert Step enum from tuple to struct variants for better readability |
| [0004](0004-event-driven-architecture.md) | Event-Driven Architecture | Accepted | Use event streams for loose coupling between components |
| [0005](0005-retry-mechanism.md) | Scenario Retry Mechanism | Accepted | Automatic retry for flaky tests with configurable options |
| [0006](0006-concurrent-execution-model.md) | Concurrent Scenario Execution | Accepted | Controlled parallel execution with serial/concurrent classification |
| [0007](0007-world-lifecycle-management.md) | World Lifecycle Management | Accepted | Per-scenario World instances with proper initialization and cleanup |

## ADR Template

When creating a new ADR, use this template:

```markdown
# ADR-XXXX: [Decision Title]

## Status
[Proposed | Accepted | Deprecated | Superseded by ADR-YYYY]

## Context
[Describe the issue motivating this decision]

## Decision
[Describe the change that we're proposing and/or doing]

## Consequences

### Positive
[List positive outcomes]

### Negative
[List negative outcomes]

## References
[Links to relevant documentation, discussions, or related ADRs]
```

## Contributing

When making significant architectural changes:
1. Create a new ADR with the next sequential number
2. Follow the template structure
3. Link related ADRs where appropriate
4. Update this README with the new entry
5. Include the ADR in the same PR as the implementation