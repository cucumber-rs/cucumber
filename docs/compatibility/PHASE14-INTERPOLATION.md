# Phase 14: opt-in official example interpolation

`feature::ExampleSemantics::{Legacy,Gherkin43}` selects outline expansion.
Existing `FeatureExt::expand_examples()` and parser defaults remain Legacy.
Use `expand_examples_with`, `Basic::with_example_semantics` or
`Cucumber::with_example_semantics` for explicit official semantics.

Official mode replaces literal tokens sequentially in Examples-header order,
retains unknown placeholders, and handles replacement dollar signs literally.
The policy is recorded in SourceSnapshot and reused by the Messages source graph
when matching native runtime candidates. Raw source/templates remain unchanged.

Feature-first acceptance covers unknown tokens, forward/reverse cascades, dollar
values, spaced/duplicate headers and regex-looking literal headers in names,
steps, both step arguments and Rule outlines. The pinned official JavaScript
Gherkin43 compiler provides expected Pickle payloads at the BDD phase boundary.
Legacy error/single-pass cases preserve existing consumers.

Only after complete implementation:
`cargo test --features output-messages --test interpolation -- --tags @phase14`.
Result: **9 scenarios / 41 steps passed**, first boundary run, exit 0.
Evidence: [phase14-interpolation-bdd.log](evidence/phase14-interpolation-bdd.log). Full grammar/semantic conformance is not claimed by this phase.
