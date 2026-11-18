# Proposed Phase38: explicit classic lexical semantics

This is a read-only source audit, not runtime failure evidence. No tests or
oracles were executed for this audit. Baseline is official Gherkin43.0.0,
commit `3d70e2bb4e0f16a568ee539a5ce0b03fb247a777`.

[GherkinLine](https://github.com/cucumber/gherkin/blob/3d70e2bb4e0f16a568ee539a5ce0b03fb247a777/javascript/src/GherkinLine.ts)
removes ECMAScript whitespace before recognizing any token. Its remainder helper
uses JavaScript trimming. Table-cell trimming deliberately uses a smaller set:
space, tab, vertical tab, form feed, carriage return, NEL and NBSP. That table
set already matches native `official_table_cells` and SourceGraph's
`table_whitespace`; there is no demonstrated trim-set gap there.

[The classic matcher](https://github.com/cucumber/gherkin/blob/3d70e2bb4e0f16a568ee539a5ce0b03fb247a777/javascript/src/GherkinClassicTokenMatcher.ts)
uses those prepared lines for headers, steps, tags, tables, comments and empty
lines. Tag boundaries and trailing comments also use ECMAScript whitespace.
Native `parser.rs` instead retains ASCII `_`/`__` rules and comment/directive
padding, Rust `trim_end` for values and Rust `is_whitespace` for tags. Rust's set
includes NEL and excludes BOM; ECMAScript differs. SourceGraph currently permits
broader indentation only for doc strings and uses Rust trimming elsewhere.
Those source differences establish candidate semantic mismatches, pending BDD.

## Minimal proposed implementation

Add `LexicalSemantics::{Legacy,Gherkin43}` independently of vocabulary and
argument policies. Preserve all legacy defaults. Record it in SourceSnapshot;
include it in the explicit latest-policy bundle and direct official compiler.
Consume whitespace on original source bytes rather than constructing a rewritten
source. Keep LF handling in the line grammar; other ECMAScript whitespace is
eligible for line padding. Use conditional title/step/tag trimming and matching
in parser rules, SourceGraph, featureless handling and language directives.

Native `env.position` currently reports byte columns. Pass the original source
as an internal PEG parameter so opt-in token columns count Unicode scalar values
while byte spans stay unchanged. Legacy locations remain as before. The source
origin extractor added in Phase34 must use the same selected padding predicate.

Acceptance should first cover NBSP, BOM and EM SPACE, with NEL as a contrasting
character; declarations, tags, comments/directives, steps, Examples tables,
featureless documents, native handler values and preserved byte spans. Compare
complete AST/Pickle graphs against the pinned oracle at phase end, and include
actual line selection plus legacy-preservation cases. No generic checks or unit
suites are proposed.

## Distinct follow-up location issue

[GherkinLine](https://github.com/cucumber/gherkin/blob/3d70e2bb4e0f16a568ee539a5ce0b03fb247a777/javascript/src/GherkinLine.ts)
iterates table cell columns in JavaScript string units, while
[countSymbols](https://github.com/cucumber/gherkin/blob/3d70e2bb4e0f16a568ee539a5ce0b03fb247a777/javascript/src/countSymbols.ts)
counts surrogate pairs as one symbol for other locations. Current SourceGraph
table columns count Rust characters. Astral table-cell locations therefore need
their own bounded differential coverage; this is separate from whitespace-set
agreement. Do not claim those columns fixed by a whitespace-only phase.

## Approved next-phase contract

The parent approved the bounded lexical-policy implementation after this audit.
Acceptance is authored first in `tests/features/lexical_semantics/lexical.feature`:
24 scenarios covering the exact whitespace distinctions, full graph differential
comparison, real native handlers, unchanged byte spans, source row selection,
featureless inputs, immutable policy generations and legacy behavior. Production
and harness changes remain held until the current Phase32b/34/35 boundary ends.
The table-cell trim set remains unchanged. Astral table-cell UTF16 location
coverage is explicitly tracked as follow-up Phase38b unless it can be included
as a small, separately accepted conversion change; it is not an assumed fix.


## Implementation checkpoint before phase-end acceptance

Implementation and the 24-scenario harness are complete. `GherkinEnv`, Basic and
Cucumber expose `with_lexical_semantics`; successful Feature and featureless
snapshots retain `lexical_semantics()`. The explicit latest bundle/direct compiler
opts in, while vocabulary-only `new_latest` and all legacy defaults retain their
previous lexical policy. Classic conversion and row extraction consume the
captured policy. Markdown source matching stays independent.

The maintained fork and portable vendor mirror are synchronized, including
updated checksums/provenance. The focused target is `lexical_semantics` with
`output-messages` enabled. It uses pinned Gherkin43 as a phase-end oracle and
Messages35 schema checks. Actual acceptance is pending the coordinated36/37/38
boundary. No preflight build, unit suite or oracle probe was run.


## Phase38 primary acceptance result

**25 scenarios / 110 steps passed**, exit0 with no compiler warnings in that
focused run. Evidence:
[`phase38-sixth-bdd.log`](evidence/phase38-sixth-bdd.log).
The new 25th scenario was authored before its implementation: it verifies
Gherkin43 lexical semantics with the Legacy dialect catalogue, with full graph
and real-handler assertions and explicit captured-policy checks. No original
oracle expectations were weakened or discarded.

The first actual lexical runtime boundary passed 14 of the original24 scenarios.
Nine complex Unicode-padding sources and the retained-generation scenario failed
because the Feature preamble left padding before its first tag line. That
production gap was corrected only for explicit lexical semantics. The first
repair used a PEG semantic action in a nonterminal position and failed to compile;
the predicate was moved into its own rule. Review then identified after-tag
padding still gated solely on vocabulary selection; the independent-policy
scenario preceded the minimal guard correction. Evidence is preserved in
[`phase36-37-38-fourth-bdd.log`](evidence/phase36-37-38-fourth-bdd.log) and
[`phase36-37-38-fifth-bdd.log`](evidence/phase36-37-38-fifth-bdd.log).

The maintained fork/vendor source and checksum manifest are synchronized.
Affected root parser/reporting and existing fork BDD regressions remain parent
coordinated. No generic test/check command or unit suite was run. Phase38b
astral table-cell UTF16 locations remain a separate, explicitly open gap.


Affected phase-end BDD has now passed:25 root targets, **408 scenarios / 1,791
steps**, recorded in
[`phase36-37-38-affected-bdd.log`](evidence/phase36-37-38-affected-bdd.log).
The maintained sibling parser's existing `cargo test --test step_arguments`
BDD harness also passed **73 scenarios / 241 steps**, recorded in
[`phase38-parser-affected-bdd.log`](evidence/phase38-parser-affected-bdd.log).
These are focused acceptance targets, not a unit-suite or generic-test claim.
