# Shared native SDK programme — phase46 installed-package evidence

The verified Rust foundation through45 is preserved on both origins. Travis has
adopted the eleven-case first SDK contract; owner calibration/adoption is no
longer a blocker. The completed B5 repair2 packet built and installed actual native
packages and passed 11 functional scenarios /33 steps plus 11 negative-control
scenarios /33 steps. Outer Messages proof covers all 22 scenarios /66 steps;
cleanup and original receipts are retained. Scoped internal review and static QA resolved their material findings. No registry publication, full-program clean claim or foreign-language
step execution is claimed. See [phase46 evidence and limits](PHASE46-SDK.md).

See the [owner adoption](../../.kbd-orchestrator/phases/phase-46-native-sdk/owner-adoption.json),
[bound contract](../../.kbd-orchestrator/phases/phase-46-native-sdk/adopted-contract.json)
and [execution record](../../.kbd-orchestrator/phases/phase-46-native-sdk/execution.md).
Earlier proposal wording is historical; this status does not amend the contract.

## Implemented source and exact boundary

| Surface | Source now present | Verification status |
| --- | --- | --- |
| Shared Rust SDK | [sdk/src/lib.rs](../../sdk/src/lib.rs): `Sdk::compile`, `Sdk::rerun`, `Sdk::capabilities`, versioned JSON `Sdk::request` | Installed Rust consumer covered by repair2; delegates to through45 core operations |
| Rust extension interface | `Extension`, `register_extension`, `execute_extension`; instance-owned namespaced operations | Rust extension contract covered by repair2; no built-in replacement or host callbacks |
| Node/TypeScript | [bindings/node](../../bindings/node/README.md): NAPI-RS addon, CommonJS facade and declarations | Actual local native archive built and installed outside checkout; repair2 behavior proof retained |
| Python | [bindings/python](../../bindings/python/README.md): PyO3 extension, typed facade and wheel metadata | Actual local native wheel built and installed outside checkout; repair2 behavior proof retained |

Host facades serialize requests and decode results. Rust owns source compilation,
rerun selection, protocol validation and structured error semantics. The bindings
are in-process native adapters; they do not call a parser subprocess. The SDK
has an independent Cargo workspace, and each binding has its own manifest;
established root build/release workflows remain unchanged.

Targets remain official Gherkin43.0.0, Cucumber.js13.3.0 and Messages34.2.1/35.0.0.
The first adopted host boundary is Darwin arm64, Rust1.88.0, Node26.5.0 and
CPython3.11.3, with the TypeScript6.0.3 declaration consumer. Repair2 passed on
these exact versions; other environments remain unverified. Python metadata's
>=3.9 allowance is not proof across those versions.

## Rust usage from the authored API

The examples below illustrate APIs exercised by the installed-consumer boundary;
they are not separate acceptance runs. A sibling application can declare
`cucumber-sdk = { path = "../cucumber/sdk" }`, retaining the whole Cucumber checkout
for its core, codegen and vendored parser dependencies.

```rust
use cucumber_sdk::{CompileInput, Sdk, SdkError, SourceFormat};

fn main() -> Result<(), SdkError> {
    let sdk = Sdk::new();
    let graph = sdk.compile(CompileInput {
        source: "Feature: Inventory\n  Scenario: Empty shelf\n    Given an empty shelf\n",
        uri: "memory:inventory.feature",
        format: SourceFormat::Gherkin,
        default_language: "en",
    })?;
    println!("{} pickles", graph.pickles.len());
    Ok(())
}
```

Choose `SourceFormat::Markdown` explicitly for Markdown. Source is supplied text,
not a path to discover or reread; URI identity is retained. Full graph fields,
including actual upstream optional-field absence, come from the existing compiler.
The documented Markdown `Feature.keyword`/Messages-schema conflict remains.

For original Messages text, `sdk.rerun(original_ndjson, "\n")?` returns all
attempts, ordered URI groups, formatted selection and independent run success.
Attempt counters and source lines are decimal strings. Do not parse and
reserialize input Messages in JavaScript before calling it: that could round
large integers. Empty `entries` means no selected scenarios. Selection neither
executes a run nor grants repair authority; keep original attempts and receipts.

An application extension implements the actual Rust trait:

```rust
use cucumber_sdk::{Extension, Sdk, SdkError};
use serde_json::Value;

struct Echo;
impl Extension for Echo {
    fn name(&self) -> &str { "application.echo" }
    fn execute(&self, input: &Value) -> Result<Value, SdkError> {
        Ok(input.clone())
    }
}

fn extension_example() -> Result<Value, SdkError> {
    let mut sdk = Sdk::new();
    sdk.register_extension(Echo)?;
    sdk.execute_extension("application.echo", &Value::String("receipt".into()))
}
```

