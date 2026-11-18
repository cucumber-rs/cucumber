# Upstream contribution overlap preparation

Observed 2026-11-18 via read-only GitHub API metadata, changed-file patches and
public discussion. This is contribution planning and source comparison, not the
final code audit or an upstream submission. Recheck all states and contribution
rules immediately before filing the completion-gated issues and pull requests.

## Current pull request status

| Upstream PR | Observed state | Inspected head | Last updated |
| --- | --- | --- | --- |
| [#391: before_step and after_step hooks](https://github.com/cucumber-rs/cucumber/pull/391) | Open, not draft, not merged | 5feb75d89e5eba488ee95ce63d0ab4d2d38d3832 | 2026-03-13T10:14:10Z |
| [#405: independent shards](https://github.com/cucumber-rs/cucumber/pull/405) | Open, not draft, not merged | d1f60a83914331c93773d14285ce8efc7d141571 | 2026-08-25T21:05:53Z |

Neither PR returned submitted review records from the reviews API at this check.
That is not an approval/rejection conclusion. PR391 has substantive discussion;
PR405 returned no issue comments. A compact [research receipt](research/2026-11-18-framework-and-upstream.json)
records source identities and API endpoints. No CI/check suites were run or
relied on as our acceptance evidence.

## PR391 and our step hooks/reporting

At the inspected head, the patch changes src/cucumber.rs, src/runner/basic.rs and
adds an event-type TODO in src/event.rs. It introduces single before_step and
after_step callback slots and two Basic generic parameters. After-step receives
an event::Step and optional mutable World. Its implementation invokes after-step
on the skipped-result branch as well as passing/failing branches, and the passing
event is sent before that after-step callback. Its before-step await sits outside
the body catch_unwind in the inspected patch. These are source observations;
this preparation did not execute or audit the upstream patch.

Our [Phase20 implementation and retained BDD](PHASE20-STEP-HOOKS.md) uses an
additive StepHooks collection: ordered setup, reverse teardown, inherited tag
filters, a shared body outcome and invocation records. Undefined, ambiguous and
unexecuted steps invoke no step hooks. Hook failures feed the native step result,
and no individual callback is invented as a Messages TestStep. Later scoped
attachments and timing/identity reporting also address part of PR391's motivating
reporting need. The two implementations overlap materially but are not identical
patches or behavioral contracts.

The maintainer discussion asks why Writer and step decoration are insufficient;
the author describes step-local log/attachment setup and mutable World needs,
then notes attachments plus decoration as another possible solution. Preserve
that distinction when proposing contributions. See the [maintainer's follow-up](https://github.com/cucumber-rs/cucumber/pull/391#issuecomment-3877432978)
and [author's response](https://github.com/cucumber-rs/cucumber/pull/391#issuecomment-3893526531).

Contribution preparation:

1. Reference and coordinate with the existing PR rather than file a second generic
   request for step hooks. No contact has been sent during this preparation.
2. Separate the actual attachment/Writer reporting capability from callback
   lifecycle behavior, with each contribution carrying its own retained BDD.
3. Agree callback composition, skipped/undefined semantics, panic handling,
   event timing and source compatibility before adapting patches for upstream.
4. Do not offer a wholesale replacement merely because our fork has more features;
   use the smallest reviewed contribution that addresses the agreed requirement.

## PR405 and our scheduling/filtering

At the inspected head, PR405 adds a public cli::Shard and an optional field on
cli::Opts, parsed from a one-based INDEX/TOTAL argument. It assigns scenarios by
index modulo total after the effective CLI/programmatic filter, using parser
order; expanded outline rows are independent cases. The patch includes book
material, CLI cases and a sharding feature. It explicitly separates cross-process
sharding from in-process concurrency and says serial tags do not coordinate
independent processes. Source: [PR405 changed files](https://github.com/cucumber-rs/cucumber/pull/405/files).

Our [dynamic assignment](PHASE37B-DYNAMIC-ASSIGNMENT.md), ordering/replay and
resource reservations operate inside one native runner. Inspection at fork
commit 9e1690f found no equivalent --shard surface in src/cli.rs. Cross-process
sharding is therefore a separate remaining capability, not coverage already
provided by our scheduler. PR405 intersects our filter/discovery pipeline and
public CLI layout. Phase33 deliberately preserved the existing Opts layout;
adding a public field needs a source-compatibility decision for struct-literal
consumers. No integration change is made here.

Contribution preparation:

1. Avoid a competing sharding implementation while PR405 remains active. At the
   contribution stage, coordinate with or build on its accepted design.
2. Define the order of discovery, outline expansion, combined filters, sharding,
   global random order and retry scheduling. Prove every eligible original case
   belongs to exactly one shard and retains original row identity.
3. Pin the same source inventory and configuration across independent jobs;
   different parser ordering or filters can invalidate membership assumptions.
4. Keep global resource coordination explicit: process-local serial tags and
   reservation predicates do not provide a distributed lock.
5. Preserve established workflows. The upstream book's CI examples are examples,
   not authorization to change our mature build/release process.

## Dependency and submission order

The sole established dependency fork remains GQAdonis/gherkin. Prepare parser and
source-semantics contributions separately from Cucumber consumers. A runner or
Messages contribution that depends on parser behavior must identify the upstream
parser change and usable dependency revision; our portable vendored snapshot is
current fork integration, not proof upstream accepts that packaging strategy.
SDK/binding proposals should be separately reviewable and describe their explicit
compatibility limits, ownership and optional-framework boundaries.

At programme completion: finish the adopted SDK BDD, complete the requested
independent audits and material corrections for both codebases, refresh upstream
policies and overlap state, then open coordinated matching issues/PRs in dependency
order. The owner has authorized that milestone; none has been opened prematurely.
This preparation leaves the exact unsigned KBD completion amendment, credentials,
allowed-signers, canonical state and blocked SDK verification untouched.
