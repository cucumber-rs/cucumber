# Vendored Gherkin parser provenance

Snapshot date: 2026-11-17. Crate: gherkin 0.16.0.

- Maintained source fork: https://github.com/GQAdonis/gherkin
- Upstream repository: https://github.com/cucumber-rs/gherkin
- Fork base commit: `4c30e3a48a0226bf907053a0a9e738e91cf06727`.
- Source checkout: `../gherkin` relative to the original Cucumber checkout.
- The source contains local unpublished Phase 1 changes to `src/parser.rs` for
  one table and one doc string in either order, plus Phase 6 literal unknown
  and trailing table-backslash preservation. Also includes Phase 6a
  `Feature::parse_file_source` for single-read source capture. This is a working-tree snapshot,
  not a claim that the remote base commit includes those changes.
- All `src/` files, `build.rs`, README, changelog and both licenses are copied
  byte-for-byte from that checkout. `SHA256SUMS` identifies these snapshot files.
- Cargo.toml retains production and optional dependencies, features, build
  dependencies and metadata. Only dev dependencies, test targets and their
  package include entries are removed, avoiding a dev dependency back to Cucumber.
- Original source Cargo.toml SHA256:
  `bee0972822b1300e83bf4db4471f307f7e1cdf8aa6b3ee3ce0725c321af6fcdf`.

The separate fork remains the source of truth: make parser changes there first,
copy the changed production files here, update hashes, and run the adopted BDD
acceptance at the phase boundary. Do not edit the snapshot independently.

Cucumber path/git consumers use this relative path dependency automatically.
Cargo registry publication strips dependency paths and resolves the declared
registry version; a published Cucumber 0.23.0 package would therefore resolve the
released gherkin 0.16 instead. Publication is not authorized or solved here. A
future release must first publish a suitable parser version and update Cucumber's
version dependency. Including this directory in a package is not sufficient to
change registry dependency resolution.

## Explicit Gherkin 43 dialect catalogue

The original `src/languages.json` remains the legacy default unchanged.
`src/languages-43.json` is an exact copy of the official Gherkin 43.0.0 catalogue
at commit `3d70e2bb4e0f16a568ee539a5ce0b03fb247a777`, Git blob
`d84cbf7f929e8a3fddd42792c02a32ca6c3b4fa0`, SHA256
`84c50533cce6f2fbb1c1e1eca14811000a69ef70b5b564075b50bdfa7333c329`.
Its upstream MIT notice is preserved in `src/LICENSE-GHERKIN`.
Build output contains independent legacy/latest maps; `GherkinEnv::new_latest`
and `is_latest_language_supported` opt into the 80-dialect official catalogue.
Language headers retain that policy. Legacy constructor/default support remains
unchanged, including removed aliases and the local `formal` extension.

## Explicit official doc-string content

`DocStringSemantics::Gherkin43` via `GherkinEnv::with_docstring_semantics` opts
into the adopted official line-oriented content behavior. All constructors retain
`Legacy` content by default, independently of the dialect catalogue. Native Step
shape is unchanged. Source fork BDD compares exact content with pinned official
Gherkin 43 docstrings and combined-arguments fixture AST expectations; coverage
and limitations are recorded in the fork's `docs/gherkin43-docstrings.md`.

## Phase 13 doc-string indentation consistency

The same explicit `DocStringSemantics::Gherkin43` policy additionally recognizes
ECMAScript indentation for doc-string delimiters and content, including NBSP and
BOM, and counts indentation by characters. Legacy grammar is unchanged. The
snapshot parser.rs is synchronized from the maintained sibling fork. Root
`messages_docstrings` BDD compares actual execution plus complete AST/Pickles
against the isolated official Gherkin 43.0.0 JavaScript parser/compiler.

## Explicit Gherkin 43 table sizing

Phase 15 adds TableSemantics::Gherkin43 through
GherkinEnv::with_table_semantics. Strict parsing retains each row's cells for
the existing inconsistent-cell-count validator, rejecting either shorter or
longer rows. All constructors retain historical later-row truncation by default.
Dialect and doc-string policies remain independent. Source acceptance and pinned
official invalid-table evidence are documented in ../gherkin/docs/gherkin43-tables.md
(relative to the Cucumber repository).

## Structured parser diagnostics

Phase17 exposes ParseError::position without changing parsing behavior, so
Messages can report the native diagnostic location without parsing Display text.

## Phase19 complete official plaintext corpus

The additive `Document::parse` API represents an optional Feature for empty or
comment-only documents without changing Feature::parse. Explicit official table
mode accepts text outside closed pipe cells; latest vocabulary prefers the
longest step keyword across kinds and accepts padded Feature preambles. Legacy
policies remain the default. Both parser.rs and lib.rs are byte-synchronized
from the sibling fork, preserving the Phase17 ParseError::position getter.

Root per-fixture BDD against50 good+12 bad officialGherkin43 sources passed62
scenarios/236steps, comparing full normative AST/Pickles and native rejection;
prior focused fork BDD73/241 also passed. See root
`docs/compatibility/gherkin-corpus.md` and the pinned corpus manifest for exact
scope/provenance. These local changes remain unpublished.

