# Phase 37a design: global ordering and retained retry slots

Status: Phase37a passed its completed-phase primary BDD: **19 native scenarios /
81 steps**, plus **2 ordered Messages/tracing scenarios / 16 steps**. Total:
**21 scenarios / 97 steps passed**, warning-free. The first exact pinned shuffle
oracle execution occurred inside this boundary. Affected regression coverage is
still pending; this is not full scheduling parity. Dynamic assignment remains
separate Phase37b with no implementation authorized here.

## Accepted scope and public contract

Add opt-in `SchedulingOptions` to Basic/Cucumber, leaving absence of options on the
existing scheduler. Proposed order is `ScenarioOrder::{Defined, Reverse,
Random { seed: String }}`. Seeded random requires an explicit string in this bounded
phase. An empty explicit seed is a deterministic string seed; a future CLI parser
must distinguish it from JS `random:` (which requests a generated/reported seed).
Do not advertise the latter CLI spelling until that behavior is implemented.

A native positive concurrency constructor accepts `NonZeroUsize`; a separately
named `parallel(usize)` compatibility builder maps JS zero to one active case.
Positive values retain their meaning. Native `--concurrency 0` must be a Clap
configuration error. Existing native builder `max_concurrent_scenarios(Some(0))`
must fail clearly before dispatch rather than stall; a documented assertion at the
setter/resolution boundary is the minimal compatible-signature correction. The
existing `None` unlimited setting and positive native values remain unchanged.
Explicit native CLI concurrency, when present, overrides the configured limit;
its zero never passes through the JS-zero conversion. This resolves configuration
precedence without changing the CLI default for legacy callers.

Defined order means feature order from the supplied parser stream, then source
order within each feature, including Rules and expanded Examples rows. Reuse the
existing expansion order for rows sharing their original outline position. Filter
first, then flatten and permute once across the whole selected application.
Native Basic currently globally sorts discovered paths, whereas JS preserves the
user's sequence of path arguments and sorts each expansion. This discovery gap is
explicitly deferred: identical permutation means identical initial case sequence.

The guarantee is actual dispatch order, not completion order, worker processes or
thread isolation. Concurrency remains native asynchronous futures. Admission must
be observed at an actual case execution boundary rather than inferred from
normalized report order. Cases admitted together can complete in any order.

## Queue and parser lifecycle

The opt-in path collects the entire selected stream before dispatching any World
or application callback, including global setup hooks. Preserve parse errors and
ParsingFinished, original Source identities, and source counts. No-fail-fast input
with errors still executes its valid selected cases and reports the errors. Do not
silently invent a different fail-fast parse-error policy; carry forward the
runner's existing policy and add focused coverage if its implementation changes.

Use a real completion signal (for example a one-shot channel) from ingestion to
ordered execution. An uncontended async mutex is not a readiness notification.
Today's executor can repeatedly continue with no running cases, no retry timer and
unfinished input; its lock futures may be immediately Ready and starve a delayed
producer in the same task. This is source evidence, not a runtime-reproduced bug.
The new path must await source completion and must not copy that polling loop.

The completed plan is one ordered queue, not separate Serial and Concurrent
queues. Track active logical cases. A Serial entry is a barrier: drain all earlier
active cases, run it alone, then admit later entries. Do not bypass or prepend it.
Buffering prevents late arrivals but does not itself enforce this exclusivity.

## Retries: minimum refactor to coordinate with lifecycle

Current `Executor::run_scenario` computes `next_try`, inserts it into shared
Features, then sends `scenario_finished(..., retried)`. Factor attempt execution
from retry placement with the smallest private return value/seam:

1. Execute one full attempt, emit its ordinary events, and return next RetryOptions.
2. Legacy caller inserts the retry in Features as before, retaining the original
   ordering of insertion and finish notification where observable.
3. Ordered caller loops within one logical-case future, awaits the retry delay,
   allocates a fresh ScenarioId and World, and starts the next attempt. It keeps
   its capacity reservation (and Serial exclusivity) throughout the delay.
