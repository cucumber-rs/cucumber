# Official Gherkin 43 Markdown corpus

Pinned upstream: cucumber/gherkin v43.0.0, commit
`3d70e2bb4e0f16a568ee539a5ce0b03fb247a777`.

This is the complete `testdata/{good,bad}/*.feature.md` inventory: five good
sources (`minimal`, `misc`, `datatables`, `docstrings`, `tags`), no bad Markdown
sources. All 25 original source, source-envelope, AST, Pickle and token files are
copied unchanged. Each file was checked against its upstream Git blob SHA1 when
copied; manifest.json also records SHA256. LICENSE is upstream's license.

Future differential BDD must compare complete source/AST/Pickle envelopes,
allowing only URI normalization and a bijection of IDs/references. Preserve
absent/present keyword fields, original Markdown columns/line numbers, argument
order, delimiter length, media type, tag locations and example-row links. Token
fixtures are source-design evidence, not permission to discard graph fields.
No oracle or acceptance execution occurred during inventory/design.

The upstream positive corpus alone cannot establish invalid-input handling or
all Markdown behavior. Additional acceptance must cover matcher boundaries and
actual native execution before claiming implementation.
