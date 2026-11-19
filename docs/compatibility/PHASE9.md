# Phase 9: opt-in official doc-string content through execution

Applications can call
`World::cucumber().with_docstring_semantics(gherkin::DocStringSemantics::Gherkin43)`
to deliver the adopted official normalized content to their Rust step handlers.
`parser::Basic` exposes the same builder. Legacy remains the default; selecting a
dialect catalogue does not silently change content semantics. The public native
Step shape remains unchanged.

The parser policy is captured in immutable `SourceSnapshot::docstring_semantics`.
Snapshots retain exact original source bytes and the raw, unexpanded native AST
with its selected content representation. Outline substitution occurs afterward,
so runtime handlers receive expanded values while reporting retains templates.

## Parser prerequisite evidence

The existing GQAdonis/gherkin fork owns the implementation; parser/lib changes and
hashes were synced into `vendor/gherkin`. No additional dependency fork is needed.
The public parser API is `GherkinEnv::with_docstring_semantics` with
`DocStringSemantics::{Legacy, Gherkin43}` exported at the gherkin crate root.

The complete focused fork BDD ran at its phase boundary:
`cargo test --test step_arguments` — **60 scenarios / 202 steps passed**, exit 0.
[Parser evidence](evidence/phase9-docstrings-parser-bdd.log).
The 10 added cases assert 18 exact external official AST contents (eight LF,
eight actual CRLF and two combined-argument payloads), four legacy-constructor
outputs, and missing/mismatched/same-line-only closure rejection. Official source
and expected AST came from Gherkin v43.0.0 commit
`3d70e2bb4e0f16a568ee539a5ce0b03fb247a777`, with its MIT notice preserved.
The fork checkpoint `../gherkin/docs/gherkin43-docstrings.md` records details.

## Root integration contract

Three `@phase9_sources` scenarios were authored before implementation. Real
Cucumber handlers observe typed content without opening media text or synthetic
edge newlines, two substituted outline values, and unchanged legacy content.
Assertions require exact handler values and passed/failed/skipped/hook/parser
statistics. Snapshot assertions require original bytes, the selected content
policy, unchanged dialect policy and the unexpanded normalized template.
The nested application runner and outer acceptance runner both run serially.

Root phase-end command: `cargo test --test sources -- --tags @phase9_sources`.
Observed **1 feature, 3 scenarios passed, 13 steps passed**, exit 0 on 2026-11-17.
[Root integration evidence](evidence/phase9-docstrings-sources-bdd.log).
The first run passed without compiler warnings or implementation corrections.
Four inner application step executions passed (one typed payload, two outline
rows, one legacy payload), with no failed/skipped steps, hook errors or parser
errors. Both the exact delivered values and snapshot metadata were asserted.
No unit suite, generic cargo test, lint/check, or premature build ran.

## Deliberate limits

This is the adopted content subset, not complete Gherkin grammar conformance.
The parser supports the official fixtures' triple/backtick delimiters, optional
media header exclusion, indentation, interior blank lines, active-separator
unescaping, and both argument orders. It follows the pinned JavaScript token
matcher's first escaped-separator replacement per line; broader cross-language
multiple-escape differences and Unicode indentation remain unverified.
Media type is still absent from the native Step API. Messages reporting has its
separate source-aware representation; this phase does not modify its writer or
claim new report-consumer coverage. No publication, deployment or merge occurs.
