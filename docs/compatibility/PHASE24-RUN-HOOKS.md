# Phase 24: global native setup and cleanup

Implemented and phase-end verified: 14 scenarios / 62 steps passed on 2026-11-17.

`runner::RunHooks` registers named callbacks around one native executor run.
Callbacks receive `RunHookScope` with actual run, invocation and registration
identity; they receive no scenario World. Applications capture shared typed
configuration explicitly. This is one native executor, not process-worker or
coordinator distribution.

All before-all callbacks execute in registration order, including after failure.
Scenarios execute only if every global setup callback passed. All after-all
callbacks execute in reverse registration order, even after failed setup or
steps. Callback construction and polling panics are caught independently. These
rules follow Cucumber.js 13.3 executor.ts and coordinator.ts; global failure
continuation was introduced in version 13.

An explicit `RunHookStore` records each actual callback with its registration,
run/invocation IDs, timestamps and result. Fresh stores are required for concurrent
independent runs. Sequential reuse retains separate run IDs and does not change
completed writer statistics.

`Cucumber::with_run_hooks` installs a `writer::RunHooks` statistics adapter
after selecting the ordinary writer. It counts global failures without adding
variants to the public native event enums or inventing scenario failures. This
ensures `run_and_exit` fails when setup prevents every scenario or cleanup alone
fails. Install the builder after selecting a custom writer; replacing the writer
later removes its adapter. A raw Basic runner exposes the shared store for custom
writers. Protocol reporting must explicitly consume that store; this adapter
itself emits no fabricated Messages envelopes.

The guaranteed cleanup applies to awaited normal runs and caught user-hook/step
failures. Dropping or cancelling an async runner future cannot synchronously run
async cleanup; no cancellation guarantee is claimed. Timeouts and global scoped
attachments are separate contracts.

Feature-first acceptance: `tests/features/run_hooks/run_hooks.feature`.
Executed phase-end command after complete integration:

```sh
cargo test --test run_hooks -- --tags @phase24
```

The contract includes ordering, setup gating, all-hook continuation, constructor
panics, empty input, retries, concurrent scenarios, isolated `run_and_exit`
processes, actual invocation identity and sequential reuse. All 14 scenarios
passed on the first phase-end run. Evidence:
`docs/compatibility/evidence/phase24-run-hooks-bdd.log`.

The macros-only build reported five existing dead-code warnings for the attachment
module whose internal payload readers are enabled by `output-messages`. No
global-hook compiler warnings or acceptance failures occurred.

Affected BDD regressions also passed:

```sh
cargo test --features output-messages --test step_hooks --test hooks --test messages_attachments -- --tags '@phase20 or @phase3 or @phase23'
```

Scenario hooks: 9 scenarios / 42 steps; typed attachments: 9 / 36; step hooks:
10 / 51. The combined output-messages build had no warnings. Evidence:
`docs/compatibility/evidence/phase24-hooks-attachments-regression.log`.

Primary sources:
- https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/runtime/executor.ts
- https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/runtime/coordinator.ts
- https://github.com/cucumber/cucumber-js/blob/v13.3.0/UPGRADING.md
