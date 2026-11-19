# Pinned official Gherkin oracle

Private acceptance tooling only. Exact dependencies and integrity are locked in
package-lock.json: @cucumber/gherkin43.0.0 + @cucumber/messages35.0.0. Install with
`npm ci --ignore-scripts --no-audit --no-fund` in this directory. node_modules is
ignored by the parent tests/interop/.gitignore.

`node tests/interop/gherkin43/oracle.mjs [uri]` reads UTF-8 source on stdin and
returns JSON `{gherkinDocument,pickles}`. It emits official parseError envelopes
to stderr and fails for rejected source. Do not run this outside a completed
BDD phase under the project's phase-first testing constraint.

The Rust Messages writer remains pinned to protocol34.2.1. Differential BDD
requires all adopted graph fields to match; only IDs/URI may be normalized.
