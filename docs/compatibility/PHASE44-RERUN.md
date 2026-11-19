# Phase44: programmatic rerun selection from preserved Messages

Status: implemented and verified on the first completed phase-end run under
**Rust 1.88.0: 19 scenarios / 73 steps passed**, exit 0. Target `messages_rerun`,
tag `@phase44`. The eight-target affected BDD batch is in progress; production
remains frozen.

## Pinned semantics and scope

The [JS13.3 RerunFormatter](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/formatter/rerun_formatter.ts)
selects non-PASSED attempts unless willBeRetried is true. This is independent of
run success: skipped, non-strict pending and dry-run outcomes remain eligible.
URI groups retain repeated lines, use newline by default, and add no suffix.
The [source-location helper](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/formatter/helpers/gherkin_document_parser.ts)
resolves the last Pickle AST ID to the original scenario declaration or Examples
body row, including Rules. No expanded native position is used.

The [official collector](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/formatter/helpers/event_data_collector.ts)
seeds every planned step UNKNOWN and aggregates at TestCaseFinished. The pinned
Messages34.2.1 helper also returns UNKNOWN for an empty result list. Before case
finish the aggregate remains UNKNOWN, even at an eventual run boundary. This
phase retains completed=false rather than inventing completion for interrupted
started attempts. Unknown sits below PASSED in the official severity ordering.

## Native API

`writer::messages::rerun::Collector` consumes borrowed standard-envelope Values.
`plan()` requires TestRunFinished and returns a snapshot retaining every actual
attempt: IDs, retry number/decision, original URI/line, completed flag and worst
status. The optional original run ID remains optional; no identity is fabricated
for standard streams that omit it. Run success is independent of selected cases.
`Plan::entries()` is only the derived rerun projection; `attempts()` retains the
full observed attempt denominator. Malformed relevant data or missing execution,
Pickle, document or AST references return explicit errors, never guessed lines.
This is a consumer, not a schema validator; unrelated envelope kinds are ignored.

`Plan::format(separator)` returns official-shaped grouped text. URI strings are
not shell-escaped, normalized or parsed by splitting colons. Standard numeric
object-key ordering is preserved for numeric-looking URI keys.

`Plan::local_selection(resolve_uri)` returns `Result<Option<LocalSelection>, Error>`.
None means nothing to rerun and does not call the resolver. Nonempty selection
pairs finite SourcePaths discovery with original LineSelectors; take both with
`into_parts()` and apply both builders to the next application. This avoids the
empty-LineSelectors-means-all trap. URI interpretation and authorization belong
to the application; a rejected mapping cannot become an implicit local path.

No runner changes, Messages output mutation, new CLI, automatic execution,
filesystem writes, workflow change or Rust dependency are introduced. The
optional Messages-store integration was unnecessary and was not added.

## Programmatic application recipe

This fragment assumes the caller supplies `original_envelopes`, a fresh
`configured_application` retaining its chosen parser/runtime/support-code
settings, and `resolve_admitted_uri`. The resolver maps original URIs only to
local resources the caller has admitted; returning an error stops selection.
Running the follow-up application still requires the caller's own authority.

```rust
use cucumber::writer::messages::rerun::Collector;

let mut collector = Collector::new();
for envelope in &original_envelopes {
    collector.ingest(envelope)?;
}
let plan = collector.plan()?;
let selection = plan.local_selection(|uri| resolve_admitted_uri(uri))?;

let mut follow_up_messages = Vec::new();
if let Some(selection) = selection {
    let (source_paths, line_selectors) = selection.into_parts();
    configured_application
        .with_source_paths(source_paths)
        .with_line_selectors(line_selectors)
        .with_messages(&mut follow_up_messages)
        .with_default_cli()
        .run(".") // explicit SourcePaths supplies discovery
        .await;
}
```

None launches no application. Do not substitute default LineSelectors or omit
the explicit SourcePaths component. Keep `original_envelopes` and their bound
artifacts separately from `follow_up_messages`; the new selection and any later
pass do not change `plan.attempts()` or the original result. Check
`plan.run_success()` independently: global/parser failures can leave no
scenario to select. Formatted text is for display/export, not shell evaluation.

## Independent oracle and acceptance

`tests/interop/rerun` is an isolated exact-version development fixture with lock,
MIT license copy and provenance hashes. Preparation installed Cucumber.js13.3.0
with lifecycle scripts disabled. The oracle directly instantiates its unchanged
RerunFormatter and EventDataCollector with a real EventEmitter. It has no ported
selection/aggregation/formatting algorithm. Imports and execution occur only
inside completed BDD acceptance; preparation is not runtime evidence.

The19 cases cover all seven protocol statuses (UNKNOWN uses an actually empty
completed TestCase), real recovering/exhausted cleanup-hook failures, a real
filtered concurrent Rule outline with duplicate row values, and a real second
execution through SourcePaths+LineSelectors. They also cover grouped/custom
separators with duplicate lines, interrupted UNKNOWN, incomplete streams, four
missing reference types, parse/global failures without case selection, passing
runs' explicit empty selection, and rejected URI mapping.

For VLBV, a new source selection is only a mechanism. It is not B5 authorization
and does not erase earlier failed receipts or reduce the original denominator.
Keep original process, Messages, assertions, control and cleanup evidence; a
repair binding still identifies its distinct prior admitted run and existing
bound B5 authorization. This phase does not invent those application bindings.


## Actual phase-end evidence

The [first Rust1.88 boundary log](evidence/phase43-45-first-bdd.log) records
**19 scenarios / 73 steps passed**, exit 0, with no harness or production
correction required. The actual installed JS13.3 formatter and collector agreed
with native grouping, statuses and retained attempts. Actual follow-up execution
selected precisely the failed duplicate Examples rows while original Messages
and the original three-attempt denominator remained unchanged.

The [exact shared command record](evidence/phase43-45-commands.txt) records the
three completed feature-first targets run together under Rust1.88.0. No unit
suites, generic cargo test or early build/check was used. The eight-target
affected acceptance batch passed106/460 on Rust1.88, including source selection,
retries, official consumers and relocated integration; see
[the retained log](evidence/phase43-45-affected-bdd.log).
