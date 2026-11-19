# Python adapter requirements before a later phase

Research checked 2026-11-18 against Behave 1.3.3 and pytest-bdd 9.0.1. These are
source-derived requirements and proposed acceptance inputs, not an adopted
phase, implementation, final audit or runtime proof. No framework was installed
or executed. The existing [target matrix](PYTHON-TARGETS.md) gives the baseline
versions and wider scope. The shared SDK's eleven-case boundary is unchanged.

## Ownership and useful integration

The first SDK phase exposes canonical source compilation and rerun plans. A
later foreign-callback phase must establish native handles, lifetime ownership,
error propagation, cancellation and exactly-once completion before either
framework adapter claims execution compatibility. Both framework tracks remain
in scope; neither is selected as a substitute for the other.

For VLBV, a useful first adapter increment should preserve the framework's own
loader and execution owner, associate its real cases and reports with Rust source
identities, and produce traceable evidence. This is an architectural proposal,
not a runtime result. Running Python functions from Rust does not preserve either
framework's lifecycle by itself. Arbitrary Python values remain owned by Python;
the bridge carries explicit handles and source/execution identifiers rather than
silently serializing fixture/context objects into JSON.

## Behave requirements

| Requirement | Tagged source finding | Future behavioral evidence and rejection control |
| --- | --- | --- |
| B1: loading and matching ownership | Environment/step loading and global matcher/registry state belong to Behave. Keyword-specific matches are tried before generic step definitions; the first successful candidate wins. | Load two step files with matcher changes, conversions and generic fallback through Behave. A shared global registry must not leak definitions into a second isolated session. Do not impose Cucumber.js ambiguity rules. |
| B2: Context layers and cleanup | Context has run, feature, rule and scenario layers. Cleanup runs in reverse registration order, attempts remaining cleanup functions after ordinary exceptions, counts cleanup errors and can rethrow the first one. | Record creation/pop order and cleanup traces across passing, failing and skipped cases. An early cleanup failure cannot silently suppress later cleanup or disappear from the run outcome. Keep custom cleanup-error handlers in the proof. |
| B3: step and hook outcomes | An undefined step returns before step hooks. Matched steps run before-step, then body unless hook setup failed, then after-step. Assertion, ordinary error, pending and hook error have separate statuses; WIP and dry-run affect pending/undefined status. | Compare actual hook/body/status traces for each path, including failed before/after hooks and remaining steps. Reject a projection that converts all failures to one successful or generic completed step. |
| B4: nested steps | Context.execute_steps parses and executes nested steps through Behave, with quiet reporting/capture controls. It saves text/table and restores them on the successful path; the tagged source raises before that restoration on nested failure. | Treat nested invocation as an explicit later compatibility boundary. Observe both success and caught failure with parent text/table; do not promise unconditional restoration or invent standalone source Pickles for nested text. |
| B5: async and host loop | The async wrapper calls run_until_complete. Timeout uses asyncio.timeout when available; older interpreters can warn that timeout support is unavailable. | Pin interpreter and loop ownership. Compare completion, timeout and host-loop conflict through the real wrapper. Unsupported active-loop invocation must fail explicitly rather than return an unawaited coroutine as success. |
| B6: capture and attachments | Step execution owns capture start/stop and captured deltas; formatters observe match/result. Nested execution can suppress its ordinary formatter/capture path. | Preserve real stdout/stderr/log/attachment provenance, step identity and timing. Do not infer attachment ownership from report ordering or reuse one step's captured data for another. |

