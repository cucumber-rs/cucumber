# Phase31: individual scenario-hook Messages

Status: focused phase-end acceptance passed **13 scenarios / 64 steps**.
Implemented against Phase29 actual observation API. Thirteen scenarios are authored in
`tests/features/messages_scenario_hooks/individual.feature` before implementation.

Consume explicit ScenarioHookStore observations from the same registry/runner.
Replace only observed registry stages with selected callback TestSteps; retain
legacy composite fallback. Definitions use real registration identity/name/source.
Selected skipped slots get protocol SKIPPED with zero duration at the actual
selection decision, without claiming a callback invocation. Executed callbacks
preserve actual times, statuses and messages. Excluded callbacks have no slot.

At case flush, assemble stable selected definitions before emitting TestCase,
replace composite lifecycle groups with callback lifecycle, then bind typed
attachments to the real definition target. Match source scenario/Rule/attempt
identity, not registration order guesses or a mutable current-case pointer.
Retries keep stable TestCase slots; invocations and outcomes remain per attempt.
Unknown/mismatched records fail visibly. Constructor evidence must retain its
actual separate native World-initialization boundary rather than being attributed
to a callback. An explicit stage plan distinguishes zero selected callbacks from
legacy replacement or missing observations. Every observed registry World initialization has its own TestStep with actual
timing/result, even on success without evidence, keeping the TestCase shape stable
across retries. This explicit native lifecycle extension is not a registered
callback and does not claim identical JS World-constructor behavior. Constructor
failures are never attributed to callbacks that did not execute. Without an explicitly supplied observation store, existing registry
attachments retain composite targets and reporting compatibility.

Pinned JS13 creates individual scenario-hook TestSteps, but aggregates step hooks
into the ordinary pickle-step result. Do not invent individual step-hook protocol
TestSteps: https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/runtime/test_case_runner.ts
and https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/assemble/assemble_test_cases.ts.

At the completed phase boundary, run only the focused BDD plus necessary affected
hook/attachment/retry regressions. Validate full official schema and actual Query
navigation. No unit suites, generic Cargo test or early compile/check.

## Actual phase-end evidence

Command: `cargo test --features output-messages --test messages_scenario_hooks -- --tags @phase31`.
[Passing evidence](evidence/phase31-scenario-hook-messages-bdd.log).
The first compilation stopped on a harness-only constructor: manually implemented
Worlds use `Cucumber::new()` instead of the inventory-dependent `World::cucumber()`.
[Initial compilation](evidence/phase31-scenario-hook-messages-first.log) is retained.

The next boundary passed12/13 scenarios and found a real concurrent Rule-outline
identity bug. Native Rules contain expanded child scenarios, while source plans
retain original outline children. Exact Rule equality prevented hook-plan and
attachment lookup. Both stores now join stable Rule source identity (header,
tags, background, position/span), preserving the actual scenario and retry identity.
Messages uses the same helper. The [failing evidence](evidence/phase31-concurrent-rule-failure.log)
and successful rerun are retained; the concurrent row attachment case now passes.

Affected shared parser/source/hook/attachment/retry BDD passed **212 scenarios /
906 steps** (13 named targets), recorded separately in
`evidence/phase28b-29-31-affected-bdd.log` after all primary phases completed. No
unit suite or generic Cargo test ran. Only new phase31 files were formatted;
stable rustfmt warned that existing repository nightly-only options were ignored.
No source publication or additional dependency fork occurred.
