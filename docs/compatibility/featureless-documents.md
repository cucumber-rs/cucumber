# Phase25: explicit latest document execution and reporting

`parser::Basic::with_gherkin43()` selects the currently adopted latest dialect,
doc-string, table and outline policies and accepts validated featureless source.
`Cucumber::with_gherkin43()` over its Basic parser/runner also selects official
empty-scenario background execution. Both return an error if an already selected
fallback language is absent from the latest catalogue; neither silently replaces
that language. Step-definition matching remains its separately selected policy.

```rust,ignore
MyWorld::cucumber()
    .with_gherkin43()?
    .with_messages(output)
    .run("features")
    .await;
```

The bundle means these implemented policies, not untested full language/runtime
parity. Existing constructors remain legacy; the separate policy builders remain
available. Default Feature-only parsing still rejects empty files.

Valid blank, whitespace-only and comment-only documents yield no Feature stream
item. Their exact source is captured in immutable `EmptyDocumentSnapshot` records
with path and fallback dialect. `SourceStore::empty_document(path)` reads the
current record; `empty_documents()` returns current records sorted by path.
`get(path)` remains a successful Feature snapshot accessor, and `input(path)`
remains all currently captured read bytes. Replacement, invalid parsing and failed
reads clear the current empty record; retained Arcs preserve earlier generations.

Messages emits each captured empty source and its official GherkinDocument before
run completion, without synthetic Features, Pickles, cases or execution events.
Mixed source directories still execute their ordinary features. Source deduping
and one record per path prevent repeated source/document envelopes. Use a fresh
store per independent run, as with existing feature snapshots. Retained record
conversion is available through `SourceGraph::from_empty_document` without a
filesystem reread or policy reinterpretation.

A bare language directive is not treated as a harmless comment: the pinned
Gherkin grammar requires FeatureLine after its Language token. Native Document
parsing and runtime report errors for it; acceptance checks the actual pinned
JavaScript parser, alongside arbitrary noncomment input. Native path-aware
`Document::parse_file_source` is additive and preserves Feature path/error identity.

The phase's scenarios and harness precede execution. Focused BDD covers exact
source/no-final-newline/CRLF, complete empty graphs, mixed documents, zero cases,
error reports, immutable generations, fallback validation and preserved legacy
behavior. The actual oracle runs only inside phase-end BDD.

```sh
cargo test --features output-messages --test messages_documents -- --tags @phase25
```

Completed phase-end BDD: **11 scenarios / 41 steps passed** on the first run.
The live pinned official oracle confirmed valid empty graphs and rejection of
bare language directives. All emitted envelopes satisfied the Messages 34.2.1
schema. Evidence: [primary log](evidence/phase25-documents-bdd.log).

Affected BDD regressions also passed: complete official plaintext corpus
**62 scenarios / 236 steps**, Messages input errors **6 / 30**, and source
snapshot/policy integration **21 / 81**. See the
[regression log](evidence/phase25-documents-regression.log). Neither command
reported compiler warnings. No generic/unit suite or publication was performed.

The bundle remains explicitly opt-in. Exact diagnostic wording and all arbitrary
Gherkin programs are not claimed by this bounded acceptance; independent runs
must use fresh stores to avoid mixing current records from different runs.
