# Rust BDD interoperability checkpoint

Baseline reviewed 2026-11-17. This is a capability assessment, not a full parity claim.

Completed bounded phases through45 have primary and affected root BDD evidence. Latest Rust1.88 primary87/348 and affected106/460 passed first try. Latest five-phase affected586/2460 and earlier sibling parser73/241 passed. See [live phase counts](LIVE-STATUS.md). See [current integration, results and continuation](STATUS.md); [Phase 1](PHASE1.md) remains historical evidence. These are verified subsets, not full parity.

Rust1.88 acceptance passed172 scenarios/670 steps across13 named BDD targets,
including external relocated and macro consumers. [Evidence](evidence/msrv-1.88-first-bdd.log).

## Pinned references

- Cucumber0.23 fork: historical upstream base d95872541ee2a43d71089c64bd3fa8848e24c922; verified through45 implementation preserved at 6e4753fc0783038bb388de1b4d76055edc697b8f, Rust minimum1.88.
- [Cucumber.js v13.3.0](https://github.com/cucumber/cucumber-js/releases/tag/v13.3.0), release commit `4f21233`.
- [Official Gherkin v43.0.0](https://github.com/cucumber/gherkin/releases/tag/v43.0.0), release commit `3d70e2b`.
- [JS pinned dependencies](https://github.com/cucumber/cucumber-js/blob/v13.3.0/package.json): Gherkin 42.0.1, Messages 34.2.1, expressions 20.1.0, tag expressions 11.0.1. Latest Gherkin and the JS bundled parser are separate baselines. Gherkin 43 moves to Messages 35.
- Rust dependencies: gherkin 0.16, cucumber-expressions 0.5. Exact resolved versions belong in phase-end evidence.

## Capability matrix

| Capability | Verified implementation / remaining boundary |
|---|---|
| Step matching | Phase1 KeywordIndependent opt-in, ambiguity/captures/localized keywords; legacy KeywordSpecific default preserved |
| Both step arguments | Table+docstring either order with both values retained; repeated argument rejection |
| Latest dialects | Pinned43 catalogue80 dialects, explicit legacy catalogue, global longest keyword matching |
| Outline interpolation | Phase14 literal header-order substitution, unknown placeholders retained, dollar literals; legacy behavior preserved |
| Docstrings/tables | Phases9/13/15 native values+source AST/Pickle agree on strict table widths, media, dedent, delimiter/escape/ECMAScript whitespace; explicit policies preserve legacy behavior |
| Plaintext AST/Pickle | Phase19 all62 official plaintext fixtures:50 complete valid AST/Pickle matches and12 invalid rejected. Phase25 featureless docs and run integration. This is corpus coverage, not every possible input |
| Lexical edges | Source audit found ECMAScript whitespace/title/tag and column differences beyond the official corpus. Phase38 explicit policy25/110 passed acceptance; table-cell UTF-16 columns38b11/53 also passed |
| Markdown | Phase45 literal tag/token precedence21/100 passed whole-graph/runtime acceptance; Phase28a native parser/graph21/78, all five official positive fixture graphs; runtime/source-format integration28b14/54 and shared affected212/906 passed |
| Messages | V34.2.1 default/V35.0.0 explicit, source/AST/Pickle, retry attempts, actual timings/results, captured input errors, logs, typed scoped attachments, empty documents. Global-hook reporting passed Phase26 |
| Consumers | Native rerun collector/typed source selection44 passed19/73 against actual JS13.3 RerunFormatter/EventDataCollector, including a real exact-row follow-up; actual pinned official Query16/JUnit0.15 consume rich source/composite hooks/retries; Query also resolves typed attachments and global callbacks. Wider formatter parity remains unverified |
| Scenario hooks | Existing Hooks registry ordered setup/reverse cleanup, tags/retries verified; legacy before/after replacement retained. Phase29 per-callback observations/evolving native cleanup18/76, Phase31 individual Messages/attachments13/64. Legacy composite reporting remains available |
| Step/global hooks | StepHooks Phase20 10/51; RunHooks Phase24 14/62, global Messages Phase26 6/34. Cancellation cleanup not guaranteed for dropped async runs; step-hook outcomes aggregate into their pickle step as in JS13; no separate callback TestSteps are promised |
| Expressions/tags | Phase21 JS-compatible builtin expressions and official tag parser53/159; legacy APIs retained. Expr_js name admission42 passed12/47 against actual compiler and JS registry. Custom types remain Rust registrations. Full parameter/capture metadata and regex-dialect equivalence not claimed |
| Support-code metadata | Phase27 final registry snapshots and actual regex definitions/matching IDs passed14/56, with protocol and official consumer regressions. 32a original builtin expressions/validated capture trees19/85; 32b explicit custom/raw descriptors16/78; raw capture mapping is caller-declared, not inferred handler binding parity. One verified upstream polyfill offset deviation documented |
| Scheduling/retries | Phase36 validated typed gate/budget16/56,37a global ordering/exact seeded shuffle/retained slots native19/81+Messages/tracing2/16 passed. Dynamic assignment37b14/55 passed, retaining retry reservations and Serial barriers. Generated seed/replay41 passed10/45 with affected69/282. Native async limits1/2/3/unlimited, overlap, World isolation, serial exclusivity, failfast1, fresh World/hooks on retry and terminal retry reporting verified. JS worker-thread isolation, cancellation/fairness/configuration boundaries not equivalent or unverified |
| Timeout/control outcomes | 30a opt-in timeouts14/50 with callback cancellation/cleanup; 30b pending/skipped20/87 core +18/54 consumer cases passed; 30c owned preparation/factory-boundary17/69 passed under explicit opt-ins; Rust future polling remains native |
| Selection | Explicit33 typed names OR, tags AND, caller predicates AND12/56; legacy CLI override preserved. Exact source-line selectors34 passed38/178 and37/173 without Messages; ordered typed source paths40 passed13/44 |
| Dry-run | Explicit35 World construction, suppressed callbacks, non-failing classification, exact skipped lifecycle14/59; JSON/JUnit/libtest9/30 |
| Attachments | Phase23 text/binary, media/file names, base64, actual step/scenario-hook scoped association, retry isolation and stale-handle rejection9/36; global attachment API not inferred from schema |
| VLBV integration | Phase43 lifecycle/retry/receipt association22/90 plus prior25/85 passed on Rust1.88; process.messages exact binding now required. Phase39 read-only standard Messages plus explicit receipts adapter passed25/85, including copied canonical reader acceptance. It refuses green-only/missing/stale evidence and preserves failed retries; no canonical phase/release mutation |
| Browser adapters | Optional and decoupled; agent-browser Rust CLI wraps Node Playwright daemon; no pure-Rust or application migration claim |
| JS support files | Native Rust support code; cross-language source loading requires explicit bridge and is outside native behavior parity |

## Historical phases1–12 and boundaries

1. Phase 1: keyword matching and both argument orders; 38 focused BDD scenarios passed across runner and parser fork.
2. Phase 2: source-provenance vendor integration and relocated external consumer; 1 new scenario plus all 20 runner compatibility scenarios passed without a patch.
3. Phase 3: ordered scenario hook registry; 9 BDD scenarios passed. Native stage event representation remains composite.
4. Phase 4: explicit Messages 34.2.1 source/AST/Pickle/execution subset; 23 BDD scenarios passed with official schema validation and semantic graph/outcome assertions.
5. Phase 5: existing async scheduling assessment; 8 BDD scenarios passed, no scheduler change needed.
6. Phase 6: immutable sources, richer AST/Pickle graph and source-backed Messages; 5 source scenarios and 12 graph/execution scenarios passed. Parser escaped-table prerequisite passed 24 scenarios including earlier cases.
7. Phase 7: official dialect catalogue; 50 parser scenarios including earlier cases, and 8 source scenarios including earlier cases passed.
8. Phase 8: localized source graphs; 7 scenarios plus 12 rich-source regressions passed.
9. Phase 9: opt-in official docstring execution; 60 parser scenarios including earlier cases and 3 root handler scenarios passed.
10. Phase 10: native retry Messages; 6 scenarios plus 22 earlier Messages regressions passed.
11. Phase 11: native composite hook-stage Messages; 10 scenarios passed. Affected regressions passed: plain Messages21/101, retries6/40 and rich/localized19/80 scenarios/steps.
12. Phase 12: official JavaScript Query and JUnit consumers; 2 scenarios / 8 steps passed for rich combined arguments/hooks and mixed final retry outcomes.

Phases13 onward and exact scenario/step counts are recorded in [LIVE-STATUS.md](LIVE-STATUS.md), with dedicated phase contracts and retained logs.

For each phase, business/compatibility features were authored first, the complete bounded implementation/harness was finished, then only focused BDD acceptance ran. No unit suites, generic cargo test, lint/check, or premature build. Details and first-attempt diagnostics are preserved in each phase's evidence. Remaining implementation contract is in [STATUS.md](STATUS.md), with source-supported [next-phase gaps](NEXT-PHASES.md).

Current integration: Phase 2 bundles a source-provenance snapshot at `vendor/gherkin`; normal consuming checkouts need no Cargo patch or sibling parser clone. A relocated external consumer passed BDD. Registry publication still requires a published compatible parser version because Cargo strips path dependencies. See [Phase 2](PHASE2.md).

Only dependency fork required so far: `cucumber-rs/gherkin` to `GQAdonis/gherkin`. No expressions, regex, peg, browser or application fork is justified by this change. Fork/clone is authorized; code publication, deployments and merges are not. The parser snapshot carries local fixes without remote publication; GQAdonis/gherkin remains its upstream source of truth.

## Reuse

Use `.step_matching(cucumber::step::Matching::KeywordIndependent)` after installing a step collection. Replacing the collection replaces its matching policy; configure mode after `.steps(...)`. Existing callers retain keyword-specific resolution. Compatibility mode currently covers only matching, not every JS runtime behavior. Idiomatic Rust receives both arguments through `Context.step.docstring` and `Context.step.table`; source snapshots preserve original cross-argument order/media type for protocol reporting; the native AST retains the executable values.

Project CI guidance is copied to `.agents/skills/ci-cd-and-automation/SKILL.md`, preserving its source under ~/.claude (symlink to ~/.agents). Project restrictions override generic early/unit gates. No CI workflow was changed or dispatched.

Latest sibling parser verification also passed73 scenarios/241 steps on Rust1.88
([evidence](evidence/phase45-parser-affected-bdd.log)). Through42 inventories are
preserved in `evidence/through42-checkpoint/`; current inventories identify the
through45 snapshot. Both repositories remain uncommitted and unpublished.
