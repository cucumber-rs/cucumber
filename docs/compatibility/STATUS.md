# Current Rust BDD interoperability status

Foundation checkpoint 2026-11-17; SDK/docs continuation 2026-11-18. All bounded phases through45 have primary and affected root BDD evidence.
Latest Rust1.88 boundary passed87/348; eight affected targets passed106/460.
The earlier13-target Rust1.88 checkpoint passed172/670 and remains historical evidence.
See [the delivery record](DELIVERY.md) for practical reuse, source provenance and
remaining interoperability limits.
**Full parity/conformance is not complete.** Current work and precise remaining
contracts are in [NEXT-PHASES.md](NEXT-PHASES.md); [live counts](LIVE-STATUS.md)
separate verified phases from remaining interoperability boundaries.

Baselines: Cucumber.js 13.3.0, its Gherkin 42.0.1 and Messages 34.2.1, alongside
latest official Gherkin 43.0.0 (Messages 35). [Pinned matrix](CAPABILITIES.md).

The foundation is preserved on both origin branches. **SDK46 is canonically
CLOSED PASS** for the adopted shared compiler/rerun and Rust-extension boundary.
Final repair2 built and installed actual local native packages and passed
11 functional cases /33 steps plus11 negative controls /33 steps, with outer
Messages proof for all22 cases/66 steps and owned cleanup. The verified host is
Darwin arm64, Rust1.88.0, Node26.5.0, CPython3.11.3 and TypeScript6.0.3.
Scoped internal review and static QA resolved their material findings; this is
not the final audit of the whole core/Gherkin programme or a full-parity claim.
See [SDK46 results and retained repairs](PHASE46-SDK.md) and [SDK roadmap](SDK-ROADMAP.md).

