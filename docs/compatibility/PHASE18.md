# Phase 18: native scenario logs in Messages

The Messages writer converts each observed native `Scenario::Log(String)` into
one Attachment with exact body, `IDENTITY` content encoding,
`text/x.cucumber.log+plain`, native event timestamp, and the actual
`testCaseStartedId`. It preserves retry boundaries and normalized concurrent
scenario attribution. This supports the existing tracing pipeline; it does not
introduce a typed attachment API or binary/file/stream attachment support.

The log media type matches the tagged official
[Cucumber.js 13.3.0 attachment manager](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/runtime/attachment_manager/index.ts#L53).
Envelopes use the existing pinned Messages 34.2.1 schema.

## Attribution limits

Native log events identify only a scenario and retry attempt. After-hook logs
arrive before their Hook::Started event and may arrive before the failed body's
deferred Step::Failed event. An apparent active step is therefore not reliable
origin evidence. The writer deliberately omits testStepId and hook identifiers.

Native unscoped tracing events are broadcast to every active scenario by the
existing collector. Each observed copy becomes an attachment for that scenario
attempt. This preserves native fan-out; it does not claim a unique originating
scenario.

Timestamps are the native runner event timestamps (when the log enters that event
stream), not an invented timestamp extracted from arbitrary formatted log text.
Logs outside the active scenario or with inconsistent retry identity are rejected.

Official Query's step-attachment lookup requires testStepId. Step-specific
retrieval is therefore deferred until the native event protocol can reliably
carry that identity; this phase does not attach logs to guessed steps simply to
make that consumer API return them. General binary/file attachment support,
attachments after scenario completion and run-hook attachments remain separate.

## Feature-first acceptance

Five BDD scenarios were authored before production edits. Each launches a real
application in a separate process because tracing initialization is global. The
application emits actual tracing events, and a separate native-event writer
records their body, timestamp, scenario name and retry metadata. Acceptance
compares these observations bijectively with emitted attachments.

Modes cover Unicode/multiline text, before/after hooks, recovered retry, concurrent
recovered/exhausted scenarios and native unscoped broadcast. Barriers coordinate
concurrent work without timing sleeps; a timeout bounds deadlocks. A failed
attempt's after-hook log demonstrates why active-step inference is misleading.

Source envelopes, schema validation, final success, exact log counts and
case-attempt boundaries are asserted. No synthetic log event is used as the
positive execution oracle.

Phase-end command:

    cargo test --test messages_logs --features output-messages,tracing -- --tags @phase18

Result: **5 scenarios / 30 steps passed**, exit 0, with 26 actual observed logs
across 10 scenario attempts. [Passing log](evidence/phase18-messages-logs-bdd.log).
The recovered retry explicitly confirmed that its after-hook log arrived while
native event ordering still appeared to have an active body step.

Two failed phase-end attempts are retained transparently. The first exposed a
harness macro function/argument name collision and failed compilation. The second
ran all five applications, then exposed the recorder's incorrect string encoding
of numeric timestamp seconds. Both corrections were confined to the acceptance
harness; the production Attachment implementation did not change between runs.
[First attempt](evidence/phase18-messages-logs-bdd-first-attempt.log),
[second attempt](evidence/phase18-messages-logs-bdd-second-attempt.log).
No unit suites, generic cargo tests, lint or premature verification ran.
