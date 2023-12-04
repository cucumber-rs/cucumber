`cucumber` changelog
====================

All user visible changes to `cucumber` crate will be documented in this file. This project uses [Semantic Versioning 2.0.0].




## [0.20.2] · 2023-12-04
[0.20.2]: /../../tree/v0.20.2

[Diff](/../../compare/v0.20.1...v0.20.2) | [Milestone](/../../milestone/27)

### Fixed

- Ignored verbosity when printing `World` on hook/background step failure. ([#313])

[#313]: /../../pull/313




## [0.20.1] · 2023-10-16
[0.20.1]: /../../tree/v0.20.1

[Diff](/../../compare/v0.20.0...v0.20.1) | [Milestone](/../../milestone/25)

### Fixed

- Incorrect terminal width detection when its height is low. ([#298])
- Incorrect terminal lines clearing in interactive mode. ([#300], [#302], [#299])

[#298]: /../../pull/298
[#299]: /../../issues/299
[#300]: /../../pull/300
[#302]: /../../pull/302




## [0.20.0] · 2023-07-10
[0.20.0]: /../../tree/v0.20.0

[Diff](/../../compare/v0.19.1...v0.20.0) | [Milestone](/../../milestone/24)

### BC Breaks

- Added `Log` variant to `event::Scenario`. ([#258])
- Added `embeddings` field to `writer::json::Step` and `writer::json::HookResult`. ([#261])
- Added `report_time` field to `writer::libtest::Cli`. ([#264], [#265])
- Bumped up [MSRV] to 1.70 for using the `IsTerminal` trait from `std`. ([#288])

### Added

- [`tracing`] crate integration behind the `tracing` feature flag. ([#213], [#258], [#261])
- Support of `--report-time` CLI option for `writer::Libtest`. ([#264], [#265])

### Fixed

- Clearing lines that are wrapped because of terminal width. ([#272], [#273])

[#213]: /../../issues/213
[#258]: /../../pull/258
[#261]: /../../pull/261
[#264]: /../../issues/264
[#265]: /../../pull/265
[#272]: /../../discussions/272
[#273]: /../../pull/273
[#288]: /../../pull/288




## [0.19.1] · 2022-12-29
[0.19.1]: /../../tree/v0.19.1

[Diff](/../../compare/v0.19.0...v0.19.1) | [Milestone](/../../milestone/23)

### Fixed

- Using autodetect for colors on `color=always|never` CLI options. ([#253])

[#253]: /../../pull/253




## [0.19.0] · 2022-12-16
[0.19.0]: /../../tree/v0.19.0

[Diff](/../../compare/v0.18.0...v0.19.0) | [Milestone](/../../milestone/22)

### BC Breaks

- Replaced `writer::FailOnSkipped::writer` field with `writer::FailOnSkipped::inner_writer()` method. ([56456e66])
- Replaced `writer::Normalized::writer` field with `writer::Normalized::inner_writer()` method. ([56456e66])
- Replaced `writer::Or::left`/`writer::Or::right` fields with `writer::Or::left_writer()`/`writer::Or::right_writer()` methods. ([56456e66])
- Replaced `writer::Repeat::writer` field with `writer::Repeat::inner_writer()` method. ([56456e66])
- Replaced `writer::Summarize::writer` field with `writer::Summarize::inner_writer()` method. ([56456e66])
- Replaced `writer::Summarize::scenarios`/`writer::Summarize::steps` fields with `writer::Summarize::scenarios_stats()`/`writer::Summarize::steps_stats()` methods. ([56456e66])
- Made `writer::Summarize::features`/`writer::Summarize::rules` fields private. ([56456e66])
- Made `writer::Summarize::parsing_errors`/`writer::Summarize::failed_hooks` fields private in favour of `writer::Stats::parsing_errors()`/`writer::Stats::failed_hooks()` methods. ([56456e66])

### Added

- [Gherkin] syntax highlighting in the Book. ([#251])
- `runner::Basic::fail_fast()` method as `Cucumber::fail_fast()`. ([#252])
- `Cucumber::with_default_cli()` method. ([56456e66])
- `Default` implementation for CLI types. ([56456e66])

### Fixed

- `@serial` `Scenario`s continue running after failure when `--fail-fast()` CLI option is specified. ([#252])

[#251]: /../../pull/251
[#252]: /../../pull/252
[56456e66]: /../../commit/56456e666be41b4190f62fecaf727042ed69c15a




## [0.18.0] · 2022-12-07
[0.18.0]: /../../tree/v0.18.0

[Diff](/../../compare/v0.17.0...v0.18.0) | [Milestone](/../../milestone/21)

### BC Breaks

- Added `NotFound` variant to `event::StepError`. ([#250])

### Fixed

- Not panicking on `fail_on_skipped()` with retries. ([#250], [#249])

[#249]: /../../issues/249
[#250]: /../../pull/250




## [0.17.0] · 2022-11-23
[0.17.0]: /../../tree/v0.17.0

[Diff](/../../compare/v0.16.0...v0.17.0) | [Milestone](/../../milestone/20)

### BC Breaks

- Added `event::ScenarioFinished` as [`Cucumber::after`][0170-1] hook's argument, explaining why the `Scenario` has finished. ([#246], [#245])

### Fixed

- Uncaught panics of user code, when they happen before first poll of the returned `Future`s. ([#246])

[#245]: /../../discussions/245
[#246]: /../../pull/246
[0170-1]: https://docs.rs/cucumber/0.17.0/cucumber/struct.Cucumber.html#method.after




## [0.16.0] · 2022-11-09
[0.16.0]: /../../tree/v0.16.0

[Diff](/../../compare/v0.15.3...v0.16.0) | [Milestone](/../../milestone/19)

### BC Breaks

- Bumped up [MSRV] to 1.65 for using `let`-`else` statements. ([7f52d4a5])

### Added

- `--ff` CLI alias for `--fail-fast` CLI option. ([#242])

### Fixed

- `--fail-fast` CLI option causing execution to hang. ([#242], [#241])

[#241]: /../../issues/241
[#242]: /../../pull/242
[7f52d4a5]: /../../commit/7f52d4a5faa3b69bec6c7fb765b50455cf7802aa




## [0.15.3] · 2022-11-01
[0.15.3]: /../../tree/v0.15.3

[Diff](/../../compare/v0.15.2...v0.15.3) | [Milestone](/../../milestone/18)

### Added

- `Clone` implementations to all public types where possible. ([#238])

[#238]: /../../pull/238




## [0.15.2] · 2022-10-25
[0.15.2]: /../../tree/v0.15.2

[Diff](/../../compare/v0.15.1...v0.15.2) | [Milestone](/../../milestone/17)

### Changed

- Upgraded [`gherkin`] crate to 0.13 version. ([4cad49f8])

### Fixed

- Parsing error on a `Feature` having comment and tag simultaneously. ([4cad49f8], [cucumber-rs/gherkin#37], [cucumber-rs/gherkin#35])
- `@retry`, `@serial` and `@allow.skipped` tags semantics inheritance. ([#237])

[#237]: /../../pull/237
[4cad49f8]: /../../commit/4cad49f8d8f5d0458dcb538aa044a5fff1e6fa10
[cucumber-rs/gherkin#35]: https://github.com/cucumber-rs/gherkin/issues/35
[cucumber-rs/gherkin#37]: https://github.com/cucumber-rs/gherkin/pull/37




## [0.15.1] · 2022-10-12
[0.15.1]: /../../tree/v0.15.1

[Diff](/../../compare/v0.15.0...v0.15.1) | [Milestone](/../../milestone/16)

### Fixed

- Conflicting [`Id`][0151-1]s of CLI options. ([#232], [#231])

[#231]: /../../issues/231
[#232]: /../../pull/232
[0151-1]: https://docs.rs/clap/latest/clap/struct.Id.html




## [0.15.0] · 2022-10-05
[0.15.0]: /../../tree/v0.15.0

[Diff](/../../compare/v0.14.2...v0.15.0) | [Milestone](/../../milestone/15)

### BC Breaks

- Upgraded [`clap`] crate to 4.0 version. ([#230])

[#230]: /../../pull/230




## [0.14.2] · 2022-09-19
[0.14.2]: /../../tree/v0.14.2

[Diff](/../../compare/v0.14.1...v0.14.2)

### Fixed

- `#[derive(World)]` macro being unhygienic regarding custom `Result` types. ([186af8b1])

[186af8b1]: /../../commit/186af8b1de37275b308897e2e30d6982830b0278




## [0.14.1] · 2022-09-12
[0.14.1]: /../../tree/v0.14.1

[Diff](/../../compare/v0.14.0...v0.14.1) | [Milestone](/../../milestone/14)

### Changed

- Considered stripping `CARGO_WORKSPACE_DIR` from output paths whenever is defined. ([ad0bb22f])

### Fixed

- `CARGO_MANIFEST_DIR` being detected in compile time. ([ad0bb22f])

### Security updated

- `junit-report` crate to 0.8 version to fix [RUSTSEC-2022-0048]. ([#229], [#226])

[#226]: /../../issues/226
[#229]: /../../pull/229
[ad0bb22f]: /../../commit/ad0bb22f9234099985cb1966f92ccefbc97060fb
[RUSTSEC-2022-0048]: https://rustsec.org/advisories/RUSTSEC-2022-0048.html




## [0.14.0] · 2022-09-08
[0.14.0]: /../../tree/v0.14.0

[Diff](/../../compare/v0.13.0...v0.14.0) | [Milestone](/../../milestone/13)

### BC Breaks

- Bumped up [MSRV] to 1.62 for more clever support of [Cargo feature]s and simplified codegen. ([fbd08ec2], [cf055ac0], [8ad5cc86])
- Replaced `#[derive(WorldInit)]` with `#[derive(World)]` to remove the need of manual `World` trait implementation. ([#219], [#217])
- Merged `WorldInit` trait into the `World` trait. ([#219])
- Added `ParsingFinished` variant to `event::Cucumber`. ([#220])
- Reworked `writer::Failure`/`writer::discard::Failure` as `writer::Stats`/`writer::discard::Stats`. ([#220])
- Renamed `WriterExt::discard_failure_writes()` to `WriterExt::discard_stats_writes()`. ([#220])
- Added `Option<step::Location>` field to `event::Step::Passed` and `event::Step::Failed`. ([#221])
- Wrapped `event::Scenario` into `event::RetryableScenario` for storing in other `event`s. ([#223], [#212])
- Added `retried_steps()` method to `writer::Stats`. ([#223], [#212])

### Added

- `writer::Libtest` (enables [IntelliJ Rust integration][0140-1]) behind the `libtest` feature flag. ([#220])
- `writer::Or` to alternate between 2 `Writer`s basing on a predicate. ([#220])
- `writer::Stats::passed_steps()` and `writer::Stats::skipped_steps()` methods. ([#220])
- `FeatureExt::count_steps()` method. ([#220])
- Location of the `fn` matching a failed `Step` in output. ([#221])
- Ability to retry failed `Scenario`s. ([#223], [#212])
- `--retry`, `--retry-after` and `--retry-tag-filter` CLI options. ([#223], [#212]) 

### Changed

- Provided default CLI options are now global (allowed to be specified after custom subcommands). ([#216], [#215])
- Stripped `CARGO_MANIFEST_DIR` from output paths whenever is possible. ([#221])

[#212]: /../../issues/212
[#215]: /../../issues/215
[#216]: /../../pull/216
[#217]: /../../issues/217
[#219]: /../../pull/219
[#220]: /../../pull/220
[#221]: /../../pull/221
[#223]: /../../pull/223
[8ad5cc86]: /../../commit/8ad5cc866bb9d6b49470790e3b0dd40690f63a09
[cf055ac0]: /../../commit/cf055ac06c7b72f572882ce15d6a60da92ad60a0
[fbd08ec2]: /../../commit/fbd08ec24dbd036c89f5f0af4d936b616790a166
[0140-1]: book/src/output/intellij.md




## [0.13.0] · 2022-03-29
[0.13.0]: /../../tree/v0.13.0

[Diff](/../../compare/v0.12.2...v0.13.0) | [Milestone](/../../milestone/12)

### BC Breaks

- Upgraded [`gherkin`] crate to 0.12 version. ([#211])

[#211]: /../../pull/211




## [0.12.2] · 2022-03-28
[0.12.2]: /../../tree/v0.12.2

[Diff](/../../compare/v0.12.1...v0.12.2) | [Milestone](/../../milestone/10)

### Changed

- [`Cucumber::after`][0122-1] now gets the `World` instance even if some `Step` or a `Hook` before it has failed. ([#209], [#207])

[#207]: /../../issues/207
[#209]: /../../pull/209
[0122-1]: https://docs.rs/cucumber/0.12.2/cucumber/struct.Cucumber.html#method.after




## [0.12.1] · 2022-03-09
[0.12.1]: /../../tree/v0.12.1

[Diff](/../../compare/v0.12.0...v0.12.1) | [Milestone](/../../milestone/11)

### Security updated

- `regex` crate to 1.5.5 version to fix [CVE-2022-24713].

[CVE-2022-24713]: https://blog.rust-lang.org/2022/03/08/cve-2022-24713.html




## [0.12.0] · 2022-02-10
[0.12.0]: /../../tree/v0.12.0

[Diff](/../../compare/v0.11.3...v0.12.0) | [Milestone](/../../milestone/9)

### BC Breaks

- `step::Context::matches` now contains regex capturing group names in addition to captured values. ([#204])

### Added

- Support for multiple capturing groups in `Parameter` regex (previously was forbidden). ([#204])

### Fixed

- Book examples failing on Windows. ([#202], [#200])
- `{string}` parameter in [Cucumber Expressions] returning its enclosing quotes. ([cucumber-rs/cucumber-expressions#7])

[#200]: /../../issues/200
[#202]: /../../pull/202
[#204]: /../../pull/204
[cucumber-rs/cucumber-expressions#7]: https://github.com/cucumber-rs/cucumber-expressions/issues/7




## [0.11.3] · 2022-01-31
[0.11.3]: /../../tree/v0.11.3

[Diff](/../../compare/v0.11.2...v0.11.3) | [Milestone](/../../milestone/8)

### Fixed

- `parser::Basic` skipping files named `.feature`. ([#201])

[#201]: /../../pull/201




## [0.11.2] · 2022-01-19
[0.11.2]: /../../tree/v0.11.2

[Diff](/../../compare/v0.11.1...v0.11.2) | [Milestone](/../../milestone/7)

### Fixed

- Skipped `Background` steps not failing in `writer::FailOnSkipped`. ([#199], [#198])

[#198]: /../../issues/198
[#199]: /../../pull/199




## [0.11.1] · 2022-01-07
[0.11.1]: /../../tree/v0.11.1

[Diff](/../../compare/v0.11.0...v0.11.1) | [Milestone](/../../milestone/6)

### Added

- `--fail-fast` CLI option to `runner::Basic`. ([#196])

### Changed

- Optimized `runner::Basic` to not wait the whole batch to complete before executing next `Scenario`s. ([#195])
 
[#195]: /../../pull/195
[#196]: /../../pull/196




## [0.11.0] · 2022-01-03
[0.11.0]: /../../tree/v0.11.0

[Diff](/../../compare/v0.10.2...v0.11.0) | [Milestone](/../../milestone/3)

### BC Breaks

- Moved `World` type parameter of `WriterExt` trait to methods. ([#160])
- Renamed `Normalized` and `Summarized` `Writer`s to `Normalize` and `Summarize`. ([#162])
- Removed `writer::Basic` `Default` impl and change `writer::Basic::new()` return type to `writer::Normalize<writer::Basic>`. ([#162])
- Bump up [MSRV] to 1.57 for better error reporting in `const` assertions. ([cef3d480])
- Switch to [`gherkin`] crate instead of [`gherkin_rust`]. ([e2a41ab0])
- Renamed `@allow_skipped` built-in tag to `@allow.skipped`. ([#181])
- Switched CLI to [`clap`] from `structopt`. ([#188], [#155])
- Reworked `verbose` CLI option of `writer::Basic`: ([#193], [#192])
    - Removed long form.
    - Made `-v` default behavior (no additional output). 
    - Made `-vv` additionally output `World` on failed steps. 
    - Made `-vvv` additionally output docstrings (old behavior). 

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
- `writer::AssertNormalized` forcing `Normalized` implementation. ([#182])
- `cli::Colored` trait for propagating `Coloring` to arbitrary `Writer`s. ([#189], [#186])

### Fixed

- Template regex in `Scenario Outline` expansion from `<(\S+)>` to `<([^>\s]+)>`. ([#163])
- Multiple `Examples` in `Scenario Outline`. ([#165], [#164])
- Docstring and name expansion in `Scenario Outline`. ([#178], [#172])
- `writer::Summarized` ignoring `Coloring` options. ([#189], [#186])

[#147]: /../../pull/147
[#151]: /../../pull/151
[#155]: /../../issues/155
[#157]: /../../pull/157
[#159]: /../../pull/159
[#160]: /../../pull/160
[#162]: /../../pull/162
[#163]: /../../pull/163
[#164]: /../../issues/164
[#165]: /../../pull/165
[#166]: /../../pull/166
[#168]: /../../pull/168
[#172]: /../../issues/172
[#178]: /../../pull/178
[#181]: /../../pull/181
[#182]: /../../pull/182
[#186]: /../../issues/186
[#188]: /../../pull/188
[#189]: /../../pull/189
[#192]: /../../issues/192
[#193]: /../../pull/193
[cef3d480]: /../../commit/cef3d480579190425461ddb04a1248675248351e
[e2a41ab0]: /../../commit/e2a41ab0a4398fe26075f0b066cc67e6e8a19e6c
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
- [Hooks](https://cucumber.io/docs/cucumber/api#hooks) now accept optional `&mut World` as their last parameter. ([#142])

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




[`clap`]: https://docs.rs/clap
[`gherkin`]: https://docs.rs/gherkin
[`gherkin_rust`]: https://docs.rs/gherkin_rust
[`tracing`]: https://docs.rs/tracing

[Cargo feature]: https://doc.rust-lang.org/cargo/reference/features.html
[Cucumber Expressions]: https://cucumber.github.io/cucumber-expressions
[Gherkin]: https://cucumber.io/docs/gherkin
[MSRV]: https://doc.rust-lang.org/cargo/reference/manifest.html#the-rust-version-field
[Semantic Versioning 2.0.0]: https://semver.org
