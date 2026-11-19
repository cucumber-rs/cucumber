# Explicit composable filters

`FilterOptions` opts an executor into name OR / tag AND / application-predicate
AND selection. The default remains the documented legacy behavior: a CLI name
selector takes precedence, otherwise a CLI tag selector, otherwise the caller's
`filter_run` predicate. Installing even empty options enables composition.

```rust,ignore
use cucumber::{FilterOptions, World as _};
use regex::Regex;

let filters = FilterOptions::new()
    .name(Regex::new("^invoice ")?)
    .name(Regex::new("^health$")?)
    .tag("@smoke".parse()?)
    .tag("not @quarantined".parse()?);
MyWorld::cucumber()
    .with_filter_options(filters)
    .filter_run("features", |_, _, scenario| application_accepts(scenario))
    .await;
```

Names match expanded scenario names using Rust regex syntax. Tag expressions use
`tag::Expression`, pinned to `@cucumber/tag-expressions` 11.0.1. Matching includes
Feature, Rule, Scenario and selected Examples tags. Each expression must match;
within a single expression the standard `and`, `or` and `not` operators apply.
Empty name/tag groups impose no restriction; an empty tag expression is true.

A configured CLI name joins the same OR group as typed names. A configured CLI
tag expression is another required condition. The existing `cli::Opts` layout
and argv name/tag conflict are unchanged; repeated typed options are an API,
not a claim that the legacy CLI now accepts repeated flags or both selectors.
Programmatic `cli::Opts` may still supply both existing fields. Clone and
parser/runner/writer replacement preserve explicit options, even where those
replacement builders reset their component-specific CLI configuration.

Filtering occurs after parser expansion and does not mutate a captured raw
`SourceStore` snapshot. The source-backed Messages writer can use original
source/Pickle IDs for selected executions. No new guarantee about emitting
source envelopes for entirely unselected features is introduced.

Phase33 acceptance is `tests/composable_filters.rs`, driven by 12 scenarios in
`tests/features/composable_filters/selection.feature`. It checks real handler
selection, Messages test-case references, inherited tags, expanded names,
empty selectors, documented legacy overrides, explicit CLI composition, clone
and type-changing builders, and retained original Examples rows. The focused
phase-end command is:

```sh
cargo test --features output-messages --test composable_filters -- --tags @phase33
```

Verified phase-end result: **12 scenarios / 56 steps passed**. The coordinated
run enabled `output-messages,output-json,output-junit,libtest` to compile all
completed shared production changes. Evidence:
[`phase33-composable-filters-bdd.log`](evidence/phase33-composable-filters-bdd.log).
The first attempt stopped before acceptance on an unconstrained initial harness
input type after replacing its parser. An explicit `PathBuf` annotation corrected
that harness issue; the unchanged production contract then passed. Initial
diagnostics are retained in
[`phase33-composable-filters-initial.log`](evidence/phase33-composable-filters-initial.log).
Three shared-production warnings were reported to their owners: unused Basic
writer helper methods and two overly public private-module outcome helpers.
No unit suites or generic test/check commands were run.

File/line selection, dry-run, retry-policy parity and seeded order remain
separate capabilities. See [the source audit](FILTERING-SCHEDULING-AUDIT.md) for
pinned upstream sources and the Markdown AST/schema contradiction.
