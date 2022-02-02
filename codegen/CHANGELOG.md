`cucumber-codegen` changelog
============================

All user visible changes to `cucumber-codegen` crate will be documented in this file. This project uses [Semantic Versioning 2.0.0].




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




[Cucumber Expressions]: https://cucumber.github.io/cucumber-expressions
[MSRV]: https://doc.rust-lang.org/cargo/reference/manifest.html#the-rust-version-field
[Semantic Versioning 2.0.0]: https://semver.org
