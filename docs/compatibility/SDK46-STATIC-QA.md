# Phase 46 independent static QA

Date: 2026-11-18. Reviewer: `/root/sdk46_static_qa`, an independent internal Codex
agent. Exact model/deployment identity is unavailable. This is not external or
cross-model certification, and it is not the final whole-program Cucumber and
Gherkin audit.

Reviewed the cumulative SDK/binding implementation at base commit
`63f14ff7a1ccf150723d98be6a07a1f4c1cb9a7a`, including the uncommitted
`prepare.py` runtime-admission repair and `sdk46-repair1` evidence. Parent-owned
current-status documentation was being updated concurrently. This report does
not certify later edits or treat signed historical contract text as a mutable
status dashboard.

## Method and scope

Read the KBD constraints, adopted eleven-case contract, concrete protocol/schema,
SDK source, both native adapters and public facades/types, manifests, licenses,
package READMEs, acceptance driver/consumers, retained command records, scenario
receipts and cleanup records. Applied the artifact-refiner/refine-validate
criteria for constraint satisfaction, artifact completeness and state consistency.
The installed artifact-refiner entry delegates to the canonical skill at
`/Volumes/T7/Projects/prometheus/prometheus-skill-pack/skills/imported/artifact-refiner/SKILL.md`.

This assignment expressly permits static review only. No build, test, import,
linter, schema-validation program, runtime probe or external review dispatch was
executed. No new refinement-state tree was created and no canonical KBD state was
edited. The report is the only reviewer-authored deliverable. Deterministic
validation and complete skill lifecycle certification are therefore not claimed.

## Result and finding dispositions

No additional material production-code defect was identified in this bounded
static pass. There is one evidence-completeness finding to resolve before claiming
that every adopted evidence requirement is fulfilled.

| ID | Severity | Finding | Disposition |
| --- | --- | --- | --- |
| SQA-01 | Medium | The adopted contract's `boundaryPacket.receipts` requests "BDD Messages and per-case/control observations". `sdk/tests/language_sdk.rs` uses the default basic writer. The repair packet has outer functional/control text logs and 22 per-case receipts, but no outer SDK acceptance Messages stream. Nested `actual-runner/*.ndjson` files exercise the rerun product and are not the lifecycle of all eleven SDK acceptance scenarios and their eleven controls. | Open; reported to the parent. Preserve the genuine existing proof. Resolve the intended scope of this evidence clause through the established KBD process; if outer Messages are required, retain them at an authorized phase-end repair boundary. Do not synthesize a historical stream or silently claim the missing artifact exists. This finding concerns evidence completeness, not an observed failure of the SDK operations. |
| SQA-02 | Publication gate | Both package notices explicitly say their retained core/Gherkin texts are not a complete resolved native dependency-license inventory. | Correctly disclosed and deferred beyond local package proof. Complete the artifact-specific inventory before public package distribution; this report is not license clearance. |
| SQA-03 | Compatibility limit | Certification is local Darwin arm64, Rust 1.88.0, Node 26.5.0 and CPython 3.11.3. Python metadata permitting >=3.9 and Node-API transport do not prove other runtimes/platforms. Foreign step execution and Behave/pytest-bdd adapters are absent. | Correctly bounded in adapter documentation and native capabilities. Keep those boundaries explicit; no additional platform or framework certification is inferred. |

Current-status prose and the stale first-attempt boundary summary are being
reconciled by the parent. They are not counted here as new product findings.
Historical snapshots must remain intact, including the first Node 24.11.1
preparation mismatch; current results must link to the repair evidence instead.

## Evidence inspected

The retained artifacts, rather than a new reviewer run, report:

- `evidence/sdk46-repair1/functional-bdd.log`: 11 scenarios and 33 steps passed.
- `evidence/sdk46-repair1/controls-bdd.log`: 11 scenarios and 33 steps passed,
  followed by successful owned-consumer cleanup.
- All `SDK-001` through `SDK-011` positive/control `receipt.json` files: PASS,
  nonempty observations, and `oracleArtifactsUnchanged: true`.
- SDK-001: 50 official good plaintext graphs and 12 rejected bad fixtures across
  Rust, Node and Python; controls reject altered locations and dangling AST IDs.
- SDK-002: five retained good Markdown fixtures and five live pinned Markdown
  boundaries; repeated arguments produce typed rejection.
- SDK-006/007/008: retained actual runner Messages, original-attempt comparison,
  pinned JS formatter/collector observations, decimal `9007199254740993`, typed
  malformed/incomplete/reference errors, and an empty-selection resolver guard.
- SDK-009: an external Rust-owned extension executes its real transformation;
  duplicate and reserved registrations are rejected.
- SDK-010: installed native paths and binary hashes outside the checkout, a real
  compiled TypeScript consumer, and fresh-process failures after removal of
  native binaries from disposable copies. The Node failure identifies the missing
  `./cucumber_sdk.node`; Python identifies the unavailable `_native` module.
