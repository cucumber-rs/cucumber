# Phase35: dry-run classification across JSON, JUnit and libtest

Status: the completed-phase shared boundary passed all nine formatter acceptance
scenarios: **9 scenarios / 30 steps**, with no failed or skipped acceptance steps.
The scenarios were authored before the bounded production change. No tests,
checks or builds ran before the shared phase was complete.

The dry-run runner retains actual `Skipped`, `Undefined`, and `Ambiguous` status
metadata while setting `fails_run=false` and `retryable=false`. JSON already maps
these exact statuses. JUnit already uses final scenario outcome flags and projects
non-failing classifications to skipped testcases, retaining the native diagnostic
in its metadata-aware captured output. Neither requires a new production change.

Libtest previously handled Pending/Skipped/Undefined metadata but let Ambiguous
fall through the native Step::Failed projection. Its existing failed counter then
made inspection fail despite authoritative `fails_run=false`. The bounded fix
adds this non-failing ambiguity case to the ignored-event projection, retaining
the actual ambiguity diagnostic and both candidate regexes. Failing ambiguity in
ordinary execution keeps its existing failed-event path. Legacy events and the
Phase30b pending/skipped behavior remain unchanged.

The focused `dry_run_formats` target covers JSON, JUnit and libtest independently:

- A recognized, an undefined, and an ambiguous scenario remain distinguishable;
  inspection succeeds, constructs three Worlds once, invokes no callbacks, and
  performs no retries.
- Real World initialization failure still fails inspection, with its diagnostic,
  one constructor attempt, and no callback execution or retry.
- Turning inspection off restores ordinary execution: the known body passes,
  undefined/ambiguous cases fail, and ambiguity details remain visible.

Every application run installs scenario, step and global hooks, including the
RunHooks statistics wrapper. Tests assert the wrapper's final success agrees with
the actual JSON/JUnit/libtest report. No new fork or dependency is required.

Focused reproduction command (the recorded run used the coordinated multi-target
32b/34/35 boundary with tracing also enabled):

```sh
cargo test --test dry_run_formats --features output-messages,output-json,output-junit,libtest -- --tags @phase35_formats
```

## Recorded acceptance evidence

The [third shared boundary log](evidence/phase32b-34-35-third-bdd.log) records
`dry_run_formats` at lines 82–126: **9 scenarios passed / 30 steps passed**.
The same log also records the separate core dry-run and line-selector targets;
the overall command later failed in the independently owned Phase32b metadata
harness. That later failure does not erase the formatter result, but this document
does not claim the whole shared command passed.

The [first boundary log](evidence/phase32b-34-35-first-bdd.log) retains the initial
formatter harness compilation failure: `StatsWriter` was not imported, so methods
on the RunHooks statistics wrapper were unavailable. The correction added
`StatsWriter as _` to the harness import only. No formatter runtime correction was
required after acceptance began. Other shared-target diagnostics remain in their
original logs rather than being hidden or attributed to this phase.

No unit suites, generic cargo test, premature checks, publishing or deployment
were performed for this formatter change. Phase37 remains design/features only
until the coordinated source-ownership release.
