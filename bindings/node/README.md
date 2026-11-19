# Local Node/TypeScript native SDK

This private package belongs to the **GQAdonis Cucumber fork**. It exposes the
shared Rust source compiler and rerun planner through an in-process Node-API
addon. It is not an official Cucumber package and does not execute JavaScript
step definitions or provide complete Cucumber.js parity.

The first SDK phase targets Darwin arm64, Node 26.5.0 and Rust 1.88.0. Implementation
alone is not certification: see the repository's phase-46 BDD evidence and review
report for verified behavior. Other platform/runtime combinations are unverified.

## Public API

```ts
import { compile, rerun, capabilities, request, SdkError } from 'cucumber-native-local';

const graph = compile({
  source: 'Feature: Example\n  Scenario: Greeting\n    Given a greeting\n',
  uri: 'example.feature',
  format: 'gherkin',
  defaultLanguage: 'en',
});
console.log(graph.gherkinDocument, graph.pickles);
console.log(capabilities().foreignStepExecution); // false
```

`compile` requires all four options. It accepts source text, never a filesystem
path; the URI is an identity preserved exactly. `format` is `gherkin` or
`markdown`; there is no syntax guessing. The returned AST/Pickle graph retains
upstream optional fields. Its open JSON types reflect that boundary rather than
claiming universal conformance to a particular Messages schema.

`rerun({ ndjson, separator })` accepts original Messages NDJSON text. Do not
`JSON.parse` and reserialize those Messages first: JavaScript can round large
input integers. Attempt and line counters in the result are decimal strings;
`runSuccess` is independent of selected rerun entries. An empty `entries` array
means no selected scenarios, never an instruction to execute all sources.

`compile`, `rerun`, and `capabilities` return result objects or throw `SdkError`
for core protocol errors. Its `code` is stable; optional `uri`, `line`,
`referenceKind`, and `referenceId` are available both directly and in `context`.
Error prose is not a compatibility guarantee. A normal invalid request does not
poison a later request. JavaScript serialization/type errors and native loading
errors remain host errors rather than fabricated core protocol failures.

`request(raw: string): string` directly exposes the native versioned JSON
transport, returning a JSON success/error envelope unchanged. It is useful for
protocol consumers, including intentionally invalid requests. Rust validates
requests and owns parsing, rerun grouping, error precedence and precision.

## Local packaging boundary

The independent Cargo project produces `cucumber_sdk_node` as a `cdylib` using
pinned NAPI-RS versions. At the completed phase boundary, the acceptance preparer
builds it and copies the host dynamic library to `cucumber_sdk.node` next to
`index.js`, then packs and installs the resulting local npm archive into a fresh
outside-checkout consumer. The native artifact must be present before packing;
`npm pack` by itself does not build it. See `sdk/acceptance/prepare.py` in the
repository for the orchestrated packaging boundary. Do not run builds or checks
before the whole phase implementation is complete.

The package has no install scripts, binary downloads, subprocess adapters or
parser fallbacks. Loading requires `./cucumber_sdk.node`; a missing or
incompatible artifact fails immediately. A tarball is specific to the native
host it was built for and is not a portable prebuilt distribution.

The provisional local name `cucumber-native-local` at `0.0.0` is private and is
not registered or published. Public names, ownership, target platforms, registry
access and publication scope require separate owner decisions. Existing core
build/release workflows are unchanged. Rust extensions are owned by the Rust
SDK; this package exposes no host callback registration or foreign step execution.

## Attribution

See `NOTICE`, the package licenses and `licenses/` for retained core/parser
attribution. Dependency inventory and license review must be tied to the final
resolved native artifact before public distribution. Source-reference baselines
are Gherkin 43.0.0, Cucumber.js 13.3.0 and Messages 34.2.1/35.0.0; they are not a
claim of complete framework parity or upstream endorsement.
