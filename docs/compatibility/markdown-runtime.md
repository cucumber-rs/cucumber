# Phase28b: explicit Markdown runtime integration

`parser::SourceFormat::{Plain, Markdown, Auto}` is selected with
`Basic::with_source_format` or `Cucumber::with_source_format`. Plain remains the
default and accepts classic source at arbitrary caller-supplied paths. Explicit
Markdown similarly accepts arbitrary paths, using the native43 Markdown parser
and official outline interpolation. It never invokes Node.

Auto dispatches `.feature` to Plain and `.md` to Markdown (case-insensitive), and
rejects other extensions before reading bytes or assigning a source media type.
Directory discovery is explicit: Plain finds `*.feature`, Markdown finds
`*.feature.md`, and Auto finds both. An explicitly selected `.md` path or glob
can parse other Markdown filenames; unrelated README.md files are not discovered
by default. This documented policy does not claim identical Cucumber.js glob or
extensionless-directory behavior.

```rust,ignore
MyWorld::cucumber()
    .with_source_format(cucumber::parser::SourceFormat::Auto)
    .with_messages(output)
    .run("features")
    .await;
```

Source syntax is separate from classic policy builders and runner policies.
For official empty-scenario background behavior also select `with_gherkin43()`
or `with_background_semantics(Gherkin43)` on the Basic runner. Markdown uses the
global fallback language; language-looking Markdown prose does not change it.
Unsupported selected languages fail visibly, without silent replacement.

SourceStore captures exact input bytes and resolved `input_format` before parsing,
including invalid Markdown. Successful SourceSnapshot and EmptyDocumentSnapshot
records expose `source_format()` and `markdown()` typed metadata. Earlier Arcs
remain immutable across replacement or parse failure. Failed reads and unresolved
Auto paths clear current input/format/AST records. Source-backed Messages derives
AST/Pickles from the retained typed model, emits the Markdown source media type,
and reports positioned native errors through the existing public error variants.
No file reread or Markdown-to-classic text rewriting is involved.

The native parser's private ParseError detail supports additive positioned messages;
legacy classic diagnostic formatting is unchanged. Full diagnostic prose/recovery
parity is not claimed. Both source and execution graph comparisons retain original
locations and source identity. An absent fallback Feature keyword stays absent,
as required by the official Markdown fixture; full schema compliance of that
upstream edge case is not claimed by fabricating a keyword.

Acceptance was authored before production changes. Focused phase-end command:

```sh
cargo test --features output-messages --test messages_markdown_runtime -- --tags @phase28b
```

The harness executes real Rust handlers for all five official Markdown fixtures,
compares their text/table/doc-string values against Pickles, and compares complete
AST/Pickle expectations. It additionally covers mixed discovery, custom paths,
unresolved Auto rejection, exact invalid bytes/error media, deleted sources,
empty documents, French fallback and immutable typed generations. Observed phase-end BDD passed **14 scenarios / 54 steps**. All five official
fixtures ran real Rust handlers and matched complete graph expectations. The
successful command reported no compiler warnings. See
[BDD evidence](evidence/phase28b-markdown-runtime-bdd.log).

The initial completed boundary stopped during compilation on two ScenarioId
import paths in parallel Phase29; its owner corrected those before this focused
retry. No Markdown runtime scenario ran in that failed attempt. The
[initial diagnostic log](evidence/phase28b-markdown-runtime-initial.log) is retained.
Affected classic/source regression execution is coordinated in the final shared
Phase28b/29/31 batch rather than repeated before those owners' boundaries.

Automatic approval review initially rejected a combined edit because it misread
shared writer ownership. The command did not execute; explicit parent coordination
confirmed Phase28b ownership and the once-only media-edit retry succeeded. There
is no outstanding approval blocker. No changes were published.
