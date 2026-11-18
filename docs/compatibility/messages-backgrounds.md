# Phase 16: empty scenarios and background execution

`runner::BackgroundSemantics::{Legacy,Gherkin43}` makes background execution an
explicit runner policy. `Legacy` remains the default and runs feature and Rule
backgrounds even when the selected scenario contains no own steps. Opt in with:

```rust,ignore
MyWorld::cucumber()
    .with_background_semantics(cucumber::runner::BackgroundSemantics::Gherkin43)
    .with_messages(output)
    .run("features")
    .await;
```

The Gherkin43 option skips both background levels only when the scenario has no
own steps, including each expanded empty outline row. Before/after scenario
hooks still run. Nonempty scenarios keep their feature-background, Rule-background,
then body order. Basic runner clones, scheduling builders, single-hook builders,
and composed-hook installation preserve the policy. Parser/dialect/content
settings remain independent.

SourceGraph always compiles the official representation: an empty scenario or
empty outline row produces a Pickle with no steps, retaining original background
nodes in the complete source AST. Direct snapshot conversion requires no runner.
A source-backed Messages writer paired with legacy background execution rejects
the unexpected background event visibly; it cannot emit a successful official
empty-scenario run. The original restricted reread-based Messages constructors
continue rejecting empty scenarios with backgrounds before execution reporting.

## Source and acceptance

The baseline is official Gherkin43's
[Pickle compiler](https://github.com/cucumber/gherkin/blob/v43.0.0/javascript/src/pickles/compile.ts),
which omits backgrounds for both empty plain scenarios and empty outline rows.
The unchanged official `incomplete_scenario.feature` is adopted under
`tests/fixtures/gherkin43` with source provenance and the existing MIT notice.
The separate exact-version oracle in `tests/interop/gherkin43` compares entire
AST/Pickle graphs at phase-end BDD, normalizing only IDs and URI.

Acceptance is authored first in `tests/features/messages_backgrounds` and covers
feature/Rule backgrounds, empty plain/outline scenarios, a mixed feature with
nonempty scenarios, no-background scenarios, actual callback/handler traces,
runner builder/clone propagation, native legacy summary results, visible
incompatible-runtime rejection, and source-only conversion.

```sh
cargo test --features output-messages --test messages_backgrounds -- --tags @phase16
```

Phase-end acceptance passed **12 scenarios / 64 steps**; see
[evidence/phase16-backgrounds-bdd.log](evidence/phase16-backgrounds-bdd.log).
The initial harness compile needed an explicit hook World type. Its first runtime
run passed11/12; the source-only fixture then needed the parsed canonical source
path for snapshot lookup. Both harness failures are retained in
`evidence/phase16-backgrounds-initial.log` and
`evidence/phase16-backgrounds-first-run.log`. No production correction was needed
after the boundary.

Affected focused prior BDD also passed: hooks9/42, scheduling8/50, plain Messages
20/97, and rich/localized source graphs19/80. See
[evidence/phase16-backgrounds-regression.log](evidence/phase16-backgrounds-regression.log).
No generic/unit suite or premature checks ran. This remains a bounded
execution/compiler contract, not general lifecycle or grammar parity.
