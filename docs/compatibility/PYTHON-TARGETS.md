# Python compatibility targets: Behave and pytest-bdd

Research date: 2026-11-18.

**Status: source research only; runtime compatibility is unverified.** This
matrix proposes two parallel optional adapter research tracks. It is not an
implemented adapter, an adopted acceptance contract, a package publication claim,
or evidence of full parity. No installs, imports, builds or tests were performed
for this research. Investigating both frameworks follows the owner's direction;
there is no exclusive framework-selection blocker to the shared Rust SDK.

The framework-neutral compiler/rerun SDK and safe Rust extension interface can
precede native Python callbacks. Framework adapters follow that foundation and
must preserve their distinct execution semantics. Existing core BDD receipts do
not prove Python wrapper or framework compatibility.

## Pinned source baselines

| Target | Verified release tag | Declared constraints |
| --- | --- | --- |
| Behave | [v1.3.3, commit 7673da8](https://github.com/behave/behave/releases/tag/v1.3.3) | Its [manifest](https://github.com/behave/behave/blob/v1.3.3/pyproject.toml) includes Python 2.7. This does not imply that the proposed native SDK supports legacy interpreters. |
| pytest-bdd | [9.0.1, commit aad3bf4](https://github.com/pytest-dev/pytest-bdd/releases/tag/9.0.1) | Its [manifest](https://github.com/pytest-dev/pytest-bdd/blob/9.0.1/pyproject.toml) requires Python >=3.10, pytest >=7 and gherkin-official >=29.0.0 excluding 31.0.0 and 32.0.0. Future acceptance must also pin the resolved pytest and parser versions. |

These are explicit compatibility baselines, not claims about the latest releases.
Context7 was used for documentation discovery; tagged primary sources establish
the version-specific findings below. The linked pytest 9.0.0 outcome definitions
are supplementary source evidence, not adoption of a complete resolved stack.

## Dual-adapter capability matrix

| Boundary | Behave 1.3.3 | pytest-bdd 9.0.1 |
| --- | --- | --- |
| Shared Rust operations | Source-preserving compilation, AST/Pickle identities, diagnostics, Messages consumption and rerun selection can use the framework-neutral SDK. | Same shared operations; pytest collection IDs need an adapter mapping to canonical source/Pickle IDs. |
| Loading | Environment hooks load before step definitions. Step files execute with decorator globals, path handling and matcher reset between files. Ordinary module import is not equivalent. [Runner](https://github.com/behave/behave/blob/v1.3.3/behave/runner.py), [loader](https://github.com/behave/behave/blob/v1.3.3/behave/runner_util.py). | Preserve pytest collection, conftest.py, plugins and scenario decorators. Steps become fixture providers; direct function calls bypass the execution contract. [Steps](https://github.com/pytest-dev/pytest-bdd/blob/9.0.1/src/pytest_bdd/steps.py), [plugin](https://github.com/pytest-dev/pytest-bdd/blob/9.0.1/src/pytest_bdd/plugin.py). |
| Matching | Keyword-specific candidates precede generic @step candidates; lookup returns the first successful match. Registration performs overlap checks. [Registry](https://github.com/behave/behave/blob/v1.3.3/behave/step_registry.py). | Keyword filtering plus pytest fixture visibility and override resolution. This differs from Cucumber.js common-registry ambiguity behavior. [Execution](https://github.com/pytest-dev/pytest-bdd/blob/9.0.1/src/pytest_bdd/scenario.py). |
| Parameters | parse, cfparse, regex and custom conversions have Python semantics. Python objects should remain in Python, with opaque callback handles across Rust. [Matchers](https://github.com/behave/behave/blob/v1.3.3/behave/matchers.py). | Exact strings, parse, cfparse, regex/custom parsers and converters. Regex uses Python full-match and named groups. [Parsers](https://github.com/pytest-dev/pytest-bdd/blob/9.0.1/src/pytest_bdd/parsers.py). |
| State and lifecycle | Layered run/feature/rule/scenario Context, environment hooks and scoped cleanup. Cleanup reverses registration order. Step text/table are exposed through Context. [Runner/Context](https://github.com/behave/behave/blob/v1.3.3/behave/runner.py), [model](https://github.com/behave/behave/blob/v1.3.3/behave/model.py). | Fixture dependencies/scopes, yield teardown, target_fixture and pluggy dispatch belong to pytest. Before-step and before-step-call differ; after-step means successful execution. [Execution](https://github.com/pytest-dev/pytest-bdd/blob/9.0.1/src/pytest_bdd/scenario.py), [hooks](https://github.com/pytest-dev/pytest-bdd/blob/9.0.1/src/pytest_bdd/hooks.py). |
| Async | Coroutine steps are wrapped and executed through run_until_complete. Timeout behavior depends on interpreter capabilities. Already-running host loops need an explicit integration boundary. [Async implementation](https://github.com/behave/behave/blob/v1.3.3/behave/async_step.py). | Tagged execution invokes pytest fixture machinery without a direct await path. Do not assume pytest-asyncio supplies async-step support automatically; separately pin and prove any plugin combination. [Execution](https://github.com/pytest-dev/pytest-bdd/blob/9.0.1/src/pytest_bdd/scenario.py). |
| Skip/failure | Preserve assertion failure, ordinary exception/error, pending, hook error, cleanup error and scenario skip distinctions, including remaining-step behavior. [Model](https://github.com/behave/behave/blob/v1.3.3/behave/model.py). | Preserve pytest skip/xfail/fail and setup/call/teardown outcomes. Outcome exceptions derive from BaseException and cannot all be treated as ordinary Exception instances. [pytest outcomes](https://github.com/pytest-dev/pytest/blob/9.0.0/src/_pytest/outcomes.py). |
| Grammar boundary | Framework compatibility and official Gherkin conformance need separate evidence. Behave uses its own parser. [Parser usage](https://github.com/behave/behave/blob/v1.3.3/behave/runner_util.py). | Uses gherkin-official, then transforms the AST into framework models. Its adapter additionally dedents docstrings; this must not alter canonical Rust AST output. [Gherkin adapter](https://github.com/pytest-dev/pytest-bdd/blob/9.0.1/src/pytest_bdd/gherkin_parser.py). |

## Proposed finite acceptance boundaries

These are research recommendations, not adopted KBD acceptance scenarios. When a
phase is adopted, author its explicit business/compatibility scenarios before
implementation and execute only its focused BDD acceptance at phase end. Do not
substitute unit suites, generic tests or premature verification.

1. **Shared installed SDK:** Given a wheel installed outside the checkout, when
   callers compile outline fixtures containing tags, Unicode, comments and step
   arguments, then canonical Rust results and source locations are preserved.
   Include invalid-source diagnostics and rerun attempt history. Behave and
   pytest must not be required for these framework-neutral operations.
2. **Foreign execution foundation:** Given independent Python sessions, when
   captured handlers execute synchronously or asynchronously, then identity and
   state remain isolated. Cover failure, timeout, late completion and exactly-once
   cleanup. This proves Python callbacks, not either framework's parity.
3. **Behave loading and matching:** Given environment.py and two step files,
   when definitions load, then matcher setup/reset, keyword separation, generic
   fallback, custom conversion and registration conflict match the pinned oracle.
4. **Behave lifecycle:** Given two scenarios beneath a feature/rule, when passing,
   failing, skipped and pending cases run, then context scopes, hook ordering and
   cleanup traces match; scenario state does not leak.
5. **Behave async:** Given a certified interpreter and synchronous host invocation,
   when an async step completes or times out, then its outcome matches the defined
   contract. An active-loop invocation requires separate proof or an explicit
   unsupported-boundary diagnostic.
6. **pytest loading and fixtures:** Given parent/child conftest.py files, when a
   scenario executes, then scoped step overrides, fixture dependencies,
   target_fixture values and yield cleanup resolve through pytest.
7. **pytest data and collection:** Given an outline with tagged example groups,
   when collected and executed, then parameters/docstrings/tables reach steps and
   collected cases map to canonical source identities. Reserved argument
   collisions must fail clearly.
8. **pytest outcomes and hooks:** Given passing, failing, missing, skipped and
   xfailed cases, when run through the pinned pytest stack, then hook traces and
   setup/call/teardown reports match; teardown failures remain visible. Unproved
   coroutine behavior must never be represented as successful execution.
9. **Optional adapter packaging:** Given an installed artifact outside the
   checkout, when an optional framework is absent, then the base SDK still works
   and an adapter request reports an actionable dependency error.

## Feasibility constraints and implementation order

The following are architectural inferences from the source findings, not runtime
results or adopted designs:

- Reuse host loaders, matchers, fixture/context machinery where practical. Do not
  duplicate Python regex, arbitrary converter objects or fixture resolution in
  Rust merely to make the interfaces look alike.
- A pytest adapter requires an active pytest session/request. Start by retaining
  pytest collection/fixture ownership while using Rust for shared compilation and
  evidence operations. Deeper Rust scheduling requires a separately proved bridge.
- Behave's global decorator registry and matcher configuration require isolation
  or serialization before independent concurrent-session claims.
- Define one scheduling owner per adapter. Do not silently nest Rust parallelism
  inside pytest/xdist or concurrently share a Behave Context.
- Keep canonical AST/Pickle output separate from framework transformations.
  Framework-specific matching and status policies must not silently replace the
  existing Rust or Cucumber.js compatibility modes.
- Complete the shared compiler/rerun SDK and safe Rust extension interface first,
  then native foreign callback ownership/lifetime/error handling, then bounded
  optional framework adapters. Callback success alone does not prove fixture,
  environment, hook or loader compatibility.
- Rust cancellation cannot forcibly terminate arbitrary Python side effects.
  Late-result rejection and cleanup need explicit callback/session contracts.
- Keep optional framework dependencies outside the base SDK's required dependency
  set. Native interpreter/ABI/platform support must reflect actual package proof,
  not the broader versions advertised by a framework.
- No additional dependency fork is established as necessary by this research.
  Public names, registry ownership, publication scope and credentials remain
  publication decisions; they are not exclusive framework-selection decisions.

No adapter implementation, Python behavioral verification, package publication,
or certification is established by this document.

## Detailed adapter preparation

The [adapter requirements](PYTHON-ADAPTER-REQUIREMENTS.md) expand the distinct
Context/fixture, hook exception, nested-step, async, outline identity and report
boundaries from tagged source. They remain unadopted future-phase candidates.
The [upstream overlap record](UPSTREAM-OVERLAP.md) separately tracks active
Cucumber hooks/sharding proposals without premature submissions.
