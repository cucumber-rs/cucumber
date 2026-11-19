# Phase28 design: native Gherkin in Markdown

Status: design agreed and native phase28a implemented. The initial inventory was
read-only; production followed authored BDD and official oracle execution occurred
only at phase end. See [observed native results](markdown-native.md). Runtime
format/store/writer integration is implemented in separately verified
[Phase28b](markdown-runtime.md).

## Versioned evidence

- Gherkin43.0.0 commit `3d70e2bb4e0f16a568ee539a5ce0b03fb247a777`:
  [Markdown matcher](https://github.com/cucumber/gherkin/blob/3d70e2bb4e0f16a568ee539a5ce0b03fb247a777/javascript/src/GherkinInMarkdownTokenMatcher.ts),
  [grammar](https://github.com/cucumber/gherkin/blob/3d70e2bb4e0f16a568ee539a5ce0b03fb247a777/gherkin.berp),
  [generated parser](https://github.com/cucumber/gherkin/blob/3d70e2bb4e0f16a568ee539a5ce0b03fb247a777/javascript/src/Parser.ts),
  [source dispatch](https://github.com/cucumber/gherkin/blob/3d70e2bb4e0f16a568ee539a5ce0b03fb247a777/javascript/src/makeSourceEnvelope.ts).
- Cucumber.js13.3.0 tag tree `4f2123366dfd5b9b030fb0010ada23752d710b14`:
  [paths](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/paths/paths.ts),
  [loader](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/api/gherkin.ts),
  [dependencies](https://github.com/cucumber/cucumber-js/blob/v13.3.0/package.json).
  It pins Gherkin42.0.1 and gherkin-streams7.0.1, not43. Its default source glob
  includes `feature` and `feature.md`; an explicitly supplied extensionless
  directory expands recursively with `.feature` only. Explicit files/globs are
  retained and passed to GherkinStreams with the configured default dialect.
- All five official43 good Markdown fixtures and their25 artifacts are pinned
  under `tests/fixtures/gherkin43-markdown`, with blob/hash manifest. There are
  no official bad Markdown fixtures in this pinned tree.

## Actual format and semantics

This is the official alternate token matcher for the same Gherkin grammar, not
a general CommonMark renderer or extraction of fenced classic Gherkin blocks.
One-to-six hash heading markers plus whitespace precede localized header
keywords; bullet markers `*`, `+`, `-` precede step keywords. The wildcard step
keyword itself is excluded. Prefix widths contribute to source columns.
Backtick-enclosed @tags retain their original columns and may be adjacent.

A first non-tag token can become a Feature title without a Feature keyword;
`misc.feature.md` has an absent AST keyword field. The matcher is stateful and
called in grammar-state order: it cannot be replaced by a context-free regex
pass. Non-Gherkin prose is ordinarily ignored, not a classic description.
Markdown does not recognize inline language directives; callers select the
fallback dialect globally. Leading blanks and language-looking prose need
explicit oracle-derived acceptance because Feature fallback consumes tokens
before the parser's Empty alternative.

Data-table lines require exactly2–5 leading ECMAScript whitespace characters
before a pipe. A row with at least one cell matching `:?-+:?` is a separator and
is ignored; it is not a data row or an emitted comment in existing expectations.
Retained row/cell locations must remain original Markdown coordinates.

Doc strings use a run of at least three backticks, closed by exactly the active
run with no trailing suffix. A shorter run remains content. The opening media
suffix, indentation removal and literal content follow the Markdown matcher,
which differs from classic escaped-delimiter handling. Table/doc-string ordering
and duplicate rejection follow the shared43 grammar. Native AST alone currently
loses delimiter/media/tag-cell metadata and cannot faithfully reconstruct this.

## Proposed integration

1. Add a standalone `gherkin::markdown` module to the existing maintained parser
   fork, enabled by its existing parser feature. No Node or Messages dependency.
   Expose `MarkdownDocument::parse(source, default_language)` plus path-aware
   equivalent and typed error positions. Use the immutable official43 catalogue.
2. Parse original source with a Rust matcher and grammar-state parser honoring
   official matching order. Build a typed source document (optional feature,
   optional feature keyword, ordered children/arguments, exact locations and
   spans, tags, tables, delimiter/media/content), then project it to the existing
   native `Feature` API for execution. Do not rewrite input and reparse classic
   text. Native projection may use an empty keyword for the fallback Feature;
   preserve the actual absence in the typed source model.
3. Keep source-format selection explicit and separate from semantic policies:
   propose `parser::SourceFormat::{Plain,Markdown,Auto}` and
   `Basic/Cucumber::with_source_format`. Default Plain remains untouched. Auto
   uses `.feature.md`/`.md` for Markdown and `.feature` for classic, rejects unknown
   extensions, and discovers `.feature`/`.feature.md` in directories. Explicit
   Markdown accepts supplied source paths. This directory policy is intentional
   and documented rather than falsely claiming exact JS path-expansion parity.
4. SourceStore records exact original bytes and the parsed Markdown typed model
   alongside its native Feature projection. Capture format metadata on invalid
   input too so error source envelopes use the correct media type. Metadata must
   survive generations and avoid re-reading files. Parser availability never
   depends on `output-messages`.
5. Add a narrow SourceGraph adapter from the typed Markdown model into existing
   graph/compiler structures; reuse Pickle compilation and native case matching.
   Keep classic scanner behavior unchanged. Preserve absent Feature keyword,
   all source locations, example links and both argument orders. Writer source
   envelopes use `text/x.cucumber.gherkin+markdown`; existing34/35 protocol
   selection and execution IDs remain unchanged.

No new fork is required. A typed source representation is necessary architectural
work, not an external blocker. The current native Feature fields are insufficient
for complete protocol fidelity, and modifying them in place would break public
struct consumers. The additive representation avoids that break. Avoid migrating
the classic parser wholesale within this phase.

## Bounded acceptance before implementation

Author BDD before production edits. Compare all five untouched official source,
AST and Pickle expectations (only URI and bijective ID normalization permitted).
Cover actual native handler execution for ordinary steps, outline values, tables
and four-backtick doc strings; source media type and successful run references.
Add boundary scenarios for localized fallback, ignored language-looking prose,
all bullet prefixes, prefix/Unicode columns, table indentation and separator
rules, variable fence closure, combined arguments in both orders, and duplicate
arguments/invalid table widths. Where the corpus has no expectation, invoke the
pinned official43 Markdown parser only inside the completed phase-end BDD.
Preserve plain corpus62, featureless documents11 and selected affected source
reporting BDD at the same phase end. No generic tests or premature oracle probes.

Exact generated diagnostic wording/recovery multiplicity, CommonMark rendering,
Cucumber.js support-file loading, and all arbitrary Markdown programs remain
outside this bounded claim. Full positive corpus coverage is five fixtures, not
proof of every matcher state. Before implementation, agree whether28 includes
runtime/writer integration or splits native28a and integration28b; either must
finish its complete acceptance harness before its first execution.
