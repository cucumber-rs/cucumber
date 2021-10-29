`cucumber-codegen` changelog
============================

All user visible changes to `cucumber-codegen` crate will be documented in this file. This project uses [Semantic Versioning 2.0.0].




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




[Semantic Versioning 2.0.0]: https://semver.org
