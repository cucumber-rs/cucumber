# Phase26: global lifecycle Messages

The scenarios in `tests/features/messages_run_hooks/run_hooks.feature` were authored
before implementation. This phase connects Phase24 actual `RunHookStore` records
to the standard reporter through `Messages::with_run_hook_store`.

Configure the raw reporter with the same store as `RunHooks`, normalize it, then
install it before `Cucumber::with_run_hooks`. Each reporter captures the current
native run at Started. Sequential store reuse is supported; concurrent independent
runs require independent stores. Definitions have real registration names and
IDs; sourceReference is empty because source registration locations are not
available. Actual invocation timestamps, elapsed duration, status and panic
message are preserved. No scenario failures are fabricated for global failures.

BeforeAll records precede scenarios, and AfterAll records follow scenarios and
precede TestRunFinished. Empty input and setup failure still emit every observed
callback. Filtering by stage avoids reporting already-buffered cleanup early.
Hook failure makes TestRunFinished.success false. The existing RunHooks statistics
adapter retains responsibility for run_and_exit failure, without duplicate counts.

Phase-end acceptance passed on its first run: **6 scenarios / 34 steps**.
Command: `cargo test --features output-messages --test messages_run_hooks -- --tags @phase26`.
Evidence: `evidence/phase26-run-hook-messages-first.log`. It covers passing, setup/cleanup/both failures,
empty input, sequential store reuse, full Messages34 schema validation and pinned
official Query16 resolution. Cancellation cleanup and concurrent shared-store
reuse are not promised. No unit suite, publishing or deployment is part of this phase.
