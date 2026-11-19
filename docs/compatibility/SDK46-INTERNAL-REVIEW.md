# Phase 46 internal implementation review

Review date: 2026-11-18. Reviewer: `/root/sdk46_review`, an independent internal Codex subagent. Exact deployed model identity is unavailable; producer/reviewer model separation is unverified. This is a harness-native internal review, not external or cross-model certification.

Scope: the first SDK phase added after `391eaeb`, against the adopted eleven-case contract with digest `a48388a624c00318a3afe449bcf7415f6f4a9a516a63b0a43742ee3c75a15dc0`. Reviewed the Rust SDK, Node and Python adapters/facades and packaging, external consumers, acceptance driver, and retained evidence. Inspected existing core implementations only to understand delegation. The full through-phase-45 core/Gherkin audit, foreign execution/adapters, Pages configuration, and publication readiness are outside this review.

No builds, tests, imports, probes, or reruns were executed. No code, specification, or review packet was sent to an external reviewer. The reviewer changed only this report. The adversarial-review skill's finding criteria were applied with the owner's explicit internal-review override; no claim is made that its external-dispatch or cross-model gates ran.

## Verdict: BLOCK pending one material correction

One CRITICAL finding, zero additional warning/suggestion findings. CRITICAL here means an adopted contract correctness blocker; this is not a claim of remote code execution or a security severity rating. The reproduction below is established by source tracing, not a newly executed behavioral result.

### SDK46-IR-01: reject JSON objects masquerading as numeric protocol values

**Locations:** `sdk/src/lib.rs:203` (request decoding), `sdk/src/lib.rs:123` (NDJSON decoding), and `sdk/Cargo.toml:21` (`serde_json/arbitrary_precision`). The current lock resolves serde_json 1.0.154.

The SDK deserializes untrusted JSON directly into `serde_json::Value` while enabling `arbitrary_precision`. In that dependency's `src/value/de.rs:121-129`, the first map key is classified, and `KeyClass::Number` is converted to `Value::Number`. Lines 1361-1364 recognize the literal key `$serde_json::private::Number`; `src/number.rs:18` defines this marker. The ordinary JSON-object parsing path in `src/de.rs` passes the map into that visitor. Consequently a caller-supplied object with that key acquires numeric type before the SDK validates the request.

For example, this wrong-type request follows the success path for protocol version 1:

```json
{"version":{"$serde_json::private::Number":"1"},"operation":"capabilities","input":{}}
```

The adopted contract requires `INVALID_REQUEST` for a noninteger version. Its schema also rejects an object in that position. `Sdk::dispatch` instead sees a number and accepts it. The raw Rust, Node, and Python request interfaces all share this defect. Passing such a map through a host-language JSON serializer does not remove the issue.

The same decoding is used for NDJSON. Replacing a legitimate numeric `testCaseStarted.attempt` with `{"$serde_json::private::Number":"9007199254740993"}` is converted into an integer accepted by the collector's `as_u64`, rather than rejecting the wrong-type field with `INVALID_MESSAGES`. This undermines strict input/evidence validation despite preserving numeric precision for valid inputs.

**Correction:** preserve and validate the original JSON token types before the special-marker-aware `Value` conversion. A raw-value-backed request decoder can validate the actual version token and existing string/object fields; the NDJSON path needs equivalent protection for numeric fields and nested marker objects. Do not use a blanket text-substring ban: valid source text, URI text, doc strings, and unrelated string values may legitimately contain the marker. Do not fix the problem by using floating-point normalization, which would lose the separately adopted large-integer guarantee. The correction belongs in the shared Rust boundary rather than duplicated Node/Python validators.

**Required focused boundary proof:** extend the existing SDK-005 wrong-type controls with the raw request above; extend SDK-007 with a real run whose numeric attempt is replaced by the marker object, expecting `INVALID_MESSAGES` and its physical line context. Exercise all three installed raw interfaces, retain the >2^53 legitimate integer case, and retain successful compilation when the marker appears as ordinary source string content. Keep acceptance authoring before implementation and execution at the completed repair-phase boundary. Preserve earlier receipts; record a fresh packet. Have the internal reviewer inspect the actual correction and retained proof before closing this finding.

