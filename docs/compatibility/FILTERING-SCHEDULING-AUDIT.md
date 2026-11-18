# Source audit: filtering, configuration and scheduling

Baseline: Cucumber.js13.3.0 tag tree
`4f2123366dfd5b9b030fb0010ada23752d710b14`; local cucumber0.23.0 fork with
completed native compatibility phases. This audit did not run tests or oracles.
Source-supported differences below are not new runtime acceptance claims.

## Selection

[JS PickleFilter](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/pickle_filter.ts)
combines line AND name AND tag filters. Repeated names are ORed; repeated tag
expressions are ANDed by the
[argument parser](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/configuration/argv_parser.ts).
Name matching uses expanded Pickle names. Tag matching uses inherited Pickle tags.

Native src/cli.rs has one Option<Regex>, one Option<TagOperation>, and an explicit
name/tags conflict. src/cucumber.rs filter_run uses CLI-name precedence, otherwise
CLI-tags, otherwise the supplied predicate. It does not compose all three.
This is preserved legacy behavior, not classified as a bug:
[book/src/cli.md](../../book/src/cli.md) documents that CLI options override code
configuration. The filter_run rustdoc itself does not describe that precedence.
Existing tag::Expression supplies the official expression language through an
idiomatic API and includes Feature/Rule/expanded Examples inheritance. Native
regex is Rust regex syntax; arbitrary JavaScript regexp parity is not claimed.

Phase33 implements explicit typed FilterOptions and conjunctive selection while
leaving cli::Opts public fields and legacy precedence unchanged. Names repeat as
OR; tags repeat as AND; caller predicate is ANDed. No JS config/support-code
module loader is necessary. A separate optional CLI can follow if required. See [the API and acceptance contract](composable-filters.md) for current verification status.

## Exact file/line semantics

[JS location map](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/formatter/helpers/gherkin_document_parser.ts)
contains Scenario declarations and Examples body rows. PickleLineFilter compares
selected lines against Pickle astNodeIds, not arbitrary contained step lines:

- Scenario declaration selects that scenario.
- Outline declaration selects all generated rows.
- An Examples body-row line selects that row.
- Feature, Rule, Background, step, Examples declaration and table-header lines do
  not select scenarios. Multiple selected lines are ORed within a source.
- Name and tag filters still apply conjunctively.

The original native parser accepts paths/globs, not file:line selectors. Its outline
expanded.position is synthesized as example.position.line + row_index +2; comments
and Markdown separator rows make it unsuitable as the true row location. A bounded
selector phase needs parser-owned scenario/row source identities independent of
Messages. Phase34 now supplies that through explicit typed `LineSelectors` and
immutable `SourceSnapshot::scenario_origins`, preserving complete raw snapshots.
See [the verified typed selection contract](line-selection-plan.md). String
file:line CLI parsing and implicit discovery remain outside that bounded phase.

## Dry-run

[JS docs](https://github.com/cucumber/cucumber-js/blob/v13.3.0/docs/dry_run.md),
[executor](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/runtime/executor.ts),
[test-case runner](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/runtime/test_case_runner.ts)
and [result policy](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/runtime/helpers.ts)
establish no hook/body execution, skipped recognized steps, reported undefined
and ambiguous steps, and no failure from those dry-run outcomes. JS still
constructs its World in TestCaseRunner. It does not retry skipped dry-run attempts.

No native built-in dry-run was found. Native Step::Skipped currently means missing
definition in Messages (UNDEFINED); it cannot also represent recognized dry-run
steps without explicit outcome/mode information. Design this after outcome
plumbing, with an explicit World-construction contract. Do not simulate it by
zero concurrency or by executing no-op replacement definitions.

## Retries

Native retries, retry_filter, retry_after, retry_options and corresponding CLI
options already exist. Fresh Worlds/hooks and Messages attempts have prior BDD
evidence. The exact policy still differs:

- A matching native retry filter alone enables the implicit default1 retry;
  [JS validation](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/configuration/validate_configuration.ts)
  requires a positive configured retry count.
- Native @retry tag settings can override a nonmatching filter. JS
  [retriesForPickle](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/runtime/helpers.ts)
  gates the configured budget with the tag expression, without native magic tags.
- Native ambiguity becomes StepPanicked and is retryable. JS TestCaseRunner retries
  only terminal FAILED, not AMBIGUOUS. Native undefined steps are not retried.

A custom retry_options closure can implement budget/tag selection now, but cannot
change outcome eligibility. A bounded explicit compatibility policy should cover
positive-count validation, tag gating, ambiguity and exhausted/recovered outcomes,
while retaining existing retry tags/delay behavior in legacy mode.

## Ordering and concurrency

[JS order implementation](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/filter/order_pickles.ts)
supports defined, reverse and seeded random. Discovery order honors supplied path/
glob sequence; seeded random uses knuth-shuffle-seeded. Native Basic parser sorts
its discovered paths, and Basic runner groups Serial/Concurrent with serial/retry
priority. No built-in reverse/random execution plan or seed option was found.
Custom parsers/runners can supply ordering, but are not built-in option parity.
Test actual invocation order rather than normalized formatter output.

JS defaults to parallel0 (serial), while native defaults to64 concurrent scenarios.
Native max_concurrent_scenarios, --concurrency, @serial and which_scenario already
provide useful bounded async scheduling. Prior scheduling BDD covers positive
limits, overlap, World isolation and serial exclusivity. It does not establish
zero-limit behavior, CLI override boundaries, fairness or cancellation.

[JS dynamic assignment](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/runtime/parallel/test_cases_phase.ts)
receives candidate plus running Pickles; it can select a later queue entry and has
an idle-worker fallback. Native which_scenario is static Serial/Concurrent
classification. [JS worker threads](https://github.com/cucumber/cucumber-js/blob/v13.3.0/docs/parallel.md)
and per-worker hooks/IDs are not equivalent to native async tasks. Keep native
execution idiomatic; do not fabricate worker identities or claim isolation.

Bounded sequencing: composable filters; source-aware selectors; dry-run after
outcomes; explicit retry compatibility; deterministic order; only then dynamic
assignment if required. Existing typed builders/custom Clap with_cli support
application configuration. JS support-file loading and deployment are not scope.

## Confirmed upstream Markdown/schema contradiction

The unchanged official43
[misc.feature.md AST](https://github.com/cucumber/gherkin/blob/3d70e2bb4e0f16a568ee539a5ce0b03fb247a777/testdata/good/misc.feature.md.ast.ndjson)
omits Feature.keyword for the fallback first-line title. The Markdown matcher
sets a FeatureLine without matchedKeyword when no explicit Feature heading is
present. Both pinned
[Messages34.2.1 schema](https://github.com/cucumber/messages/blob/v34.2.1/jsonschema/messages.schema.json)
and [Messages35.0.0 schema](https://github.com/cucumber/messages/blob/v35.0.0/jsonschema/messages.schema.json)
list keyword in Feature.required and define it as a string. The local unmodified
fixtures/schemas confirm the same field-presence conflict.

Exact upstream AST fidelity and schema validity cannot both be claimed for that
shape. The implementation retains the absent field; it does not fabricate an
empty keyword in emitted AST or fork the protocol specification. The native
execution projection's empty keyword is separate and documented. Explicit
Markdown Feature headings avoid this specific conflict. Finite corpus agreement
must not be described as universal schema conformance.
