# Official Gherkin 43 fixture subset

These are unmodified primary-source fixtures and expected outputs from
[`cucumber/gherkin` v43.0.0](https://github.com/cucumber/gherkin/tree/v43.0.0/testdata/good),
resolved to commit `3d70e2bb4e0f16a568ee539a5ce0b03fb247a777`.
The annotated tag object is `ef799694208f16e9e3eb9824dfdbc7df293999b1`.
Downloaded 2026-11-17 using read-only GitHub connector requests. No oracle was
regenerated with Node, Rust, a build, or a test runner.

The original [MIT license](LICENSE) is included. [manifest.json](manifest.json)
records each immutable source URL, upstream Git blob identity, SHA256 and byte
length. Downloaded fixture bytes matched their upstream Git blob identities;
this establishes artifact provenance, not parser conformance.

## Selected coverage

Each basename has three files: `.feature`, `.feature.ast.ndjson`, and
`.feature.pickles.ndjson`.

| Basename | Reason for selection |
|---|---|
| `rule_with_tag` | Feature/rule/scenario/outline/examples tags; descriptions on Rules; nested AST children and inherited Pickle tags |
| `complex_background` | Feature and Rule backgrounds; ordinary scenarios and two example rows; distinct compiled step IDs for repeated background execution |
| `descriptions` | Feature/scenario/examples descriptions, indentation and blank-line preservation, source comments |
| `several_examples` | Multiple tagged Examples blocks and a separate ordinary scenario |
| `escaped_pipes` | Unicode cells plus escaped pipes, backslashes and newlines |
| `docstrings` | Indentation/dedent, XML media type, backtick delimiter, embedded and escaped delimiters |
| `step_with_datatable_and_docstring` | Both argument orders on a single step, with `argumentIndex` preserved |
| `example_tokens_everywhere` | Two example rows substituting scenario names, step text, doc-string content and table cells |

## Expected output contract

- AST files contain one JSON object per line with the `gherkinDocument` envelope.
- Pickle files contain one JSON object per line with the `pickle` envelope,
  preserving official scenario/Examples expansion order.
- The official URI is `../testdata/good/<basename>.feature` in each envelope.
  Prefer supplying that URI to the source converter. A comparison may map only
  this known URI prefix when running fixtures from their local location.
- Preserve all fields, values, absent/present distinctions, descriptions, source
  locations, array order, keyword types, arguments/media types, tags and AST links.
  Expected files are the external oracle; do not edit them to match local output.
- Official IDs are retained as downloaded. A comparison may establish a bijection
  between expected and actual generated IDs, applying that same mapping to every
  `id`, `astNodeIds`, and `astNodeId` reference. Do not simply delete IDs or ignore
  linkage: uniqueness, repeated references and the original AST relationship
  must still match. Structurally distinct background Pickle steps retain distinct
  compiled IDs while referring to their shared source AST step.
- Preserve `Pickle.location` and `argumentIndex` on data-table/doc-string arguments.
  The Gherkin fixture release and the selected Cucumber Messages schema are
  independently pinned baselines; do not strip fields merely because the phase
  uses a different explicitly compatible Messages version.

These eight fixtures are a bounded acceptance corpus, not all official Gherkin
43 fixtures. Downloading them does not mean the converter passes them. Execution
and observed conformance belong to the phase-end BDD evidence maintained by the
conversion implementation. No Rust code, fixture parser, test suite, build, or
Node oracle was executed during corpus curation.


## Phase 8 localized addition

`i18n_fr` and `i18n_emoji` add six untouched source/AST/Pickle files from the
same pinned Gherkin 43 commit. Their provenance and hashes are recorded separately
in `localized-manifest.json`. They use the same full-field comparison contract;
only a bijection of IDs/references and URI normalization is permitted.