**Disposition at report creation:** OPEN; concrete finding and recommendation delivered to the parent. No implementation changed by this reviewer.

## Evidence assessed

`docs/compatibility/evidence/sdk46-repair1/functional-bdd.log` records **11 scenarios / 33 steps passed**; `controls-bdd.log` separately records **11 scenarios / 33 steps passed**. `cleanup-complete.json` records removal of the disposable consumer root after the complete receipt set. Runtime receipts show Rust 1.88.0, Node 26.5.0, and Python 3.11.3; the preparer admits Darwin arm64 explicitly. These are retained results read by the reviewer, not tests rerun during review.

The earlier `sdk46-first` packet retains its Node 24 admission mismatch. It is not accepted as the adopted runtime proof and has not been silently replaced.

The source-graph comparator requires matching complete structure and a bijective ID mapping, checks reference mappings, and does not normalize source locations or URI fields. Expected fixture graphs come from the pinned upstream corpus. Markdown edge cases invoke the pinned official oracle. Rerun checks distinguish original attempt identities and run success from the actual Cucumber.js formatter/collector projection. Deliberate expected-output mutations are rejected by the comparator; native-error controls exercise production entry points. The passing control set currently lacks the numeric-marker input described above.

Package evidence uses installed npm/wheel facades and native artifact paths outside the checkout, including fresh-process failure after removing disposable native artifacts. The TypeScript consumer is compiled and executed within SDK-010. Rust consumption is an external Cargo project with a path dependency on this checkout; it is not proof of a published or independently distributable Rust package. Source/build/native/archive identity receipts are retained. This review did not independently recompute every oracle inventory hash; the driver performs those checks before and after each case.

## Boundaries retained

The implementation delegates compilation and rerun semantics to the existing Rust core and does not duplicate those algorithms in the language facades. Rust extensions are separately dispatched and instance-owned; core names and duplicate registrations are refused. No foreign callbacks or foreign step execution are advertised. Documentation correctly treats framework baselines as targets rather than complete Behave, pytest-bdd, or Cucumber.js parity.

The proof is limited to the adopted Darwin arm64 runtime versions and local package artifacts. It does not establish all supported platform/Python versions, source distributions, public package identities, full dependency-license readiness for distribution, framework adapters, or registry publication. Names remain private/local placeholders. Existing README/schema planning-status prose needs normal final reconciliation after accepted proof; this is not permission to alter the signed historical contract or canonical state.

Ordinary input-error recovery and the declared package/native boundaries were exercised. Exhaustive panic behavior, concurrency, resource exhaustion, every possible malformed Messages envelope, and public-distribution matrices were not established by this packet. No broader clean-codebase claim follows from this bounded review. The programme's final core/Gherkin audit remains pending.

## Reviewed snapshot

HEAD at inspection: `63f14ff7a1ccf150723d98be6a07a1f4c1cb9a7a`, with existing working-tree changes preserved.

| File | SHA-256 |
| --- | --- |
| `sdk/src/lib.rs` | `c04eee1cb5befe37729ce23bcad8d1ebce0aed4db80f5e00102993b8033a363b` |
| `sdk/Cargo.toml` | `e1eee141f51d8be6aae54ad39bd29db918b45551025d0a434d7ed84ae13e8850` |
| `bindings/node/src/lib.rs` | `d051d87dcc70fda1f99ad023d6b02d0c8423905a56d8f501f3589dab130e5109` |
| `bindings/node/index.js` | `5dfeac957dd0132b5100f1bf7cabc74b7efe7a41a8c7424e0ed77f7f1bdb19c2` |
| `bindings/python/src/lib.rs` | `53f13e620e947316fe2f80df09fd69abceef7d901c523ece7dda4891fbf4a23b` |
| `bindings/python/python/cucumber_native_local/__init__.py` | `24a84210b1538c00ada8eb47c1087f29a2c5ab3567afa891fcab06d3d65c8cc1` |
| `sdk/acceptance/cases.py` | `46bade28e302175795fb0e69fec1b01e14f6d6d1b3d6aa34f44b1f80b4bb39ad` |

