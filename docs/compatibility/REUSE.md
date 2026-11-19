# Reusing this Rust BDD foundation

Use a complete, pinned checkout of this fork as a path dependency. The manifest
uses its checked-in `vendor/gherkin` parser, so consuming projects do not need the
maintainer's sibling parser checkout or a workspace `[patch]` override. Keep
`src`, `codegen`, `vendor`, and their manifests together; copying the whole
checkout is simplest. Record its revision and your application lockfile. The
manifest declares Rust1.88; the final selected BDD acceptance passed172 scenarios/
670 steps on that compiler, including a relocated external consumer.
See [the exact verification](MSRV-ACCEPTANCE-PLAN.md).
The verified through45 implementation is preserved at commit
6e4753fc0783038bb388de1b4d76055edc697b8f on the origin branch
foundation/rust-bdd-sdk. Pin that commit for the implementation described here;
retain the source-digest inventory and application lockfile alongside it.

```toml
[dev-dependencies]
cucumber = { path = "../cucumber", features = ["output-messages"] }

[[test]]
name = "acceptance"
harness = false
```

The default `macros` feature supplies Rust World/step registration macros.
Your application supplies its own async entry point, World, and step callbacks.
[Relocation acceptance](../../tests/portability.rs) exercises a separate consumer
with this layout. This is a pinned-checkout integration. These changes are present on the remote
fork, but are not a published crates.io release.

## Select behavior explicitly

The existing defaults remain available. These choices are independent:

| Need | Opt-in and tested reference |
| --- | --- |
| Adopted Gherkin43 syntax, dialects, arguments, outlines and empty-document/background semantics | `.with_gherkin43()?`; [featureless documents](featureless-documents.md) and [lexical semantics](lexical-semantics-audit.md). This does not select every Cucumber.js runtime policy. |
| Match step text across Given/When/Then registrations | `.step_matching(cucumber::step::Matching::KeywordIndependent)`; [support-code acceptance](../../tests/messages_support_code.rs). Install the final step collection before selecting its matching policy. |
| JS pending/skipped/failure rules | `.outcome_policy(cucumber::execution::OutcomePolicy::cucumber_js())`; [execution control](PHASE30B-EXECUTION-CONTROL.md). |
| Callback deadlines | `.with_timeouts(cucumber::runner::Timeouts::cucumber_js())`; [timeout acceptance](../../tests/timeouts.rs). Legacy timeouts are disabled; JS preset is five seconds with per-definition overrides. |
| Names/tags/predicate together | `.with_filter_options(FilterOptions::new().name(regex).tag(expression))`; names OR, tags AND, caller predicate AND. [Filter guide](composable-filters.md) explains preserved legacy CLI override behavior. |
| Actual scenario/outline/Examples row locations | `.with_line_selectors(LineSelectors::new().lines(path, [line])?)`; [selection guide](line-selection-plan.md). Selectors filter discovered inputs; they do not discover missing files. |
| Definition inspection | `.execution_mode(ExecutionMode::DryRun)`; [dry-run contract](PHASE35-DRY-RUN.md). Worlds are constructed, callbacks are not executed. |
| Native follow-up selection | `writer::messages::rerun::Collector`, `Plan::local_selection`; [tested ingest/selection recipe](PHASE44-RERUN.md). Preserve all attempts and apply both SourcePaths and LineSelectors; None means no execution. |
| Markdown input | Explicit `SourceFormat::Markdown` or `Auto`; [discovery and source-format rules](markdown-runtime.md). Plain remains the default. |

Types in the last rows live in `cucumber::filter`, `cucumber::execution`, and
`cucumber::parser`, respectively. Use `.run("features")` for the established
single-root integration. Programmatic `SourcePaths` list/glob discovery is
documented in [Phase40](PHASE40-SOURCE-PATHS.md); its
[harness](../../tests/source_paths.rs) passed **13 scenarios / 44 steps** at the
completed primary boundary. Use `SourcePaths::new().path(path)` or its fallible
`.glob(pattern)` builder with `.with_source_paths(paths)` for ordered discovery.

## Report original sources and actual execution

For a Basic parser, `.with_messages(output)` installs fresh shared source and
support-code stores automatically. It preserves parser policy, emits standard
NDJSON, and publishes the final registered step definitions for supporting
runners. See the small real runner branches in
[the support-code harness](../../tests/messages_support_code.rs). Custom runners
must explicitly support registry publication; absent metadata is not invented.

For hooks or attachments, configure a raw source-backed writer, attach the same
stores to their producers, then call `.normalized()` before installing it.
Create fresh stores for each independent run. These are shared handles, not
separate copies of records:

