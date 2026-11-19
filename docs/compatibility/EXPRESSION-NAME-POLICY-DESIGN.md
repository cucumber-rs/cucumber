# Phase42: custom names in explicit expr_js definitions

Status: complete. The coordinated phase-end BDD passed **12 scenarios / 47
steps**, including eight expected compiler rejections and four executed child
BDD scenarios. No unit suite, standalone build/check or premature oracle ran.

## Concrete source discrepancy

`codegen/src/parameter.rs::Definition::try_from` accepts the declared custom name
without JavaScript name validation. This is intentional native permissiveness
and must remain for legacy consumers. `codegen/src/attribute.rs::Step::expand`
uses the same cucumber-expressions0.5 parser for `expr` and `expr_js`.
`parse.rs::parameter::is_name` in that dependency excludes `{}(\\/` but allows
`)`, square brackets, dollar, dot, pipe, question mark, star and plus. Existing
provider/type-name equality checks do not reject those allowed characters.

Pinned cucumber-expressions20.1.0 `ParameterType.checkParameterTypeName` delegates
to `isValidParameterTypeName`, which rejects the actual character class
`[ [ ] ( ) $ . | ? * + ]` (spaces here separate listed characters; space itself
is allowed). The JS source constant is `ILLEGAL_PARAMETER_NAME_PATTERN`; its textual error message
lists a different set and must not be used instead of the implemented rule.
`CucumberExpressionParser.parseName` additionally rejects structural delimiter
tokens, including a bare closing parenthesis. Ordinary spaces, hyphens and
Unicode names remain legal. The existing native parser already rejects bare
opening parentheses in parameter names; that case does not justify new parser
work.

Before this phase: an explicit `expr_js = "value {dotted.name}"` custom
Rust definition can be accepted where the pinned JS registry rejects the same
parameter name. A name containing `)` has an additional JS parser rejection.
The completed compiler-facing BDD now verifies the corrected rejection against
the pinned JS registry, verifies accepted names through actual handlers, and
retains legacy acceptance. Existing successful-expression evidence remains valid.

## Narrow implementation

Validate custom parameter names only when expanding `expr_js`, using the pinned
JS registry character rule and a clear registration diagnostic. Keep existing
native parsing, Parameter derive, `expr`, raw regex registration and manual
metadata APIs unchanged. No dependency fork is needed: this is a narrow opt-in
codegen admission rule, not a replacement expression parser or registry.

`tests/features/expression_names/names.feature` defines12 cases: eight invalid
name examples, three accepted names and one legacy permissiveness case. The
approved BDD harness compiles actual tiny macro consumers in isolated temporary
fixtures at the completed phase boundary, then executes accepted handlers;
rejected cases assert compiler diagnostics and compare the same name with the
installed pinned JS registry. These are compiler-facing BDD scenarios, not a
unit suite or premature cargo check. The parent approved only the named child BDD target `cargo test --test
registration` with `harness=false` inside the outer final BDD boundary. No
standalone cargo build/check, unit suite or trybuild runner is used.

A separate temporary Cargo target prevents nested Cargo lock conflicts
while the parent BDD executable is running. No project dependency manifest,
release pipeline, other repository or installed JS package needs modification.
The harness exercises the actual proc macro; no unused alternate validation API
was added for testing.


## Authored boundary command and evidence retention

Outer target: `cargo test --features output-messages --test expression_names --
--tags @phase42_expression_names` (one command line). Inside each scenario the
only compiler invocation is `cargo test --offline --manifest-path <temporary
consumer>/Cargo.toml --test registration`. The child target is `harness=false`
and its actual Cucumber feature asserts both the custom value and a subsequent
World observation. Invalid cases require the exact name in a source-located
macro diagnostic, not merely a failed command.

One isolated temporary workspace/target is shared sequentially across the12
cases; it starts with a fresh offline lock and pins cucumber0.23.0 to the actual
local repository path. It does not mutate the parent lockfile or publish. Each
boundary uses a new `docs/compatibility/evidence/phase42-children/boundary-*`
directory retaining original stdout/stderr, source/manifest/lock, exact argv,
status/timing and pinned registry-oracle output. Compilation and oracle execution
occur only inside this final BDD target, never during authoring.


## Actual evidence

The successful parent command was:

```sh
cargo test --features output-messages,tracing --test timeout_preparation --test dynamic_assignment --test messages_table_columns --test source_paths --test expression_names
```

`evidence/phase30c-37b-38b-40-42-second-bdd.log` records all five primary targets,
**67 scenarios / 268 steps**, including Phase42 **12/47**. The first joint command
failed while compiling neighboring harnesses; Phase42 did not execute then.
No Phase42 implementation/harness correction was needed at its actual boundary.

Original child evidence is retained under:
`evidence/phase42-children/boundary-1794927495500/`.
Directories00–07 contain the eight expected expr_js compiler failures;08–10
contain accepted expr_js registrations and11 contains the permissive legacy
expr registration. Each receipt records exact named `registration` BDD argv,
exit code and timing, along with source, manifest, resolved lock, original
stdout/stderr and pinned registry output. Accepted children executed one actual
Cucumber scenario with two steps each; their callback marker confirms the value
assertion ran. The legacy dotted.name child passed while the official registry
rejected that name, demonstrating intentional compatibility preservation.

The isolated child uses default macros without Messages. Its library compilation
emitted five existing dead-code warnings for attachment storage fields/methods;
these do not concern name admission and are retained in the child stderr logs.
This phase is passing, not warning-free. No dependency fork, publication or
production scope outside the explicit name check was added. Coordinated affected
regressions remain the parent coordinator's evidence boundary.

Final minimum-toolchain boundary: this target also passed within the13-target
Rust1.88 acceptance (172 scenarios/670 steps total). See
[evidence](evidence/msrv-1.88-first-bdd.log) and
[exact command](evidence/msrv-1.88-command.txt). Earlier pending statements
above record prior checkpoints. Production needed no MSRV correction.
