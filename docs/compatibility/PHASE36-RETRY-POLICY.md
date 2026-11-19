# Phase36: validated explicit retry policy

Status: implemented and phase-end verified on 2026-11-17: **16 scenarios /
56 steps passed**, exit 0. The helper is exported and the `retry_policy` BDD
target is registered. Corrected primary evidence:
`evidence/phase36-retry-policy-bdd.log`.

Initial primary acceptance passed 8 of 16 scenarios. The other eight reached an
incorrect harness assertion treating official Query's final-case view as all
attempts; Query deliberately excludes attempts marked `willBeRetried`. The
correction preserved all raw start/finish/World/retry-flag checks, asserted
observedAttempts separately, and checked one terminal Query case at the final
attempt index. No RetryPolicy or Basic production changes were needed. Initial
evidence is retained as `evidence/phase36-37-primary-bdd.log`. No unit suites or
premature implementation checks were run. The parent coordinates affected
regressions separately; they are not included in the 16/56 primary count.

The pinned [JS13.3 validator](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/configuration/validate_configuration.ts)
rejects a configured nonempty tag filter without a positive retry budget.
The [runtime helper](https://github.com/cucumber/cucumber-js/blob/v13.3.0/src/runtime/helpers.ts)
uses the budget only when inherited tags satisfy the gate. It never interprets
native `@retry` tag settings. Phase30b already provides FAILED-only eligibility;
this phase supplies reusable budget/gate selection through the existing callback.

## Small composable API

`runner::RetryPolicy::new(count: usize, tags: Option<tag::Expression>)` returns
`Result<RetryPolicy, RetryPolicyError>` before parsing or execution starts. Zero
without a gate disables retries. Zero with Some gate is invalid. An explicitly
supplied typed Expression is a real gate, including an empty/always-true parsed
expression; applications translating JS configuration should normalize an empty
filter string to None. No invalid configuration is disguised as a parse event.

Private fields retain the validated count and optional official expression.
`options(feature, rule, scenario)` produces existing `basic::RetryOptions` with
`Retries::initial(count)` and no delay only when the positive budget and inherited
tag gate permit retry. `into_options()` returns a closure with the existing
four-argument `retry_options` signature, ignoring its merged legacy CLI argument.
No Basic fields, second runner, implicit outcome-policy changes or new CLI are
required. There is no added delay setter in this bounded phase.

Applications compose:

```rust,ignore
let retry = runner::RetryPolicy::new(2, Some("@retryable".parse()?))?;
cucumber.retry_options(retry.into_options())
    .outcome_policy(execution::OutcomePolicy::cucumber_js());
```

The typed callback owns budget and tag selection: `--retry`, `--retry-after`,
legacy retry filters and native magic `@retry` settings do not override it.
A later explicit `retry_options` callback replaces it, following the existing
last-callback-wins builder behavior. Applications wanting CLI-sourced compatible
settings parse/validate their typed input first, then inject this policy. Failure
eligibility remains independently selected via OutcomePolicy; selecting a budget
alone does not silently alter legacy behavior.

## Acceptance and boundary

The16 authored cases cover early invalid configuration, disabled retry, Feature/
Rule/Scenario/Examples tag inheritance, magic-tag isolation, conflicting legacy
CLI inputs, later callback replacement, four non-failed classifications, and
recovered/exhausted actual Messages attempts. Protocol cases require schema and
official Query agreement, fresh Worlds and cleanup per attempt. Implement the
whole helper and harness before only its focused phase-end BDD acceptance.
