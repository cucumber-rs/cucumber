# Phases43–45 coordinated behavior boundary

Status: first Rust1.88 primary boundary passed87 scenarios/348 steps, exit0.
Eight affected root targets passed106/460, exit0. No compiler/runtime correction
was needed. Primary includes22/90 new VLBV plus25/85 prior cases,19/73 rerun and
21/100 Markdown precedence. The maintained sibling parser passed73/241 on Rust1.88, also first try:
[sibling evidence](evidence/phase45-parser-affected-bdd.log).

Evidence: [primary](evidence/phase43-45-first-bdd.log),
[affected](evidence/phase43-45-affected-bdd.log),
[exact commands](evidence/phase43-45-commands.txt). The original plan follows.

These three bounded changes address concrete source findings and practical
programmatic use: VLBV receipt consistency, a native Messages rerun plan, and
Markdown executable-token precedence. They do not change build/release workflows.

No compilation, oracle execution or behavior test starts while any of the three
implementations or harnesses is partial. The first execution will use the declared
Rust1.88 compiler, rather than using a newer compiler as a proxy for compatibility:

```sh
RUSTUP_TOOLCHAIN=1.88.0 cargo test --locked --features output-messages --test vlbv_adapter --test messages_rerun --test messages_markdown_precedence
```

The existing VLBV target includes Phase39 regressions and the new Phase43 cases.
The other two targets are dedicated. All scenario counts must be nonzero; an exit
status alone is insufficient. Preserve initial compiler/harness/runtime failures,
correct only evidenced causes, and rerun the affected focused boundary. These are
harness=false BDD targets, not a generic Cargo or unit suite.

After primary acceptance, affected native Markdown, source-selection, retry and
protocol targets are justified by the touched paths. The sibling parser's named
BDD contract and relocated consumer remain appropriate distribution checks for the
changed vendor snapshot. Do not add unrelated build/lint/unit checks.

The actual pinned Cucumber.js13.3 RerunFormatter and EventDataCollector are isolated
acceptance dependencies with a lockfile and provenance; only the completed BDD
harness imports them. Production rerun selection is native Rust. The Markdown
oracle remains pinned official Gherkin43. Node remains optional for production;
the standalone VLBV projection uses only its standard library.

Receipt consistency and source hashes do not prove producer truth. A rerun plan
does not authorize a new boundary, erase failed attempts or replace B5 repair
lineage. Keep original execution streams and receipts when applying the selection.

Through42 source/evidence inventories are preserved under
`evidence/through42-checkpoint/`. Current inventories and final-provenance identify
the completed through45 local snapshot; hashes bind bytes, not observation truth.
