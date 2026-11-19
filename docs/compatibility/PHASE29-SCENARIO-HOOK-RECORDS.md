# Phase 29: observed scenario callbacks and evolving cleanup context

Status: implemented and phase-end verified on 2026-11-17: 18 scenarios / 76 steps
passed. Existing Hooks and legacy callback semantics remain default. Individual
attachment reporting was exercised with the coordinated Phase31 Messages writer;
its own broader protocol acceptance and consolidated regressions run separately.

## Pinned source findings

Cucumber.js 13.3 assembles a separate TestStep per selected Before/After callback.
Before order follows registration; After order reverses it. Each result joins the
completed-result list before the next callback. Failed setup skips later Before
callbacks, but their selected TestSteps still receive SKIPPED results. After
callbacks receive the current worst result, including earlier cleanup failures;
willBeRetried is recomputed. The separate error argument reflects the last
processed pickle step, not the worst result or preceding hook failure.

The pinned runner also has an empty-Pickle edge case: didWeRunStepsYet never
becomes true. Registered After callbacks therefore take the Before path, lack
result/error/willBeRetried, and can be skipped after an earlier failure. This is
source evidence, not a newly executed differential acceptance result. Preserve
native behavior by default; expose this behavior only through an explicit policy.

The Messages 35 worst-result helper orders FAILED above AMBIGUOUS, UNDEFINED,
PENDING, SKIPPED, PASSED and UNKNOWN. Stable ties retain the earliest worst result;
its duration is not a sum of every scenario step.

Sources:
- https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/runtime/test_case_runner.ts#L163-L278
- https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/assemble/assemble_test_cases.ts#L20-L83
- https://github.com/cucumber/messages/blob/v35.0.0/javascript/src/getWorstTestStepResult.ts

## Small additive API

The existing `runner::Hooks<W>` now provides:

- `with_store(ScenarioHookStore)`, `store()` and `definitions()` expose observation.
- `before_scoped(name, callback)` and `after_scoped(name, callback)` take a rich
  `ScenarioHookScope`; official tag-matching variants select inherited tags.
- Existing before/after/before_if/after_if/before_matching/after_matching signatures
  stay intact. Existing After callbacks keep their pre-teardown ScenarioFinished.
- `empty_scenario_policy(EmptyScenarioHooks::Native | CucumberJs13_3)` preserves
  native defaults and makes the pinned empty-Pickle behavior explicit.

Internally, registrations hold stable IDs, a Legacy/Scoped callback variant and
actual registration source captured with track_caller. Unknown tag-expression
source text is omitted from reporting rather than reconstructed as original text.

Scope includes native Source<Feature>/Rule/Scenario, ScenarioId, attempt, definition
and actual invocation ID. Scoped After receives the original native body snapshot
separately from the evolving prior-result summary and earlier hook failures. It
can decide whether cleanup failure warrants additional diagnostics. Preserve the
first equally severe diagnostic instead of replacing it with the newest failure.

Do not advertise the summary as a complete JS TestStepResult: native deferred and
unexecuted step handling does not expose every result or the exact last processed
pickle-step error. Do not invent raw JS error or aggregate duration. The implemented summary classifies the actual native body as Passed, Undefined,
Ambiguous or Failed, retains the first most-severe diagnostic, and reports
retry_if_finished_now using native policy (including its ambiguity behavior).
It is not a reconstructed list of all JS pickle-step results.

## Runner integration and compatibility

Basic retains the Hooks registry until execution instead of immediately erasing
it into two callbacks. `with_hooks` retains its current public return type, clears
legacy callback Options and installs the registry. Native helpers invoke selected
registered callbacks directly with runner-owned scope. No task-local registry,
callback-position guesses, or no-op marker callbacks are required.

Legacy `.before()` or `.after()` replaces only that stage and clears only that
stage in the retained registry. `.with_hooks()` replaces both stages. Clone and
all type-changing constructors preserve the other registry and attachment fields.
Existing public native event enums continue emitting aggregate stage outcomes, so
custom writers and legacy hook statistics stay compatible.

Store each selected slot's actual executor decision:

- Executed: actual invocation ID, start/completion timestamps and result.
- Skipped: decision timestamp and reason, without an invented callback invocation.

Excluded registrations have no selected slot. Selected slot IDs remain stable
across retries; ScenarioId, attempt and invocation IDs distinguish executions.
Store keys include native feature URI, Rule/scenario identity and retry attempt;
concurrent cases must not join through a shared mutable current-scenario pointer.

Typed attachments use a private ScenarioHook(definition_id) target only when
with_store explicitly enables observation. Every registry callback gets a fresh
scope with closed-handle enforcement; unobserved registries retain composite
BeforeScenario/AfterScenario targets for existing Messages integrations. Constructor evidence remains an
explicit separate case; do not silently attribute it to a callback that has not
started. Preserve existing composite attachment targets for legacy callbacks.

## Acceptance boundary and subsequent protocol phase

The scenarios were authored before production implementation. After the complete
runner/store/context/attachment phase and coordinated writer were ready, this
focused BDD command passed 18 scenarios / 76 steps:

```sh
cargo test --features output-messages --test scenario_hook_records -- --tags @phase29
```

Evidence: `docs/compatibility/evidence/phase29-scenario-hook-records-bdd.log`.
No warnings or runtime failures occurred. The first own acceptance invocation
found a harness constructor with incomplete generic arguments; the correction
uses normal type inference. Its compile-only log is retained as
`phase29-scenario-hook-records-first-compile.log`. The preceding coordinated28b
boundary also found two ScenarioId import paths, corrected before runtime tests.
The Phase31 protocol acceptance then exposed a real Rule-outline join mismatch:
source reporting retained original Rule child scenarios while native execution
held expanded children. Record/attachment lookup now shares the Messages stable
Rule-source comparison, excluding the changing child list while preserving Rule
header/background/span and scenario/attempt identity. Phase31's concurrent
Rule-outline case verified the correction; its full primary run passed 13
scenarios / 64 steps. The consolidated affected BDD batch passed 212 scenarios / 906 steps, including
this Phase29 target again at 18 / 76 after the correction. Related coverage passed:
legacy hooks 9 / 42, Messages hooks 10 / 70, attachments 9 / 36, retries 6 / 40,
global native hooks 14 / 62 and global Messages hooks 6 / 34, alongside the
applicable parser/source cases. Evidence:
`docs/compatibility/evidence/phase28b-29-31-affected-bdd.log`.

No unit suites, generic Cargo test/check/build, or pre-phase verification ran.

The coordinated Phase31 Messages implementation consumes the store to replace the composite
stage's report with individual selected hook TestSteps and actual results. Its separate acceptance covers SKIPPED selected callbacks, retries, official schema/Query references
and per-callback attachments. Keep composite reporting when no store is supplied;
never infer execution from registrations or manufacture extra callbacks. Native
summary counts remain composite for compatibility until explicitly changed.

Each observed registry setup attempt records actual native World initialization
separately, including when selection is empty or construction fails. Phase31 can
report this native operation without falsely labeling it a callback. Unobserved
registries discard their temporary context records when each attempt finishes.

Excluded scope: global attachments (JS13 global hook this exposes parameters),
timeouts, JS support-file execution, process workers, and unproven complete parity.