## Phase25 path-aware featureless documents

Additive `Document::parse_file_source` accepts already-read source with file/error
identity and optional Feature, enabling exact single-read runtime capture. The
featureless grammar now rejects a bare language directive, matching the actual
pinned Gherkin43 oracle: Language requires a Feature header. Feature::parse and
legacy defaults remain unchanged. Fork src/lib.rs and src/parser.rs are synced
byte-for-byte with this snapshot and their SHA256 entries updated.

Root Phase25 BDD passed11 scenarios/41 steps; affected official corpus62/236,
input-errors6/30 and source integration21/81 all passed. See
`docs/compatibility/featureless-documents.md` in the root repository for contract
and archived evidence. No changes were published.

## Phase28a native Markdown source API

Additive src/markdown.rs exports a standalone typed MarkdownDocument parser under
the existing parser feature, using the official43 dialect catalogue. It preserves
original locations, tags/cells, optional Feature keyword and doc-string metadata,
then projects into native Feature without changing classic APIs. No Node or
Messages dependency is introduced. The new file and lib.rs export are byte-synced
from the maintained sibling fork, with SHA256 entries recorded.

Root focused BDD passed21 scenarios/78 steps, including all five official Markdown
fixture graphs and fifteen live-oracle matcher boundaries. Affected classic
corpus62/236, featureless11/41 and rich/localized19/80 BDD passed. See root
`docs/compatibility/markdown-native.md` for scope and evidence. Runtime format
selection is deferred to28b. Nothing was published.

## Phase28b positioned Markdown diagnostics

ParseError gains an additive positioned-message constructor with private detail;
legacy constructors keep empty detail and existing Display text. This preserves
existing ParseFileError variants for Markdown errors and unresolved Auto formats.
Fork/vendor lib.rs remain byte-synchronized with updated SHA256. Root runtime BDD
passed14 scenarios/54 steps, including all five official Markdown fixtures through
actual native handlers, exact graph/source reporting and typed snapshots. See
root `docs/compatibility/markdown-runtime.md`. No publication occurred.


## Phase38 explicit classic lexical semantics

The maintained parser fork adds `LexicalSemantics::{Legacy,Gherkin43}`. Legacy
padding/trimming/byte columns remain the default; explicit latest lexical mode
uses ECMAScript whitespace and scalar token columns on the original source bytes.
The internal PEG receives original source for location calculation. Its existing
private grammar call sites, including the legacy tag-expression adapter, were
updated mechanically; no unit suite was added or run. Native public AST shapes
and table-cell trim sets are unchanged. Fork `src/parser.rs`, `src/lib.rs` and
`src/tagexpr.rs` are byte-synchronized and SHA256SUMS refreshed.

Root capture/compiler/row-origin integrations record the selected policy.
Phase38 feature-first acceptance implementation is complete and pending the
joint Phase36/37/38 phase-end BDD boundary. This is not a passing acceptance
claim. No new fork, Node runtime dependency or publication was introduced.


Phase38 primary root acceptance subsequently passed25 scenarios/110 steps with
no compiler warnings (`docs/compatibility/evidence/phase38-sixth-bdd.log`). The
runtime preamble-padding failure and PEG-action syntax repair are retained in
prior evidence. An independently authored25th scenario verified LexicalG43 with
Legacy vocabulary before widening the after-tag guard. Fork/vendor parser bytes
and hashes include both bounded corrections. Affected regressions are tracked
separately; no unit suite or publication occurred.


Phase38 affected root acceptance subsequently passed408 scenarios/1,791 steps
across25 focused targets. The maintained sibling parser's existing BDD target
`cargo test --test step_arguments` passed73 scenarios/241 steps. Root evidence:
`phase36-37-38-affected-bdd.log` and `phase38-parser-affected-bdd.log`.
No source change, new fork or publication followed those passing boundaries.


Phase45 implementation (acceptance pending): Markdown executable token precedence
now keeps inline backtick tags literal in steps, eligible argument rows/fences,
and Examples rows. Pending declaration tags retain priority. The bounded source
change is in the maintained sibling `src/markdown.rs`, mirrored byte-for-byte here;
SHA256SUMS records the updated parser source. No native classic parser/default,
public AST shape, dependency or protocol specification changed. Feature-first
root acceptance is `messages_markdown_precedence`, with complete pinned official
AST/Pickle comparisons and actual handler payloads. No tests or oracles have run
before the coordinated completed-phase boundary; no publication occurred.

Phase45 subsequently passed its first Rust1.88 coordinated boundary:21 scenarios/
100 steps, whole official graphs and actual handler payloads. Root affected BDD
passed106/460 across8 targets and the maintained sibling step_arguments BDD
passed73/241 on Rust1.88. Evidence is retained in cucumber docs/compatibility/
evidence/phase43-45-first-bdd.log, phase43-45-affected-bdd.log and
phase45-parser-affected-bdd.log. No production correction or publication followed.
