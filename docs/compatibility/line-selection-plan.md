# Phase34 source-aware selection contract

This phase adopts Cucumber.js13.3.0's source-location selection boundary:
scenario headers and Examples body rows select generated executions. Outline
headers select all their rows. Other lines do not select executions. See the
[pinned audit](FILTERING-SCHEDULING-AUDIT.md#exact-fileline-semantics).

The additive typed API is `filter::LineSelectors`, configured by repeatable
`lines(path, lines)` and `file(path)` operations. Lines must be positive; paths
must be nonempty. Empty selectors impose no restriction. Selected lines for one
file are ORed, and selecting that entire file supersedes its restricted lines.
Only sources already discovered by Basic are considered; an absent or undiscovered
selector is valid and never causes implicit discovery.

Paths become absolute relative to the configuration-time working directory.
Existing resolvable paths use filesystem canonicalization, so symlink and direct
aliases union. Otherwise lexical normalization preserves a missing selector as a
valid path. Basic applies the same identity normalization to discovered paths.
An alias created or retargeted after selector construction is outside this
configuration-time identity guarantee. No source bytes are reread for origins.

Basic owns the selection and aligns parser-owned origin records with its own
expansion order before filtering. `SourceSnapshot::scenario_origins()` exposes
Rule index, expanded ordinal, actual scenario declaration line and optional
Examples body-row line from that immutable generation. Plain table rows use
validated source table byte spans with a count check. Markdown uses retained
source Row positions, excluding separator rows. Synthetic native expanded
positions and textual matching are not source identity; identical rows remain
separate records.

`Basic::with_line_selectors` enables source capture internally when necessary.
`Cucumber::with_line_selectors` delegates to Basic and enables the Phase33
composable policy if absent: line selection AND name/tag filters AND caller
predicate. Existing typed filters remain configured. Default/legacy behavior is
unchanged. Replacing Basic intentionally replaces its parser-specific policies;
no public native AST or CLI struct fields change. Messages is not a dependency.
Raw captured sources and ASTs remain complete for reporting and retained records.

Acceptance was authored first in
`tests/features/line_selection/selection.feature`. Production implementation and
manual review are complete. The coordinated all-feature phase-end run passed
**38 scenarios / 178 steps**. String `file:line`
parsing, implicit input discovery, and generic custom-parser origin registration
are outside this bounded typed-API phase.


## Usage and acceptance boundary

```rust,ignore
use cucumber::filter::LineSelectors;

let selected = LineSelectors::new()
    .lines("features/accounts.feature", [12, 28])?
    .file("features/health.feature")?;
MyWorld::cucumber()
    .with_line_selectors(selected)
    .run("features")
    .await;
```

The harness contains 38 scenarios when `output-messages` is enabled. Its extra
Messages case selects the second of two otherwise identical Examples rows and
requires the test case to reference that original row while retaining all
source Pickles. A second focused run without reporting features selects the
other 37 scenarios, proving the core APIs do not require Messages. Both focused runs passed: **38 scenarios / 178 steps** with reporting enabled,
and **37 scenarios / 173 steps** without Messages.
No test, check, or oracle was executed while implementing this phase.


Primary evidence:
[`phase32b-34-35-third-bdd.log`](evidence/phase32b-34-35-third-bdd.log).
The first joint compilation stopped on an unconstrained custom CLI parameter in
this harness. An explicit `cli::Empty` annotation fixed that test wiring; no
production correction was needed for the 38 passing cases. The initial record
remains in
[`phase32b-34-35-first-bdd.log`](evidence/phase32b-34-35-first-bdd.log).
Other targets in the coordinated run have separate outcomes and are not included
in this phase's acceptance count.


Reporting-independent evidence:
[`phase34-without-messages-bdd.log`](evidence/phase34-without-messages-bdd.log).
The no-Messages build reported five reporter-only attachment dead-code warnings
for payload/record fields and store helpers. The focused BDD passed; this is not
a warning-free feature-off build claim. No selector production correction was
needed. No new dependency fork or runtime bridge was introduced.
