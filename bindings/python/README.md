# Local Python native SDK

This is an unpublished Python adapter for the **GQAdonis Cucumber fork**. The
native extension delegates compilation and rerun planning to `cucumber-sdk`.
Python only constructs protocol requests and decodes responses. It does not
parse Gherkin or Messages, execute step definitions, or launch another process.
It is not an official upstream Python distribution.

The adopted phase-46 boundary targets Gherkin 43.0.0 source graphs, Cucumber.js
13.3.0 rerun behavior, and Messages 34.2.1/35.0.0 interoperability. These are
compatibility targets, not claims of complete framework parity. Behave and
pytest-bdd adapters and foreign step execution remain following-phase work.

## API

```python
from cucumber_native_local import SdkError, capabilities, compile_source, rerun

compiled = compile_source(
    "Feature: Inventory\n  Scenario: Empty shelf\n    Given an empty shelf\n",
    "inventory.feature",
)
print(compiled["gherkinDocument"])
print(compiled["pickles"])
print(capabilities()["foreignStepExecution"])  # False in this phase

try:
    compiled = compile_source("Feature: Inventory\n", "inventory.feature")
except SdkError as error:
    print(error.code, error.context)
```

- `compile_source(source, uri, format="gherkin", default_language="en")`
  returns `CompiledSource`; `format="markdown"` explicitly selects Markdown.
  Source, URI and full AST/Pickle objects remain intact; no path lookup occurs.
- `rerun(ndjson, separator="\n")` accepts the **original text**, returning
  `RerunPlan`: every attempt, ordered URI/line groups, the formatted selection,
  nullable run ID and independent run success. Attempt and line counters are
  decimal strings. An empty `entries` list means no rerun, never all sources.
- `capabilities()` returns `Capabilities` from the Rust implementation.
- `request(raw: str) -> str` exposes the unmodified version-1 JSON transport.
  Raw requests return error envelopes instead of raising `SdkError`.

Conveniences raise `SdkError` on Rust error envelopes. Its `code`, `message`
and `context` preserve structured native errors; `uri`, `line`, `reference_kind`
and `reference_id` are optional aliases. Context retains exact wire names such
as `referenceKind` and retains an explicitly empty URI. Parser error text is
not scraped to invent locations. The native module must load successfully:
missing/incompatible binaries raise an import error without fallback.

The package includes Python type hints, TypedDict result shapes and `py.typed`.
AST/Pickle graphs intentionally use open dictionaries so upstream optional
fields are not dropped or invented. Calls are synchronous, with a fresh native
SDK per request. This phase exposes no Python callback/extension registration.

## Local package preparation

The independent Cargo manifest pins PyO3 **0.30.1**, requires Rust **1.88**, and
sets `publish = false`. The build backend pins maturin **1.12.6**. The mixed
package uses `python/cucumber_native_local` and the native submodule
`cucumber_native_local._native`. Its Rust cdylib is `cucumber_sdk_python`.

Metadata permits Python >=3.9, but the adopted first verification target is
CPython **3.11.3 on Darwin arm64** only. No version or platform is claimed
verified by these source files. Full phase-end evidence belongs to the parent
SDK acceptance packet; do not run isolated builds, imports or tests early.

At that boundary the orchestrator builds a wheel using maturin, installs it
into a fresh virtual environment outside the checkout, exercises the public
API and records facade/native realpaths plus archive/native hashes. A separate
fresh process must fail visibly when its disposable installed native artifact
is removed. No unit suite is introduced here. Existing root build and release
workflows remain unchanged.

Source builds currently require this repository layout (`../../sdk` and its
core dependencies). The deliverable for this phase is an installable local
wheel; standalone source distribution and cross-platform wheels are not yet
certified. Lock generation occurs only at the complete phase boundary.

## Publication decisions

`cucumber-native-local==0.0.0` is a temporary local identity, not a reserved PyPI
name. `Private :: Do Not Upload` marks this package as private. Do not register
or publish it. Public name, account/ownership, credentials/access and permitted
publication scope still require Travis's confirmation. Public distribution
also requires complete dependency-license inventory, selected Python/platform
support and corresponding wheel evidence. No credentials belong in this tree.

The root license texts and Gherkin notices are retained in this package and
included via `license-files`; see `NOTICE` for limits of the current inventory.
No upstream endorsement or trademark permission is implied.

Implementation references: [PyO3 0.30.1 function guide](https://github.com/PyO3/pyo3/blob/v0.30.1/guide/src/function.md),
[maturin mixed project layout](https://www.maturin.rs/project_layout.html).
