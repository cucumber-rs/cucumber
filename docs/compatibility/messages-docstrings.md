# Phase 13: consistent official doc-string values

Select `DocStringSemantics::Gherkin43` for native Rust handlers to receive the
same doc-string content as the adopted Gherkin 43.0.0 JavaScript parser/compiler.
The shared-source Messages scanner independently preserves source media type,
opening delimiter/location, and normalized AST/Pickle content. Existing Legacy
handler semantics remain unchanged and can differ from normalized Messages.

This phase covers delimiter-prefix closure (trailing closing-line text is
ignored), replacing only the first escaped active delimiter per content line,
inactive delimiter escapes, and ECMAScript indentation. NBSP, BOM, mixed Unicode
spaces, tabs, vertical/form feeds and standalone carriage return count as
indentation characters; NEL and zero-width space do not. Content removes up to
the opening indentation, preserving deeper indentation, blank lines and payload
characters. The native grammar changes are limited to explicit official doc-string
mode; source report scanning consistently follows the official representation.

The authoritative parser edits live in the existing `GQAdonis/gherkin` checkout,
synchronized byte-for-byte to `vendor/gherkin/src/parser.rs` with SHA256 provenance.
No additional fork is needed. Other dialect/token whitespace behavior is outside
this bounded phase.

## Official oracle and acceptance

`tests/interop/gherkin43` independently pins `@cucumber/gherkin` 43.0.0 and
`@cucumber/messages` 35.0.0, with exact npm lockfile integrity. Install with
`npm --prefix tests/interop/gherkin43 ci --ignore-scripts --no-audit --no-fund`.
Its `oracle.mjs` reads source on stdin, accepts an optional URI argument, and
returns `{gherkinDocument,pickles}`. Parsing errors produce stderr and nonzero exit.
The parser/compiler runs only inside phase-end BDD, never as an early check.
This is test tooling only; native consumers acquire no Node dependency.

The official baseline is Gherkin v43.0.0 commit
`3d70e2bb4e0f16a568ee539a5ce0b03fb247a777`, particularly the tagged
[GherkinLine](https://github.com/cucumber/gherkin/blob/v43.0.0/javascript/src/GherkinLine.ts)
and [token matcher](https://github.com/cucumber/gherkin/blob/v43.0.0/javascript/src/GherkinClassicTokenMatcher.ts).
The adopted AST/Pickle fields are also accepted by the pinned Messages34.2.1
schema; the writer protocol version has not changed to Messages35.

Feature scenarios were authored first. Focused acceptance compares real handler
content and the entire AST/Pickle graph against the actual pinned official
compiler, permitting only ID bijection and URI normalization. No content,
location, media, delimiter, linkage, or optional fields are discarded. Every
emitted envelope also validates against Messages34, with successful step/run
outcomes asserted. Legacy scenarios assert exact unchanged payload and continued
rejection of Unicode delimiter indentation under the default policy.

```sh
cargo test --features output-messages --test messages_docstrings -- --tags @phase13
```

Phase-end acceptance passed on its first run: **15 scenarios / 86 steps**,
including 14 actual official-parser comparisons plus the legacy rejection case.
See [phase13-docstrings-bdd.log](evidence/phase13-docstrings-bdd.log).
Affected prior source graph cases passed **19 scenarios / 80 steps**, and native
handler/snapshot cases passed **3 scenarios / 13 steps**; see
[phase13-docstrings-regression.log](evidence/phase13-docstrings-regression.log).
No generic/unit suites, premature builds, publication, or remote changes ran.
