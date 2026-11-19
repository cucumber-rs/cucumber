# Independent Phase37 seeded-order oracle

These development-only sources are unchanged npm artifacts for
`knuth-shuffle-seeded`1.0.6 and `seed-random`2.2.0. Their original licenses and
package manifests are retained. `provenance.json` records registry URLs, published
tarball integrity/SHA1, independently computed tarball SHA256, source-file SHA256,
and the published git head where available. Acquisition verified both published
integrity algorithms without executing package code or lifecycle scripts.

The explicit string-seed algorithm is the dependency chain selected by pinned
Cucumber.js13.3.0. The local Rust port must be compared with this independent
implementation only at the completed phase-end BDD boundary. No npm dependency,
Node process or JavaScript runtime is used by the Rust production library.

`oracle.mjs` accepts JSON `{ "items": [...], "seed": "..." }` on stdin and writes
the shuffled array as JSON. Its minimal CommonJS loader loads the two unchanged
sources from tracked fixture folders and rejects unknown dependencies. This avoids
ignored `node_modules` paths and any package installation step. Explicit empty
string seeds are supported; absent/non-string seeds are rejected. This does not
claim the JS CLI's generated-seed spelling or worker isolation.

Acquisition and wrapper authoring did not execute the oracle or precompute any
permutation. The first execution belongs to the Phase37 BDD evidence.
