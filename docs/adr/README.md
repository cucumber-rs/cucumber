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
| [0008](0008-three-component-architecture.md) | Three-Component Architecture | Accepted (Original) | Parser-Runner-Writer separation for ultimate extensibility |
| [0009](0009-async-trait-migration.md) | Async Trait Migration | Accepted (v0.21.0) | Migration from async_trait macro to native async fn in traits |
| [0010](0010-world-trait-unification.md) | World Trait Unification | Accepted (v0.14.0) | Merge WorldInit into World trait with derive macro |
| [0011](0011-writer-normalization.md) | Writer Normalization | Accepted (Original) | Automatic event reordering for consistent output |
| [0012](0012-writer-composition.md) | Writer Composition | Accepted | Tee and Or combinators for complex output strategies |
| [0013](0013-performance-optimizations.md) | Performance Optimizations | Accepted (v0.22.0) | Memory and performance improvements for large files |
| [0014](0014-libtest-integration.md) | Libtest Integration | Accepted (v0.13.0) | IDE test runner support via libtest protocol |
| [0015](0015-stats-collection.md) | Stats Collection | Accepted | Comprehensive test execution metrics tracking |
| [0016](0016-cli-trait-pattern.md) | CLI Trait Pattern | Accepted (Original) | Component-specific CLI configuration via associated types |
| [0017](0017-fail-fast-mechanism.md) | Fail-Fast Mechanism | Accepted (v0.11.3) | Stop test execution on first failure for faster feedback |
| [0018](0018-panic-payload-extraction.md) | Panic Payload Extraction Enhancement | Accepted | Enhanced error handling to display actual panic messages instead of generic placeholders |
| [0019](0019-data-table-api.md) | DataTable API and Direct Parameter Support | Accepted | Rich DataTable API with direct parameter injection matching cucumber-js patterns |
| [0020](0020-rust-190-modernization.md) | Rust 1.90+ Language Modernization | Accepted | Adopt modern Rust idioms including is_some_and and improved error handling patterns |

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