# Native retry reporting boundary

This phase adds Messages 34.2.1 attempts for the existing native retry policy.
It does not alter runner retries or claim JavaScript retry policy parity.

One TestCase is emitted per pickle; each real attempt has its own contiguous
zero-based attempt number and TestCaseStarted ID. Failed attempts preserve their
results. willBeRetried reflects remaining native retries for panics/ambiguity;
native undefined steps are terminal. Run success uses terminal outcomes, so a
recovered scenario does not permanently poison success or hide other failures.

The writer checks unchanged retry metadata during each attempt and rejects
unexpected duplicate/noncontiguous attempts or completion with a pending retry.
Hooks and logs remain unsupported at this boundary.

Acceptance: tests/features/messages_retries/retries.feature, covering recovered,
exhausted, passing, undefined, zero-budget and mixed concurrent scenarios. All
emitted envelopes are checked against the vendored official schema and reference
and lifecycle contracts. Phase-end acceptance passed: **6 scenarios / 40 steps**, exit 0. The existing
Messages BDD contract also passed **22 scenarios / 105 steps** after removing
its obsolete negative retry case. The first boundary attempt caught a BDD
macro function/argument-name collision; the harness was corrected and the
initial diagnostic preserved. No production correction was needed.

Evidence: `evidence/phase10-retries-bdd.log`,
`evidence/phase10-retries-initial-compilation.log`, and
`evidence/phase10-messages-regression-bdd.log`.

Commands (only at the completed phase boundary):

```sh
cargo test --test messages_retries --features output-messages -- --tags @phase10
cargo test --test messages --features output-messages -- --tags @phase4
```
