# Native hook stages in Messages

Phase 11 represents the native runner's observed before-scenario and
after-scenario stages using Messages 34.2.1 `Hook` and hook `TestStep` envelopes.
It applies to both original and snapshot-backed Messages constructors.

Each installed native stage is one composite hook, including when the `Hooks`
registry contains multiple callbacks or its tag filters select no callbacks.
The native events expose only the composite stage. The report therefore does
not claim that every registered callback ran, or invent callback identities,
source locations, or tag expressions. Hook names explicitly identify the native
composite stage; types are `BEFORE_TEST_CASE` / `AFTER_TEST_CASE`.
`sourceReference: {}` is permitted by the pinned official schema and expresses
that no source reference is available. Individual callback locations and other
lifecycle hook kinds remain outside this contract.

The writer buffers one normalized scenario attempt until its finish, allowing
its complete `TestCase` (setup, body, cleanup) to precede execution envelopes.
Observed timestamps and elapsed durations remain unchanged. The report emits
hook definitions once, reuses a test case and hook test-step identities across
retries, and rejects an attempt whose observed stage shape changes. This trades
per-attempt output latency and memory proportional to that attempt's envelopes
for a complete protocol execution plan without guessing installed hooks.

Failed setup marks unexecuted body steps `SKIPPED`; cleanup still reports its
observed outcome. Skipped steps use zero duration and the observed cleanup-start
boundary (scenario-finish boundary if no cleanup stage exists). Native undefined
steps remain `UNDEFINED`, followed by skipped body steps. Failed setup or cleanup
can trigger the native retry policy; only terminal failed attempts make the
completed run unsuccessful. Cleanup's native aggregated panic text is retained,
including failures from multiple callbacks that continued during teardown.

The phase's feature-first acceptance target is:

```sh
cargo test --features output-messages --test messages_hooks -- --tags @phase11
```

Acceptance covers successful stages, setup/body/cleanup failure, undefined body,
cleanup continuation after failures, filtered callbacks, empty scenarios,
recovered setup and cleanup retries, and exhausted cleanup retries. Every
reported envelope is validated against the pinned schema; definitions precede
references, hook steps are distinct from Pickle steps, execution follows the
TestCase order, and start/end durations and retry identities are checked.
Phase-end BDD passed **10 scenarios / 70 steps**, with evidence in
[evidence/phase11-hooks-bdd.log](evidence/phase11-hooks-bdd.log). The initial
harness compile failure (macro argument shadowing its step function name) is
preserved in [evidence/phase11-hooks-initial.log](evidence/phase11-hooks-initial.log);
it was corrected before the successful run.

Because attempt buffering changes the common execution path, focused prior
Messages acceptance also ran at this phase end: plain Messages **21 scenarios /
101 steps**, retries **6 scenarios / 40 steps**, and rich/localized source graphs
**19 scenarios / 80 steps** all passed. See
[evidence/phase11-messages-regression.log](evidence/phase11-messages-regression.log).
The obsolete “reject hooks” example was removed because the new positive
acceptance now covers that capability. No generic or unit suites were run.