- SDK-011: same-host source URI, parse-error recovery and run-identity isolation;
  controls reject cross-associated source and run identities.
- `prepare-01.stdout`, `prepare-03.stdout`, `prepare-05.stdout`: Rust 1.88.0,
  Node v26.5.0 and Python 3.11.3 respectively.
- `prepare-10.stdout`: npm archive listing includes the native artifact, public
  declarations, facade, README, notices and retained core/parser licenses.
- `cleanup-complete.json`: the successful packet's owned disposable consumer root
  was removed; local package artifacts were retained. This does not claim cleanup
  of the separately retained failed initial preparation.

The reviewer inspected evidence content and the comparator/driver source; no
independent rerun or comprehensive rehash of every artifact was performed.
Archives are intentionally ignored local artifacts, not registry publications.

## Cumulative implementation assessment

`Sdk::compile` delegates to the existing source graph and `Sdk::rerun` to the
existing collector. Host facades serialize requests and decode results; they do
not introduce alternative parsers or rerun policies. Exact source identities,
optional upstream fields, decimal counter strings and structured error context
are retained. Raw NDJSON crosses the host boundary as text.

The Rust SDK forbids unsafe code, owns extension registrations per instance and
rejects reserved/duplicate names before mutation. Extensions cannot replace the
core JSON operations. Its ordinary panic containment explicitly excludes abort,
OOM and process faults and does not promise rollback of extension side effects.
NAPI-RS/PyO3 adapters are small in-process transports with mandatory native loading.
This review is not an audit of those dependency implementations.

The three independent Cargo manifests preserve the existing workspace and build
process. npm is private, Cargo publication is disabled, and Python uses the
private classifier. The npm file allowlist and Python license-file metadata
include attribution material. Local host-specific archives, repository-layout
source dependencies, synchronous calls and missing foreign callbacks are
identified as limits rather than presented as portable public SDK parity.

## Closure recommendation

Retain the successful bounded behavioral result and this independent static
review. Resolve SQA-01 explicitly; retain SQA-02 and SQA-03 as release/compatibility
gates. No gratuitous refactor or new unit suite is recommended. This pass does
not establish whole-codebase cleanliness, final programme completion, external
certification, registry readiness, or readiness for upstream submissions.

## Repair-2 follow-up: SQA-01 resolved

2026-11-18, same independent reviewer `/root/sdk46_static_qa`; exact model remains
unknown. This section supersedes the initial open disposition and closure
recommendation for SQA-01 above. The initial finding and repair-1 evidence are
retained as history, not rewritten as though outer Messages existed then.

Inspected the amended `sdk/tests/language_sdk.rs`,
`sdk/acceptance/finalize.py`, and the retained `evidence/sdk46-repair2` packet.
The harness now tees the ordinary summary writer into a source-aware Messages
writer, selects exactly one adopted tag packet, and exclusive-creates the
functional/control streams. The emitted source and lifecycle events are actual
outer SDK acceptance events rather than reconstructed nested product evidence.

The finalizer reads both original streams before cleanup and requires one
successful completed run per stream; the exact adopted feature source; matching
case/step identities; eleven complete, non-retried cases; all 33 steps passed;
and exactly SDK-001 through SDK-011 under the appropriate packet tag. It checks
all 22 positive/control receipts before recording proof and deleting the owned
consumer root. This is a focused phase-evidence check, not a claim that the
finalizer is a general-purpose adversarial Messages validator.

Read the streams' source/lifecycle content, both log summaries, the 22 receipt
status/identity records, `outer-messages-proof.json`, and
`cleanup-complete.json`. The retained result is:

| Artifact | Recorded result |
| --- | --- |
| `functional-messages.ndjson` and `functional-bdd.log` | 11 scenarios / 33 steps passed; successful outer run |
| `controls-messages.ndjson` and `controls-bdd.log` | 11 scenarios / 33 steps passed; successful outer run |
| `outer-messages-proof.json` | PASS, 22 cases / 66 steps, exact scenario lists and stream SHA-256 identities |
| `cleanup-complete.json` | Owned repair-2 consumer root removed after proof; archives retained |

The recorded stream SHA-256 values are
`7c5338c4299c76588fb2543e636059d526a2e20bc0415afd323b75d1036c173b`
(functional) and
`5d46653708010188328c39ba927454c840f11e475679d108ce446900a78a6acd`
(controls). They are quoted from the retained proof, not independently recomputed
by this static reviewer.

Disposition: **SQA-01 resolved by repair-2 evidence and source inspection**.
No additional material finding arose in this bounded follow-up. No test, build,
import, linter, verification script or external transmission was run by the
reviewer. Only this report was edited. The separate adversarial review owns the
repair-2 lexical JSON correction; this follow-up does not independently certify
that change.

The static-QA finding no longer prevents bounded phase-46 closure. SQA-02 and
SQA-03 remain the disclosed publication and compatibility gates; the final
whole-program audit, foreign execution/framework adapter work and publication
requirements remain separate. Neither full-program parity nor whole-codebase
cleanliness is claimed.
