# Phase32b design: explicit custom and raw capture metadata

Status: complete and verified at the coordinated phase-end BDD boundary:
**16 scenarios / 78 steps passed**. Production activation followed the preceding
passed shared phase batch. No unit suites, generic tests or early checks ran.

## Existing capability and precise gaps

Phase32a already exposes `ExpressionMetadata::from_generated` (documented as
codegen glue) plus `DefinitionOptions::expression`. A caller can supply complete
manual custom descriptors, runtime/capture patterns and real nested group data
through that path today. This should remain compatible rather than be replaced.

Automatic custom macro metadata is missing: the `Parameter` trait retains only
`NAME` and `REGEX`, and the expression metadata generator deliberately omits a
registration when it sees an unknown custom name. No snippet/preference flags
are available to infer. Raw regex metadata is also missing: the current
`ExpressionMetadata` necessarily reports `CUCUMBER_EXPRESSION`, and its arguments
always have known parameter names. It must not be reused to mislabel raw regexes.

The current Rust regex macro conversion visits flat capture indices. JavaScript
RegularExpression constructs arguments from top-level capture groups. A raw
`Step(Context)` may consume either layout or a chosen subset. Source ranges and
AST nesting are available; intended callback argument mapping is not inferable.

## Implemented additive APIs

1. Add `Parameter::metadata() -> Option<ParameterDescriptor>` with a default of
   `None`, preserving existing manual implementations. A derive declaration may
   opt in only with both explicit snippet/preference flags. Generated descriptors
   retain actual `NAME` and `REGEX`; any supplied descriptor must agree with those
   actual constants. Codegen builds the complete capture matcher from the parsed
   original expression and actual custom types. Missing descriptors preserve
   omission. This does not implement JavaScript transformers or change existing
   Rust `FromStr` conversion.
2. Add `RegexMetadata::new(pattern, argument_capture_indices)` with an accessor
   for the regex compiled by that factory. It compiles the regex and parses its
   AST under known default settings; inline settings are part of the supplied
   pattern. Explicit indices select real captures in argument order, allowing
   outer-only or native flat mapping without guessing. No parameter names or
   descriptor flags are invented. `DefinitionOptions::regular_expression` is
   additive and mutually exclusive with expression metadata.
3. Reuse actual runtime/source/capture/range validation and UTF-16 conversion.
   Invalid capture indices fail construction. Explicit metadata that disagrees
   with the observed registered matcher fails visibly. Existing unannotated raw
   registrations, including those using hidden RegexBuilder settings, continue
   omitting unavailable captures. This does not reconstruct external flags or
   promise that Rust regex source is replayable as JavaScript syntax.
4. Preserve existing insertion/find signatures and stale-option clearing. Mixed
   complete expression/raw definitions can provide aligned argument lists; any
   unavailable matching definition continues to cause optional-list omission.

Parameter source provenance remains optional; it will not be guessed. Conflicting
complete descriptors for one name must fail visibly rather than overwrite facts.

## Authored bounded acceptance

`tests/features/messages_custom_metadata/custom.feature`: eight scenarios cover
four custom group/descriptor fixtures, unchanged legacy custom registration,
existing manual metadata capability, conflicting names and descriptor/type
mismatch. Nested custom cases preserve the existing first-selected-capture Rust
conversion while reporting actual full raw group structure.

`tests/features/messages_custom_metadata/raw.feature`: eight scenarios
cover independent/outer-only/flat-nested argument mappings, absent versus empty
captures, Unicode offsets, invalid indices, contradictory external flags and
replacement clearing. Native JavaScript indices provide independent source-range
evidence; no argument mapping is inferred from an opaque Rust callback.

The existing `Parameter::REGEX` documentation inaccurately claims nested groups
are prohibited and validated. Source inspection shows derive only checks regex
validity, while the native expander explicitly handles nested captures. That documentation is corrected in this phase; existing handler conversion is
preserved.

