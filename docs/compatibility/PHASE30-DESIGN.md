# Phase 30 design: timeouts, then explicit pending/skipped outcomes

Status: Phase30a production integration and all fourteen authored BDD cases are
implemented and the focused acceptance passed: **14 scenarios / 50 steps**.
No build/check/test ran during authoring or integration. Phase30b remains a design
only. The pre-implementation gap table below is retained as the baseline.

## Pinned behavior and verified native gaps

Baseline: cucumber/cucumber-js v13.3.0.

| Behavior | Tagged JavaScript behavior | Current native gap |
| --- | --- | --- |
| Timeout defaults | Async steps and all hook definitions use 5000 ms; each definition can override, negative values disable | Callbacks are awaited without a runner deadline |
| Expiry | FAILED; scenario failures can retry; cleanup still follows normal lifecycle | No native timeout outcome |
| Pending | PENDING; fails by default, succeeds with no-strict; never retries merely for pending | Fixed step callback returns unit; non-unit macro returns are treated as Result and errors panic |
| Explicit skip | SKIPPED; run succeeds; remaining body steps skip, including undefined/ambiguous definitions | Native Step::Skipped means undefined; Messages maps it to UNDEFINED |
| Later undefined after pending/failure | Definition lookup still happens, so the later step is UNDEFINED | Existing abort/skip handling cannot be assumed equivalent |
| Before-scenario pending/skip | Later before hooks and bodies skip; after hooks still run | No explicit completion API |
| Step-hook pending/skip | All selected callbacks run; only a FAILED before-step aggregate suppresses the body; results/durations aggregate into the pickle step | Existing hooks expose only passed/not-run/failed body outcomes |
| Global-hook pending/skip strings | Global execution inspects error only; a normal non-error result passes | Do not invent global pending semantics |

