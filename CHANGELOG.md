`cucumber` changelog
====================

All user visible changes to `cucumber` crate will be documented in this file. This project uses [Semantic Versioning 2.0.0].




## [0.11.0] · 2021-??-??
[0.11.0]: /../../tree/v0.11.0

[Diff](/../../compare/v0.10.2...v0.11.0) | [Milestone](/../../milestone/3)

### BC Breaks

- Moved `World` type parameter of `WriterExt` trait to methods. ([#160])
- Renamed `Normalized` and `Summarized` `Writer`s to `Normalize` and `Summarize`. ([#162])
- Removed `writer::Basic` `Default` impl and change `writer::Basic::new()` return type to `writer::Normalize<writer::Basic>`. ([#162])
- Bump up [MSRV] to 1.57 for better error reporting in `const` assertions. ([cef3d480])
- Switch to [`gherkin`] crate instead of [`gherkin_rust`]. ([rev])

### Added

- Ability for step functions to return `Result`. ([#151])
- Arbitrary output for `writer::Basic`. ([#147])
- `writer::JUnit` ([JUnit XML report][0110-1]) behind the `output-junit` feature flag. ([#147])
- `writer::Json` ([Cucumber JSON format][0110-2]) behind the `output-json` feature flag. ([#159])
- `writer::Tee` for outputting to multiple terminating `Writer`s simultaneously. ([#160])
- `writer::discard::Arbitrary` and `writer::discard::Failure` for providing no-op implementations of the corresponding `Writer` traits. ([#160])
- Inability to build invalid `Writer`s pipelines:
    - `writer::Normalized` trait required for `Writer`s in `Cucumber` running methods. ([#162])
    - `writer::NonTransforming` trait required for `writer::Repeat`. ([#162])
    - `writer::Summarizable` trait required for `writer::Summarize`. ([#162])
- Support for [Cucumber Expressions] via `#[given(expr = ...)]`, `#[when(expr = ...)]` and `#[then(expr = ...)]` syntax. ([#157])
- Support for custom parameters in [Cucumber Expressions] via `#[derive(cucumber::Parameter)]` macro. ([#168])
- Merging tags from `Feature` and `Rule` with `Scenario` when filtering with `--tags` CLI option. ([#166])

### Fixed

- Template regex in `Scenario Outline` expansion from `<(\S+)>` to `<([^>\s]+)>`. ([#163])
- Multiple `Examples` in `Scenario Outline`. ([#165], [#164])
- Docstring and name expansion in `Scenario Outline`. ([#178], [#172])

[#147]: /../../pull/147
[#151]: /../../pull/151
[#157]: /../../pull/157
[#159]: /../../pull/159
[#160]: /../../pull/160
[#162]: /../../pull/162
[#163]: /../../pull/163
[#164]: /../../issues/164
[#165]: /../../pull/165
[#166]: /../../pull/166
[#168]: /../../pull/168
[#172]: /../../pull/172
[#178]: /../../pull/178
[cef3d480]: /../../commit/cef3d480579190425461ddb04a1248675248351e
[rev]: /../../commit/rev-full
[0110-1]: https://llg.cubic.org/docs/junit
[0110-2]: https://github.com/cucumber/cucumber-json-schema




## [0.10.2] · 2021-11-03
[0.10.2]: /../../tree/v0.10.2

[Diff](/../../compare/v0.10.1...v0.10.2) | [Milestone](/../../milestone/5)

### Fixed

- Multiple `WorldInit` derivers conflicting implementations in a single module. ([#150])

[#150]: /../../pull/150




## [0.10.1] · 2021-10-29
[0.10.1]: /../../tree/v0.10.1

[Diff](/../../compare/v0.10.0...v0.10.1) | [Milestone](/../../milestone/4)

### Fixed

- Console output hanging because of executing wrong `Concurrent` `Scenario`s. ([#146])

[#146]: /../../pull/146




## [0.10.0] · 2021-10-26
[0.10.0]: /../../tree/v0.10.0

[Diff](/../../compare/v0.9.0...v0.10.0) | [Milestone](/../../milestone/2)

### BC Breaks

- Renamed crate to `cucumber`.
- Complete redesign: ([#128])
    - Introduce new abstractions: `Parser`, `Runner`, `Writer`;
    - Provide reference implementations for those abstractions;
    - Enable `macros` feature by default.
- Replaced `#[given(step)]`, `#[when(step)]` and `#[then(step)]` function argument attributes with a single `#[step]`. ([#128])
- Made test callbacks first argument `&mut World` instead of `World`. ([#128])
- Made `#[step]` argument of step functions `Step` instead of `StepContext` again, while test callbacks still receive `StepContext` as a second parameter. ([#128])
- Completely redesign and reworked CLI, making it composable and extendable. ([#144])
- [Hooks](https://cucumber.io/docs/cucumber/api/#hooks) now accept optional `&mut World` as their last parameter. ([#142])

### Added

- Ability to run `Scenario`s concurrently. ([#128])
- Highlighting of regex capture groups in terminal output with __bold__ style. ([#136])
- Error on a step matching multiple step functions ([#143]).
- `timestamps` Cargo feature that enables collecting of timestamps for all the happened events during tests execution (useful for `Writer`s which format requires them) ([#145]).

[#128]: /../../pull/128
[#136]: /../../pull/136
[#137]: /../../pull/137
[#142]: /../../pull/142
[#143]: /../../pull/143
[#144]: /../../pull/144
[#145]: /../../pull/145




## [0.9.0] · 2021-07-19
[0.9.0]: /../../tree/v0.9.0

[Diff](/../../compare/v0.8.4...v0.9.0)

### BC Breaks

- The second parameter in the test callbacks is now a `StepContext` object, which contains the `Step` as a `step` field.

### Added

- Add `before` and `after` lifecycle functions to the `Cucumber` builder. These functions take a selector for determining when to run `before` or `after`, and a callback.

### Fixed

- Literal paths to `.feature` files will now work in the `Cucumber` builder.
- Removed unnecessary internal `Rc<T>` usage.




## [0.8.4] · 2021-02-18
[0.8.4]: /../../tree/v0.8.4

[Diff](/../../compare/v0.8.3...v0.8.4)

### Added

- `language` argument to `Cucumber` builder to set default language for all `.feature` files.
- `--debug` flag to always print STDOUT and STDERR per step.




## [0.8.3] · 2021-02-09
[0.8.3]: /../../tree/v0.8.3

[Diff](/../../compare/v0.8.2...v0.8.3)

### Changed

- Update `t!` macro to support specifying type of `World` argument in closure.




## [0.8.2] · 2021-01-30
[0.8.2]: /../../tree/v0.8.2

[Diff](/../../compare/v0.8.1...v0.8.2)

### Added

- Re-export `async_trait::async_trait` and `futures` crate for convenience.
- Update examples to use `tokio`.




## [0.8.1] · 2021-01-30
[0.8.1]: /../../tree/v0.8.1

[Diff](/../../compare/v0.8.0...v0.8.1)

### Added

- Proper i18n support via [`gherkin_rust`] `0.9`.




## [0.8.0] · 2021-01-18
[0.8.0]: /../../tree/v0.8.0

[Diff](/../../compare/v0.7.3...v0.8.0)

### Added

- Failure reporting. ([#91])
- `macros` feature providing attributes: ([#81])
    - [`given`](https://docs.rs/cucumber_rust/0.8.0/cucumber_rust/attr.given.html);
    - [`when`](https://docs.rs/cucumber_rust/0.8.0/cucumber_rust/attr.when.html);
    - [`then`](https://docs.rs/cucumber_rust/0.8.0/cucumber_rust/attr.then.html).

### Fixed

- Filtering of tests by tag. ([#67])
- Removed unnecessary dependent traits from `World` trait.

[#67]: /../../issues/67
[#81]: /../../pull/81
[#91]: /../../issues/91




## [0.7.3] · 2020-09-20
[0.7.3]: /../../tree/v0.7.3

[Diff](/../../compare/v0.7.2...v0.7.3)

### Fixed

- Fix missing `mut` in `t!` macro for regexes — thanks [@stefanpieck](https://github.com/stefanpieck)! ([#68])

[#68]: /../../issues/68




## [0.7.2] · 2020-09-14
[0.7.2]: /../../tree/v0.7.2

[Diff](/../../compare/v0.7.1...v0.7.2)

### Added

- Enforce `UnwindSafe` on async test types.




## [0.7.1] · 2020-09-09
[0.7.1]: /../../tree/v0.7.1

[Diff](/../../compare/v0.7.0...v0.7.1)

### Fixed

- Issue with `t!` macro for unbraced blocks.




## [0.7.0] · 2020-09-07
[0.7.0]: /../../tree/v0.7.0

[Diff](/../../compare/v0.6.8...v0.7.0)

### BC Breaks

- The macro approach provided in `0.6.x` and lower has been entirely removed. It was hard to maintain and limited maintenance of the tests themselves.

### Added

- A new builder approach has been implemented.
- Support for asynchronous tests has been implemented — this is runtime agnostic.




[`gherkin`]: https://docs.rs/gherkin
[`gherkin_rust`]: https://docs.rs/gherkin_rust

[Cucumber Expressions]: https://cucumber.github.io/cucumber-expressions
[MSRV]: https://doc.rust-lang.org/cargo/reference/manifest.html#the-rust-version-field
[Semantic Versioning 2.0.0]: https://semver.org
