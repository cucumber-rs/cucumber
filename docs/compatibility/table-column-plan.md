# Phase38b plain-table column correction

The pinned [Gherkin43 GherkinLine implementation](https://github.com/cucumber/gherkin/blob/3d70e2bb4e0f16a568ee539a5ce0b03fb247a777/javascript/src/GherkinLine.ts)
advances table-cell positions using JavaScript string indices. Astral characters
therefore consume two column units inside table rows. Token indentation is
computed separately by scalar counting; recognized whitespace contains no astral
characters. The converter's previous plain `table_row` advanced a `Vec<char>` index,
which differs after astral content. The correction now has completed phase-end differential acceptance: **11
scenarios / 53 steps passed**.

The minimal production change is confined to plain SourceGraph conversion:
calculate each cell's start from the original raw prefix in UTF16 units, retaining
its scalar indentation and the already-correct cell whitespace trim set. Keep
escaped delimiters and raw-prefix accounting; never derive locations from a
reconstructed/unescaped value. Row locations, native Table values, byte spans,
source line origins and public native AST shapes do not change. No parser fork
change or new dependency is needed. Markdown's typed parser already advances
cell positions with `len_utf16`; its adapter is a regression boundary.

Feature-first acceptance is authored in
`tests/features/messages_table_columns/columns.feature` (11 scenarios): astral
content, recognized/unknown escapes, combining characters, NBSP versus EM SPACE
cell behavior, Examples interpolation/row selection, Unicode indentation and
unchanged Markdown positions. Compare whole AST/Pickle graphs against the pinned
plain or Markdown oracle, preserving all fields and bijective references; assert
real handler values and exact source bytes. A few hand-selected columns alone
are insufficient evidence.

Current status: the completed coordinated primary boundary passed **11 scenarios
/ 53 steps** for `messages_table_columns`. Evidence is retained in
[`phase30c-37b-38b-40-42-second-bdd.log`](evidence/phase30c-37b-38b-40-42-second-bdd.log).
No tests, checks or oracles ran before the completed phase boundary. The shared
32-target affected regression batch is still running; its result is not claimed
here. Production changes are confined to the plain
converter's cell-column arithmetic. This is distinct from the verified Phase38
lexical policy; its native/parser table-cell trim set remains untouched.

Acceptance checks both captured runtime Messages35 and the direct source
compiler against the pinned official whole graph. It validates the schema,
actual handler names/text/table values, exact bytes, scalar row locations and
Examples-row test-case linkage. No native parser fork changes or new dependencies
were needed. No publication occurred.

Final minimum-toolchain boundary: this target also passed within the13-target
Rust1.88 acceptance (172 scenarios/670 steps total). See
[evidence](evidence/msrv-1.88-first-bdd.log) and
[exact command](evidence/msrv-1.88-command.txt). Earlier pending statements
above record prior checkpoints. Production needed no MSRV correction.
