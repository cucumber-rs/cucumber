# Phase39: Messages projection into existing VLBV receipts

Status: 25 finite acceptance scenarios were authored before implementation.
The completed phase-end BDD passed **25 scenarios / 85 steps**, exit0.
No unit suites, generic Cargo tests or early checks/builds ran.

The standalone converter will live in cucumber and use Node standard library
only. It reads an explicit root/bundle and writes derived JSON to stdout; it does
not invoke commands, authorize execution, update KBD state, or close/release a
phase. Existing build/release patterns remain unchanged. Scope is deterministic
adopted-contract v2; stochastic/unknown profiles are BLOCKED.

## Existing authority and output shape

Read-only provenance:
- `/Volumes/T7/Projects/prometheus/prometheus-skill-pack/skills/testing/vlbv/SKILL.md`
- `skills/testing/value-scenario-verify/SKILL.md` in the same pack.
- `shared/scripts/kbd-phase-close-eval.sh`, `kbd-closure-packet.mjs`, and
  `kbd-boundary-authorize.mjs`.
- `skills/process/kbd-process-orchestrator/references/schemas/adopted-contract.v2.schema.json`.
- `substrate/kbd-runtime/src/evidence_admission.rs` and `phase_closure.rs`.

Canonical output keeps phase/projectId/runId/contractDigest/sourceDigests,
executedBy, budgetConsumption, and scenarios with id/status/attempted/observed/
evidence. Observation fields can be JSON objects; the canonical Rust reader
accepts Value. All attempts and receipt references go inside evidence, retaining
material failures without adding obsolete top-level failure rows that would
permanently veto a legitimate authorized repair. Only the canonical evaluator
and writer may close a phase. This adapter emits no closure or release claim.

## Input and evidence mapping

`--root <evidence-root> --bundle <relative-json-path>` supplies a manifest with
schema `cucumber.vlbv-receipts/v1`. Every artifact reference is `{path, sha256}`;
paths are confined to root and hashes cover original bytes. The bundle names:

- bound contract snapshot, canonical status and disposition snapshots;
- existing `kbd.boundary-authorization` record (B4/B5 event indices per run);
- current frozen candidate manifest with sourceDigests and build/configuration/
  fixture/reference artifact references;
- actual executedBy and cumulative budgetConsumption with accounting source;
- original Messages and process receipts for all runs, each with its own frozen
  candidate reference, authorization event index and optional predecessor run;
- exact adopted scenario IDs mapped to stream-qualified pickle IDs; assertions,
  negative controls and cleanup are explicit pinned producer receipts.

The accompanying `phase39-receipts.schema.json` defines adapter input and receipt
shapes. It is an adapter schema, not a replacement canonical KBD schema. Actual
contract fields remain canonical and the original snapshot digest must equal the
current disposition binding. No title/tag inference or mutable working-contract
fallback. Every adopted scenario remains in the output denominator.

Messages provide actual invocation IDs, all testCaseStarted attempts, step
outcomes, completion and original attachments. They cannot establish independent
truth, oracle sensitivity, actual candidate provenance or restoration on their
own. Explicit receipts must bind those facts to the exact stream/attempt and
frozen candidate. Never infer executedBy.model from Cucumber implementation data.

PASS requires valid current identity, exercised adopted assertions, demonstrated
negative-control rejection for their oracle, met cleanup and known budgets.
Green without assertions and skipped/dry/undefined required paths are
NOT_EXERCISED. An exercised product mismatch is FAIL. Wrong/stale identity,
missing prerequisites, unknown cleanup, exit2 and truncated output are BLOCKED.
Missing/non-sensitive negative controls leave acceptance NOT_EXERCISED. Preserve
all failed observations even if cleanup makes current acceptance BLOCKED.

Automatic retry does not supersede a failed product oracle. A later repair may
supply current acceptance only when its explicit predecessor and existing B5
authorization event match the bound phase/contract and predate the new run;
retain prior failed attempts and the original authority receipt. A supplied flag
or free-text approval is never authority. The converter does not create B4/B5
events, run repair commands or infer authorization from runner retry settings.

## Trust and limits

The adapter checks relationships and hashes; hashes prove bytes, not that a
producer's claimed observation happened. Assertion/control/cleanup receipts are
explicit producer testimony whose actual origin and independent oracle remain
review obligations. No mock assertion or green formatter is promoted silently.
A runner exit status is recorded but never acts as a product oracle. Unsupported
profiles and malformed/inconsistent receipts fail closed.

Original receipts and NDJSON remain intact. Current production source digests
are re-read under the supplied root; historical failed attempts retain their
frozen identities without being relabeled current. Existing canonical budget
and freshness checks remain authoritative downstream. Unknown enforced budget
dimensions cannot support PASS. All file writes are limited to cucumber task
artifacts during implementation; projection itself performs no file writes.

## Concrete receipts and execution boundary

