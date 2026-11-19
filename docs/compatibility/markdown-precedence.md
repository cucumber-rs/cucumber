# Phase45: literal Markdown payloads and tag precedence

Status: the completed primary boundary passed **21 scenarios / 100 steps** on
**Rust 1.88**, first attempt, exit0. Evidence:
[`phase43-45-first-bdd.log`](evidence/phase43-45-first-bdd.log). Features preceded
implementation, and the whole bounded phase and manual review were complete
before execution. No oracle, check or test ran before that boundary.

The native Markdown loop previously extracted every inline backtick tag before
recognizing steps, table rows or opening fences. This could turn executable text
such as `- Given render ` followed by a literal backtick-wrapped `@literal` into
pending tags, losing a step or rejecting an otherwise valid document.

Pinned official source: Gherkin43 commit
`3d70e2bb4e0f16a568ee539a5ce0b03fb247a777`:

- [Generated parser states12/14/15/19](https://github.com/cucumber/gherkin/blob/3d70e2bb4e0f16a568ee539a5ce0b03fb247a777/javascript/src/Parser.ts)
  prioritize StepLine, eligible TableRow and DocStringSeparator before TagLine in
  their respective grammar states.
- [Markdown matcher](https://github.com/cucumber/gherkin/blob/3d70e2bb4e0f16a568ee539a5ce0b03fb247a777/javascript/src/GherkinInMarkdownTokenMatcher.ts)
  recognizes backtick tags anywhere on a line; grammar precedence determines
  whether that line is a tag or executable content.

The bounded parser fix caches candidate step/row/fence tokens and gives executable
payloads priority only in scenario/background or Examples table contexts, with no
pending declaration tags. Genuine standalone/declaration tags retain priority.
Existing multiplicity and row-width checks still reject invalid arguments. No
classic parser/default, public AST, source format, protocol or dependency changes
are part of this phase. Typed Markdown nodes feed the existing SourceGraph without
a converter rewrite. This does not resolve the independent upstream fallback
Feature.keyword/schema contradiction documented in the filtering audit.

The maintained source of truth is `../gherkin/src/markdown.rs`, mirrored in
`vendor/gherkin/src/markdown.rs` with refreshed SHA256SUMS/provenance. No new fork
or publication. Acceptance is in `tests/messages_markdown_precedence.rs` and
`tests/features/messages_markdown_precedence/precedence.feature`: **21 scenarios,
100 passed steps**, covering first/following/background steps, data/Examples
rows, media fences, both argument orders, literal docstring content, genuine tags,
consumed argument slots falling back to declaration tags, and five invalid
argument/tag shapes. The two consumed-slot preservation scenarios were authored
before the corresponding minimal slot guard was added during manual review.

Positive cases compare every AST/Pickle field and reference against the pinned
official Markdown oracle, allowing only URI and bijective ID normalization. The
same cases execute actual Rust handlers through captured Messages35, validate its
schema, and assert literal argument values, original bytes and typed spans/
locations. Invalid cases require official/native rejection, zero handler calls
and unsuccessful run completion. Node is only the phase-end acceptance oracle;
the production parser/compiler remains native Rust.

The focused target is `messages_markdown_precedence` with `output-messages`
and tag `@phase45`; the evidence log records the coordinated executed command.
The shared eight-target affected BDD batch passed106 scenarios/460 steps on
Rust1.88, including Markdown graph/runtime, table columns and source selectors.
The maintained sibling parser's focused `step_arguments` BDD passed73/241 on
Rust1.88. Evidence: [root affected](evidence/phase43-45-affected-bdd.log) and
[sibling](evidence/phase45-parser-affected-bdd.log). No generic Cargo test, unit
suite or publication occurred; production did not change after the first pass.
