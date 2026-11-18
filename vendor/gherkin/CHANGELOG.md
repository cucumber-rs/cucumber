`gherkin` crate changelog
=========================

All user visible changes to `gherkin` crate will be documented in this file. This project uses [Semantic Versioning 2.0.0].




## [0.16.0] · 2026-04-23
[0.16.0]: https://github.com/cucumber-rs/gherkin/tree/v0.16.0

[Diff](https://github.com/cucumber-rs/gherkin/compare/v0.15.0...v0.16.0) | [Milestone](https://github.com/cucumber-rs/gherkin/milestone/5)

### BC Breaks

- Fixed precedence of operations in tag expressions to align with upstream. ([#52], [#51])

### Changed

- Allowed `And`-like and `But`-like keywords at the beginning of `Scenario`, automatically considering them as `Given`-type steps. ([#53])

[#51]: https://github.com/cucumber-rs/gherkin/issues/51
[#52]: https://github.com/cucumber-rs/gherkin/pull/52
[#53]: https://github.com/cucumber-rs/gherkin/pull/53




## [0.15.0] · 2025-12-12
[0.15.0]: https://github.com/cucumber-rs/gherkin/tree/v0.15.0

[Diff](https://github.com/cucumber-rs/gherkin/compare/v0.14.0...v0.15.0)

### BC Breaks

- Bump up [MSRV] to 1.88 to support newer versions of dependencies. ([17c13206])

### Fixed

- Incorrect parsing of [escape sequences in table cells](https://cucumber.io/docs/gherkin/reference#table-cell-escaping). ([#47])

### Upgraded

- [`juniper`] crate to 0.17 version. ([f1dbe13a])

[#47]: https://github.com/cucumber-rs/gherkin/pull/47
[17c13206]: https://github.com/cucumber-rs/gherkin/commit/17c1320605c07229580277cf54f6737f91cfeb13
[f1dbe13a]: https://github.com/cucumber-rs/gherkin/commit/f1dbe13a8db50500ff23da6a74c9b04df5bd617f




## [0.14.0] · 2023-07-14
[0.14.0]: https://github.com/cucumber-rs/gherkin/tree/v0.14.0

[Diff](https://github.com/cucumber-rs/gherkin/compare/v0.13.0...v0.14.0)

### BC Breaks

- Bump up [MSRV] to 1.65 to support newer versions of dependencies.

### Upgraded

- [`syn`] crate to 2.0 version. ([bdf31e7c])
- [`typed-builder`] crate to 0.15 version. ([bdf31e7c])

[bdf31e7c]: https://github.com/cucumber-rs/gherkin/commit/bdf31e7c093b6a3c74155d140125978cb3f6a4dc




## [0.13.0] · 2022-10-24
[0.13.0]: https://github.com/cucumber-rs/gherkin/tree/v0.13.0

[Diff](https://github.com/cucumber-rs/gherkin/compare/v0.12.0...v0.13.0) | [Milestone](https://github.com/cucumber-rs/gherkin/milestone/4)

### BC Breaks

- Bump up [MSRV] to 1.62 to support newer versions of dependencies.

### Fixed

- Parsing error on a `Feature` having comment and `Tag` simultaneously. ([#37], [#35])

[#35]: https://github.com/cucumber-rs/gherkin/issues/35
[#37]: https://github.com/cucumber-rs/gherkin/pull/37




## [0.12.0] · 2022-03-28
[0.12.0]: https://github.com/cucumber-rs/gherkin/tree/v0.12.0

[Diff](https://github.com/cucumber-rs/gherkin/compare/v0.11.2...v0.12.0) | [Milestone](https://github.com/cucumber-rs/gherkin/milestone/2)

### BC Breaks

- Made `name` field of `Background` required. ([#32])
- Make `table` field of `Examples` optional. ([#32])

### Added

- Support text after `Background` and `Examples` keywords. ([#31])
- `description` field to `Background`, `Examples`, `Rule` and `Scenario`. ([#32], [#10])

[#10]: https://github.com/cucumber-rs/gherkin/issues/10
[#31]: https://github.com/cucumber-rs/gherkin/pull/31
[#32]: https://github.com/cucumber-rs/gherkin/pull/32




## [0.11.2] · 2022-02-18
[0.11.2]: https://github.com/cucumber-rs/gherkin/tree/v0.11.2

[Diff](https://github.com/cucumber-rs/gherkin/compare/v0.11.1...v0.11.2)

### Fixed

- Incorrect line numbers reporting. ([#33])

[#33]: https://github.com/cucumber-rs/gherkin/pull/33




## [0.11.1] · 2021-12-08
[0.11.1]: https://github.com/cucumber-rs/gherkin/tree/v0.11.1

[Diff](https://github.com/cucumber-rs/gherkin/compare/v0.11.0...v0.11.1)

### Fixed

- Allowed keywords in `Feature`s `Description`. ([#30], [cucumber#175])
- Allowed characters in `Tag`s. ([#30], [cucumber#174])
- Comments on the same line with `Tag`s. ([#30])
- `Tag`s requiring whitespaces between them. ([#30])
- [Escaping][0111-1] in `TagOperation`. ([#30])

[#30]: https://github.com/cucumber-rs/gherkin/pull/30
[cucumber#174]: https://github.com/cucumber-rs/cucumber/issues/174
[cucumber#175]: https://github.com/cucumber-rs/cucumber/issues/175
[0111-1]: https://github.com/cucumber/tag-expressions/tree/6f444830b23bd8e0c5a2617cd51b91bc2e05adde#escaping




## [0.11.0] · 2021-12-06
[0.11.0]: https://github.com/cucumber-rs/gherkin/tree/v0.11.0

[Diff](https://github.com/cucumber-rs/gherkin/compare/v0.10.2...v0.11.0)

### BC Breaks

- Renamed crate from `gherkin_rust` to just `gherkin`. ([d8803b80])
- Yank [0.10.2] version, as it's appeared to be backwards incompatible.

[d8803b80]: https://github.com/cucumber-rs/gherkin/commit/d8803b808eb5bd2684b9dc7c868a9637a0398100




## [0.10.2] · 2021-11-29
[0.10.2]: https://github.com/cucumber-rs/gherkin/tree/v0.10.2

[Diff](https://github.com/cucumber-rs/gherkin/compare/v0.10.1...v0.10.2)

### Added

- Support of multiple `Examples` in `Scenario Outline`. ([#29])

[#29]: https://github.com/cucumber-rs/gherkin/pull/29




[`syn`]: https://docs.rs/syn
[`typed-builder`]: https://docs.rs/typed-builder
[MSRV]: https://doc.rust-lang.org/cargo/reference/manifest.html#the-rust-version-field
[Semantic Versioning 2.0.0]: https://semver.org
