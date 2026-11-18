# Phase30b: explicit completion and outcome semantics

Status:completed. Core20 scenarios/87 steps and consumer18 scenarios/54 steps
passed at the phase boundary. The original29 business cases were authored first;
nine additional retry-consumer cases were authored before their implementation.
Shared30b/33 affected BDD passed209 scenarios/1008 steps across17 named targets.

Evidence: [core](evidence/phase30b-core-first-bdd.log),
[consumers](evidence/phase30b-formats-bdd.log),
[affected](evidence/phase30b-33-affected-bdd.log). The first two consumer compile
attempts required harness composition/type-annotation corrections; no scenario
had run then. Those diagnostics remain in the evidence directory.

The explicit policy is `OutcomePolicy::CucumberJs { strict_pending }`; default
Legacy remains unchanged and control is disabled. Each actual callback/body owns
one ExecutionControl invocation guard; World initialization has a WorldContext
but no active control scope. `handle.complete(Pending|Skipped)` records the first
request; it does not return from user code. Panic/timeout overrides requests.
Handles close on completion/drop and never retarget across callbacks or attempts.
Overlapping engine scopes are an invariant violation. The default new World
constructor delegates through the existing attachment-aware constructor.

`Event::outcome: Option<Outcome>` preserves authoritative Copy metadata through
split/map/replace/normalize. Outcome contains actual status, fails_run and retryable
(eligibility, distinct from remaining attempts). Every JS terminal Step/Hook and
Scenario::Finished carries metadata; Legacy events retain None. The existing public
native enum variants remain source compatible. Pending/Skipped step projection is
Step::Skipped; controlled nonpassing scenario hooks use a Hook::Failed diagnostic
carrier without panicking. Old unaware custom writers see a documented lossy
nonpassing projection. Metadata-aware built-ins must never call Pending passed.

Use one shared ordered native step-plan iterator, preserving Legacy's first
nonpassing break. Explicit JS mode continues classification of later planned steps:
current worst SKIPPED bypasses lookup; Pending/Failed still discover later undefined
or ambiguous steps. Once a World-bearing failure is deferred, queue subsequent
start/terminal descriptors with original metadata and flush in original order after
cleanup, retaining ownership of the World. Do not fork a second runner or fabricate
timestamps. Only aggregate FAILED is retryable under the explicit JS policy.

Every selected step hook still runs. BeforeStep Pending/Skipped does not suppress
the body; only failed setup does. AfterStep receives truthful Pending/Skipped body
variants. Registry observations retain normal-return Result separately from exact
Outcome. Scoped scenario cleanup sees the evolved actual status; legacy body
snapshot remains a clearly separate projection. Only real panic/timeout failures
enter prior-failure diagnostics. Cleanup failure can override Pending and enable
retry; normal control requests cannot impersonate errors or generate panic hooks.

The completed boundary must cover native execution, official Messages/Query and
Basic/Summarize/FailOnSkipped/Repeat/Normalize/Tee/JSON/JUnit/libtest behavior.
Strict Pending fails, non-strict Pending succeeds unless a worse outcome occurs,
and explicit skip alone succeeds. Stats and run_and_exit must agree and include a
meaningful pending failure diagnostic. A format without Pending may use an honest
nonpassing representation and explicit failure semantics; never silently pass it.

Only focused BDD targets ran at phase end. No unit suites, generic Cargo test,
premature build/check or lint ran. See PHASE30B-FORMATS.md for format projections.
