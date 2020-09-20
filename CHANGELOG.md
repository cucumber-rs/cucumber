### Current

#### Known issues:

- Filtering of tests bug tag was refactored out by accident ([#67](https://github.com/bbqsrc/cucumber-rust/issues/67))
- `Scenario Outline` is treated the same as `Outline` or `Example` in the parser ([gherkin/#19](https://github.com/bbqsrc/cucumber-rust/issues/19))

If these issues affect you, it is recommended to stick with v0.6.x for a little longer, or contribute a fix. 😄

### 0.7.3 — 2020-09-20

- Fix missing mut in t! macro for regexes ([#68](https://github.com/bbqsrc/cucumber-rust/issues/68)) — thanks [@stefanpieck](https://github.com/stefanpieck)!

### 0.7.2 — 2020-09-14

- Enforce `UnwindSafe` on async test types

### 0.7.1 — 2020-09-09

- Fix issue with `t!` macro for unbraced blocks

### 0.7.0 — 2020-09-07

- **Breaking changes**: the macro approach provided in 0.6.x and lower has been entirely removed. It was hard to maintain and limited maintenance of the tests themselves.
- A new builder approach has been implemented.
- Support for asynchronous tests has been implemented — this is runtime agnostic.