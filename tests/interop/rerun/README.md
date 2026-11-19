# Pinned Cucumber.js rerun oracle

This isolated development fixture installs exactly `@cucumber/cucumber` 13.3.0.
The lock records all transitive versions and integrity digests. The copied
upstream MIT license and provenance JSON retain source/tag/file hashes.

Preparation only (lifecycle scripts disabled):

```sh
npm ci --ignore-scripts --no-audit --no-fund --prefix tests/interop/rerun
```

`oracle.mjs` directly instantiates the installed, unchanged RerunFormatter and
EventDataCollector, supplies a real EventEmitter, and feeds the acceptance
stream. Its adapter only supplies formatter plumbing and serializes results;
it does not implement selection, aggregation, source resolution or formatting.
The package and oracle must be imported/executed only by the completed Phase44
BDD harness. No project workflow or production dependency was changed.
