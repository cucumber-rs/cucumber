`cucumber-codegen` changelog
============================

All user visible changes to `cucumber-codegen` crate will be documented in this file. This project uses [Semantic Versioning 2.0.0].




## [0.15.3] · 2022-11-01
[0.15.3]: /../../tree/v0.15.3/codegen

[Milestone](/../../milestone/18)

### Version bump only

See `cucumber` crate [changelog](https://github.com/cucumber-rs/cucumber/blob/v0.15.3/CHANGELOG.md).




## [0.15.2] · 2022-10-25
[0.15.2]: /../../tree/v0.15.2/codegen

[Milestone](/../../milestone/17)

### Version bump only

See `cucumber` crate [changelog](https://github.com/cucumber-rs/cucumber/blob/v0.15.2/CHANGELOG.md).




## [0.15.1] · 2022-10-12
[0.15.1]: /../../tree/v0.15.1/codegen

[Milestone](/../../milestone/16)

### Version bump only

See `cucumber` crate [changelog](https://github.com/cucumber-rs/cucumber/blob/v0.15.1/CHANGELOG.md).




## [0.15.0] · 2022-10-05
[0.15.0]: /../../tree/v0.15.0/codegen

[Milestone](/../../milestone/15)

### Version bump only

See `cucumber` crate [changelog](https://github.com/cucumber-rs/cucumber/blob/v0.15.0/CHANGELOG.md).




## [0.14.2] · 2022-09-19
[0.14.2]: /../../tree/v0.14.2/codegen

### Fixed

- `#[derive(World)]` macro being unhygienic regarding custom `Result` types. ([186af8b1])

[186af8b1]: /../../commit/186af8b1de37275b308897e2e30d6982830b0278




## [0.14.1] · 2022-09-12
[0.14.1]: /../../tree/v0.14.1/codegen

[Milestone](/../../milestone/14)

### Version bump only

See `cucumber` crate [changelog](https://github.com/cucumber-rs/cucumber/blob/v0.14.1/CHANGELOG.md).




## [0.14.0] · 2022-09-08
[0.14.0]: /../../tree/v0.14.0/codegen

[Milestone](/../../milestone/13)

### BC Breaks

- Bumped up [MSRV] to 1.62 for more clever support of [Cargo feature]s and simplified codegen. ([fbd08ec2], [cf055ac0], [8ad5cc86])
- Replaced `#[derive(WorldInit)]` with `#[derive(World)]` to remove the need of manual `World` trait implementation. ([#219], [#217])

[#217]: /../../issues/217
[#219]: /../../pull/219
[8ad5cc86]: /../../commit/8ad5cc866bb9d6b49470790e3b0dd40690f63a09
[cf055ac0]: /../../commit/cf055ac06c7b72f572882ce15d6a60da92ad60a0
[fbd08ec2]: /../../commit/fbd08ec24dbd036c89f5f0af4d936b616790a166




## [0.13.0] · 2022-03-29
[0.13.0]: /../../tree/v0.13.0/codegen

[Milestone](/../../milestone/12)

### Version bump only

See `cucumber` crate [changelog](https://github.com/cucumber-rs/cucumber/blob/v0.13.0/CHANGELOG.md).




## [0.12.2] · 2022-03-28
[0.12.2]: /../../tree/v0.12.2/codegen

[Milestone](/../../milestone/10)

### Version bump only

See `cucumber` crate [changelog](https://github.com/cucumber-rs/cucumber/blob/v0.12.2/CHANGELOG.md).




## [0.12.1] · 2022-03-09
[0.12.1]: /../../tree/v0.12.1/codegen

[Milestone](/../../milestone/11)

### Security updated

- `regex` crate to 1.5.5 version to fix [CVE-2022-24713].

[CVE-2022-24713]: https://blog.rust-lang.org/2022/03/08/cve-2022-24713.html




## [0.12.0] · 2022-02-10
[0.12.0]: /../../tree/v0.12.0/codegen

[Milestone](/../../milestone/9)

### Added

- Support for multiple capturing groups in `Parameter` regex (previously was forbidden). ([#204])

[#204]: /../../pull/204




## [0.11.3] · 2022-01-31
[0.11.3]: /../../tree/v0.11.3/codegen

[Milestone](/../../milestone/8)

### Version bump only

See `cucumber` crate [changelog](https://github.com/cucumber-rs/cucumber/blob/v0.11.3/CHANGELOG.md).




## [0.11.2] · 2022-01-19
[0.11.2]: /../../tree/v0.11.2/codegen

[Milestone](/../../milestone/7)

### Version bump only

See `cucumber` crate [changelog](https://github.com/cucumber-rs/cucumber/blob/v0.11.2/CHANGELOG.md).




## [0.11.1] · 2022-01-07
[0.11.1]: /../../tree/v0.11.1/codegen

[Milestone](/../../milestone/6)

### Version bump only

See `cucumber` crate [changelog](https://github.com/cucumber-rs/cucumber/blob/v0.11.1/CHANGELOG.md).




## [0.11.0] · 2022-01-03
[0.11.0]: /../../tree/v0.11.0/codegen

[Milestone](/../../milestone/3)

### BC Breaks

- Bump up [MSRV] to 1.57 for better error reporting in `const` assertions. ([cef3d480])

### Added

- Unwrapping `Result`s returned by step functions. ([#151])
- `expr = ...` argument to `#[given(...)]`, `#[when(...)]` and `#[then(...)]` attributes allowing [Cucumber Expressions]. ([#157])
- `#[derive(Parameter)]` attribute macro for implementing custom parameters of [Cucumber Expressions]. ([#168])

[#151]: /../../pull/151
[#157]: /../../pull/157
[#168]: /../../pull/168
[cef3d480]: /../../commit/cef3d480579190425461ddb04a1248675248351e




## [0.10.2] · 2021-11-03
[0.10.2]: /../../tree/v0.10.2/codegen

[Milestone](/../../milestone/5)

### Added

- World's type name to the generated `WorldInit` machinery to omit conflicts for different types in the same module. ([#150])

[#150]: /../../pull/150




## [0.10.1] · 2021-10-29
[0.10.1]: /../../tree/v0.10.1/codegen

[Milestone](/../../milestone/4)

### Version bump only

See `cucumber` crate [changelog](https://github.com/cucumber-rs/cucumber/blob/v0.10.1/CHANGELOG.md).




## [0.10.0] · 2021-10-26
[0.10.0]: /../../tree/v0.10.0/codegen

[Milestone](/../../milestone/2)

### BC Breaks

- Renamed crate to `cucumber-codegen`.
- Replaced `#[given(step)]`, `#[when(step)]` and `#[then(step)]` function argument attributes with a single `#[step]`. ([#128])

[#128]: /../../pull/128




## [0.1.0] · 2021-01-18
[0.1.0]: /../../tree/v0.8.0/codegen

### Added

- Attribute macros: ([#81])
    - [`given`](https://docs.rs/cucumber_rust_codegen/0.1.0/cucumber_rust_codegen/attr.given.html); 
    - [`when`](https://docs.rs/cucumber_rust_codegen/0.1.0/cucumber_rust_codegen/attr.when.html);
    - [`then`](https://docs.rs/cucumber_rust_codegen/0.1.0/cucumber_rust_codegen/attr.then.html).

[#81]: /../../pull/81




[Cargo feature]: https://doc.rust-lang.org/cargo/reference/features.html
[Cucumber Expressions]: https://cucumber.github.io/cucumber-expressions
[MSRV]: https://doc.rust-lang.org/cargo/reference/manifest.html#the-rust-version-field
[Semantic Versioning 2.0.0]: https://semver.org