4. Each attempt still emits native Finished, attachment/hook identities and honest
   retry metadata. Intermediate finish notifications carry retried=true; only the
   terminal attempt increments feature/rule completion counts and releases a slot.
5. Fail fast considers a terminal exhausted failure, not an intermediate retry.

Coordinate this seam with lifecycle after Phase36; no concurrent Basic edits.
Tracing currently registers scenario spans from runnable batches. The ordered
wrapper must register/unregister each fresh attempt too; retaining only the first
attempt in TracingCollector would misattribute logs. Cleanup and finish notices
must be drained while an ordered case awaits a delay, not only after its complete
logical future resolves. Preserve output Messages attempt IDs, Stats retry counts,
attachment scope closure and hook observation identity across every attempt.

An injected runtime-independent delay seam should support deterministic BDD gates.
Do not use wall-clock sleeps to prove ordering. Reuse an existing timer abstraction
where its semantics fit, without changing timeout policy itself.

## Exact pinned shuffle and attribution

Cucumber.js v13.3.0 uses `knuth-shuffle-seeded` ^1.0.6. Pin the comparison oracle to
1.0.6 and its `seed-random` dependency to 2.2.0, and retain their licenses/provenance
(Apache-2.0 for the shuffle; MIT modifications and BSD original for seed-random).
The former performs descending Fisher-Yates, including the final length-one draw.
The latter mixes string UTF-16 code units into ARC4, discards 256 initial outputs,
and generates IEEE-754 doubles using its documented numerator/denominator loops.
Port those deterministic string-seed semantics exactly. A numeric seed, UTF-8
byte mixing, StdRng or another Fisher-Yates random-index generator is insufficient.
No dependency fork is needed for this local attributed compatibility helper.

The focused BDD will invoke the independent pinned JavaScript package only at the
completed phase boundary, with ASCII, zero-string and non-BMP seeds and the actual
selected native case sequence. Do not precompute an oracle by running code before
that boundary. Exact package provenance and licenses must be recorded before use.

Primary sources:

