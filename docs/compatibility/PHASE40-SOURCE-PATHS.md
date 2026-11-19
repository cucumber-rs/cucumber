# Phase40: ordered source selection

Thirteen behavior scenarios authored before their implementation. Production and
real-run harness are complete; primary phase-end BDD passed13 scenarios/44 steps.
The shared32-target affected BDD boundary passed586 scenarios/2460 steps.

Pinned Cucumber.js13.3 paths.ts expands each user argument, sorts each expansion,
flattens in argument order and retains the first normalized path occurrence:
https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/paths/paths.ts

Provide an explicit typed SourcePaths builder containing literal paths/directories
and prevalidated glob patterns. Basic.with_source_paths selects this complete plan;
it overrides the single run input and legacy CLI glob intentionally. Without this
builder, existing resolution, CLI precedence and sorted directory behavior remain.
Canonicalize existing paths for deduplication; missing explicit paths remain native
reading errors (a documented useful divergence from JS glob's silent omission).
Each directory/glob expansion sorts paths before first-occurrence deduplication.
No automatic @file or file:line string mini-language: existing LineSelectors remain
the typed source-line contract. SourceFormat still controls directory patterns,
while explicit files/globs remain caller choices. No JS support file loading.

The plan reuses Basic source parsing, immutable capture, example expansion and line
selection. It feeds the existing whole-plan scheduling option, so actual World/body
entry order is asserted with serial Defined scheduling. Errors remain visible and
valid sources still execute. Empty explicit plan means empty selection.

This provides ordered multi-source programmatic integration without changing the
established CLI/build/release interface. Glob dialect remains globwalk; arbitrary
Node glob equivalence and symlink spelling equivalence are not claimed.

Source review identified a non-BMP filename ordering difference before the boundary.
The thirteenth scenario was authored before changing explicit-plan sorting to
UTF16 for valid Unicode paths. Legacy sorting and non-Unicode native path fallback
remain unchanged; glob syntax equivalence is still not asserted.

Actual primary evidence: [shared passing log](evidence/phase30c-37b-38b-40-42-second-bdd.log)
and [exact commands](evidence/phase30c-37b-38b-40-42-commands.txt).
The first boundary exposed a harness-only parser CLI type inference diagnostic;
its explicit type annotation was corrected. Production needed no runtime repair.
The initial diagnostic log is retained. No units or premature checks/builds ran.

Final minimum-toolchain boundary: this target also passed within the13-target
Rust1.88 acceptance (172 scenarios/670 steps total). See
[evidence](evidence/msrv-1.88-first-bdd.log) and
[exact command](evidence/msrv-1.88-command.txt). Earlier pending statements
above record prior checkpoints. Production needed no MSRV correction.