| Records | Producer | Messages writer |
| --- | --- | --- |
| Original source/AST | `Cucumber::with_source_store(sources.clone())` | `Messages::raw_with_source_store(output, sources)` |
| Scenario callbacks | `Hooks::with_store(store.clone())` | `.with_scenario_hook_store(store)` |
| Global callbacks | `RunHooks::with_store(store.clone())` | `.with_run_hook_store(store)` |
| Typed text/binary evidence | `Cucumber::with_attachments(store.clone())` | `.with_attachment_store(store)` |
| Registered definitions | Supporting `Runner::with_support_code_store(store.clone())` | `.with_support_code_store(store)` |

The complete, exercised wiring is in
[scenario callbacks](../../tests/messages_scenario_hooks.rs),
[attachments](../../tests/messages_attachments.rs), and
[global callbacks](../../tests/messages_run_hooks.rs). Choose the writer before
calling `Cucumber::with_run_hooks`: that builder installs the lifecycle statistics
adapter around the chosen writer. Replacing the writer afterward discards that
adapter. A later `.with_messages(...)` also creates new stores, so do not use it
to overwrite a manually wired rich writer.

Prefer these source-backed constructors. Original `Messages::new`/`raw`
constructors retain their restricted source reconstruction contract; they are
not substitutes for captured raw source, outlines and immutable source generations.
The older native JSON writer is not Cucumber Messages.

Messages defaults to 34.2.1 for the pinned JS13.3 baseline. Explicit
`.with_messages_protocol(output, ProtocolVersion::V35)` selects35.0.0;
see [protocol boundaries](PHASE22-PROTOCOLS.md). Source fidelity has one documented
upstream conflict: the official Markdown fallback Feature can omit `keyword`,
although both pinned Messages schemas require it. The graph preserves the actual
field absence; [the audit](FILTERING-SCHEDULING-AUDIT.md) explains why universal
schema conformance cannot be claimed for that shape.

## Decide process success separately

Awaiting `.run(...)` with Messages does not set an application exit code. Inspect
the actual final `testRunFinished.success`, treat missing/truncated/invalid
completion as failure, preserve the original NDJSON, and return your chosen
process status. The actual producer in
[the VLBV harness](../../tests/vlbv_adapter.rs) demonstrates completion-to-exit
mapping; its parent records the real process status, stdout/stderr and timing.
Do not promote a green formatter, dry-run, or exit0 into evidence that the
application's business assertion was exercised. `run_and_exit` requires the
appropriate statistics-capable writer composition; it is not an automatic
property of a bare Messages sink.

For existing VLBV consumers, use the [Phase39 receipt adapter](PHASE39-VLBV-ADAPTER.md)
after collecting actual run, assertion, negative-control and cleanup receipts.
That separately verified read-only Node adapter needs no npm dependencies and
neither executes tests nor authorizes release or closes a phase.
[The producer recipe](VLBV-PRODUCER.md) explains how to bind actual assertion
attachments, every attempt and the original process/stream without selecting the
first case or step by array position.

## Use the compiler without running scenarios

`cucumber::writer::messages::source::SourceGraph::parse_gherkin43(uri, source, "en")`
returns a native Rust graph with `gherkin_document()` and `pickles()` payloads.
`parse_markdown_gherkin43` handles Markdown; `from_snapshot` consumes a retained
`SourceSnapshot`. These require the `output-messages` feature, do not read files
again, and do not require Node. See [the pinned corpus acceptance](gherkin-corpus.md).
The native Gherkin parser remains independently accessible through
`cucumber::gherkin` without the reporting feature.

Share feature files, wire protocols and application adapters across projects and
languages. Rust callbacks do not execute JavaScript support files. Keep browser
adapters outside this foundation: the existing `agent-browser` candidate wraps
a Node/Playwright daemon and is not a pure Rust browser implementation.

Consult [live evidence](LIVE-STATUS.md) before adopting newer surfaces. The latest
completed primary boundary passed **67 scenarios / 268 steps**: Phase40 discovery
13/44, Phase38b UTF-16 table-cell columns 11/53, Phase30c preparation timeouts
17/69, Phase37b dynamic assignment 14/55, and Phase42 custom parameter name
admission 12/47.
[Original evidence](evidence/phase30c-37b-38b-40-42-second-bdd.log) records those
results. The shared 32-target affected regression batch passed586 scenarios/2460
steps ([evidence](evidence/phase30c-37b-38b-40-42-affected-bdd.log)). Generated
seed replay41 passed10/45 with affected69/282; Rust1.88 passed172/670.
Latest phases43–45 passed87/348 on Rust1.88, including prior VLBV cases; eight
affected contracts passed106/460. Receipt v1 producers must now capture the exact
process.messages artifact binding; see [migration](PHASE43-VLBV-CONSISTENCY.md).
Full Cucumber.js parity is not claimed; [delivery limits](DELIVERY.md) remain explicit.