Primary tagged sources: [registry](https://github.com/behave/behave/blob/v1.3.3/behave/step_registry.py),
[Context/runner](https://github.com/behave/behave/blob/v1.3.3/behave/runner.py),
[step model](https://github.com/behave/behave/blob/v1.3.3/behave/model.py),
[async wrapper](https://github.com/behave/behave/blob/v1.3.3/behave/async_step.py).
The B4 failure-path observation is source-level and needs differential behavioral
proof; it is not an instruction to change Behave or our shared compiler.

## pytest-bdd requirements

| Requirement | Tagged source finding | Future behavioral evidence and rejection control |
| --- | --- | --- |
| P1: collection and fixture ownership | Step providers are selected through visible fixture definitions, sorted by fixture path, temporarily injected and resolved through request.getfixturevalue. | Run with parent/child conftest files and overriding step fixtures. Prove visibility, dependencies, scope and teardown using a real pytest session; direct handler calls are insufficient. |
| P2: argument preparation | Parsed arguments are restricted to the handler signature; datatable/docstring are reserved, injected arguments. Missing required arguments resolve as fixtures. | Cover custom parsers/converters, reserved-name collisions, empty arguments and fixture lookup failures. Preserve Python object identity and canonical Rust graph bytes separately. |
| P3: lifecycle exception boundaries | before_step precedes the argument/body try block. Argument preparation, before_step_call and call_fixture_func are inside it; Exception and pytest Failed trigger step_error. target_fixture injection and after_step occur after that block. after_step means successful execution. | Compare ordered traces for failure in each distinct position. Do not emit step_error or after_step for every failure indiscriminately. Explicitly include pytest skip/xfail and BaseException-derived outcomes. |
| P4: scenario hooks and yield teardown | before_scenario precedes the scenario try/finally. Once inside that block, after_scenario runs from finally. Yield steps execute with call_fixture_func and pytest-managed finalization. | Include failed before_scenario, missing-step lookup, failed body and teardown failure. Preserve teardown reports even when the step body passed; do not equate after_step with completed fixture teardown. |
| P5: outline identity and tags | Example rows are parametrized through pytest; display IDs join row values. Example tags become parameter marks; feature/rule/scenario tags use pytest_bdd_apply_tag. | Use duplicate row values, multiple Examples groups and custom tag handling. Map the actual collected item to original URI/Examples-row identity, retaining pytest's final node ID. Display names alone cannot identify rows or rerun selectors. |
| P6: report fidelity | pytest-bdd serializes step fields including a failed boolean and duration; pytest also owns setup/call/teardown reports. | Preserve the original pytest reports alongside any Messages projection. A single failed boolean cannot recover skip/xfail/teardown distinctions or prove the process completed. |
| P7: optional concurrency/async plugins | The tagged step call path has no explicit await branch. pytest plugin/session ownership remains distinct from native scheduling. | Base acceptance excludes unproved plugin combinations. Any pytest-asyncio/xdist support needs separately pinned versions, collection agreement, process/loop identities and cleanup evidence; never nest independent scheduling owners silently. |

Primary tagged sources: [scenario execution](https://github.com/pytest-dev/pytest-bdd/blob/9.0.1/src/pytest_bdd/scenario.py),
[hook contracts](https://github.com/pytest-dev/pytest-bdd/blob/9.0.1/src/pytest_bdd/hooks.py),
[reporting](https://github.com/pytest-dev/pytest-bdd/blob/9.0.1/src/pytest_bdd/reporting.py),
[plugin integration](https://github.com/pytest-dev/pytest-bdd/blob/9.0.1/src/pytest_bdd/plugin.py).
Context7 documentation was consulted for navigation; these tagged sources govern
the version-specific findings, including hook exception boundaries.

## Cross-framework evidence contract to adopt later

A proposed future adapter case should retain the original framework invocation,
exit status and reports; framework version and resolved dependency/ABI/platform
identities; exact source bytes and canonical AST/Pickle IDs; actual collected
case identity; callback/hook/context/fixture traces; attempt and process/worker
identity; and cleanup observations. Compare the adapter with the pinned original
framework running equivalent fixtures, not with expected output generated by the
adapter itself. Mutate a row association, omit teardown, or drop an outcome as a
negative control appropriate to the adopted behavior.

Mappings that cannot represent a framework outcome losslessly must retain the
original report and expose the limitation. An empty Rust rerun selection does
not authorize rerunning all tests, and a formatter's empty selection cannot hide
a failed setup, collection or teardown. Public package names, registry ownership
and publication remain separate decisions. No new dependency fork is justified
by these requirements alone.

## Decisions still needed for later adoption

- Bound the first foreign callback phase: synchronous calls first or an explicit
  host-loop integration, supported interpreter/ABI, cancellation and reentrancy.
- Adopt a finite adapter behavior set for each framework with a resolved version
  lock and real installed-package consumers. The tables above are candidates,
  not new mandatory work inserted into phase46.
- Specify retained native report formats and any Messages projection policy,
  including unrepresentable statuses, nested steps and fixture teardown.
- Decide plugin combinations only when a concrete consumer needs them. No
  universal Python Cucumber parity claim follows from shared source compilation.

Phase46 signature and installed-native boundary are complete; final proof passed22 cases/66 steps. These framework adapter requirements remain future, unadopted scope. See [SDK46](PHASE46-SDK.md).