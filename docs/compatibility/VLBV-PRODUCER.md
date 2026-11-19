# Producing VLBV receipts from a native application

This recipe connects the existing native runner to the read-only receipt adapter.
It does not execute or authorize a boundary. Use the project's adopted packet and
existing B4/B5 authorization; retain its original contract and candidate snapshots.
The application owns the actual oracle, negative control and restoration checks.

## Capture evidence at the actual assertion

Give the application World an `AttachmentContext` via `World::new_with_context`.
Use one fresh `AttachmentStore` shared by the runner and Messages writer, as in
[the exercised attachment producer](../../tests/messages_attachments.rs). The
runner binds each attachment to the current attempt and step. This avoids choosing
the first case or step from a report, which is unsafe for outlines, retries and
concurrent scenarios.

Inside the real assertion callback, record an application-defined JSON attachment
before reporting its result. This fragment assumes `world.evidence` is that context
and that the application already obtained `actual` and `expected` through its
adopted oracle. It is a recipe fragment, not a complete standalone executable:

```rust
let status = if actual == expected { "PASS" } else { "FAIL" };
let observation = serde_json::json!({
    "scenarioId": adopted_scenario_id,
    "assertionId": adopted_assertion_id,
    "status": status,
    "actual": actual,
    "expected": expected,
});
world.evidence.scope()?.attach(cucumber::attachment::Attachment::text(
    observation.to_string(),
    "application/vnd.example.boundary-observation+json",
))?;
assert_eq!(actual, expected);
```

The private media type and body are application choices; Cucumber does not infer
canonical authority or acceptance from them. Preserve a failed observation before
the callback panics. An attachment from a setup hook is not automatically evidence
that a required application assertion executed. Existing attachment acceptance
covers concurrent attempts, retries, hook scopes and closed-handle rejection.

## Keep bytes, process status and identities together

Configure a source-backed Messages writer with the same attachment store and,
when used, the actual scenario/global-hook stores. The complete wiring is in
[REUSE.md](REUSE.md#report-original-sources-and-actual-execution). Capture the original
NDJSON bytes without sorting, filtering, regenerating IDs or normalizing whitespace.
Keep diagnostic stderr separately. Await `.run(...)`, then inspect completion and
return an appropriate process exit code; awaiting `.run` alone does not set one.
[The actual child producer](../../tests/vlbv_adapter.rs) demonstrates this boundary.

The existing parent packet runner records actual argv, cwd, timestamps, exit code,
signal, command identity, candidate and contract bindings. Hash the captured stream
bytes and store the same artifact reference in both `run.messages` and the process
receipt's required `messages` field:

```json
"messages": {
  "path": "evidence/original-run.ndjson",
  "sha256": "ACTUAL_SHA256_OF_ORIGINAL_BYTES"
}
```

Record a new receipt for each real execution. Do not relabel an earlier process
receipt after substituting another stream. Missing legacy binding is blocked with
migration guidance; a hash binds bytes, not the truth of a producer's testimony.
The process receipt still needs all fields in
[the adapter schema](phase39-receipts.schema.json#/$defs/process).

## Resolve every assertion through explicit Messages references

For each application observation attachment, resolve this chain by exact IDs:

| Observed reference | Required resolution |
| --- | --- |
| Attachment `testCaseStartedId` | The actual TestCaseStarted, including its `attempt` number |
| TestCaseStarted `testCaseId` | The actual TestCase |
| TestCase `pickleId` | The source Pickle; bind its ID to the adopted scenario explicitly |
| Attachment `testStepId` | That TestCase's TestStep and this attempt's TestStepFinished |
| Pickle `astNodeIds` and `uri` | Original scenario/Examples row identity when source-based mapping is needed |

Use the private body to identify the adopted assertion and its observed value,
then create the explicit assertion receipt required by the adapter. Preserve the
original attachment/stream as pinned evidence. Ensure its PASS/FAIL agrees with
the bound invocation result and record the actual oracle, collaborator boundaries,
candidate and `exercised` boolean. Reject missing or ambiguous associations; do not
infer IDs from scenario titles, row values, array position or synthetic expanded
scenario locations. All starts of a bound Pickle belong in the attempt history.

Negative controls have their own actual process/Messages pair and rejecting
assertion identity. Cleanup receipts report the observed restoration requirement.
Neither can be synthesized from a green product step. Every supplied receipt must
remain associated and consistent, including prior failed runs and later repairs.

## Project, inspect and select a follow-up

Submit the preserved bundle to the existing
[adapter invocation](PHASE39-VLBV-ADAPTER.md#caller-integration). Capture its derived
JSON as a new artifact using the packet runner. It neither runs the canonical
writer nor overwrites earlier evidence. The deterministic v2 profile is the
implemented boundary; unsupported profiles are blocked.

The native Messages rerun API provides a separate source selection for a follow-up
run. It retains every original attempt, resolves original source locations and
returns `None` for no selection. A selection is not B5 authorization. A legitimate
repair still needs the existing authority and explicit predecessor lineage, with
all original failed attempts and receipts retained. See the [verified Phase44 rerun guide](PHASE44-RERUN.md) and
[live results](LIVE-STATUS.md).

This recipe uses the existing exercised attachment and process APIs. It does not
claim a new end-to-end application certification or independent receipt authenticity.
