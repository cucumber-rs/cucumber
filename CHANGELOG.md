`cucumber` changelog
====================

All user visible changes to `cucumber` crate will be documented in this file. This project uses [Semantic Versioning 2.0.0].




## [0.10.0] · 2021-??-??
[0.10.0]: /../../tree/v0.10.0

[Diff](/../../compare/v0.9.0...v0.10.0)

### BC Breaks

- Renamed crate to `cucumber`.
- Complete redesign: ([#128])
    - Introduce new abstractions: `Parser`, `Runner`, `Writer`;
    - Provide reference implementations for those abstractions;
    - Enable `macros` feature by default.
- Replaced `#[given(step)]`, `#[when(step)]` and `#[then(step)]` function argument attributes with a single `#[step]`. ([#128])
- Made test callbacks first argument `&mut World` instead of `World`. ([#128])
- Made `#[step]` argument of step functions `Step` instead of `StepContext` again, while test callbacks still receive `StepContext` as a second parameter. ([#128])
- Deprecated `--nocapture` and `--debug` CLI options to be completely redesigned in `0.11` release. ([#137])
- [Hooks](https://cucumber.io/docs/cucumber/api/#hooks) now accept optional `&mut World` as their last parameter. ([#142])

### Added

- Ability to run `Scenario`s concurrently. ([#128])
- Highlighting of regex capture groups in terminal output with __bold__ style. ([#136])
- Error on a step matching multiple step functions ([#143]).

[#128]: /../../pull/128
[#136]: /../../pull/136
[#137]: /../../pull/137
[#142]: /../../pull/142
[#143]: /../../pull/143




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




[`gherkin_rust`]: https://docs.rs/gherkin_rust

[Semantic Versioning 2.0.0]: https://semver.org