## Unchanged baseline deviation and boundaries

The observed cucumber-expressions 20.1.0 / regexp-match-indices 1.0.2 repeated-group
start defect remains documented in
[Phase32a source/range evidence](evidence/phase32a-polyfill-span.md). Truthful native
ranges remain authoritative for that exact demonstrated case. This design neither
fixes upstream packages nor claims full Cucumber.js parity.

No additional dependency fork was needed. No deployment, publication, security
settings changes, generic tests or early verification ran.

## Usage and acceptance boundary

A derived custom type opts in with both actual declaration flags:

```rust
#[derive(cucumber::Parameter)]
#[param(name = "color", regex = "red|blue",
    use_for_snippets = false, prefer_for_regexp_match = true)]
struct Color(String);
```

Existing `Parameter` implementations inherit metadata omission. Neither flag is
inferred. A partial flag declaration is rejected by the derive parser. Complete
manual descriptors using the earlier generated-data constructor remain accepted;
new custom macro descriptors are checked against actual type constants.

Raw capture mapping is explicit native metadata, not general JavaScript raw
handler parity:

```rust
let metadata = cucumber::support_code::RegexMetadata::new(
    r"^code (([A-Z]+)-([0-9]+))$", [1],
)?;
let regex = metadata.regex().clone();
let options = cucumber::step::DefinitionOptions::new().regular_expression(metadata);
// Pass regex, the existing native Step callback and options to given_with_options.
```

`[1]` selects the outer argument and retains real children; `[1, 2, 3]` explicitly
selects flat native arguments. Zero may select the real whole-match capture;
repeated indices intentionally repeat that actual capture in the declared order. This
does not change the callback or infer which arguments it consumes. External
RegexBuilder settings are never reconstructed; conflicting observed matches fail
visibly, while unannotated registrations retain omission.

The dev-only oracle registers emitted custom descriptors with the pinned official
expression library, compares original expression/group data, and independently
uses native JavaScript `/d` indices for the declared raw mappings. No installed
oracle package was modified. The 32a repeated-capture deviation remains unchanged.

Focused phase-end boundary (parent coordinated, with the complete shared feature set):

```
cargo test --features output-messages --test messages_custom_metadata
```

Files: custom metadata trait/derive/codegen helper; explicit raw matcher metadata;
shared registration options and snapshots; the approved Messages `flush_case`
block; `tests/messages_custom_metadata.rs`, two feature contracts, the pinned
`custom-metadata.mjs` consumer, and Cargo target/includes. No runner/parser change
or new production dependency was needed. The unrelated completed dry-run formatter
BDD target was registered in the same coordinated manifest edit at its owner's
request; it is not part of Phase32b's acceptance count.

### First runtime boundary finding

The shared third boundary compiled and executed Phase32b: 15 of 16 scenarios
passed. The absent custom group fixture's official raw argument-tree comparison
passed; its separate legacy handler expectation was incorrect. For `(a)?b`,
cucumber-expressions 0.5.0 `expand/parameters.rs::has_capture_groups` treats a
repetition node as false and wraps the complete parameter in an outer capture.
The existing `codegen/src/attribute.rs::arg_ident_and_parse_code` selects the
first nonempty capture, so the handler receives `b`, not an empty string.
Only that harness expectation changed; metadata/oracle and handler production
code remain unchanged. The first diagnostic is retained in
`evidence/phase32b-34-35-third-bdd.log`. The focused rerun passed all 16 scenarios / 78 steps, with evidence in
`evidence/phase32b-custom-metadata-bdd.log`. An earlier mistyped parent tag selected
zero scenarios (`phase32b-empty-tag-attempt.log`) and is explicitly excluded from
acceptance evidence.

The completed shared affected BDD batch also passed **267 scenarios / 1115 steps**
(parent-coordinated Phase32b/34/35 affected boundary). This supplements the focused
16/78 acceptance; it does not expand the Phase32b contract or claim full parity.
