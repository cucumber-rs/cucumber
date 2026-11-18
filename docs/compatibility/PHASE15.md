# Phase 15: explicit strict table row sizing

Applications opt in with
`World::cucumber().with_table_semantics(gherkin::TableSemantics::Gherkin43)`.
`parser::Basic` exposes the same builder, and successful source snapshots record
the policy through `SourceSnapshot::table_semantics()`. The setting is independent
of dialect, doc-string and Examples interpolation policies. Their existing
values are preserved.

The parser retains all cells under the strict policy, then rejects any later row
whose width differs from the first row. This applies to step and Examples tables.
Legacy remains the default and retains historical excess-cell truncation. The
public native AST is unchanged. Failed parses do not publish successful snapshots.

## Parser prerequisite and official evidence

The existing GQAdonis/gherkin fork owns `TableSemantics::{Legacy, Gherkin43}` and
`GherkinEnv::with_table_semantics`. Production parser/lib files were copied into
`vendor/gherkin`; hashes and provenance were updated. No new fork is needed.

Pinned contract: Gherkin v43.0.0 commit
`3d70e2bb4e0f16a568ee539a5ce0b03fb247a777`.
[AstBuilder.ensureCellCount](https://github.com/cucumber/gherkin/blob/v43.0.0/javascript/src/AstBuilder.ts#L120)
rejects both shorter and longer rows. Exact upstream inconsistent-cell-count
source and error fixtures, MIT notice and provenance live in the parser fork's
`tests/fixtures/gherkin43-tables`.

Phase-end `cargo test --test step_arguments` passed on its first run:
**73 scenarios / 241 steps**, including **13 new scenarios / 39 steps**.
[Parser evidence](evidence/phase15-tables-parser-bdd.log).
The native diagnostic is checked structurally, retaining original inconsistent
row values. Native fail-fast behavior remains; exact official multi-error
aggregation and diagnostic coordinates are outside this phase.

## Root acceptance

Ten `@phase15_sources` scenarios were authored before integration:

- Four shorter/longer step/Examples inputs report one parser error, run no handlers
  and create no successful snapshot.
- Valid step and Examples tables deliver exact values to real step handlers.
- Default application behavior retains excess-cell truncation for both kinds.
- Direct Basic parsing exposes typed inconsistent-cell-count diagnostics.
- Successful snapshots retain original source, table policy and independent
  legacy settings.

Root phase-end `cargo test --test sources -- --tags @phase15_sources` passed:
**10 scenarios / 38 steps**, exit 0. [Root evidence](evidence/phase15-tables-sources-bdd.log).
The first boundary attempt failed to compile because the harness referenced the
optional serde_json dependency in the default-feature target. The harness was
corrected to assert native row values directly, without changing dependencies or
production code. [First attempt](evidence/phase15-tables-sources-bdd-first-attempt.log).
No unit suites, generic tests, lint checks or unrelated builds ran in this phase.

Table whitespace, escaping, trailing content and empty table grammar are unchanged;
this is a bounded table-sizing capability, not complete Gherkin conformance.
