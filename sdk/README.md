# Native Cucumber SDK (GQAdonis fork)

This independent, unpublished Rust crate exposes the fork's existing source
compiler and rerun collector. It is the shared implementation behind the native
TypeScript and Python bindings. No parsing, outline expansion or rerun-selection
behavior is reimplemented in those host languages.

Phase 46 implementation is awaiting its complete phase-end behavioral boundary.
The compatibility targets are Gherkin 43.0.0, Cucumber.js 13.3.0 and Messages
34.2.1/35.0.0. These identities are targets, not a claim of universal parity.
Foreign step execution, Behave/pytest-bdd adapters and registry publication are
outside this first SDK phase.

```rust
use cucumber_sdk::{CompileInput, Sdk, SourceFormat};

let sdk = Sdk::new();
let compiled = sdk.compile(CompileInput {
    source: "Feature: Example\n  Scenario: Native\n    Given a source\n",
    uri: "memory:example.feature",
    format: SourceFormat::Gherkin,
    default_language: "en",
})?;
assert_eq!(compiled.pickles.len(), 1);
# Ok::<(), cucumber_sdk::SdkError>(())
```

`Sdk::request(&str) -> String` is the version 1 JSON boundary documented by
[the adopted schema](../.kbd-orchestrator/phases/phase-46-native-sdk/sdk-contract.schema.json).
Requests contain `version`, `operation`, and `input`, with no additional fields.
`compile` requires `source`, `uri`, `format` (`gherkin` or `markdown`), and
`defaultLanguage`. `rerun` requires the original `ndjson` string and `separator`.
`capabilities` requires an empty input object.

`Sdk::rerun(ndjson, separator)` returns every original attempt, run success,
ordered URI groups and formatted projection. Retry counters and source lines
are decimal strings, preserving values beyond JavaScript's safe integer range.
Keep input NDJSON as text: parsing and serializing it in JavaScript first can
irreversibly round those counters. Empty selections mean no scenarios to rerun.

Source errors retain the supplied URI, including empty URIs. Rerun errors
preserve missing references and incomplete/already-finished run distinctions.
Any error line is a physical NDJSON line, not a fabricated source location.
Ordinary unwinding panics become `INTERNAL_ERROR`; aborts, process faults and OOM
cannot be recovered. Library-load errors occur before this JSON boundary.

## Rust extensions

Implement `Extension::name(&self) -> &str` and
`Extension::execute(&self, &serde_json::Value) -> Result<Value, SdkError>`.
Use `sdk.register_extension(extension)` to transfer ownership to an SDK instance,
then `sdk.execute_extension("verification.summary", &input)` to invoke it.
Names consist of at least two dot-separated ASCII identifiers; identifiers start
with a letter or underscore and continue with letters, digits or underscores.
The `core` namespace and duplicate registrations are rejected before mutation.
Other SDK instances cannot see these registrations.

Extensions are Rust application operations, not Cucumber step definitions or
host-language callbacks. They are separate from core request operations.
An extension's own interior state must remain valid if its implementation
unwinds; the SDK does not roll back application side effects or suppress panic
hooks. The core crate forbids unsafe code. Native adapter crates own their
respective FFI boundary and packaging.

The SDK has its own workspace and manifest. Existing root build and release
workflows are unchanged. Package names are local placeholders, `publish = false`
is deliberate, and registry publication requires separately confirmed ownership,
names and authorization. See the phase contract for the focused BDD boundary;
there are no SDK unit suites.
