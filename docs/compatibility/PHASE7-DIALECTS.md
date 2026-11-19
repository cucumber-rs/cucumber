# Phase 7: explicit Gherkin 43 dialect catalogue

The existing native catalogue remains the default. Applications opt into the
exact official Gherkin 43.0.0 catalogue, containing 80 dialects (three added;
21 existing entries changed). Latest keyword matching prefers longer aliases,
matching the official token matcher; legacy ordering and vocabulary remain.
Source language directives retain the selected catalogue.

Native API: `GherkinEnv::new_latest(language)` and
`is_latest_language_supported(language)`. Runner API:

```rust,ignore
use cucumber::parser::DialectCatalog;
YourWorld::cucumber()
    .with_dialect_catalog(DialectCatalog::Gherkin43)?
    .language("be")?
    .run("features")
    .await;
```

Select catalogue before a newly supported language. Switching an already
configured language to an incompatible catalogue returns an error rather than
silently reverting to English. SourceSnapshot records the catalogue as well as
configured fallback dialect. This API selects **keyword vocabulary only**; it
is not a claim of complete Gherkin 43 grammar/runtime conformance.

The source fork and vendor carry an unmodified official languages-43.json plus
upstream MIT notice and provenance. Legacy languages.json is unchanged. See
`../gherkin/docs/gherkin43-dialects.md` from the repository parent (or the
separate fork's docs directory) for the exact catalogue changes and source.
SHA256: `84c50533cce6f2fbb1c1e1eca14811000a69ef70b5b564075b50bdfa7333c329`.

## Phase-end BDD

Features were authored before production changes. No unit suites/checks or
premature compilation ran. Completed focused commands:

- In the parser fork, `cargo test --test step_arguments`: **50 scenarios /172
  steps passed**, including 182 fixed keyword-source cases across 24 changed or
  new dialects. This includes prior argument/escape scenarios; they are repeat
  coverage. [Observed log](evidence/gherkin43-dialects-bdd.log).
- In Cucumber, `cargo test --test sources -- --tags '@phase6_sources or
  @phase7_sources'`: **8 scenarios /30 steps passed**. Includes five earlier
  snapshot scenarios and three new catalogue integration scenarios. Texan Rule
  correction, default legacy policy and incompatible formal-dialect switch are
  explicitly asserted. [Observed log](evidence/phase7-sources.log).

The first integration attempt exposed ownership of the existing Cow-based
UnsupportedLanguageError: its borrowed configured dialect could not escape the
builder. Returning an owned name fixed the error path; the focused command was
rerun. [Initial diagnostic](evidence/phase7-sources-initial.log).

No new forks, pushes, deployments, registry publication or settings changes.
Localizations outside the selected cases are data-backed but not independently
claimed as exhaustively parser-conformant. Full grammar/corpus and runtime
semantics remain separate acceptance work.
