# Phase 21: explicit expression and tag compatibility

This phase pins behavior to the Cucumber.js13.3.0 dependency versions:
`@cucumber/cucumber-expressions`20.1.0 and `@cucumber/tag-expressions`11.0.1.
The independent oracle is isolated in `tests/interop/expressions`, with exact
package versions and transitive integrity lock. It executes only as part of the
completed BDD target, not during implementation. Node is development-only.

## Expression API

`#[given(expr_js = "...")]` (also when/then) is additive. Existing `expr` uses
unchanged native patterns and raw quote-escape content. Explicit mode overrides
built-in parameter matchers with the pinned JS vocabulary: ASCII integer digits,
restricted float exponent/decimal syntax, ECMAScript word whitespace, quoted
string patterns and anonymous input. It adds double, bigdecimal, byte, short,
long and biginteger aliases and unescapes quote escapes in string captures.
Existing optional/alternative/escaped-expression parsing and custom parameters
are reused. No new expression dependency fork is needed for this boundary.

Handler types still use idiomatic Rust `FromStr`: ranges and precision follow
the chosen type. Use a precision-preserving string or suitable numeric type for
large integers/decimals; this is not automatic JS Number/BigInt emulation.
JavaScript support files, async custom transformers, snippet preference and
regular-expression registry precedence remain outside this phase's claim.

The generated wrapper also binds its handler function hygienically before parsed
argument bindings. This fixes the observed function-name/argument-name collision
for both legacy and explicit expression handlers without changing handler APIs.

## Tag API

`tag::Expression` implements explicit JS11 parsing and full-literal evaluation.
Empty expressions are true; bare literals, escaped whitespace and ECMAScript
whitespace are supported. `Expression::matches(feature, rule, scenario)` adapts
native marker-free tags and inherits feature/Rule/expanded-Examples tags. Pass it
through `filter_run`, or use `Hooks::before_matching` / `after_matching`.
Existing TagOperation and CLI interpretation remain unchanged. The pinned JS
trailing-escape behavior is retained deliberately rather than silently tightened.

## Acceptance and limits

Feature-first scenarios run actual macro handlers, compare match/value results
with the official expression library, compare tag acceptance/selection, and run
real filtered outline rows with matching hooks. Legacy float/string/tag defaults
and hygienic function/argument collisions have explicit cases. This is bounded
adopted behavior, not complete expression/parser/CLI parity.

Only at the completed phase boundary:

```sh
npm ci --prefix tests/interop/expressions --ignore-scripts --no-audit --no-fund
cargo test --features output-messages --test expression_compat -- --tags '@phase21_expression or @phase21_tags'
```

First phase-end result: **53 scenarios / 159 steps passed**, exit 0, with no
compiler warnings or corrections. The official oracle ran only in this BDD
boundary. The focused existing compatibility target then passed **20 scenarios /
115 steps**, covering legacy macro invocation, captures, matching, and combined
arguments after the generated-handler binding change. Legacy hooks9/42 also
passed in the coordinated Phase20 boundary with these shared hook changes.
No unit suite, generic Cargo test, early build/check/lint or publication ran.

Evidence:
- [phase21-expression-tags-bdd.log](evidence/phase21-expression-tags-bdd.log)
- [phase21-legacy-compatibility-bdd.log](evidence/phase21-legacy-compatibility-bdd.log)

Files: codegen/src/attribute.rs, codegen/src/lib.rs, src/tag.rs,
src/runner/hooks.rs; tests/expression_compat.rs and its two feature files;
tests/interop/expressions package, lock and oracle; root Cargo target/include.
No additional dependency fork was needed or created.

Pinned primary sources:
- https://github.com/cucumber/cucumber-js/blob/v13.3.0/package.json
- https://github.com/cucumber/cucumber-expressions/blob/v20.1.0/javascript/src/defineDefaultParameterTypes.ts
- https://github.com/cucumber/tag-expressions/blob/v11.0.1/javascript/src/index.ts
