# Phase37b: explicit candidate assignment and logical resource reservations

Status: implemented and verified at the coordinated phase-end BDD boundary:
**14 scenarios passed / 55 steps passed** in target `dynamic_assignment`, tag
`@phase37b`. The business acceptance cases preceded production. The shared
32-target affected BDD run also passed: **586 scenarios / 2,460 steps**, exit 0.

## Pinned behavior and bounded native adaptation

[Cucumber.js13.3 TestCasesPhase](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/runtime/parallel/test_cases_phase.ts)
selects the first queued candidate accepted against its running Pickles. With no
eligible candidate and active work it waits; with no active work it admits the
queue head and counts an idle intervention. It logs one aggregate warning when
work completes. Native Serial barriers remain mandatory even when a predicate
would accept other candidates; this is an intentional native scheduling contract.

The feature is opt-in within Phase37's existing buffered ordered dispatcher.
It does not create a second runner, change default native scheduling, allocate
worker identities, or alter the shared scenario-attempt implementation.

## Additive API

`AssignmentPolicy::new(name, predicate)` accepts a synchronous immutable
`Fn(&AssignmentCase, &[AssignmentCase]) -> bool`. It does not require Worlds,
async callbacks, JS support files, or worker/process identifiers. Captures may
hold application-owned static resource configuration. Native local callback
bounds should remain consistent with the existing classifier API.

`AssignmentCase` contains a stable logical case ID, actual Source<Feature>,
optional Source<Rule>, expanded Source<Scenario>, ScenarioType, and plan_index.
The index is fixed after the configured global permutation and before dynamic
queue removals. Logical identity stays fixed while retries get their usual fresh
native ScenarioId and World. The running slice follows admission order and
contains every currently reserved logical case, including retry delays.

`SchedulingOptions::with_assignment(policy)` activates the policy only in the
existing explicit scheduling path. `AssignmentPolicy::with_store(store)` and
`store()` share structured observations, consistent with existing lifecycle
stores. Cloning/type-changing builders preserve this configuration through the
already-retained SchedulingOptions rather than new Basic fields.

`AssignmentPolicy::on_warning(handler)` replaces the default stderr sink with
an application callback receiving a typed aggregate AssignmentWarning. The
explicit policy emits at most one aggregate warning on normal dispatcher
completion when idle interventions occurred. The same warning remains in the
store. No assignment policy means no new warning output. Panic/cancellation does
not fabricate a completed run or guarantee an aggregate completion warning.

## Dispatcher changes

Maintain admission-ordered logical reservations separately from FuturesUnordered.
Insert a reservation before polling its case future or evaluating another
candidate. Change the existing logical retry-loop future to return its logical
ID on completion; release only then. Never release in response to an intermediate
Scenario::Finished notification, which can precede a retry delay.

For each available capacity slot, scan only the concurrent queue prefix before
the first Serial entry. If Serial is at the front, consider it only when no
reservations exist, and run it alone. Never evaluate beyond a Serial barrier.
Choose the first predicate-eligible candidate. If none qualify while reservations
exist, await actual dispatcher progress. If all reservations are absent, admit
the accessible prefix's head as explicit IdleFallback; even fallback cannot
bypass or overlap Serial. Retrying logical cases retain their reservation and
bypass fresh assignment, preserving Phase37a's reserved-slot behavior.

Record only real evaluations, admissions, releases and transitions to waiting.
Waiting must await existing completion/notification futures, not poll the
predicate or append repeated idle records. A running-set revision or equivalent
waiting-state deduplication can avoid re-evaluating an unchanged blocked plan on
intermediate retry notifications. Global ordering is applied once before this
process, leaving unselected queue entries in their original relative order.

## Honest observations

`AssignmentDecision` contains run_id, monotonic per-run sequence, observed
timestamp, and a structured kind:

- Evaluated: actual candidate, ordered running snapshot, and returned eligibility.
- Admitted: actual case and reason Eligible or IdleFallback.
- Released: logical case whose entire retry-loop future completed.
- Waiting: NoEligible or SerialBarrier plus the actual running snapshot.
- Warning: aggregate actual idle intervention count and diagnostic message.

IDs identify dispatcher decisions and logical cases. They are not hook/body
invocation IDs or simulated worker IDs. No public native event enum changes or
synthetic Messages callback events are necessary. Completed actual case attempts
continue through the existing native/standard protocol path.

A predicate panic propagates as a programming error, following arbitrary native
classifier callback conventions. It is never translated into false, fallback or
successful completion. The policy should be a pure eligibility function of the
candidate, assigned cases and stable captured configuration; it is not an
external readiness notification mechanism.

## VLBV resource use

A programmatic VLBV application can associate
cases with browser/workspace resource keys, using explicit inherited tags such
as `@resource.browser-A` or a captured source-to-resource map. Reject a candidate
whose keys intersect any assigned case. Disjoint browser/workspace cases can
run concurrently; shared ones wait through all retry attempts and delay.
This schedules logical resource conflicts inside one runner. Idle fallback makes
it unsuitable for enforcing external leases or security isolation, and it does
not claim process-level isolation.

## Acceptance boundary

The14 authored cases cover resource exclusion and overlap, first-eligible
selection, actual waiting and bounded records, retained retry reservations,
queued/active/rejected Serial cases, fallback warning/default and custom sinks,
outline source/tag identity, builder retention, unchanged native defaults, and
panic propagation. Complete implementation and actual-execution harness before
its focused BDD phase-end run; no unit suites or premature checks.


## Actual phase-end BDD evidence

The [second coordinated boundary log](evidence/phase30c-37b-38b-40-42-second-bdd.log)
records all **14 scenarios / 55 steps passing** for `dynamic_assignment`. Actual
child application executions establish resource exclusion and independent overlap,
first-eligible selection, bounded waiting observations, reservations retained
through controlled retry delay, fresh retry World/attempt identities, Serial
barriers, default/custom fallback warning delivery, expanded source/tag identity,
builder retention, unchanged native defaults and original predicate-panic
propagation. No production or Phase37b harness corrections were needed at the
boundary. The first shared boundary stopped on Phase30c harness compiler errors
before Phase37b runtime execution; its diagnostics remain in the
[first log](evidence/phase30c-37b-38b-40-42-first-bdd.log).

All five primary targets in the second shared run passed **67 scenarios / 268
steps**, exit 0. The broader 32-target affected run passed **586 scenarios / 2,460 steps**,
exit 0, as confirmed by the coordinating parent. No unit suites, generic cargo test or premature build/check
were run. No new dependency fork, publication or deployment was needed.

The exact shared invocation is retained in the
[command record](evidence/phase30c-37b-38b-40-42-commands.txt).

Focused reproduction command (the retained result above came from the shared
multi-target boundary, not a separate invocation of this command):

```sh
cargo test --test dynamic_assignment --features output-messages -- --tags '@phase37b'
```

Final minimum-toolchain boundary: this target also passed within the13-target
Rust1.88 acceptance (172 scenarios/670 steps total). See
[evidence](evidence/msrv-1.88-first-bdd.log) and
[exact command](evidence/msrv-1.88-command.txt). Earlier pending statements
above record prior checkpoints. Production needed no MSRV correction.