- [filter then global order](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/filter/filter_plugin.ts#L13)
- [defined, reverse and seed handling](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/filter/order_pickles.ts#L11)
- [per-argument discovery order](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/paths/paths.ts#L44)
- [parallel zero chooses in-process execution](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/runtime/make_runtime.ts#L72)
- [all retry attempts remain in one case execution](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/runtime/test_case_runner.ts#L203)
- [shuffle implementation and Apache-2.0 notice](https://github.com/TimothyGu/knuth-shuffle-seeded/blob/v1.0.6/index.js#L1)
- [shuffle dependency pin](https://github.com/TimothyGu/knuth-shuffle-seeded/blob/v1.0.6/package.json)
- [ARC4 double generation](https://github.com/ForbesLindesay/seed-random/blob/2.2.0/index.js#L35)
- [UTF-16 seed mixing](https://github.com/ForbesLindesay/seed-random/blob/2.2.0/index.js#L134)

## Acceptance and deferred scope

Feature-first contract: `tests/features/ordered_scheduling/order.feature`, tag
`@phase37a`. It defines 19 expanded cases: global defined/reverse, three pinned
seed permutations, filter-before-order, gated parser completion, parse errors,
capacity overlap, Serial barrier, three retained-slot retry cases, fail fast,
JS-zero serial, native CLI/builder zero rejection, empty input and legacy behavior.

The harness will use real native executions and capture actual admission/body
observations, terminal stats and source/attempt identities. A watchdog bounds
broken progress cases; it is not timing evidence. The `ordered_scheduling` harness=false target and tracked oracle fixtures are
registered in Cargo. At full phase completion run only the focused ordered-scheduling
BDD and then the specifically agreed affected scheduling/retry/tracing/attachment
BDD targets; preserve logs and exact scenario/step counts.

Phase37b may add a candidate/running-set predicate with first-eligible selection
and JS's idle fallback warning. It must keep native Serial barriers mandatory.
It is not part of the current feature, API activation or acceptance counts.
[JS assignment and idle fallback](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/runtime/parallel/test_cases_phase.ts#L49).

Also deferred: generated random seeds and replay reporting, JS path-argument
expansion parity, process isolation and worker IDs. No full scheduling parity claim
is justified until these boundaries and the adopted cases are covered.

## Implementation and remaining verification boundaries

`runner/scheduling.rs` exposes SchedulingOptions and ScenarioOrder; Basic accepts
with_scheduling_options and Cucumber exposes scheduling_options. The default path
remains streaming/legacy. Native zero is rejected by the CLI value parser and by
the existing builder assertion. Explicit parallel(0) selects one logical slot.

The completed plan uses a one-shot ingestion notification and a single VecDeque.
Serial entries wait for all active logical futures and remain exclusive through
retries and delay. RetryDispatch::{Queue,ReturnToSlot} shares the same complete
attempt function. The ordered wrapper renews native ScenarioId/World and registers
each trace attempt. The dispatch loop also listens to per-attempt finish notices
while logical futures remain active, preserving feature/rule accounting and
removing completed trace registrations during retry delay.

The independently acquired npm oracle lives in tests/fixtures/shuffle37 with
original licenses, unchanged package files, registry-integrity verification and
SHA256 provenance. Its first execution occurred inside the completed-phase BDD reported below.
No new dependency fork or runtime JavaScript dependency was introduced.

## Added consumer/tracing boundary

Two feature-first `@phase37a_messages` scenarios extend the existing real-process
`messages_logs` BDD harness: ordered recovering/exhausted cases, with and without
an injected yielding retry delay. They require exact native case/attempt log
attachments, schema validity, independent Query resolution of all four attempts
and final outcomes, and the actual configured retry-clock invocation count.
Query16 offers only step/run-hook attachment lookup, so the oracle resolves case
identities through Query and explicitly joins case-scoped logs by their observed
case ID; it does not invent step association or claim a nonexistent Query API.
These21 total phase cases are authored/implemented but have not run yet.

## Actual completed-phase evidence

[phase36-37-primary-bdd.log](evidence/phase36-37-primary-bdd.log) records:

| Target | Scenarios | Steps | Result |
| --- | ---: | ---: | --- |
| ordered_scheduling | 19 | 81 | All passed |
| messages_logs, tag phase37a_messages | 2 | 16 | All passed |
| Phase37 total | 21 | 97 | All passed |

The native target proves actual callback admission/body observations, complete
cross-source permutation, filter-before-shuffle, independent pinned string-seed
oracle equality (including non-BMP input), repeatability, gated asynchronous
source completion, serial barriers, retained retry reservations and delays, fresh
World/ScenarioId per attempt, stable Source identities, terminal fail-fast, zero
configuration behavior and the established legacy fixture.

The added consumer target proves ordered retry outcomes through Messages and
the independent official Query consumer, plus tracing log attribution to fresh
attempts. Messages may normalize its envelope order by feature/case; no claim is
made that its physical NDJSON sequence equals global native dispatch order.

The combined process later failed in the separately owned Phase36 retry-policy
target (8 of 16 scenarios failed). Its original evidence remains in the same log.
These are target-specific Phase37 passes, not a claim that the full batch passed.
Affected regressions and diagnosis of that shared-boundary failure remain pending.

Earlier shared boundary evidence is retained without replacing failures:

- [first compile boundary](evidence/phase36-37-38-first-bdd.log): harness Hooks
  World inference, missing recorder Stats adapter, and a completion-send Copy
  warning; corrected by explicit Application annotation, discard_stats_writes and
  let-underscore handling of the send result.
- [second compile boundary](evidence/phase36-37-38-second-bdd.log): explicit
  Application annotation also needed on the independent StepHooks recorder.
- [third compile boundary](evidence/phase36-37-38-third-bdd.log): initial Cucumber
  input type needed PathBuf before replacement with the custom parser.
- Fourth/fifth shared attempts stopped in the lexical/Gherkin target before
  scheduling acceptance. Their logs are retained by the parent phase record.

No scheduling runtime correction was needed after its acceptance scenarios began.
No unit suites, generic cargo test, premature checks, new dependency forks, pushes
or deployments were performed for this phase.