## Repair-2 follow-up — SDK46-IR-01 resolved

Follow-up date: 2026-11-18. Same independent internal reviewer and identity limitations. **Current bounded review verdict: PASS; zero open material findings.** The original BLOCK verdict and source snapshot above remain as historical evidence. This disposition resolves SDK46-IR-01 only; it does not declare the complete programme/core/Gherkin codebases clean or supersede the remaining final audit.

The correction introduces `sdk/src/json_input.rs`, used by both `Sdk::dispatch` (`sdk/src/lib.rs:204`) and `Sdk::rerun` (`sdk/src/lib.rs:124`). It first retains original JSON using `RawValue`, then selects deserialization from the actual token. Objects are decoded as maps of raw values and explicitly constructed as `Value::Object`; arrays recurse through raw elements. Only original numeric tokens enter `Number` deserialization. Consequently neither the arbitrary-precision private marker nor the enabled raw-value marker changes an actual object's type. Strings use typed `String` deserialization and are not searched for reserved substrings. Exact number spelling is retained; no floating-point conversion was introduced. A 128-container nesting guard bounds the new recursive materialization.

Read the actual retained repair-2 source diff and positive/control receipts for SDK-005 and SDK-007. SDK-005 sends the original forged-version request through each installed Rust/Node/Python raw interface, receives `INVALID_REQUEST`, and successfully handles the next request in the same host. SDK-007 changes an attempt in a real runner stream into the forged-number object; each interface returns `INVALID_MESSAGES`, then successfully processes the original stream. Retained positive raw outputs independently show physical NDJSON line `"9"` for that rejection:

- `evidence/sdk46-repair2/SDK-005-positive/0004-stdout.txt` through `0006-stdout.txt` and their process records: Rust, Node, Python version rejection and recovery, all child exits zero.
- `evidence/sdk46-repair2/SDK-007-positive/0002-stdout.txt` through `0004-stdout.txt` and their process records: Rust, Node, Python attempt rejection with line context and recovery, all child exits zero.
- `evidence/sdk46-repair2/SDK-005-control/receipt.json` and `SDK-007-control/receipt.json`: the same added rejection/recovery assertions executed again within the retained control packet and passed.
- `evidence/sdk46-repair2/SDK-006-positive/receipt.json`: legitimate attempt `9007199254740993` remains exactly represented; the earlier precision requirement was retained.

The complete repair packet's retained logs record **11 functional scenarios / 33 passed steps** and **11 control scenarios / 33 passed steps**. Its cleanup receipt records removal of the disposable consumer root. The additional outer Messages proof is retained in `evidence/sdk46-repair2/outer-messages-proof.json`; that reporting correction is tracked by the parent and does not replace the raw child observations used to resolve this source finding.

One recommendation is supported by source review rather than a dedicated new case: literal private-marker text inside a source string was not separately added to this packet. The correction performs typed string decoding and no marker-name filtering, so this does not leave the reported object-to-number coercion unresolved. The report makes no claim that this extra string-content case was executed. Similarly the driver asserts the error class, while the physical error line is confirmed here from retained raw output rather than a new line-equality assertion.

This follow-up performed no builds, imports, tests, probes, or reruns and transmitted nothing externally. It was limited to the correction, its callers and affected retained evidence. Three follow-up tool calls were used: two read-only inspection calls and the report append/read call. Exact producer/reviewer model separation remains unverified.

Correction snapshot SHA-256:

| File | SHA-256 |
| --- | --- |
| `sdk/src/json_input.rs` | `4bc24a768c4fda7fc3c617fb8cf40776b3b1f6c2b9b1a191abacde14e0c0d54f` |
| `sdk/src/lib.rs` | `725dca9787e2c51e5cbb52c463d805356af969b2ffcd3f46c8369e09c6312c8f` |
| `sdk/Cargo.toml` | `097216a83f9be2cce1348afee022f5ac4fa83937c9451a764a9c3dfd19eecb60` |
| `sdk/acceptance/cases.py` | `0e2d9b2f4cd8314eb9e166d939c98aa22a348529ccffbdb0600013ad215642cc` |