The adapter schema is `docs/compatibility/phase39-receipts.schema.json`. The CLI
accepts only `--root` and `--bundle`, reads confined regular files and emits JSON
on stdout. Exit0 means every projected scenario PASS, exit3 means product FAIL,
exit2 means BLOCKED/NOT_EXERCISED or invalid input. Exit status is an adapter
summary only, never evidence of product truth. Canonical consumers still apply
their own gate. Invalid manifests without usable identities emit BLOCKED on
stderr and no misleading boundary-results document.

`tests/vlbv_adapter.rs` invokes the completed fixture producer only at its BDD
boundary. It launches the actual compiled Cucumber application as a subprocess,
captures original NDJSON/stderr/process timing/status, and challenges the same
real `assert_eq` callback with the wrong observation. Failure, retry, skip,
undefined and dry-run streams are real runner outputs. Receipt mutation fixtures
exercise refusal while retaining their underlying original runner outputs.
A complete positive fixture also invokes a byte-for-byte copied canonical
phase-close reader with explicit fixture status/disposition and `--decision-sink`
in isolated scratch. It never invokes the canonical writer or source-pack files.
Copies and original SHA-256 provenance are in `tests/fixtures/vlbv/provenance.json`.

The contract and authorization fixtures are explicitly synthetic BDD records;
they are not authority for any real KBD phase. In production callers supply
existing canonical snapshots and authorization records. Receipt authenticity and
semantic independence remain producer/reviewer responsibilities. Adapter code
contains no child-process, network or file-write operation. The fixture producer
performs these operations only to exercise the real boundary and scratch reader.

Compatibility scope: deterministic v2 adopted scenarios with explicit bindings;
Messages34.2.1/35.0.0. The adapter does not replace full canonical contract adoption
validation, execute comparators, discover new obligations, normalize a mutable
contract or silently repair evidence. Source paths with whitespace are refused
because the existing shell reader cannot reliably consume them. Original control
and product receipt references remain visible even on refusal.

### Pre-boundary review amendment

Before any execution, parent source review identified a real incomplete-run
acceptance gap. Four refusal scenarios were added before correcting code:
nonzero product exit with passing steps, failed global completion with passing
steps, an unknown step status, and an unmapped failing attempt. Product run
nonzero exit or unsuccessful completion can never yield PASS; an explicit
mapped product mismatch remains FAIL and an unexercised path remains
NOT_EXERCISED. Negative controls are different: their real rejecting assertion
may intentionally produce a nonzero process exit, but exit2 or invalid process
termination still blocks. All product streams are admitted, including unmapped
attempts; malformed streams and unclassified failed attempts cannot be ignored.
Bindings select a pickle and enumerate every start; there is deliberately no
caller-supplied list of attempt IDs that could cherry-pick successful retries.

## Verified phase-end evidence

The actual first execution reached all25 scenarios, but the harness could not
parse its adapter JSON schema (missing closing structure). The schema was
rewritten as readable structured JSON and parser diagnostics were labeled. No
production converter change was needed for this diagnostic. The focused rerun
passed25 scenarios/85 steps, including the actual copied canonical reader in
isolated scratch with decision-sink. Evidence:

- `evidence/phase39-second-bdd.log`: first actual run, schema parsing failure.
- `evidence/phase39-third-bdd.log`: completed PASS25/85.
- `evidence/phase39-command.txt`: exact successful command.
- `evidence/phase39-first-bdd.log`: mistyped tag selected zero scenarios;
  explicitly excluded from acceptance evidence.

The byte-for-byte canonical reader is `kbd-phase-close-eval.sh`, a script with
no separate declared semantic version. Its copied SHA-256 is
`e5b7d9e2f20a5b2352b877271b576b9cd5b9ef45de7481e17068a02320f7f403`.
It consumes `kbd.phase-disposition/v1`; the audited canonical Rust evidence
admission version is2. The copied adopted-contract v2 schema hash is
`5307cb383c06ff624b014954c54bded50d331a69947c9e1c27d98d1f010a10cc`.
The companion `kbd-closure-packet.mjs` hash is
`eca555fd84de9d2486f1c64249e6b3e8a003c3a18e332c8fb4f6103590e72a1a`.
Original absolute paths and copy identities are in the provenance JSON; no
source pack file was modified. The skills read were VLBV1.0.0 and
value-scenario-verify1.2.0. These pins describe the inspected reader, not an
assumed future reader version.

## Caller integration

During the adopted boundary packet, after the existing runner has produced its
original receipts, invoke the converter as an exact packet command:

```sh
node scripts/messages-to-boundary-results.mjs --root /path/to/project --bundle evidence/bundle.json
```

Have the existing packet runner capture stdout as a new evidence artifact. The
converter does not create or overwrite `boundary-results.json`; retain previous
artifacts and select the new file through the existing canonical evaluator's
`--evidence-file` mechanism when authorized. It does not run that evaluator.
No npm install is needed. Node must already be available to the selected packet.

Minimal caller bundle outline (replace placeholders with actual pinned artifacts
and original identifiers; these example strings are not runnable authority):

