# Phase32a: Original built-in expressions and faithful argument groups

Status: bounded implementation and primary phase-end BDD passed: **19 scenarios /
85 steps**, exit 0 and warning-free. The coordinated affected BDD batch also
passed **174 scenarios / 751 steps** across 11 targets.

The target is Cucumber.js 13.3.0, cucumber-expressions 20.1.0, and Messages 34.2.1
with the same argument schema accepted by Messages 35.0.0. This is not complete
custom-parameter or raw-regex argument interoperability.

## Actual retained data

Builtin-only `expr_js` macros retain original expression text, known parameter
descriptors, and a separate capture-layout matcher generated from the parsed
expression. Handler regexes, captures, argument parsing and Rust `FromStr`
semantics remain unchanged. The metadata matcher includes real outer parameter
captures and the official string type's nested escape captures. It does not
infer group hierarchy from flat runtime offsets: `regex-syntax` parses the
actual generated capture pattern.

Before reporting, metadata checks the registered regex source, observed versus
expected runtime captures, the whole-match span, character boundaries and nested
parent spans. An incompatible explicit descriptor fails visibly. Raw captured
strings remain untransformed; quote escapes are not removed from report values.
Byte starts become UTF-16 positions through the actual preceding text. Unmatched
captures omit value/start, while matched empty captures retain both. This is
especially important for empty strings and astral Unicode prefixes.

Messages emits `CUCUMBER_EXPRESSION` only when that actual descriptor exists,
ordered `stepMatchArgumentsLists` only when all matching definitions have complete
metadata, and used known builtin `parameterType` envelopes once per run. Native
`expr`, unannotated raw regexes, external RegexBuilder flags and undescribed
custom types retain truthful `REGULAR_EXPRESSION` reporting and capture omission.
Mixed supported/unsupported ambiguity omits the entire optional argument-list
field rather than inventing a partial aligned list. Unknown source provenance is
not supplied for builtins. Known snippet/preference descriptor flags identify the
pinned vocabulary; this does not add a JavaScript snippet engine or raw-regex
parameter lookup to native execution.

## Shared registration API

`step::DefinitionOptions` is shared with Phase30a timeouts. Existing
`given`/`when`/`then` methods insert default options and clear stale metadata on
replacement. Additive `*_with_options(location, regex, handler, options)` methods
preserve explicit options. `find_with_options` returns the existing `WithContext`
tuple plus a borrowed options reference in one resolution pass; `find` and the
public `Step` alias keep their existing signatures.

`StepConstructor::options` has a default implementation, so existing manual
implementations need no new required method. Generated inventory records and
fields are explicitly `#[doc(hidden)]` implementation glue; those records gain
an actual options factory. Direct manual struct literals for those undocumented
generated records therefore need the additional field. Original metadata is
never guessed by matching unrelated inventory entries.

## Authored acceptance

The 19 feature-first scenarios compare full raw capture trees with the pinned
JavaScript expression library for integer, float, word, anonymous, both string
quotes, empty/escaped strings, Unicode offsets, optionals/alternatives and a
numeric alias. They also cover ambiguous-list alignment, replacement clearing,
legacy/raw/custom omission, visible mismatches, and a manual StepConstructor
implementing only the original required method. Every reported envelope is
validated against both supported official schemas.

The Phase27 macro-source fixture now uses legacy `expr`, preserving that phase's
compiled-regex reporting contract. Phase32a covers retained `expr_js` provenance.

Focused boundary, run after both coordinated implementations were complete:

```
cargo test --features output-messages --test messages_expression_metadata
cargo test --features output-messages --test messages_support_code --test expression_compat --test compatibility
```

No new dependency fork is needed. `regex-syntax` is now an explicit direct
runtime dependency; its parser was already used transitively by the expression
crate. The official JavaScript oracle is development-only, pinned by the existing
`tests/interop/expressions/package-lock.json`, and runs only inside the completed
BDD target. Custom complete descriptors are a later bounded Phase32b contract.

## Verified baseline deviation

The first boundary passed 18 of 19 scenarios. The remaining escaped-string case
exposed an incorrect nested start offset in the pinned JavaScript dependency:
its last repeated capture has value `\" word` but start 8, while the actual text
places that value at 16. The feature contract was amended first to require
independent native JavaScript `/d` indices. The rerun confirmed Rust and native
JavaScript agree on the truthful start 16, and isolated only the pinned library's
incorrect nested start. Every other field still compares completely. Production
matching/reporting code and installed JavaScript packages were unchanged.

This is a verified deviation from the pinned expression library, not full
Cucumber.js parity or an upstream fix. Details and reproducible source evidence:
[polyfill span evidence](evidence/phase32a-polyfill-span.md).

Evidence:

- [First boundary with the precise baseline discrepancy](evidence/phase32a-expression-metadata-first-boundary.log)
- [Final primary BDD, 19 scenarios / 85 steps](evidence/phase32a-expression-metadata-bdd.log)

No unit suites, generic Cargo tests, early checks/builds/lints, code publication
or additional dependency forks ran for this phase. A shared registration-drop
warning discovered during the preceding coordinated timeout compilation was
resolved before the Phase32a boundary. Custom complete descriptors and arbitrary
raw-regex capture layouts remain deferred to their own bounded contracts.

## Coordinated affected BDD result

The shared Phase30a/32a batch passed all 11 focused targets, **174 scenarios /
751 steps**, with no warnings or failures. Phase32a's requested regressions were
`compatibility` **20/115**, `expression_compat` **53/159**, and
`messages_support_code` **14/56**. The other targets exercise the timeout phase's
shared lifecycle and reporting changes; they were run once in the combined batch.

- [Affected BDD output](evidence/phase30a-32a-affected-bdd.log)
- [Exact combined command](evidence/phase30a-32a-affected-command.txt)

No remaining blocker exists for this bounded phase. Shared production and Cargo
were released to the coordinator after the passing batch.
