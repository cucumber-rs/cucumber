# Phase41: generated ordering seeds and replay

Status: implemented and verified at the completed phase-end BDD boundary.
Target `generated_ordering`, tag `@phase41`: **10 scenarios / 45 steps passed**
on its first run, exit 0. All five affected BDD targets also passed: **69 scenarios
/ 282 steps**, exit 0. Production remains frozen for the final Rust 1.88 boundary.

## Pinned contract

[Cucumber.js13.3 order_pickles](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/filter/order_pickles.ts)
realizes a decimal seed in 0..999999 when the split random-order seed is empty,
including its `random:` spelling, then reports it. This was re-read from the
pinned source before implementation. The existing native explicit string-seed shuffle already ports
the pinned knuth-shuffle-seeded/seed-random permutation. This phase adds only
seed realization and observation at that same global permutation seam.

## Small additive API

Add `ScenarioOrder::RandomGenerated`. Preserve `Random { seed }`, including the
explicit empty string, and all Defined/Reverse/native behavior. No implicit
randomization is introduced by calling the default constructor. Keeping an
explicit native empty-string seed deterministic is an intentional existing API
distinction from JS order-string parsing; RandomGenerated supplies the generated
behavior without changing that native interpretation.

A shared `OrderingStore` exposes one actual `OrderingRecord` per completed
permutation. Its fields are run ID, requested order, realized seed when
random, selected case count, and observed timestamp. Run IDs distinguish
sequential reuse; random seeds are not identities and may legitimately repeat.
The record contains actual chosen data, not a fabricated Messages property or
callback/worker invocation. Defined and Reverse have no realized seed; an unused
store receives no record from the default native scheduling path.

`SchedulingOptions::with_ordering_store(store)` shares records with the caller.
A generated-only `on_generated_seed(handler)` sink receives the exact recorded
realization and replaces default stderr reporting. Emit exactly once after the
permutation is realized and before dispatch begins, including an empty selected
plan. Explicitly seeded, Defined and Reverse orders do not generate reports.
Cloning retains shared observation/sink configuration, but each actual run
resolves RandomGenerated anew. Cancellation before permutation creates no
realization; later cancellation does not erase an already completed permutation.

## Native realization and integration

Use a fresh standard-library `RandomState` to hash a per-realization counter,
then take modulo 1,000,000 and format decimal. This is noncryptographic ordering
entropy, not a claim to reproduce JavaScript's entropy generator or an exact
uniform probability distribution. Its realized decimal string is passed to the
existing pinned shuffle unchanged. No new dependency, seed-factory API, second
runner, queue, or RNG abstraction is necessary.

Resolve once at `execute_ordered`'s existing global ordering seam after selection
and outline expansion and before dynamic assignment. Report and store that same
realization. The application's replay path is existing `Random { seed }`; the
fixed-seed JS oracle remains the behavior reference. Exact warning spelling is
not a public compatibility promise. The current report uses the pinned wording
`Random order using seed: <seed>`. No Basic runner edit was required: its existing
call to SchedulingOptions.order is the sole integration seam.

## Acceptance boundary

Ten cases in `tests/features/generated_ordering/generated.feature` cover seed
range and actual once-per-run observation, replay against the existing pinned
oracle, a globally filtered expanded plan, a custom report sink, repeated cloned
options without probabilistic inequality assertions, explicit empty seed,
Defined/Reverse compatibility, untouched native scheduling, and a filtered empty
plan with zero actual World constructions and one recorded/reported realization. The harness runs
actual child applications, captures report order relative to real setup
callbacks, and invokes the existing pinned Node shuffle oracle only during its
acceptance steps. No unit suites,
early builds, or runtime probes are authorized.


## Actual phase-end BDD evidence

The [first boundary log](evidence/phase41-first-bdd.log) records **10 scenarios /
45 steps passed**, exit 0, with no production or harness correction required.
All acceptance features were authored before their corresponding implementation
or harness changes, and execution began only after the phase was complete.

Exact completed primary command:

```sh
cargo test --features output-messages,tracing --test generated_ordering
```

Actual child applications proved once-per-permutation seed storage/reporting
before scenario callbacks; explicit replay against the pinned JavaScript shuffle
for the full and filtered expanded plans; custom report sinks; sequential cloned
reuse with distinct run identities without probabilistic seed inequality;
explicit empty-string, Defined/Reverse and default-native compatibility; and an
empty selected plan with zero World constructions and one recorded/reported seed.

The five affected BDD targets (`ordered_scheduling`, `dynamic_assignment`,
`retry_policy`, `messages_logs`, `source_paths`) passed **69 scenarios / 282
steps**, exit 0, in the [affected boundary log](evidence/phase41-affected-bdd.log).
The parent is running the final Rust 1.88 acceptance boundary, including this
target; that compatibility result is still pending. No unit suites, generic cargo test,
early builds/checks, new dependencies, forks, publication or deployment were
introduced for this phase.

Final minimum-toolchain boundary: this target also passed within the13-target
Rust1.88 acceptance (172 scenarios/670 steps total). See
[evidence](evidence/msrv-1.88-first-bdd.log) and
[exact command](evidence/msrv-1.88-command.txt). Earlier pending statements
above record prior checkpoints. Production needed no MSRV correction.
