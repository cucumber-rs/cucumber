# Phase19: complete official plain-text corpus adoption

The pinned Gherkin43 corpus is inventoried under
`tests/fixtures/gherkin43-corpus/manifest.json`:50 good inputs,12 bad inputs,
175 source and normative expectation artifacts. Upstream CRLF checkout rules and
MIT license are preserved. Markdown grammar is a separate contract.

Two additive APIs support reusable document/graph compilation:

- `gherkin::Document::parse(source, environment)` returns a document with an
  optional native Feature, allowing empty/comment-only sources. Existing
  `Feature::parse` still requires a Feature and keeps legacy defaults.
- `writer::messages::source::SourceGraph::parse_gherkin43(uri, source, language)`
  selects the adopted latest catalogue, normalized doc strings, official strict
  table rows, and official outline expansion. It produces no fake Feature or
  Pickles for featureless input. Existing shared-snapshot conversion retains
  its snapshot-selected policies. This is an explicit mode, not a blanket claim
  of all language/runtime parity.

Source-based fixes included before the assessment boundary address the official
extra-table-content and prefixed-keyword fixtures: explicit official table mode
ignores text outside closed pipe-delimited cells; latest keyword selection
prefers the longest step keyword across kinds. Source graph tags handle adjacent
@-separated tags and whitespace-prefixed comments. Legacy Feature/table/vocabulary
policies remain independent.

Scenarios were authored for every input before implementation. The completed
assessment harness compares every good AST/Pickle field and all reference
relationships to the unchanged official expectation, normalizing only URI and
bijective IDs. Every graph envelope validates against Messages34.2.1. Each bad
fixture must fail native Document parsing; exact diagnostic wording/count/location
is not claimed. Full bounded corpus results and concrete gaps are recorded only
from phase-end BDD, never from an early parser/oracle preflight.

```sh
cargo test --features output-messages --test messages_corpus -- --tags @phase19
```

## Observed phase-end outcome

The full corpus passed **62 scenarios / 236 steps**: all50 good source graphs
matched complete normative AST/Pickles and Messages34 schema, and all12 bad
sources were rejected natively. [Final evidence](evidence/phase19-corpus-bdd.log).
The prior focused fork BDD passed **73 scenarios / 241 steps**, covering existing
argument ordering/escapes, dialects, doc strings, strict tables and legacy
compatibility. [Regression evidence](evidence/phase19-parser-regression.log).

The first completed-harness corpus run passed60/62. Only the official space/tab
preamble fixtures failed native parsing. A latest-mode Feature-padding correction
resolved them; the two focused fixtures passed2/8 before the final full run.
[Initial corpus](evidence/phase19-corpus-first-bdd.log) and
[focused correction](evidence/phase19-corpus-padding-bdd.log) are retained.
Correction attempts also exposed PEG conditional-action placement and mutually
exclusive name/tag CLI filters; both were corrected with their diagnostics saved
as `phase19-padding-initial.log` and `phase19-padding-cli.log`.

The corpus builds also printed32 missing-documentation warnings from the parallel
step-hook phase; the parser regression compiled without warnings. No generic/unit
suite, early parser/oracle preflight, publication, merge or deployment ran.
Passing this complete pinned plaintext corpus establishes this finite compiler
contract; it does not prove arbitrary Gherkin inputs, exact error diagnostics,
Markdown, every dialect, or full Cucumber.js runtime parity.
