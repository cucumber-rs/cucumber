# Phase 3: ordered scenario lifecycle hooks

Contract adopted before implementation, 2026-11-17. Baseline:
https://github.com/cucumber/cucumber-js/blob/v13.3.0/docs/support_files/hooks.md.
Before hooks follow registration order; after hooks run in reverse order and
continue through failures. Conditional hooks evaluate inherited feature/rule/
scenario tags. Each retry runs its hooks with a fresh World. Existing single
`.before` / `.after` methods retain replacement semantics.

Implement a reusable `runner::Hooks<World>` registry, with named before/after
callbacks and optional tag expressions, installed by `.with_hooks(...)` on
Basic or Cucumber. Installation replaces both existing hook slots; a later
legacy `.before` or `.after` replaces that stage. Empty stages remain absent so
an after-only registry does not eagerly initialize World.

Bounded reporting: the existing event protocol reports each composed stage as
one hook. All after-hook panic diagnostics are retained in that stage's error.
Every after callback receives the native pre-teardown `ScenarioFinished` reason;
per-hook result updates, distinct hook IDs/events, timeouts, imperative skip,
BeforeAll/AfterAll and BeforeStep/AfterStep are separate later work. Do not claim
full JS lifecycle parity for this composite registry.

Acceptance was authored first in tests/features/hooks/ordered.feature. No tests,
builds or checks may run until the complete implementation and BDD harness are
ready. Phase-end target: `cargo test --test hooks -- --tags @phase3`.

Status: implemented and verified locally at phase end. `cargo test --test hooks
-- --tags @phase3` exited 0: **9 scenarios passed, 42 steps passed**, no failed
or skipped acceptance steps. No source corrections were needed after this run.
Raw evidence: [phase3-bdd.log](evidence/phase3-bdd.log).

Files: `src/runner/hooks.rs`, `src/runner/mod.rs`, `src/runner/basic.rs`,
`src/cucumber.rs`, `Cargo.toml`, `tests/hooks.rs`, and the feature above.

Example:

```rust
let hooks = cucumber::runner::Hooks::new()
    .before("seed account", |_, _, _, world: &mut YourWorld| Box::pin(async move {
        // Seed this scenario's World.
    }))
    .after("release account", |_, _, _, finished, world| Box::pin(async move {
        // Release resources even when setup or a step failed.
    }));
YourWorld::cucumber().with_hooks(hooks).run("features").await;
```

Use `before_if` / `after_if` with parsed `gherkin::tagexpr::TagOperation`
expressions for conditional registration. Register teardown in acquisition
order; it executes in reverse. `Hooks` can be cloned to reuse registrations;
captured state is shared according to the closures supplied by the application.
All callbacks are local async futures, matching the runner's existing contract.

No new dependency forks, remote writes or deployments. Remaining lifecycle work
is the bounded reporting/timeout/run-and-step-hook list above, not a claim of
complete Cucumber.js lifecycle equivalence.
