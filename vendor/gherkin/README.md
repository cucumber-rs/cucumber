Gherkin language for Rust
=========================

[![crates.io](https://img.shields.io/crates/v/gherkin.svg "crates.io")](https://crates.io/crates/gherkin)
[![Rust 1.88+](https://img.shields.io/badge/rustc-1.88+-lightgray.svg "Rust 1.88+")](https://blog.rust-lang.org/2025/06/26/Rust-1.88.0)
[![Unsafe Forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg "Unsafe forbidden")](https://github.com/rust-secure-code/safety-dance)\
[![CI](https://github.com/cucumber-rs/gherkin/actions/workflows/ci.yml/badge.svg?branch=main "CI")](https://github.com/cucumber-rs/gherkin/actions?query=workflow%3ACI+branch%3Amain)
[![Rust docs](https://docs.rs/gherkin/badge.svg "Rust docs")](https://docs.rs/gherkin)

[Changelog](https://github.com/cucumber-rs/gherkin/blob/v0.16.0/CHANGELOG.md)

A pure [Rust] implementation of the [Gherkin] (`.feature` file) language for the [Cucumber] testing framework.

If you want to run [Cucumber] tests in [Rust], try [`cucumber` crate](https://github.com/cucumber-rs/cucumber)!




## Usage

```toml
[dependencies]
gherkin = "0.15"
```




## Further information

For a detailed description of [Gherkin] usage, you can refer to upstream [Cucumber] documentation.


### Upstream documentation

1. for Cucumber **developers**:
   * [the Gherkin readme](https://github.com/cucumber/gherkin/blob/main/README.md)
   * [the Gherkin contributing guide](https://github.com/cucumber/gherkin/blob/main/CONTRIBUTING.md)
1. for Cucumber **users**:
   * [the Cucumber user documentation](https://cucumber.io/docs/cucumber).
   * [the Gherkin user documentation](https://cucumber.io/docs/gherkin).




## License

This project is licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

[Original source](https://github.com/cucumber/gherkin/blob/main/gherkin-languages.json) of `src/languages.json` is used under the [MIT license](https://github.com/cucumber/gherkin/blob/main/LICENSE).




[Cucumber]: https://cucumber.io
[Gherkin]: https://cucumber.io/docs/gherkin
[Rust]: https://www.rust-lang.org
