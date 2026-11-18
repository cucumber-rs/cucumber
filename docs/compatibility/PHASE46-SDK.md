# Phase46: installed native source and rerun SDK

The adopted first SDK phase has completed its bounded behavioral proof and
internal review on Darwin arm64, Rust1.88.0, Node26.5.0, CPython3.11.3 and a real
TypeScript6.0.3 declaration consumer. Final evidence: **11 accepted scenarios /
33 steps, plus11 negative-control scenarios /33 steps; both commands exit0**.
Actual outer BDD Messages prove22 cases/66 passed steps before owned cleanup.

This is shared compilation/rerun and safe Rust-extension evidence. Foreign step
execution, Behave/pytest-bdd adapters, other platforms/interpreters, registry
publication and complete Cucumber.js parity are not established.

## Usable surfaces

- sdk/: safe Rust Sdk compile, rerun, capabilities, versioned JSON request,
  instance-owned namespaced Extension registration and invocation.
- bindings/node/: in-process NAPI-RS addon, CommonJS facade, TypeScript
  declarations and typed errors; private local npm archive.
- bindings/python/: in-process PyO3 extension, typed facade, structured errors
  and a local CPython3.11 macOS arm64 wheel.

The language wrappers delegate native behavior to the same Rust implementation;
they do not duplicate Gherkin semantics or invoke a parser subprocess. The Rust
consumer is built in an external Cargo project; npm/wheel consumers are installed
outside the checkout with module/native realpaths and artifact hashes retained.
The base packages require neither Behave nor pytest. Public names remain
unconfirmed placeholders, and no registry registration/publication occurred.

## Accepted behavior

The eleven cases cover the full retained plaintext source corpus and invalid
fixtures; Markdown canonical graphs and argument restrictions; featureless
sources; contextual errors and same-host recovery; strict protocol capabilities
and request shapes; real retry/outline Messages with original row identities,
full attempts and independent pinned JavaScript formatter/collector comparison;
incomplete/missing-reference evidence; safe empty rerun selection; external Rust
extensions; actual installed artifacts plus TypeScript compile/execute; and
same-host request isolation. Decimal-string counters preserve integers above
2^53. Controls corrupt graph references/locations, drop attempts, forge numeric
objects, remove native libraries in fresh processes and reject false selections.

Final retained artifacts:

- [Functional BDD](evidence/sdk46-repair2/functional-bdd.log)
- [Control BDD](evidence/sdk46-repair2/controls-bdd.log)
- [Outer Messages proof](evidence/sdk46-repair2/outer-messages-proof.json)
- [Preparation/provenance](evidence/sdk46-repair2/preparation-complete.json)
- [Cleanup](evidence/sdk46-repair2/cleanup-complete.json)

Each case/control also retains original child argv, exits, raw inputs/outputs,
observations and oracle identities. Native archives remain locally under the
ignored artifacts directory; their hashes remain in runtime.json evidence.

## Retained failures and review corrections

The first preparation successfully built/installed native packages but discovered
Node24.11.1 instead of the adopted26.5.0. No behavior acceptance was claimed.
Repair1 added exact runtime admission and used the existing pinned installation;
all22 cases/66 steps then passed. Both original directories remain intact.

Independent post-BDD review identified SDK46-IR-01: serde_json arbitrary-precision
Value parsing can reinterpret a specially named object as a number, bypassing
strict field-type validation. Repair2 uses token-preserving RawValue
materialization with a nesting limit, preserving exact numbers while keeping
objects as objects. Added SDK005/007 cases prove rejection and recovery through
all three installed consumers. Static QA SQA-01 identified missing promised outer
BDD Messages; repair2 retains and validates both streams alongside text receipts.
The complete affected packet passed after both corrections.

[Independent review](SDK46-INTERNAL-REVIEW.md) and
[static QA](SDK46-STATIC-QA.md) preserve original findings and resolved
dispositions. Reviewers were internal Codex agents with exact deployment/model
unknown. No external transmission or cross-model certification is claimed. This
phase review is not the final audit of all core/Gherkin/programme work; that gate
remains before upstream submissions after the remaining requested work.

## Authorization and limits

Travis adopted the eleven-case phase and explicitly authorized signing the exact
prepared implementation-completion amendment with his existing registered key.
The signature verified; no new key or allowed-signer change was made. The two
bounded repairs are retained, with their authorizations and cumulative budget
recorded. No unit suites or early implementation checks ran. Existing root
Cargo/build/release workflows remain unchanged; separately authorized
[Pages workflows](DOCS-PAGES.md) publish documentation only.

GQAdonis/gherkin remains the sole required dependency fork; portable integration
uses the existing vendored parser snapshot. Full upstream contribution planning,
package publication ownership/license inventory and optional framework execution
remain separate gates. Do not present these bounded results as universal parity.

## Closure bookkeeping limitation

Canonical behavioral closure is CLOSED/PASS; OpenSpec static validation, archive and task reconciliation succeeded. The authored reflection is retained, but canonical Reflect registration was refused because the already-closed phase is immutable and that stage was not previously registered. A success-shaped best-effort hook line is not canonical completion; its generated handoff was retained as unaccepted evidence and removed from active handoffs. See [the exact runtime limitation](evidence/sdk46-closure/reflection-runtime-limitation.md). No unsafe reopening or direct canonical-state edits were attempted. This metadata gap does not change the retained behavior verdict.
