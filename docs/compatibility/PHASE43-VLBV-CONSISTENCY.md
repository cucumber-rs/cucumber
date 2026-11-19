# Phase43: complete and associated VLBV receipts

Status: all 22 feature-first scenarios passed at the first completed phase-end
boundary: **22 scenarios / 90 steps**, Rust 1.88, exit 0. The same target
reran Phase39 successfully: **25 scenarios / 85 steps**, giving **47 scenarios /
175 steps** total. This is an adapter v1 receipt-consistency correction,
not a new canonical schema, runner, authority or build/release process.

The finite contract covers orphan/reversed lifecycle records, invalid retry
chains, self-referencing repairs, concealed parse/global/exception failures,
product/control stream substitution, valid parallel interleaving, and malformed
or conflicting explicit receipts. Existing Phase39 cases remain regression
coverage for actual product failures, unexercised obligations, ordinary retries,
authorized repairs, producer trust and canonical-reader compatibility.

Process receipts require `messages: {path, sha256}` naming the exact NDJSON
artifact captured by that process. The converter compares it with the
product/control stream reference. Legacy receipts without that binding must be
BLOCKED with a migration diagnostic, never treated as PASS. Producers should
write this field while capturing the original process output; the converter must
not invent it from an unrelated supplied stream. The bundle and canonical output
remain v1. Hashes establish byte association, not authenticity or observation
truth. Existing authority records and candidate binding remain unchanged.

Lifecycle validation enforces pairing and causal order per invocation while
allowing independent scenarios to interleave. Retry attempts must start at 0,
increase contiguously, and agree with boolean willBeRetried promises. Repair
predecessors must be distinct already admitted runs, rooted in an existing B4.
Optional protocol run references are checked when supplied rather than made
new universal requirements. Parse/global failures and run exceptions cannot be
hidden by success=true. Original evidence and prior attempts remain retained.

Assertions require a boolean exercised field and an actual bound attempt/step.
Duplicate control or cleanup associations are refused rather than choosing the
first receipt. These are bounded semantic checks; the converter does not become
a general JSON Schema validator or certify a producer's claims.


## Receipt v1 migration

When capturing a process's original NDJSON stdout, place its exact artifact
reference in the same immutable process receipt:

```json
"messages": {"path": "evidence/run.ndjson", "sha256": "ACTUAL_64_HEX_DIGEST"}
```

Use that same reference in the product run or negative-control receipt. A legacy
process receipt missing the field receives BLOCKED with `Messages artifact
binding missing: record process.messages {path, sha256} when capturing original
stdout`. Do not retroactively bind an unrelated stream merely to obtain PASS;
retain the old receipt and create a new capture receipt under existing authorized
execution. No adapter/bundle/canonical schema version is replaced, and old
unbound receipts remain insufficient evidence rather than silently accepted.
The two stream-substitution scenarios also verify this migration diagnostic for
both product and negative-control processes.

The BDD harness retains original process output/receipts before creating separate
mutant artifacts. It archives complete input/output receipts under
`docs/compatibility/evidence/vlbv-adapter/boundary-*/`, without duplicating the
large executed binary (its digest is retained). The valid interleaving fixture
reorders original records from two real completed Cucumber scenarios into a
causally valid merged stream; this proves graph admission, not new runtime
parallelism. Original unmodified streams remain archived separately.

The existing `vlbv_adapter` Cargo target runs both phase39 and phase43 by default;
`--tags @phase43_vlbv` selects only these 22 cases. No new target or dependency was
needed. Only the focused BDD target ran after the full implementation and harness
were complete; no unit suite or early execution was used.

Independent source review added three scenarios before the corresponding fixes:
an extra control naming no supplied assertion, an authorized repair after a prior
unexercised run with explicitly unknown cleanup, and a repair after a failed
product assertion whose supplied control references an inconsistent process
stream. Every supplied cleanup and control is validated before product-outcome
shortcuts; absent or genuinely non-sensitive controls retain NOT_EXERCISED
semantics. These additions do not make producer testimony authentic.


## Verified phase-end evidence

The parent-coordinated Rust 1.88 boundary passed on its first execution, exit 0.
The `vlbv_adapter` target ran both feature files: Phase39 passed 25/85 and Phase43
passed 22/90, totaling 47 scenarios / 175 steps. The shared boundary log is
[evidence/phase43-45-first-bdd.log](evidence/phase43-45-first-bdd.log).
Original process streams, receipts, separate mutated fixtures and projection
outputs are retained in `evidence/vlbv-adapter/boundary-*/`.

These results verify the finite consistency contract, including migration
refusals and the copied canonical reader regression. They do not establish
receipt authenticity or universal VLBV acceptance. The subsequent affected-target
BDD batch is tracked by the parent separately and is not included in these counts.
