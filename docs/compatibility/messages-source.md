# Source-backed AST and Pickle interoperability

This document records the source-backed path introduced in Phase6. Current coverage
includes the full pinned plaintext corpus, Markdown, localized sources, exact
line selection, definition/capture metadata, and typed attachments. See the
[current matrix](CAPABILITIES.md) and [live phase evidence](LIVE-STATUS.md).
The original Phase6 acceptance section below is historical.

Phase 6 adds a public `writer::messages::source::SourceGraph` representation and
snapshot-backed standard NDJSON reporting. It is pinned to Messages 34.2.1 and
differentially targets the adopted plain-text English Gherkin 43 fixture corpus.
This is a bounded conformance claim, not complete grammar/runtime parity.
[Phase 8 localized source graphs](messages-localized.md) extend the same converter
using the captured Legacy/Gherkin43 vocabulary and language, with additional
French, emoji, Malayalam and Chinese acceptance evidence.

## Integration

With the `output-messages` feature:

```rust,ignore
MyWorld::cucumber()
    .with_messages(std::fs::File::create("results.ndjson")?)
    .run("features")
    .await;
```

`with_messages` installs a shared `SourceStore` in the Basic parser and Messages
writer. The parser captures exact bytes and the unexpanded native Feature at its
single file read. Reporting uses the captured snapshot and performs no second
filesystem read. Explicit integrations can share `SourceStore` with
`Basic::with_source_store` / `Cucumber::with_source_store` and
`Messages::with_source_store(output, store)`; raw event consumers can use
`Messages::raw_with_source_store`. Use a fresh store for each independent run.

For other language adapters/reporting applications, call
`SourceGraph::from_snapshot(uri, &snapshot)`, then read `gherkin_document()` and
`pickles()` as standard JSON payloads without executing any support code. The
Rust wrapper retains its internal execution mapping separately from those
protocol values. It is intentionally independent of browser adapters.

## Adopted source contract

The new opt-in path handles English tags (including inheritance), Rules, feature
and Rule Backgrounds, exact descriptions/comments, scenario outlines/multiple
Examples, table cells and escaped values, doc-string delimiters/media types,
normalized content, and both table/doc-string argument orders. Positions are
Unicode character columns from original source, and example identities refer
to the original table row. Pickle argument indexes are present only when both
arguments exist. The official Messages 34.2.1 schema already represents these
fields; no fields are discarded from the Gherkin 43 reference fixtures.

The graph contains all captured source scenarios and Pickles. Runtime filtering
selects which `testCase`s execute, without deleting the unexpanded source graph.
Each executed scenario is matched to a native expansion candidate derived from
the raw snapshot, with source Examples rows paired in native expansion order
and their values checked. Synthetic native expanded line numbers are used only
inside native identity comparisons; they never become reported source row
locations. A transformed scenario with no unique captured candidate fails
visibly instead of receiving an invented reference.

## Representation versus native execution

The source graph exposes official normalized doc-string content and media type.
The existing native handler still receives its legacy `gherkin::Step` payload:
its doc-string may include delimiter-adjacent newlines, the media prefix, and
native indentation. The runtime BDD explicitly asserts that native representation
while separately asserting normalized protocol output. Graph equality does not
establish argument-value execution parity. Phase 9 adds `.with_docstring_semantics(gherkin::DocStringSemantics::Gherkin43)`
to opt into official handler content without changing the legacy default. See
[docstring acceptance](PHASE9.md). Source graphs retain normalized protocol
content under either policy; exact source remains unchanged.

Native step-text and example-table discrepancies against official interpolation
fail conversion. Native parser acceptance remains a prerequisite for a captured
snapshot. The source converter does not turn rejected grammar into executable
features.

## Remaining boundaries

- Localized conversion uses the captured catalogue and is covered by the
  adopted Phase 8 cases; complete conformance across every registered dialect
  is not claimed. The native `formal` extension is outside the JSON catalogues.
- Syntax/expansion failures now emit standard parseError envelopes and captured
  source; read failures produce an unsuccessful run without fabricated source.
  See [Phase17](PHASE17-INPUT-ERRORS.md).
- Native logs now emit case-attempt Attachments preserving exact bodies/timestamps.
  Step IDs are omitted because native event ordering cannot identify them reliably.
  Typed/binary attachments are available through the explicitly shared store
  ([Phase23](PHASE23.md)); see [Phase18](PHASE18.md) for native logs.
  Phase 10 adds native [retry attempts and final outcomes](PHASE10-RETRIES.md).
  Phase 11 adds [observed composite scenario hook stages](messages-hooks.md).
- Empty Pickles now omit backgrounds; opt into
  `runner::BackgroundSemantics::Gherkin43` for matching execution. Legacy execution
  remains default and a mismatched background event fails visibly. See
  [empty-scenario acceptance](messages-backgrounds.md).
- The final registry is available through the shared support-code store (installed
  by `with_messages`). Original builtin expressions, custom descriptors and
  caller-declared raw capture mappings have separate Phase27/32 acceptance.
  Unknown metadata stays omitted rather than fabricated.
- The original `Messages::new`/`raw` constructors retain their earlier restricted
  reread-based behavior. Rich support requires the shared-snapshot path.

## Phase-end acceptance

Scenarios were authored before implementation in
`tests/features/messages_source/messages_source.feature`. The target is:

```sh
cargo test --features output-messages --test messages_source -- --tags @phase6
```

Eight untouched official fixture triplets compare complete AST and Pickles using
only a bijection of IDs/references and URI normalization. Locations, values,
optional fields, argument indexes, descriptions and reference integrity remain
part of equality. Real-run scenarios cover selected outline rows with comments,
source deletion/no final newline, and missing-snapshot rejection. Every emitted
graph/report envelope is independently validated against the pinned schema.

Fixture provenance and license are in `tests/fixtures/gherkin43/README.md` and
`manifest.json`.

Verified phase-end result on 2026-11-17: **12 scenarios / 49 steps passed**,
exit 0. All eight untouched official AST/Pickle fixture pairs matched exactly
under the stated ID/URI bijection. The initial boundary attempt exposed two
BDD harness compilation issues (a macro name shadow and an erased generic input
type); those were fixed without changing fixture expectations or converter
behavior. The focused target then passed. Because the shared writer candidate
mapping changed, the existing Messages target was also rerun at this boundary:
**23 scenarios / 109 steps passed**. No unit suites or general Cargo test ran.

Evidence:
- [`phase6-messages-source-bdd.log`](evidence/phase6-messages-source-bdd.log)
- [`phase6-messages-source-initial.log`](evidence/phase6-messages-source-initial.log)
- [`phase6-messages-legacy-bdd.log`](evidence/phase6-messages-legacy-bdd.log)
