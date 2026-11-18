# Phase30b runner integration: explicit completion without panic control flow

Status: core implementation verified at the completed phase boundary on
2026-11-17: **20 scenarios / 87 steps passed** on the first core acceptance run.
The separate consumer-format phase and affected legacy regressions are coordinated
by the parent and are not claimed here. No compilation or tests were run during
implementation. The shared Phase33 boundary exposed unreachable-public warnings
in the internal outcome helper; visibility was narrowed before this core run.
Its log also records Basic writer dead-code warnings owned by the writer phase.

```sh
cargo test --test execution_control --features output-messages,output-json,output-junit,libtest
```

Evidence: `evidence/phase30b-core-first-bdd.log`. The core target covers real runner
execution, normal-return callback records distinct from failure, timeout precedence,
concurrent handle closure, lookup-only classifications, retry eligibility, native
event order, and old constructor/custom writer compatibility. Scope remains
explicitly opt-in; Legacy emits no authoritative outcome metadata and continues
its first-non-passing native execution contract.

## Shared authoritative types

The agreed execution module supplies Copy OutcomeStatus, Outcome and OutcomePolicy.
Status severity is Passed < Skipped < Pending < Undefined < Ambiguous < Failed.
Outcome contains status, fails_run and retryable. Retryable denotes eligibility,
not remaining-attempt availability. CucumberJs policy fails on Failed, Ambiguous,
Undefined and strict Pending, and only Failed is retry-eligible. Legacy remains
the default, disables execution control and emits no new outcome metadata.

Every JS-policy terminal step/hook and Scenario::Finished event carries
Some(Outcome). Event::new remains compatible; map/replace/split and consumers must
preserve this Copy metadata. The existing native event variants are explicitly
lossy projections: Pending and Skipped use Step::Skipped; controlled non-passing
setup uses Hook::Failed with a diagnostic carrier constructed directly, without
panicking. Never project a controlled status as a passed event.

Primary sources:
- https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/runtime/test_case_runner.ts
- https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/runtime/step_runner.ts
- https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/runtime/helpers.ts

## One native execution loop

Replace the three existing try_fold sequences with one ordered iterator of native
feature-background, Rule-background and scenario steps, retaining the configured
background policy and source objects. Do not copy run_scenario or build a second
runner for the opt-in mode.

The attempt keeps one owned Option<World>, a worst actual outcome and its first
most-severe diagnostic, plus pending event descriptors. Equal severity retains
the earlier result. A completed step returns its World and actual outcome rather
than requiring panic-based control flow to exit the body loop.

Each planned step uses one of three modes in the same run_step helper:

1. Execute: resolve the definition, lazily initialize World, run step callbacks
   and body, then aggregate their actual outcomes.
2. LookupOnly: resolve definitions, emitting Undefined or Ambiguous if warranted;
   a unique definition is Skipped without initializing World or invoking hooks,
   parameter conversion or a body.
3. ExplicitSkip: do not resolve definitions; the selected slot is Skipped.

Legacy stops at its first non-passing result, as before. JS policy visits every
planned slot. Its ExplicitSkip decision uses the current aggregate status equal
to Skipped, not a sticky boolean recording whether any earlier callback requested
skip. Pending, Undefined, Ambiguous and Failed all continue definition lookup.
A failed before-step callback suppresses the body; pending/skipped before-step
callbacks do not. All selected step callbacks still run, cleanup in reverse.

## Preserve World ownership and native event order

Real failure events retain Arc<World>, while cleanup requires mutable World.
Continue the existing deferred-failure strategy: do not clone World or put it
behind an application-visible mutex.

When the first terminal event requires the final World snapshot, start a
per-attempt deferred queue. Queue that terminal event and every following body
Started/terminal descriptor. Descriptors own source identities, captures,
locations, diagnostics and observed Event metadata, but no World. This prevents
a later Step::Started from overtaking an earlier deferred failed terminal.
Before that point, ordinary World-free events may still stream as they do today.

After all scenario cleanup, convert the one World to Arc once and flush queued
body events in their original order, followed by the existing deferred After
stage events. Preserve original timestamps, including decision timestamps with
zero duration for unexecuted slots. Never infer that a classified slot ran.
Traced scenario logs keep their existing independent native stream contract.

The attempt's final flags must be separate: fails_run drives fail-fast/final
failure, while retryable and remaining attempts drive retry insertion. A strict
Pending attempt can fail the run without retrying. Failed cleanup can raise a
Pending attempt to Failed and enable a retry. Scenario::Finished metadata carries
the final aggregate so writers do not infer retryability from configured retries.

## Context and per-invocation boundaries

Create one WorldContext per native attempt with its existing attachment context
and a fresh enabled ExecutionControl only under the explicit policy. Call the new
World::new_with_runtime_context; its default delegates to new_with_context, which
preserves both existing constructor styles. World construction has no active
execution-control scope.

Each actual legacy/registered scenario hook, step hook and step body gets its own
InvocationGuard. This differs from attachment scope: all callbacks within one
pickle step may share that step's attachment target, but they must never share a
completion request. Do not put an outer control guard around the entire step.

On normal completion, finish closes the handle and reads its request. A later
panic or timeout wins over an earlier pending/skip request. Dropping a future
closes its scope; it does not execute asynchronous cleanup by itself. The engine
must reject overlapping active invocation scopes instead of silently replacing
one. Constructors and global run hooks cannot manufacture scoped completion.

## Registry changes without false failures

Registered scenario setup stops after any non-passing outcome. Selected later
slots become explicit skipped decisions; excluded registrations stay absent.
Cleanup keeps running and updates the actual prior outcome between invocations,
except for the separately selected JS13.3 empty-plan classification policy.

Return an internal stage summary to Basic instead of throwing an aggregate panic
for a normal control request. Actual callback errors retain existing diagnostics.
Generate the legacy Hook::Failed projection directly when a controlled stage needs
one. Basic, rather than a callback panic, attaches authoritative metadata.

Records add outcome: Option<Outcome> separately from Result<(),String>. A callback
that returns normally after requesting Pending still has Result::Ok; its actual
outcome is Pending. Readers use the outcome first. Existing Legacy records keep
outcome None. ScenarioHookScope exposes the exact prior Outcome independently of
its legacy body snapshot. The new unpublished ScenarioHookStatus gains Pending
and Skipped, derived from actual aggregate Outcome rather than the projection.
PreviousNonPassing distinguishes control-driven skipped setup slots from hard
PreviousFailure; only genuine callback errors enter previous_hook_failures.
The new unpublished StepBodyOutcome API gains truthful
Pending/Skipped variants; the old exhaustive Phase20 harness will explicitly
reject those variants in its unchanged scenarios instead of weakening assertions.

Step callback results aggregate across all before-step callbacks, the optional
body and all after-step callbacks. Every after-step callback sees the same body
result, not the evolving cleanup aggregate. No individual step-hook Messages
TestSteps are introduced.

## Ownership and phase boundary

Root owns execution context/types, Event metadata/default World constructor,
Cucumber policy builder and built-in writer adaptation, including record outcome
consumption in Messages. Lifecycle agent owns Basic's shared iterator, deferred
descriptors, actual outcome propagation and scenario/step registry invocation.
Phase30a timeout seams are preserved and reused after its release.

Complete both sides and the real application acceptance harness before the first
focused Phase30b BDD run. Existing custom writer source must still compile, while
its lossy projection is documented rather than claimed semantically exact.
Focused regressions follow completed primary acceptance; no generic suites,
unit tests or premature build/check commands are authorized.
