# Phase 2: portable parser integration

Bounded contract: a relocated Cucumber source checkout carries the Phase 1
parser implementation. A consuming application needs only a path dependency on
that checkout, without a sibling `gherkin` checkout or `[patch.crates-io]`.

The `@phase2 @portability` acceptance feature was authored before production
changes. Its scenario copies Cucumber's production source, codegen source, declared test
sources and vendor snapshot into a temporary location, creates an unrelated consumer, and
runs that consumer to assert exact table-before-docstring values and following
step boundaries. The consumer has no Cargo patch/configuration and its lockfile
must resolve gherkin as a path package. Compilation occurs only inside this
phase-end BDD scenario. Offline execution expects already cached dependencies.

## Implementation

- `gherkin = { version = "0.16", path = "vendor/gherkin" }` in root Cargo.toml.
- `vendor/gherkin` includes all parser source/languages, build script, licenses,
  README/changelog, production dependencies and optional features. The snapshot
  excludes parser dev dependencies and tests to avoid a dev cycle into Cucumber.
- The vendor crate is excluded from the Cucumber workspace.
- See [provenance](../../vendor/gherkin/PROVENANCE.md) and
  [source hashes](../../vendor/gherkin/SHA256SUMS). The separate GQAdonis/gherkin
  fork remains the maintenance source of truth. Its changes are unpublished.
- `scripts/phase1-bdd.sh` now runs the same focused runner acceptance without
  patches or a sibling checkout. Parser-fork BDD can still be run explicitly
  inside that fork at an authorized phase boundary; it is no longer an implicit
  requirement of a portable Cucumber checkout.
- `scripts/phase2-bdd.sh` runs portability acceptance followed by full Phase 1
  runner acceptance against the vendored parser.

## Reuse and limitations

An application can use `cucumber = { path = "../cucumber" }` and obtains the
vendored parser automatically. A Git dependency will behave similarly once a
revision containing these changes is separately authorized for publication;
this phase does not publish changes or claim the current remote includes them.

Cargo registry publication strips path dependencies and resolves their registry
version. The existing `0.16` requirement would select the released parser and
lose this behavior. Before any registry release, publish an appropriate parser
version and update the version dependency. Including the vendor directory in
Cargo's package include list does not solve registry resolution.

No new dependency forks, CI/deployment changes, messages protocol support or
additional Gherkin conformance is claimed by this phase.

## Phase-end evidence

Completed 2026-11-17 with `bash scripts/phase2-bdd.sh`, exit 0:

| Focused BDD target | Observed outcome |
|---|---|
| portability (`@phase2`) | 1 feature, 1 scenario passed, 5 steps passed |
| compatibility (`@phase1`, vendored parser) | 2 features, 20 scenarios passed, 115 steps passed |
| Phase-end total | 21 scenarios passed, 120 steps passed |

Evidence: [passing run](evidence/phase2-bdd.log). Both argument values and the
following step were observed by a separately compiled consumer using the
relocated source and a relative gherkin dependency. The application had no
sibling parser checkout or Cargo patch. This establishes local source portability;
no remote Git revision or crates.io package was published or exercised.

The [first attempt](evidence/phase2-first-attempt.log) failed in the relocation
fixture: Cargo validates explicitly declared test source paths even when the
crate is being consumed as a library. The fixture initially copied production
source only. It now includes root/codegen test sources as part of the relocated
checkout; the consuming application's `cargo run` does not execute those tests.
Production parser/dependency code did not require a correction.

The separate parser fork's 18 Phase 1 scenarios were not rerun here: its source
is unchanged, the vendored production files are byte-for-byte copies recorded by
SHA256SUMS, and this phase reran all 20 integrated runner scenarios against that
snapshot. The original parser BDD evidence remains in PHASE1.md.

No unit suite, generic cargo test, lint/check, or premature build ran. The only
compilation was part of focused phase-end BDD and its external-consumer action.
No remaining execution blocker. Registry release dependency resolution and
publication remain explicitly outside this completed phase.
