# Local Rust BDD foundation delivery — through45

The latest concrete gaps are implemented and verified: VLBV receipt consistency,
native Messages rerun selection and Markdown tag-token precedence. The first
Rust1.88 boundary passed87 scenarios/348 steps; eight affected targets passed
106/460, including the relocated consumer. Earlier evidence remains historical,
and through42 inventories are preserved separately before current regeneration.

The local foundation now provides source-faithful native Gherkin compilation,
standard Cucumber Messages, explicit compatibility policies, ordered lifecycle
callbacks, typed evidence, deterministic and resource-aware scheduling, and a
read-only VLBV receipt adapter. All bounded implementation phases through45 have
primary and affected root BDD evidence. Final Rust1.88 delivery acceptance passed
all13 selected targets:172 scenarios/670 steps, with no source/dependency fixes.

## Use it

Start with [REUSE.md](REUSE.md). A complete local checkout is a path dependency;
its `vendor/gherkin` snapshot removes any sibling-clone or machine-specific Cargo
patch requirement. The native compiler and runner require no JavaScript runtime.
The optional [VLBV adapter](PHASE39-VLBV-ADAPTER.md) uses Node standard library,
accepts original Messages plus explicitly bound receipts, and writes derived
boundary-results JSON to stdout. It neither executes a packet nor mutates
canonical phase/release state. Keep original process, assertion, negative-control
and cleanup receipts: a passing runner alone is insufficient acceptance evidence.

Key implementations are `src/parser/basic.rs`, `src/parser/source.rs`,
`src/writer/messages.rs` and its source graph, `src/runner` lifecycle/scheduling/
timeout modules, `src/execution.rs`, `src/attachment.rs`, `src/support_code`,
`codegen/src/attribute.rs`, and `scripts/messages-to-boundary-results.mjs`.
The [pinned matrix](CAPABILITIES.md) and [live results](LIVE-STATUS.md) distinguish
verified contracts from API/model boundaries.

## Recent completed acceptance

| Contract | Scenarios / steps passed |
| --- | --- |
| Latest receipt consistency, native rerun and Markdown boundary (including25 prior cases) | 87 / 348 |
| Their eight affected Rust1.88 BDD targets | 106 / 460 |
| VLBV projection and actual canonical reader | 25 / 85 |
| Preparation, assignment, table columns, source paths and name admission | 67 / 268 |
| Their32-target affected BDD regression | 586 / 2460 |
| Generated ordering observation and cross-language seed replay | 10 / 45 |
| Its five affected BDD targets | 69 / 282 |
| Rust1.88:13 selected BDD targets, including relocated and macro consumers | 172 / 670 |

These counts describe separate runs, not a sum of unique scenarios. Features were
authored before their implementation; execution occurred only after each complete
phase. Only focused BDD targets ran. Initial failures and zero-selection mistakes
are retained and explicitly excluded from passing evidence. Exact commands are
in `evidence/phase43-45-commands.txt`, `evidence/phase39-command.txt`, `evidence/phase30c-37b-38b-40-42-commands.txt`
`evidence/phase41-commands.txt`, and `evidence/msrv-1.88-command.txt`. Child macro-consumer commands, sources,
compiler diagnostics and oracle output are retained under `evidence/phase42-children`.

## Fork and distribution status

Only dependency fork created/needed: [GQAdonis/gherkin](https://github.com/GQAdonis/gherkin),
from cucumber-rs/gherkin, cloned at `/Users/gqadonis/Projects/references/gherkin`.
Its production snapshot is bundled in cucumber. No expression, regex, browser or
application fork was needed. The existing cucumber fork is
[GQAdonis/cucumber](https://github.com/GQAdonis/cucumber).

The verified through45 foundation was committed and pushed on 2026-11-18 to
the nonrelease branch foundation/rust-bdd-sdk in both verified origins:
Cucumber 6e4753fc0783038bb388de1b4d76055edc697b8f, Gherkin
efb91e8bb5a7d59a717f88c05d1c06ec5d06a819. No main branch, release tag,
deployment or registry publication changed. Inventories/provenance retain the
historical implementation record; their earlier local-only status is superseded
by these commits, not silently rewritten.

The requested SDK/FFI, documentation sites, package preparation and review
programme is described in [SDK-ROADMAP.md](SDK-ROADMAP.md). The SDK/native bindings remain proposed. Both additive documentation sites now
build locally and passed their separate10-scenario/64-step browser/build boundary;
see [documentation evidence](DOCS-SITE-ACCEPTANCE.md) and [review](DOCS-SITE-REVIEW.md).
No site deployment occurred. Registry publication requires confirmed
owned names and full dependency/artifact closure.

The CI/CD skill was copied from
`~/.claude/skills/ci-cd-and-automation/SKILL.md` (resolving to `~/.agents`) into
`.agents/skills/ci-cd-and-automation/SKILL.md`, preserving its original source.
The project copy explicitly preserves phase-end-only BDD restrictions. Existing
workflow/build/release configuration and security settings were not changed.
No merge, deployment, production action or unrelated application migration occurred.

## Limits that prevent a full-parity claim

- Official Gherkin43 Markdown fallback AST can omit Feature.keyword, while both
  pinned Messages schemas require it. Exact upstream AST fidelity and universal
  schema validity conflict for that shape; explicit Feature headings avoid it.
- The pinned JS capture-index polyfill returns a wrong nested repeated-capture
  offset. Native code retains the actual source span, independently confirmed by
  JavaScript native indices; byte-for-byte equivalence to that defect is not claimed.
- Native async cases do not provide JS worker-thread/process isolation or per-worker
  support-code loading. Rust handlers do not execute JavaScript support files.
- Rust regex syntax, numeric types and FromStr conversions remain native. Multiple
  captures and asynchronous transformations are available through typed handlers or
  raw Context, rather than a JavaScript ParameterType transformer registry.
  Snippet/preference metadata does not implement JS snippet generation or automatic
  raw-regex registry selection. See [the expression assessment](EXPRESSION-NAME-POLICY-DESIGN.md).
- Actual official Query16, JUnit0.15 and JS13.3 rerun formatter interoperability is covered. Arbitrary
  external formatter/plugin behavior is not certified. VLBV stochastic profiles
  are explicitly blocked by the bounded deterministic receipt adapter.

These are explicit remaining interoperability boundaries, not successful tests or
hidden production changes. Full Cucumber.js parity and universal Gherkin/Messages
conformance are not claimed. The practical local VLBV integration is separately
verified within its documented contract.

Latest sibling parser verification also passed73 scenarios/241 steps on Rust1.88
([evidence](evidence/phase45-parser-affected-bdd.log)). Through42 inventories are
preserved in `evidence/through42-checkpoint/`; current inventories identify the
through45 snapshot. The implementation is preserved on both origins; package artifacts remain unpublished.
