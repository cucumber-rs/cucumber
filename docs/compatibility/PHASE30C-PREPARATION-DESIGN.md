# Phase30c proposal: argument preparation and callback construction

Status: Phase30c's completed-phase primary BDD passed **17 scenarios / 69 steps**.
The design and feature preceded implementation. The coordinated five-target
primary boundary passed **67 scenarios / 268 steps** in total. Affected regression
coverage is still running; no full timing-parity claim is made. Phase32b remains
separately complete (16 scenarios / 78 steps).

## Observed boundary

Pinned Cucumber.js 13.3.0 resolves invocation parameters before its user-code
runner and calls the supplied function before installing the asynchronous timer.
Sources: [step runner](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/runtime/step_runner.ts),
[user-code runner](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/user_code_runner.ts).
These source facts do not establish general JavaScript/Rust async equivalence.

The Rust macro's generated closure currently places string unescaping and
`FromStr` conversion inside its `async move` block
(`codegen/src/attribute.rs::Step::expand`). `Basic` passes an async wrapper that
calls `step_fn` into `Timeouts::run_catching`; the timer is constructed before
that wrapper's first poll. The scenario, step and run hook paths use analogous
async wrappers. Thus both actual synchronous callback factory work and generated
argument preparation currently occur after timer construction.

## Approved explicit opt-in shape

Existing `regex`, `expr`, `expr_js` and literal macro registrations retain their
current first-poll conversion and handler timing. Add an optional trailing flag,
for example `#[given(expr_js = "name {string}", prepare_before_timeout)]`.
Only this explicit macro flag eagerly transforms owned Context matches and
parses owned scalar values or slice vectors when its registered callback is
constructed. Move those values and Context into the returned owner future.
Borrowed Step/slice bindings remain inside it, so no self-reference or unsafe
lifetime workaround is needed. Actual handler body execution remains lazy.
The flag explicitly changes direct-call preparation timing for that registration.

Add a callback-construction boundary to Timeouts with two choices:
`WithinDeadline` (legacy default) and `BeforeDeadline` (the explicit Cucumber.js
preset). An explicit setter allows selecting either independently of durations.
A private helper receives `FnOnce() -> Future`. WithinDeadline wraps factory
invocation in the existing timed future; BeforeDeadline catches factory panic
before installing the timer, then runs the returned future through the existing
catching/timer helper. Move a world reborrow into the factory so its future may
borrow safely. Preserve execution/attachment guards, cleanup and classification.

Macro preparation requires both the macro flag and BeforeDeadline policy to be
excluded from an active callback deadline. BeforeDeadline alone cannot inspect
or move arbitrary work hidden inside a manual callback's returned future.
Neither choice changes public Step/Context signatures, adds fields to Context,
or requires a second registration path. Timer policy propagation must preserve
the boundary across existing clones/builders. Hook callbacks use the same helper.

Rust async bodies begin during polling; JS async functions execute their prefix
during invocation. This phase does not emulate that distinction or make Rust
synchronous handler bodies preemptible. Existing callback-first tie behavior and
disabled-timeout behavior remain. The bounded syntax/API cost is one optional
macro flag, one Timeouts configuration enum/field and one private invocation
helper; no new dependency or fork is needed.

## Authored bounded acceptance (17 scenarios)

`tests/features/timeout_preparation/preparation.feature` was authored before any
production or harness changes. It includes three legacy direct-call cases, three
opted-in conversion/borrow cases, conversion failure, two explicit raw factory
order cases, factory panic, six lifecycle factory cases and disabled deadlines.
Use an injected recording clock and deterministic trace order, not sleeps or
wall-clock thresholds. Legacy scenario-hook APIs must remain represented by the
hook fixtures; preserve scoped callbacks as affected regression coverage.

The custom scalar fixture observes FromStr before the timer. The quoted-string
fixture verifies unescaping/conversion values. The slice fixture keeps a borrowed
Step and vector-backed slice usable across suspension. Negative fixtures retain
ordinary failures and cleanup while proving no timer was created for a failed
factory/preparation. Both raw boundary choices prove cancellation remains real.

Existing timeout/cancellation, expression conversion and borrowed-context BDD
are candidate affected regressions only at the completed phase boundary. No
production or harness activation may begin until the parent releases shared
runner ownership after Phase37.

## Implemented API and integration

- `runner::CallbackConstruction::{WithinDeadline, BeforeDeadline}`; default
  Timeouts uses WithinDeadline, `Timeouts::cucumber_js()` selects BeforeDeadline
  alongside its existing five-second duration. `with_callback_construction` and
  `callback_construction` set/read the boundary independently of durations.
- `#[given(..., prepare_before_timeout)]` and the equivalent when/then forms
  prepare owned scalars/vectors and quoted-string conversion in the registered
  callback factory. Only borrowed Step/slice bindings and handler execution
  remain in the returned future. The unflagged expansion remains exactly lazy.
- `Timeouts::invoke_callback` catches factory panic separately in BeforeDeadline,
  then reuses existing polling/deadline cancellation. The WithinDeadline branch
  invokes the factory inside the timed future, preserving prior behavior.
- Basic's step, legacy before/after callbacks and all retained scenario, step and
  run hook registries call this helper with actual factories. World reborrows are
  moved into FnOnce factories; no unsafe lifetime conversion is introduced.
- `tests/timeout_preparation.rs` implements all17 authored cases with a recording
  clock. Success clocks remain pending; cancellation clocks resolve immediately
  after the callback-first poll. No sleeps or timing thresholds prove ordering.

The focused target is `timeout_preparation`, tag `@phase30c_preparation`. Its Cargo
entry requires macros only; affected phase-end targets should include timeouts,
expression_compat, compatibility, hooks, step_hooks, run_hooks and relevant
scoped-hook/attachment cases. No unit suites or pre-boundary checks are authorized.

Limits remain: synchronous factory work is not preemptible; arbitrary work inside
a manually returned future is not moved; Rust async body prefixes still start
during polling. JavaScript async-prefix equivalence and complete timing parity
are not claimed. No new dependency or fork is required.

## Actual phase-end BDD evidence

The [second coordinated boundary log](evidence/phase30c-37b-38b-40-42-second-bdd.log)
records `timeout_preparation`: **17 scenarios passed / 69 steps passed**. The
recording clock established actual callback-factory/timer order; direct callback
construction established legacy laziness. Real execution also proved conversion
values, borrowed scalar-context and vector-slice arguments across suspension,
preparation/factory failures without a body timer, ordinary cleanup, pending-future
drop on expiry, all six lifecycle factory seams and disabled deadlines.

The [first boundary log](evidence/phase30c-37b-38b-40-42-first-bdd.log) retains the
initial harness-only compiler diagnostics: legacy callback function pointers
needed boxing after `with_hooks`, and the custom Basic writer needed normalization
before the RunHooks statistics wrapper. Corrections were confined to the harness.
Production compiled at that first boundary; no production/runtime correction was
required after the authored scenarios started executing.

All five primary targets in the second coordinated run passed (67 scenarios /
268 steps total). This document claims the Phase30c result separately; the broader
32-target affected BDD run and Rust1.88 boundary are pending. No generic cargo test,
unit suite or premature build/check was run. No new fork, push or deployment was
needed.

Final minimum-toolchain boundary: this target also passed within the13-target
Rust1.88 acceptance (172 scenarios/670 steps total). See
[evidence](evidence/msrv-1.88-first-bdd.log) and
[exact command](evidence/msrv-1.88-command.txt). Earlier pending statements
above record prior checkpoints. Production needed no MSRV correction.
