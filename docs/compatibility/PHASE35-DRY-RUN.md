# Phase35: explicit dry-run inspection

Status: core **14 scenarios / 59 steps passed** at the completed shared phase
boundary on2026-11-17, with tracing and all relevant output features enabled.
The coordinated consumer target separately passed9 scenarios /30 steps. The
parent subsequently completed16 affected BDD targets:267 scenarios /1115 steps
passed, exit0 (`evidence/phase32b-34-35-affected-bdd.log`).
Default execution and configured outcome policy remain independent of the
explicit ExecutionMode::DryRun setting.

Evidence: `evidence/phase32b-34-35-third-bdd.log`. The prior boundary retained
`evidence/phase32b-34-35-second-bdd.log`:10 core scenarios passed and four strong
protocol assertions failed because the manually assembled harness omitted the
shared support-code store. The correction wired the existing actual runner store
to the custom Messages reporter; no production behavior or identity assertion
was weakened. The first shared boundary had only unrelated harness compilation
corrections. No tests/checks/builds ran during implementation; only completed
phase-end BDD targets were invoked.

Pinned source: Cucumber.js13.3.0 [test-case runner](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/runtime/test_case_runner.ts),
[executor](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/runtime/executor.ts),
and [failure policy](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/runtime/helpers.ts).
The constructor still runs. Dry-run suppresses scenario/global/step callbacks and
bodies, classifies all definitions (recognized SKIPPED, missing UNDEFINED,
multiple AMBIGUOUS), never retries, and does not fail from those classifications.
An empty attempt is SKIPPED. Global dry-run emits started/finished envelopes with
SKIPPED and zero duration; those are decisions, not actual callback invocations.

## Native contract

`execution::ExecutionMode::{Run,DryRun}` defaults to Run. Basic exposes
`with_execution_mode`; Cucumber exposes `execution_mode`. Mode survives clones
and type-changing builders independently of OutcomePolicy. Dry-run terminal
metadata overrides classification failure/retry flags; returning to Run restores
the previously configured outcome policy. Definitions are resolved by the same
native collection and one shared step iterator, without argument conversion.

World is initialized once per selected scenario, including empty or unresolved
scenarios, with no active attachment or completion scope during construction.
Unlike a callback, constructor failure is a real initialization error: it fails
inspection, is never retried, and suppresses every application callback. Existing
native diagnostic carriers and observed native initialization records identify
this as initialization, not an executed user hook. This is a documented native
recovery boundary; JS construction throws outside its test-case result policy.

ScenarioHookStore preserves selected slots with Skipped{DryRun}; excluded slots
remain absent and no invocation IDs are manufactured. RunHookStore retains
separate RunHookSkip decisions, preserving its existing actual invocation records.
StepHookStore receives no records because no step callbacks are invoked. Native
composite stages use the existing nonpassing Hook projection plus exact SKIPPED
metadata. Started/finished classification pairs share a decision timestamp,
yielding a truthful zero duration. Successful World construction may retain an
observed native initialization slot; it is explicitly a native extension.

## Phase boundary

The core dry_run target covers14 cases: four definition classifications, observed
and legacy hooks, global decisions, empty scenarios, constructor capabilities and
failures, selected World creation, builder preservation, restoration, and defaults.
The same core target exercises parent-integrated Messages/Stats through schemas and official Query. No generic suites,
unit tests or intermediate checks are authorized.
