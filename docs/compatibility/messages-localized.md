# Localized source graphs

The snapshot-backed `SourceGraph` and Messages writer now read keyword spellings
from the same immutable catalogue files used by the parser:

- `DialectCatalog::Legacy`: `vendor/gherkin/src/languages.json`.
- `DialectCatalog::Gherkin43`: `vendor/gherkin/src/languages-43.json`, pinned to
  the official Gherkin 43 catalogue.

No latest aliases are merged into Legacy. The snapshot records its catalogue
and fallback language. Only a directive in the pre-feature comment/blank
preamble overrides that fallback; literal directives in doc strings and later
comments retain their ordinary source meaning.

Localized structural words are classified by their catalogue role, rather than
translated or approximated. Step aliases use longest-prefix matching. A spelling
registered more than once has `Unknown` AST keyword type, following the official
[Gherkin 43 token matcher](https://github.com/cucumber/gherkin/blob/v43.0.0/javascript/src/GherkinClassicTokenMatcher.ts).
Conjunction and wildcard Pickle inheritance follows the same compiler rule used
in the English source graph. The AST and every Pickle retain the actual language
code; keyword strings retain their exact spelling and spacing. Source columns
count Unicode characters, not UTF-8 bytes.

Example with a configured fallback and explicit latest vocabulary:

```rust,ignore
use cucumber::{World as _, parser::DialectCatalog};

MyWorld::cucumber()
    .with_dialect_catalog(DialectCatalog::Gherkin43)?
    .language("ml")?
    .with_messages(std::fs::File::create("results.ndjson")?)
    .run("features")
    .await;
```

The selected catalogue contains the registered dialect vocabulary, but this
phase does not establish complete execution/grammar coverage for every dialect.
Native parser acceptance and source/native candidate agreement remain required.
The native `formal` extension is outside the JSON catalogues and remains
unsupported by this source converter. Existing native doc-string representation
and unsupported execution-event boundaries remain as described in
[messages-source.md](messages-source.md).

## Adopted acceptance

Scenarios in `tests/features/messages_source/localized.feature` were authored
before this implementation. Two untouched official fixture triplets (French and
emoji) compare their complete AST/Pickles with only ID/reference and URI
normalization. French covers overlapping aliases and apostrophe/no-space forms;
emoji verifies a non-Latin no-space step. Other focused scenarios cover a French
fallback without directive, newly registered Malayalam with no-space keywords,
Chinese latest-only Rule spelling and Unicode table columns, wildcard/context
classification, and Legacy preserving the same latest-only spelling as ordinary
description text. No full-80-dialect claim is inferred from these cases.

Fixture provenance is in `tests/fixtures/gherkin43/localized-manifest.json`; the
existing corpus license applies. Official expected fields are not edited or
stripped. Independent Messages 34.2.1 schema validation remains enabled.

Phase-end commands:

```sh
cargo test --features output-messages --test messages_source -- --tags @phase8
cargo test --features output-messages --test messages_source -- --tags @phase6
```

Verified phase-end result on 2026-11-17: **7 scenarios / 31 steps passed** on
the first run. Both official localized fixture graphs matched in full under the
stated ID/URI bijection. Because the shared converter changed, the Phase 6 rich
source target was also rerun at the boundary: **12 scenarios / 49 steps passed**.
Only these focused BDD commands ran; no unit suites or generic Cargo test ran.

Evidence:
- [`phase8-localized-bdd.log`](evidence/phase8-localized-bdd.log)
- [`phase8-rich-regression-bdd.log`](evidence/phase8-rich-regression-bdd.log)
