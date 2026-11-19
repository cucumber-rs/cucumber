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




## GQAdonis fork status

The current [contribution-readiness repair](docs/CONTRIBUTION-READINESS.md) is
implemented and awaits phase-end behavioral proof; earlier results below are
historical. No PR has been marked ready or merged on this unverified change.

This fork adds opt-in Gherkin43 dialect, lexical, table and docstring semantics,
both step argument orders and source-preserving Markdown parsing. The verified
foundation is preserved at
[efb91e8](https://github.com/GQAdonis/gherkin/commit/efb91e8bb5a7d59a717f88c05d1c06ec5d06a819)
on foundation/rust-bdd-sdk. It is not a published crates.io release.

See [corpus coverage](docs/gherkin43-corpus.md),
[dialect policy](docs/gherkin43-dialects.md) and
[Markdown API](docs/gherkin43-markdown.md). The latest focused sibling BDD run
passed 73 scenarios/241 steps on Rust1.88;
[evidence is retained in the consuming fork](https://github.com/GQAdonis/cucumber/blob/6e4753fc0783038bb388de1b4d76055edc697b8f/docs/compatibility/evidence/phase45-parser-affected-bdd.log).
This is bounded evidence, not universal conformance.

An additive [Docusaurus site](docs-site/README.md) now has local source and an
independent behavior acceptance boundary. The paired-site run passed 10 scenarios /
64 steps; evidence is retained in the Cucumber fork. It identifies this fork; the additive Pages workflow targets
[public fork documentation](https://gqadonis.github.io/gherkin/). See
[deployment status and verification](docs-site/README.md#pages-delivery). The shared native SDK programme lives in Cucumber
has completed its first installed-native phase (22 scenarios/66 steps); foreign execution and framework adapters remain future work. See [reflection and contribution limits](docs/CONTRIBUTION-REFLECTION.md). Draft upstream contributions are authorized, without a full-parity or whole-codebase-clean claim. Established build/release processes remain unchanged.

## Usage

For the fork behavior described above, use the complete pinned checkout from
[Getting started](docs-site/docs/getting-started.md). The registry package does
not contain these unpublished changes.

```toml
[dependencies]
gherkin = { path = "../gherkin" }
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
