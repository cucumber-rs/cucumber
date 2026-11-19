# Phase30b: explicit outcomes across built-in report formats

Status: **18 scenarios / 54 steps passed**, warning-free at the completed
Phase30b/33 boundary. Acceptance was authored before writer changes; no tests,
checks, builds, or standalone consumer execution ran during implementation.

The existing nine-row consumer outline is now in
`tests/features/execution_control_formats/consumers.feature` (`@phase30b_formats`).
A second separately authored nine-row outline covers recovery after an actual
cleanup failure overrides pending and enables a successful retry. The formats
target therefore has eighteen acceptance scenarios. The native execution-control
feature retains its separate twenty core scenarios.
The formats harness runs real applications under strict and non-strict policies,
with pending/skipped bodies and pending/skipped scenario hooks. Each run has four
scenarios and three actual body calls despite two retries being available.

JSON retains exact lowercase `pending` and `skipped` statuses in step and hook
results. Authoritative metadata also disambiguates actual undefined and ambiguous
outcomes from compatibility event projections. Events without metadata retain
legacy JSON behavior.

JUnit buffers complete Event metadata and uses the final scenario Outcome to
classify each testcase. Intermediate attempts that will retry are omitted only
in opt-in mode; final attempts retain their own stdout and outcomes. Strict pending is a failure named Pending; non-strict
pending and explicit skip are skipped testcases. Captured Basic stdout receives
each buffered event's original outcome, so it does not mislabel pending as undefined
or lose the controlled hook status. Ordinary panic/error diagnostic details remain
available on failed cases. Legacy events retain their previous interpretation.

Libtest emits strict pending as failed and non-strict pending as ignored, with
an explicit Pending diagnostic; skipped stays ignored. These are honest format
projections because libtest has no pending result. Native Stats separately counts
pending steps, pending scenario stages, and final strict-pending scenarios. Those
counts are not added to failed_steps, skipped_steps, or hook_errors. A normalized
scenario counter snapshot is restored when the actual final outcome schedules a
retry, preventing earlier pending/failure counts from contaminating final success.
Per-attempt libtest events still preserve the actual projected results. Suite output
counts the projected failed/ignored invocation events, whereas pending_failures
counts terminal scenarios. The existing libtest started test_count represents
planned steps/parser errors; additional hook result events remain the existing
format convention. Opt-in ambiguity is not retry-eligible just because attempts
remain.

The nine consumers are Basic, Summarize, FailOnSkipped, Repeat, Normalize+Tee,
Messages+official Query, JSON, JUnit, and libtest. Assertions cover final failure
policy, separate pending counters, no unintended retries, metadata through
normalization/replay/tee, exact JSON statuses, JUnit failure/skipped classification,
and libtest failed/ignored classification. The recovered-retry outline separately
checks final native counters, JUnit final-only success, libtest final suite success,
exact JSON attempt history, and Query retaining two observed attempts with one
final passing case. Messages are consumed by the installed
pinned Query via the existing `tests/interop/consume.mjs`; this executes only within
the completed-phase BDD target. No new crate or fork is required.

Completed phase-end command, after parser33 and native execution-control primary
acceptance had passed and released Cargo:

```sh
cargo test --test execution_control_formats --features output-messages,output-json,output-junit,libtest -- --tags @phase30b_formats
```

The first attempt compiled production but found two unsupported harness
compositions: nested Repeat wrappers and direct Summarize wrapping JUnit's
non-Colored CLI. The harness now selects failed/skipped Repeat independently and
tees JUnit with a native Basic statistics observer. The second attempt required
an explicit application World type for Tee. No runtime scenario executed on
those attempts, and no production format correction was needed.

- [First compiler diagnostics](evidence/phase30b-formats-first-bdd.log)
- [Second compiler diagnostics](evidence/phase30b-formats-second-bdd.log)
- [Successful focused BDD](evidence/phase30b-formats-bdd.log)
- [Exact command](evidence/phase30b-formats-command.txt)

The third command invocation was warning-free and passed all **18 scenarios /
54 steps**, exit 0, on the first runtime execution. The shared Basic unused-wrapper
warning was corrected by its owner before this successful run. The joint affected
regression batch remains parent-coordinated; it is not claimed by this primary
acceptance record. No new dependency/fork, unit suite, generic cargo test, lint,
format check, deployment, or publication was introduced.
