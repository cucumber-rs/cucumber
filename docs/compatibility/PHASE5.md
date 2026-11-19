# Phase 5: native async scheduling acceptance

This bounded phase assesses the existing Basic runner. It does not introduce a
scheduler redesign or claim Cucumber.js worker-process isolation. The scenarios
were authored before the acceptance harness. Execution began only after the
complete bounded harness and Cargo registration were ready. The adopted behavior
passed without any production scheduler change.

## Adopted behavior

- Positive concurrency limits 1, 2 and 3 bound active application scenarios.
- Limit 2 actually allows overlap; cohort barriers make sequential execution
  unable to pass. Unlimited (`None`) permits all six adopted scenarios to overlap.
- Every scenario starts with a fresh World and retains its own ledger across
  overlap; shared observation state is explicitly injected by the harness.
- `@serial` at scenario, rule or feature scope excludes other scenario work.
- Fail-fast at concurrency 1 executes one failing scenario and leaves the other
  two queued scenarios unstarted, while teardown completes.

The harness counts active scenarios from before-hook entry through after-hook
completion, records every identity and exact per-World values, and checks inner
runner scenario/step/error statistics. Tokio barriers and cooperative yields
coordinate execution; there are no sleeps or elapsed-time success assertions.
A 15-second timeout is only a deadlock failure guard. Outer acceptance scenarios
run one at a time to keep nested runner observation isolated.

## Explicitly deferred

No coverage is claimed for zero limits, CLI overrides, retry scheduling,
cancellation, fairness/order guarantees, late streamed features, process-worker
isolation, shared application resources, or cross-process lifecycle. The normal
positive-limit cases and native World isolation are narrower than JS parallel
worker parity. Timeout expiry is a failed acceptance observation, not permission
to alter scenario semantics or retry around a flaky result.

## Phase-end evidence

Completed 2026-11-17, focused command:

```sh
cargo test --test scheduling -- --tags @phase5
```

Observed result: **1 feature, 8 scenarios passed, 50 steps passed; exit 0**.
No skipped outer acceptance steps. [Passing log](evidence/phase5-bdd.log).

The seven successful scheduling cases each observed six passing inner application
scenarios with exact isolated World values (42 total). The fail-fast case
observed exactly one expected failed inner scenario/step, one completed teardown,
and two scenarios that never started. Its outer acceptance checks the actual
failure message and exact statistics, so expected failure is not silently ignored.

The first phase-end attempt failed compilation of the harness because the step
function named `limit` was shadowed by its same-named capture argument in macro
expansion. Renaming the function to `configure_limit` corrected only that
harness naming issue. [Original diagnostic](evidence/phase5-first-attempt.log).
The same focused BDD command then passed. No production scheduler change was
needed and no claim is made about the deferred cases above.

Cargo compiled newly introduced shared dev dependencies while building this
acceptance target; it did not execute their suites. No unit suite, generic
cargo test, lint/check, or premature build ran. No remaining execution blocker.