The example additionally requires the application's own `serde_json` dependency.
Names contain at least two dot-separated ASCII identifiers; `core` is reserved
and duplicates are rejected before registry mutation. The SDK owns the registered
extension per instance. Extensions are Rust application operations, not step
definitions, Python callbacks or JavaScript handlers. Ordinary unwinding panics
are mapped to `INTERNAL_ERROR`; aborts/process faults/OOM and application side
effects are outside recovery. Native library loading occurs before the JSON
error boundary. See [the SDK guide](../../sdk/README.md) for the exact contract.

## Local native artifacts and completed boundary

The completed [preparation](../../sdk/acceptance/prepare.py), eleven behavior cases
and eleven negative controls ran as the adopted whole-phase packet. Future
behavioral changes retain that phase-end BDD rule; an isolated package smoke test
or unit suite is not a substitute.

The repair2 packet completed these steps:

1. Resolved independent lockfiles under Rust1.88 and built the actual native addon.
   The host library was copied to `bindings/node/cucumber_sdk.node` before packing;
   `npm pack` alone does not build a binary. The local private archive was installed
   into a fresh outside-checkout consumer with scripts disabled.
2. Built a wheel using pinned maturin1.12.6 and PyO3 0.30.1, then installed it
   into a fresh outside-checkout Python environment.
3. Built the external Rust consumer and retained original argv, exits, logs,
   archive/native hashes, installed realpaths and dependency identities.
4. Exercised public Rust/TypeScript/Python operations, official pinned oracles and
   negative controls, including visible failure when an installed native binary
   was missing; retained outer Messages and cleanup observations.

Local artifacts are retained under
`docs/compatibility/evidence/sdk46-repair2/artifacts/`, ignored by Git; hashes and
process/install receipts are retained with the evidence. They are not registry
releases. The Node archive is host-specific; cross-platform prebuilts and a
standalone Python source distribution are not certified.

The initial Node24 runtime mismatch remains recorded. Repair1 passed its BDD
cases, but review subsequently found JSON map/type confusion and missing outer
Messages evidence. Repair2 corrected those issues and passed the full packet;
earlier evidence was not overwritten. See [the phase report](PHASE46-SDK.md). See [packaging acceptance](../../sdk/acceptance/README.md)
and the binding READMEs for current layout and provisional identities.

Both local distributions use the placeholder name `cucumber-native-local` at
0.0.0; Python imports use `cucumber_native_local`. Node is marked private, Python
has `Private :: Do Not Upload`, and Rust manifests use `publish = false`.
Names are not registered or reserved. Public npm scope/names, Python name,
ownership/access, publication scope and supported platforms require confirmation.
Resolved artifact dependency/license closure also precedes public distribution.
No credentials belong in the repository.

## Remaining compatibility work

Foreign step execution, captured registrations/World factories and asynchronous
host callbacks are not implemented in this first SDK phase. Both Behave1.3.3 and
pytest-bdd9.0.1 remain separate unverified optional adapter tracks with different
Context/fixture/plugin contracts; see [the dual-target matrix](PYTHON-TARGETS.md).
Shared parsing does not establish parity with either framework or Cucumber.js
support-file execution. Rust extensions are currently exposed to Rust callers
only, not registered through the Node/Python facades.

## Historical evidence and documentation

Through45 core evidence remains valid for its preserved source snapshot; it is
not new SDK proof. Both independent Docusaurus packages passed the earlier
[documentation boundary](DOCS-SITE-ACCEPTANCE.md): 10 scenarios /64 steps,
including isolated broken-link controls. That evidence describes the prior
site source, not subsequent SDK page edits. Both fork Pages sites are now live:
[Cucumber](https://gqadonis.github.io/cucumber/) and
[Gherkin](https://gqadonis.github.io/gherkin/). Their separate public boundary passed
4 scenarios /22 steps and both repository About links were updated. These latest
page edits still need their own documentation boundary and delivery. mdBook and
mature build/release workflows remain intact.

## Review and upstream contribution gates

Scoped internal review and static QA resolved their material findings after the
completed SDK behavioral boundary. The final programme gate covers both
repositories: core/SDK correctness, FFI ownership and panic/exception handling,
callbacks/cancellation where implemented, compatibility/defaults, packaging
closure, documentation and maintainability. Retain finding IDs, severity, source
references, reviewer identity/limitations, dispositions and repair evidence.
Material behavioral repairs require their own complete BDD boundary. No clean
claim before necessary corrections and review are complete.

The retained [internal planning review](../../.kbd-orchestrator/phases/phase-46-native-sdk/review/internal-review.md)
is separate from that final code audit. Exact model deployment is unknown;
no external or cross-model certification is claimed. External project/review
packet dispatch remains prohibited and the denied dispatch will not be retried.

Then inspect current upstream policies and existing issues, separate parser
changes from runner/protocol and SDK contributions, and preserve parser-first
dependency order. Create matching issues/PRs only after completion and review.
The owner has now authorized draft contribution issues/PRs. The contribution reflection records current overlap and limits; recheck these leads before each submission:
https://github.com/cucumber-rs/cucumber/pull/391 (step hooks/reporting) and
https://github.com/cucumber-rs/cucumber/pull/405 (scenario sharding).
These leads do not block authorized preservation pushes to the forks.
