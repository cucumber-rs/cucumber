# Cucumber Messages: standard NDJSON reporting

The `output-messages` feature provides `cucumber::writer::Messages`, a standard
NDJSON envelope writer pinned to **Cucumber Messages 34.2.1**. Cucumber.js
**13.3.0** pins this protocol version and Gherkin **42.0.1** in its
[tagged package manifest](https://github.com/cucumber/cucumber-js/blob/v13.3.0/package.json).
The separate latest Gherkin **43.0.0** baseline uses Messages **35**. This writer
also supports explicitly selected protocol35.0.0 through `ProtocolVersion::V35`;
existing constructors stay34.2.1. It does not claim complete formatter parity.
See [protocol acceptance](PHASE22-PROTOCOLS.md).

The existing `output-json` / `writer::Json` feature emits the legacy Cucumber
JSON report. That format is not Messages NDJSON and cannot be substituted for it.

For the expanded source-faithful path, use `Cucumber::with_messages(output)`.
It shares parser snapshots automatically and supports the adopted tags, Rules,
descriptions, outlines and rich arguments. See
[Source-backed AST/Pickle interoperability](messages-source.md) for its verified
corpus and remaining boundaries. The original constructors described below
retain the Phase 4 restricted behavior.

## Original constructor

Enable `output-messages`, then replace the run's writer:

```rust,ignore
use cucumber::{World as _, writer::Messages};

let output = std::fs::File::create("results.ndjson")?;
MyWorld::cucumber()
    .with_writer(Messages::new(output))
    .run("features")
    .await;
```

`Messages::new` normalizes concurrent scenario events. The normalizer preserves
the timestamps carried by the runner. `Messages::raw` is available for
already-normalized inputs; `try_handle_event` returns a typed error and poisons
the writer after an error. The ordinary Writer interface fails visibly by
panicking when reporting cannot continue. I/O errors are also propagated through
that failure path. No success completion is emitted for rejected reports.

## Supported source and execution contract

- Plain English `Feature`, `Scenario` / `Example`, and feature `Background`.
  Pickle keyword types follow the [official 42.0.1 compiler](https://github.com/cucumber/gherkin/blob/v42.0.1/javascript/src/pickles/compile.ts):
  initial conjunction is Unknown, conjunction inherits the previous type across
  background/scenario boundaries, and wildcard resets the type to Unknown.
- Multiple scenarios, including concurrently executed scenarios whose events are
  normalized before reporting.
- Exact readable source text, source/document/pickle linkage, scenario and step
  AST locations, background steps referenced from each applicable pickle.
- `meta`, `source`, `gherkinDocument`, `pickle`, `testCase`, `testRunStarted`,
  `testCaseStarted`, `testStepStarted`, `testStepFinished`, `testCaseFinished`,
  and `testRunFinished` envelopes.
- `PASSED`, `FAILED`, `AMBIGUOUS`, `UNDEFINED`, and subsequent `SKIPPED` outcomes.
  Native `Step::Skipped` denotes an unmatched definition and maps to `UNDEFINED`.
  The runner stops sending step events after failure or an undefined step. Only
  after that abort is observed, remaining planned steps are reported `SKIPPED`
  at the observed scenario-finish instant, with zero duration.
- Step durations calculated from the actual start/finish event timestamps.
- Stable identifiers based on source URI and source location within a single
  run. Each stream is a standalone run; do not concatenate independent streams
  without remapping their IDs.

Source files are reread when their feature starts. The writer reparses the text
and requires equality with the runner's feature AST. This detects semantic
changes between parsing and reporting. The event model does not retain original
source bytes, so changes that produce an identical native AST cannot be detected.
Keep feature files immutable during a run. Name/line filtering or custom parser
transformations that change the Feature AST are rejected by this equality check;
filtering is compatible only when it leaves the original whole Feature unchanged.
This bounded writer does not claim general filter interoperability.
In-memory/custom parser features need
a real UTF-8 source path whose content produces the same AST.

Phase 10 extends both original and snapshot-backed constructors with native
retry attempts, terminal run outcomes, and contiguous attempt identities.
See [retry reporting](PHASE10-RETRIES.md). Phase 11 adds
[observed composite scenario hook stages](messages-hooks.md).

## Explicitly unsupported

The writer rejects Rules, tags, descriptions, non-English sources, scenario
outlines and all data tables/doc strings. Typed attachments are a separate active phase; native logs already emit
case-attempt attachments. Empty scenarios combined with
backgrounds are also rejected: the native runner executes the background, while
the official compiler omits it from an empty scenario's pickle. These boundaries are deliberate:
this native AST/event model does not yet provide enough identity and source
information to claim full fidelity for those cases. In particular argument
ordering, media types, delimiter/source locations, and official doc-string
normalization must not be guessed.

The runner does not expose the full step-definition registry or original
expression/capture metadata to writers. Optional `stepDefinitionIds` and
`stepMatchArgumentsLists` are therefore omitted; no synthetic definitions are
emitted. Consumers requiring support-code navigation or argument highlighting
need a future runner metadata extension.

An error encountered before run completion may leave an initial valid prefix,
but reporting stops without a successful `testRunFinished`. An invalid extra
event after an already completed run cannot retract previously written bytes. Consumers must require run completion before accepting a
report. Schema validation alone does not establish lifecycle completeness or
full formatter compatibility.

## Phase-end acceptance

Authored scenarios are in `tests/features/messages/messages.feature`. The only
execution command for this phase is the dedicated BDD target:

```sh
cargo test --features output-messages --test messages -- --tags @phase4
```

The harness runs real parser/runner pipelines. For supported runs it validates
every output line against the unmodified official bundled 34.2.1 JSON schema,
checks unique IDs and resolved references, asserts lifecycle pairing and measured
durations, and checks exact statuses and run success. Unsupported cases must fail
visibly without successful run completion. It does not run Rust unit suites.

Pinned schema source:
https://github.com/cucumber/messages/blob/v34.2.1/jsonschema/messages.schema.json
(Git blob `3abcc5814426479444c7a278c4e8dd72b3e0b2dd`). The schema and upstream MIT
license are vendored in `tests/fixtures/messages`; validation needs no schema
network access. Schema validation is independent of the writer implementation.

Status: verified at the phase-end boundary on 2026-11-17: **23 scenarios,
109 steps passed**, exit code 0. The first completed-phase run exposed a native
event-ordering assumption (`ParsingFinished` can precede `Started`); that was
corrected, then only this focused BDD target was rerun. No unit suite or general
`cargo test` was executed. Full logs are in
[`evidence/phase4-messages.log`](evidence/phase4-messages.log) and
[`evidence/phase4-messages-initial.log`](evidence/phase4-messages-initial.log).
