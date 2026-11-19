# Phase 6a: exact source snapshots

Adopted before implementation, 2026-11-17. Preserve exact UTF-8 source bytes
(including absence of EOF newline), configured default dialect, and raw native
Feature before outline expansion. The parser must parse those same bytes rather
than reread. A shared SourceStore gives writers immutable snapshots even after
the original file changes or is deleted. Reusing a store can replace its current
entry while existing Arc snapshots remain immutable. Existing native AST fields
and default parser API/behavior remain unchanged; capture is opt-in.

SourceStore is per-run infrastructure, not an on-disk cache or an official AST.
The source-faithful Messages/AST/Pickle conversion consumes it in Phase 6b.
Capture parse errors and full lifecycle identity are later contracts unless
explicitly adopted. SourceStore should not be shared by concurrent independent
runs using the same URI; use a fresh store per run.

Acceptance feature authored first: tests/features/sources/snapshots.feature.
Execute only after complete implementation/harness:
`cargo test --test sources -- --tags @phase6_sources`.
Status: implemented and phase-end BDD passed: **5 scenarios / 20 steps**, exit 0.
[Observed evidence](evidence/phase6-sources.log).

Files: parser/source.rs (immutable snapshots), parser/basic.rs and parser/mod.rs
(capture integration), Cucumber::with_source_store, and
`gherkin::Feature::parse_file_source` in the maintained fork and vendor snapshot.
The new Gherkin API shares the same file-path diagnostics and EOF normalization
as parse_path, without rereading. Native AST fields and doc-string values are
unchanged. The outer BDD runner also exercises the normal noncapturing parser.

The first phase-end command could not start because the future messages_source
target had been registered before its source file existed; registration was
removed pending completion. The next attempt exposed a harness lifetime error:
its borrowed dialect argument did not satisfy the existing owned/static language
API. Passing an owned String fixed the harness; production code needed no
correction. Both diagnostics are retained in evidence/phase6-sources-initial.log
and evidence/phase6-sources-compilation.log. Only this focused BDD target was
rerun. No unit suites or premature checks ran.

Use a new parser::SourceStore for each run and pass clones to the Basic parser
(or Cucumber builder) and the source-aware reporting consumer. Snapshot getters
return exact source, raw pre-expansion Feature, and configured fallback dialect.
A source directive can override the fallback dialect. Snapshot capture is not a
claim that the native AST itself has gained full official metadata.

