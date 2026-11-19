# Phase 1 result and continuation

Historical checkpoint: Phase 2 supersedes the local override requirement below;
see [portable integration](PHASE2.md). Phase 3 adds [ordered hooks](PHASE3.md).

Completed locally 2026-11-17. Initial checkouts were clean. No code was pushed,
no PR/merge/deployment was created, and existing CI workflows were not changed.
This completes a bounded first phase, not full Cucumber.js parity or complete
Gherkin 43 conformance. See [capability matrix](CAPABILITIES.md).

## Implemented and verified

- `step::Matching::KeywordIndependent` searches every keyword group and reports
  ambiguity across groups. `KeywordSpecific` remains the default.
- Configuration is available via `Collection::matching`, `Basic::step_matching`,
  and `Cucumber::step_matching`; collection cloning retains its policy. Set the
  mode after replacing a collection with `.steps(...)`.
- Local gherkin parser accepts one table and one doc string in either order for
  all step keywords, retaining both native AST values and following-step boundaries.
- BDD covers all 9 registration/keyword combinations; captures; ambiguity without
  body execution; unknown definitions; legacy default behavior; And/But/wildcard
  and French keywords; combined arguments; outline substitution; invalid repeated
  arguments; existing zero/single arguments; escaped table cells.

## Phase-end BDD evidence

Scenarios were authored before implementation. Execution began only after the
runner/parser implementation and focused harnesses were complete. No unit suite,
generic cargo test, lint, check, formatting check, or premature build was run.
Compilation occurred only as part of the following focused BDD commands.

From cucumber: `bash scripts/phase1-bdd.sh`, which runs:

```sh
cargo --config 'patch.crates-io.gherkin.path="../gherkin"' test --test compatibility -- --tags @phase1
(cd ../gherkin && cargo test --test step_arguments -- --tags @step_arguments)
```

| Target | Observed outcome |
|---|---|
| cucumber compatibility | 2 features, 20 scenarios passed, 115 steps passed |
| gherkin step_arguments | 1 feature, 18 scenarios passed, 69 steps passed |
| Total | 38 scenarios passed, 184 steps passed; no skipped acceptance steps |

The first integrated attempt had 17 passed / 3 failed scenarios. The three
failures exposed an incorrect harness expectation about existing native
`gherkin 0.16` doc-string content: it retains delimiter-adjacent newlines.
Production behavior was preserved, and the acceptance adapter was corrected to
assert exact `"\ninvoice 42\n"`, rather than trim the value or weaken the
assertion. Parser acceptance also asserts exact `"\nreusable payload\n"`.
Official content normalization is explicitly deferred.

The second combined run exited 0. It exposed one Rust 2018 panic-format warning
in the parser harness; that diagnostic formatting was corrected. The parser
BDD target alone was then rerun: exit 0, 18/18 scenarios and 69/69 steps, with no
warning. The runner source and harness have not changed since its passing run.

Raw evidence: [initial failed attempt](evidence/phase1-first-attempt.log),
[passing combined run](evidence/phase1-combined.log),
[final parser run](evidence/phase1-parser-final.log), and
[source SHA256 snapshot](evidence/source-sha256.txt).

Execution toolchain: `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, aarch64 macOS.
Manifest MSRV is 1.88; an actual MSRV run was not performed.
Resolved runner dependencies: local cucumber/codegen 0.23.0, local gherkin
0.16.0, cucumber-expressions 0.5.0, peg 0.6.3, regex 1.13.1.
The parser's existing dev harness resolves cucumber/codegen 0.22.1 and their
registry gherkin 0.15.0; its step bodies directly invoke the changed local
`gherkin 0.16.0`. The integrated runner target proves the two changed local
crates work together. Generated Cargo.lock files remain local/ignored per the
repositories' existing policy.

## Dependency fork and local adoption

Exactly one dependency fork was needed and created:
[**GQAdonis/gherkin**](https://github.com/GQAdonis/gherkin), from
`cucumber-rs/gherkin`, base `4c30e3a48a0226bf907053a0a9e738e91cf06727`.
Clone: `/Users/gqadonis/Projects/references/gherkin` (same underlying path as
`/Volumes/T7/Projects/references/gherkin`). Changes remain uncommitted locally;
the remote fork contains the upstream base, not these implementation changes.
No other dependency fork is justified by Phase 1.

The existing runner fork is `/Users/gqadonis/Projects/references/cucumber`, base
`d95872541ee2a43d71089c64bd3fa8848e24c922`. To reuse both local changes, consuming
application workspaces must select this local cucumber and explicitly patch
crates.io gherkin at their workspace root, for example with paths relative to
that application's manifest:

```toml
[dev-dependencies]
cucumber = { path = "../cucumber" }

[patch.crates-io]
gherkin = { path = "../gherkin" }
```

Then configure `YourWorld::cucumber().step_matching(
cucumber::step::Matching::KeywordIndependent)` in that application's BDD runner.
The phase script supplies the local parser override explicitly. The runner's
normal dependency remains `gherkin = "0.16"`; without an override it still uses
the released parser. Do not assume a plain dependency on the GitHub fork enables
the unpublished parser changes. A portable remote revision pin requires a
separately authorized publication step; no such step has been taken.

## Skill reused

Copied `~/.claude/skills/ci-cd-and-automation/SKILL.md` (symlink target
`~/.agents/skills/ci-cd-and-automation/SKILL.md`) to project
`.agents/skills/ci-cd-and-automation/SKILL.md`, preserving the source.
One file, no companion references. Added explicit BDD-only / phase-end-only
scope that overrides generic unit/early gates. Reused repeatable scoped commands,
captured failure output, and retained evidence; no workflow/CI configuration claim.
The deployment-specific ArgoCD skill was found but left untouched.

## Precise continuation

1. Start from these uncommitted changes and read CAPABILITIES.md plus the parser
   checkpoint. Do not rerun generic suites. Preserve the phase-end-only BDD rule.
2. Adopt a bounded official AST/Pickle interoperability contract before Messages
   output. Pin the protocol deliberately: JS 13.3.0 uses Messages 34.2.1, whereas
   Gherkin 43 uses the 35 series. Do not silently mix schemas.
3. Author acceptance scenarios for stable IDs/source links, source locations,
   argument order, doc-string media type/content normalization, outlines, tags,
   and backgrounds using tagged official fixtures. Separate compatibility
   representation from the existing public Rust AST so consumers remain stable.
4. Implement only that adopted representation/compiler phase, then run its
   focused BDD acceptance at the end. Carry deferred cases explicitly.
5. Subsequent phases: schema-valid Messages NDJSON with official consumer BDD;
   ordered lifecycle hooks and failure/teardown behavior; process/async scheduling,
   isolation and retries. None is implemented or verified here.

Browser adapters remain decoupled. Existing agent-browser wraps Node Playwright;
no pure-Rust claim, browser migration, or San Saba application change was made.

No access or approval blockers remain. Full parity requires the deferred phases;
remote reuse of these exact changes requires separately authorized publication.
