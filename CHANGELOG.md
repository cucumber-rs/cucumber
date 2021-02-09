### Current

#### Known issues:

- `Scenario Outline` is treated the same as `Outline` or `Example` in the parser ([gherkin/#19](https://github.com/bbqsrc/gherkin-rust/issues/19))

### 0.8.3 — 2021-02-09

- Update `t!` macro to support specifying type of world argument in closure

### 0.8.2 — 2021-01-30

- Re-export `async_trait::async_trait` and `futures` crate for convenience
- Update examples to use `tokio`

### 0.8.1 — 2021-01-30

- Added proper i18n support via gherkin 0.9

### 0.8.0 — 2021-01-18

- Fixed filtering of tests by tag ([#67](https://github.com/bbqsrc/cucumber-rust/issues/67))
- Implemented failure reporting ([#91](https://github.com/bbqsrc/cucumber-rust/issues/91))
- Removed unnecessary dependent traits from `World` trait
- Added proc-macro variant (thanks Ilya Solovyiov and Kai Ren)

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