# Phase27: Actual support-code metadata

Status: complete bounded implementation and focused BDD acceptance passed.
This verifies the contract below, not full Cucumber.js parity.

The bounded contract is Cucumber Messages 34.2.1 / Cucumber.js 13.3.0 support-code
discovery, consumed by official `@cucumber/query` 16.0.0. It exposes actual final
Rust step registrations, not reconstructed JavaScript support files.

`step::Collection::support_code()` captures each live registration's compiled
regex, optional `step::Location`, keyword scope, and the collection's matching
policy. Stable snapshot identities are assigned after deterministic sorting.
The existing map insertion/replacement and cross-keyword ambiguity semantics
are preserved. Regex objects are cloned so runtime flags remain effective when
resolving matching identities.

A fresh `support_code::Store` shares the registry between runner and reporter.
The additive `Runner::with_support_code_store` defaults to a no-op for
custom runners. Basic publishes at `run`, after all builders, rather than at
`with_messages` configuration time. Independent runs require independent stores.

Messages definitions use `pattern.type = REGULAR_EXPRESSION` and the actual
compiled regex string. Source references retain the registered Rust file, line,
and column; registrations without locations use an empty source reference.
Original Cucumber Expression strings, parameter types and capture trees are not
stored by the existing registry and remain omitted. Macro registrations therefore
report their compiled regex accurately, even when authored with `expr`/`expr_js`.
The pattern source is `Regex::as_str()` in Rust regex syntax. The snapshot clones
actual Regex objects, so matching IDs honor programmatic builder flags, but the
metadata does not reconstruct those unavailable flags or promise JavaScript
regex replay. Replacing the entire runner after configuring Messages requires
reconfiguring the reporting handoff; replacing a collection retains it.

BDD contract in `tests/features/messages_support_code/support_code.feature`:

- Official Query resolves macro regex and actual Rust registration source.
- Unique, ambiguous and undefined references follow the selected keyword policy.
- Policy and collection builders after `with_messages` affect the frozen registry.
- Duplicate raw registrations retain replacement semantics and unknown source.
- Retries reuse one stable definition; skipped planned steps still resolve it.
- An existing custom Runner implementation needs no new required method and
  unsupported metadata is omitted rather than fabricated.
- Every envelope validates against the pinned official Messages 34.2.1 schema.

Focused boundary, run only after all integration and harness work was complete:

```
cargo test --features output-messages --test messages_support_code
cargo test --features output-messages --test messages_versions --test messages_consumer
```

No dependency fork is required. The official Query consumer is development-only
and reuses the existing pinned `tests/interop` installation.

## Observed phase-end evidence

- Primary acceptance: **14 scenarios / 56 steps passed**, exit 0.
- Affected existing official Query/JUnit consumer: **2 scenarios / 8 steps passed**.
- Affected existing protocol 34.2.1 / 35.0.0 acceptance: **3 scenarios / 12 steps passed**.
- No unit suite, generic Cargo test, premature check/build/lint, commit or publish.

The first focused boundary passed 13 of 14 scenarios. Its retry-consumer fixture
incorrectly used Query's `findAllTestCaseStarted`, which deliberately excludes
attempts marked `willBeRetried`. The fixture was corrected to enumerate observed
start envelopes while resolving every attempt through official Query. An unused
harness import was removed. No production correction was needed; the complete
14-scenario rerun and both focused regressions then passed without warnings.

Evidence:

- [First boundary, including the consumer fixture failure](evidence/phase27-support-code-first-boundary.log)
- [Final support-code acceptance](evidence/phase27-support-code-bdd.log)
- [Protocol and consumer regressions](evidence/phase27-protocol-consumer-bdd.log)

Deliverables: `src/support_code.rs`, `Collection::support_code` in `src/step.rs`,
additive Runner sink and Basic run capture, module export, both Cucumber Messages
factories, Messages definition/reference emission, `tests/messages_support_code.rs`,
its feature contract, `tests/interop/support-code.mjs`, and Cargo target/include.
No additional dependency fork was needed or created. No remaining blocker exists
for this bounded phase. Parameter-type/capture metadata and original expression
identity remain explicitly deferred because the registry does not retain them.

The schema fixture is the official
[Messages 34.2.1 StepDefinition contract](https://github.com/cucumber/messages/blob/v34.2.1/jsonschema/StepDefinition.json).
The development consumer is pinned by `tests/interop/package-lock.json` to
[@cucumber/query 16.0.0](https://github.com/cucumber/query/tree/v16.0.0).
