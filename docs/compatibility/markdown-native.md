# Phase28a: native Markdown parsing and source graphs

This phase adds `gherkin::markdown::MarkdownDocument::parse(source, language)`
under the parser crate's existing parser feature. It returns typed source nodes
with original positions, tags, table cells, doc-string media and delimiter,
argument order by location, and optional fallback Feature keyword. `to_feature`
and `to_feature_at` project that model into existing native execution structures.
Native doc strings contain the actual Markdown content; they do not inherit the
classic parser's legacy delimiter/newline representation.

`SourceGraph::parse_markdown_gherkin43` and `from_markdown_document` adapt these
nodes into complete source graphs, using the existing Pickle compiler. The
classic scanner and default parser behavior remain unchanged. The implementation
has no JavaScript/Node runtime requirement. The pinned official Node parser is
invoked solely by phase-end BDD for boundary differential acceptance.

This remains an explicit direct API. Basic/Cucumber format selection, source-store
Markdown records, and runtime Messages source media types are now integrated by
[Phase28b](markdown-runtime.md), with its own acceptance evidence. Users of the
native parser do not need to enable output-messages.

The adopted positive corpus is all five official43 Markdown fixtures; expectations
are unchanged. Comparisons preserve field presence, original lines/columns,
keywords, tags, arguments, delimiter length, all IDs and their reference graph
(with a bijective renaming only). Fifteen additional boundaries exercise valid
matcher behavior and invalid arguments with the pinned official parser. Native
projection assertions additionally compare original step positions and arguments.
See [design/provenance](markdown-plan.md) and the fixture manifest.

The API preserves typed metadata rather than rewriting Markdown into classic
Gherkin. Ordinary prose is ignored, language is selected globally, and a first
fallback title has no source Feature keyword (the native projection uses an empty
string). Diagnostic text/recovery multiplicity and all arbitrary Markdown states
are not claimed by this bounded corpus. Full schema validation cannot be claimed
for arbitrary official Markdown ASTs: the official misc fixture itself omits
Feature.keyword, which standard schemas may require; source fidelity keeps that
absence rather than fabricating a keyword.

Phase-end command (run only once the complete bounded implementation is ready):

```sh
cargo test --features output-messages --test messages_markdown -- --tags @phase28a
```

First completed phase-end BDD passed **21 scenarios / 78 steps**, with no
compiler warnings. All five official graphs matched in full, including the
fallback Feature's absent keyword. All ten valid live-oracle boundaries and
five invalid boundary rejections agreed with the pinned official parser.
Evidence: [Markdown BDD](evidence/phase28a-markdown-bdd.log).

Affected BDD regressions passed **62 / 236** for the complete classic corpus,
**11 / 41** for featureless document execution, and **19 / 80** for rich/localized
source graphs. [Regression evidence](evidence/phase28a-markdown-regression.log).
No generic/unit suite, Node runtime integration, or publication was performed.