Both documentation sites are live:
[Cucumber](https://gqadonis.github.io/cucumber/) and
[Gherkin](https://gqadonis.github.io/gherkin/). Their separate [Pages delivery](DOCS-PAGES.md)
records local10/64 and public4/22 BDD evidence plus the repository About updates.
These counts apply to the recorded source/deployment snapshots.
[Behave and pytest-bdd](PYTHON-TARGETS.md) remain researched, unimplemented adapters;
foreign step execution, other platforms and public package publication are later gates.

## Available locally

- `cucumber_sdk::Sdk` provides shared native compilation, lossless rerun plans,
  capability reporting and versioned JSON requests; instance-owned Rust
  `Extension` registration does not replace built-ins or expose host callbacks.
- Actual NAPI-RS Node/TypeScript and PyO3 Python artifacts were installed into
  fresh outside-checkout consumers at the SDK46 boundary. Local archives are
  retained under ignored `evidence/sdk46-repair2/artifacts/`; identity hashes,
  realpaths and process receipts remain in retained evidence. Packages use
  provisional private names and are not registered or published.
- Native Messages rerun plans retain every attempt and produce safe paired source/line selections, independently compared with the actual JS13.3 formatter. An empty plan never launches an unrestricted rerun.
- VLBV receipt projection validates lifecycle/retry chains, exact process-to-stream binding, and every supplied control/cleanup association. It remains read-only and does not authorize repairs.
- Markdown literal tag-like content now follows official state-aware token precedence, including consumed argument slots and genuine declaration tags.

- Keyword-independent matching is explicit; keyword-specific remains default.
- The relative-path `vendor/gherkin` snapshot supports one table and docstring
  in either order and preserves unknown table escapes. No sibling checkout or
  machine-specific Cargo patch is required by consumers.
- `DialectCatalog::Gherkin43` selects the pinned 80-dialect catalogue; Legacy
  remains default. `DocStringSemantics::Gherkin43` independently selects adopted
  official handler content/dedent/line endings and outline argument substitution.
- `SourceStore` captures the exact parsed bytes and raw pre-expansion AST once,
  including the selected policies. Retained snapshots survive file deletion,
  rewrites and subsequent generations.
- Public `writer::messages::source::SourceGraph` returns official AST/Pickle JSON
  without executing support code. Full plaintext corpus acceptance covers all50 valid official43 source/AST/Pickle
  pairs and rejects all12 invalid fixtures, with only ID/reference and URI
  normalization. Featureless documents are preserved without fake Features.
- `Cucumber::with_messages(output)` shares source capture with the standard
  Messages 34.2.1 NDJSON writer. Rich arguments, tags, Rules, descriptions,
  outlines, original example-row locations and filtering are represented.
  Retry attempts and final outcomes are reported. Native before/after composite
  stages are Hook/TestSteps; individual callbacks are not falsely represented.
  One attempt is buffered to emit its full TestCase before execution envelopes.
  Explicit Messages35 output, actual step definitions/matched IDs, global-hook
  lifecycle and typed text/binary scoped attachments are also verified.
- Further explicit policies now include `feature::ExampleSemantics::Gherkin43`
  (official header-order interpolation, unknown tokens retained),
  `gherkin::TableSemantics::Gherkin43` (reject ragged rows), and
  `runner::BackgroundSemantics::Gherkin43` (omit backgrounds for empty scenarios).
- Syntax/expansion errors retain exact captured source and native locations;
  read errors complete the run unsuccessfully without fabricated source.
  Native logs are exact case-attempt Attachments, with no inferred step IDs.
- Named `runner::Hooks` composes setup in registration order and teardown in
  reverse, continues teardown after panics, and supports inherited tag filters.
  Legacy `.before` / `.after` retain replacement semantics.
- Existing native async scheduling has bounded evidence for limits, actual
  overlap, World isolation, inherited serial exclusivity, fail-fast and retries.
- Actual official JavaScript Query 16.0.0 and JUnit formatter 0.15.0 consume rich
  native reports and final retry outcomes. These locked Node packages are only
  development fixtures; they are not Rust runtime or browser dependencies.

Example explicit integration (enable `output-messages`):

```rust,ignore
MyWorld::cucumber()
    .step_matching(cucumber::step::Matching::KeywordIndependent)
    .with_gherkin43()?
    .with_messages(std::fs::File::create("results.ndjson")?)
    .run("features")
    .await;
```

These policies cover the documented subsets; they are not an all-parity switch.
Configure matching after replacing a step collection.

## Phase-end BDD evidence

Counts are scenarios / steps. Regressions and accumulated parser cases are not
new coverage. Expected nested application failures are asserted by passing outer
BDD scenarios. Every accepted final command exited 0.

| Phase | Focused result | Evidence / contract |
|---|---|---|
| 46 installed SDK, final repair2 | 11/33 functional +11/33 controls; outer Messages22/66; canonical CLOSED PASS | [SDK46](PHASE46-SDK.md) |
| Documentation/Pages | local10/64; public4/22, separate recorded boundaries | [Pages delivery](DOCS-PAGES.md) |
| 43 receipt consistency | 22/90 new;25/85 prior | [Phase43](PHASE43-VLBV-CONSISTENCY.md) |
| 44 native rerun | 19/73 | [Phase44](PHASE44-RERUN.md) |
| 45 Markdown precedence | 21/100 | [Phase45](markdown-precedence.md) |
| 43–45 affected root contracts | 106/460 across8 targets | [Evidence](evidence/phase43-45-affected-bdd.log) |
| 1 matching + argument order | runner 20/115; parser 18/69 | [Phase1](PHASE1.md) |
| 2 portable integration | relocated consumer 1/5; runner 20/115 regression | [Phase2](PHASE2.md) |
| 3 ordered hooks | 9/42 | [Phase3](PHASE3.md) |
| 4 initial Messages | 23/109 historical | [Messages](messages.md) |
| 5 scheduling | 8/50 | [Phase5](PHASE5.md) |
| 6 exact sources + rich graph | sources 5/20; rich graph 12/49; parser escapes 24/93 accumulated | [Source capture](PHASE6-SOURCES.md), [rich graph](messages-source.md) |
| 7 dialect catalogue | parser 50/172 accumulated; sources 8/30 including prior scenarios | [Phase7](PHASE7-DIALECTS.md) |
| 8 localized graphs | localized 7/31; rich regression 12/49 | [Localized graph](messages-localized.md) |
| 9 official docstring execution | parser 60/202 accumulated; real handler integration 3/13 | [Phase9](PHASE9.md) |
| 10 retry reporting | 6/40; earlier Messages 22/105 | [Phase10](PHASE10-RETRIES.md) |
| 11 native hook reporting | 10/70; regressions Messages 21/101, retries 6/40, rich/localized 19/80 | [Hook Messages](messages-hooks.md) |
| 12 official consumers | 2/8 first-run pass | [Phase 12](PHASE12-CONSUMERS.md) |
| 13 docstring edge consistency | 15/86; rich/localized19/80 and handler3/13 regressions | [Docstrings](messages-docstrings.md) |
| 14 official outline interpolation | 9/41, first-run pass | [Phase14](PHASE14-INTERPOLATION.md) |
| 15 strict table semantics | parser73/241 accumulated; root10/38 | [Phase15](PHASE15.md) |
| 16 empty-scenario background semantics | 12/64 plus affected hooks/scheduling/Messages regressions | [Backgrounds](messages-backgrounds.md) |
| 17 input-error reporting | 6/30; source-generation regression5/20 | [Phase17](PHASE17-INPUT-ERRORS.md) |
| 18 native log attachments | 5/30 | [Phase18](PHASE18.md) |
| 19 complete plaintext corpus | 62/236; parser73/241 regression | [Corpus](gherkin-corpus.md) |
| 20 native step hooks | 10/51 | [Phase20](PHASE20-STEP-HOOKS.md) |
| 21 expressions/tags | 53/159; legacy20/115 regression | [Phase21](PHASE21-EXPRESSIONS.md) |
| 22 protocol34/35 | 3/12 | [Phase22](PHASE22-PROTOCOLS.md) |
| 23 typed scoped attachments | 9/36; affected42/227 regressions | [Phase23](PHASE23.md) |
| 24 native global hooks | 14/62; affected28/129 regressions | [Phase24](PHASE24-RUN-HOOKS.md) |
| 25 featureless documents | 11/41; affected89/347 regressions | [Documents](featureless-documents.md) |
| 26 global-hook Messages | 6/34 first-run pass | [Phase26](PHASE26-RUN-HOOK-MESSAGES.md) |
| 27 support-code metadata | 14/56; protocols/consumer5/20 regressions | [Phase27](PHASE27-SUPPORT-CODE.md) |
| 28a native Markdown parser/graphs | 21/78; affected92/357 regressions | [Markdown design](markdown-plan.md) |
| 28b Markdown runtime/source | 14/54 | [Markdown runtime](markdown-runtime.md) |
| 29 observed scenario callbacks | 18/76 | [Phase29](PHASE29-SCENARIO-HOOK-RECORDS.md) |
| 31 individual callback Messages | 13/64; shared28b/29/31 affected212/906 | [Phase31](PHASE31-SCENARIO-HOOK-MESSAGES.md) |
| 30a invocation timeouts | 14/50; shared30a/32a affected174/751 | [Phase30](PHASE30-DESIGN.md) |
| 32a builtin expression metadata | 19/85; known upstream nested capture offset deviation | [Phase32a](PHASE32A-EXPRESSION-METADATA.md) |
| 30b explicit outcomes | core20/87; consumers18/54 | [Phase30b](PHASE30B-EXECUTION-CONTROL.md) |
| 33 composable filters | 12/56; shared30b/33 affected209/1008 | [Filters](composable-filters.md) |
| 32b custom/raw capture metadata | 16/78 | [Custom metadata](PHASE32B-DESIGN.md) |
| 34 source-line selection | 38/178; without Messages37/173 | [Selectors](line-selection-plan.md) |
| 35 dry-run | core14/59; formats9/30; shared32b/34/35 affected267/1115 | [Dry-run](PHASE35-DRY-RUN.md) |
| 36 retry policy | 16/56 | [Retry policy](PHASE36-RETRY-POLICY.md) |
| 37a global order | native19/81; tracing2/16 | [Ordering](PHASE37-DESIGN.md) |
| 38 lexical semantics | 25/110; shared affected408/1791 | [Lexical policy](lexical-semantics-audit.md) |
| 39 VLBV adapter | 25/85 | [Adapter](PHASE39-VLBV-ADAPTER.md) |
| 30c preparation;37b assignment;38b columns;40 sources;42 names | 67/268; shared affected586/2460 | [Delivery](DELIVERY.md) |
| 41 generated seeds | 10/45; affected69/282 | [Replay](PHASE41-GENERATED-ORDERING.md) |
| Final Rust1.88 acceptance | 172/670 across13 named BDD targets | [Compiler boundary](MSRV-ACCEPTANCE-PLAN.md) |

For the historical core phases through45, only focused phase-end BDD ran: no unit
suites, generic Cargo test, lint/check, early builds, workflow dispatches or
deployment checks. SDK46 later built/installed artifacts only in its adopted
whole-phase packet; separately authorized documentation delivery has its own
Pages workflows and public acceptance. Initial harness and protocol-order
failures are retained in evidence. The shrinking original Messages
count removes obsolete negative cases as retries/hooks become supported; their
new positive/negative behavior coverage lives in dedicated phase targets.
Primary development boundaries used rustc1.99.1. The final selected boundary
also passed on rustc1.88.0, including actual relocated and macro consumers.

Phase 12 reproduction, after the relevant implementation phase is complete:

```sh
npm ci --prefix tests/interop --ignore-scripts --no-audit --no-fund
cargo test --features output-messages --test messages_consumer -- --tags @phase12
```

Other focused commands and exact logs are linked from each phase document.
Do not replace them with a general test suite or rerun them before a phase ends.

## Files, forks and publication

Core changes are in `src/step.rs`, `src/cucumber.rs`, parser/basic/source modules,
runner/basic/hooks,
and writer/messages plus its source converter. BDD feature files and harnesses
live under `tests`; official schema/corpus fixtures retain licenses and pinned
provenance. Node consumer fixture lock is `tests/interop/package-lock.json`.
The final source inventory is `evidence/current-source-sha256.txt`; the earlier
historical inventory is preserved as `evidence/source-sha256-pre-final.txt`.
`evidence/final-provenance.json` records scope, compiler and unchanged base revisions.

Exactly one dependency fork was created:
[github.com/GQAdonis/gherkin](https://github.com/GQAdonis/gherkin), based on
cucumber-rs/gherkin `4c30e3a48a0226bf907053a0a9e738e91cf06727`, cloned at
`/Users/gqadonis/Projects/references/gherkin`. It is the parser source of truth;
production changes are mirrored into `vendor/gherkin` with provenance/hashes.
The existing GQAdonis/cucumber fork base remains
`d95872541ee2a43d71089c64bd3fa8848e24c922`. Authorized preservation commits and
pushes supersede the historical local-only status; the through45 source is on
both origin `foundation/rust-bdd-sdk` branches. See [delivery](DELIVERY.md) for
preserved foundation identities. No registry package is published, and no
additional dependency fork is justified yet.

The CI/CD skill was found at
`~/.claude/skills/ci-cd-and-automation/SKILL.md` (symlink into `~/.agents`) and
copied to `.agents/skills/ci-cd-and-automation/SKILL.md`, preserving the source.
Project overrides enforce BDD-only/end-phase execution. Existing Rust CI,
release and mdBook workflows remain unchanged. The separately authorized
[documentation Pages workflows and narrow hosting setup](DOCS-PAGES.md) are
additive; no unrelated branch-protection or security change is claimed.

No access blocker remains for the completed foundation/SDK boundary. Remaining
literal-JS, protocol and unadopted execution work is explicit in
[NEXT-PHASES.md](NEXT-PHASES.md) and [DELIVERY.md](DELIVERY.md).
Repository preservation pushes and fork documentation deployment are authorized
and have occurred. Registry publication remains gated on names, ownership,
publication scope and dependency/artifact closure; Cargo strips path dependencies
for registry releases. Matching upstream issues/PRs follow the remaining work
and full programme audit, not this scoped SDK closure. No merge or application
production change is claimed. Browser adapters remain decoupled, agent-browser
still wraps Node Playwright, and San Saba is untouched.

Latest sibling parser verification also passed73 scenarios/241 steps on Rust1.88
([evidence](evidence/phase45-parser-affected-bdd.log)). Through42 inventories are
preserved in `evidence/through42-checkpoint/`; current inventories identify the
through45 snapshot. Those historical inventories do not describe newer SDK or
documentation sources. Both foundation forks are preserved remotely; SDK native
archives remain local and registry-unpublished.
