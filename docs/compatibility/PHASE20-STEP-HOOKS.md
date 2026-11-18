# Phase20: actual ordered per-step lifecycle

`runner::StepHooks<W>` installs through Basic/Cucumber `with_step_hooks`, without
replacing existing scenario hooks. All selected before-step callbacks execute in
registration order, including after failures. The resolved step body executes
only if setup passed. All selected after-step callbacks run in reverse order,
even after setup/body/cleanup failure. Each receives the same optional body result
(`StepBodyOutcome::NotRun/Passed/Failed`), not a result updated by earlier cleanup.
Undefined, ambiguous and unexecuted steps invoke no step hooks. Feature/Rule
backgrounds use the same lifecycle. Retries use fresh World and execution IDs.

These rules follow pinned Cucumber.js13.3 `test_case_runner.ts` lines280–365;
Context7 v13.3 docs were also consulted. The native event enum is unchanged:
aggregate body/hook failures are ordinary native step failures, so existing
writers and retry scheduling retain their contract. Step hooks do not generate
invented hook TestSteps (the JS runner also aggregates them into the step result).

An explicit `StepHookStore` records each callback actually invoked: stable
registration ID/name/kind, unique invocation ID, actual native Source values for
feature/Rule/scenario/step, ScenarioId, retry attempt, timestamps and exact panic
result. Filters produce no invocation record. Records append at completion;
concurrent start/completion order may differ. Use a fresh store per run. This
provides real identity for future reporting integrations without changing public
native events or pretending registry membership proves callback execution.

Feature-first acceptance is `tests/features/step_hooks/step_hooks.feature`.
Only after complete implementation:
`cargo test --test step_hooks -- --tags @phase20`.
First phase-end run passed **10 scenarios / 51 steps**, exit0, with no warnings
or corrections. Affected focused regressions also passed: existing scenario
hooks **9/42** and background/source execution **12/64** scenarios/steps.
Evidence: `evidence/phase20-step-hooks-bdd.log`,
`evidence/phase20-scenario-hooks-regression.log`, and
`evidence/phase20-backgrounds-regression.log`.
No generic suite, unit tests, lint/check or early build ran. Run-level hooks, timeouts, explicit pending/skipping,
and original Cucumber expression support-file execution remain separate contracts.

Primary source: https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/runtime/test_case_runner.ts