```json
{
  "schema": "cucumber.vlbv-receipts/v1",
  "contract": {"path": ".kbd-orchestrator/phases/my-phase/adopted/CONTRACT_SHA.json", "sha256": "CONTRACT_SHA"},
  "canonicalStatus": {"path": "evidence/status.json", "sha256": "STATUS_SHA"},
  "canonicalDisposition": {"path": "evidence/disposition.json", "sha256": "DISPOSITION_SHA"},
  "authorization": {"path": ".kbd-orchestrator/phases/my-phase/boundary-authorization.json", "sha256": "AUTH_SHA"},
  "candidate": {"path": "evidence/candidate.json", "sha256": "CANDIDATE_SHA"},
  "executedBy": {"harness": "actual packet harness", "version": "actual version", "model": "actual session model"},
  "budgetConsumption": {
    "source": "actual cumulative accounting receipt",
    "values": {
      "timeMinutes": {"value": 4, "class": "measured"},
      "toolCalls": {"value": 8, "class": "measured"},
      "repairAttempts": {"value": 0, "class": "measured"},
      "noProgress": {"value": 0, "class": "measured"}
    }
  },
  "runs": [{
    "id": "boundary-command-1",
    "messages": {"path": "evidence/run.ndjson", "sha256": "MESSAGES_SHA"},
    "process": {"path": "evidence/process.json", "sha256": "PROCESS_SHA"},
    "candidate": {"path": "evidence/candidate.json", "sha256": "CANDIDATE_SHA"},
    "authorizationIndex": 0
  }],
  "scenarios": [{
    "id": "SC-1",
    "bindings": [{"runId": "boundary-command-1", "pickleId": "ACTUAL_PICKLE_ID"}],
    "assertions": [{"path": "evidence/assertion.json", "sha256": "ASSERTION_SHA"}],
    "negativeControls": [{"path": "evidence/control.json", "sha256": "CONTROL_SHA"}],
    "cleanup": [{"path": "evidence/cleanup.json", "sha256": "CLEANUP_SHA"}]
  }]
}
```

The candidate artifact contains nonempty project-relative `sourceDigests` plus
`build`, `configuration`, `fixtures` and `references` arrays of pinned file refs.
The process receipt records projectId/runId/phase/contractDigest/candidateDigest,
adopted commandId, exact argv/cwd, startedAt/finishedAt and actual exitCode.
It also requires `messages: {"path": "evidence/run.ndjson", "sha256": "MESSAGES_SHA"}`
with the exact path and digest referenced by its product run or negative control.
Record that association while capturing original stdout. A missing legacy field
is BLOCKED with actionable migration guidance; do not attach an unrelated stream
retroactively. This v1 receipt-consistency correction is specified in
[Phase43](PHASE43-VLBV-CONSISTENCY.md), whose completed boundary passed 22 new
scenarios / 90 steps and reran the original 25 scenarios / 85 steps.
`authorizationIndex` selects an existing event; it never creates authorization.
A repair run additionally names `predecessor` and an existing bound B5 event.

For `assertion.json`, supply the exact adopted oracle object, actual scenarioId,
runId, testCaseStartedId/testStepId, candidateDigest, stable assertion id, status,
observed text, exercised boolean, realCollaborators/substitutedBoundaries and
pinned observation evidence. Each control names that assertionId and the adopted
negativeControl commandRef, actual expected/observed rejection, exercised and
sensitive booleans, and its own original process/Messages files and rejecting
step identifiers. Cleanup names the adopted restoration requirement, observed
restoration and its evidence. Complete field definitions are in the schema's
`$defs`; the BDD fixture producer provides a working integration example.

The CLI applies the bounded semantic/identity checks described above; it is not
a general JSON Schema validator or a replacement for canonical contract adoption.
The BDD harness validates its input against the supplied schemas. Production
receipt producers should validate their own records before submission. Neither
schema validation nor hashing proves a producer truthful: independent oracle
quality, real observation provenance and owner authority remain explicit trust
boundaries. A green test report or fabricated receipt cannot responsibly be
claimed as externally verified value.


Phase43 corrects receipt consistency within adapter v1: process receipts now
require the exact `messages: {path, sha256}` reference for both product and
control output. Legacy missing bindings are BLOCKED with migration guidance;
see [Phase43 migration](PHASE43-VLBV-CONSISTENCY.md). The original25/85 evidence
above remains historical; the consistency correction has its own feature-first
boundary and does not alter canonical writer or release ownership.


The Phase43 consistency boundary reran this historical contract successfully on
Rust 1.88: Phase39 **25 scenarios / 85 steps**, plus Phase43 **22 scenarios / 90
steps**, totaling **47 scenarios / 175 steps**, exit 0 on the first execution.
See [the shared boundary log](evidence/phase43-45-first-bdd.log) and
[Phase43's scoped evidence](PHASE43-VLBV-CONSISTENCY.md#verified-phase-end-evidence).
This regression result supplements the original Phase39 checkpoint above; it
does not retroactively expand that checkpoint's claims or certify producer truth.
