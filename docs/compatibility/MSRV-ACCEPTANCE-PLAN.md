# Rust1.88 acceptance boundary — passed

Final result: all13 selected BDD targets passed172 scenarios/670 steps, exit0,
on `rustc 1.88.0 (6b00bc388 2025-06-23)`. Production required no MSRV fixes.
The exact command is in [msrv-1.88-command.txt](evidence/msrv-1.88-command.txt),
with [full evidence](evidence/msrv-1.88-first-bdd.log) and
[compiler identity](evidence/msrv-toolchain.txt). The original plan follows
as historical context; its pending statements describe the pre-execution state.


The manifest promises Rust1.88. Existing completed phase evidence used the session's
Rust1.99.1 toolchain. Rust1.88 is installed, but has not run acceptance. Static review
of215 available resolved registry manifests found none declaring a higher MSRV;
34 omit MSRV and one resolved manifest was absent. This is not compilation evidence.

After active production phases are complete, run named business acceptance targets
under the installed1.88 toolchain with the actual output-messages/output-json/
output-junit/libtest/tracing features. Reuse the authored scenarios that exercise
custom/generated support code, official lexical source handling, ordered scheduling
and retries, dry-run, explicit outcomes, and native formatter integration. These
are meaningful runtime/consumer contracts; do not introduce unit tests or a generic
Cargo build/check as a substitute. Record exact selected targets, compiler version,
command, scenario counts and result. Do not run while an implementation is partial.

Any compiler/dependency incompatibility must retain its initial diagnostic. Fix only
an evidenced incompatibility, preserve unrelated dependency choices, and rerun the
focused affected BDD. A successful newer compiler or manifest metadata alone must
never be presented as proof of the declared minimum version.

No MSRV acceptance has run at the time this plan was authored. It remains an explicit
remaining delivery requirement in NEXT-PHASES.md and LIVE-STATUS.md.

## Final selected boundary

After Phase41 completes, set `RUSTUP_TOOLCHAIN=1.88.0` for the exact named Cargo
BDD command. This pins nested actual-consumer invocations as well as the parent.
Use `--locked` to preserve the established dependency resolution; do not silently
change dependencies to make the declaration pass. Include timeout_preparation,
ordered_scheduling, dynamic_assignment, retry_policy, source_paths,
messages_table_columns, expression_names, execution_control_formats,
dry_run_formats, messages_logs, generated_ordering, portability and vlbv_adapter
with the declared output-messages/output-json/output-junit/libtest/tracing features.
The standalone VLBV consumer proves protocol behavior; it does not establish the
Rust compiler identity itself. Preserve initial compiler/dependency diagnostics.

## Through45 follow-up

Phases43–45 used Rust1.88 for their first primary execution:87 scenarios/348
steps across3 named targets, including25 historical VLBV cases. Eight affected
root targets passed106/460 on the same compiler, including the actual relocated
consumer. No compilation/runtime correction was needed. These are distinct runs,
not additional unique coverage to sum with the earlier172/670 checkpoint.
[Commands and evidence](PHASE43-45-BOUNDARY.md).
