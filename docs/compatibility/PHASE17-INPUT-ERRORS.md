# Phase 17: standard reports for failed inputs

The Basic parser captures exact readable source before parsing. `SourceStore::input`
retains it even if syntax or example expansion fails; `get` remains the successful
raw-AST snapshot accessor. A new read invalidates the current generation while
retained snapshots remain immutable. Reporting never rereads the failed source.
The fork exports `ParseError::position` for structured native diagnostics.

Messages reports syntax and legacy expansion failures as `parseError` envelopes
with native locations and captured `source` when available. No AST, Pickle or
TestCase is invented for the failed input. Input I/O failures become an unsuccessful
`testRunFinished` with `InputReadError` exception text and no fabricated source.
Pre-start errors are queued until metadata/run start. Valid scenarios may still
report alongside invalid sources, but the overall run remains unsuccessful.

Feature-first acceptance covers deleted/rewritten failed input, legacy expansion,
missing files and a mixed valid/invalid directory, with independent schema checks.
Run only at phase end:
`cargo test --features output-messages --test messages_errors -- --tags @phase17`.
Result: **6 scenarios / 30 steps passed**, plus immutable-source regression
**5 scenarios / 20 steps passed**, exit0. The initial boundary compile caught a
harness generic input type erased by `with_parser`; specifying the constructor's
input type fixed it. All diagnostic evidence is retained under
`evidence/phase17-input-errors-*.log` and `phase17-sources-regression.log`. Native diagnostics are preserved; this does not claim identical
parser error wording/locations across Gherkin implementations.
