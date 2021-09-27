### Current

#### Known issues:

- `Scenario Outline` is treated the same as `Outline` or `Example` in the parser ([gherkin/#19](https://github.com/bbqsrc/gherkin-rust/issues/19))

### [0.10.0] — ???
[0.10.0]: /../../tree/v0.10.0

- Update attributes according to the redesign done in `0.10` of `cucumber_rust` crate.
- Replace `#[given(step)]`, `#[when(step)]` and `#[then(step)]` with single `#[step]` attribute.

### 0.1.0 — 2021-01-18

- Initial implementation for [`given`](https://docs.rs/cucumber_rust_codegen/0.9.0/cucumber_rust_codegen/attr.given.html), 
  [`when`](https://docs.rs/cucumber_rust_codegen/0.9.0/cucumber_rust_codegen/attr.when.html), 
  [`then`](https://docs.rs/cucumber_rust_codegen/0.9.0/cucumber_rust_codegen/attr.then.html) attribute macros.