Primary evidence:
[defaults](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/support_code_library_builder/index.ts#L448),
[per-definition timeout selection](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/runtime/step_runner.ts#L49),
[async timeout and disabled handling](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/user_code_runner.ts#L30),
[pending/skipped result mapping](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/runtime/step_runner.ts#L70),
[retry condition](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/runtime/test_case_runner.ts#L266),
[strict failure policy](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/runtime/helpers.ts#L62),
[explicit-skip versus definition lookup](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/runtime/test_case_runner.ts#L317),
[step-hook aggregation](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/runtime/test_case_runner.ts#L295),
[global-hook result handling](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/runtime/executor.ts#L60).

Official acceptance examples:
[step timeouts](https://github.com/cucumber/cucumber-js/blob/v13.3.0/features/step_definition_timeouts.feature),
[hook timeouts](https://github.com/cucumber/cucumber-js/blob/v13.3.0/features/hook_timeouts.feature),
[global hook timeouts](https://github.com/cucumber/cucumber-js/blob/v13.3.0/features/before_after_all_hook_timeouts.feature),
[pending](https://github.com/cucumber/cucumber-js/blob/v13.3.0/features/pending_steps.feature),
[skipped](https://github.com/cucumber/cucumber-js/blob/v13.3.0/features/skipped_steps.feature),
[strict mode](https://github.com/cucumber/cucumber-js/blob/v13.3.0/features/strict_mode.feature).

Native source evidence: step::Step remains a function pointer returning
LocalBoxFuture<Output=()>; step::Context is an exhaustive public struct, so adding
fields would break external literals. World construction and step/hook callback
awaits are in runner/basic.rs. Event<T> is already non-exhaustive and conditionally
Copy; its new/map/replace/split path can preserve new Copy metadata without changing
public Step/Hook/Scenario variants.

## Phase 30a: runtime-independent timeouts first

Keep legacy unlimited execution as the default. Proposed additive types:

- TimeoutPolicy::Disabled or TimeoutPolicy::After(Duration).
- Timeouts::default() is disabled; Timeouts::cucumber_js() selects five seconds.
- Definition/callback override Option<TimeoutPolicy>: None inherits; Disabled
  disables that invocation without changing another definition.
- TimeoutClock supplies an owned timer future. The default backend is
  futures-timer 3.0.4, pinned before registration; its Delay uses a helper timer
  thread and does not require a Tokio runtime. A clock can be injected for
  deterministic BDD, without changing the caller's runtime.
- A generic helper races a borrowed callback future against its deadline. It does
  not spawn the callback, require Send, or detach it. Expiry drops the future and
  returns a typed TimeoutError. Runtime integration converts this to existing
  failed step/hook events with a clear string diagnostic, preserving old writers.

[futures-timer 3.0.4 Delay contract](https://docs.rs/futures-timer/3.0.4/futures_timer/struct.Delay.html).

Timeout each actual callback/body separately, not an entire scenario or the
combined before-step/body/after-step operation: otherwise a setup/body timeout
could suppress cleanup. Run all applicable cleanup callbacks under their own
limits and preserve their failures. Registration overrides must be keyed to real
definition identity, not a guessed regex match or current source line.

The current Rust deadline starts before the step invocation future is polled.
Source parsing and World initialization occur outside that deadline. Generated
parameter conversion (`FromStr`) and expression string unescaping currently run
inside the timed step future, so their elapsed time consumes the same deadline.
This differs from Cucumber.js 13.3, which awaits parameter preparation before
installing the callback timer. JavaScript also calls the user function before
installing its asynchronous timeout. The Rust callback-first poll may accept a
ready result even after synchronous work consumed the deadline; that work cannot
be preempted. This remains a limitation for unflagged definitions and arbitrary work inside
manual futures. [Phase30c](PHASE30C-PREPARATION-DESIGN.md) adds explicit
`prepare_before_timeout` plus `CallbackConstruction::BeforeDeadline`; its primary
17-scenario BDD verifies the bounded preparation/factory timing behavior. It does
not establish general Rust/JavaScript async-prefix or blocking-work equivalence.

Evidence: [generated async wrapper](../../codegen/src/attribute.rs),
[native invocation boundary](../../src/runner/basic.rs),
[timer construction and race](../../src/runner/timeout.rs),
[JS parameter preparation](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/models/step_definition.ts#L25),
[JS step invocation boundary](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/runtime/step_runner.ts#L36),
and [JS function call and asynchronous timer](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/user_code_runner.ts#L30).

Legacy raw before/after callbacks are one callback each; registered Hooks and
StepHooks apply limits inside each callback loop. Global RunHooks use the same
helper and existing record/error propagation. A future bounded phase could move
owned generated argument preparation into future construction and construct that
future before starting the timer, while retaining the public Step alias. Borrowed
step/slice arguments and constructor panics require explicit acceptance. Arbitrary
manual or asynchronous transformations need an explicit preparation contract for
a general exclusion guarantee; no such new API or behavior is implemented here.

Rust cancellation is cooperative: dropping a future releases its borrowed World
and locals but does not undo side effects or stop externally spawned work.
Blocking synchronous code cannot be preempted. JavaScript Promise.race also does
not forcibly terminate underlying work; do not claim hard execution isolation.

The staged helper is not active until its dependency, exports, builders,
definition overrides and every intended lifecycle seam are wired and reviewed.
Only then run focused BDD.

Acceptance boundaries:

1. Legacy unlimited, explicit five-second preset, completed-before-deadline.
2. Body expiry reports failure, drops its future, closes its attachment scope,
   runs cleanup, and can retry with a fresh World.
3. Longer per-definition limit and Disabled override do not leak to other steps.
4. Before/after scenario, before/after step, and before/after run callbacks time
   out individually while their established cleanup policy remains intact.
5. Concurrency does not share deadline state.
6. A plain futures executor runs the backend; no implicit Tokio dependency.
7. Existing scenario/step/run hooks, attachments, retries and Messages remain
   compatible through focused BDD regressions.

Use channels and an injected timer for ordering. A real backend acceptance can
race an intentionally never-ready body with a short timeout, asserting outcome
and cancellation rather than a fragile wall-clock threshold.

## Phase 30b: explicit outcomes after the timeout boundary

Proposed no-panic API, requiring explicit opt-in:

- Add World::new_with_runtime_context(WorldContext), whose default delegates to
  the existing new_with_context(context.attachments()). Existing original and
  attachment-aware constructors keep working.
- WorldContext supplies a scoped ExecutionControl. A body can explicitly return
  `world.execution.current()?.complete(Completion::Pending)` or Skipped from a
  Result<(), ControlError> function. This works with existing unit-returning
  wrappers. Completion records a request; it DOES NOT exit the function. The
  caller must return. A subsequent panic or timeout wins over the request.
- Each actual callback/body invocation gets its own immutable bound handle.
  Completion is accepted once; closed or stale handles cannot affect another
  callback, step or attempt.
- OutcomePolicy::Legacy remains default. The explicit JS policy has strict
  pending enabled by default and an explicit no-strict option.

No panic payload should be used for normal pending/skipped control flow. A direct
macro return type may be a later convenience, after the scoped mechanism is
proven; do not change the established Step function signature in this phase.

Add small Copy OutcomeMetadata to non-exhaustive Event and preserve it through
new/map/replace/split/normalize/clone. It records actual result status, whether it
causes final run failure and whether it is retryable. Pending/skipped are never
carried as Passed. Existing non-passing native variants form a documented lossy
projection; metadata is authoritative for opt-in execution. The precise projection
for composite hook pending still needs agreement before implementation.

This preserves source compatibility for custom writer matches and Event::new
callers. It cannot make an old writer that ignores metadata report a previously
unrepresentable status faithfully. No blanket claim of semantic compatibility
for unaware custom writers is justified. Opt-in users must choose metadata-aware
consumers; unknown writers may conservatively report the non-passing projection.

## Built-in writer audit required for 30b

- Basic: current step/hook matches print existing Passed/Skipped/Failed variants.
- Summarize: current Step::Skipped increments skipped, and failures with retries
  count as retried. Add accurate pending counts and strict final failure policy.
- Stats trait: add default methods if needed so existing custom implementations
  remain source-compatible; forward through all built-in wrappers.
- FailOnSkipped: currently rewrites every Step::Skipped to NotFound. Explicit
  skipped outcomes must not become UNDEFINED; pending strictness stays distinct.
- Repeat::failed/skipped: current predicates inspect variants only. Classification
  must use authoritative status while retaining complete event metadata.
- Normalize, Tee, Or, Discard, RunHooks and passthrough output adapters: preserve
  metadata and forward final failure/pending statistics rather than recomputing
  only from legacy counters.
- Messages and JSON: map actual Pending/Skipped separately from Undefined, retain
  real execution duration and no-retry policy.
- JUnit and libtest: use explicit documented representations for pending/skipped
  and ensure exit/failure semantics agree with strict policy. Do not call pending
  passed merely because a format lacks a dedicated Pending status.
- Cucumber::run_and_exit: pending strict failure must produce a meaningful summary,
  not an empty failure message.

30b business scenarios must cover: pending strict/non-strict with no retries;
explicit skip success; cleanup after each; skipping later before hooks; subsequent
undefined and ambiguous steps after explicit skip versus pending/failure; cleanup
failure overriding pending; before-step pending/skip still running the body;
after-step pending/skip aggregation; concurrent and retried scope isolation;
stale completion handle rejection; native stats, all built-in writer adapters and
official Query agreeing; old custom writer source compatibility with documented
lossy projection.

No separate Messages TestSteps are invented for individual step hooks: the
tagged JS compiler aggregates their results and durations into the pickle step.


## Phase30a implementation boundary

`runner::Timeouts` defaults to `TimeoutPolicy::Disabled`; applications explicitly
select `.with_timeouts(Timeouts::cucumber_js())` for the pinned five-second preset
or `.with_default(TimeoutPolicy::After(duration))`. `TimeoutClock` permits an owned,
runtime-independent timer factory; production uses `futures-timer =3.0.4`. No new
fork is needed. Existing event variants and callback aliases remain unchanged.

`DefinitionOptions::timeout` overrides one step registration. Every actual
registered callback is timed independently. `Hooks`, `StepHooks`, and `RunHooks`
provide additive `before_with_timeout`/`after_with_timeout` registration methods;
conditional/scoped variants follow their existing method names with
`_with_timeout`. Missing overrides inherit the runner default; `Disabled` opts
out locally. Raw Basic/Cucumber before/after closures inherit the runner default.
World initialization remains outside callback limits.

Expiration drops the borrowed callback future, records the existing failed
step/hook representation with the configured duration, and retains native cleanup
and retry behavior. Step attachment handle generations close immediately inside
each timed-out invocation boundary, before the next hook/body cleanup; the new
generation retains the same native step target. Detection uses a typed internal
`TimeoutError`, never diagnostic text. Observed scenario/global callback records
and Messages continue to represent actual invocations.

The helper polls the callback before the timer when both are ready. This is
cooperative cancellation: blocking synchronous polls cannot be preempted, and
application-owned independently spawned tasks are not cancelled by the runner.
Rust future dropping is stronger than JavaScript's Promise.race behavior, which
does not cancel the underlying promise. Neither is process isolation.

The focused harness uses channel-controlled deadline completion and body gates;
there are no sleep-threshold assertions. It checks native failure statistics,
Messages 34.2.1 schema, cancellation before cleanup, stale attachment rejection,
retry attempts and new Worlds, concurrent independent deadlines, and a real timer
under `futures::executor::block_on` with no Tokio runtime. Phase-end command:

```sh
cargo test --test timeouts --features output-messages -- --tags @phase30_timeouts
```

At the coordinated 30a/32a source-complete boundary, the first command compiled
production but stopped on the harness calling `Application::cucumber()` for a
manual World without inventory. It was corrected to `Cucumber::new()`. Three
Registration insertion drop warnings were corrected by the Phase32a owner.
No scenario executed on that first attempt; no timeout production defect was
found. [First compile diagnostics](evidence/phase30-timeouts-first-compile.log).

The repeated focused command passed **14 scenarios / 50 steps**, warning-free,
exit 0. [Focused BDD evidence](evidence/phase30-timeouts-bdd.log). The six hook
stages exercised actual timed callback observations, downstream cleanup, and
Messages/native failure agreement. BeforeStep timeout also proved that a following
BeforeStep callback rejects the old attachment handle immediately, before the
before-stage loop returns. The after-step cleanup rejected expired body/AfterStep
handles in their corresponding cases.

After Phase32a primary acceptance passed (19 scenarios / 85 steps), one agreed
batch of eleven affected BDD targets passed **174 scenarios / 751 steps**, exit 0,
warning-free. No production or harness correction was needed during this batch.
[Exact command](evidence/phase30a-32a-affected-command.txt) and
[full output](evidence/phase30a-32a-affected-bdd.log).

| Target | Scenarios passed | Steps passed |
| --- | ---: | ---: |
| compatibility | 20 | 115 |
| expression_compat | 53 | 159 |
| hooks | 9 | 42 |
| messages_attachments | 9 | 36 |
| messages_retries | 6 | 40 |
| messages_scenario_hooks | 13 | 64 |
| messages_support_code | 14 | 56 |
| run_hooks | 14 | 62 |
| scenario_hook_records | 18 | 76 |
| scheduling | 8 | 50 |
| step_hooks | 10 | 51 |
| **Total** | **174** | **751** |

Shared production/Cargo ownership was released for Phase30b after this boundary.
No unit suites, generic cargo tests, lint, format checks or premature build
commands were used.
